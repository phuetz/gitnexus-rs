//! The `serve` command: starts an HTTP server for the web UI and MCP HTTP endpoint.
//!
//! `/api/chat` — Server-Sent Events (SSE) chat endpoint.
//!
//! Request body (JSON):
//! ```json
//! {
//!   "question": "Explain the DossiersController",
//!   "repo": "Alise_v2",
//!   "history": [
//!     { "role": "user",      "content": "Previous question" },
//!     { "role": "assistant", "content": "Previous answer"   }
//!   ]
//! }
//! ```
//!
//! Response: SSE stream of text deltas, terminated by `data: [DONE]`.

use std::net::IpAddr;
use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{header, HeaderName, Method, StatusCode, Uri},
    middleware::{self, Next},
    response::sse::{Event, KeepAlive},
    response::Response,
    response::Sse,
    response::{IntoResponse, Redirect},
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use std::convert::Infallible;
use tokio::sync::Mutex;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;

use gitnexus_mcp::backend::local::LocalBackend;
use gitnexus_mcp::transport::http::{mcp_http_router, SharedBackend};

const MAX_CHAT_QUESTION_CHARS: usize = 16_000;
const MAX_CHAT_HISTORY_MESSAGES: usize = 12;
const MAX_CHAT_HISTORY_CONTENT_CHARS: usize = 16_000;
const MAX_CHAT_HISTORY_TOTAL_CHARS: usize = 48_000;

#[derive(Deserialize)]
struct ChatRequest {
    question: String,
    #[serde(default)]
    repo: String,
    /// Optional conversation history for multi-turn context.
    /// Each entry: { "role": "user"|"assistant", "content": "..." }
    #[serde(default)]
    history: Vec<HistoryEntry>,
}

#[derive(Deserialize, Clone)]
struct HistoryEntry {
    role: String,
    content: String,
}

fn http_auth_token() -> Option<Arc<String>> {
    std::env::var("GITNEXUS_HTTP_TOKEN")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .map(Arc::new)
}

fn is_loopback_host(host: &str) -> bool {
    let normalized = host.trim().trim_start_matches('[').trim_end_matches(']');
    normalized.eq_ignore_ascii_case("localhost")
        || normalized
            .parse::<IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false)
}

fn is_loopback_origin(origin: &str) -> bool {
    let Ok(uri) = origin.parse::<Uri>() else {
        return false;
    };

    if !matches!(uri.scheme_str(), Some("http" | "https")) {
        return false;
    }
    if uri
        .path_and_query()
        .map(|path_and_query| path_and_query.as_str() != "/")
        .unwrap_or(false)
    {
        return false;
    }

    let Some(authority) = uri.authority() else {
        return false;
    };
    if authority.as_str().contains('@') {
        return false;
    }

    uri.host().map(is_loopback_host).unwrap_or(false)
}

fn validate_chat_payload(payload: &ChatRequest) -> Result<(), (StatusCode, String)> {
    let question_chars = payload.question.chars().count();
    if payload.question.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Question is required.".to_string()));
    }
    if question_chars > MAX_CHAT_QUESTION_CHARS {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            format!(
                "Question is too large ({} chars, max {}).",
                question_chars, MAX_CHAT_QUESTION_CHARS
            ),
        ));
    }
    if payload.history.len() > MAX_CHAT_HISTORY_MESSAGES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            format!(
                "Chat history is too large ({} messages, max {}).",
                payload.history.len(),
                MAX_CHAT_HISTORY_MESSAGES
            ),
        ));
    }

    let mut total_history_chars = 0usize;
    for entry in &payload.history {
        if entry.role != "user" && entry.role != "assistant" {
            return Err((
                StatusCode::BAD_REQUEST,
                "History role must be either 'user' or 'assistant'.".to_string(),
            ));
        }
        let entry_chars = entry.content.chars().count();
        if entry_chars > MAX_CHAT_HISTORY_CONTENT_CHARS {
            return Err((
                StatusCode::PAYLOAD_TOO_LARGE,
                format!(
                    "A chat history message is too large ({} chars, max {}).",
                    entry_chars, MAX_CHAT_HISTORY_CONTENT_CHARS
                ),
            ));
        }
        total_history_chars = total_history_chars.saturating_add(entry_chars);
    }

    if total_history_chars > MAX_CHAT_HISTORY_TOTAL_CHARS {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            format!(
                "Chat history is too large ({} chars, max {}).",
                total_history_chars, MAX_CHAT_HISTORY_TOTAL_CHARS
            ),
        ));
    }

    Ok(())
}

async fn auth_middleware(
    State(token): State<Arc<String>>,
    request: Request,
    next: Next,
) -> Response {
    let authorized = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(|value| value == token.as_str())
        .unwrap_or(false)
        || request
            .headers()
            .get("x-api-key")
            .and_then(|value| value.to_str().ok())
            .map(|value| value == token.as_str())
            .unwrap_or(false);

    if !authorized {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Missing or invalid HTTP auth token" })),
        )
            .into_response();
    }

    next.run(request).await
}

pub async fn run(port: u16, host: &str) -> anyhow::Result<()> {
    let auth_token = http_auth_token();
    if auth_token.is_none() {
        if is_loopback_host(host) {
            eprintln!(
                "Warning: GITNEXUS_HTTP_TOKEN is not set; HTTP APIs are only intended for this local machine."
            );
        } else {
            anyhow::bail!(
                "Refusing to start HTTP server on non-loopback host '{host}' without GITNEXUS_HTTP_TOKEN. Set GITNEXUS_HTTP_TOKEN or bind to 127.0.0.1."
            );
        }
    }

    let mut backend = LocalBackend::new();
    if let Err(e) = backend.init() {
        eprintln!("Warning: failed to initialize backend: {e}");
    }

    let shared: SharedBackend = Arc::new(Mutex::new(backend));

    // CORS -- allow browser access from bundled UI and loopback dev servers,
    // including custom ChatPort values chosen by the launcher.
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::AUTHORIZATION,
            HeaderName::from_static("x-api-key"),
        ])
        .allow_origin(AllowOrigin::predicate(|origin, _request_parts| {
            origin.to_str().map(is_loopback_origin).unwrap_or(false)
        }));

    let chat_routes = Router::new().route("/api/chat", post(chat_handler).get(chat_get_redirect));
    let chat_routes = if let Some(token) = auth_token {
        chat_routes.route_layer(middleware::from_fn_with_state(token, auth_middleware))
    } else {
        chat_routes
    };

    // Base router from MCP, plus the chat endpoint using the same optional
    // bearer token gate (`GITNEXUS_HTTP_TOKEN`) as the MCP HTTP routes.
    let app = mcp_http_router().merge(chat_routes).layer(cors);

    // Static file serving — two candidates, first match wins.
    //
    // 1. `<binary_dir>/web/` — used by the portable USB kit. The packaging
    //    script copies `chat-ui/dist/` here so visiting the server root
    //    in a browser loads the React UI directly, no `npm run dev` required.
    // 2. `<cwd>/.gitnexus/docs/` — legacy: the generated documentation HTML
    //    of whatever repo `gitnexus serve` was started in.
    let bin_web_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("web")))
        .filter(|d| d.exists());
    let cwd_docs_dir = std::env::current_dir()?.join(".gitnexus").join("docs");

    let app = if let Some(web_dir) = bin_web_dir {
        println!("Serving chat-ui from {}", web_dir.display());
        app.fallback_service(ServeDir::new(web_dir))
    } else if cwd_docs_dir.exists() {
        println!("Serving documentation from {}", cwd_docs_dir.display());
        app.fallback_service(ServeDir::new(cwd_docs_dir))
    } else {
        app
    };

    let app = app.with_state(shared);

    let addr = format!("{host}:{port}");
    println!("GitNexus HTTP server starting on http://{addr}");
    println!("  Documentation: http://{addr}/index.html");
    println!("  Chat API:      POST http://{addr}/api/chat  (SSE)");
    println!("  MCP endpoint:  POST http://{addr}/mcp");
    println!("  Press Ctrl+C to stop");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    println!("Server stopped.");
    Ok(())
}

async fn chat_handler(
    State(backend): State<SharedBackend>,
    Json(payload): Json<ChatRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    validate_chat_payload(&payload)?;

    let backend_guard = backend.lock().await;

    // Resolve repo path from name, fallback to first indexed repo.
    let registry = backend_guard.registry();
    let repo_entry = registry
        .iter()
        .find(|e| e.name == payload.repo)
        .or_else(|| registry.first())
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                "No repository found. Run 'gitnexus analyze' first.".to_string(),
            )
        })?;
    let repo_path = std::path::PathBuf::from(repo_entry.path.clone());
    drop(backend_guard);

    let question = payload.question;
    let history = payload.history;

    // Build prior turn messages for context window (last 6 turns = 12 messages)
    let history_context: String = history
        .iter()
        .rev()
        .take(6)
        .rev()
        .map(|h| format!("**{}**: {}", h.role, h.content))
        .collect::<Vec<_>>()
        .join("\n\n");
    let augmented_question = if history_context.is_empty() {
        question
    } else {
        format!(
            "{}\n\n---\n*Contexte de la conversation précédente :*\n{}",
            question, history_context
        )
    };

    // Channel feeds the SSE stream. The tool-loop runs in a tokio::spawn
    // (no spawn_blocking — ask_question_with_tools is fully async) and the
    // callback bridges StreamEvent → typed SSE Event.
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let tx_cb = tx.clone();
    let backend_for_loop = backend.clone();

    tokio::spawn(async move {
        let stream_cb = Box::new(move |ev: super::ask::StreamEvent| {
            let event = match ev {
                super::ask::StreamEvent::Delta(text) => Event::default().data(text),
                super::ask::StreamEvent::ToolCallStart { id, name, args } => {
                    Event::default().event("tool_call").data(
                        serde_json::json!({
                            "phase": "start",
                            "id": id,
                            "name": name,
                            "args": args,
                        })
                        .to_string(),
                    )
                }
                super::ask::StreamEvent::ToolCallEnd { id, name, success } => {
                    Event::default().event("tool_call").data(
                        serde_json::json!({
                            "phase": "end",
                            "id": id,
                            "name": name,
                            "success": success,
                        })
                        .to_string(),
                    )
                }
            };
            let _ = tx_cb.send(Ok::<Event, Infallible>(event));
        });

        let result = super::ask::ask_question_with_tools(
            &augmented_question,
            &repo_path,
            backend_for_loop,
            Some(stream_cb),
        )
        .await;

        match result {
            Ok(_) => {
                let _ = tx.send(Ok(Event::default().data("[DONE]")));
            }
            Err(e) => {
                let _ = tx.send(Ok(Event::default()
                    .event("error")
                    .data(format!("Error: {}", e))));
                let _ = tx.send(Ok(Event::default().data("[DONE]")));
            }
        }
    });

    let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

async fn chat_get_redirect() -> Redirect {
    Redirect::temporary("/")
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install ctrl+c handler");
    println!("\nShutting down...");
}

#[cfg(test)]
mod tests {
    use super::{is_loopback_host, is_loopback_origin};

    #[test]
    fn loopback_host_detection_accepts_local_hosts() {
        assert!(is_loopback_host("127.0.0.1"));
        assert!(is_loopback_host("localhost"));
        assert!(is_loopback_host("::1"));
        assert!(is_loopback_host("[::1]"));
    }

    #[test]
    fn loopback_host_detection_rejects_network_binds() {
        assert!(!is_loopback_host("0.0.0.0"));
        assert!(!is_loopback_host("::"));
        assert!(!is_loopback_host("192.168.1.10"));
    }

    #[test]
    fn loopback_origin_detection_accepts_any_local_port() {
        assert!(is_loopback_origin("http://localhost:5175"));
        assert!(is_loopback_origin("http://127.0.0.1:5177"));
        assert!(is_loopback_origin("https://[::1]:1420"));
    }

    #[test]
    fn loopback_origin_detection_rejects_remote_and_non_http_origins() {
        assert!(!is_loopback_origin("http://192.168.1.10:5175"));
        assert!(!is_loopback_origin("https://example.com"));
        assert!(!is_loopback_origin("file://localhost/tmp"));
        assert!(!is_loopback_origin("null"));
        assert!(!is_loopback_origin("http://localhost:5175/sneaky"));
        assert!(!is_loopback_origin("http://user@localhost:5175"));
    }
}
