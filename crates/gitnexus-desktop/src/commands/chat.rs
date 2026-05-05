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
use gitnexus_mcp::backend::local::LocalBackend;
use gitnexus_search::embeddings::{EmbeddingConfig, EmbeddingStore};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

use crate::state::AppState;
use crate::types::{ChatConfig, ChatRequest, ChatResponse, ChatSearchCapabilities, ChatSource};

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
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        alias = "api_key",
        alias = "apiKey"
    )]
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
    base_url.contains("localhost") || base_url.contains("127.0.0.1") || base_url.contains("[::1]")
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
    state
        .chat_config()
        .await
        .unwrap_or_else(load_persisted_config)
}

// ─── Tauri Commands ──────────────────────────────────────────────────

use futures_util::StreamExt;
use gitnexus_core::llm::openai::OpenAILlmProvider;
use gitnexus_core::llm::{
    FunctionDefinition, LlmProvider, LlmResponseChunk, Message, Role, ToolCall, ToolDefinition,
};

/// Execute an agent tool call against the knowledge graph or memory store.
#[allow(clippy::too_many_arguments)]
async fn execute_mcp_tool(
    name: &str,
    args: &str,
    repo_path: &Path,
    graph: &KnowledgeGraph,
    indexes: &gitnexus_db::inmemory::cypher::GraphIndexes,
    fts_index: &FtsIndex,
    embeddings: Option<&EmbeddingStore>,
    embeddings_config: Option<&EmbeddingConfig>,
    mcp_backend: Option<&Arc<TokioMutex<LocalBackend>>>,
) -> String {
    let parsed = serde_json::from_str::<serde_json::Value>(args).unwrap_or_default();

    match name {
        // ── search_code ──────────────────────────────────────────
        "search_code" => {
            let query = parsed
                .get("query")
                .and_then(|q| q.as_str())
                .unwrap_or_default();
            if query.is_empty() {
                return "Error: missing required parameter 'query'".to_string();
            }
            let results =
                search_relevant_context(query, graph, fts_index, 8, embeddings, embeddings_config);
            let sources = build_sources(&results, graph, repo_path);
            if sources.is_empty() {
                return format!("No results found for '{}'.", query);
            }
            let mut out = format!("Found {} results for '{}':\n\n", sources.len(), query);
            for (i, src) in sources.iter().enumerate() {
                out.push_str(&format!(
                    "{}. **{}** ({}) — `{}`",
                    i + 1,
                    src.symbol_name,
                    src.symbol_type,
                    src.file_path
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
            let start = parsed
                .get("start_line")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);
            let end = parsed
                .get("end_line")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);
            match read_code_snippet(repo_path, path, start, end) {
                Some(content) => {
                    let lang = detect_language(path);
                    format!(
                        "File `{}` (lines {}-{}):\n```{}\n{}\n```",
                        path,
                        start.unwrap_or(1),
                        end.unwrap_or(start.unwrap_or(1) + 30),
                        lang,
                        content
                    )
                }
                None => format!(
                    "Error: could not read file '{}' (not found or outside repo)",
                    path
                ),
            }
        }

        // ── get_impact ───────────────────────────────────────────
        "get_impact" => {
            let target = match parsed.get("target").and_then(|t| t.as_str()) {
                Some(t) => t,
                None => return "Error: missing required parameter 'target'".to_string(),
            };
            let direction = parsed
                .get("direction")
                .and_then(|d| d.as_str())
                .unwrap_or("both");
            let max_depth = parsed
                .get("max_depth")
                .and_then(|d| d.as_u64())
                .unwrap_or(3) as u32;

            // Resolve target: try exact node ID first, then name search
            let target_id = if graph.get_node(target).is_some() {
                target.to_string()
            } else {
                let target_lower = target.to_lowercase();
                match graph
                    .iter_nodes()
                    .find(|n| n.properties.name.to_lowercase() == target_lower)
                {
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
                crate::commands::impact::bfs_impact_pub(
                    graph, indexes, &target_id, max_depth, false,
                )
            } else {
                Vec::new()
            };

            let mut out = format!(
                "Impact analysis for '{}' (depth={}, direction={}):\n\n",
                target, max_depth, direction
            );
            if !upstream.is_empty() {
                out.push_str(&format!("**Upstream ({} affected):**\n", upstream.len()));
                for n in upstream.iter().take(20) {
                    out.push_str(&format!(
                        "  - {} ({}) at depth {} — `{}`\n",
                        n.node.name, n.node.label, n.depth, n.node.file_path
                    ));
                }
                if upstream.len() > 20 {
                    out.push_str(&format!("  ... and {} more\n", upstream.len() - 20));
                }
                out.push('\n');
            }
            if !downstream.is_empty() {
                out.push_str(&format!(
                    "**Downstream ({} affected):**\n",
                    downstream.len()
                ));
                for n in downstream.iter().take(20) {
                    out.push_str(&format!(
                        "  - {} ({}) at depth {} — `{}`\n",
                        n.node.name, n.node.label, n.depth, n.node.file_path
                    ));
                }
                if downstream.len() > 20 {
                    out.push_str(&format!("  ... and {} more\n", downstream.len() - 20));
                }
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
                match graph
                    .iter_nodes()
                    .find(|n| n.properties.name.to_lowercase() == sym_lower)
                {
                    Some(n) => n.id.clone(),
                    None => return format!("Error: symbol '{}' not found", symbol),
                }
            };

            let node = match graph.get_node(&node_id) {
                Some(n) => n,
                None => return format!("Error: node '{}' not found in graph", node_id),
            };
            let mut out = format!(
                "**{}** ({}) — `{}`\n\n",
                node.properties.name,
                node.label.as_str(),
                node.properties.file_path
            );

            // Callers/callees via indexes
            let mut callers = Vec::new();
            let mut callees = Vec::new();
            let mut imports = Vec::new();
            let mut inherited = Vec::new();

            if let Some(outs) = indexes.outgoing.get(&node_id) {
                for (tid, rtype) in outs {
                    if let Some(t) = graph.get_node(tid) {
                        match rtype {
                            RelationshipType::Calls => callees.push(format!(
                                "{} ({})",
                                t.properties.name,
                                t.label.as_str()
                            )),
                            RelationshipType::Imports => imports.push(t.properties.name.clone()),
                            RelationshipType::Inherits
                            | RelationshipType::Extends
                            | RelationshipType::Implements => inherited.push(format!(
                                "{} ({})",
                                t.properties.name,
                                t.label.as_str()
                            )),
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

            if !callers.is_empty() {
                out.push_str(&format!("**Called by:** {}\n", callers.join(", ")));
            }
            if !callees.is_empty() {
                out.push_str(&format!("**Calls:** {}\n", callees.join(", ")));
            }
            if !imports.is_empty() {
                out.push_str(&format!("**Imports:** {}\n", imports.join(", ")));
            }
            if !inherited.is_empty() {
                out.push_str(&format!(
                    "**Inherits/Implements:** {}\n",
                    inherited.join(", ")
                ));
            }

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
                                let json =
                                    serde_json::to_string_pretty(&truncated).unwrap_or_default();
                                format!(
                                    "Cypher returned {} results{}:\n```json\n{}\n```",
                                    rows.len(),
                                    if rows.len() > 25 {
                                        " (showing first 25)"
                                    } else {
                                        ""
                                    },
                                    json
                                )
                            }
                        }
                        Err(e) => format!("Cypher execution error: {}", e),
                    }
                }
                Err(e) => format!("Cypher parse error: {}", e),
            }
        }

        // ── get_process_flow ─────────────────────────────────────
        "get_process_flow" => {
            let keyword = parsed
                .get("keyword")
                .or_else(|| parsed.get("query"))
                .and_then(|k| k.as_str())
                .unwrap_or_default()
                .trim();
            if keyword.is_empty() {
                return "Error: missing required parameter 'keyword'".to_string();
            }

            let cypher = format!(
                "MATCH (n:Process) WHERE n.name CONTAINS '{}' RETURN n.name, n.description, n.step_count LIMIT 5",
                keyword.replace('\'', "\\'")
            );
            execute_cypher_query(&cypher, graph, indexes, fts_index)
        }

        // ── search_processes ─────────────────────────────────────
        "search_processes" => {
            let query = parsed
                .get("query")
                .and_then(|q| q.as_str())
                .unwrap_or_default();
            if query.is_empty() {
                return "Error: missing required parameter 'query'".to_string();
            }
            let cypher = format!(
                "MATCH (n:Process) WHERE n.name CONTAINS '{}' OR n.description CONTAINS '{}' \
                 RETURN n.name, n.description, n.step_count ORDER BY n.step_count DESC LIMIT 10",
                query.replace('\'', "\\'"),
                query.replace('\'', "\\'")
            );
            match gitnexus_db::inmemory::cypher::parse(&cypher) {
                Ok(stmt) => {
                    match gitnexus_db::inmemory::cypher::execute(&stmt, graph, indexes, fts_index) {
                        Ok(rows) => {
                            if rows.is_empty() {
                                format!("No business processes found for '{}'.", query)
                            } else {
                                let json = serde_json::to_string_pretty(&rows).unwrap_or_default();
                                format!(
                                    "Processus métier trouvés pour '{}' :\n```json\n{}\n```",
                                    query, json
                                )
                            }
                        }
                        Err(e) => format!("Execution error: {}", e),
                    }
                }
                Err(e) => format!("Parse error: {}", e),
            }
        }

        // ── get_diagram ──────────────────────────────────────────
        "get_diagram" => {
            let target = match parsed.get("target").and_then(|t| t.as_str()) {
                Some(t) => t,
                None => return "Error: missing required parameter 'target'".to_string(),
            };
            let diagram_type = parsed.get("diagram_type").and_then(|d| d.as_str());
            match crate::commands::diagram::build_diagram(graph, indexes, target, diagram_type) {
                Ok(diagram) => format!(
                    "Mermaid {} diagram for '{}':\n```mermaid\n{}\n```",
                    diagram.diagram_type, target, diagram.mermaid
                ),
                Err(e) => format!("Error: {}", e),
            }
        }

        // ── read_method ─────────────────────────────────────────
        // Full method source (up to 250 lines) — for algorithm questions
        "read_method" => {
            let symbol = match parsed.get("symbol").and_then(|s| s.as_str()) {
                Some(s) => s,
                None => return "Error: missing required parameter 'symbol'".to_string(),
            };
            read_full_method(symbol, graph, repo_path).await
        }

        // ── list_sfd_pages ───────────────────────────────────────
        // Doc-SFD workflow (P1.1): the chat needs to know which pages exist
        // before it can write into one. Walks `.gitnexus/docs/modules/` and
        // also surfaces drafts under `.gitnexus/docs/_drafts/` so the LLM
        // sees in-progress work it might want to extend.
        "list_sfd_pages" => {
            let docs_dir = repo_path.join(".gitnexus").join("docs");
            if !docs_dir.exists() {
                return format!(
                    "No docs directory at {} — run `gitnexus generate docs` first.",
                    docs_dir.display()
                );
            }
            let mut pages: Vec<String> = Vec::new();
            let mut drafts: Vec<String> = Vec::new();
            let modules_dir = docs_dir.join("modules");
            if modules_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&modules_dir) {
                    for e in entries.flatten() {
                        let p = e.path();
                        if p.extension().and_then(|x| x.to_str()) == Some("md") {
                            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                                pages.push(name.to_string());
                            }
                        }
                    }
                }
            }
            let drafts_dir = docs_dir.join("_drafts");
            if drafts_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&drafts_dir) {
                    for e in entries.flatten() {
                        let p = e.path();
                        if p.extension().and_then(|x| x.to_str()) == Some("md") {
                            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                                drafts.push(name.to_string());
                            }
                        }
                    }
                }
            }
            pages.sort();
            drafts.sort();
            let mut out = format!("Found {} module page(s):\n", pages.len());
            for p in &pages {
                out.push_str(&format!("- {}\n", p));
            }
            if !drafts.is_empty() {
                out.push_str(&format!("\n{} draft(s) in _drafts/:\n", drafts.len()));
                for d in &drafts {
                    out.push_str(&format!("- {} (draft)\n", d));
                }
            }
            out
        }

        // ── write_sfd_draft ──────────────────────────────────────
        // Writes Markdown into `.gitnexus/docs/_drafts/<page>` — never into
        // the live `modules/` tree. The user keeps control over promotion: a
        // bad LLM run loses a draft, not the 137 enriched production pages.
        "write_sfd_draft" => {
            let page = match parsed.get("page").and_then(|p| p.as_str()) {
                Some(p) => p.trim(),
                None => return "Error: missing required parameter 'page'".to_string(),
            };
            let content = match parsed.get("content").and_then(|c| c.as_str()) {
                Some(c) => c,
                None => return "Error: missing required parameter 'content'".to_string(),
            };
            // Reject path traversal — drafts must land in the drafts dir,
            // not anywhere on disk via `../../../etc/passwd`.
            if page.contains("..")
                || page.contains('\\')
                || page.starts_with('/')
                || page.is_empty()
            {
                return format!(
                    "Error: invalid page name '{}' (no '..', '\\', leading '/', or empty)",
                    page
                );
            }
            let drafts_dir = repo_path.join(".gitnexus").join("docs").join("_drafts");
            if let Err(e) = std::fs::create_dir_all(&drafts_dir) {
                return format!("Error: could not create drafts dir: {}", e);
            }
            let target = drafts_dir.join(page);
            // Atomic write (.tmp → rename) so a partial flush doesn't leave
            // a half-written .md the validator might choke on.
            let tmp = target.with_extension("md.tmp");
            if let Err(e) = std::fs::write(&tmp, content) {
                return format!("Error: write failed for {}: {}", target.display(), e);
            }
            if let Err(e) = std::fs::rename(&tmp, &target) {
                return format!("Error: atomic rename failed: {}", e);
            }
            format!(
                "Draft written: {} ({} bytes). Call `validate_sfd` with `path: \"_drafts\"` \
                 to lint it before promotion.",
                target.display(),
                content.len()
            )
        }

        // ── validate_sfd ─────────────────────────────────────────
        // Run the validator against the docs tree (or a sub-path like
        // `_drafts/`) and return the structured RED/GREEN report. The chat
        // can render this directly; promotion to live docs stays a deliberate
        // user action outside the agent loop.
        "validate_sfd" => {
            let sub_path = parsed.get("path").and_then(|p| p.as_str()).unwrap_or("");
            let docs_dir = if sub_path.is_empty() {
                repo_path.join(".gitnexus").join("docs")
            } else if sub_path.contains("..") || sub_path.starts_with('/') {
                return format!(
                    "Error: invalid path '{}' (no '..' or absolute paths)",
                    sub_path
                );
            } else {
                repo_path.join(".gitnexus").join("docs").join(sub_path)
            };
            match gitnexus_rag::validator::validate(&docs_dir, &repo_path.display().to_string()) {
                Ok(report) => {
                    let status = if report.red_count == 0 && report.yellow_count == 0 {
                        "GREEN — ready to ship"
                    } else if report.red_count == 0 {
                        "YELLOW — ships but has style issues"
                    } else {
                        "RED — blocked, must fix"
                    };
                    let mut out = format!(
                        "**Validation: {}**\n\n\
                         - {} pages scanned, {} with issues\n\
                         - {} RED issues, {} YELLOW issues\n",
                        status,
                        report.total_pages,
                        report.pages_with_issues,
                        report.red_count,
                        report.yellow_count,
                    );
                    if !report.by_kind.is_empty() {
                        out.push_str("\n**Issue kinds:**\n");
                        for (kind, count) in &report.by_kind {
                            out.push_str(&format!("- {}: {}\n", kind, count));
                        }
                    }
                    if !report.pages.is_empty() {
                        out.push_str("\n**Top pages with issues:**\n");
                        let mut sorted = report.pages.clone();
                        sorted.sort_by(|a, b| b.issues.len().cmp(&a.issues.len()));
                        for page in sorted.iter().take(5) {
                            out.push_str(&format!(
                                "\n*{}* — {} issue(s):\n",
                                page.path,
                                page.issues.len()
                            ));
                            for iss in page.issues.iter().take(3) {
                                let sev = match iss.severity {
                                    gitnexus_rag::validator::Severity::Red => "RED",
                                    gitnexus_rag::validator::Severity::Yellow => "YELLOW",
                                };
                                let line = iss
                                    .line
                                    .map(|n| format!("L{}", n))
                                    .unwrap_or_else(|| "-".to_string());
                                out.push_str(&format!(
                                    "  - [{}] {} {}: {}\n",
                                    sev, line, iss.kind, iss.detail
                                ));
                            }
                            if page.issues.len() > 3 {
                                out.push_str(&format!("  - ... +{} more\n", page.issues.len() - 3));
                            }
                        }
                    }
                    out
                }
                Err(e) => format!("Error: validation failed: {}", e),
            }
        }

        // ── recall_memory ────────────────────────────────────────
        // The LLM has `save_memory` but no way to query what's already there
        // — without this, persisted facts are write-only from its point of
        // view and only surface via the system prompt's pre-injected memory
        // context. Adding a query path lets the agent ask "do I already know
        // X about this codebase?" before redoing work.
        "recall_memory" => {
            let query = parsed
                .get("query")
                .and_then(|q| q.as_str())
                .unwrap_or("")
                .trim()
                .to_lowercase();
            let scope_arg = parsed
                .get("scope")
                .and_then(|s| s.as_str())
                .unwrap_or("all");

            let mut matches: Vec<(String, String)> = Vec::new();
            let scopes_to_check: &[gitnexus_core::memory::MemoryScope] = match scope_arg {
                "global" => &[gitnexus_core::memory::MemoryScope::Global],
                "project" => &[gitnexus_core::memory::MemoryScope::Project],
                _ => &[
                    gitnexus_core::memory::MemoryScope::Global,
                    gitnexus_core::memory::MemoryScope::Project,
                ],
            };

            for scope in scopes_to_check {
                let label = match scope {
                    gitnexus_core::memory::MemoryScope::Global => "global",
                    gitnexus_core::memory::MemoryScope::Project => "project",
                };
                let store = gitnexus_core::memory::MemoryStore::load(*scope, Some(repo_path));
                for fact in &store.facts {
                    if query.is_empty() || fact.to_lowercase().contains(&query) {
                        matches.push((label.to_string(), fact.clone()));
                    }
                }
            }

            if matches.is_empty() {
                if query.is_empty() {
                    return "No facts saved yet (use save_memory to record one).".to_string();
                }
                return format!("No saved facts matching '{}'.", query);
            }
            let mut out = format!("Found {} saved fact(s):\n\n", matches.len());
            for (scope, fact) in matches {
                out.push_str(&format!("- [{}] {}\n", scope, fact));
            }
            out
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
                Some(other) => {
                    return format!(
                        "Error: invalid scope '{}', expected 'global' or 'project'",
                        other
                    )
                }
                None => return "Error: missing required parameter 'scope'".to_string(),
            };
            let mut store = gitnexus_core::memory::MemoryStore::load(scope, Some(repo_path));
            store.add_fact(fact.to_string());
            match store.save(scope, Some(repo_path)) {
                Ok(()) => "Fact saved successfully.".to_string(),
                Err(e) => format!("Failed to save memory: {}", e),
            }
        }

        // ── Extended MCP tools (hotspots, coupling, ownership, coverage,
        //    report, business, find_cycles, find_similar_code, list_todos,
        //    get_complexity, list_endpoints, list_db_tables, list_env_vars,
        //    get_endpoint_handler, detect_changes, analyze_execution_trace,
        //    get_insights, rename) — delegated to LocalBackend.
        _ => match mcp_backend {
            Some(backend) => dispatch_via_local_backend(backend, name, args, repo_path).await,
            None => format!("Error: unknown tool '{}'", name),
        },
    }
}

/// Forward a tool call to the shared `LocalBackend` so the chat surfaces the
/// full 27-tool MCP catalogue without re-implementing each one. Auto-injects
/// the active repo path into `args.repo` when missing — the chat already knows
/// which repo is active, so making the LLM specify it on every call would just
/// be friction.
async fn dispatch_via_local_backend(
    backend: &Arc<TokioMutex<LocalBackend>>,
    name: &str,
    args_str: &str,
    repo_path: &Path,
) -> String {
    let mut args: serde_json::Value =
        serde_json::from_str(args_str).unwrap_or_else(|_| serde_json::json!({}));
    if !args.is_object() {
        args = serde_json::json!({});
    }
    if let Some(obj) = args.as_object_mut() {
        if !obj.contains_key("repo") {
            obj.insert(
                "repo".to_string(),
                serde_json::Value::String(repo_path.to_string_lossy().to_string()),
            );
        }
    }

    let mut backend_lock = backend.lock().await;
    match backend_lock.call_tool(name, &args).await {
        Ok(response) => extract_mcp_text(&response),
        Err(e) => format!("Error from MCP tool '{}': {}", name, e),
    }
}

/// Pull the human-readable text out of an MCP tool response envelope
/// (`{"content": [{"type": "text", "text": "..."}]}`). Falls back to the
/// stringified JSON when the envelope shape is unexpected, so the LLM still
/// gets *something* useful instead of an empty response.
fn extract_mcp_text(response: &serde_json::Value) -> String {
    if let Some(text) = response
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
    {
        return text.to_string();
    }
    serde_json::to_string_pretty(response)
        .unwrap_or_else(|_| "Error: failed to format MCP response".to_string())
}

/// Execute a Cypher query and format a compact JSON result for prompt injection.
fn execute_cypher_query(
    query: &str,
    graph: &KnowledgeGraph,
    indexes: &gitnexus_db::inmemory::cypher::GraphIndexes,
    fts_index: &FtsIndex,
) -> String {
    match gitnexus_db::inmemory::cypher::parse(query) {
        Ok(stmt) => {
            match gitnexus_db::inmemory::cypher::execute(&stmt, graph, indexes, fts_index) {
                Ok(rows) => {
                    if rows.is_empty() {
                        "Query returned 0 results.".to_string()
                    } else {
                        let truncated: Vec<_> = rows.iter().take(25).collect();
                        let json = serde_json::to_string_pretty(&truncated).unwrap_or_default();
                        format!(
                            "Cypher returned {} results{}:\n```json\n{}\n```",
                            rows.len(),
                            if rows.len() > 25 {
                                " (showing first 25)"
                            } else {
                                ""
                            },
                            json
                        )
                    }
                }
                Err(e) => format!("Cypher execution error: {}", e),
            }
        }
        Err(e) => format!("Cypher parse error: {}", e),
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

    // 1. Get the active repo's graph, FTS index, and (optionally) embeddings
    //    for hybrid search. Embeddings are loaded once at open_repo time and
    //    cached in LoadedRepo, so this is a cheap Arc clone.
    let (graph, indexes, fts_index, embeddings, embeddings_config, repo_path_str) =
        state.get_repo_with_embeddings(None).await?;
    let repo_path = PathBuf::from(&repo_path_str);
    let emb_ref = embeddings.as_deref();
    let emb_cfg_ref = embeddings_config.as_deref();
    let mcp_backend = Arc::clone(&state.mcp_backend);
    let mcp_ref = Some(&mcp_backend);

    // 2. Classify the question → determines canvas + pre-fetch strategy
    let question_type = classify_question(&request.question);

    // 3. Search for relevant symbols (hybrid BM25+semantic when embeddings exist)
    let search_results = search_relevant_context(
        &request.question,
        &graph,
        &fts_index,
        10,
        emb_ref,
        emb_cfg_ref,
    );

    // 4. Read code snippets for top results
    let sources = build_sources(&search_results, &graph, &repo_path);

    // 4b. Read enriched documentation pages for relevant modules (higher quality context)
    let enriched_doc_context = load_enriched_doc_pages(&search_results, &graph, &repo_path);

    // 4c. Pre-fetch tool results for the question type (deterministic, before LLM call)
    let prefetched = prefetch_for_type(
        question_type,
        &request.question,
        &search_results,
        &graph,
        &indexes,
        &fts_index,
        &repo_path,
        emb_ref,
        emb_cfg_ref,
        mcp_ref,
    )
    .await;

    // 5. Assemble the prompt
    let system_prompt = build_system_prompt(
        &graph,
        &sources,
        &repo_path,
        &enriched_doc_context,
        &prefetched,
    );

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

    // Append canvas instruction to the user message
    let canvas = canvas_instruction(question_type);
    let user_content = format!("{}\n\n{}", request.question, canvas);
    messages.push(Message {
        role: Role::User,
        content: Some(user_content),
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

    // Send the LLM the *full* tool catalogue (10 historical + recall_memory +
    // 17 extended MCP tools). Previously this list only carried the original
    // ten — adding descriptors via `build_tool_descriptors()` made them visible
    // in the UI but NOT in the function-calling schema, so the LLM could see
    // them in the panel and still couldn't call them. Derive from the same
    // source of truth used by the UI to keep the two views in sync.
    //
    // TODO: also pass `tool_choice: "required"` on the first iteration once
    // `LlmProvider::stream_completion` accepts a request struct — today the
    // anti-stall heuristic at L1117+ catches "I'll search…" announcements as
    // a fallback, but `tool_choice` would let the API enforce it server-side.
    let tools: Vec<ToolDefinition> = descriptors_to_llm_tools(&build_tool_descriptors());

    let mut final_answer = String::new();
    let mut iteration = 0;
    // Vary iteration budget by question type — Lookup rarely needs more than 2;
    // Algorithm/Architecture need up to 8 to fully trace call chains.
    let max_iterations: usize = match question_type {
        QuestionType::Lookup => 2,
        QuestionType::Impact => 3,
        QuestionType::Functional => 5,
        QuestionType::Algorithm | QuestionType::Architecture => 8,
    };

    while iteration < max_iterations {
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
            // Check cancellation flag — set by chat_cancel command
            if state.cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                state
                    .cancel_flag
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                let _ = app.emit("chat-stream-done", ());
                let _ = app.emit("chat-stream-cancelled", ());
                return Ok(ChatResponse {
                    answer: final_answer,
                    sources: sources.clone(),
                    model: Some(config.model.clone()),
                });
            }
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

        // Lower-cased copy used by the nudge-detection branch below (we
        // can't borrow `text_received` after moving it into the Message).
        let text_lc = text_received.to_lowercase();

        if !text_received.is_empty() || !tool_calls_received.is_empty() {
            messages.push(Message {
                role: Role::Assistant,
                content: if text_received.is_empty() {
                    None
                } else {
                    Some(text_received)
                },
                tool_calls: if tool_calls_received.is_empty() {
                    None
                } else {
                    Some(tool_calls_received.clone())
                },
                tool_call_id: None,
                name: None,
            });
        }

        if tool_calls_received.is_empty() {
            // The LLM emitted only text this turn. Detect if it *announced* a
            // tool intent ("I'll search…", "Je vais rechercher…") without
            // actually emitting a tool_call. Gemini 2.5-flash is particularly
            // prone to this: it describes what it *would* search for but never
            // emits the structured tool_call payload.
            let announced_tool_intent = [
                // French
                "je vais rechercher",
                "je vais chercher",
                "je vais examiner",
                "je vais regarder",
                "je vais analyser",
                "je vais inspecter",
                "commencer par rechercher",
                "pour commencer",
                "rechercher des",
                "mots-clés comme",
                "mots clés comme",
                "termes clés comme",
                "identifier les symboles",
                "rechercher les symboles",
                // English
                "i'll search",
                "let me search",
                "let me check",
                "let me look",
                "i will search",
                "i need to find",
                "i'll look",
                "i'll check",
                "first, i'll",
                "first, let me",
                "let me start by",
                "i'll start by",
                "keywords like",
                "search for",
            ]
            .iter()
            .any(|h| text_lc.contains(h));

            // FIX: fire on any iteration, not just 1 — Gemini often announces
            // tool intent after receiving tool results too (turn 2, 3, etc.)
            if announced_tool_intent && iteration < max_iterations.saturating_sub(1) {
                // FALLBACK: auto-execute search_code on the user's question
                // so we always make forward progress. Text nudging alone was
                // not enough — the model would just re-announce on the next
                // turn and stall again.
                let user_question = messages
                    .iter()
                    .rev()
                    .find(|m| matches!(m.role, Role::User))
                    .and_then(|m| m.content.clone())
                    .unwrap_or_default();

                let synthetic_id = format!("auto-search-{}", iteration);
                let tool_args = serde_json::json!({
                    "query": user_question.trim()
                })
                .to_string();

                let _ = app.emit("tool_execution_start", "search_code".to_string());
                let tool_result = execute_mcp_tool(
                    "search_code",
                    &tool_args,
                    &repo_path,
                    &graph,
                    &indexes,
                    &fts_index,
                    emb_ref,
                    emb_cfg_ref,
                    mcp_ref,
                )
                .await;
                let _ = app.emit("tool_execution_end", "search_code".to_string());

                // Retrofit the assistant message we just pushed so the API
                // sees a matching tool_call/tool_result pair (OpenAI protocol
                // requires the assistant to "own" the call via tool_calls).
                if let Some(last) = messages.last_mut() {
                    if matches!(last.role, Role::Assistant) {
                        last.tool_calls = Some(vec![ToolCall {
                            id: synthetic_id.clone(),
                            name: "search_code".to_string(),
                            arguments: tool_args.clone(),
                        }]);
                    }
                }
                messages.push(Message {
                    role: Role::Tool,
                    content: Some(tool_result),
                    tool_calls: None,
                    tool_call_id: Some(synthetic_id),
                    name: Some("search_code".to_string()),
                });
                // Also nudge so the next turn synthesizes a real answer
                // using the injected tool result.
                messages.push(Message {
                    role: Role::User,
                    content: Some(
                        "Based on those search results, please provide a complete answer to my \
                         original question. Cite specific files and symbols. If you need more \
                         detail, call get_symbol_context or read_file — but do not announce it, \
                         just call it."
                            .to_string(),
                    ),
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                });
                continue;
            }
            break; // Done, no tools called
        }

        for tc in tool_calls_received {
            let _ = app.emit("tool_execution_start", tc.name.clone());
            let result = execute_mcp_tool(
                &tc.name,
                &tc.arguments,
                &repo_path,
                &graph,
                &indexes,
                &fts_index,
                emb_ref,
                emb_cfg_ref,
                mcp_ref,
            )
            .await;
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

    if iteration >= max_iterations {
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
pub async fn chat_set_config(state: State<'_, AppState>, config: ChatConfig) -> Result<(), String> {
    state.set_chat_config(config.clone()).await;
    save_config(&config)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatConnectionTestResult {
    pub ok: bool,
    pub status: u16,
    pub model: String,
    /// Short preview of the model's reply (when ok) or of the error body.
    pub message: String,
    /// Round-trip latency in milliseconds.
    pub latency_ms: u64,
}

/// Ping the configured LLM endpoint with a trivial prompt so the user can
/// validate provider + api_key + base_url + model from the UI. Mirrors what
/// the CLI's `gitnexus config test` command does.
#[tauri::command]
pub async fn chat_test_connection(config: ChatConfig) -> Result<ChatConnectionTestResult, String> {
    // Hydrate API key from env if the UI left it blank — the CLI does the
    // same, and power users often set GEMINI_API_KEY / OPENAI_API_KEY
    // instead of typing the key into the form.
    let config = hydrate_api_key_from_env(config);

    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": config.model,
        "messages": [{"role": "user", "content": "Say hello in one word."}],
        "max_tokens": 10,
        "temperature": 0.0,
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client build failed: {e}"))?;

    let is_local = is_local_llm_url(&config.base_url);
    let start = std::time::Instant::now();
    let mut request = client.post(&url).json(&body);
    if !config.api_key.is_empty() && !is_local {
        request = request.header("Authorization", format!("Bearer {}", config.api_key));
    }

    let response = match request.send().await {
        Ok(r) => r,
        Err(e) => {
            return Ok(ChatConnectionTestResult {
                ok: false,
                status: 0,
                model: config.model,
                message: format!("Network error: {e}"),
                latency_ms: start.elapsed().as_millis() as u64,
            });
        }
    };

    let latency_ms = start.elapsed().as_millis() as u64;
    let status = response.status().as_u16();
    let ok = response.status().is_success();

    let message = if ok {
        match response.json::<serde_json::Value>().await {
            Ok(json) => json
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("content"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .chars()
                .take(120)
                .collect::<String>(),
            Err(e) => format!("parse error: {e}"),
        }
    } else {
        response
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(200)
            .collect::<String>()
    };

    Ok(ChatConnectionTestResult {
        ok,
        status,
        model: config.model,
        message,
        latency_ms,
    })
}

// ─── Theme B: Agent-tool introspection + retry ──────────────────────

/// Static descriptor for a single agent/MCP tool. Surfaced to the UI so
/// the "Tools" panel can show what capabilities the chat agent currently
/// has. Kept in sync with `execute_mcp_tool` above.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatToolDescriptor {
    /// Tool name as the LLM sees it (stable; matches `execute_mcp_tool`).
    pub name: String,
    /// Human description shown in the tools panel.
    pub description: String,
    /// Grouping label — purely presentational.
    pub category: String,
    /// JSON Schema (draft-07 shape) describing the expected args. Kept in
    /// sync with the `ToolDefinition` blocks built in `chat_ask`.
    pub parameters: serde_json::Value,
}

/// Convert chat tool descriptors (the UI-facing list) into the OpenAI-style
/// `ToolDefinition` array the agent loop sends to the LLM. Single source of
/// truth: keep both views derived from `build_tool_descriptors()` so adding
/// a tool is one edit, not three.
///
/// Also stamps `additionalProperties: false` on every schema so the API
/// rejects args the LLM hallucinates that we don't accept — a quiet source
/// of "the model called the tool with `{since: 30}` but the backend silently
/// substituted the default 90" bugs.
fn descriptors_to_llm_tools(descriptors: &[ChatToolDescriptor]) -> Vec<ToolDefinition> {
    descriptors
        .iter()
        .map(|d| {
            let mut params = d.parameters.clone();
            if let Some(obj) = params.as_object_mut() {
                obj.entry("additionalProperties")
                    .or_insert(serde_json::Value::Bool(false));
            }
            ToolDefinition {
                type_: "function".to_string(),
                function: FunctionDefinition {
                    name: d.name.clone(),
                    description: d.description.clone(),
                    parameters: params,
                },
            }
        })
        .collect()
}

fn build_tool_descriptors() -> Vec<ChatToolDescriptor> {
    vec![
        ChatToolDescriptor {
            name: "search_code".to_string(),
            description: "Full-text search over the knowledge graph (BM25 + exact name)."
                .to_string(),
            category: "search".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query (symbol, keyword, pattern)" }
                },
                "required": ["query"]
            }),
        },
        ChatToolDescriptor {
            name: "read_file".to_string(),
            description: "Read a range of lines from a source file in the repository.".to_string(),
            category: "files".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "start_line": { "type": "number" },
                    "end_line": { "type": "number" }
                },
                "required": ["path"]
            }),
        },
        ChatToolDescriptor {
            name: "get_impact".to_string(),
            description: "Upstream/downstream impact (BFS over Calls/Imports/Inherits)."
                .to_string(),
            category: "analysis".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "target": { "type": "string" },
                    "direction": { "type": "string", "enum": ["upstream", "downstream", "both"] },
                    "max_depth": { "type": "number" }
                },
                "required": ["target"]
            }),
        },
        ChatToolDescriptor {
            name: "get_symbol_context".to_string(),
            description: "360° view of a symbol: callers, callees, imports, inheritance, module."
                .to_string(),
            category: "analysis".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "symbol": { "type": "string" }
                },
                "required": ["symbol"]
            }),
        },
        ChatToolDescriptor {
            name: "execute_cypher".to_string(),
            description: "Read-only Cypher against the in-memory graph. SUPPORTED: MATCH (n:Label) / MATCH (n)-[r:TYPE]->(m), WHERE with =, <>, !=, CONTAINS, STARTS WITH, ENDS WITH, AND, OR, NOT, RETURN (with DISTINCT), ORDER BY, LIMIT, count(n), CALL QUERY_FTS_INDEX('table', 'query'). NOT supported: IN, COUNT(r) AS alias, GROUP BY, OPTIONAL MATCH, map literals {k:v}, UNWIND, multi-statement queries. Stick to a single-pattern MATCH + WHERE + RETURN — anything fancier will fail to parse.".to_string(),
            category: "query".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Cypher in the supported subset above. Example: MATCH (n:Function) WHERE n.name CONTAINS 'auth' AND NOT n.filePath ENDS WITH '.test.ts' RETURN DISTINCT n.name, n.filePath ORDER BY n.name LIMIT 20" }
                },
                "required": ["query"]
            }),
        },
        ChatToolDescriptor {
            name: "search_processes".to_string(),
            description: "Search business process flows in the graph. Use for workflow, business process, or multi-step operation questions.".to_string(),
            category: "search".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" }
                },
                "required": ["query"]
            }),
        },
        ChatToolDescriptor {
            name: "get_process_flow".to_string(),
            description: "Targeted business process lookup by keyword.".to_string(),
            category: "analysis".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "keyword": { "type": "string" }
                },
                "required": ["keyword"]
            }),
        },
        ChatToolDescriptor {
            name: "get_diagram".to_string(),
            description: "Generate a Mermaid flowchart, sequence, or class diagram.".to_string(),
            category: "visualize".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "target": { "type": "string" },
                    "diagram_type": {
                        "type": "string",
                        "enum": ["flowchart", "sequence", "class"]
                    }
                },
                "required": ["target"]
            }),
        },
        ChatToolDescriptor {
            name: "read_method".to_string(),
            description: "Read complete method or function source up to 250 lines.".to_string(),
            category: "files".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "symbol": { "type": "string" }
                },
                "required": ["symbol"]
            }),
        },
        ChatToolDescriptor {
            name: "save_memory".to_string(),
            description: "Persist a fact or preference across future chat sessions.".to_string(),
            category: "memory".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "fact": { "type": "string" },
                    "scope": { "type": "string", "enum": ["global", "project"] }
                },
                "required": ["fact", "scope"]
            }),
        },
        ChatToolDescriptor {
            name: "recall_memory".to_string(),
            description: "Query previously-saved facts by keyword (case-insensitive substring). Symmetric of save_memory — use before redoing work to check what's already known.".to_string(),
            category: "memory".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Keyword to match against saved facts. Empty string returns all." },
                    "scope": { "type": "string", "enum": ["global", "project", "all"], "description": "Which scope to search (default 'all')" }
                }
            }),
        },
        // ── Doc-SFD workflow (P1.1) ───────────────────────────────────
        // Three tools that turn the chat into a write-side authoring loop
        // for `.gitnexus/docs/`. Drafts live in `_drafts/` so a bad LLM run
        // can't trash the 137 enriched production pages — promotion stays
        // a user-driven action outside the agent loop.
        ChatToolDescriptor {
            name: "list_sfd_pages".to_string(),
            description: "List Markdown pages under `.gitnexus/docs/modules/` and any in-progress drafts under `.gitnexus/docs/_drafts/`. Call this first to see what pages exist before writing.".to_string(),
            category: "docs".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        ChatToolDescriptor {
            name: "write_sfd_draft".to_string(),
            description: "Write Markdown content to `.gitnexus/docs/_drafts/<page>`. The full SFD section text the LLM has composed goes in `content` — do not write code-side files. Drafts are atomic (tmp+rename) and never overwrite the live `modules/` tree.".to_string(),
            category: "docs".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "page": { "type": "string", "description": "Filename only, e.g. 'aspnet-services.md' — no path components, no '..'" },
                    "content": { "type": "string", "description": "Full Markdown body of the page (including all SFD sections — Besoin, Exigences, Modèle, §4 Algorithmes, Diagrammes)" }
                },
                "required": ["page", "content"]
            }),
        },
        ChatToolDescriptor {
            name: "validate_sfd".to_string(),
            description: "Run the pre-delivery linter against the docs tree (or a sub-path like '_drafts'). Returns a structured RED/GREEN report listing residual TODOs, unfilled GNX:* anchors, broken links, short sections, and missing §4 Algorithmes on service/controller pages.".to_string(),
            category: "docs".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Optional sub-path under `.gitnexus/docs/` (e.g. '_drafts' or 'modules'). Omit to validate the entire docs tree." }
                }
            }),
        },
        // ── Extended MCP tools (delegated to LocalBackend) ─────────────
        // These hit the full 27-tool gitnexus-mcp catalogue rather than
        // re-implementing every analytic. Adding a tool here makes the LLM
        // aware of it; the dispatch fallback in `execute_mcp_tool` does the
        // routing. Keep parameters terse — verbose schemas eat prompt budget.
        ChatToolDescriptor {
            name: "hotspots".to_string(),
            description: "Top files by churn over the last N days (refactor candidates).".to_string(),
            category: "git".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "since_days": { "type": "number", "description": "Lookback window in days (default 90)" },
                    "limit": { "type": "number" }
                }
            }),
        },
        ChatToolDescriptor {
            name: "coupling".to_string(),
            description: "Files that change together over git history (temporal coupling).".to_string(),
            category: "git".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "min_shared": { "type": "number", "description": "Minimum shared commits to consider coupled (default 3)" },
                    "limit": { "type": "number" }
                }
            }),
        },
        ChatToolDescriptor {
            name: "ownership".to_string(),
            description: "Per-file author distribution (who wrote/maintains what).".to_string(),
            category: "git".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "number" }
                }
            }),
        },
        ChatToolDescriptor {
            name: "coverage".to_string(),
            description: "Tracing instrumentation coverage and dead-code candidates (zero callers).".to_string(),
            category: "quality".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "target": { "type": "string", "description": "Optional class/service to scope to; omit for global stats" }
                }
            }),
        },
        ChatToolDescriptor {
            name: "report".to_string(),
            description: "Codebase health score (A-E grade) with sub-scores.".to_string(),
            category: "quality".to_string(),
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
        },
        ChatToolDescriptor {
            name: "business".to_string(),
            description: "Documented business processes (workflows, payments, mass mail, etc.).".to_string(),
            category: "domain".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "process": { "type": "string", "description": "Optional process name (e.g. 'paiements', 'courriers'); omit to list all" }
                }
            }),
        },
        ChatToolDescriptor {
            name: "find_cycles".to_string(),
            description: "Strongly-connected components (Tarjan) — circular import or call cycles.".to_string(),
            category: "quality".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "scope": { "type": "string", "enum": ["imports", "calls"] },
                    "limit": { "type": "number" }
                }
            }),
        },
        ChatToolDescriptor {
            name: "find_similar_code".to_string(),
            description: "Detect near-duplicate code via Rabin-Karp + Jaccard similarity.".to_string(),
            category: "quality".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "min_tokens": { "type": "number", "description": "Minimum window size in tokens (default 30)" },
                    "threshold": { "type": "number", "description": "Min Jaccard similarity (default 0.9)" },
                    "limit": { "type": "number" }
                }
            }),
        },
        ChatToolDescriptor {
            name: "list_todos".to_string(),
            description: "TODO / FIXME / HACK / XXX markers across the codebase.".to_string(),
            category: "inventory".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "severity": { "type": "string", "enum": ["TODO", "FIXME", "HACK", "XXX"] },
                    "limit": { "type": "number" }
                }
            }),
        },
        ChatToolDescriptor {
            name: "get_complexity".to_string(),
            description: "Cyclomatic complexity statistics (averages, percentiles, top-N hot spots).".to_string(),
            category: "quality".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "threshold": { "type": "number", "description": "Only list symbols with complexity ≥ this (default 0)" },
                    "limit": { "type": "number" }
                }
            }),
        },
        ChatToolDescriptor {
            name: "list_endpoints".to_string(),
            description: "REST/GraphQL endpoints (Express, FastAPI, Flask, Spring, ASP.NET MVC, …).".to_string(),
            category: "inventory".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "method": { "type": "string", "description": "Filter by HTTP verb (GET/POST/...) — case-insensitive" },
                    "pattern": { "type": "string", "description": "Substring filter on the route path (case-insensitive)" },
                    "limit": { "type": "number" }
                }
            }),
        },
        ChatToolDescriptor {
            name: "list_db_tables".to_string(),
            description: "Database tables (SQL migrations, Prisma, SQLAlchemy, TypeORM, EF6, …).".to_string(),
            category: "inventory".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "number" }
                }
            }),
        },
        ChatToolDescriptor {
            name: "list_env_vars".to_string(),
            description: "Environment variables declared vs. used (audit).".to_string(),
            category: "inventory".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "unused_only": { "type": "boolean", "description": "If true, surface declared-but-unused vars" },
                    "limit": { "type": "number" }
                }
            }),
        },
        ChatToolDescriptor {
            name: "get_endpoint_handler".to_string(),
            description: "Resolve an endpoint route + verb to its handler method + degree-1 call neighborhood.".to_string(),
            category: "inventory".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "route": { "type": "string", "description": "Route path as discovered by list_endpoints (e.g. '/api/users/:id')" },
                    "method": { "type": "string", "description": "HTTP method (GET/POST/PUT/DELETE/PATCH)" }
                },
                "required": ["route", "method"]
            }),
        },
        ChatToolDescriptor {
            name: "detect_changes".to_string(),
            description: "Analyze the repo's uncommitted changes: map git-diff hunks to symbols, BFS upstream, classify risk (none/low/medium/high).".to_string(),
            category: "git".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "max_upstream_depth": { "type": "number", "description": "BFS depth from changed symbols (default 3, max 10)" }
                }
            }),
        },
        ChatToolDescriptor {
            name: "get_insights".to_string(),
            description: "Per-symbol insights: complexity, dead-code flag, tracing, smells, design patterns, risk.".to_string(),
            category: "analysis".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "symbol": { "type": "string" }
                },
                "required": ["symbol"]
            }),
        },
        ChatToolDescriptor {
            name: "rename".to_string(),
            description: "Multi-file rename (graph-confirmed + text-search fallback). Dry-run by default.".to_string(),
            category: "edit".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "target": { "type": "string", "description": "Current symbol name or node ID" },
                    "new_name": { "type": "string", "description": "Proposed new name" },
                    "dry_run": { "type": "boolean", "description": "Preview without writing (default true)" }
                },
                "required": ["target", "new_name"]
            }),
        },
    ]
}

/// Return the static list of tools the chat agent can invoke. Used by
/// the desktop Tools panel to show capabilities to the user; does not
/// touch the graph so it is cheap and can be called at any time.
#[tauri::command]
pub async fn list_chat_tools() -> Result<Vec<ChatToolDescriptor>, String> {
    Ok(build_tool_descriptors())
}

/// Surface the 6 MCP prompts (`detect_impact`, `generate_map`,
/// `analyze_hotspots`, `find_dead_code`, `trace_dependencies`,
/// `describe_process`) as ready-to-paste templates. Each one encodes a
/// validated tool-chain (e.g. analyze_hotspots → hotspots + coupling +
/// ownership → recommend refactor priorities) — exposing them to the UI
/// lets the chat reuse those chains instead of re-inventing the orchestration
/// in `chat_planner`.
#[tauri::command]
pub async fn list_chat_prompts() -> Result<serde_json::Value, String> {
    Ok(gitnexus_mcp::prompts::prompt_definitions())
}

/// Render a named MCP prompt to plain text. The UI pastes the result into
/// the chat input as the user's first message — the LLM then follows the
/// embedded "Please: 1. use X tool 2. use Y tool…" recipe naturally via
/// the existing tool-calling loop.
#[tauri::command]
pub async fn get_chat_prompt(name: String, args: serde_json::Value) -> Result<String, String> {
    let rendered = gitnexus_mcp::prompts::get_prompt(&name, &args)
        .ok_or_else(|| format!("Unknown MCP prompt: '{}'", name))?;
    rendered
        .get("messages")
        .and_then(|m| m.as_array())
        .and_then(|a| a.first())
        .and_then(|m| m.get("content"))
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            format!(
                "MCP prompt '{}' returned an unexpected envelope shape",
                name
            )
        })
}

/// Report whether the active repo's chat is running with embeddings (hybrid
/// BM25+semantic search) or in BM25-only fallback. The UI uses this to show
/// an actionable banner — a degraded chat ought to look different from a
/// healthy one, otherwise users keep typing into a worse-than-necessary
/// experience without knowing why.
#[tauri::command]
pub async fn chat_search_capabilities(
    state: State<'_, AppState>,
) -> Result<ChatSearchCapabilities, String> {
    let (_g, _i, _f, embeddings, embeddings_config, _path) =
        state.get_repo_with_embeddings(None).await?;
    Ok(ChatSearchCapabilities {
        embeddings_loaded: embeddings.is_some() && embeddings_config.is_some(),
        model_name: embeddings_config.as_ref().map(|c| c.model_name.clone()),
        vector_count: embeddings.as_ref().map(|s| s.header.count),
    })
}

/// Payload returned by `chat_retry_tool` — the UI merges the fields back
/// into the corresponding `ToolCall` in the chat-session store.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatToolRetryResult {
    pub tool_call_id: String,
    pub name: String,
    /// Final args used (possibly overridden by the UI). Always valid JSON.
    pub args: String,
    pub result: String,
    pub duration_ms: u64,
    /// "success" or "error" — on parse failure, we still return this
    /// envelope so the UI can store the error message in the tool call.
    pub status: String,
}

/// Payload accepted by `chat_retry_tool`. Kept flat so the Tauri layer
/// can deserialize with camelCase keys from TS.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatToolRetryRequest {
    /// Chat-session id (ignored by the backend — purely an identifier the
    /// UI uses to reconcile the response). Kept for symmetry so we can
    /// persist audit trails later without changing the signature.
    pub session_id: String,
    /// Id of the message that hosts the tool call.
    pub message_id: String,
    /// Tool-call id (stable across retries).
    pub tool_call_id: String,
    /// Tool name. Must match one of `build_tool_descriptors()`.
    pub name: String,
    /// New arguments, as a JSON-encoded string. When `None`, the UI
    /// didn't override anything; backend reuses the prior args.
    #[serde(default)]
    pub new_args: Option<String>,
    /// Prior arguments (JSON-encoded string) — used when `new_args`
    /// is not supplied.
    #[serde(default)]
    pub prior_args: Option<String>,
}

/// Re-execute a tool call. Mirrors the single-shot dispatch inside
/// `chat_ask`'s agentic loop: looks up the active repo, routes the
/// call through `execute_mcp_tool`, and returns the freshly-computed
/// result along with timing metadata.
///
/// The chat executor (`chat_executor.rs`) is intentionally untouched —
/// retries target the user-facing tool calls that the LLM already made
/// inside `chat_ask`. New LLM round-trips are NOT performed here.
#[tauri::command]
pub async fn chat_retry_tool(
    state: State<'_, AppState>,
    request: ChatToolRetryRequest,
) -> Result<ChatToolRetryResult, String> {
    let (graph, indexes, fts_index, embeddings, embeddings_config, repo_path_str) =
        state.get_repo_with_embeddings(None).await?;
    let repo_path = PathBuf::from(&repo_path_str);
    let mcp_backend = Arc::clone(&state.mcp_backend);

    // Pick the arg source in order: new_args overrides prior_args;
    // empty/invalid strings fall back to `{}` so the tool still runs.
    let args_raw = request
        .new_args
        .clone()
        .or_else(|| request.prior_args.clone())
        .unwrap_or_else(|| "{}".to_string());
    // Validate JSON before hitting the tool — surface parse errors back
    // to the UI so the inline editor can flag them.
    if let Err(e) = serde_json::from_str::<serde_json::Value>(&args_raw) {
        return Ok(ChatToolRetryResult {
            tool_call_id: request.tool_call_id,
            name: request.name,
            args: args_raw,
            result: format!("Error: invalid JSON args — {e}"),
            duration_ms: 0,
            status: "error".to_string(),
        });
    }

    let start = std::time::Instant::now();
    let result_text = execute_mcp_tool(
        &request.name,
        &args_raw,
        &repo_path,
        &graph,
        &indexes,
        &fts_index,
        embeddings.as_deref(),
        embeddings_config.as_deref(),
        Some(&mcp_backend),
    )
    .await;
    let duration_ms = start.elapsed().as_millis() as u64;

    // Heuristic: any result that starts with "Error:" is surfaced as a
    // failed retry; the tool dispatcher itself returns that shape on
    // validation failure or unknown tool name.
    let status = if result_text.starts_with("Error:") {
        "error"
    } else {
        "success"
    }
    .to_string();

    Ok(ChatToolRetryResult {
        tool_call_id: request.tool_call_id,
        name: request.name,
        args: args_raw,
        result: result_text,
        duration_ms,
        status,
    })
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

/// Public search function for the executor module (BM25-only path).
pub fn search_relevant_context_pub(
    query: &str,
    graph: &KnowledgeGraph,
    fts_index: &FtsIndex,
    max_results: usize,
) -> Vec<(String, f64)> {
    search_relevant_context(query, graph, fts_index, max_results, None, None)
}

// ─── Context Assembly ────────────────────────────────────────────────

/// Search for symbols relevant to the question.
///
/// When `embeddings` and `embeddings_config` are both `Some`, fuses BM25 with
/// semantic cosine top-K via Reciprocal Rank Fusion (K=60). When either is
/// `None` — the common case for repos that haven't run `gitnexus embed` —
/// degrades silently to BM25 + exact name match (the historical behaviour).
fn search_relevant_context(
    query: &str,
    graph: &KnowledgeGraph,
    fts_index: &FtsIndex,
    max_results: usize,
    embeddings: Option<&EmbeddingStore>,
    embeddings_config: Option<&EmbeddingConfig>,
) -> Vec<(String, f64)> {
    // Pull a wider BM25 pool when fusing — RRF reorders the top-K, so giving
    // it more candidates than `max_results` lets a strong semantic match
    // promote a BM25-rank-15 result past a BM25-rank-3 mediocre one.
    let bm25_pool = if embeddings.is_some() && embeddings_config.is_some() {
        max_results.max(20)
    } else {
        max_results * 2
    };
    let fts_results = fts_index.search(graph, query, None, bm25_pool);

    // Hybrid path: fuse BM25 with semantic via RRF. On any failure (model
    // missing, query embedding all-zeros, malformed store) we drop back to
    // BM25-only — semantic search is a quality lift, not a correctness gate.
    if let (Some(store), Some(cfg)) = (embeddings, embeddings_config) {
        let bm25_wrapped: Vec<gitnexus_search::bm25::BM25SearchResult> = fts_results
            .iter()
            .enumerate()
            .map(|(i, r)| gitnexus_search::bm25::BM25SearchResult {
                file_path: r.file_path.clone(),
                score: r.score,
                rank: i + 1,
                node_id: r.node_id.clone(),
                name: r.name.clone(),
                label: r.label.clone(),
                start_line: r.start_line,
                end_line: r.end_line,
            })
            .collect();

        match gitnexus_search::fusion::hybrid_with_preloaded(
            query,
            &bm25_wrapped,
            &store.entries,
            cfg,
            graph,
            max_results,
        ) {
            Ok(fused) => {
                let mut seen = std::collections::HashSet::new();
                let mut results: Vec<(String, f64)> = Vec::new();
                for h in fused {
                    if seen.insert(h.node_id.clone()) {
                        results.push((h.node_id, h.score));
                    }
                }
                if !results.is_empty() {
                    results.truncate(max_results);
                    return results;
                }
                // Empty fused list (unlikely but possible if both paths returned
                // nothing) — fall through to the BM25 path below so the exact
                // name match still gets a chance.
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "hybrid search failed — falling back to BM25-only for this query"
                );
            }
        }
    }

    // BM25 + exact name match fallback (historical path).
    let mut seen = std::collections::HashSet::new();
    let mut results: Vec<(String, f64)> = Vec::new();

    for fts_result in fts_results {
        if seen.insert(fts_result.node_id.clone()) {
            results.push((fts_result.node_id, fts_result.score));
        }
    }

    // Also search by exact name match in graph. Apply the same path penalty
    // used by FTS so a name hit inside `jquery-ui-1.8.20.min.js` doesn't
    // outscore a real BM25 match in business code.
    let query_lower = query.to_lowercase();
    for node in graph.iter_nodes() {
        if node.properties.name.to_lowercase().contains(&query_lower)
            && seen.insert(node.id.clone())
        {
            let score = gitnexus_db::inmemory::fts::path_weight(&node.properties.file_path);
            results.push((node.id.clone(), score));
        }
    }

    // Sort by score descending, take top N
    results.sort_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap_or_else(|| {
            // Handle NaN: treat NaN as less than any number
            if a.1.is_nan() {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            }
        })
    });
    results.truncate(max_results);
    results
}

// ─── DocChunk RAG (P0.3) ─────────────────────────────────────────────

/// Maximum DocChunks to surface in the prefetch block. Three is the sweet
/// spot per audit testing on Alise_v2 (137 enriched pages): four+ starts
/// diluting the LLM's attention without bringing new information.
const DOC_CHUNK_TOP_N: usize = 3;

/// Truncate each chunk's content to keep the prefetch payload bounded. Whole
/// pages can be 5-10KB; the LLM only needs an excerpt to decide whether the
/// chunk is actually relevant or a false positive.
const DOC_CHUNK_EXCERPT_BYTES: usize = 600;

/// Hybrid BM25+semantic search over DocChunk nodes. Reuses the same RRF
/// fusion as `search_relevant_context` then filters down to the doc nodes,
/// so a question phrased differently from the chunk title still finds the
/// chunk via embedding similarity (the original CONTAINS-on-first-keyword
/// approach missed those entirely).
fn prefetch_doc_chunks_hybrid(
    question: &str,
    graph: &KnowledgeGraph,
    fts_index: &FtsIndex,
    embeddings: Option<&EmbeddingStore>,
    embeddings_config: Option<&EmbeddingConfig>,
) -> Option<String> {
    // Pull a wider pool than needed — RRF + filter-by-label means many of the
    // top hits are probably code symbols, not chunks.
    const POOL: usize = 25;
    let hits = search_relevant_context(
        question,
        graph,
        fts_index,
        POOL,
        embeddings,
        embeddings_config,
    );

    let mut chunks: Vec<(String, String)> = Vec::new();
    for (node_id, _score) in hits {
        let node = match graph.get_node(&node_id) {
            Some(n) => n,
            None => continue,
        };
        if node.label != NodeLabel::DocChunk {
            continue;
        }
        let title = node
            .properties
            .title
            .clone()
            .or_else(|| Some(node.properties.name.clone()))
            .unwrap_or_default();
        let content = node.properties.content.clone().unwrap_or_default();
        let excerpt = truncate_at_char_boundary(&content, DOC_CHUNK_EXCERPT_BYTES);
        chunks.push((title, excerpt));
        if chunks.len() >= DOC_CHUNK_TOP_N {
            break;
        }
    }

    if chunks.is_empty() {
        return None;
    }

    let mut out = String::with_capacity(chunks.len() * (DOC_CHUNK_EXCERPT_BYTES + 100));
    for (i, (title, excerpt)) in chunks.iter().enumerate() {
        out.push_str(&format!("**{}. {}**\n\n", i + 1, title));
        out.push_str("> ");
        out.push_str(&excerpt.replace('\n', "\n> "));
        out.push_str("\n\n");
    }
    Some(out)
}

/// Lexical fallback when no embeddings are available — runs the original
/// Cypher CONTAINS on the first sufficiently-long keyword.
async fn prefetch_doc_chunks_lexical(
    question: &str,
    graph: &KnowledgeGraph,
    indexes: &gitnexus_db::inmemory::cypher::GraphIndexes,
    fts_index: &FtsIndex,
    repo_path: &Path,
) -> Option<String> {
    let kw = question.split_whitespace().find(|w| w.len() >= 5)?;
    let cypher = format!(
        "MATCH (n:DocChunk) WHERE n.content CONTAINS '{}' RETURN n.title, n.content LIMIT 3",
        kw.replace('\'', "\\'")
    );
    let result = execute_mcp_tool(
        "execute_cypher",
        &serde_json::json!({"query": cypher}).to_string(),
        repo_path,
        graph,
        indexes,
        fts_index,
        None,
        None,
        None,
    )
    .await;
    if result.len() > 50 && !result.contains("[]") {
        Some(result)
    } else {
        None
    }
}

fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut cut = max_bytes;
    while cut > 0 && !s.is_char_boundary(cut) {
        cut -= 1;
    }
    let mut out = s[..cut].to_string();
    out.push_str(" […]");
    out
}

// ─── Question Classification ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum QuestionType {
    Lookup,       // "où est", "what is", "défini dans"
    Functional,   // "comment fonctionne", "explain", "expliquer le module"
    Algorithm,    // "comment calculé/généré", "traitement", "algorithme"
    Architecture, // "architecture", "vue d'ensemble", "schéma global"
    Impact,       // "impact", "dépendances", "qui appelle", "blast radius"
}

fn classify_question(q: &str) -> QuestionType {
    let l = q.to_lowercase();

    // Impact patterns — highest priority (specific intent)
    if l.contains("impact")
        || l.contains("dépendanc")
        || l.contains("dependenc")
        || l.contains("qui appelle")
        || l.contains("who calls")
        || l.contains("blast radius")
        || l.contains("casse")
        || l.contains("impacté")
        || l.contains("affected")
    {
        return QuestionType::Impact;
    }

    // Architecture / overview
    if l.contains("architecture")
        || l.contains("vue d'ensemble")
        || l.contains("overview")
        || l.contains("big picture")
        || l.contains("schéma global")
        || l.contains("présentation générale")
        || l.contains("global") && (l.contains("fonctionn") || l.contains("architect"))
    {
        return QuestionType::Architecture;
    }

    // Algorithm / process — "comment X est calculé/généré/traité" or "how is X computed"
    if (l.contains("comment") || l.contains("how"))
        && (l.contains("calculé")
            || l.contains("calculated")
            || l.contains("computed")
            || l.contains("généré")
            || l.contains("generated")
            || l.contains("traitement")
            || l.contains("traité")
            || l.contains("construit")
            || l.contains("built")
            || l.contains("sont "))
        || l.contains("algorithme")
        || l.contains("algorithm")
        || l.contains("étape")
        || l.contains("step by step")
        || l.contains("describe the algorithm")
        || l.contains("décrire l'algorithme")
    {
        return QuestionType::Algorithm;
    }

    // Functional explanation
    if l.contains("comment fonctionne")
        || l.contains("how does")
        || l.contains("how do")
        || l.contains("expliquer")
        || l.contains("expliqu")
        || l.contains("explain")
        || l.contains("describe")
        || l.contains("décrire")
        || l.contains("présenter le module")
        || l.contains("présente le module")
        || l.contains("module")
            && (l.contains("explique") || l.contains("décri") || l.contains("fonctionn"))
    {
        return QuestionType::Functional;
    }

    // Lookup — simple locate/define queries
    if l.contains("où est")
        || l.contains("where is")
        || l.contains("where's")
        || l.contains("défini dans")
        || l.contains("defined in")
        || l.contains("defined")
        || l.contains("qu'est-ce que")
        || l.contains("c'est quoi")
        || l.contains("what is")
        || l.contains("trouve")
        || l.contains("find")
        || l.contains("locate")
    {
        return QuestionType::Lookup;
    }

    // Default: treat as functional explanation
    QuestionType::Functional
}

/// Extract the likely module, class, controller, service, or method target from a question.
fn detect_target_symbol(question: &str) -> Option<String> {
    let normalized = question
        .replace("l'", "l' ")
        .replace("L'", "l' ")
        .replace("l’", "l' ")
        .replace("L’", "l' ");
    let tokens: Vec<String> = normalized
        .split_whitespace()
        .map(clean_target_token)
        .filter(|token| !token.is_empty())
        .collect();

    for (idx, token) in tokens.iter().enumerate() {
        if is_target_cue(token) {
            if let Some(target) = tokens
                .iter()
                .skip(idx + 1)
                .filter(|candidate| !is_target_stop_word(candidate))
                .find(|candidate| is_target_candidate_after_cue(candidate))
            {
                return Some(target.clone());
            }
        }
    }

    tokens.into_iter().find(|token| is_symbol_like_token(token))
}

/// Normalize a possible symbol token while preserving namespace/member separators.
fn clean_target_token(token: &str) -> String {
    token
        .trim_matches(|c: char| !c.is_alphanumeric() && !matches!(c, '_' | '.' | ':'))
        .to_string()
}

/// Return whether a token announces that a target symbol likely follows.
fn is_target_cue(token: &str) -> bool {
    let lower = token.to_lowercase();
    matches!(
        lower.as_str(),
        "module"
            | "classe"
            | "class"
            | "controller"
            | "controleur"
            | "contrôleur"
            | "service"
            | "méthode"
            | "methode"
            | "method"
            | "repository"
            | "le"
            | "la"
            | "l"
    )
}

/// Return whether a token is connective prose rather than a target symbol.
fn is_target_stop_word(token: &str) -> bool {
    let lower = token.to_lowercase();
    matches!(
        lower.as_str(),
        "a" | "an"
            | "and"
            | "about"
            | "comment"
            | "de"
            | "des"
            | "du"
            | "explique"
            | "expliquer"
            | "fonctionne"
            | "for"
            | "la"
            | "le"
            | "les"
            | "l"
            | "module"
            | "of"
            | "présente"
            | "presente"
            | "the"
    )
}

/// Return whether a cue-following token looks like a concrete symbol or module name.
fn is_target_candidate_after_cue(token: &str) -> bool {
    starts_with_uppercase(token)
        || has_symbol_suffix(token)
        || token.contains("::")
        || token.contains('.')
}

/// Return whether a standalone token looks like a symbol rather than sentence text.
fn is_symbol_like_token(token: &str) -> bool {
    if token.len() < 3 || is_target_stop_word(token) {
        return false;
    }

    if has_symbol_suffix(token) {
        return true;
    }

    if token.contains("::") || token.contains('.') || token.contains('_') {
        return true;
    }

    starts_with_uppercase(token)
        && token.chars().skip(1).any(|c| c.is_uppercase())
        && token.chars().any(|c| c.is_lowercase())
}

/// Return whether a token starts with an uppercase character.
fn starts_with_uppercase(token: &str) -> bool {
    token.chars().next().is_some_and(|c| c.is_uppercase())
}

/// Return whether a token has a common code-symbol suffix.
fn has_symbol_suffix(token: &str) -> bool {
    let lower = token.to_lowercase();
    [
        "controller",
        "service",
        "repository",
        "manager",
        "provider",
        "context",
        "dbcontext",
        "viewmodel",
        "helper",
        "factory",
    ]
    .iter()
    .any(|suffix| lower.ends_with(suffix))
}

fn canvas_instruction(qt: QuestionType) -> &'static str {
    match qt {
        QuestionType::Lookup => {
            "\
[CANEVAS TYPE A — LOOKUP]\n\
Structure ta réponse ainsi :\n\
## [Nom du symbole] — Définition\n\
**Type :** [Class/Method/Controller/Service]\n\
**Localisation :** `Fichier.cs:ligne`\n\
**Rôle :** [1 phrase]\n\
**Appelé par :** [callers]\n\
**Voir aussi :** [modules liés]\n"
        }

        QuestionType::Functional => {
            "\
[CANEVAS TYPE B — FONCTIONNEL]\n\
Structure ta réponse ainsi :\n\
## Vue d'ensemble\n\
[2-3 phrases depuis la documentation enrichie]\n\
## Diagramme d'appels\n\
[Inclus le diagramme Mermaid pré-chargé]\n\
## Flux de traitement\n\
[Inclus un diagramme Mermaid **sequenceDiagram** montrant les interactions entre composants]\n\
## Méthodes clés\n\
| Méthode | Fichier:Ligne | Rôle |\n\
| :--- | :--- | :--- |\n\
| ... | ... | ... |\n\
## Fonctionnement détaillé\n\
[Description fonctionnelle avec sections logiques]\n\
## Intégration avec d'autres modules\n\
[Explique comment ce module interagit avec les autres (Injections, Events, API) basé sur les imports/dépendances]\n\
## Voir aussi\n\
[cross-références]\n"
        }

        QuestionType::Algorithm => {
            "\
[CANEVAS TYPE D — ORGANIGRAMME OBLIGATOIRE]\n\
\n\
TON PREMIER ELEMENT doit etre un organigramme Mermaid flowchart TD complet.\n\
Utilise le squelette et les sources pre-charges pour construire les conditions.\n\
\n\
## Organigramme : [Nom du traitement]\n\
```mermaid\n\
flowchart TD\n\
    A([Declenchement: NomMethode\\nFichier.cs:ligne]) --> B[Etape 1\\nFichier.cs:ligne]\n\
    B --> C{Condition en langage metier?}\n\
    C -- Oui --> D[Traitement A\\nFichier.cs:ligne]\n\
    C -- Non --> E([Erreur/Abandon])\n\
    D --> F[(BDD: SaveChanges)]\n\
    F --> G([Fin: Succes])\n\
```\n\
\n\
REGLES Mermaid :\n\
- [action] = rectangles pour les actions\n\
- {condition?} = losanges pour if/else (en langage metier, pas code)\n\
- [(BDD: op)] = cylindres pour base de donnees\n\
- ([debut/fin]) = ronds pour entree et sortie\n\
- Chaque noeud : annotation sur nouvelle ligne avec Fichier.cs:ligne\n\
- Branches nommees : -- Oui -->, -- Non -->, -- Fournisseur -->\n\
- Max 25 noeuds, inclure cas d'erreur\n\
\n\
APRES le flowchart :\n\
## Etapes detaillees\n\
[Numerotees, citations Fichier.cs:ligne, conditions si/sinon explicites]\n\
## Points d'attention\n\
[Bugs connus, cas limites, regles metier cachees]\n"
        }

        QuestionType::Architecture => {
            "\
[CANEVAS TYPE C — ARCHITECTURE]\n\
Structure ta réponse ainsi :\n\
## Architecture globale\n\
[Diagramme Mermaid pré-chargé]\n\
## Modules principaux\n\
| Module | Rôle | Symboles |\n\
...\n\
## Flux de données\n\
[Description des flux entre modules]\n\
## Points d'entrée\n\
[Controllers/Services principaux]\n"
        }

        QuestionType::Impact => {
            "\
[CANEVAS TYPE E — IMPACT]\n\
Structure ta réponse ainsi :\n\
## Blast radius : [Symbole X]\n\
**Impactés en amont (qui appellent X) :**\n\
- [liste avec fichiers]\n\
**Impactés en aval (ce que X appelle) :**\n\
- [liste avec fichiers]\n\
## Risque de modification\n\
[LOW/MEDIUM/HIGH avec justification]\n\
## Recommandations\n\
[que faire si on modifie X]\n"
        }
    }
}

/// Resolve a symbol name to the graph node best suited for prefetch targeting.
fn resolve_symbol_node_id(symbol: &str, graph: &KnowledgeGraph) -> Option<String> {
    let symbol_lower = symbol.to_lowercase();
    graph
        .iter_nodes()
        .filter(|node| node.properties.name.to_lowercase() == symbol_lower)
        .min_by_key(|node| prefetch_symbol_priority(node.label))
        .map(|node| node.id.clone())
}

/// Rank duplicate symbol-name matches so module-level nodes win over members.
fn prefetch_symbol_priority(label: NodeLabel) -> u8 {
    match label {
        NodeLabel::Controller => 0,
        NodeLabel::Class => 1,
        NodeLabel::Service => 2,
        NodeLabel::Repository => 3,
        NodeLabel::Method | NodeLabel::Function | NodeLabel::ControllerAction => 4,
        _ => 5,
    }
}

/// Collect method names to prefetch from a targeted symbol and its direct calls.
fn collect_prefetch_method_names(
    root_id: &str,
    graph: &KnowledgeGraph,
    indexes: &gitnexus_db::inmemory::cypher::GraphIndexes,
) -> Vec<String> {
    let mut names = Vec::new();
    let mut seen_names = std::collections::HashSet::new();
    let mut method_ids = Vec::new();

    push_prefetch_method_id(root_id, graph, &mut method_ids);

    if let Some(outgoing) = indexes.outgoing.get(root_id) {
        for (target_id, rel_type) in outgoing {
            if matches!(
                rel_type,
                RelationshipType::HasMethod | RelationshipType::HasAction
            ) {
                push_prefetch_method_id(target_id, graph, &mut method_ids);
            }
        }
    }

    for method_id in method_ids {
        push_prefetch_method_name(&method_id, graph, &mut names, &mut seen_names);
        if names.len() >= 5 {
            break;
        }

        if let Some(callees) = indexes.outgoing.get(&method_id) {
            // BUG FIX: filter BEFORE take — otherwise non-Calls edges consume the
            // 4-slot budget and real Calls edges beyond slot 4 are silently dropped.
            for (callee_id, _rel_type) in callees
                .iter()
                .filter(|(_, rel)| {
                    matches!(
                        rel,
                        RelationshipType::Calls
                            | RelationshipType::CallsAction
                            | RelationshipType::CallsService
                    )
                })
                .take(4)
            {
                push_prefetch_method_name(callee_id, graph, &mut names, &mut seen_names);
                if names.len() >= 5 {
                    break;
                }
            }
        }

        if names.len() >= 5 {
            break;
        }
    }

    names
}

/// Append a method-like node ID if the graph node can be read as method source.
fn push_prefetch_method_id(node_id: &str, graph: &KnowledgeGraph, method_ids: &mut Vec<String>) {
    if graph
        .get_node(node_id)
        .is_some_and(|node| is_prefetch_method_label(node.label))
    {
        method_ids.push(node_id.to_string());
    }
}

/// Append a unique method name for `read_full_method`.
fn push_prefetch_method_name(
    node_id: &str,
    graph: &KnowledgeGraph,
    names: &mut Vec<String>,
    seen_names: &mut std::collections::HashSet<String>,
) {
    let Some(node) = graph.get_node(node_id) else {
        return;
    };
    if !is_prefetch_method_label(node.label) || node.properties.name.is_empty() {
        return;
    }

    let dedupe_key = node.properties.name.to_lowercase();
    if seen_names.insert(dedupe_key) {
        names.push(node.properties.name.clone());
    }
}

/// Return whether a node label has method/function source that can be prefetched.
fn is_prefetch_method_label(label: NodeLabel) -> bool {
    matches!(
        label,
        NodeLabel::Method
            | NodeLabel::Function
            | NodeLabel::ControllerAction
            | NodeLabel::Constructor
    )
}

/// Pre-fetch tool results for the detected question type.
/// Returns a formatted string injected as additional system context.
#[allow(clippy::too_many_arguments)]
async fn prefetch_for_type(
    qt: QuestionType,
    question: &str,
    search_results: &[(String, f64)],
    graph: &KnowledgeGraph,
    indexes: &gitnexus_db::inmemory::cypher::GraphIndexes,
    fts_index: &gitnexus_db::inmemory::fts::FtsIndex,
    repo_path: &std::path::Path,
    embeddings: Option<&EmbeddingStore>,
    embeddings_config: Option<&EmbeddingConfig>,
    mcp_backend: Option<&Arc<TokioMutex<LocalBackend>>>,
) -> String {
    let mut pre = String::new();

    // Extract top symbol name for tool calls
    let detected_target = detect_target_symbol(question);
    let top_symbol_id = detected_target
        .as_deref()
        .and_then(|target| resolve_symbol_node_id(target, graph))
        .or_else(|| {
            search_results
                .first()
                .and_then(|(id, _)| graph.get_node(id).map(|_| id.clone()))
        });
    let top_symbol_name = top_symbol_id
        .as_deref()
        .and_then(|id| graph.get_node(id))
        .map(|n| n.properties.name.clone())
        .unwrap_or_default();

    // RAG DocChunk retrieval. When embeddings are loaded, use hybrid
    // BM25+semantic on the full question and keep only DocChunk hits — that
    // surfaces the most semantically relevant chunks (e.g. a question about
    // "fusion Indu/Compta" finds the matching SFD section even if the chunk
    // doesn't share keywords with the question). Falls back to BM25-on-first-
    // keyword via Cypher CONTAINS when no embeddings are available, which is
    // the historical behaviour and good enough for keyword-style queries.
    let chunk_block = if embeddings.is_some() && embeddings_config.is_some() {
        prefetch_doc_chunks_hybrid(question, graph, fts_index, embeddings, embeddings_config)
    } else {
        prefetch_doc_chunks_lexical(question, graph, indexes, fts_index, repo_path).await
    };
    if let Some(block) = chunk_block {
        pre.push_str("## Documentation RAG (DocChunk)\n\n");
        pre.push_str(&block);
        pre.push_str("\n\n");
    }

    match qt {
        QuestionType::Functional | QuestionType::Algorithm => {
            if qt == QuestionType::Functional {
                if let Some(target) = detected_target.as_deref() {
                    let doc = load_enriched_module_doc_page(target, repo_path);
                    if !doc.is_empty() {
                        pre.push_str(&format!("## Documentation du module {}\n\n", target));
                        pre.push_str(&doc);
                        pre.push_str("\n\n");
                    }
                }

                let process_keyword = detected_target.as_deref().unwrap_or(&top_symbol_name);
                if !process_keyword.is_empty() {
                    let process_flow = execute_mcp_tool(
                        "get_process_flow",
                        &serde_json::json!({"keyword": process_keyword}).to_string(),
                        repo_path,
                        graph,
                        indexes,
                        fts_index,
                        embeddings,
                        embeddings_config,
                        mcp_backend,
                    )
                    .await;
                    if process_flow.len() > 50
                        && !process_flow.starts_with("Error:")
                        && !process_flow.contains("0 results")
                    {
                        pre.push_str("## Processus métier pré-chargés\n\n");
                        pre.push_str(&process_flow);
                        pre.push_str("\n\n");
                    }
                }
            }

            if !top_symbol_name.is_empty() {
                // get_symbol_context for the top symbol
                let ctx = execute_mcp_tool(
                    "get_symbol_context",
                    &serde_json::json!({"symbol": top_symbol_name}).to_string(),
                    repo_path,
                    graph,
                    indexes,
                    fts_index,
                    embeddings,
                    embeddings_config,
                    mcp_backend,
                )
                .await;
                if ctx.len() > 50 {
                    pre.push_str("## Contexte du symbole principal\n\n");
                    pre.push_str(&ctx);
                    pre.push_str("\n\n");
                }
                // get_diagram for functional questions
                if qt == QuestionType::Functional {
                    let diag = execute_mcp_tool(
                        "get_diagram",
                        &serde_json::json!({"target": top_symbol_name}).to_string(),
                        repo_path,
                        graph,
                        indexes,
                        fts_index,
                        embeddings,
                        embeddings_config,
                        mcp_backend,
                    )
                    .await;
                    if diag.len() > 30 {
                        pre.push_str("## Diagramme d'appels pré-chargé\n\n");
                        pre.push_str(&diag);
                        pre.push_str("\n\n");
                    }
                }
                // Algorithm: read FULL call chain — top method + its callees (up to 5 total)
                if qt == QuestionType::Algorithm {
                    let methods_to_read = top_symbol_id
                        .as_deref()
                        .map(|top_id| collect_prefetch_method_names(top_id, graph, indexes))
                        .unwrap_or_default();

                    // Read each method in full (250 lines cap via read_full_method)
                    if !methods_to_read.is_empty() {
                        pre.push_str("## Chaîne de traitement pré-chargée (sources complètes)\n\n");
                        for method_name in &methods_to_read {
                            let code = read_full_method(method_name, graph, repo_path).await;
                            if code.len() > 80 {
                                pre.push_str(&code);
                                pre.push_str("\n\n");
                            }
                        }
                    }

                    // Add skeleton flowchart (topology without conditions)
                    if let Some(top_id) = top_symbol_id.as_deref() {
                        let skeleton = crate::commands::diagram::build_skeleton_flowchart(
                            graph, indexes, top_id,
                        );
                        if !skeleton.is_empty() {
                            pre.push_str("## Squelette d'organigramme (topologie — à enrichir avec les conditions)\n\n");
                            pre.push_str("```mermaid\n");
                            pre.push_str(&skeleton);
                            pre.push_str("\n```\n\n");
                        }
                    }
                }
            }
        }
        QuestionType::Architecture => {
            // List top communities
            let cypher =
                "MATCH (n:Community) RETURN n.name, n.description, n.member_count LIMIT 15";
            let communities = execute_mcp_tool(
                "execute_cypher",
                &serde_json::json!({"query": cypher}).to_string(),
                repo_path,
                graph,
                indexes,
                fts_index,
                embeddings,
                embeddings_config,
                mcp_backend,
            )
            .await;
            if communities.len() > 30 {
                pre.push_str("## Modules fonctionnels (pré-chargés)\n\n");
                pre.push_str(&communities);
                pre.push_str("\n\n");
            }
            // Diagram of top module
            if !top_symbol_name.is_empty() {
                let diag = execute_mcp_tool(
                    "get_diagram",
                    &serde_json::json!({"target": top_symbol_name}).to_string(),
                    repo_path,
                    graph,
                    indexes,
                    fts_index,
                    embeddings,
                    embeddings_config,
                    mcp_backend,
                )
                .await;
                if diag.len() > 30 {
                    pre.push_str("## Diagramme architecture pré-chargé\n\n");
                    pre.push_str(&diag);
                    pre.push_str("\n\n");
                }
            }
        }
        QuestionType::Impact => {
            if !top_symbol_name.is_empty() {
                let impact = execute_mcp_tool(
                    "get_impact",
                    &serde_json::json!({"target": top_symbol_name, "direction": "both", "max_depth": 4}).to_string(),
                    repo_path,
                    graph,
                    indexes,
                    fts_index,
                    embeddings,
                    embeddings_config,
                    mcp_backend,
                )
                .await;
                if impact.len() > 50 {
                    pre.push_str("## Blast radius pré-chargé\n\n");
                    pre.push_str(&impact);
                    pre.push_str("\n\n");
                }
            }
        }
        QuestionType::Lookup => {
            // search_code already done via search_relevant_context
        }
    }

    pre
}

/// Read enriched documentation pages for the nodes most relevant to the query.
///
/// The enriched .md pages (in `.gitnexus/docs/`) contain LLM-generated descriptions,
/// call graphs, evidence refs — much richer context than raw code snippets.
/// This function maps node types → page paths and returns the markdown content.
fn load_enriched_doc_pages(
    results: &[(String, f64)],
    graph: &KnowledgeGraph,
    repo_path: &Path,
) -> String {
    let docs_dir = repo_path.join(".gitnexus").join("docs");
    if !docs_dir.exists() {
        return String::new();
    }

    let mut seen_pages = std::collections::HashSet::new();
    let mut content = String::new();

    for (node_id, _score) in results.iter().take(6) {
        let node = match graph.get_node(node_id) {
            Some(n) => n,
            None => continue,
        };

        // Map node label + name to a documentation page path
        let candidates = doc_page_candidates(&node.label, &node.properties.name);

        for candidate in candidates {
            if seen_pages.contains(&candidate) {
                continue;
            }
            let page_path = docs_dir.join(&candidate);
            if page_path.exists() {
                if let Ok(text) = std::fs::read_to_string(&page_path) {
                    if let Some(page) = format_enriched_doc_page(&candidate, &text) {
                        seen_pages.insert(candidate.clone());
                        content.push_str(&page);
                        break; // first matching page per node is enough
                    }
                }
            }
        }
    }

    content
}

/// Read a targeted enriched module documentation page by direct module filename.
fn load_enriched_module_doc_page(module_name: &str, repo_path: &Path) -> String {
    let modules_dir = repo_path.join(".gitnexus").join("docs").join("modules");
    if !modules_dir.exists() {
        return String::new();
    }

    let sanitized = sanitize_doc_segment(module_name);
    let mut candidates = vec![
        format!("{}.md", sanitized),
        format!("ctrl-{}.md", sanitized),
    ];
    if let Some(base_name) = strip_symbol_suffix(&sanitized) {
        candidates.push(format!("{}.md", base_name));
        candidates.push(format!("ctrl-{}.md", base_name));
    }

    for candidate in candidates {
        let page_path = modules_dir.join(&candidate);
        if page_path.exists() {
            if let Ok(text) = std::fs::read_to_string(&page_path) {
                if let Some(page) =
                    format_enriched_doc_page(&format!("modules/{}", candidate), &text)
                {
                    return page;
                }
            }
        }
    }

    String::new()
}

/// Strip common code-role suffixes from a sanitized module name.
fn strip_symbol_suffix(name: &str) -> Option<String> {
    // BUG FIX: aligned with has_symbol_suffix — was missing 8 suffixes
    // (provider, context, dbcontext, viewmodel, helper, factory + 2 more)
    for suffix in [
        "controller",
        "service",
        "manager",
        "repository",
        "provider",
        "context",
        "dbcontext",
        "viewmodel",
        "helper",
        "factory",
    ] {
        if let Some(base) = name.strip_suffix(suffix) {
            if !base.is_empty() {
                return Some(base.trim_end_matches('-').to_string());
            }
        }
    }
    None
}

/// Format one enriched documentation page for injection into the chat prompt.
fn format_enriched_doc_page(candidate: &str, text: &str) -> Option<String> {
    // Only include pages with meaningful enrichment (> 200 chars).
    if text.len() <= 200 {
        return None;
    }

    let clean: String = text
        .lines()
        .filter(|l| !l.trim().starts_with("<!-- GNX:"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut trimmed = clean.chars().take(3000).collect::<String>();
    if clean.chars().count() > 3000 {
        trimmed.push_str("\n\n*[…tronqué]*");
    }

    Some(format!(
        "### Documentation enrichie : `{}`\n\n{}\n\n---\n\n",
        candidate, trimmed
    ))
}

/// Convert a symbol/module name into a documentation filename segment.
fn sanitize_doc_segment(name: &str) -> String {
    name.to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .collect()
}

/// Return candidate doc page paths (relative to docs_dir) for a given node.
fn doc_page_candidates(label: &NodeLabel, name: &str) -> Vec<String> {
    let sanitized = sanitize_doc_segment(name);

    match label {
        NodeLabel::Controller => vec![
            format!("modules/ctrl-{}.md", sanitized),
            format!("modules/{}.md", sanitized),
        ],
        NodeLabel::Service | NodeLabel::Repository => vec![
            format!("modules/services.md"),
            format!("modules/{}.md", sanitized),
        ],
        NodeLabel::DbEntity | NodeLabel::DbContext => vec![
            format!("modules/data-{}.md", sanitized),
            format!("aspnet-entities.md"),
            format!("aspnet-data-model.md"),
        ],
        NodeLabel::View | NodeLabel::PartialView => {
            vec![format!("modules/views.md"), format!("aspnet-views.md")]
        }
        NodeLabel::ExternalService => vec![
            format!("modules/external-services.md"),
            format!("aspnet-external.md"),
        ],
        NodeLabel::Community => vec![format!("modules/{}.md", sanitized)],
        NodeLabel::Process => vec![format!("processes/{}.md", sanitized)],
        _ => vec![format!("modules/{}.md", sanitized)],
    }
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
            read_code_snippet(
                repo_path,
                &node.properties.file_path,
                node.properties.start_line,
                node.properties.end_line,
            )
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
            callers: if callers.is_empty() {
                None
            } else {
                Some(callers)
            },
            callees: if callees.is_empty() {
                None
            } else {
                Some(callees)
            },
            community,
            relevance_score: *score,
        });
    }

    sources
}

/// Read the COMPLETE source of a method with line numbers — no 50-line cap (up to 250).
/// Used for algorithm questions where the full if/else/loop structure is needed.
async fn read_full_method(symbol: &str, graph: &KnowledgeGraph, repo_path: &Path) -> String {
    // Find the node by name (case-insensitive)
    let node = graph.iter_nodes().find(|n| {
        n.properties.name.eq_ignore_ascii_case(symbol)
            && matches!(
                n.label,
                NodeLabel::Method
                    | NodeLabel::Function
                    | NodeLabel::ControllerAction
                    | NodeLabel::Constructor
            )
    });

    let node = match node {
        Some(n) => n,
        None => return format!("Symbol '{}' not found in graph.\n", symbol),
    };

    let file_path = &node.properties.file_path;
    let start = node.properties.start_line;
    let end = node.properties.end_line;

    if file_path.is_empty() {
        return format!("No file path for '{}'.\n", symbol);
    }

    let full_path = repo_path.join(file_path);
    let canonical_repo = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());
    match full_path.canonicalize() {
        Ok(c) if !c.starts_with(&canonical_repo) => {
            return format!("Path traversal blocked for '{}'.\n", file_path);
        }
        Err(_) => return format!("File not found: {}\n", file_path),
        _ => {}
    }

    let content = match std::fs::read_to_string(&full_path) {
        Ok(c) => c,
        Err(e) => return format!("Cannot read {}: {}\n", file_path, e),
    };

    let lines: Vec<&str> = content.lines().collect();
    let start_idx = start
        .map(|s| s.saturating_sub(1) as usize)
        .unwrap_or(0)
        .min(lines.len());
    let end_idx = end
        .map(|e| e as usize)
        .unwrap_or(start_idx + 60)
        .min(lines.len());
    // Cap at 250 lines (vs 50 for read_file)
    let end_idx = end_idx.min(start_idx + 250);

    if start_idx >= end_idx {
        return format!("Empty range for '{}'.\n", symbol);
    }

    let lang = detect_language(file_path);
    let numbered: String = lines[start_idx..end_idx]
        .iter()
        .enumerate()
        .map(|(i, l)| format!("{:4}: {}", start_idx + i + 1, l))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Method: `{}` @ `{}:{}–{}`\n```{}\n{}\n```\n",
        node.properties.name,
        file_path,
        start_idx + 1,
        end_idx,
        lang,
        numbered
    )
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
            if start >= end {
                return None;
            }
            Some(lines[start..end].join("\n"))
        }
        (Some(start), None) => {
            let start = std::cmp::min((start.saturating_sub(1)) as usize, lines.len());
            let end = std::cmp::min(start + 20, lines.len());
            if start >= end {
                return None;
            }
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

fn build_system_prompt(
    graph: &KnowledgeGraph,
    sources: &[ChatSource],
    repo_path: &Path,
    enriched_doc_context: &str,
    prefetched: &str,
) -> String {
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

    // ── Règle fondamentale : Organigramme en premier ─────────────
    prompt.push_str(
        "# Règle fondamentale — Organigramme en premier\n\n\
         Pour toute question sur un **TRAITEMENT**, **CALCUL**, **GÉNÉRATION** ou **ALGORITHME** :\n\
         **TON PREMIER ÉLÉMENT DE RÉPONSE DOIT ÊTRE UN ORGANIGRAMME MERMAID `flowchart TD`.**\n\n\
         Le flowchart doit être autonome et complet — quelqu'un qui le lit sans le texte comprend le traitement.\n\n\
         Sources dans l'ordre de priorité :\n\
         1. **Code source complet** (`read_method`) → conditions réelles if/else/switch\n\
         2. **Squelette topologique pré-chargé** → ordre correct des étapes\n\
         3. **Documentation enrichie** (.gitnexus/docs/) → description fonctionnelle\n\
         4. **RAG DocChunk** → specs et spécifications\n\n\
         Pour les questions de type **Lookup/Impact** uniquement : pas d'organigramme requis.\n\n"
    );

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
        prompt.push_str(&format!(
            "- **Frameworks detected**: {}\n",
            meta.frameworks.join(", ")
        ));
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

    // ── Pre-fetched tool results (deterministic, before LLM call) ───
    if !prefetched.is_empty() {
        prompt.push_str("# Contexte pré-chargé (résultats d'outils vérifiés)\n\n");
        prompt.push_str("Ces données ont été extraites directement du graphe et du code. ");
        prompt.push_str(
            "Cite-les directement dans ta réponse sans re-appeler les outils correspondants.\n\n",
        );
        prompt.push_str(prefetched);
    }

    // ── Enriched documentation pages ────────────────────────────
    // LLM-generated descriptions of relevant modules — primary context source
    if !enriched_doc_context.is_empty() {
        prompt.push_str("# Enriched module documentation\n\n");
        prompt.push_str("The following pages were auto-generated by analyzing the codebase and enriched by an LLM. ");
        prompt.push_str("They contain functional descriptions, call graphs and evidence — use them as primary context.\n\n");
        prompt.push_str(enriched_doc_context);
    }

    // ── Relevant code context ───────────────────────────────────
    if !sources.is_empty() {
        prompt.push_str("# Relevant code context\n\n");
        prompt.push_str(
            "These symbols are the most relevant to the user's question (ranked by FTS score):\n\n",
        );

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

    // ── Methodology (the "three sources" pattern) ───────────────
    // This prompt mirrors METHODOLOGIE-PRODUCTION-DOC.md v1.0 (12/04/2026),
    // the doc Patrice wrote during the "Nuit des 500 pages" while producing
    // the Alise v2 functional documentation. The same discipline applies
    // here: every factual claim must be grounded in one of three sources,
    // never in guesswork.
    prompt.push_str(
        "# Methodology — the three-sources principle\n\n\
         Every answer you give must be grounded in AT LEAST ONE of these three sources \
         (ideally cross-referenced across two or three for complex questions):\n\n\
         1. **Code graph** (structure, relationships) — accessed via `execute_cypher`, `get_symbol_context`, `get_impact`, `get_diagram`.\n\
         2. **Source code** (exact lines) — accessed via `read_file` after locating targets with `search_code`.\n\
         3. **Functional docs** (RAG-indexed specs: .md, .docx, PDF) — accessed via Cypher on `DocChunk` nodes:\n\
            `MATCH (d:DocChunk) WHERE d.name CONTAINS '<doc_name>' RETURN d.name, d.content LIMIT 30`.\n\n\
         The pre-fetched context above is a **starting point**, NOT the final answer. It comes \
         from a BM25 retrieval that can surface noisy or wrong sources (minified libraries, \
         unrelated tests). **Never trust it blindly** — always verify specific symbols with a \
         tool call before asserting.\n\n\
         ## When to use which tool\n\n\
         | Question shape | Tool chain | Why |\n\
         |---|---|---|\n\
         | *\"comment fonctionne X ?\"* / *\"how does X work?\"* | `search_processes` → `get_process_flow` → `get_symbol_context` → `read_file` on the method body | Business process first, THEN code-level verification |\n\
         | *\"comment X est calculé ?\"* / *\"comment sont …\"* | `search_code` → `get_symbol_context` → **`read_method` on ALL key methods** | Algorithm answers demand real source code, not summaries |\n\
         | *\"explique le module Y\"* / *\"explain module Y\"* | `execute_cypher` to list Y's symbols → `read_file` the top 2-3 methods | Exhaustive listing beats noisy RAG + algorithm detail |\n\
         | *\"où est défini X ?\"* / *\"where is X?\"* | `search_code` on \"X\" | Fast lookup |\n\
         | *\"qu'est-ce qui appelle X ?\"* / *\"what calls X?\"* | `get_impact` direction=upstream | BFS of incoming edges |\n\
         | Specific doc excerpt needed (e.g. *\"que dit la SFD sur Y ?\"*) | `execute_cypher` on `DocChunk` | RAG retrieval is noisy; direct extraction is reliable |\n\
         | Architecture / sequence / flow diagram request | `get_diagram` + `read_file` on the 2-3 pivotal methods | Diagram + algorithmic captions |\n\
         | Read a specific file / more than the 50-line snippet | `read_file` | Exact code |\n\n\
         **Core heuristic for quality answers**: the 50-line snippet you see in the pre-fetched \
         context is often TRUNCATED. For any algorithm question, call `read_method` for the \
         key methods to see the full body, then trace the \
         control flow step-by-step.\n\n\
         ## Rules — applied from the Alise methodology\n\n\
         - **Never invent**: if a tool returns nothing, say so. Do NOT fabricate class names, \
           file paths, method signatures, or CCAS terminology. The methodology's #1 error is \
           *« Inventer des noms de champs »* — don't.\n\
         - **Always cite**: every statement about a specific symbol must be followed by its \
           location, formatted `` `MethodName()` in `path/to/file.cs:123` ``.\n\
         - **Cypher over RAG for specific docs**: if the user names a controller / module / \
           document, query it directly (`execute_cypher` on the code graph or on `DocChunk`). \
           Only use `search_code` (BM25 lexical) for truly open-ended queries.\n\
         - **Quote verbatim**: when RAG functional docs (`DocChunk` content) provide the answer, \
           reproduce the original text between `> *\"...\"*` followed by `— Source: [doc name]`. \
           Do NOT reformulate — authenticity matters.\n\
         - **Cross-reference**: for non-trivial questions, back your answer with TWO of the \
           three sources (e.g. graph structure + source code, or source code + functional doc).\n\
         - **No announcements (HARD RULE)**: when you need information, you MUST emit the \
           tool call in the same turn — never describe what you would search for in plain text \
           and stop. There is no \"next turn for the planning\" — the runtime treats text-only \
           replies as final. Stalling phrases that trigger an automatic fallback search and waste \
           one of your iteration budget slots: *\"Je vais rechercher…\"*, *\"I'll search…\"*, \
           *\"Pour commencer\"*, *\"Let me check\"*, *\"First, I'll…\"*. If you absolutely cannot \
           proceed without something the user can supply (an ambiguous symbol name, a missing \
           file path), ask one focused question — but never narrate intent without acting.\n\
         - **Algorithms, not summaries** (CRITICAL): when asked *\"comment ça marche ?\"* / \
           *\"comment X est calculé ?\"* / any process / treatment / computation question, \
           you MUST describe the **actual algorithm**, not a high-level narrative. This means:\n\
            1. `read_method` the relevant method body so you see the real control flow.\n\
            2. Break the logic into numbered steps (*Étape 1 → Étape 2 → Étape 3…*) that trace \
               the actual if/else/loop structure of the code.\n\
            3. Make every conditional explicit: *« Si `facture.Statut == DemPaiemVal` ET \
               `facture.CodeAuxiliaire != null` alors … Sinon … »*.\n\
            4. Expose input → transformations → output: what parameters does the method receive, \
               what shape is the return, what side-effects (DB writes, log, throws) occur.\n\
            5. Cite the file and line ranges for each step.\n\
            6. Whenever useful, emit a `sequenceDiagram` or `flowchart` Mermaid block that \
               visually traces the algorithm.\n\
           A quality answer looks like: *« Étape 1 (FactureService.cs:42-58) : récupère les \
           `IdAide` distincts de `LigneFacture` via LINQ `.Select(l => l.IdAide).Distinct()`. \
           Étape 2 (:60-74) : pour chaque `IdAide`, interroge `Plafonds` avec filtre temporel \
           `p.DateDebut <= dateDebutPrestation AND p.DateFin >= datefinPresta`. Étape 3 (:76-90) : \
           exclut les unités `Pourcentage` via `p.UniteRef != EnumRefUnitePlafond.Pourcentage`. \
           Étape 4 : retourne `List<Plafond>`. »*\n\n\
         ## Available tools (10)\n\n\
         1. **search_code** — BM25 lexical search. Best for open-ended concept queries.\n\
         2. **read_file** — Read exact lines from a source file.\n\
         3. **get_symbol_context** — Callers, callees, imports, inheritance for one symbol.\n\
         4. **get_impact** — Blast radius via BFS (upstream | downstream | both).\n\
         5. **execute_cypher** — Read-only graph queries. Core queries:\n\
            - `MATCH (n:Class) WHERE n.name = '<X>' RETURN n` — find a class\n\
            - `MATCH (m:Method) WHERE m.filePath CONTAINS '<Controller>' RETURN m.name` — list methods in a file\n\
            - `MATCH (c:Controller)-[:CALLS_SERVICE]->(s:Service) RETURN c.name, s.name` — architecture slice\n\
            - `MATCH (d:DocChunk) WHERE d.name CONTAINS '<doc>' RETURN d.content LIMIT 30` — extract a CCAS / functional doc\n\
            - `MATCH (p:Process) WHERE p.name CONTAINS '<X>' RETURN p` — find a business process\n\
         6. **search_processes** — Search business process flows for workflow and multi-step operation questions.\n\
         7. **get_process_flow** — Targeted business process lookup by keyword.\n\
         8. **get_diagram** — Generate a Mermaid flowchart / sequence / class diagram.\n\
         9. **read_method** — Read complete method source up to 250 lines.\n\
         10. **save_memory** — Persist a fact across sessions (project conventions, user preferences).\n\n\
         The runtime caps the loop at 5 iterations. Be deliberate — no hard limit on calls per \
         response, but each call must advance the answer.\n\n",
    );

    // ── Response format ─────────────────────────────────────────
    prompt.push_str(
        "# Response format\n\n\
         - **Language**: reply in the SAME language as the user's question (French, English, etc.).\n\
         - **Structure**: use `##` markdown headers. On architectural questions, mirror the CCAS \
           SFD template: *Expression du besoin → Exigences → Modèle de données → \
           **Algorithmes** (obligatoire si la question porte sur un traitement) → Diagrammes*.\n\
         - **Algorithm section** (quality-critical): when describing a treatment/computation, \
           use this canonical structure:\n\
           ```\n\
           ### Algorithme — <NomTraitement>\n\
           \n\
           **Entrée** : <paramètres avec types>\n\
           **Sortie** : <type de retour + shape>\n\
           **Effets de bord** : <DB writes / logs / events / throws / aucun>\n\
           \n\
           **Étape 1** (`FichierService.cs:42-58`) — <description précise de ce que fait ce bloc>\n\
           > `LINQ`/pseudocode ou citation courte du code\n\
           \n\
           **Étape 2** (`FichierService.cs:60-74`) — Si `<condition>` alors <action> sinon <action>.\n\
           …\n\
           \n\
           **Invariants** : <règles métier que la méthode garantit>\n\
           **Cas d'erreur** : <exceptions levées, valeurs de retour spéciales>\n\
           ```\n\
         - **Code citations**: symbols and files in backticks with line numbers, e.g. \
           `` `ReglePaiementMasse.GetAideSelectPaiementMasse()` in `CCAS.Alise.BAL/Facture/Regles/ReglePaiementMasse.cs:42` ``.\n\
         - **Doc citations** (when DocChunk content is used): the CCAS verbatim block:\n\
           ```\n\
           > *\"Citation exacte du document d'origine\"*\n\
           > — Source : `NomDocument.docx`, section 2.3\n\
           ```\n\
         - **Code blocks**: fenced with the correct language tag (```csharp, ```typescript, …).\n\
         - **Diagrams**: ```mermaid blocks on architectural or flow questions (sequenceDiagram, \
           classDiagram, stateDiagram, flowchart) — one diagram > five paragraphs of prose. \
           For algorithms, prefer `flowchart TD` with decision nodes that mirror the if/else \
           structure of the code.\n\
         - **Length**: match the question's scope. A \"where is X?\" question gets 2 lines. \
           A \"comment fonctionne le module paiement ?\" question gets a full structured answer \
           WITH an explicit Algorithme section per traitement non-trivial.\n\
         - **Honesty**: if the tools return nothing relevant, say *\"Aucune information trouvée \
           dans le graphe ni la documentation\"* rather than speculate.\n\
         - **Voir aussi**: end complex answers with a `## Voir aussi` (or `## See also`) section \
           listing 3-5 related symbols/modules/docs the user might want to explore next.\n\n\
         ## Canvas par type de question\n\n\
         **TYPE A — Lookup** (où est X, what is X) :\n\
         `## [Symbole] — Définition | Type | Localisation: Fichier.cs:ligne | Rôle | Appelé par | Voir aussi`\n\n\
         **TYPE B — Functional** (comment fonctionne, expliquer le module) :\n\
         `## Vue d'ensemble [depuis doc enrichie] | ## Diagramme d'appels [Mermaid] | ## Fonctionnement | ## Sources clés | ## Voir aussi`\n\n\
         **TYPE C — Architecture** (vue d'ensemble, architecture globale) :\n\
         `## Architecture globale [Mermaid] | ## Modules principaux [table] | ## Flux de données | ## Points d'entrée`\n\n\
         **TYPE D — Algorithm** (comment calculé, comment généré, traitement) :\n\
         OBLIGATOIRE: commencer par `## Organigramme : [Nom]` avec un `flowchart TD` Mermaid complet.\n\
         Puis: `## Etapes détaillées [numérotées avec Fichier.cs:ligne] | ## Points d'attention`\n\n\
         **TYPE E — Impact** (impact, dépendances, blast radius) :\n\
         `## Blast radius: [Symbole] | Impactés en amont | Impactés en aval | ## Risque [LOW/MEDIUM/HIGH] | ## Recommandations`\n",
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

/// Signal the current streaming chat request to stop.
/// The flag is checked between each streamed chunk in `chat_ask`.
#[tauri::command]
pub async fn chat_cancel(state: State<'_, AppState>) -> Result<(), String> {
    state
        .cancel_flag
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

/// Call an OpenAI-compatible LLM API (non-streaming, for the executor module).
async fn call_llm(config: &ChatConfig, messages: &[serde_json::Value]) -> Result<String, String> {
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
            let lang = if source.symbol_type == "DocChunk" {
                "markdown"
            } else {
                detect_language(&source.file_path)
            };
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

    #[test]
    fn classify_question_matches_requested_french_examples() {
        let cases = [
            ("où est défini FactureDelete ?", QuestionType::Lookup),
            (
                "Comment fonctionne le module Courrier ?",
                QuestionType::Functional,
            ),
            (
                "Comment sont générés les courriers en masse ?",
                QuestionType::Algorithm,
            ),
            (
                "Présente l'architecture globale d'Alise v2",
                QuestionType::Architecture,
            ),
            (
                "Quel est l'impact de modifier RootController ?",
                QuestionType::Impact,
            ),
            ("Explique le module Elodie", QuestionType::Functional),
        ];

        for (question, expected) in cases {
            assert_eq!(
                classify_question(question),
                expected,
                "unexpected classification for `{question}`"
            );
        }
    }

    #[test]
    fn detect_target_symbol_matches_requested_examples() {
        let cases = [
            ("Comment fonctionne le module Courrier ?", Some("Courrier")),
            ("Explique DossiersController", Some("DossiersController")),
            ("Comment sont calculés les plafonds ?", None),
            ("Présente le module Elodie", Some("Elodie")),
            ("Explique l'Elodie", Some("Elodie")),
        ];

        for (question, expected) in cases {
            assert_eq!(
                detect_target_symbol(question).as_deref(),
                expected,
                "unexpected target symbol for `{question}`"
            );
        }
    }

    #[test]
    fn classify_question_matches_requested_english_examples() {
        let cases = [
            ("where is the login method defined?", QuestionType::Lookup),
            ("How does the payment flow work?", QuestionType::Functional),
            ("how does the payment flow work?", QuestionType::Functional),
            ("how is the tax calculated?", QuestionType::Algorithm),
        ];

        for (question, expected) in cases {
            assert_eq!(
                classify_question(question),
                expected,
                "unexpected classification for `{question}`"
            );
        }
    }

    #[test]
    fn classify_question_impact_variants() {
        let cases = [
            (
                "Quelles sont les dépendances de FactureService ?",
                QuestionType::Impact,
            ),
            (
                "impact of changing the RootController",
                QuestionType::Impact,
            ),
            ("who calls GetTauxFassAide ?", QuestionType::Impact),
            ("blast radius of DossiersController", QuestionType::Impact),
            (
                "qu'est-ce qui casse si je modifie RegleFacture ?",
                QuestionType::Impact,
            ),
        ];
        for (q, expected) in cases {
            assert_eq!(classify_question(q), expected, "failed: `{q}`");
        }
    }

    #[test]
    fn classify_question_algorithm_variants() {
        let cases = [
            (
                "Comment est calculé le taux FASS ?",
                QuestionType::Algorithm,
            ),
            (
                "comment sont traités les dossiers ?",
                QuestionType::Algorithm,
            ),
            (
                "Décris l'algorithme de génération des courriers",
                QuestionType::Algorithm,
            ),
            (
                "step by step: how is the invoice built?",
                QuestionType::Algorithm,
            ),
            ("How is the plafond computed?", QuestionType::Algorithm),
        ];
        for (q, expected) in cases {
            assert_eq!(classify_question(q), expected, "failed: `{q}`");
        }
    }

    #[test]
    fn classify_question_architecture_variants() {
        let cases = [
            (
                "Vue d'ensemble de l'architecture Alise",
                QuestionType::Architecture,
            ),
            (
                "présentation générale du système",
                QuestionType::Architecture,
            ),
            (
                "Give me an overview of the codebase architecture",
                QuestionType::Architecture,
            ),
            ("Schéma global de l'application", QuestionType::Architecture),
        ];
        for (q, expected) in cases {
            assert_eq!(classify_question(q), expected, "failed: `{q}`");
        }
    }

    #[test]
    fn classify_question_lookup_variants() {
        let cases = [
            ("Trouve la classe ParametrageService", QuestionType::Lookup),
            ("What is StackLogger ?", QuestionType::Lookup),
            ("c'est quoi un DossierOuvrantDroit ?", QuestionType::Lookup),
            ("where is CourrierController defined?", QuestionType::Lookup),
        ];
        for (q, expected) in cases {
            assert_eq!(classify_question(q), expected, "failed: `{q}`");
        }
    }

    #[test]
    fn classify_question_canvas_instruction_not_empty() {
        for qt in [
            QuestionType::Lookup,
            QuestionType::Functional,
            QuestionType::Algorithm,
            QuestionType::Architecture,
            QuestionType::Impact,
        ] {
            let canvas = canvas_instruction(qt);
            assert!(
                !canvas.is_empty(),
                "canvas should not be empty for {:?}",
                qt
            );
            assert!(
                canvas.contains("CANEVAS"),
                "canvas should contain CANEVAS for {:?}",
                qt
            );
        }
    }
}
