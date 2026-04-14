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
    #[serde(alias = "base_url")]
    base_url: String,
    model: String,
    #[serde(alias = "max_tokens")]
    max_tokens: u32,
    #[serde(default, alias = "reasoning_effort")]
    reasoning_effort: String,
    /// Optional API key persisted in the file (CLI compatibility).
    /// If absent, will be loaded from environment variables.
    #[serde(default, alias = "api_key", alias = "apiKey")]
    api_key: String,
}

impl From<&ChatConfig> for PersistedChatConfig {
    fn from(config: &ChatConfig) -> Self {
        Self {
            provider: config.provider.clone(),
            base_url: config.base_url.clone(),
            model: config.model.clone(),
            max_tokens: config.max_tokens,
            reasoning_effort: config.reasoning_effort.clone(),
            api_key: String::new(),
        }
    }
}

impl From<PersistedChatConfig> for ChatConfig {
    fn from(config: PersistedChatConfig) -> Self {
        ChatConfig {
            provider: config.provider,
            api_key: config.api_key,
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
    indexes: &gitnexus_db::inmemory::cypher::GraphIndexes,
    fts_index: &FtsIndex,
) -> String {
    let parsed = serde_json::from_str::<serde_json::Value>(args).unwrap_or_default();

    match name {
        // ── search_code ──────────────────────────────────────────
        "search_code" => {
            let query = parsed.get("query").and_then(|q| q.as_str()).unwrap_or_default();
            if query.is_empty() {
                return "Error: missing required parameter 'query'".to_string();
            }
            let results = search_relevant_context(query, graph, fts_index, 8);
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

        // ── read_file ────────────────────────────────────────────
        "read_file" => {
            let path = match parsed.get("path").and_then(|p| p.as_str()) {
                Some(p) => p,
                None => return "Error: missing required parameter 'path'".to_string(),
            };
            let start = parsed.get("start_line").and_then(|v| v.as_u64()).map(|v| v as u32);
            let end = parsed.get("end_line").and_then(|v| v.as_u64()).map(|v| v as u32);
            match read_code_snippet(repo_path, path, start, end) {
                Some(content) => {
                    let lang = detect_language(path);
                    format!("File `{}` (lines {}-{}):\n```{}\n{}\n```", path,
                        start.unwrap_or(1), end.unwrap_or(start.unwrap_or(1) + 30),
                        lang, content)
                }
                None => format!("Error: could not read file '{}' (not found or outside repo)", path),
            }
        }

        // ── get_impact ───────────────────────────────────────────
        "get_impact" => {
            let target = match parsed.get("target").and_then(|t| t.as_str()) {
                Some(t) => t,
                None => return "Error: missing required parameter 'target'".to_string(),
            };
            let direction = parsed.get("direction").and_then(|d| d.as_str()).unwrap_or("both");
            let max_depth = parsed.get("max_depth").and_then(|d| d.as_u64()).unwrap_or(3) as u32;

            // Resolve target: try exact node ID first, then name search
            let target_id = if graph.get_node(target).is_some() {
                target.to_string()
            } else {
                let target_lower = target.to_lowercase();
                match graph.iter_nodes().find(|n| n.properties.name.to_lowercase() == target_lower) {
                    Some(n) => n.id.clone(),
                    None => return format!("Error: symbol '{}' not found", target),
                }
            };

            let upstream = if direction == "upstream" || direction == "both" {
                crate::commands::impact::bfs_impact_pub(graph, indexes, &target_id, max_depth, true)
            } else {
                Vec::new()
            };
            let downstream = if direction == "downstream" || direction == "both" {
                crate::commands::impact::bfs_impact_pub(graph, indexes, &target_id, max_depth, false)
            } else {
                Vec::new()
            };

            let mut out = format!("Impact analysis for '{}' (depth={}, direction={}):\n\n", target, max_depth, direction);
            if !upstream.is_empty() {
                out.push_str(&format!("**Upstream ({} affected):**\n", upstream.len()));
                for n in upstream.iter().take(20) {
                    out.push_str(&format!("  - {} ({}) at depth {} — `{}`\n",
                        n.node.name, n.node.label, n.depth, n.node.file_path));
                }
                if upstream.len() > 20 { out.push_str(&format!("  ... and {} more\n", upstream.len() - 20)); }
                out.push('\n');
            }
            if !downstream.is_empty() {
                out.push_str(&format!("**Downstream ({} affected):**\n", downstream.len()));
                for n in downstream.iter().take(20) {
                    out.push_str(&format!("  - {} ({}) at depth {} — `{}`\n",
                        n.node.name, n.node.label, n.depth, n.node.file_path));
                }
                if downstream.len() > 20 { out.push_str(&format!("  ... and {} more\n", downstream.len() - 20)); }
            }
            if upstream.is_empty() && downstream.is_empty() {
                out.push_str("No impact found — this symbol has no causal dependencies.\n");
            }
            out
        }

        // ── get_symbol_context ───────────────────────────────────
        "get_symbol_context" => {
            let symbol = match parsed.get("symbol").and_then(|s| s.as_str()) {
                Some(s) => s,
                None => return "Error: missing required parameter 'symbol'".to_string(),
            };

            // Resolve: exact ID or name search
            let node_id = if graph.get_node(symbol).is_some() {
                symbol.to_string()
            } else {
                let sym_lower = symbol.to_lowercase();
                match graph.iter_nodes().find(|n| n.properties.name.to_lowercase() == sym_lower) {
                    Some(n) => n.id.clone(),
                    None => return format!("Error: symbol '{}' not found", symbol),
                }
            };

            let node = match graph.get_node(&node_id) {
                Some(n) => n,
                None => return format!("Error: node '{}' not found in graph", node_id),
            };
            let mut out = format!("**{}** ({}) — `{}`\n\n", node.properties.name, node.label.as_str(), node.properties.file_path);

            // Callers/callees via indexes
            let mut callers = Vec::new();
            let mut callees = Vec::new();
            let mut imports = Vec::new();
            let mut inherited = Vec::new();

            if let Some(outs) = indexes.outgoing.get(&node_id) {
                for (tid, rtype) in outs {
                    if let Some(t) = graph.get_node(tid) {
                        match rtype {
                            RelationshipType::Calls => callees.push(format!("{} ({})", t.properties.name, t.label.as_str())),
                            RelationshipType::Imports => imports.push(t.properties.name.clone()),
                            RelationshipType::Inherits | RelationshipType::Extends | RelationshipType::Implements =>
                                inherited.push(format!("{} ({})", t.properties.name, t.label.as_str())),
                            _ => {}
                        }
                    }
                }
            }
            if let Some(ins) = indexes.incoming.get(&node_id) {
                for (sid, rtype) in ins {
                    if let Some(s) = graph.get_node(sid) {
                        if *rtype == RelationshipType::Calls {
                            callers.push(format!("{} ({})", s.properties.name, s.label.as_str()));
                        }
                    }
                }
            }

            if !callers.is_empty() { out.push_str(&format!("**Called by:** {}\n", callers.join(", "))); }
            if !callees.is_empty() { out.push_str(&format!("**Calls:** {}\n", callees.join(", "))); }
            if !imports.is_empty() { out.push_str(&format!("**Imports:** {}\n", imports.join(", "))); }
            if !inherited.is_empty() { out.push_str(&format!("**Inherits/Implements:** {}\n", inherited.join(", "))); }

            // Community
            if let Some(outs) = indexes.outgoing.get(&node_id) {
                for (tid, rtype) in outs {
                    if *rtype == RelationshipType::MemberOf {
                        if let Some(c) = graph.get_node(tid) {
                            out.push_str(&format!("**Module:** {}\n", c.properties.name));
                            break;
                        }
                    }
                }
            }
            out
        }

        // ── execute_cypher ───────────────────────────────────────
        "execute_cypher" => {
            let query = match parsed.get("query").and_then(|q| q.as_str()) {
                Some(q) => q,
                None => return "Error: missing required parameter 'query'".to_string(),
            };
            match gitnexus_db::inmemory::cypher::parse(query) {
                Ok(stmt) => {
                    match gitnexus_db::inmemory::cypher::execute(&stmt, graph, indexes, fts_index) {
                        Ok(rows) => {
                            if rows.is_empty() {
                                "Query returned 0 results.".to_string()
                            } else {
                                let truncated: Vec<_> = rows.iter().take(25).collect();
                                let json = serde_json::to_string_pretty(&truncated).unwrap_or_default();
                                format!("Cypher returned {} results{}:\n```json\n{}\n```",
                                    rows.len(),
                                    if rows.len() > 25 { " (showing first 25)" } else { "" },
                                    json)
                            }
                        }
                        Err(e) => format!("Cypher execution error: {}", e),
                    }
                }
                Err(e) => format!("Cypher parse error: {}", e),
            }
        }

        // ── get_diagram ──────────────────────────────────────────
        "get_diagram" => {
            let target = match parsed.get("target").and_then(|t| t.as_str()) {
                Some(t) => t,
                None => return "Error: missing required parameter 'target'".to_string(),
            };
            let target_lower = target.to_lowercase();
            let start_node = match graph.iter_nodes().find(|n| n.properties.name.to_lowercase() == target_lower) {
                Some(n) => n,
                None => return format!("Error: symbol '{}' not found", target),
            };

            let node_id = &start_node.id;
            let mut lines = vec!["graph TD".to_string()];
            lines.push(format!("    {}[\"{}\"]", diagram_sanitize(node_id), diagram_escape(&start_node.properties.name)));

            let empty: Vec<(String, RelationshipType)> = Vec::new();
            let outgoing = indexes.outgoing.get(node_id).unwrap_or(&empty);
            let methods: Vec<String> = outgoing.iter()
                .filter(|(_, rt)| matches!(rt, RelationshipType::HasMethod | RelationshipType::HasAction))
                .map(|(tid, _)| tid.clone()).collect();

            for mid in &methods {
                if let Some(m) = graph.get_node(mid) {
                    lines.push(format!("    {} --> {}[\"{}\"]", diagram_sanitize(node_id), diagram_sanitize(mid), diagram_escape(&m.properties.name)));
                    if let Some(m_outs) = indexes.outgoing.get(mid) {
                        for (cid, rt) in m_outs {
                            if matches!(rt, RelationshipType::Calls | RelationshipType::CallsAction | RelationshipType::CallsService) {
                                if let Some(callee) = graph.get_node(cid) {
                                    lines.push(format!("    {} --> {}[\"{}\"]", diagram_sanitize(mid), diagram_sanitize(cid), diagram_escape(&callee.properties.name)));
                                }
                            }
                        }
                    }
                }
            }
            if methods.is_empty() {
                for (tid, rt) in outgoing {
                    if matches!(rt, RelationshipType::Calls | RelationshipType::Imports | RelationshipType::DependsOn) {
                        if let Some(t) = graph.get_node(tid) {
                            lines.push(format!("    {} -->|{}| {}[\"{}\"]", diagram_sanitize(node_id), rt.as_str(), diagram_sanitize(tid), diagram_escape(&t.properties.name)));
                        }
                    }
                }
            }
            format!("Mermaid diagram for '{}':\n```mermaid\n{}\n```", target, lines.join("\n"))
        }

        // ── save_memory ──────────────────────────────────────────
        "save_memory" => {
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

fn diagram_sanitize(id: &str) -> String {
    id.replace([':', '/', '.', ' ', '<', '>', '(', ')', '{', '}'], "_")
}

fn diagram_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
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
    let (graph, indexes, fts_index, repo_path_str) = state.get_repo(None).await?;
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
                description: "Search the codebase for symbols, functions, classes, or patterns. Returns matching symbols with code snippets.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query (symbol name, keyword, or pattern)" }
                    },
                    "required": ["query"]
                }),
            }
        },
        ToolDefinition {
            type_: "function".to_string(),
            function: FunctionDefinition {
                name: "read_file".to_string(),
                description: "Read source code from a file in the repository. Use when you need to see the actual implementation of a symbol.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Relative file path (e.g. 'src/main.rs')" },
                        "start_line": { "type": "number", "description": "Start line (1-based, optional)" },
                        "end_line": { "type": "number", "description": "End line (optional, max 50 lines)" }
                    },
                    "required": ["path"]
                }),
            }
        },
        ToolDefinition {
            type_: "function".to_string(),
            function: FunctionDefinition {
                name: "get_impact".to_string(),
                description: "Blast radius analysis: find all symbols affected if a given symbol changes. Uses BFS on causal edges (Calls, Imports, Inherits, etc.).".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "target": { "type": "string", "description": "Symbol name or node ID to analyze" },
                        "direction": { "type": "string", "enum": ["upstream", "downstream", "both"], "description": "Direction of impact (default: both)" },
                        "max_depth": { "type": "number", "description": "Max BFS depth (default: 3)" }
                    },
                    "required": ["target"]
                }),
            }
        },
        ToolDefinition {
            type_: "function".to_string(),
            function: FunctionDefinition {
                name: "get_symbol_context".to_string(),
                description: "360-degree context for a symbol: callers, callees, imports, inheritance, and module membership.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "symbol": { "type": "string", "description": "Symbol name or node ID" }
                    },
                    "required": ["symbol"]
                }),
            }
        },
        ToolDefinition {
            type_: "function".to_string(),
            function: FunctionDefinition {
                name: "execute_cypher".to_string(),
                description: "Execute a read-only Cypher query against the knowledge graph. Only MATCH and CALL statements are allowed.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Cypher query (e.g. MATCH (n:Class) RETURN n.name LIMIT 10)" }
                    },
                    "required": ["query"]
                }),
            }
        },
        ToolDefinition {
            type_: "function".to_string(),
            function: FunctionDefinition {
                name: "get_diagram".to_string(),
                description: "Generate a Mermaid flowchart diagram showing the methods and call relationships of a class/controller.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "target": { "type": "string", "description": "Symbol name (class, controller, service)" }
                    },
                    "required": ["target"]
                }),
            }
        },
        ToolDefinition {
            type_: "function".to_string(),
            function: FunctionDefinition {
                name: "save_memory".to_string(),
                description: "Persist a fact or preference across ALL future sessions. Use 'global' for cross-project preferences, 'project' for workspace-specific facts.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "fact": { "type": "string", "description": "The fact to remember" },
                        "scope": { "type": "string", "enum": ["global", "project"], "description": "Scope of the memory" }
                    },
                    "required": ["fact", "scope"]
                }),
            }
        },
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
            let result = execute_mcp_tool(&tc.name, &tc.arguments, &repo_path, &graph, &indexes, &fts_index).await;
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
/// Project-level metadata gathered from the graph for system prompt context.
struct ProjectMeta {
    node_count: usize,
    edge_count: usize,
    community_count: usize,
    process_count: usize,
    top_languages: Vec<(String, usize)>,
    frameworks: Vec<&'static str>,
    repo_name: String,
}

/// Functional community summary for the system prompt.
struct CommunitySummary {
    label: String,
    member_count: u32,
    description: Option<String>,
    keywords: Option<Vec<String>>,
}

/// Business process summary for the system prompt.
struct ProcessSummary {
    name: String,
    step_count: u32,
}

/// Compute project metadata: counts per label, frameworks, top languages.
fn gather_project_meta(graph: &KnowledgeGraph, repo_path: &Path) -> ProjectMeta {
    use gitnexus_core::graph::types::NodeLabel;
    use std::collections::HashMap;

    let mut by_label: HashMap<NodeLabel, usize> = HashMap::new();
    let mut by_lang: HashMap<String, usize> = HashMap::new();

    for node in graph.iter_nodes() {
        *by_label.entry(node.label).or_insert(0) += 1;
        if let Some(lang) = &node.properties.language {
            *by_lang.entry(format!("{:?}", lang)).or_insert(0) += 1;
        }
    }

    // Detect frameworks based on presence of specific node types
    let mut frameworks: Vec<&'static str> = Vec::new();
    if by_label.get(&NodeLabel::Controller).copied().unwrap_or(0) > 0 {
        frameworks.push("ASP.NET MVC");
    }
    if by_label.get(&NodeLabel::DbContext).copied().unwrap_or(0) > 0 {
        frameworks.push("Entity Framework");
    }
    if by_label.get(&NodeLabel::View).copied().unwrap_or(0) > 0
        && !frameworks.contains(&"ASP.NET MVC")
    {
        frameworks.push("MVC Views");
    }

    // Top 3 languages
    let mut langs: Vec<(String, usize)> = by_lang.into_iter().collect();
    langs.sort_by(|a, b| b.1.cmp(&a.1));
    langs.truncate(3);

    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    ProjectMeta {
        node_count: graph.iter_nodes().count(),
        edge_count: graph.iter_relationships().count(),
        community_count: by_label.get(&NodeLabel::Community).copied().unwrap_or(0),
        process_count: by_label.get(&NodeLabel::Process).copied().unwrap_or(0),
        top_languages: langs,
        frameworks,
        repo_name,
    }
}

/// Top N functional communities sorted by member count.
fn top_communities(graph: &KnowledgeGraph, limit: usize) -> Vec<CommunitySummary> {
    use gitnexus_core::graph::types::NodeLabel;

    let mut communities: Vec<CommunitySummary> = graph
        .iter_nodes()
        .filter(|n| n.label == NodeLabel::Community)
        .map(|n| {
            let label = n
                .properties
                .heuristic_label
                .clone()
                .unwrap_or_else(|| n.properties.name.clone());
            CommunitySummary {
                label,
                member_count: n.properties.symbol_count.unwrap_or(0),
                description: n.properties.description.clone(),
                keywords: n.properties.keywords.clone(),
            }
        })
        .collect();

    communities.sort_by(|a, b| b.member_count.cmp(&a.member_count));
    communities.truncate(limit);
    communities
}

/// Top N business processes sorted by step count.
fn top_processes(graph: &KnowledgeGraph, limit: usize) -> Vec<ProcessSummary> {
    use gitnexus_core::graph::types::NodeLabel;

    let mut processes: Vec<ProcessSummary> = graph
        .iter_nodes()
        .filter(|n| n.label == NodeLabel::Process)
        .map(|n| ProcessSummary {
            name: n.properties.name.clone(),
            step_count: n.properties.step_count.unwrap_or(0),
        })
        .collect();

    processes.sort_by(|a, b| b.step_count.cmp(&a.step_count));
    processes.truncate(limit);
    processes
}

fn build_system_prompt(graph: &KnowledgeGraph, sources: &[ChatSource], repo_path: &Path) -> String {
    let meta = gather_project_meta(graph, repo_path);
    let communities = top_communities(graph, 10);
    let processes = top_processes(graph, 5);
    let memory_context = gitnexus_core::memory::build_memory_context(Some(repo_path));

    let mut prompt = String::with_capacity(8192);

    // ── Identity ────────────────────────────────────────────────
    prompt.push_str(&format!(
        "# Identity\n\n\
         You are GitNexus, an AI code intelligence assistant specialized in analyzing \
         the **{}** codebase using a structured knowledge graph.\n\n",
        meta.repo_name
    ));

    // ── Project context ─────────────────────────────────────────
    prompt.push_str("# Project context\n\n");
    prompt.push_str(&format!("- **Repository**: `{}`\n", meta.repo_name));
    if !meta.top_languages.is_empty() {
        let langs = meta
            .top_languages
            .iter()
            .map(|(l, c)| format!("{} ({})", l, c))
            .collect::<Vec<_>>()
            .join(", ");
        prompt.push_str(&format!("- **Languages**: {}\n", langs));
    }
    if !meta.frameworks.is_empty() {
        prompt.push_str(&format!("- **Frameworks detected**: {}\n", meta.frameworks.join(", ")));
    }
    prompt.push_str(&format!(
        "- **Graph statistics**: {} symbols, {} relationships, {} functional modules, {} business processes\n\n",
        meta.node_count, meta.edge_count, meta.community_count, meta.process_count
    ));

    // ── Knowledge graph schema ──────────────────────────────────
    prompt.push_str(
        "# Knowledge graph schema\n\n\
         Node types you can reason about:\n\
         - **Code**: `Class`, `Method`, `Function`, `Interface`, `Enum`, `Struct`, `Constructor`\n\
         - **ASP.NET**: `Controller`, `ControllerAction`, `View`, `ViewModel`, `DbContext`, `DbEntity`, `Service`, `Repository`\n\
         - **Architecture**: `Module`, `Community` (functional groups), `Process` (business workflows)\n\n\
         Relationships you can traverse:\n\
         - **Calls** (method→method), **HasMethod** (class→method), **Inherits**, **Implements**\n\
         - **RendersView** (controller→view), **MapsToEntity** (view→entity), **CallsService**\n\
         - **HandlesRoute** (action→route), **MemberOf** (symbol→community), **StepInProcess** (method→process)\n\n",
    );

    // ── Functional modules ──────────────────────────────────────
    if !communities.is_empty() {
        prompt.push_str("# Functional modules (top by size)\n\n");
        for c in &communities {
            prompt.push_str(&format!("- **{}** ({} symbols)", c.label, c.member_count));
            if let Some(desc) = &c.description {
                let short = desc.chars().take(120).collect::<String>();
                prompt.push_str(&format!(" — {}", short));
            } else if let Some(kw) = &c.keywords {
                if !kw.is_empty() {
                    prompt.push_str(&format!(" — keywords: {}", kw.join(", ")));
                }
            }
            prompt.push('\n');
        }
        prompt.push('\n');
    }

    // ── Business processes ──────────────────────────────────────
    if !processes.is_empty() {
        prompt.push_str("# Business processes detected\n\n");
        for p in &processes {
            prompt.push_str(&format!("- **{}** — {} steps\n", p.name, p.step_count));
        }
        prompt.push('\n');
    }

    // ── Persistent memory ───────────────────────────────────────
    if !memory_context.trim().is_empty() {
        prompt.push_str("# Persistent memory\n\n");
        prompt.push_str(&memory_context);
        prompt.push_str("\n\n");
    }

    // ── Relevant code context ───────────────────────────────────
    if !sources.is_empty() {
        prompt.push_str("# Relevant code context\n\n");
        prompt.push_str("These symbols are the most relevant to the user's question (ranked by FTS score):\n\n");

        for (i, source) in sources.iter().enumerate() {
            prompt.push_str(&format!(
                "## {} — `{}` ({}) in `{}`\n",
                i + 1,
                source.symbol_name,
                source.symbol_type,
                source.file_path
            ));

            if let Some(community) = &source.community {
                prompt.push_str(&format!("- **Module**: {}\n", community));
            }
            if let Some(callers) = &source.callers {
                if !callers.is_empty() {
                    prompt.push_str(&format!("- **Called by**: {}\n", callers.join(", ")));
                }
            }
            if let Some(callees) = &source.callees {
                if !callees.is_empty() {
                    prompt.push_str(&format!("- **Calls**: {}\n", callees.join(", ")));
                }
            }

            if let Some(snippet) = &source.snippet {
                let lang = detect_language(&source.file_path);
                prompt.push_str(&format!("\n```{}\n{}\n```\n\n", lang, snippet));
            }
        }
    }

    // ── Tools strategy ──────────────────────────────────────────
    prompt.push_str(
        "# Available tools — when to use them\n\n\
         You have 7 tools. Use them ONLY when the context above is insufficient. \
         Limit to 2-3 tool calls per response unless the question genuinely requires deeper exploration.\n\n\
         1. **search_code** — Search by keyword when the question mentions a feature/concept not in the context.\n\
         2. **read_file** — Read more of a file when the snippet (~50 lines) is truncated.\n\
         3. **get_symbol_context** — 360° view of a symbol (callers, callees, imports, inheritance). Use for \"how does X work?\".\n\
         4. **get_impact** — Blast radius via BFS. Use for \"what breaks if I change X?\" or \"what depends on X?\".\n\
         5. **execute_cypher** — Read-only graph queries for complex traversals. Examples:\n\
            - `MATCH (c:Controller)-[:CALLS_SERVICE]->(s:Service) RETURN c.name, s.name LIMIT 20`\n\
            - `MATCH (p:Process) WHERE p.name CONTAINS 'Courrier' RETURN p`\n\
         6. **get_diagram** — Generate a Mermaid flowchart for a class/controller (visual call graph).\n\
         7. **save_memory** — Persist a fact when the user states a project convention or preference \
         (\"we always use repository pattern\", \"tests live in /Tests\").\n\n",
    );

    // ── Response format ─────────────────────────────────────────
    prompt.push_str(
        "# Response format\n\n\
         - **Language**: respond in the SAME language as the user's question (French, English, etc.).\n\
         - **Structure**: use markdown sections with `##` headers.\n\
         - **Citations**: cite symbols and files with backticks, e.g. `RegleCourriers.GenerateCourrier()` in `Courrier/RegleCourriers.cs:123`.\n\
         - **Code**: use fenced code blocks with the correct language tag (```csharp, ```typescript, etc.).\n\
         - **Diagrams**: use ```mermaid blocks for call graphs or flows when they aid understanding.\n\
         - **Length**: be concise but complete. Bullet points for lists, prose for explanations.\n\
         - **Honesty**: if you don't know, say so. Do NOT invent symbols, files, or behaviors.\n\
         - **End** with a `## Voir aussi` (or `## See also`) section listing 3-5 related symbols to explore.\n",
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
