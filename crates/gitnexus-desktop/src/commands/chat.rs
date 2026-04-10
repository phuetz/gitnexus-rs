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
use crate::types::{ChatConfig, ChatMessage, ChatRequest, ChatResponse, ChatSource};

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
    let system_prompt = build_system_prompt(&graph, &sources);
    let messages = build_llm_messages(&system_prompt, &request.history, &request.question);

    // 5. Call LLM if configured.
    // Skip LLM when no API key AND not targeting a local server (localhost).
    // Local servers (e.g. Ollama, LM Studio) typically don't need an API key.
    let is_local_llm = is_local_llm_url(&config.base_url);
    if config.api_key.is_empty() && !is_local_llm {
        return Ok(build_graph_only_response(&search_results, &sources, &graph));
    }

    match call_llm_streaming(&config, &messages, &app).await {
        Ok(answer) => Ok(ChatResponse {
            answer,
            sources,
            model: Some(config.model.clone()),
        }),
        Err(llm_error) => {
            // LLM failed — fall back to graph-only response
            let _ = app.emit("chat-stream-done", ());
            let mut fallback = build_graph_only_response(&search_results, &sources, &graph);
            fallback.answer = format!(
                "> **Note:** LLM unavailable ({}). Showing graph-based results.\n\n{}",
                llm_error, fallback.answer
            );
            Ok(fallback)
        }
    }
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

/// Call an OpenAI-compatible LLM API with SSE streaming.
/// Emits `chat-stream-chunk` events to the frontend as each token arrives.
async fn call_llm_streaming(
    config: &ChatConfig,
    messages: &[serde_json::Value],
    app_handle: &AppHandle,
) -> Result<String, String> {
    let (_client, request) = build_llm_request(config, messages, true)?;

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

    let mut full_text = String::new();
    let mut stream = response.bytes_stream();
    // Buffer raw bytes (not String) so multi-byte UTF-8 sequences that span
    // chunk boundaries are not corrupted by `from_utf8_lossy`. Lines are decoded
    // only after a complete `\n` is in the buffer.
    let mut byte_buffer: Vec<u8> = Vec::new();
    const MAX_LINE_BUFFER: usize = 1_048_576; // 1MB safety cap for a single partial line

    use futures_util::StreamExt;

    let process_line = |line: &str, full_text: &mut String, app_handle: &AppHandle| {
        if let Some(data) = line.strip_prefix("data: ") {
            let data = data.trim();
            if data == "[DONE]" {
                return;
            }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(delta) = json["choices"][0]["delta"]["content"].as_str() {
                    if !delta.is_empty() {
                        full_text.push_str(delta);
                        let _ = app_handle.emit("chat-stream-chunk", delta);
                    }
                }
            }
        }
    };

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
        byte_buffer.extend_from_slice(&chunk);

        // Drain complete lines (terminated by `\n`) from the buffer.
        while let Some(newline_pos) = byte_buffer.iter().position(|&b| b == b'\n') {
            let line_bytes: Vec<u8> = byte_buffer.drain(..=newline_pos).collect();
            // Strip the trailing `\n` (and optional `\r`) before decoding.
            let mut end = line_bytes.len() - 1;
            if end > 0 && line_bytes[end - 1] == b'\r' {
                end -= 1;
            }
            // Decode lossily — at this point we have a complete line so any
            // remaining replacement characters represent genuine bad bytes,
            // not chunk-boundary artifacts.
            let line = String::from_utf8_lossy(&line_bytes[..end]);
            process_line(&line, &mut full_text, app_handle);
        }

        // Cap the *residual* partial line length, not the total buffer.
        // Checking total-buffer-size before draining (as the previous code did)
        // falsely aborts a legitimate chunk that happens to contain many
        // complete lines summing to >1MB, because the check fires before the
        // drain gets a chance to shrink the buffer. Moving the check after the
        // drain loop makes the cap measure what it's actually meant to limit:
        // an unbounded single line from a misbehaving server that never emits
        // a newline.
        if byte_buffer.len() > MAX_LINE_BUFFER {
            return Err("SSE stream partial line exceeded 1MB — aborting".to_string());
        }
    }

    // Process any trailing line without a final newline.
    if !byte_buffer.is_empty() {
        let line = String::from_utf8_lossy(&byte_buffer);
        let trimmed = line.trim();
        process_line(trimmed, &mut full_text, app_handle);
    }

    // Signal stream completion
    let _ = app_handle.emit("chat-stream-done", ());

    if full_text.is_empty() {
        return Err("No content received from LLM stream".to_string());
    }

    Ok(full_text)
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
