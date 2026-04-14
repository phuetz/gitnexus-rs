//! Chat Q&A commands — ask questions about the codebase.
//!
//! Pipeline:
//!   1. Search the knowledge graph (FTS + graph traversal) for relevant context
//!   2. Read source code snippets for the top results
//!   3. Assemble a structured prompt with graph context
//!   4. Call an OpenAI-compatible LLM API
//!   5. Return the answer with source citations

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_db::inmemory::fts::FtsIndex;

use crate::state::AppState;
use crate::types::{ChatConfig, ChatRequest, ChatResponse, ChatSource};

// ─── LLM Configuration ──────────────────────────────────────────────

const DEFAULT_CONFIG_FILENAME: &str = "chat-config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedChatConfig {
    provider: String,
    base_url: String,
    model: String,
    max_tokens: u32,
    #[serde(default)]
    reasoning_effort: String,
}

impl From<&ChatConfig> for PersistedChatConfig {
    fn from(config: &ChatConfig) -> Self {
        Self {
            provider: config.provider.clone(),
            base_url: config.base_url.clone(),
            model: config.model.clone(),
            max_tokens: config.max_tokens,
            reasoning_effort: config.reasoning_effort.clone(),
        }
    }
}

impl From<PersistedChatConfig> for ChatConfig {
    fn from(config: PersistedChatConfig) -> Self {
        ChatConfig {
            provider: config.provider,
            api_key: String::new(),
            base_url: config.base_url,
            model: config.model,
            max_tokens: config.max_tokens,
            reasoning_effort: config.reasoning_effort,
        }
    }
}

fn config_path() -> PathBuf {
    let home = dirs_fallback();
    home.join(".gitnexus").join(DEFAULT_CONFIG_FILENAME)
}

/// Detect whether `base_url` points at a local LLM endpoint that doesn't
/// require an API key. Centralized so all callers (chat_ask, chat_execute_plan,
/// chat_execute_step) agree on the rule. Previously some callsites only
/// matched `"localhost"` and missed `127.0.0.1`, silently degrading to a
/// graph-only response when Ollama was reachable on the loopback IP.
pub fn is_local_llm_url(base_url: &str) -> bool {
    base_url.contains("localhost")
        || base_url.contains("127.0.0.1")
        || base_url.contains("[::1]")
}

/// Sanitize an LLM error response body before surfacing it in the UI.
///
/// LLM providers occasionally echo the request's Authorization header back
/// in error responses (e.g. "Invalid api key sk-abc..."), and a few echo
/// the full request payload. Truncate aggressively and scrub anything that
/// looks like a bearer token or API key so the user (or application logs)
/// can't accidentally leak credentials when displaying the error toast.
pub(crate) fn sanitize_llm_error_body(body: &str) -> String {
    const MAX_LEN: usize = 300;
    let truncated: String = body.chars().take(MAX_LEN).collect();
    // Replace anything that looks like an API key / bearer token.
    let mut out = String::with_capacity(truncated.len());
    for word in truncated.split_whitespace() {
        let lower = word.to_ascii_lowercase();
        let looks_secret = lower.starts_with("sk-")
            || lower.starts_with("bearer")
            || lower.starts_with("api_key")
            || lower.starts_with("apikey")
            || lower.starts_with("token=")
            || (word.len() > 20 && word.chars().filter(|c| c.is_ascii_alphanumeric()).count() > 18);
        if !out.is_empty() {
            out.push(' ');
        }
        if looks_secret {
            out.push_str("[REDACTED]");
        } else {
            out.push_str(word);
        }
    }
    if body.chars().count() > MAX_LEN {
        out.push_str(" …");
    }
    out
}

fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            tracing::warn!("Neither HOME nor USERPROFILE set, falling back to temp dir for config");
            std::env::temp_dir()
        })
}

fn env_api_key_candidates(provider: &str) -> &'static [&'static str] {
    match provider {
        "openai" => &["OPENAI_API_KEY", "GITNEXUS_API_KEY"],
        "anthropic" => &["ANTHROPIC_API_KEY", "GITNEXUS_API_KEY"],
        "openrouter" => &["OPENROUTER_API_KEY", "GITNEXUS_API_KEY"],
        "gemini" => &["GEMINI_API_KEY", "GOOGLE_API_KEY", "GITNEXUS_API_KEY"],
        _ => &["GITNEXUS_API_KEY"],
    }
}

fn hydrate_api_key_from_env(mut config: ChatConfig) -> ChatConfig {
    if !config.api_key.is_empty() {
        return config;
    }
    for key in env_api_key_candidates(&config.provider) {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                config.api_key = trimmed.to_string();
                break;
            }
        }
    }
    config
}

fn load_persisted_config() -> ChatConfig {
    let path = config_path();
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<PersistedChatConfig>(&content) {
                return hydrate_api_key_from_env(config.into());
            }
        }
    }
    hydrate_api_key_from_env(ChatConfig::default())
}

fn save_config(config: &ChatConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(&PersistedChatConfig::from(config))
        .map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}

async fn load_config(state: &AppState) -> ChatConfig {
    state.chat_config().await.unwrap_or_else(load_persisted_config)
}

// ─── Tauri Commands ──────────────────────────────────────────────────

use gitnexus_core::llm::{LlmProvider, LlmResponseChunk, Message, Role, ToolCall, ToolDefinition, FunctionDefinition};
use gitnexus_core::llm::openai::OpenAILlmProvider;
use futures_util::StreamExt;

/// Execute an agent tool call against the knowledge graph or memory store.
async fn execute_mcp_tool(
    name: &str,
    args: &str,
    repo_path: &Path,
    graph: &KnowledgeGraph,
    fts_index: &FtsIndex,
) -> String {
    match name {
        "search_code" => {
            let query = serde_json::from_str::<serde_json::Value>(args)
                .ok()
                .and_then(|v| v.get("query").and_then(|q| q.as_str()).map(String::from))
                .unwrap_or_default();
            if query.is_empty() {
                return "Error: missing required parameter 'query'".to_string();
            }
            let results = search_relevant_context(&query, graph, fts_index, 8);
            let sources = build_sources(&results, graph, repo_path);
            if sources.is_empty() {
                return format!("No results found for '{}'.", query);
            }
            let mut out = format!("Found {} results for '{}':\n\n", sources.len(), query);
            for (i, src) in sources.iter().enumerate() {
                out.push_str(&format!(
                    "{}. **{}** ({}) — `{}`",
                    i + 1, src.symbol_name, src.symbol_type, src.file_path
                ));
                if let (Some(start), Some(end)) = (src.start_line, src.end_line) {
                    out.push_str(&format!(" L{}-{}", start, end));
                }
                out.push('\n');
                if let Some(callers) = &src.callers {
                    out.push_str(&format!("   Called by: {}\n", callers.join(", ")));
                }
                if let Some(callees) = &src.callees {
                    out.push_str(&format!("   Calls: {}\n", callees.join(", ")));
                }
                if let Some(snippet) = &src.snippet {
                    let short: String = snippet.lines().take(15).collect::<Vec<_>>().join("\n");
                    out.push_str(&format!("   ```\n{}\n   ```\n", short));
                }
                out.push('\n');
            }
            out
        }
        "save_memory" => {
            let parsed = match serde_json::from_str::<serde_json::Value>(args) {
                Ok(v) => v,
                Err(_) => return "Error: invalid JSON arguments for save_memory".to_string(),
            };
            let fact = match parsed.get("fact").and_then(|f| f.as_str()) {
                Some(f) => f,
                None => return "Error: missing required parameter 'fact'".to_string(),
            };
            let scope = match parsed.get("scope").and_then(|s| s.as_str()) {
                Some("global") => gitnexus_core::memory::MemoryScope::Global,
                Some("project") => gitnexus_core::memory::MemoryScope::Project,
                Some(other) => return format!("Error: invalid scope '{}', expected 'global' or 'project'", other),
                None => return "Error: missing required parameter 'scope'".to_string(),
            };
            let mut store = gitnexus_core::memory::MemoryStore::load(scope, Some(repo_path));
            store.add_fact(fact.to_string());
            match store.save(scope, Some(repo_path)) {
                Ok(()) => "Fact saved successfully.".to_string(),
                Err(e) => format!("Failed to save memory: {}", e),
            }
        }
        _ => format!("Error: unknown tool '{}'", name),
    }
}

/// Ask a question about the codebase.
#[tauri::command]
pub async fn chat_ask(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ChatRequest,
) -> Result<ChatResponse, String> {
    let config = load_config(&state).await;

    // 1. Get the active repo's graph and FTS index
    let (graph, _indexes, fts_index, repo_path_str) = state.get_repo(None).await?;
    let repo_path = PathBuf::from(&repo_path_str);

    // 2. Search for relevant symbols
    let search_results = search_relevant_context(&request.question, &graph, &fts_index, 10);

    // 3. Read code snippets for top results
    let sources = build_sources(&search_results, &graph, &repo_path);

    // 4. Assemble the prompt
    let system_prompt = build_system_prompt(&graph, &sources, &repo_path);
    
    // Convert history to gitnexus_core::llm::Message format
    let mut messages = vec![Message {
        role: Role::System,
        content: Some(system_prompt),
        tool_calls: None,
        tool_call_id: None,
        name: None,
    }];

    for msg in request.history.iter().rev().take(10).rev() {
        let role = match msg.role.as_str() {
            "assistant" => Role::Assistant,
            "tool" => Role::Tool,
            _ => Role::User,
        };
        messages.push(Message {
            role,
            content: Some(msg.content.clone()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        });
    }

    messages.push(Message {
        role: Role::User,
        content: Some(request.question.clone()),
        tool_calls: None,
        tool_call_id: None,
        name: None,
    });

    let is_local_llm = is_local_llm_url(&config.base_url);
    if config.api_key.is_empty() && !is_local_llm {
        return Ok(build_graph_only_response(&search_results, &sources, &graph));
    }

    let provider = OpenAILlmProvider::new(
        config.base_url.clone(),
        config.api_key.clone(),
        config.model.clone(),
        config.max_tokens,
        config.reasoning_effort.clone(),
    )?;

    // Define mock tools for the agent
    let tools = vec![
        ToolDefinition {
            type_: "function".to_string(),
            function: FunctionDefinition {
                name: "search_code".to_string(),
                description: "Search the codebase for specific patterns or symbols".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" }
                    },
                    "required": ["query"]
                }),
            }
        },
        ToolDefinition {
            type_: "function".to_string(),
            function: FunctionDefinition {
                name: "save_memory".to_string(),
                description: "Persists preferences or facts across ALL future sessions. Supports two scopes: 'global' for cross-project preferences, and 'project' for facts specific to the current workspace.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "fact": { "type": "string", "description": "The fact to remember" },
                        "scope": { "type": "string", "enum": ["global", "project"], "description": "Scope of the memory" }
                    },
                    "required": ["fact", "scope"]
                }),
            }
        }
    ];

    let mut final_answer = String::new();
    let mut iteration = 0;
    const MAX_ITERATIONS: usize = 5;

    while iteration < MAX_ITERATIONS {
        iteration += 1;
        let mut tool_calls_received: Vec<ToolCall> = Vec::new();
        let mut text_received = String::new();

        let mut stream = match provider.stream_completion(&messages, &tools).await {
            Ok(s) => s,
            Err(e) => {
                if iteration == 1 {
                    // Fall back on first iteration if error
                    let _ = app.emit("chat-stream-done", ());
                    let mut fallback = build_graph_only_response(&search_results, &sources, &graph);
                    fallback.answer = format!(
                        "> **Note:** LLM unavailable ({}). Showing graph-based results.\n\n{}",
                        e, fallback.answer
                    );
                    return Ok(fallback);
                } else {
                    return Err(format!("LLM failed in loop: {}", e));
                }
            }
        };

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(LlmResponseChunk::Text(text)) => {
                    final_answer.push_str(&text);
                    text_received.push_str(&text);
                    let _ = app.emit("chat-stream-chunk", text);
                }
                Ok(LlmResponseChunk::ToolCall(tc)) => {
                    tool_calls_received.push(tc);
                }
                Err(e) => {
                    tracing::error!("Stream error: {}", e);
                }
            }
        }

        if !text_received.is_empty() || !tool_calls_received.is_empty() {
            messages.push(Message {
                role: Role::Assistant,
                content: if text_received.is_empty() { None } else { Some(text_received) },
                tool_calls: if tool_calls_received.is_empty() { None } else { Some(tool_calls_received.clone()) },
                tool_call_id: None,
                name: None,
            });
        }

        if tool_calls_received.is_empty() {
            break; // Done, no tools called
        }

        for tc in tool_calls_received {
            let _ = app.emit("tool_execution_start", tc.name.clone());
            let result = execute_mcp_tool(&tc.name, &tc.arguments, &repo_path, &graph, &fts_index).await;
            let _ = app.emit("tool_execution_end", tc.name.clone());
            
            messages.push(Message {
                role: Role::Tool,
                content: Some(result),
                tool_calls: None,
                tool_call_id: Some(tc.id.clone()),
                name: Some(tc.name.clone()),
            });
        }
    }

    if iteration >= MAX_ITERATIONS {
        final_answer.push_str("\n\n> *Agent reached the maximum number of iterations. The response may be incomplete.*");
    }

    let _ = app.emit("chat-stream-done", ());

    Ok(ChatResponse {
        answer: final_answer,
        sources,
        model: Some(config.model.clone()),
    })
}

/// Get the current chat configuration.
#[tauri::command]
pub async fn chat_get_config(state: State<'_, AppState>) -> Result<ChatConfig, String> {
    Ok(load_config(&state).await)
}

/// Save chat configuration (LLM provider settings).
#[tauri::command]
pub async fn chat_set_config(
    state: State<'_, AppState>,
    config: ChatConfig,
) -> Result<(), String> {
    state.set_chat_config(config.clone()).await;
    save_config(&config)
}

// ─── Public Helpers (used by chat_executor) ─────────────────────────

/// Public config loader for the executor module.
pub async fn load_config_pub(state: &AppState) -> ChatConfig {
    load_config(state).await
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
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or_else(|| {
        // Handle NaN: treat NaN as less than any number
        if a.1.is_nan() { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Less }
    }));
    results.truncate(max_results);
    results
}

/// Build source citations with code snippets.
fn build_sources(
    results: &[(String, f64)],
    graph: &KnowledgeGraph,
    repo_path: &Path,
) -> Vec<ChatSource> {
    let mut sources = Vec::new();

    for (node_id, score) in results {
        let node = match graph.get_node(node_id) {
            Some(n) => n,
            None => continue,
        };

        // Skip non-code nodes (Community, Process, File, etc.) except DocChunks
        match node.label {
            NodeLabel::Function
            | NodeLabel::Method
            | NodeLabel::Constructor
            | NodeLabel::Class
            | NodeLabel::Struct
            | NodeLabel::Trait
            | NodeLabel::Interface
            | NodeLabel::Enum
            | NodeLabel::TypeAlias
            | NodeLabel::DocChunk => {}
            _ => continue,
        }

        // Try to read a code snippet or use DocChunk content
        let snippet = if node.label == NodeLabel::DocChunk {
            node.properties.content.clone()
        } else {
            read_code_snippet(repo_path, &node.properties.file_path, node.properties.start_line, node.properties.end_line)
        };

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
    repo_path: &Path,
    file_path: &str,
    start_line: Option<u32>,
    end_line: Option<u32>,
) -> Option<String> {
    let full_path = repo_path.join(file_path);
    // Path traversal guard: ensure the resolved path stays inside the repo.
    // The graph node `file_path` field is sourced from snapshots that may
    // contain `..` segments (a corrupted or hand-crafted graph), and without
    // this check the snippet would happily include arbitrary files from the
    // host filesystem in the LLM prompt context.
    let canonical_repo = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());
    match full_path.canonicalize() {
        Ok(canonical) if !canonical.starts_with(&canonical_repo) => {
            tracing::warn!("read_code_snippet: path traversal blocked: {}", file_path);
            return None;
        }
        Err(_) => return None, // file doesn't exist or permission denied
        _ => {}
    }
    let content = std::fs::read_to_string(&full_path).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    match (start_line, end_line) {
        (Some(start), Some(end)) => {
            let start = std::cmp::min((start.saturating_sub(1)) as usize, lines.len());
            let end = std::cmp::min(end as usize, lines.len());
            let end = std::cmp::min(end, start + 50);
            if start >= end { return None; }
            Some(lines[start..end].join("\n"))
        }
        (Some(start), None) => {
            let start = std::cmp::min((start.saturating_sub(1)) as usize, lines.len());
            let end = std::cmp::min(start + 20, lines.len());
            if start >= end { return None; }
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
fn build_system_prompt(graph: &KnowledgeGraph, sources: &[ChatSource], repo_path: &Path) -> String {
    let node_count = graph.iter_nodes().count();
    let edge_count = graph.iter_relationships().count();
    
    let memory_context = gitnexus_core::memory::build_memory_context(Some(repo_path));

    let mut prompt = format!(
        r#"You are an expert code analyst helping a developer understand a codebase.
You have access to a knowledge graph of this repository ({} symbols, {} relationships).

{}

## Relevant Code Context

The following symbols and code snippets are the most relevant to the user's question:

"#,
        node_count, edge_count, memory_context
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

/// Build the common request components for LLM API calls.
fn build_llm_request(
    config: &ChatConfig,
    messages: &[serde_json::Value],
    stream: bool,
) -> Result<(reqwest::Client, reqwest::RequestBuilder), String> {
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

    let mut body = serde_json::json!({
        "model": config.model,
        "messages": messages,
        "max_tokens": config.max_tokens,
        "temperature": 0.3,
        "stream": stream
    });

    // Add reasoning_effort for models that support thinking (e.g. Gemini)
    let effort = config.reasoning_effort.trim().to_lowercase();
    if !effort.is_empty() && effort != "none" {
        body["reasoning_effort"] = serde_json::Value::String(effort);
    }

    let timeout_secs = if stream { 300 } else { 120 };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut request = client.post(&url).json(&body);

    // Add authorization header if API key is provided
    if !config.api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", config.api_key));
    }

    Ok((client, request))
}



/// Call an OpenAI-compatible LLM API (non-streaming, for the executor module).
async fn call_llm(
    config: &ChatConfig,
    messages: &[serde_json::Value],
) -> Result<String, String> {
    let (_client, request) = build_llm_request(config, messages, false)?;

    let response = request
        .send()
        .await
        .map_err(|e| format!("LLM API request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!(
            "LLM API error ({}): {}",
            status,
            sanitize_llm_error_body(&error_text)
        ));
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
    // Build score lookup from search results
    let score_map: std::collections::HashMap<&str, f64> = results
        .iter()
        .map(|(id, score)| (id.as_str(), *score))
        .collect();

    let mut answer = String::from("## Relevant Symbols Found\n\n");
    answer.push_str(&format!(
        "*No LLM configured — showing graph search results ({} nodes, {} relationships). Configure an API key in Settings to get AI-powered answers.*\n\n",
        graph.node_count(),
        graph.relationship_count(),
    ));

    for source in sources {
        let score_str = score_map
            .get(source.node_id.as_str())
            .map(|s| format!(" (score: {:.2})", s))
            .unwrap_or_default();
        answer.push_str(&format!(
            "### `{}` ({}){} — `{}`\n",
            source.symbol_name, source.symbol_type, score_str, source.file_path
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
            let lang = if source.symbol_type == "DocChunk" { "markdown" } else { detect_language(&source.file_path) };
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

#[cfg(test)]
mod tests {
    use super::*;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn sample_config() -> ChatConfig {
        ChatConfig {
            provider: "openai".to_string(),
            api_key: "sk-secret-value".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            max_tokens: 1234,
            reasoning_effort: "high".to_string(),
        }
    }

    #[test]
    fn persisted_config_never_contains_api_key() {
        let persisted = PersistedChatConfig::from(&sample_config());
        let json = serde_json::to_string(&persisted).unwrap();

        assert!(!json.contains("apiKey"));
        assert!(!json.contains("sk-secret-value"));
        assert!(json.contains("provider"));
    }

    #[test]
    fn hydrate_api_key_uses_provider_specific_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("GITNEXUS_API_KEY");
        std::env::set_var("OPENAI_API_KEY", "sk-env-value");

        let hydrated = hydrate_api_key_from_env(ChatConfig {
            api_key: String::new(),
            ..sample_config()
        });

        assert_eq!(hydrated.api_key, "sk-env-value");
        std::env::remove_var("OPENAI_API_KEY");
    }

    #[test]
    fn hydrate_api_key_does_not_override_explicit_secret() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("OPENAI_API_KEY", "sk-env-value");

        let hydrated = hydrate_api_key_from_env(sample_config());

        assert_eq!(hydrated.api_key, "sk-secret-value");
        std::env::remove_var("OPENAI_API_KEY");
    }
}
