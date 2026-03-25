//! HTTP transport for MCP via axum.
//!
//! Provides a single POST /mcp endpoint that bridges JSON-RPC requests
//! to the MCP backend.

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use tokio::sync::Mutex;

use crate::backend::local::LocalBackend;
use crate::jsonrpc::JsonRpcRequest;
use crate::server::handle_request;

/// Shared state for the HTTP handler.
pub type SharedBackend = Arc<Mutex<LocalBackend>>;

/// Create an axum Router with the MCP HTTP endpoint.
///
/// The router exposes:
/// - POST /mcp - JSON-RPC 2.0 endpoint for MCP tool invocations
pub fn mcp_http_router(backend: SharedBackend) -> Router {
    Router::new()
        .route("/mcp", post(mcp_handler))
        .with_state(backend)
}

/// Handle a POST /mcp request: parse JSON-RPC, dispatch, respond.
async fn mcp_handler(
    State(backend): State<SharedBackend>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let mut backend_guard = backend.lock().await;
    let response = handle_request(&request, &mut *backend_guard).await;
    (StatusCode::OK, Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let backend = Arc::new(Mutex::new(LocalBackend::new()));
        let _router = mcp_http_router(backend);
        // Router creation should not panic
    }
}
