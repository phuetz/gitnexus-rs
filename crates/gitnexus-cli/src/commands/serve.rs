//! The `serve` command: starts an HTTP server for the web UI and MCP HTTP endpoint.

use std::sync::Arc;

use axum::{
    routing::post,
    Json,
    extract::State,
    http::StatusCode,
    response::Sse,
    response::sse::{Event, KeepAlive},
    response::IntoResponse,
};
use tower_http::services::ServeDir;
use tokio::sync::Mutex;
use serde::Deserialize;
use std::convert::Infallible;

use gitnexus_mcp::backend::local::LocalBackend;
use gitnexus_mcp::transport::http::{mcp_http_router, SharedBackend};

#[derive(Deserialize)]
struct ChatRequest {
    question: String,
    repo: String,
    #[allow(dead_code)]
    history: Vec<serde_json::Value>,
}

pub async fn run(port: u16, host: &str) -> anyhow::Result<()> {
    let mut backend = LocalBackend::new();
    if let Err(e) = backend.init() {
        eprintln!("Warning: failed to initialize backend: {e}");
    }

    let shared: SharedBackend = Arc::new(Mutex::new(backend));
    
    // Base router from MCP
    let app = mcp_http_router()
        .route("/api/chat", post(chat_handler));

    // Add Static File serving for documentation
    let docs_dir = std::env::current_dir()?.join(".gitnexus").join("docs");
    let app = if docs_dir.exists() {
        println!("Serving documentation from {}", docs_dir.display());
        app.fallback_service(ServeDir::new(docs_dir))
    } else {
        app
    };

    // Apply shared state LAST to get Router<()>
    let app = app.with_state(shared);

    let addr = format!("{host}:{port}");
    println!("GitNexus HTTP server starting on http://{addr}");
    println!("  Documentation: http://{addr}/index.html");
    println!("  MCP endpoint: POST http://{addr}/mcp");
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
    
    // Resolve repo path
    let registry = backend_guard.registry();
    let repo_entry = registry.iter().find(|e| e.name == payload.repo)
        .or_else(|| registry.first()) // Fallback to first repo if name doesn't match
        .ok_or_else(|| (StatusCode::NOT_FOUND, "No repository found".to_string()))?;
    
    let repo_path = repo_entry.path.clone();
    drop(backend_guard); // Release lock before calling LLM

    let question = payload.question;
    
    // Create a channel for streaming
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    // Spawn blocking task for the 'ask' logic
    tokio::task::spawn_blocking(move || {
        let tx_err = tx.clone();
        let stream_cb = Box::new(move |delta: &str| {
            let _ = tx.send(Ok::<Event, Infallible>(Event::default().data(delta)));
        });

        if let Err(e) = super::ask::ask_question(&question, Some(&repo_path), Some(stream_cb)) {
            let _ = tx_err.send(Ok(Event::default().data(format!("Error: {}", e))));
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
