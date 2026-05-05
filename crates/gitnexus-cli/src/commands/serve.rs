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

use std::sync::Arc;

use axum::{
    extract::State,
    http::{header, Method, StatusCode},
    response::sse::{Event, KeepAlive},
    response::IntoResponse,
    response::Sse,
    routing::post,
    Json,
};
use serde::Deserialize;
use std::convert::Infallible;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use gitnexus_mcp::backend::local::LocalBackend;
use gitnexus_mcp::transport::http::{mcp_http_router, SharedBackend};

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

pub async fn run(port: u16, host: &str) -> anyhow::Result<()> {
    let mut backend = LocalBackend::new();
    if let Err(e) = backend.init() {
        eprintln!("Warning: failed to initialize backend: {e}");
    }

    let shared: SharedBackend = Arc::new(Mutex::new(backend));

    // CORS — allow browser access from documentation HTML served on same host
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT])
        .allow_origin(tower_http::cors::Any);

    // Base router from MCP
    let app = mcp_http_router()
        .route("/api/chat", post(chat_handler))
        .layer(cors);

    // Static file serving for documentation
    let docs_dir = std::env::current_dir()?.join(".gitnexus").join("docs");
    let app = if docs_dir.exists() {
        println!("Serving documentation from {}", docs_dir.display());
        app.fallback_service(ServeDir::new(docs_dir))
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
    let backend_guard = backend.lock().await;

    // Resolve repo path from name, fallback to first indexed repo
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

    let repo_path = repo_entry.path.clone();
    drop(backend_guard);

    let question = payload.question;
    let history = payload.history;

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    tokio::task::spawn_blocking(move || {
        let tx_chunk = tx.clone();
        let tx_done = tx.clone();

        // Build prior turn messages for context window (last 6 turns = 12 messages)
        let history_context: String = history
            .iter()
            .rev()
            .take(6)
            .rev()
            .map(|h| format!("**{}**: {}", h.role, h.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        // Augment the question with history context if available
        let augmented_question = if history_context.is_empty() {
            question.clone()
        } else {
            format!(
                "{}\n\n---\n*Contexte de la conversation précédente :*\n{}",
                question, history_context
            )
        };

        let stream_cb = Box::new(move |delta: &str| {
            let _ = tx_chunk.send(Ok::<Event, Infallible>(Event::default().data(delta)));
        });

        let result =
            super::ask::ask_question(&augmented_question, Some(&repo_path), Some(stream_cb));

        // Send [DONE] sentinel so clients know the stream has ended
        match result {
            Ok(_) => {
                let _ = tx_done.send(Ok(Event::default().data("[DONE]")));
            }
            Err(e) => {
                let _ = tx_done.send(Ok(Event::default()
                    .event("error")
                    .data(format!("Error: {}", e))));
                let _ = tx_done.send(Ok(Event::default().data("[DONE]")));
            }
        }
    });

    let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install ctrl+c handler");
    println!("\nShutting down...");
}
