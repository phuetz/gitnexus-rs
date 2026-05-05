//! OAuth authentication for gitnexus.
//!
//! This crate-level entry point currently exposes the **Codex** OAuth flow,
//! which lets the user authenticate against their ChatGPT Pro / Plus
//! subscription and use that quota for OpenAI API calls instead of paying
//! per-token through a developer API key.
//!
//! Why we need this: the LLM tool loop in `commands::ask::ask_question_with_tools`
//! can fire 5–8 OpenAI calls per question (one per tool round-trip). On a
//! free-tier API key that's an instant rate-limit smackdown. On a ChatGPT
//! Pro subscription the quota is generous, the model is GPT-5+, and the
//! marginal cost is zero — Patrice already pays the $200/month flat.
//!
//! Borrowed from the `openai/codex` Rust CLI, which OpenAI publishes under
//! Apache-2.0. Their crate ships the official PKCE-based Authorization
//! Code Flow that the ChatGPT IdP accepts. Our port keeps only the parts
//! that work standalone (no `codex-config`, no `codex-utils-template`,
//! no `tiny_http` dependency — we use `axum` which is already in the tree).

pub mod codex_oauth;

pub use codex_oauth::{clear, get_access_token, login};
