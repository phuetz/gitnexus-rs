//! HTTP transport for MCP via axum.
//!
//! Provides JSON-RPC endpoint (POST /mcp) and REST API endpoints for
//! direct HTTP integration without MCP protocol overhead.

use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

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

use gitnexus_core::secret_store::{decode_secret_from_storage, secret_payload_needs_migration};
use gitnexus_core::storage::repo_manager::registry_entry_id;

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
        std::env::var("GITNEXUS_EXPOSE_REPO_PATHS").ok().as_deref(),
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
        .route("/api/llm-config", get(llm_config_handler))
        .route("/api/diagnostics", get(diagnostics_handler))
        .route("/api/repos/:name/search", get(search_handler))
        .route("/api/repos/:name/stats", get(stats_handler))
        .route("/api/repos/:name/hotspots", get(hotspots_handler));

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
                    "http://localhost:3010".parse::<HeaderValue>().unwrap(),
                    "http://127.0.0.1".parse::<HeaderValue>().unwrap(),
                    "http://127.0.0.1:3000".parse::<HeaderValue>().unwrap(),
                    "http://127.0.0.1:3010".parse::<HeaderValue>().unwrap(),
                    "http://localhost:1420".parse::<HeaderValue>().unwrap(),
                    "http://localhost:5174".parse::<HeaderValue>().unwrap(),
                    "http://localhost:5176".parse::<HeaderValue>().unwrap(),
                    "http://127.0.0.1:5174".parse::<HeaderValue>().unwrap(),
                    "http://127.0.0.1:5176".parse::<HeaderValue>().unwrap(),
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
async fn mcp_handler(State(backend): State<SharedBackend>, body: Bytes) -> impl IntoResponse {
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

/// GET /api/llm-config — LLM model metadata for the local UI.
///
/// Deliberately omits secrets (`api_key`) and provider endpoint details.
async fn llm_config_handler() -> impl IntoResponse {
    Json(llm_config_payload())
}

fn llm_config_payload() -> Value {
    match crate::llm_config::load_llm_config() {
        Some(config) => json!({
            "configured": true,
            "provider": crate::llm_config::display_provider(&config),
            "model": config.model,
            "reasoningEffort": config.reasoning_effort,
            "maxTokens": config.max_tokens,
            "bigContextModel": config.big_context_model,
        }),
        None => json!({ "configured": false }),
    }
}

/// GET /api/diagnostics — Safe runtime metadata for the local chat UI.
///
/// This intentionally excludes tokens, API keys, OAuth account data, provider
/// base URLs, and repository filesystem paths unless the existing path-exposure
/// flag is enabled elsewhere. The goal is to help a local user diagnose setup
/// issues without creating a support bundle full of secrets.
async fn diagnostics_handler(State(backend): State<SharedBackend>) -> impl IntoResponse {
    let backend_guard = backend.lock().await;
    Json(diagnostics_payload(&backend_guard))
}

fn diagnostics_payload(backend: &LocalBackend) -> Value {
    let repos = backend.registry();
    let expose_paths = expose_repo_paths();
    let generated_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();

    json!({
        "service": "gitnexus",
        "version": env!("CARGO_PKG_VERSION"),
        "generatedAtUnixMs": generated_at_unix_ms,
        "httpAuthRequired": http_auth_token().is_some(),
        "repoPathsExposed": expose_paths,
        "repos": {
            "count": repos.len(),
            "names": repos.iter().map(|entry| {
                json!({
                    "id": registry_entry_id(entry),
                    "name": entry.name,
                    "pathExposed": expose_paths,
                    "indexedAt": entry.indexed_at,
                })
            }).collect::<Vec<_>>(),
        },
        "llm": llm_config_payload(),
        "auth": {
            "chatgptOAuth": chatgpt_oauth_status_payload(),
        },
    })
}

#[derive(Debug, Deserialize)]
struct StoredChatGptAuthFile {
    pub tokens: StoredChatGptTokens,
    #[serde(default, rename = "last_refresh")]
    pub last_refresh: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StoredChatGptTokens {
    #[serde(default)]
    pub id_token: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
}

fn chatgpt_auth_file_path() -> PathBuf {
    gitnexus_core::storage::repo_manager::get_global_dir()
        .join("auth")
        .join("openai.json")
}

fn chatgpt_oauth_status_payload() -> Value {
    chatgpt_oauth_status_payload_at(&chatgpt_auth_file_path())
}

fn chatgpt_oauth_status_payload_at(path: &FsPath) -> Value {
    let stored = match std::fs::read(path) {
        Ok(stored) => stored,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return json!({
                "loggedIn": false,
                "status": "missing",
                "tokenFilePresent": false,
                "tokenFileReadable": false,
                "refreshTokenPresent": false,
                "storage": "none",
            });
        }
        Err(err) => {
            return json!({
                "loggedIn": false,
                "status": "unreadable",
                "tokenFilePresent": true,
                "tokenFileReadable": false,
                "refreshTokenPresent": false,
                "storage": "unknown",
                "errorKind": format!("{:?}", err.kind()),
            });
        }
    };

    let storage = if cfg!(windows) {
        if secret_payload_needs_migration(&stored) {
            "legacy_plaintext"
        } else {
            "dpapi"
        }
    } else {
        "file"
    };

    let decoded = match decode_secret_from_storage(&stored) {
        Ok(decoded) => decoded,
        Err(_) => {
            return json!({
                "loggedIn": false,
                "status": "unreadable",
                "tokenFilePresent": true,
                "tokenFileReadable": false,
                "refreshTokenPresent": false,
                "storage": storage,
                "errorKind": "secret_storage",
            });
        }
    };

    let parsed: StoredChatGptAuthFile = match serde_json::from_slice(&decoded) {
        Ok(parsed) => parsed,
        Err(_) => {
            return json!({
                "loggedIn": false,
                "status": "invalid",
                "tokenFilePresent": true,
                "tokenFileReadable": true,
                "refreshTokenPresent": false,
                "storage": storage,
                "errorKind": "json",
            });
        }
    };

    let has_access_token = !parsed.tokens.access_token.trim().is_empty();
    let has_refresh_token = !parsed.tokens.refresh_token.trim().is_empty();
    let has_id_token = !parsed.tokens.id_token.trim().is_empty();
    let logged_in = has_access_token && has_refresh_token && has_id_token;

    json!({
        "loggedIn": logged_in,
        "status": if logged_in { "logged_in" } else { "incomplete" },
        "tokenFilePresent": true,
        "tokenFileReadable": true,
        "refreshTokenPresent": has_refresh_token,
        "lastRefresh": parsed.last_refresh,
        "storage": storage,
    })
}

/// GET /api/repos — List indexed repositories
async fn repos_handler(State(backend): State<SharedBackend>) -> impl IntoResponse {
    let backend_guard = backend.lock().await;
    let expose_paths = expose_repo_paths();
    let repos: Vec<Value> = backend_guard
        .registry()
        .iter()
        .map(|e| {
            let mut obj = json!({
                "id": registry_entry_id(e),
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
        let _router = mcp_http_router();
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

    #[test]
    fn test_diagnostics_payload_omits_secrets_and_paths_by_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("GITNEXUS_EXPOSE_REPO_PATHS");
        std::env::remove_var("GITNEXUS_HTTP_TOKEN");

        let backend = LocalBackend::new();
        let payload = diagnostics_payload(&backend);
        let text = serde_json::to_string(&payload).unwrap();

        assert_eq!(payload["service"], "gitnexus");
        assert_eq!(payload["repoPathsExposed"], false);
        assert_eq!(payload["httpAuthRequired"], false);
        assert!(!text.contains("api_key"));
        assert!(!text.contains("access_token"));
        assert!(!text.contains("refresh_token"));
    }

    #[test]
    fn test_chatgpt_oauth_status_missing_is_safe() {
        let path = unique_temp_auth_path("missing");
        let payload = chatgpt_oauth_status_payload_at(&path);
        let text = serde_json::to_string(&payload).unwrap();

        assert_eq!(payload["loggedIn"], false);
        assert_eq!(payload["status"], "missing");
        assert_eq!(payload["tokenFilePresent"], false);
        assert!(!text.contains("openai.json"));
        assert!(!text.contains("access_token"));
        assert!(!text.contains("refresh_token"));
    }

    #[test]
    fn test_chatgpt_oauth_status_valid_file_omits_token_values() {
        let path = unique_temp_auth_path("valid");
        let raw = json!({
            "tokens": {
                "id_token": "secret-id-token",
                "access_token": "secret-access-token",
                "refresh_token": "secret-refresh-token"
            },
            "last_refresh": "2026-05-06T20:00:00Z"
        })
        .to_string();
        std::fs::write(&path, raw).unwrap();

        let payload = chatgpt_oauth_status_payload_at(&path);
        let text = serde_json::to_string(&payload).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(payload["loggedIn"], true);
        assert_eq!(payload["status"], "logged_in");
        assert_eq!(payload["refreshTokenPresent"], true);
        assert_eq!(payload["lastRefresh"], "2026-05-06T20:00:00Z");
        assert!(!text.contains("secret-id-token"));
        assert!(!text.contains("secret-access-token"));
        assert!(!text.contains("secret-refresh-token"));
    }

    fn unique_temp_auth_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!(
            "gitnexus-chatgpt-auth-status-{label}-{}-{nanos}.json",
            std::process::id()
        ))
    }
}
