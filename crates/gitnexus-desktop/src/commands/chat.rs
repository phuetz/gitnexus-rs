//! Chat Q&A commands — ask questions about the codebase.
//!
//! Pipeline:
//!   1. Search the knowledge graph (FTS + graph traversal) for relevant context
//!   2. Read source code snippets for the top results
//!   3. Assemble a structured prompt with graph context
//!   4. Call an OpenAI-compatible LLM API
//!   5. Return the answer with source citations

use std::path::PathBuf;

use tauri::State;

use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_db::inmemory::fts::FtsIndex;

use crate::state::AppState;
use crate::types::{ChatConfig, ChatMessage, ChatRequest, ChatResponse, ChatSource};

// ─── LLM Configuration ──────────────────────────────────────────────

const DEFAULT_CONFIG_FILENAME: &str = "chat-config.json";

fn config_path() -> PathBuf {
    let home = dirs_fallback();
    home.join(".gitnexus").join(DEFAULT_CONFIG_FILENAME)
}

fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn load_config() -> ChatConfig {
    let path = config_path();
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str(&content) {
                return config;
            }
        }
    }
    ChatConfig::default()
}

fn save_config(config: &ChatConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Tauri Commands ──────────────────────────────────────────────────

/// Ask a question about the codebase.
#[tauri::command]
pub async fn chat_ask(
    state: State<'_, AppState>,
    request: ChatRequest,
) -> Result<ChatResponse, String> {
    let config = load_config();

    // 1. Get the active repo's graph and FTS index
    let (graph, _indexes, fts_index, repo_path_str) = state.get_repo(None).await?;
    let repo_path = PathBuf::from(&repo_path_str);

    // 2. Search for relevant symbols
    let search_results = search_relevant_context(&request.question, &graph, &fts_index, 10);

    // 3. Read code snippets for top results
    let sources = build_sources(&search_results, &graph, &repo_path);

    // 4. Assemble the prompt
    let system_prompt = build_system_prompt(&graph, &sources);
    let messages = build_llm_messages(&system_prompt, &request.history, &request.question);

    // 5. Call LLM if configured
    if config.api_key.is_empty() && !config.base_url.contains("localhost") {
        // No LLM configured — return graph-only answer
        return Ok(build_graph_only_response(&search_results, &sources, &graph));
    }

    let answer = call_llm(&config, &messages).await?;

    Ok(ChatResponse {
        answer,
        sources,
        model: Some(config.model.clone()),
    })
}

/// Get the current chat configuration.
#[tauri::command]
pub async fn chat_get_config() -> Result<ChatConfig, String> {
    Ok(load_config())
}

/// Save chat configuration (LLM provider settings).
#[tauri::command]
pub async fn chat_set_config(config: ChatConfig) -> Result<(), String> {
    save_config(&config)
}

/// Quick search — returns relevant symbols without calling LLM.
/// Used for "Deep Research" context gathering.
#[tauri::command]
pub async fn chat_search_context(
    state: State<'_, AppState>,
    query: String,
    max_results: Option<usize>,
) -> Result<Vec<ChatSource>, String> {
    let (graph, _indexes, fts_index, repo_path_str) = state.get_repo(None).await?;
    let repo_path = PathBuf::from(&repo_path_str);

    let results = search_relevant_context(&query, &graph, &fts_index, max_results.unwrap_or(10));
    let sources = build_sources(&results, &graph, &repo_path);

    Ok(sources)
}

// ─── Public Helpers (used by chat_executor) ─────────────────────────

/// Public config loader for the executor module.
pub fn load_config_pub() -> ChatConfig {
    load_config()
}

/// Public LLM call for the executor module.
pub async fn call_llm_pub(
    config: &ChatConfig,
    messages: &[serde_json::Value],
) -> Result<String, String> {
    call_llm(config, messages).await
}

/// Public search function for the executor module.
pub fn search_relevant_context_pub(
    query: &str,
    graph: &KnowledgeGraph,
    fts_index: &FtsIndex,
    max_results: usize,
) -> Vec<(String, f64)> {
    search_relevant_context(query, graph, fts_index, max_results)
}

// ─── Context Assembly ────────────────────────────────────────────────

/// Search for symbols relevant to the question using FTS + graph traversal.
fn search_relevant_context(
    query: &str,
    graph: &KnowledgeGraph,
    fts_index: &FtsIndex,
    max_results: usize,
) -> Vec<(String, f64)> {
    // BM25 full-text search
    let fts_results = fts_index.search(graph, query, None, max_results * 2);

    // Deduplicate and score
    let mut seen = std::collections::HashSet::new();
    let mut results: Vec<(String, f64)> = Vec::new();

    for fts_result in fts_results {
        if seen.insert(fts_result.node_id.clone()) {
            results.push((fts_result.node_id, fts_result.score));
        }
    }

    // Also search by exact name match in graph
    let query_lower = query.to_lowercase();
    for node in graph.iter_nodes() {
        if node.properties.name.to_lowercase().contains(&query_lower) && seen.insert(node.id.clone()) {
            results.push((node.id.clone(), 1.0));
        }
    }

    // Sort by score descending, take top N
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(max_results);
    results
}

/// Build source citations with code snippets.
fn build_sources(
    results: &[(String, f64)],
    graph: &KnowledgeGraph,
    repo_path: &PathBuf,
) -> Vec<ChatSource> {
    let mut sources = Vec::new();

    for (node_id, score) in results {
        let node = match graph.get_node(node_id) {
            Some(n) => n,
            None => continue,
        };

        // Skip non-code nodes (Community, Process, File, etc.)
        match node.label {
            NodeLabel::Function
            | NodeLabel::Method
            | NodeLabel::Constructor
            | NodeLabel::Class
            | NodeLabel::Struct
            | NodeLabel::Trait
            | NodeLabel::Interface
            | NodeLabel::Enum
            | NodeLabel::TypeAlias => {}
            _ => continue,
        }

        // Try to read a code snippet
        let snippet = read_code_snippet(repo_path, &node.properties.file_path, node.properties.start_line, node.properties.end_line);

        // Get relationships for context
        let mut callers = Vec::new();
        let mut callees = Vec::new();
        for rel in graph.iter_relationships() {
            if rel.rel_type == RelationshipType::Calls {
                if rel.source_id == *node_id {
                    if let Some(target) = graph.get_node(&rel.target_id) {
                        callees.push(target.properties.name.clone());
                    }
                } else if rel.target_id == *node_id {
                    if let Some(source) = graph.get_node(&rel.source_id) {
                        callers.push(source.properties.name.clone());
                    }
                }
            }
        }

        // Get community
        let community = graph
            .iter_relationships()
            .find(|r| r.rel_type == RelationshipType::MemberOf && r.source_id == *node_id)
            .and_then(|r| graph.get_node(&r.target_id))
            .map(|c| {
                c.properties
                    .heuristic_label
                    .clone()
                    .unwrap_or_else(|| c.properties.name.clone())
            });

        sources.push(ChatSource {
            node_id: node_id.clone(),
            symbol_name: node.properties.name.clone(),
            symbol_type: node.label.as_str().to_string(),
            file_path: node.properties.file_path.clone(),
            start_line: node.properties.start_line,
            end_line: node.properties.end_line,
            snippet,
            callers: if callers.is_empty() { None } else { Some(callers) },
            callees: if callees.is_empty() { None } else { Some(callees) },
            community,
            relevance_score: *score,
        });
    }

    sources
}

/// Read a code snippet from a source file.
fn read_code_snippet(
    repo_path: &PathBuf,
    file_path: &str,
    start_line: Option<u32>,
    end_line: Option<u32>,
) -> Option<String> {
    let full_path = repo_path.join(file_path);
    let content = std::fs::read_to_string(&full_path).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    match (start_line, end_line) {
        (Some(start), Some(end)) => {
            let start = (start.saturating_sub(1)) as usize;
            let end = std::cmp::min(end as usize, lines.len());
            // Limit snippet to 50 lines max
            let end = std::cmp::min(end, start + 50);
            Some(lines[start..end].join("\n"))
        }
        (Some(start), None) => {
            let start = (start.saturating_sub(1)) as usize;
            let end = std::cmp::min(start + 20, lines.len());
            Some(lines[start..end].join("\n"))
        }
        _ => {
            // Return first 30 lines if no line info
            let end = std::cmp::min(30, lines.len());
            Some(lines[..end].join("\n"))
        }
    }
}

// ─── Prompt Construction ─────────────────────────────────────────────

/// Build the system prompt with graph context.
fn build_system_prompt(graph: &KnowledgeGraph, sources: &[ChatSource]) -> String {
    let node_count = graph.iter_nodes().count();
    let edge_count = graph.iter_relationships().count();

    let mut prompt = format!(
        r#"You are an expert code analyst helping a developer understand a codebase.
You have access to a knowledge graph of this repository ({} symbols, {} relationships).

## Relevant Code Context

The following symbols and code snippets are the most relevant to the user's question:

"#,
        node_count, edge_count
    );

    for (i, source) in sources.iter().enumerate() {
        prompt.push_str(&format!(
            "### {} — `{}` ({}) in `{}`\n",
            i + 1,
            source.symbol_name,
            source.symbol_type,
            source.file_path
        ));

        if let Some(community) = &source.community {
            prompt.push_str(&format!("**Module**: {}\n", community));
        }

        if let Some(callers) = &source.callers {
            prompt.push_str(&format!("**Called by**: {}\n", callers.join(", ")));
        }
        if let Some(callees) = &source.callees {
            prompt.push_str(&format!("**Calls**: {}\n", callees.join(", ")));
        }

        if let Some(snippet) = &source.snippet {
            let lang = detect_language(&source.file_path);
            prompt.push_str(&format!("\n```{}\n{}\n```\n\n", lang, snippet));
        }
        prompt.push('\n');
    }

    prompt.push_str(
        r#"## Instructions

- Answer the developer's question based on the code context above.
- Reference specific symbols, files, and line numbers when relevant.
- If you include code examples, use markdown code blocks with the correct language.
- If you're unsure about something, say so rather than guessing.
- Be concise but thorough. Use bullet points for lists.
- If applicable, mention related modules or functions the developer should look at.
- Respond in the same language as the user's question.
"#,
    );

    prompt
}

/// Build the messages array for the LLM API call.
fn build_llm_messages(
    system_prompt: &str,
    history: &[ChatMessage],
    question: &str,
) -> Vec<serde_json::Value> {
    let mut messages = vec![serde_json::json!({
        "role": "system",
        "content": system_prompt
    })];

    // Add conversation history (last 10 messages)
    for msg in history.iter().rev().take(10).rev() {
        messages.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content
        }));
    }

    // Add the current question
    messages.push(serde_json::json!({
        "role": "user",
        "content": question
    }));

    messages
}

/// Detect language from file extension.
fn detect_language(file_path: &str) -> &str {
    match file_path.rsplit('.').next() {
        Some("rs") => "rust",
        Some("js" | "mjs" | "cjs") => "javascript",
        Some("ts" | "mts" | "cts") => "typescript",
        Some("tsx") => "tsx",
        Some("jsx") => "jsx",
        Some("py") => "python",
        Some("java") => "java",
        Some("c" | "h") => "c",
        Some("cpp" | "hpp" | "cc") => "cpp",
        Some("cs") => "csharp",
        Some("go") => "go",
        Some("rb") => "ruby",
        Some("php") => "php",
        Some("kt" | "kts") => "kotlin",
        Some("swift") => "swift",
        Some("toml") => "toml",
        Some("json") => "json",
        Some("yaml" | "yml") => "yaml",
        Some("md") => "markdown",
        Some("sql") => "sql",
        Some("sh" | "bash") => "bash",
        _ => "",
    }
}

// ─── LLM API Call ────────────────────────────────────────────────────

/// Call an OpenAI-compatible LLM API.
async fn call_llm(
    config: &ChatConfig,
    messages: &[serde_json::Value],
) -> Result<String, String> {
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

    let body = serde_json::json!({
        "model": config.model,
        "messages": messages,
        "max_tokens": config.max_tokens,
        "temperature": 0.3,
        "stream": false
    });

    let client = reqwest::Client::new();
    let mut request = client.post(&url).json(&body);

    // Add authorization header if API key is provided
    if !config.api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", config.api_key));
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("LLM API request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("LLM API error ({}): {}", status, error_text));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

    // Extract the assistant message content
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| "No content in LLM response".to_string())?;

    Ok(content.to_string())
}

// ─── Graph-Only Response ─────────────────────────────────────────────

/// Build a response using only graph data (no LLM).
fn build_graph_only_response(
    results: &[(String, f64)],
    sources: &[ChatSource],
    graph: &KnowledgeGraph,
) -> ChatResponse {
    let mut answer = String::from("## Relevant Symbols Found\n\n");
    answer.push_str("*No LLM configured — showing graph search results. Configure an API key in Settings to get AI-powered answers.*\n\n");

    for source in sources {
        answer.push_str(&format!(
            "### `{}` ({}) — `{}`\n",
            source.symbol_name, source.symbol_type, source.file_path
        ));

        if let Some(community) = &source.community {
            answer.push_str(&format!("**Module**: {}\n", community));
        }
        if let Some(callers) = &source.callers {
            answer.push_str(&format!("**Called by**: {}\n", callers.join(", ")));
        }
        if let Some(callees) = &source.callees {
            answer.push_str(&format!("**Calls**: {}\n", callees.join(", ")));
        }

        if let Some(snippet) = &source.snippet {
            let lang = detect_language(&source.file_path);
            // Show first 15 lines of snippet
            let short: String = snippet.lines().take(15).collect::<Vec<_>>().join("\n");
            answer.push_str(&format!("\n```{}\n{}\n```\n\n", lang, short));
        }
    }

    ChatResponse {
        answer,
        sources: sources.to_vec(),
        model: None,
    }
}

