//! The `serve` command: starts an HTTP server for the web UI and MCP HTTP endpoint.

use std::sync::Arc;

use tokio::sync::Mutex;

use gitnexus_mcp::backend::local::LocalBackend;
use gitnexus_mcp::transport::http::{mcp_http_router, SharedBackend};

pub async fn run(port: u16, host: &str) -> anyhow::Result<()> {
    let mut backend = LocalBackend::new();
    if let Err(e) = backend.init() {
        eprintln!("Warning: failed to initialize backend: {e}");
    }

    let shared: SharedBackend = Arc::new(Mutex::new(backend));
    let app = mcp_http_router(shared);

    let addr = format!("{host}:{port}");
    println!("GitNexus HTTP server starting on http://{addr}");
    println!("  MCP endpoint: POST http://{addr}/mcp");
    println!("  Press Ctrl+C to stop");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    println!("Server stopped.");
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install ctrl+c handler");
    println!("\nShutting down...");
}
