//! The `ask` command: ask questions about the codebase using graph + LLM.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use colored::Colorize;
use serde_json::{json, Value};
use tokio::sync::Mutex as TokioMutex;

use gitnexus_db::snapshot;
use gitnexus_mcp::backend::local::LocalBackend;

pub fn run(question: &str, path: Option<&str>) -> Result<()> {
    let (answer, top_nodes) = ask_question(
        question,
        path,
        Some(Box::new(|delta| {
            print!("{}", delta);
            use std::io::Write;
            std::io::stdout().flush().unwrap();
        })),
    )?;

    if answer.is_empty() && top_nodes.is_empty() {
        return Ok(());
    }

    println!("\n\n{}", "\u{2500}".repeat(60));

    // Show sources
    println!("\n{}", "Sources:".dimmed());
    for (node, _) in top_nodes.iter().take(5) {
        println!(
            "  {} `{}` in {}",
            "->".dimmed(),
            node.properties.name,
            node.properties.file_path
        );
    }

    Ok(())
}

pub type StreamCallback = Box<dyn Fn(&str) + Send>;

pub fn ask_question(
    question: &str,
    path: Option<&str>,
    stream_cb: Option<StreamCallback>,
) -> Result<(String, Vec<(gitnexus_core::graph::types::GraphNode, f64)>)> {
    let repo_path = if let Some(p) = path {
        std::path::PathBuf::from(p)
    } else {
        std::env::current_dir()?
    };

    // Load config
    let config = super::generate::load_llm_config();
    let config = match config {
        Some(c) => c,
        None => {
            return Err(anyhow::anyhow!(
                "No LLM configured. Create ~/.gitnexus/chat-config.json"
            ));
        }
    };

    // Load graph
    let storage_path = repo_path.join(".gitnexus");
    let snap_path = storage_path.join("graph.bin");
    if !snap_path.exists() {
        return Err(anyhow::anyhow!(
            "No index found. Run 'gitnexus analyze' first."
        ));
    }

    let graph = snapshot::load_snapshot(&snap_path)
        .map_err(|e| anyhow::anyhow!("Failed to load graph: {}", e))?;

    // Search the graph for relevant symbols
    let query_lower = question.to_lowercase();
    let mut relevant_nodes: Vec<(&gitnexus_core::graph::types::GraphNode, f64)> = Vec::new();

    for node in graph.iter_nodes() {
        let name_lower = node.properties.name.to_lowercase();
        let file_lower = node.properties.file_path.to_lowercase();

        let mut score = 0.0;
        for word in query_lower.split_whitespace() {
            if name_lower.contains(word) {
                score += 2.0;
            }
            if file_lower.contains(word) {
                score += 0.5;
            }
            if let Some(desc) = &node.properties.description {
                if desc.to_lowercase().contains(word) {
                    score += 1.0;
                }
            }
            if let Some(content) = &node.properties.content {
                if content.to_lowercase().contains(word) {
                    score += 1.0;
                }
            }
        }
        if score > 0.0 {
            relevant_nodes.push((node, score));
        }
    }

    relevant_nodes.sort_by(|a, b| b.1.total_cmp(&a.1));
    let top_nodes = &relevant_nodes[..relevant_nodes.len().min(10)];

    if top_nodes.is_empty() {
        return Ok((String::new(), Vec::new()));
    }

    // Build context from top nodes
    let mut context = String::new();
    for (node, _score) in top_nodes {
        context.push_str(&format!(
            "**{}** ({}) in `{}`\n",
            node.properties.name,
            node.label.as_str(),
            node.properties.file_path
        ));

        if let Some(content) = &node.properties.content {
            context.push_str("```markdown\n");
            context.push_str(content);
            context.push_str("\n```\n\n");
            continue;
        }

        let source_path = repo_path.join(&node.properties.file_path);
        if let Ok(source) = std::fs::read_to_string(&source_path) {
            let lines: Vec<&str> = source.lines().collect();
            let start = node
                .properties
                .start_line
                .map(|l| l as usize)
                .unwrap_or(1)
                .saturating_sub(1)
                .min(lines.len());
            let end = (start + 15).min(lines.len());
            context.push_str("```\n");
            for line in &lines[start..end] {
                context.push_str(line);
                context.push('\n');
            }
            context.push_str("```\n\n");
        }
    }

    // Call LLM
    //
    // System prompt orientation: clients pay for clarity, not for prose. The
    // LLM is told to lean on Mermaid, tables, and code blocks whenever they
    // beat plain text — Gemini 2.5 Flash already produces good Mermaid when
    // explicitly invited, and react-markdown + a Mermaid renderer in the UI
    // turns those fences into SVG diagrams the user can show a stakeholder.
    let messages = vec![
        serde_json::json!({
            "role": "system",
            "content": "Tu es un expert en analyse de code travaillant pour un cabinet de conseil. \
Tes réponses sont destinées à des clients professionnels — elles doivent être structurées, \
précises, et impressionner par leur clarté.\n\
\n\
Règles :\n\
- Base-toi UNIQUEMENT sur le contexte fourni. Ne fais pas de suppositions.\n\
- Format de réponse : Markdown structuré (titres ##, listes, gras pour les noms de classes/méthodes).\n\
- Si la question implique un flux d'exécution, une architecture, des dépendances ou une \
hiérarchie : illustre avec un diagramme Mermaid. Préfère `flowchart TD` pour les flux, \
`sequenceDiagram` pour les interactions entre composants, `classDiagram` pour les héritages, \
`erDiagram` pour le schéma de données. Le diagramme va dans un bloc ```mermaid ... ```.\n\
- Pour le code cité : bloc ```<lang>``` avec la bonne langue (csharp, typescript, rust, …) — \
pas seulement ``` nu.\n\
- Pour les comparaisons ou inventaires (endpoints, tables, propriétés) : utilise un tableau Markdown.\n\
- Cite les chemins de fichiers en `code inline`. Liste les sources à la fin sous une rubrique \
**Sources** (un fichier par puce).\n\
- Reste concise : un client paye pour la pertinence, pas pour le volume."
        }),
        serde_json::json!({
            "role": "user",
            "content": format!("Question : {}\n\nContexte :\n{}", question, context)
        }),
    ];

    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
    let mut body = serde_json::json!({
        "model": config.model,
        "messages": messages,
        "max_tokens": config.max_tokens,
        "temperature": 0.3,
        "stream": stream_cb.is_some()
    });

    let effort = config.reasoning_effort.trim().to_lowercase();
    if !effort.is_empty() && effort != "none" {
        body["reasoning_effort"] = serde_json::Value::String(effort);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let mut request = client.post(&url).json(&body);
    if !config.api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", config.api_key));
    }

    let response = request.send()?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("LLM error: {}", response.status()));
    }

    use std::io::{BufRead, BufReader};

    let mut full_answer = String::new();
    let reader = BufReader::new(response);
    for line in reader.lines() {
        let line = line?;
        if let Some(data) = line.strip_prefix("data: ") {
            if data.trim() == "[DONE]" {
                break;
            }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(delta) = json
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("delta"))
                    .and_then(|d| d.get("content"))
                    .and_then(|v| v.as_str())
                {
                    if let Some(cb) = &stream_cb {
                        cb(delta);
                    }
                    full_answer.push_str(delta);
                }
            }
        } else if stream_cb.is_none() {
            // Non-streaming response body parsing if stream is false
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(content) = json
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|v| v.as_str())
                {
                    full_answer.push_str(content);
                }
            }
        }
    }

    let top_nodes_vec = top_nodes.iter().map(|(n, s)| ((*n).clone(), *s)).collect();
    Ok((full_answer, top_nodes_vec))
}

// ─── Wave 2: tool-calling enabled ask_question (async) ─────────────────────
//
// The legacy `ask_question` above pre-fetches BM25+semantic context and
// hands it to the LLM in a single shot — the model never gets to ask for
// more. `ask_question_with_tools` keeps that initial context as a free
// "head-start" but additionally exposes the full 30-tool MCP catalogue so
// the LLM can run `diagram`, `hotspots`, `find_cycles`, etc. when the
// question demands deeper traversal. Both UIs (chat-ui via /api/chat SSE
// and the desktop Tauri chat) benefit through the shared backend, per the
// "core partagé, UIs spécialisées" pattern.

/// Stream events surfaced by [`ask_question_with_tools`]. The chat-ui
/// converts these to typed SSE events (see `serve.rs::chat_handler`) so
/// the React layer can render "🔍 Exécute search_code…" badges inline
/// while the LLM is still thinking.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Partial text from the LLM final answer.
    Delta(String),
    /// A tool call is about to be dispatched to `LocalBackend`.
    ToolCallStart {
        id: String,
        name: String,
        args: String,
    },
    /// The tool returned (or failed). UI flips the badge to ✓ or ✗.
    ToolCallEnd {
        id: String,
        name: String,
        success: bool,
    },
}

pub type ToolStreamCallback = Box<dyn Fn(StreamEvent) + Send + Sync>;

/// Maximum LLM ↔ tool-loop round-trips before we force a final answer.
/// Patrice's empirical observation on the desktop chat: lookup-style
/// questions converge in 1-2; architectural traces sometimes need 5-6;
/// 8 leaves headroom without letting a runaway loop burn the token budget.
const MAX_TOOL_ITERATIONS: usize = 8;

/// LLM-driven tool loop. Pre-fetches context like the legacy `ask_question`
/// (so the model has a head start instead of starting blind), then spins
/// the OpenAI tool-calling loop, dispatching each requested tool through
/// the shared `LocalBackend`.
pub async fn ask_question_with_tools(
    question: &str,
    repo_path: &Path,
    backend: Arc<TokioMutex<LocalBackend>>,
    stream_cb: Option<ToolStreamCallback>,
) -> Result<(String, Vec<(gitnexus_core::graph::types::GraphNode, f64)>)> {
    // ── Phase 1: bootstrap context (same logic as legacy ask_question) ───
    let config = super::generate::load_llm_config()
        .ok_or_else(|| anyhow::anyhow!("No LLM configured. Create ~/.gitnexus/chat-config.json"))?;

    let storage_path = repo_path.join(".gitnexus");
    let snap_path = storage_path.join("graph.bin");
    if !snap_path.exists() {
        return Err(anyhow::anyhow!(
            "No index found. Run 'gitnexus analyze' first."
        ));
    }
    let graph = snapshot::load_snapshot(&snap_path)
        .map_err(|e| anyhow::anyhow!("Failed to load graph: {}", e))?;

    let query_lower = question.to_lowercase();
    let mut relevant_nodes: Vec<(&gitnexus_core::graph::types::GraphNode, f64)> = Vec::new();
    for node in graph.iter_nodes() {
        let name_lower = node.properties.name.to_lowercase();
        let file_lower = node.properties.file_path.to_lowercase();
        let mut score = 0.0;
        for word in query_lower.split_whitespace() {
            if name_lower.contains(word) {
                score += 2.0;
            }
            if file_lower.contains(word) {
                score += 0.5;
            }
            if let Some(desc) = &node.properties.description {
                if desc.to_lowercase().contains(word) {
                    score += 1.0;
                }
            }
            if let Some(content) = &node.properties.content {
                if content.to_lowercase().contains(word) {
                    score += 1.0;
                }
            }
        }
        if score > 0.0 {
            relevant_nodes.push((node, score));
        }
    }
    relevant_nodes.sort_by(|a, b| b.1.total_cmp(&a.1));
    let top_slice = &relevant_nodes[..relevant_nodes.len().min(10)];

    let mut context = String::new();
    for (node, _) in top_slice {
        context.push_str(&format!(
            "**{}** ({}) in `{}`\n",
            node.properties.name,
            node.label.as_str(),
            node.properties.file_path
        ));
        if let Some(content) = &node.properties.content {
            context.push_str("```markdown\n");
            context.push_str(content);
            context.push_str("\n```\n\n");
        } else if let Ok(source) = std::fs::read_to_string(repo_path.join(&node.properties.file_path)) {
            let lines: Vec<&str> = source.lines().collect();
            let start = node
                .properties
                .start_line
                .map(|l| l as usize)
                .unwrap_or(1)
                .saturating_sub(1)
                .min(lines.len());
            let end = (start + 15).min(lines.len());
            context.push_str("```\n");
            for line in &lines[start..end] {
                context.push_str(line);
                context.push('\n');
            }
            context.push_str("```\n\n");
        }
    }
    let top_nodes_vec: Vec<(gitnexus_core::graph::types::GraphNode, f64)> = top_slice
        .iter()
        .map(|(n, s)| ((*n).clone(), *s))
        .collect();

    // ── Phase 2: build messages + tools catalogue ──────────────────────────
    let system_prompt = "Tu es un expert en analyse de code travaillant pour un cabinet de conseil. \
Tes réponses sont destinées à des clients professionnels — elles doivent être structurées, \
précises, et impressionner par leur clarté.\n\
\n\
Règles :\n\
- Tu disposes d'outils MCP (search_code, query, context, impact, hotspots, coupling, ownership, \
diagram, find_cycles, list_endpoints, list_db_tables, …). Utilise-les pour creuser quand le \
contexte initial ne suffit pas — ne devine pas.\n\
- Format de réponse : Markdown structuré (titres ##, listes, gras pour les noms de classes/méthodes).\n\
- Si la question implique un flux d'exécution, une architecture, des dépendances ou une \
hiérarchie : illustre avec un diagramme Mermaid (`flowchart TD`, `sequenceDiagram`, `classDiagram`, \
`erDiagram`). Le diagramme va dans un bloc ```mermaid ... ```.\n\
- Pour le code cité : bloc ```<lang>``` avec la bonne langue (csharp, typescript, rust, …).\n\
- Pour les comparaisons ou inventaires : utilise un tableau Markdown.\n\
- Cite les chemins de fichiers en `code inline`. Liste les sources à la fin sous une rubrique \
**Sources**.\n\
- Reste concise : un client paye pour la pertinence, pas pour le volume.";

    let mut messages: Vec<Value> = vec![
        json!({"role": "system", "content": system_prompt}),
        json!({
            "role": "user",
            "content": format!("Question : {}\n\nContexte initial (top-10 symboles pertinents) :\n{}", question, context),
        }),
    ];

    let tools: Vec<Value> = gitnexus_mcp::tools::definitions::tool_definitions()
        .into_iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema,
                }
            })
        })
        .collect();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()?;
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

    let mut full_answer = String::new();
    let repo_label = repo_path.display().to_string();

    // ── Phase 3: tool loop ────────────────────────────────────────────────
    for _iter in 0..MAX_TOOL_ITERATIONS {
        let mut body = json!({
            "model": config.model,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto",
            "max_tokens": config.max_tokens,
            "temperature": 0.3,
            "stream": false,
        });
        let effort = config.reasoning_effort.trim().to_lowercase();
        if !effort.is_empty() && effort != "none" {
            body["reasoning_effort"] = Value::String(effort);
        }

        let mut request = client.post(&url).json(&body);
        if !config.api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", config.api_key));
        }
        let response = request.send().await?;
        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("LLM error: {} {}", status, body_text));
        }
        let resp: Value = response.json().await?;

        let message = &resp["choices"][0]["message"];
        let content = message["content"].as_str().unwrap_or("");
        let tool_calls = message["tool_calls"].as_array().cloned().unwrap_or_default();
        let finish_reason = resp["choices"][0]["finish_reason"]
            .as_str()
            .unwrap_or("stop")
            .to_string();

        if !content.is_empty() {
            full_answer.push_str(content);
            if let Some(cb) = stream_cb.as_ref() {
                cb(StreamEvent::Delta(content.to_string()));
            }
        }

        // Append the assistant turn (with tool_calls if any) to history.
        let mut assistant_msg = json!({"role": "assistant"});
        if !content.is_empty() {
            assistant_msg["content"] = json!(content);
        }
        if !tool_calls.is_empty() {
            assistant_msg["tool_calls"] = json!(tool_calls);
        }
        messages.push(assistant_msg);

        // Done when the model emitted a final answer with no tool requests.
        if tool_calls.is_empty() || finish_reason == "stop" {
            break;
        }

        // Dispatch each tool call through the shared backend.
        for tc in &tool_calls {
            let id = tc["id"].as_str().unwrap_or("").to_string();
            let name = tc["function"]["name"].as_str().unwrap_or("").to_string();
            let args_str = tc["function"]["arguments"]
                .as_str()
                .unwrap_or("{}")
                .to_string();
            let mut args: Value = serde_json::from_str(&args_str).unwrap_or_else(|_| json!({}));
            // Auto-inject the active repo when the model omits it — the chat
            // already knows which repo is active, no point making the LLM
            // restate it on every call.
            if args.is_object() && args.get("repo").is_none() {
                args["repo"] = json!(repo_label);
            }

            if let Some(cb) = stream_cb.as_ref() {
                cb(StreamEvent::ToolCallStart {
                    id: id.clone(),
                    name: name.clone(),
                    args: args.to_string(),
                });
            }

            let result = {
                let mut backend_guard = backend.lock().await;
                backend_guard.call_tool(&name, &args).await
            };
            let (success, result_str) = match result {
                Ok(v) => (true, v.to_string()),
                Err(e) => (false, format!("{{\"error\":\"{}\"}}", e)),
            };

            if let Some(cb) = stream_cb.as_ref() {
                cb(StreamEvent::ToolCallEnd {
                    id: id.clone(),
                    name: name.clone(),
                    success,
                });
            }

            messages.push(json!({
                "role": "tool",
                "tool_call_id": id,
                "name": name,
                "content": result_str,
            }));
        }
    }

    Ok((full_answer, top_nodes_vec))
}
