//! HTTP transport for MCP via axum.
//!
//! Provides JSON-RPC endpoint (POST /mcp) and REST API endpoints for
//! direct HTTP integration without MCP protocol overhead.

use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Path, Query, Request, State},
    http::{header, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

use crate::backend::local::LocalBackend;
use crate::jsonrpc::{JsonRpcRequest, JsonRpcResponse};
use crate::server::handle_request;

/// Shared state for the HTTP handler.
pub type SharedBackend = Arc<Mutex<LocalBackend>>;

fn http_auth_token() -> Option<Arc<String>> {
    std::env::var("GITNEXUS_HTTP_TOKEN")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .map(Arc::new)
}

fn expose_repo_paths() -> bool {
    matches!(
        std::env::var("GITNEXUS_EXPOSE_REPO_PATHS")
            .ok()
            .as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "YES")
    )
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

/// Create an axum Router with MCP and REST API endpoints.
pub fn mcp_http_router() -> Router<SharedBackend> {
    let protected = Router::new()
        .route("/mcp", post(mcp_handler))
        .route("/api/repos", get(repos_handler))
        .route("/api/repos/{name}/search", get(search_handler))
        .route("/api/repos/{name}/stats", get(stats_handler))
        .route("/api/repos/{name}/hotspots", get(hotspots_handler));

    let protected = if let Some(token) = http_auth_token() {
        protected.route_layer(middleware::from_fn_with_state(token, auth_middleware))
    } else {
        protected
    };

    Router::new()
        .route("/health", get(health_handler))
        .merge(protected)
        .layer(
            CorsLayer::new()
                .allow_origin([
                    "http://localhost".parse::<HeaderValue>().unwrap(),
                    "http://localhost:3000".parse::<HeaderValue>().unwrap(),
                    "http://127.0.0.1".parse::<HeaderValue>().unwrap(),
                    "http://127.0.0.1:3000".parse::<HeaderValue>().unwrap(),
                    "http://localhost:1420".parse::<HeaderValue>().unwrap(),
                ])
                .allow_methods([Method::GET, Method::POST])
                .allow_headers(tower_http::cors::Any),
        )
}

/// Handle a POST /mcp request: parse JSON-RPC, dispatch, respond.
///
/// Parses the body as raw bytes (instead of `Json<JsonRpcRequest>`) so that
/// malformed JSON or schema mismatches return a JSON-RPC 2.0 -32700 (parse
/// error) / -32600 (invalid request) response with HTTP 200, not axum's
/// default 422 plain-text response. MCP clients only know how to handle
/// JSON-RPC envelopes — a 422 leaves them in an undefined state.
async fn mcp_handler(
    State(backend): State<SharedBackend>,
    body: Bytes,
) -> impl IntoResponse {
    // Try to extract the request id up front so any error response can echo it
    // back per JSON-RPC spec; fall back to null if parsing fails entirely.
    let raw_json: Result<Value, _> = serde_json::from_slice(&body);
    let id = raw_json
        .as_ref()
        .ok()
        .and_then(|v| v.get("id").cloned())
        .unwrap_or(Value::Null);

    let request: JsonRpcRequest = match raw_json {
        Ok(v) => match serde_json::from_value::<JsonRpcRequest>(v) {
            Ok(r) => r,
            Err(e) => {
                return (
                    StatusCode::OK,
                    Json(JsonRpcResponse::error(
                        id,
                        -32600, // Invalid Request
                        format!("Invalid JSON-RPC request: {e}"),
                        None,
                    )),
                );
            }
        },
        Err(e) => {
            return (
                StatusCode::OK,
                Json(JsonRpcResponse::error(
                    id,
                    -32700, // Parse error
                    format!("Parse error: {e}"),
                    None,
                )),
            );
        }
    };

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
    let expose_paths = expose_repo_paths();
    let repos: Vec<Value> = backend_guard
        .registry()
        .iter()
        .map(|e| {
            let mut obj = json!({
                "name": e.name,
                "indexedAt": e.indexed_at,
                "lastCommit": e.last_commit,
                "stats": e.stats,
                "pathExposed": expose_paths,
            });
            if expose_paths {
                obj["path"] = json!(e.path);
            }
            obj
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
    if params.q.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Query parameter 'q' is required" })),
        ));
    }
    if params.q.len() > 1000 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Query too long (max 1000 chars)" })),
        ));
    }

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

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_router_creation() {
        let backend = Arc::new(Mutex::new(LocalBackend::new()));
        let _router = mcp_http_router(backend);
        // Router creation should not panic
    }

    #[test]
    fn test_http_auth_token_is_trimmed_and_optional() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("GITNEXUS_HTTP_TOKEN");
        assert!(http_auth_token().is_none());

        std::env::set_var("GITNEXUS_HTTP_TOKEN", "  secret-token  ");
        assert_eq!(
            http_auth_token().as_deref().map(|s| s.as_str()),
            Some("secret-token")
        );
        std::env::remove_var("GITNEXUS_HTTP_TOKEN");
    }

    #[test]
    fn test_expose_repo_paths_defaults_to_false() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("GITNEXUS_EXPOSE_REPO_PATHS");
        assert!(!expose_repo_paths());

        std::env::set_var("GITNEXUS_EXPOSE_REPO_PATHS", "true");
        assert!(expose_repo_paths());
        std::env::remove_var("GITNEXUS_EXPOSE_REPO_PATHS");
    }
}
