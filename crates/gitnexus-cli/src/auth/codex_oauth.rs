//! ChatGPT Codex OAuth Authorization Code + PKCE flow.
//!
//! This module implements the same login dance the official `openai/codex`
//! Rust CLI uses, ported to a single self-contained file so we don't have
//! to drag in `codex-app-server-protocol`, `codex-config`,
//! `codex-utils-template`, etc. The flow:
//!
//! 1. Generate a 64-byte random `code_verifier`, base64url-encoded.
//! 2. SHA-256 it and base64url-encode the digest → `code_challenge` (S256).
//! 3. Spin up an `axum` callback server on `127.0.0.1:1455`.
//! 4. Open `https://auth.openai.com/oauth/authorize?...` in the user's
//!    browser. They're already signed in to ChatGPT, so it's one click.
//! 5. The browser redirects to `http://localhost:1455/auth/callback?code=...`.
//! 6. Server exchanges the code for `{id_token, access_token, refresh_token}`
//!    by POSTing to `https://auth.openai.com/oauth/token`.
//! 7. Tokens land on disk under `<GITNEXUS_HOME>/.gitnexus/auth/openai.json`.
//!
//! Once stored, [`get_access_token`] returns a usable bearer token, doing
//! an opportunistic refresh when the cached token is older than an hour.
//! The bearer the OAuth flow returns is accepted by the *public* OpenAI API
//! at `https://api.openai.com/v1/chat/completions` — no need to hit the
//! `chatgpt.com/backend-api` URL the official codex CLI uses for its
//! `/responses` endpoint, because we still speak the classic
//! `/chat/completions` shape.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::sync::oneshot;
use tokio::sync::Mutex;

/// OpenAI's public OAuth client id for the Codex CLI. This is not a
/// secret — it identifies the application to the IdP and is paired with
/// PKCE for security. Same constant used by `openai/codex` and Code Buddy.
const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

/// OAuth issuer — the URL the user's browser is sent to and the URL we
/// POST the token exchange against.
const ISSUER: &str = "https://auth.openai.com";

/// Local port the callback server listens on. Must be in OpenAI's
/// allow-list of registered redirect URIs for the client id above.
const CALLBACK_PORT: u16 = 1455;

/// OAuth scopes requested at sign-in. `offline_access` is what gives us a
/// refresh_token so we don't have to re-prompt the user every hour. The
/// `api.connectors.*` scopes are required by the ChatGPT backend; harmless
/// to request even when we only intend to call the classic `/v1` API.
const SCOPES: &str =
    "openid profile email offline_access api.connectors.read api.connectors.invoke";

/// How long to consider a cached access_token valid before refreshing it.
/// The IdP issues tokens with a TTL longer than this, but we play safe.
const TOKEN_REFRESH_AGE: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("OAuth flow failed: {0}")]
    Flow(String),
    #[error("token refresh failed: {0}")]
    Refresh(String),
    #[error("auth file IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("auth file JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Token bundle returned by the IdP. Mirrors the JSON the
/// `https://auth.openai.com/oauth/token` endpoint returns.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OauthTokens {
    pub id_token: String,
    pub access_token: String,
    pub refresh_token: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub account_id: Option<String>,
}

/// Persistent auth file. We store the last refresh timestamp as an ISO-8601
/// string (rather than `chrono::DateTime<Utc>`) so we don't have to enable
/// the optional `serde` feature on the workspace `chrono` dependency just
/// for this struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuthFile {
    pub tokens: OauthTokens,
    pub last_refresh: String,
}

fn auth_file_path() -> PathBuf {
    gitnexus_core::storage::repo_manager::get_global_dir()
        .join("auth")
        .join("openai.json")
}

fn load_auth() -> Result<Option<AuthFile>, AuthError> {
    let path = auth_file_path();
    match std::fs::read_to_string(&path) {
        Ok(raw) => {
            let parsed: AuthFile = serde_json::from_str(&raw)?;
            Ok(Some(parsed))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(AuthError::Io(e)),
    }
}

fn save_auth(auth: &AuthFile) -> Result<(), AuthError> {
    let path = auth_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(auth)?;
    std::fs::write(&path, raw)?;
    Ok(())
}

/// Remove the cached tokens. `gitnexus logout` calls this.
pub fn clear() -> Result<(), AuthError> {
    let path = auth_file_path();
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AuthError::Io(e)),
    }
}

#[derive(Debug, Clone)]
struct PkceCodes {
    code_verifier: String,
    code_challenge: String,
}

/// Generate a fresh PKCE pair. Identical to `openai/codex/codex-rs/login/pkce.rs`.
fn generate_pkce() -> PkceCodes {
    let mut bytes = [0u8; 64];
    rand::rng().fill_bytes(&mut bytes);
    let code_verifier = URL_SAFE_NO_PAD.encode(bytes);
    let digest = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = URL_SAFE_NO_PAD.encode(digest);
    PkceCodes {
        code_verifier,
        code_challenge,
    }
}

fn random_state() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Exchange the authorization code received from the IdP for the bearer
/// token bundle. Posts an `application/x-www-form-urlencoded` body — the
/// IdP rejects JSON bodies on this endpoint.
async fn exchange_code(
    client: &reqwest::Client,
    code: &str,
    code_verifier: &str,
) -> Result<OauthTokens, AuthError> {
    let redirect_uri = format!("http://localhost:{CALLBACK_PORT}/auth/callback");
    let form = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri.as_str()),
        ("client_id", CLIENT_ID),
        ("code_verifier", code_verifier),
    ];
    let resp = client
        .post(format!("{ISSUER}/oauth/token"))
        .form(&form)
        .send()
        .await
        .map_err(|e| AuthError::Flow(format!("token exchange request: {e}")))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AuthError::Flow(format!(
            "token exchange returned {status}: {body}"
        )));
    }
    let tokens: OauthTokens = resp
        .json()
        .await
        .map_err(|e| AuthError::Flow(format!("token exchange JSON: {e}")))?;
    Ok(tokens)
}

/// Use the refresh_token to get a fresh access_token. The IdP rotates the
/// refresh_token on every call, so we always overwrite both.
async fn refresh_tokens(
    client: &reqwest::Client,
    refresh_token: &str,
) -> Result<OauthTokens, AuthError> {
    let form = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", CLIENT_ID),
    ];
    let resp = client
        .post(format!("{ISSUER}/oauth/token"))
        .form(&form)
        .send()
        .await
        .map_err(|e| AuthError::Refresh(e.to_string()))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AuthError::Refresh(format!("{status}: {body}")));
    }
    let tokens: OauthTokens = resp
        .json()
        .await
        .map_err(|e| AuthError::Refresh(format!("JSON: {e}")))?;
    Ok(tokens)
}

/// Returns a usable access_token, opportunistically refreshing the cached
/// one when it's older than [`TOKEN_REFRESH_AGE`]. Returns `Ok(None)` when
/// no credentials are on disk yet (caller should suggest `gitnexus login`).
pub async fn get_access_token() -> Result<Option<String>, AuthError> {
    let Some(mut auth) = load_auth()? else {
        return Ok(None);
    };
    let last_refresh: chrono::DateTime<Utc> = auth
        .last_refresh
        .parse::<chrono::DateTime<chrono::FixedOffset>>()
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now() - chrono::Duration::days(1));
    let age = (Utc::now() - last_refresh)
        .to_std()
        .unwrap_or(Duration::ZERO);
    if age < TOKEN_REFRESH_AGE {
        return Ok(Some(auth.tokens.access_token));
    }
    // Stale — refresh.
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .context("build reqwest client")?;
    match refresh_tokens(&client, &auth.tokens.refresh_token).await {
        Ok(fresh) => {
            auth.tokens = fresh.clone();
            auth.last_refresh = Utc::now().to_rfc3339();
            save_auth(&auth)?;
            Ok(Some(fresh.access_token))
        }
        Err(AuthError::Refresh(msg)) => {
            // Refresh failed: likely revoked or expired. Wipe credentials so
            // the next call falls back cleanly to `chat-config.json::api_key`.
            tracing::warn!("OAuth refresh failed ({msg}); clearing cached tokens");
            let _ = clear();
            Ok(None)
        }
        Err(other) => Err(other),
    }
}

/// State the callback handler hands back to the main flow after a
/// successful exchange. `Err` carries the user-facing failure message.
type CallbackOutcome = Result<OauthTokens, String>;

#[derive(Clone)]
struct CallbackState {
    code_verifier: Arc<String>,
    expected_state: Arc<String>,
    sender: Arc<Mutex<Option<oneshot::Sender<CallbackOutcome>>>>,
    http: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

async fn callback_handler(
    State(st): State<CallbackState>,
    Query(q): Query<CallbackQuery>,
) -> impl IntoResponse {
    let send = |outcome: CallbackOutcome| async {
        // Take the sender exactly once — second redirect would be a no-op.
        if let Some(tx) = st.sender.lock().await.take() {
            let _ = tx.send(outcome);
        }
    };

    if let Some(err) = q.error {
        let detail = q.error_description.unwrap_or_else(|| err.clone());
        send(Err(format!("OAuth provider error: {detail}"))).await;
        return (
            StatusCode::BAD_REQUEST,
            Html(error_html("OpenAI a refusé la connexion", &detail)),
        );
    }

    let (Some(code), Some(state)) = (q.code, q.state) else {
        send(Err("callback missing code or state".to_string())).await;
        return (
            StatusCode::BAD_REQUEST,
            Html(error_html(
                "Réponse incomplète",
                "code ou state manquant dans le callback",
            )),
        );
    };

    if state != *st.expected_state {
        send(Err("state mismatch (possible CSRF)".to_string())).await;
        return (
            StatusCode::BAD_REQUEST,
            Html(error_html(
                "État invalide",
                "Le state OAuth ne correspond pas — possible attaque CSRF.",
            )),
        );
    }

    match exchange_code(&st.http, &code, &st.code_verifier).await {
        Ok(tokens) => {
            send(Ok(tokens)).await;
            (StatusCode::OK, Html(success_html()))
        }
        Err(e) => {
            send(Err(e.to_string())).await;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(error_html("Échec de l'échange de jetons", &e.to_string())),
            )
        }
    }
}

fn success_html() -> String {
    r##"<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><title>GitNexus — Connecté</title>
<style>body{font-family:system-ui;background:#0a0a0a;color:#e5e5e5;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0}
.card{background:#171717;border:1px solid #262626;border-radius:12px;padding:32px 40px;max-width:420px;text-align:center}
h1{font-size:18px;margin:0 0 8px;color:#a7f3d0}
p{margin:0;font-size:14px;color:#a3a3a3;line-height:1.5}</style>
</head><body><div class="card"><h1>✅ Authentifié à ChatGPT</h1>
<p>Tu peux fermer cet onglet et retourner dans ton terminal — GitNexus a stocké ton jeton.</p>
<script>setTimeout(() => window.close(), 1200)</script>
</div></body></html>"##
        .to_string()
}

fn error_html(title: &str, detail: &str) -> String {
    let safe_title = html_escape(title);
    let safe_detail = html_escape(detail);
    format!(
        r##"<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><title>GitNexus — Erreur OAuth</title>
<style>body{{font-family:system-ui;background:#0a0a0a;color:#e5e5e5;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0}}
.card{{background:#171717;border:1px solid #7f1d1d;border-radius:12px;padding:32px 40px;max-width:520px}}
h1{{font-size:18px;margin:0 0 12px;color:#fca5a5}}
pre{{margin:0;font-size:12px;color:#a3a3a3;white-space:pre-wrap;word-break:break-word}}</style>
</head><body><div class="card"><h1>{safe_title}</h1><pre>{safe_detail}</pre></div></body></html>"##
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Run the full interactive login flow: spin a callback server, open the
/// browser, wait for the redirect, exchange the code, persist the tokens.
///
/// Blocks until the user completes (or rejects) the flow in their browser.
/// 5-minute timeout so a forgotten browser tab doesn't leave a zombie task.
pub async fn login() -> Result<(), AuthError> {
    let pkce = generate_pkce();
    let state = random_state();
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .context("build reqwest client")?;

    let (tx, rx) = oneshot::channel::<CallbackOutcome>();
    let cb_state = CallbackState {
        code_verifier: Arc::new(pkce.code_verifier.clone()),
        expected_state: Arc::new(state.clone()),
        sender: Arc::new(Mutex::new(Some(tx))),
        http,
    };

    let app = Router::new()
        .route("/auth/callback", get(callback_handler))
        .with_state(cb_state);

    let addr: SocketAddr = ([127, 0, 0, 1], CALLBACK_PORT).into();
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind callback server on {addr} (port already in use?)"))?;

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    let auth_url = build_authorize_url(&pkce.code_challenge, &state);
    eprintln!("Opening browser for ChatGPT login…");
    eprintln!("  {auth_url}");
    if webbrowser::open(&auth_url).is_err() {
        eprintln!("Couldn't auto-open the browser — copy the URL above into your browser manually.");
    }

    // Wait for callback (or timeout).
    let outcome = match tokio::time::timeout(Duration::from_secs(300), rx).await {
        Ok(Ok(outcome)) => outcome,
        Ok(Err(_)) => Err("callback channel dropped before completion".to_string()),
        Err(_) => Err("login timed out after 5 minutes".to_string()),
    };

    let _ = shutdown_tx.send(());
    let _ = server.await;

    let tokens = outcome.map_err(AuthError::Flow)?;
    let auth = AuthFile {
        tokens,
        last_refresh: Utc::now().to_rfc3339(),
    };
    save_auth(&auth)?;
    eprintln!("Saved tokens to {}", auth_file_path().display());
    Ok(())
}

fn build_authorize_url(code_challenge: &str, state: &str) -> String {
    let redirect_uri = format!("http://localhost:{CALLBACK_PORT}/auth/callback");
    // Manual query-string assembly. We percent-encode by piggy-backing on
    // serde_json + url::form_urlencoded would add a dependency for nothing.
    let params = json!({
        "response_type": "code",
        "client_id": CLIENT_ID,
        "redirect_uri": redirect_uri,
        "scope": SCOPES,
        "code_challenge": code_challenge,
        "code_challenge_method": "S256",
        "id_token_add_organizations": "true",
        "codex_cli_simplified_flow": "true",
        "state": state,
    });
    let qs = params
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| format!("{k}={}", percent_encode(v.as_str().unwrap_or(""))))
        .collect::<Vec<_>>()
        .join("&");
    format!("{ISSUER}/oauth/authorize?{qs}")
}

/// Minimal RFC 3986 percent-encoder for the URL params we emit. We escape
/// every byte that is not in the unreserved set, which is pessimistic but
/// always safe.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
