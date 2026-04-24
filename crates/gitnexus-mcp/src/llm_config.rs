//! LLM config loader mirrored from `gitnexus-cli/src/commands/generate/enrichment.rs`.
//!
//! Kept minimal (file read + deserialize) to avoid pulling CLI-specific code
//! into the MCP crate. If a third caller needs the same format, promote this
//! to `gitnexus-core::llm::config` and deduplicate.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmConfig {
    #[serde(alias = "apiKey")]
    pub api_key: String,
    #[serde(alias = "baseUrl")]
    pub base_url: String,
    pub model: String,
}

/// Resolve `~/.gitnexus/chat-config.json` across OS home-dir env variations.
pub fn load_llm_config() -> Option<LlmConfig> {
    let candidates = [
        std::env::var("USERPROFILE").ok(),
        std::env::var("HOME").ok(),
        std::env::var("HOMEDRIVE").ok().and_then(|d| {
            std::env::var("HOMEPATH")
                .ok()
                .map(|p| format!("{}{}", d, p))
        }),
    ];
    for home in candidates.into_iter().flatten() {
        let p = PathBuf::from(home).join(".gitnexus").join("chat-config.json");
        if p.exists() {
            if let Ok(raw) = std::fs::read_to_string(&p) {
                if let Ok(cfg) = serde_json::from_str::<LlmConfig>(&raw) {
                    return Some(cfg);
                }
            }
        }
    }
    None
}
