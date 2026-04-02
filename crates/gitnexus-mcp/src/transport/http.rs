//! HTTP transport for MCP via axum.
//!
//! Provides JSON-RPC endpoint (POST /mcp) and REST API endpoints for
//! direct HTTP integration without MCP protocol overhead.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::backend::local::LocalBackend;
use crate::jsonrpc::JsonRpcRequest;
use crate::server::handle_request;

/// Shared state for the HTTP handler.
pub type SharedBackend = Arc<Mutex<LocalBackend>>;

/// Create an axum Router with MCP and REST API endpoints.
///
/// Endpoints:
/// - POST /mcp           — JSON-RPC 2.0 endpoint for MCP tool invocations
/// - GET  /health        — Liveness check
/// - GET  /api/repos     — List indexed repositories
/// - GET  /api/repos/:name/search?q=...&limit=N — Search symbols
/// - GET  /api/repos/:name/stats — Repository statistics
/// - GET  /api/repos/:name/hotspots?days=N — File hotspots
pub fn mcp_http_router(backend: SharedBackend) -> Router {
    Router::new()
        .route("/mcp", post(mcp_handler))
        .route("/health", get(health_handler))
        .route("/api/repos", get(repos_handler))
        .route("/api/repos/{name}/search", get(search_handler))
        .route("/api/repos/{name}/stats", get(stats_handler))
        .route("/api/repos/{name}/hotspots", get(hotspots_handler))
        .with_state(backend)
}

/// Handle a POST /mcp request: parse JSON-RPC, dispatch, respond.
async fn mcp_handler(
    State(backend): State<SharedBackend>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let mut backend_guard = backend.lock().await;
    let response = handle_request(&request, &mut backend_guard).await;
    (StatusCode::OK, Json(response))
}

/// GET /health — Liveness check
async fn health_handler() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "gitnexus",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// GET /api/repos — List indexed repositories
async fn repos_handler(
    State(backend): State<SharedBackend>,
) -> impl IntoResponse {
    let backend_guard = backend.lock().await;
    let repos: Vec<Value> = backend_guard
        .registry()
        .iter()
        .map(|e| {
            json!({
                "name": e.name,
                "path": e.path,
                "indexedAt": e.indexed_at,
                "lastCommit": e.last_commit,
                "stats": e.stats,
            })
        })
        .collect();
    Json(json!({ "repos": repos }))
}

#[derive(Deserialize)]
struct SearchParams {
    q: String,
    limit: Option<usize>,
}

/// GET /api/repos/:name/search?q=...&limit=N — Search symbols
async fn search_handler(
    State(backend): State<SharedBackend>,
    Path(name): Path<String>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut backend_guard = backend.lock().await;
    let args = json!({
        "query": params.q,
        "repo": name,
        "limit": params.limit.unwrap_or(20),
    });
    match backend_guard.call_tool("query", &args).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )),
    }
}

/// GET /api/repos/:name/stats — Repository statistics
async fn stats_handler(
    State(backend): State<SharedBackend>,
    Path(name): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let backend_guard = backend.lock().await;
    let resource = crate::resources::read_resource(
        &format!("gitnexus://repos/{}/stats", name),
        backend_guard.registry(),
    );
    match resource {
        Some(r) => Ok(Json(r)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Repository '{}' not found", name) })),
        )),
    }
}

#[derive(Deserialize)]
struct HotspotsParams {
    days: Option<u32>,
    limit: Option<usize>,
}

/// GET /api/repos/:name/hotspots?days=90&limit=20 — File hotspots
async fn hotspots_handler(
    State(backend): State<SharedBackend>,
    Path(name): Path<String>,
    Query(params): Query<HotspotsParams>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut backend_guard = backend.lock().await;
    let args = json!({
        "repo": name,
        "since_days": params.days.unwrap_or(90),
        "limit": params.limit.unwrap_or(20),
    });
    match backend_guard.call_tool("hotspots", &args).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )),
    }
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
