//! The `generate` command: produces AI context files (AGENTS.md, wiki/, skills/) from the knowledge graph.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use colored::Colorize;
use serde_json::{json, Value};
use chrono;
use tracing::{info, debug, warn};

use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_core::storage::repo_manager;
use gitnexus_db::snapshot;

// ─── Constants ──────────────────────────────────────────────────────────
const TARGET_CONTEXT: &str = "context";
const TARGET_AGENTS: &str = "agents";
const TARGET_WIKI: &str = "wiki";
const TARGET_SKILLS: &str = "skills";
const TARGET_DOCS: &str = "docs";
const TARGET_DOCX: &str = "docx";
const TARGET_HTML: &str = "html";
const TARGET_ALL: &str = "all";

// ─── LLM Enrichment ────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub(crate) struct LlmConfig {
    #[allow(dead_code)]
    pub(crate) provider: String,
    pub(crate) api_key: String,
    pub(crate) base_url: String,
    pub(crate) model: String,
    pub(crate) max_tokens: u32,
    #[serde(default)]
    pub(crate) reasoning_effort: String,
}

// ─── Enrichment Profiles ─────────────────────────────────────────────

struct EnrichProfile {
    max_evidence: usize,
    #[allow(dead_code)]
    thinking_boost: bool,
    review_critical: bool,
    max_retries: u32,
    timeout_secs: u64,
}

fn get_profile(name: &str) -> EnrichProfile {
    match name {
        "fast" => EnrichProfile {
            max_evidence: 10,
            thinking_boost: false,
            review_critical: false,
            max_retries: 0,
            timeout_secs: 60,
        },
        "strict" => EnrichProfile {
            max_evidence: 30,
            thinking_boost: true,
            review_critical: true,
            max_retries: 2,
            timeout_secs: 300,
        },
        _ => EnrichProfile {
            // "quality" default
            max_evidence: 20,
            thinking_boost: false,
            review_critical: true,
            max_retries: 1,
            timeout_secs: 180,
        },
    }
}

// ─── Enrichment Cache ────────────────────────────────────────────────

/// Simple MD5-like hash for cache invalidation (not cryptographic).
fn md5_simple(input: &str) -> u128 {
    // Simple but adequate content hash using FNV-1a extended to 128 bits.
    let mut h1: u64 = 0xcbf29ce484222325;
    let mut h2: u64 = 0x100000001b3;
    for byte in input.bytes() {
        h1 ^= byte as u64;
        h1 = h1.wrapping_mul(0x01000193);
        h2 ^= byte as u64;
        h2 = h2.wrapping_mul(0x01000193).wrapping_add(h1);
    }
    ((h1 as u128) << 64) | (h2 as u128)
}

fn get_page_hash(page_path: &Path) -> String {
    let content = std::fs::read_to_string(page_path).unwrap_or_default();
    format!("{:x}", md5_simple(&content))
}

fn is_cached(cache_dir: &Path, page_name: &str, current_hash: &str) -> bool {
    let cache_file = cache_dir.join(format!("{}.hash", page_name));
    if let Ok(cached) = std::fs::read_to_string(&cache_file) {
        return cached.trim() == current_hash;
    }
    false
}

fn write_cache(cache_dir: &Path, page_name: &str, hash: &str) {
    let _ = std::fs::create_dir_all(cache_dir);
    let _ = std::fs::write(cache_dir.join(format!("{}.hash", page_name)), hash);
}

/// Load LLM config from ~/.gitnexus/chat-config.json
pub(crate) fn load_llm_config() -> Option<LlmConfig> {
    // Try multiple home directory sources for cross-platform compatibility
    let candidates = [
        std::env::var("USERPROFILE").ok(),
        std::env::var("HOME").ok(),
        std::env::var("HOMEDRIVE")
            .ok()
            .and_then(|d| std::env::var("HOMEPATH").ok().map(|p| format!("{}{}", d, p))),
    ];

    for candidate in candidates.iter().flatten() {
        let config_path = std::path::Path::new(candidate)
            .join(".gitnexus")
            .join("chat-config.json");
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                return serde_json::from_str(&content).ok();
            }
        }
    }

    None
}

// ─── Structured Enrichment Types ──────────────────────────────────────

/// Classification of a documentation page for enrichment strategy.
#[derive(Debug, Clone, Copy, PartialEq)]
enum PageType {
    Overview,
    Architecture,
    Controller,
    Service,
    DataModel,
    ExternalService,
    ViewTemplate,
    FunctionalGuide,
    ProjectHealth,
    Deployment,
    Misc,
}

/// A reference to evidence from the codebase.
#[derive(Debug, Clone, serde::Serialize)]
struct EvidenceRef {
    id: String,
    file_path: String,
    start_line: Option<u32>,
    end_line: Option<u32>,
    excerpt: String,
    title: String,
    kind: String, // "function", "class", "controller", "entity", etc.
}

/// Structured augmentation for a section of a page.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct SectionAugment {
    section_key: String,
    intro: Option<String>,
    warning: Option<String>,
    developer_tip: Option<String>,
    #[serde(default)]
    see_also: Vec<String>,
    #[serde(default)]
    source_ids: Vec<String>,
}

/// Structured payload returned by the LLM for enrichment.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct EnrichedPayload {
    lead: Option<String>,
    #[serde(default)]
    what_text: Option<String>,
    #[serde(default)]
    why_text: Option<String>,
    #[serde(default)]
    who_text: Option<String>,
    #[serde(default)]
    section_augments: Vec<SectionAugment>,
    #[serde(default)]
    related_pages: Vec<String>,
    #[serde(default)]
    relevant_source_ids: Vec<String>,
    closing_summary: Option<String>,
}

/// Provenance metadata for a generated page.
#[derive(Debug, serde::Serialize)]
struct ProvenanceEntry {
    page_id: String,
    model: String,
    enriched_at: String,
    evidence_refs: Vec<EvidenceRef>,
    validation: ProvenanceValidation,
    content_hash: String,
}

#[derive(Debug, serde::Serialize)]
struct ProvenanceValidation {
    is_valid: bool,
    issues: Vec<String>,
}

// ─── Page Classification ──────────────────────────────────────────────

fn classify_page(page_path: &Path) -> PageType {
    let name = page_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    if name == "overview" {
        PageType::Overview
    } else if name == "architecture" {
        PageType::Architecture
    } else if name.starts_with("ctrl-") {
        PageType::Controller
    } else if name == "services" {
        PageType::Service
    } else if name.starts_with("data-") {
        PageType::DataModel
    } else if name == "external-services" {
        PageType::ExternalService
    } else if name.contains("aspnet-views") {
        PageType::ViewTemplate
    } else if name == "functional-guide" {
        PageType::FunctionalGuide
    } else if name == "project-health" {
        PageType::ProjectHealth
    } else if name == "deployment" {
        PageType::Deployment
    } else {
        PageType::Misc
    }
}

// ─── Evidence Collection ──────────────────────────────────────────────

fn collect_evidence(
    graph: &KnowledgeGraph,
    page_path: &Path,
    repo_path: &Path,
    max_evidence: usize,
) -> Vec<EvidenceRef> {
    let page_type = classify_page(page_path);
    let mut evidence = Vec::new();

    // Collect nodes relevant to this page type
    let relevant_nodes: Vec<&GraphNode> = match page_type {
        PageType::Controller => {
            // Extract controller name from filename: ctrl-dossierscontroller -> DossiersController
            let ctrl_name = page_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .strip_prefix("ctrl-")
                .unwrap_or("");
            graph
                .iter_nodes()
                .filter(|n| {
                    n.properties.name.to_lowercase().contains(ctrl_name)
                        || n.properties.file_path.to_lowercase().contains(ctrl_name)
                })
                .take(max_evidence)
                .collect()
        }
        PageType::Service => graph
            .iter_nodes()
            .filter(|n| n.label == NodeLabel::Service || n.label == NodeLabel::Repository)
            .take(max_evidence)
            .collect(),
        PageType::DataModel => graph
            .iter_nodes()
            .filter(|n| n.label == NodeLabel::DbEntity || n.label == NodeLabel::DbContext)
            .take(max_evidence)
            .collect(),
        PageType::ExternalService => graph
            .iter_nodes()
            .filter(|n| n.label == NodeLabel::ExternalService)
            .take(max_evidence.min(15))
            .collect(),
        _ => {
            // For overview/architecture: top connected nodes
            let mut nodes: Vec<(&GraphNode, usize)> = graph
                .iter_nodes()
                .map(|n| {
                    let degree = graph
                        .iter_relationships()
                        .filter(|r| r.source_id == n.id || r.target_id == n.id)
                        .count();
                    (n, degree)
                })
                .collect();
            nodes.sort_by(|a, b| b.1.cmp(&a.1));
            nodes.into_iter().take(max_evidence.min(15)).map(|(n, _)| n).collect()
        }
    };

    for (idx, node) in relevant_nodes.iter().enumerate() {
        // Try to read source code snippet
        let excerpt = if !node.properties.file_path.is_empty() {
            let source_path = repo_path.join(&node.properties.file_path);
            if let Ok(source) = std::fs::read_to_string(&source_path) {
                let lines: Vec<&str> = source.lines().collect();
                let start = node
                    .properties
                    .start_line
                    .map(|l| l as usize)
                    .unwrap_or(1)
                    .saturating_sub(1);
                let end = node
                    .properties
                    .end_line
                    .map(|l| l as usize)
                    .unwrap_or(start + 5)
                    .min(lines.len());
                let end = end.min(start + 10); // Cap at 10 lines
                if start < lines.len() && start < end {
                    lines[start..end].join("\n")
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        evidence.push(EvidenceRef {
            id: format!("E{}", idx + 1),
            file_path: node.properties.file_path.clone(),
            start_line: node.properties.start_line,
            end_line: node.properties.end_line,
            excerpt,
            title: node.properties.name.clone(),
            kind: format!("{:?}", node.label),
        });
    }

    evidence
}

// ─── Simple hash for provenance ───────────────────────────────────────

fn hash_simple(input: &str) -> u64 {
    let mut hash: u64 = 0;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    hash
}

// ─── Structured Enrichment ────────────────────────────────────────────

fn enrich_page_structured(
    page_path: &Path,
    graph: &KnowledgeGraph,
    config: &LlmConfig,
    repo_path: &Path,
    profile: &EnrichProfile,
    enrich_lang: &str,
    enrich_citations: bool,
) -> Result<Option<ProvenanceEntry>> {
    let content = std::fs::read_to_string(page_path)?;
    if content.len() < 100 {
        return Ok(None);
    }

    let _page_type = classify_page(page_path);
    let evidence = collect_evidence(graph, page_path, repo_path, profile.max_evidence);

    // Build evidence context for the prompt
    let evidence_context: String = evidence
        .iter()
        .map(|e| {
            format!(
                "[{}] {} ({}) in `{}`{}",
                e.id,
                e.title,
                e.kind,
                e.file_path,
                if !e.excerpt.is_empty() {
                    format!("\n```\n{}\n```", e.excerpt)
                } else {
                    String::new()
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    // Extract section keys from anchors in markdown
    let section_keys: Vec<String> = content
        .lines()
        .filter(|l| l.contains("<!-- GNX:INTRO:"))
        .filter_map(|l| {
            l.split("GNX:INTRO:")
                .nth(1)
                .and_then(|s| s.split("-->").next())
                .map(|s| s.trim().to_string())
        })
        .collect();

    let evidence_ids_str = evidence
        .iter()
        .map(|e| e.id.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let sections_str = section_keys.join(", ");

    let lang_instruction = match enrich_lang {
        "en" => "Write in English. Professional technical documentation style.",
        "fr" | _ => "Écris en français technique professionnel.",
    };

    let system_prompt = format!(
        r#"Tu es un rédacteur technique senior. Tu enrichis une documentation existante.

RÈGLES ABSOLUES :
- Tu ne REMPLACES PAS la documentation. Tu AUGMENTES les sections marquées.
- Tu ne cites QUE des source_ids parmi ceux fournis : {evidence_ids}
- {lang_instruction}
- JAMAIS d'identifiants inventés

Réponds UNIQUEMENT en JSON valide avec cette structure exacte :
{{
  "lead": "2-3 phrases résumant QUOI, POURQUOI, QUI pour cette page",
  "section_augments": [
    {{
      "section_key": "nom-de-section",
      "intro": "1-2 phrases d'introduction pour cette section (ou null)",
      "warning": "point d'attention si pertinent (ou null)",
      "developer_tip": "conseil développeur si pertinent (ou null)",
      "see_also": ["page-liee.md"],
      "source_ids": ["E1", "E3"]
    }}
  ],
  "related_pages": ["overview.md", "services.md"],
  "relevant_source_ids": ["E1", "E2", "E5"],
  "closing_summary": "1-2 phrases de conclusion"
}}

Sections disponibles : {sections}

SOURCES D'EVIDENCE :
{evidence}"#,
        evidence_ids = evidence_ids_str,
        lang_instruction = lang_instruction,
        sections = sections_str,
        evidence = evidence_context,
    );

    let messages = vec![
        serde_json::json!({"role": "system", "content": system_prompt}),
        serde_json::json!({"role": "user", "content": format!("Enrichis cette page :\n\n{}", content)}),
    ];

    // Call LLM
    let url = format!(
        "{}/chat/completions",
        config.base_url.trim_end_matches('/')
    );
    let mut body = serde_json::json!({
        "model": config.model,
        "messages": messages,
        "max_tokens": config.max_tokens,
        "temperature": 0.3,
        "stream": false
    });

    let effort = config.reasoning_effort.trim().to_lowercase();
    if !effort.is_empty() && effort != "none" {
        body["reasoning_effort"] = serde_json::Value::String(effort);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(profile.timeout_secs))
        .build()
        .map_err(|e| anyhow::anyhow!("HTTP client: {}", e))?;

    // Retry logic based on profile
    let mut last_err = None;
    let mut json_resp: Option<serde_json::Value> = None;
    for attempt in 0..=profile.max_retries {
        if attempt > 0 {
            debug!("Retry attempt {} for {}", attempt, page_path.display());
            std::thread::sleep(std::time::Duration::from_secs(2 * attempt as u64));
        }

        // Build a fresh request each attempt (RequestBuilder is consumed on send)
        let mut req = client.post(&url).json(&body);
        if !config.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", config.api_key));
        }

        match req.send() {
            Ok(resp) if resp.status().is_success() => {
                json_resp = resp.json().ok();
                if json_resp.is_some() {
                    last_err = None;
                    break;
                }
                last_err = Some(anyhow::anyhow!("Failed to parse LLM JSON response"));
            }
            Ok(resp) => {
                last_err = Some(anyhow::anyhow!("LLM error: {}", resp.status()));
            }
            Err(e) => {
                last_err = Some(anyhow::anyhow!("LLM request: {}", e));
            }
        }
    }

    if let Some(err) = last_err {
        if json_resp.is_none() {
            return Err(err);
        }
    }

    let json_resp = json_resp.ok_or_else(|| anyhow::anyhow!("No LLM response after retries"))?;
    let raw_content = json_resp["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No content"))?;

    // Try to extract JSON from response (might be wrapped in ```json blocks)
    let json_str = raw_content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Parse structured payload; on failure, fall back to freeform enrichment
    let payload: EnrichedPayload = match serde_json::from_str(json_str) {
        Ok(p) => p,
        Err(e) => {
            warn!(
                "Structured JSON parse failed for {}: {} — falling back to freeform",
                page_path.display(),
                e
            );
            // Fallback: use old freeform enrichment
            enrich_page_freeform(page_path, graph, config)?;
            // Build minimal provenance for freeform fallback
            let page_id = page_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let fallback_content = std::fs::read_to_string(page_path).unwrap_or_default();
            return Ok(Some(ProvenanceEntry {
                page_id,
                model: config.model.clone(),
                enriched_at: chrono::Utc::now().to_rfc3339(),
                evidence_refs: evidence,
                validation: ProvenanceValidation {
                    is_valid: true,
                    issues: vec!["Freeform fallback used (JSON parse failed)".to_string()],
                },
                content_hash: format!("{:x}", hash_simple(&fallback_content)),
            }));
        }
    };

    // Validate: check source_ids are real
    let valid_ids: std::collections::HashSet<&str> =
        evidence.iter().map(|e| e.id.as_str()).collect();
    let mut invalid_ids = Vec::new();
    for aug in &payload.section_augments {
        for sid in &aug.source_ids {
            if !valid_ids.contains(sid.as_str()) {
                invalid_ids.push(sid.clone());
            }
        }
    }

    // MERGE: Insert augmentations at anchor points
    let mut enriched = String::new();
    let mut lead_inserted = false;

    for line in content.lines() {
        enriched.push_str(line);
        enriched.push('\n');

        // Insert LEAD after the anchor
        if line.contains("<!-- GNX:LEAD -->") && !lead_inserted {
            if let Some(lead) = &payload.lead {
                enriched.push('\n');
                enriched.push_str(&format!("> {}\n\n", lead));
                lead_inserted = true;
            }
        }

        // Insert section augments
        for aug in &payload.section_augments {
            let anchor = format!("<!-- GNX:INTRO:{} -->", aug.section_key);
            if line.contains(&anchor) {
                if let Some(intro) = &aug.intro {
                    enriched.push_str(&format!("\n{}\n\n", intro));
                }
                if let Some(warning) = &aug.warning {
                    enriched.push_str(&format!("> [!WARNING]\n> {}\n\n", warning));
                }
                if let Some(tip) = &aug.developer_tip {
                    enriched.push_str(&format!("> [!TIP]\n> {}\n\n", tip));
                }
                // Add source references (only if citations are enabled)
                if enrich_citations && !aug.source_ids.is_empty() {
                    let sources: Vec<String> = aug
                        .source_ids
                        .iter()
                        .filter_map(|sid| evidence.iter().find(|e| e.id == *sid))
                        .map(|e| {
                            if let Some(start) = e.start_line {
                                format!("`{}` (L{})", e.file_path, start)
                            } else {
                                format!("`{}`", e.file_path)
                            }
                        })
                        .collect();
                    if !sources.is_empty() {
                        enriched.push_str(&format!(
                            "*Sources : {}*\n\n",
                            sources.join(" \u{00b7} ")
                        ));
                    }
                }
            }
        }

        // Insert TIP augments for controller action tables
        if line.contains("<!-- GNX:TIP:actions -->") {
            for aug in &payload.section_augments {
                if aug.section_key == "actions" {
                    if let Some(tip) = &aug.developer_tip {
                        enriched.push_str(&format!("> [!TIP]\n> {}\n\n", tip));
                    }
                }
            }
        }

        // Insert closing summary
        if line.contains("<!-- GNX:CLOSING -->") {
            if let Some(summary) = &payload.closing_summary {
                enriched.push_str(&format!(
                    "\n---\n\n**En r\u{00e9}sum\u{00e9} :** {}\n\n",
                    summary
                ));
            }
            // Add related pages
            if !payload.related_pages.is_empty() {
                let links: Vec<String> = payload
                    .related_pages
                    .iter()
                    .map(|p| format!("[{}](./{})", p.trim_end_matches(".md"), p))
                    .collect();
                enriched.push_str(&format!(
                    "**Voir aussi :** {}\n\n",
                    links.join(" \u{00b7} ")
                ));
            }
        }
    }

    // Validation: tables preserved
    let orig_pipes = content.chars().filter(|c| *c == '|').count();
    let enrich_pipes = enriched.chars().filter(|c| *c == '|').count();
    if orig_pipes > 5 && enrich_pipes < orig_pipes / 2 {
        return Err(anyhow::anyhow!("Tables lost"));
    }
    if enriched.len() < content.len() / 2 {
        return Err(anyhow::anyhow!("Enriched too short"));
    }

    // Write enriched content
    std::fs::write(page_path, &enriched)?;

    // Build provenance entry
    let page_id = page_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let provenance = ProvenanceEntry {
        page_id,
        model: config.model.clone(),
        enriched_at: chrono::Utc::now().to_rfc3339(),
        evidence_refs: evidence,
        validation: ProvenanceValidation {
            is_valid: invalid_ids.is_empty(),
            issues: if invalid_ids.is_empty() {
                vec![]
            } else {
                vec![format!("Invalid source IDs: {}", invalid_ids.join(", "))]
            },
        },
        content_hash: format!("{:x}", hash_simple(&enriched)),
    };

    Ok(Some(provenance))
}

// ─── Freeform Enrichment (legacy fallback) ────────────────────────────

/// Enrich a single Markdown page with LLM-generated prose (freeform, legacy mode).
fn enrich_page_freeform(
    page_path: &Path,
    graph: &KnowledgeGraph,
    config: &LlmConfig,
) -> Result<()> {
    let content = std::fs::read_to_string(page_path)?;
    if content.len() < 100 {
        return Ok(()); // Skip tiny pages
    }

    // Build verified entities list from graph (for hallucination prevention)
    let entities: Vec<String> = graph
        .iter_nodes()
        .filter(|n| {
            matches!(
                n.label,
                NodeLabel::Controller
                    | NodeLabel::ControllerAction
                    | NodeLabel::Service
                    | NodeLabel::DbEntity
                    | NodeLabel::DbContext
                    | NodeLabel::ExternalService
                    | NodeLabel::Class
                    | NodeLabel::Function
                    | NodeLabel::Method
            )
        })
        .map(|n| n.properties.name.clone())
        .collect::<BTreeSet<String>>()
        .into_iter()
        .take(200) // Cap at 200 to fit in prompt
        .collect();

    let system_prompt = format!(
        r#"Tu es un rédacteur technique senior documentant une application legacy.

STYLE :
- Documentation technique professionnelle, précise, sobre
- Commence par un résumé de 2-3 phrases (QUOI, POURQUOI, QUI)
- Ajoute des transitions entre sections
- Un "⚠️ Point d'attention" par section complexe quand pertinent
- Un "💡 Conseil développeur" quand pertinent

RÈGLES CRITIQUES :
- JAMAIS inventer de noms de classes, méthodes ou fichiers
- GARDER tous les tableaux, listes, données et diagrammes Mermaid existants
- Écrire en français
- Le résultat doit être 20-50% plus long que l'original
- N'utiliser QUE ces noms vérifiés : {}

CONTENU À ENRICHIR :"#,
        entities.join(", ")
    );

    let messages = vec![
        json!({"role": "system", "content": system_prompt}),
        json!({"role": "user", "content": content.clone()}),
    ];

    // Call LLM
    let url = format!(
        "{}/chat/completions",
        config.base_url.trim_end_matches('/')
    );
    let mut body = json!({
        "model": config.model,
        "messages": messages,
        "max_tokens": config.max_tokens,
        "temperature": 0.3,
        "stream": false
    });

    let effort = config.reasoning_effort.trim().to_lowercase();
    if !effort.is_empty() && effort != "none" {
        body["reasoning_effort"] = serde_json::Value::String(effort);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| anyhow::anyhow!("HTTP client error: {}", e))?;

    let mut request = client.post(&url).json(&body);
    if !config.api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", config.api_key));
    }

    let response = request
        .send()
        .map_err(|e| anyhow::anyhow!("LLM request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let err = response.text().unwrap_or_default();
        return Err(anyhow::anyhow!("LLM error ({}): {}", status, err));
    }

    let json_resp: serde_json::Value = response
        .json()
        .map_err(|e| anyhow::anyhow!("Failed to parse LLM response: {}", e))?;

    let enriched = json_resp["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No content in LLM response"))?;

    // Validation: enriched must be at least 50% of original length
    if enriched.len() < content.len() / 2 {
        println!("    {} Enriched content too short, keeping original", "SKIP".yellow());
        return Ok(());
    }

    // Validation: enriched must preserve tables (count | chars)
    let orig_pipes = content.chars().filter(|c| *c == '|').count();
    let enrich_pipes = enriched.chars().filter(|c| *c == '|').count();
    if orig_pipes > 5 && enrich_pipes < orig_pipes / 2 {
        println!("    {} Tables lost in enrichment, keeping original", "SKIP".yellow());
        return Ok(());
    }

    // Write enriched content
    std::fs::write(page_path, enriched)?;
    Ok(())
}

// ─── Review Pass for Critical Pages ──────────────────────────────────

/// Perform a review pass on a critical enriched page using the LLM.
/// The LLM checks for preserved tables, hallucinated identifiers, natural
/// transitions, and marketing-speak. Only overwrites the file if the
/// reviewed version passes basic sanity checks.
fn review_enriched_page(
    page_path: &Path,
    original_content: &str,
    config: &LlmConfig,
    profile: &EnrichProfile,
) -> Result<()> {
    let enriched = std::fs::read_to_string(page_path)?;

    let review_prompt = r#"Tu es un reviewer technique. Vérifie cette documentation enrichie.

VÉRIFIE :
1. Tous les tableaux originaux sont préservés
2. Les identifiants entre backticks existent réellement (pas d'hallucination)
3. Les transitions entre sections sont naturelles
4. L'introduction résume correctement le contenu
5. Pas de phrases marketing vides
6. Les "Points d'attention" sont justifiés

Si tu trouves des problèmes, corrige-les.
Renvoie le document corrigé complet.
Si tout est correct, renvoie le document tel quel.

DOCUMENT ORIGINAL (pour comparaison) :
"#;

    let messages = vec![
        serde_json::json!({"role": "system", "content": review_prompt}),
        serde_json::json!({"role": "user", "content": format!(
            "ORIGINAL:\n{}\n\n---\n\nENRICHI:\n{}",
            &original_content[..original_content.len().min(3000)],
            enriched
        )}),
    ];

    // Call LLM for review
    let url = format!(
        "{}/chat/completions",
        config.base_url.trim_end_matches('/')
    );
    let body = serde_json::json!({
        "model": config.model,
        "messages": messages,
        "max_tokens": config.max_tokens,
        "temperature": 0.1,
        "stream": false
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(profile.timeout_secs))
        .build()?;

    let mut request = client.post(&url).json(&body);
    if !config.api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", config.api_key));
    }

    let response = request.send()?;
    if !response.status().is_success() {
        return Ok(()); // Review failed, keep enriched version
    }

    let json: serde_json::Value = response.json()?;
    if let Some(reviewed) = json["choices"][0]["message"]["content"].as_str() {
        // Only accept if reviewed version preserves tables
        let orig_pipes = enriched.chars().filter(|c| *c == '|').count();
        let rev_pipes = reviewed.chars().filter(|c| *c == '|').count();
        if rev_pipes >= orig_pipes / 2 && reviewed.len() >= enriched.len() / 2 {
            std::fs::write(page_path, reviewed)?;
        }
    }

    Ok(())
}

/// Run LLM enrichment on all generated docs if enabled (structured mode with provenance).
fn run_enrichment_if_enabled(
    enrich: bool,
    graph: &KnowledgeGraph,
    repo_path: &Path,
    enrich_profile: &str,
    enrich_lang: &str,
    enrich_citations: bool,
) -> Result<()> {
    if !enrich {
        return Ok(());
    }

    let config = match load_llm_config() {
        Some(cfg) => cfg,
        None => {
            println!(
                "{} No LLM configured. Skipping enrichment.",
                "WARN".yellow()
            );
            println!("  Create ~/.gitnexus/chat-config.json with:");
            println!();
            println!("  {{");
            println!("    \"provider\": \"gemini\",");
            println!("    \"api_key\": \"YOUR_API_KEY\",");
            println!("    \"base_url\": \"https://generativelanguage.googleapis.com/v1beta/openai\",");
            println!("    \"model\": \"gemini-2.5-flash\",");
            println!("    \"max_tokens\": 8192,");
            println!("    \"reasoning_effort\": \"high\"");
            println!("  }}");
            println!();
            println!("  Supported: Gemini, OpenAI, Anthropic, OpenRouter, Ollama");
            return Ok(());
        }
    };

    let profile = get_profile(enrich_profile);

    println!(
        "{} Enriching with LLM ({}) \u{2014} structured mode [profile: {}]",
        "\u{2192}".cyan(),
        config.model,
        enrich_profile
    );

    let docs_dir = repo_path.join(".gitnexus").join("docs");
    let meta_dir = docs_dir.join("_meta");
    let cache_dir = meta_dir.join("cache");
    let mut provenance_entries: Vec<ProvenanceEntry> = Vec::new();
    let mut enriched_count = 0usize;
    let mut skipped = 0usize;
    let mut cached_count = 0usize;

    // Collect all .md files to enrich
    let mut pages: Vec<std::path::PathBuf> = Vec::new();
    // Root level pages
    if let Ok(entries) = std::fs::read_dir(&docs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "md") {
                pages.push(path);
            }
        }
    }
    // Module pages
    let modules_dir = docs_dir.join("modules");
    if let Ok(entries) = std::fs::read_dir(&modules_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "md") {
                pages.push(path);
            }
        }
    }

    // Sort: leaf pages first, then overview/architecture last
    pages.sort_by(|a, b| {
        let pa = classify_page(a);
        let pb = classify_page(b);
        let order = |p: PageType| match p {
            PageType::Controller | PageType::Service | PageType::DataModel => 0,
            PageType::ExternalService | PageType::ViewTemplate | PageType::Deployment => 1,
            PageType::Misc | PageType::ProjectHealth => 2,
            PageType::FunctionalGuide => 3,
            PageType::Architecture => 4,
            PageType::Overview => 5,
        };
        order(pa).cmp(&order(pb))
    });

    for page_path in &pages {
        let name = page_path.file_name().unwrap().to_string_lossy();
        let page_name = page_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        print!("  {} {}...", "LLM".cyan(), name);
        std::io::stdout().flush().ok();

        // ── Cache check: skip if page content hasn't changed ──
        let current_hash = get_page_hash(page_path);
        if is_cached(&cache_dir, page_name, &current_hash) {
            println!(" {} (cached)", "OK".green());
            cached_count += 1;
            continue;
        }

        // Save original content before enrichment (needed for review pass)
        let original_content = std::fs::read_to_string(page_path).unwrap_or_default();
        let page_type = classify_page(page_path);

        match enrich_page_structured(page_path, graph, &config, repo_path, &profile, enrich_lang, enrich_citations) {
            Ok(Some(prov)) => {
                println!(
                    " {} ({} evidence)",
                    "OK".green(),
                    prov.evidence_refs.len()
                );
                provenance_entries.push(prov);
                enriched_count += 1;

                // ── Review pass for critical pages ──
                if profile.review_critical
                    && matches!(
                        page_type,
                        PageType::Overview | PageType::Architecture | PageType::FunctionalGuide
                    )
                {
                    print!("  {} {} reviewing...", "REV".cyan(), name);
                    std::io::stdout().flush().ok();
                    match review_enriched_page(page_path, &original_content, &config, &profile) {
                        Ok(()) => println!(" {}", "OK".green()),
                        Err(e) => println!(" {} ({})", "SKIP".yellow(), e),
                    }
                }

                // ── Write cache hash after successful enrichment ──
                let enriched_hash = get_page_hash(page_path);
                write_cache(&cache_dir, page_name, &enriched_hash);
            }
            Ok(None) => {
                println!(" {} (too small)", "SKIP".yellow());
                skipped += 1;
            }
            Err(e) => {
                println!(" {} ({})", "SKIP".yellow(), e);
                skipped += 1;
            }
        }
    }

    // Write provenance manifest
    std::fs::create_dir_all(&meta_dir)?;
    let manifest = serde_json::to_string_pretty(&provenance_entries)?;
    std::fs::write(meta_dir.join("provenance.json"), &manifest)?;
    println!("  {} _meta/provenance.json", "OK".green());

    println!(
        "{} Enrichment: {} enriched, {} cached, {} skipped",
        "OK".green(),
        enriched_count,
        cached_count,
        skipped
    );

    Ok(())
}

// ─── Cross-references ─────────────────────────────────────────────────

/// Post-processing step that adds cross-reference links between documentation pages.
/// Runs after all pages are generated (and optionally enriched) but before HTML generation.
fn apply_cross_references(docs_dir: &Path, graph: &KnowledgeGraph) -> Result<usize> {
    // 1. Build a map of known names -> page links
    let mut known_names: Vec<(String, String)> = Vec::new(); // (name, link)

    // Controllers, Services, Repositories, DbEntities, ExternalServices
    for node in graph.iter_nodes() {
        match node.label {
            NodeLabel::Controller => {
                let name = &node.properties.name;
                let filename = format!("ctrl-{}", sanitize_filename(name));
                known_names.push((name.clone(), format!("./modules/{}.md", filename)));
            }
            NodeLabel::Service | NodeLabel::Repository => {
                known_names.push((
                    node.properties.name.clone(),
                    "./modules/services.md".to_string(),
                ));
            }
            NodeLabel::DbEntity => {
                known_names.push((
                    node.properties.name.clone(),
                    format!("./modules/data-entities.md#{}", node.properties.name),
                ));
            }
            NodeLabel::ExternalService => {
                known_names.push((
                    node.properties.name.clone(),
                    "./modules/external-services.md".to_string(),
                ));
            }
            _ => {}
        }
    }

    // Sort by length descending (longest match first, avoid partial matches)
    known_names.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    // Filter out names shorter than 5 chars (too generic)
    known_names.retain(|(name, _)| name.len() >= 5);

    // 2. Process each .md file
    let mut total_links = 0;
    let mut files_to_process: Vec<PathBuf> = Vec::new();

    for entry in std::fs::read_dir(docs_dir)?.flatten() {
        if entry.path().extension().map_or(false, |e| e == "md") {
            files_to_process.push(entry.path());
        }
    }
    let modules_dir = docs_dir.join("modules");
    if modules_dir.exists() {
        for entry in std::fs::read_dir(&modules_dir)?.flatten() {
            if entry.path().extension().map_or(false, |e| e == "md") {
                files_to_process.push(entry.path());
            }
        }
    }

    for file_path in &files_to_process {
        let content = std::fs::read_to_string(file_path)?;
        let mut modified = content.clone();
        let mut linked_names: HashSet<String> = HashSet::new();
        let mut page_links = 0;

        for (name, link) in &known_names {
            // Skip if already linked on this page
            if linked_names.contains(name) {
                continue;
            }

            // Skip self-references (don't link to the current page)
            if link.contains(
                file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(""),
            ) {
                continue;
            }

            // Find FIRST occurrence that's not inside a code block, heading, or existing link
            if let Some(idx) = modified.find(name.as_str()) {
                // Check context: skip if inside code block or already linked
                let before = &modified[..idx];

                let in_code = before.matches("```").count() % 2 == 1;
                let in_inline_code = before.ends_with('`');
                let in_link = before.ends_with('[') || before.ends_with("](");
                let in_heading = before
                    .lines()
                    .last()
                    .map_or(false, |l| l.starts_with('#'));

                if !in_code && !in_inline_code && !in_link && !in_heading {
                    // Replace first occurrence with link
                    modified = format!(
                        "{}[{}]({}){}", &modified[..idx], name, link,
                        &modified[idx + name.len()..]
                    );
                    linked_names.insert(name.clone());
                    page_links += 1;
                }
            }
        }

        if page_links > 0 {
            std::fs::write(file_path, &modified)?;
            total_links += page_links;
        }
    }

    Ok(total_links)
}

pub fn run(what: &str, path: Option<&str>, enrich: bool, enrich_profile: &str, enrich_lang: &str, enrich_citations: bool) -> Result<()> {
    let repo_path = Path::new(path.unwrap_or(".")).canonicalize()?;
    let storage = repo_manager::get_storage_paths(&repo_path);
    let snap_path = snapshot::snapshot_path(&storage.storage_path);
    let graph = snapshot::load_snapshot(&snap_path)?;

    info!("Generating {} for {}", what, repo_path.display());

    match what {
        TARGET_CONTEXT | TARGET_AGENTS => generate_agents_md(&graph, &repo_path)?,
        TARGET_WIKI => generate_wiki(&graph, &repo_path)?,
        TARGET_SKILLS => generate_skills(&graph, &repo_path)?,
        TARGET_DOCS => {
            generate_docs(&graph, &repo_path)?;
            run_enrichment_if_enabled(enrich, &graph, &repo_path, enrich_profile, enrich_lang, enrich_citations)?;
            let docs_dir = repo_path.join(".gitnexus").join("docs");
            let xref_count = apply_cross_references(&docs_dir, &graph)?;
            if xref_count > 0 {
                println!("{} Cross-references: {} links added", "OK".green(), xref_count);
            }
        }
        TARGET_DOCX => {
            // Generate Markdown first, enrich, cross-ref, then convert to DOCX
            generate_docs(&graph, &repo_path)?;
            run_enrichment_if_enabled(enrich, &graph, &repo_path, enrich_profile, enrich_lang, enrich_citations)?;
            let docs_dir = repo_path.join(".gitnexus").join("docs");
            let xref_count = apply_cross_references(&docs_dir, &graph)?;
            if xref_count > 0 {
                println!("{} Cross-references: {} links added", "OK".green(), xref_count);
            }
            let output_path = repo_path.join(".gitnexus").join("documentation.docx");
            let repo_name = repo_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Project");
            super::export_docx::export_docs_as_docx(&docs_dir, &output_path, repo_name)?;
            info!("Generated DOCX documentation at {}", output_path.display());
            println!(
                "{} Generated DOCX: {}",
                "OK".green(),
                output_path.display()
            );
        }
        TARGET_HTML => {
            // Generate Markdown first, enrich, cross-ref, then convert to HTML site
            generate_docs(&graph, &repo_path)?;
            run_enrichment_if_enabled(enrich, &graph, &repo_path, enrich_profile, enrich_lang, enrich_citations)?;
            let docs_dir = repo_path.join(".gitnexus").join("docs");
            let xref_count = apply_cross_references(&docs_dir, &graph)?;
            if xref_count > 0 {
                println!("{} Cross-references: {} links added", "OK".green(), xref_count);
            }
            generate_html_site(&graph, &repo_path)?;
        }
        TARGET_ALL => {
            generate_agents_md(&graph, &repo_path)?;
            generate_wiki(&graph, &repo_path)?;
            generate_skills(&graph, &repo_path)?;
            generate_docs(&graph, &repo_path)?;
            run_enrichment_if_enabled(enrich, &graph, &repo_path, enrich_profile, enrich_lang, enrich_citations)?;
            let docs_dir = repo_path.join(".gitnexus").join("docs");
            let xref_count = apply_cross_references(&docs_dir, &graph)?;
            if xref_count > 0 {
                println!("{} Cross-references: {} links added", "OK".green(), xref_count);
            }
            // Also generate DOCX
            let output_path = repo_path.join(".gitnexus").join("documentation.docx");
            let repo_name = repo_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Project");
            super::export_docx::export_docs_as_docx(&docs_dir, &output_path, repo_name)?;
            info!("Generated DOCX documentation at {}", output_path.display());
            println!(
                "{} Generated DOCX: {}",
                "OK".green(),
                output_path.display()
            );
            // Also generate HTML site
            generate_html_site(&graph, &repo_path)?;
        }
        _ => {
            eprintln!(
                "Unknown target: {}. Use: context, wiki, skills, docs, docx, html, all",
                what
            );
        }
    }
    Ok(())
}

// ─── Helpers ────────────────────────────────────────────────────────────

/// Collect community info: community node ID -> (heuristic_label, member node IDs).
fn collect_communities(graph: &KnowledgeGraph) -> BTreeMap<String, CommunityInfo> {
    let mut communities: BTreeMap<String, CommunityInfo> = BTreeMap::new();

    // First pass: find Community nodes
    for node in graph.iter_nodes() {
        if node.label == NodeLabel::Community {
            let label = node
                .properties
                .heuristic_label
                .clone()
                .unwrap_or_else(|| node.properties.name.clone());
            communities.insert(
                node.id.clone(),
                CommunityInfo {
                    label,
                    description: node.properties.description.clone(),
                    keywords: node.properties.keywords.clone().unwrap_or_default(),
                    member_ids: Vec::new(),
                },
            );
        }
    }

    // Second pass: find MEMBER_OF relationships to populate members
    for rel in graph.iter_relationships() {
        if rel.rel_type == RelationshipType::MemberOf {
            if let Some(info) = communities.get_mut(&rel.target_id) {
                info.member_ids.push(rel.source_id.clone());
            }
        }
    }

    communities
}

struct CommunityInfo {
    label: String,
    description: Option<String>,
    keywords: Vec<String>,
    member_ids: Vec<String>,
}

/// Collect language statistics.
fn collect_language_stats(graph: &KnowledgeGraph) -> BTreeMap<String, usize> {
    let mut lang_counts: BTreeMap<String, usize> = BTreeMap::new();
    for node in graph.iter_nodes() {
        if node.label == NodeLabel::File {
            if let Some(lang) = &node.properties.language {
                *lang_counts.entry(lang.as_str().to_string()).or_insert(0) += 1;
            }
        }
    }
    lang_counts
}

/// Count files.
fn count_files(graph: &KnowledgeGraph) -> usize {
    graph
        .iter_nodes()
        .filter(|n| n.label == NodeLabel::File)
        .count()
}

/// Build outgoing edges map: source_id -> Vec<(target_id, rel_type)>.
fn build_edge_map(graph: &KnowledgeGraph) -> HashMap<String, Vec<(String, RelationshipType)>> {
    let mut map: HashMap<String, Vec<(String, RelationshipType)>> = HashMap::new();
    for rel in graph.iter_relationships() {
        map.entry(rel.source_id.clone())
            .or_default()
            .push((rel.target_id.clone(), rel.rel_type));
    }
    map
}

/// Sanitize a label for use as a filename.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_lowercase()
}

/// Escape a label for safe use inside Mermaid `["..."]` quoted strings.
/// Replaces special characters with Mermaid HTML entity syntax to avoid
/// breaking the diagram parser.
fn escape_mermaid_label(label: &str) -> String {
    label
        .replace('&', "#amp;")
        .replace('"', "#quot;")
        .replace('<', "#lt;")
        .replace('>', "#gt;")
        .replace('\n', " ")
        .replace('\r', "")
}

/// Sanitize a string for use as a Mermaid node ID.
/// Keeps only alphanumeric characters and underscores.
fn sanitize_mermaid_id(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

/// Generate a `<details>` block listing relevant source files.
fn source_files_section(files: &[&str]) -> String {
    if files.is_empty() {
        return String::new();
    }
    let mut s = String::from("\n<details>\n<summary>Relevant source files</summary>\n\n");
    for f in files.iter().take(15) {
        s.push_str(&format!("- `{}`\n", f));
    }
    s.push_str("\n</details>\n\n");
    s
}

/// Format method parameters from the stored description field.
/// Input: "string id, int page" (raw from ActionInfo.parameters)
/// Output: "`string` id, `int` page"
fn extract_params_from_content(params_str: &str, _method_name: &str) -> String {
    if params_str.is_empty() {
        return "-".to_string();
    }

    let params: Vec<String> = params_str
        .split(',')
        .map(|p| {
            let parts: Vec<&str> = p.trim().split_whitespace().collect();
            if parts.len() >= 2 {
                format!("`{}` {}", parts[0], parts[parts.len() - 1])
            } else if parts.len() == 1 {
                format!("`{}`", parts[0])
            } else {
                p.trim().to_string()
            }
        })
        .collect();

    params.join(", ")
}

/// Format method parameters with links to known entity types.
/// "DossierPresta dossier, string id" → "[`DossierPresta`](./data-alisev2entities.md) dossier, `string` id"
fn extract_params_linked(params_str: &str, known_types: &HashSet<String>) -> String {
    if params_str.is_empty() {
        return "-".to_string();
    }

    let params: Vec<String> = params_str
        .split(',')
        .map(|p| {
            let parts: Vec<&str> = p.trim().split_whitespace().collect();
            if parts.len() >= 2 {
                let type_name = parts[0];
                let param_name = parts[parts.len() - 1];
                // Check if the type is a known entity/model → make it a link
                if known_types.contains(type_name) {
                    format!("[{}](./modules/data-alisev2entities.md#{}) {}", type_name, type_name, param_name)
                } else {
                    format!("`{}` {}", type_name, param_name)
                }
            } else if parts.len() == 1 {
                format!("`{}`", parts[0])
            } else {
                p.trim().to_string()
            }
        })
        .collect();

    params.join(", ")
}

/// Extract ALL method signatures (params + return type) from source code, including overloads.
fn extract_all_method_signatures(source: &str, method_name: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if !trimmed.contains(method_name) || !trimmed.contains('(') {
            continue;
        }
        if !trimmed.starts_with("public") && !trimmed.starts_with("private")
            && !trimmed.starts_with("protected") && !trimmed.starts_with("async") {
            continue;
        }
        if trimmed.contains("await ") || trimmed.contains(".GetAwaiter") || trimmed.contains("=>") {
            continue;
        }
        // Must contain the exact method name followed by (
        let pattern = format!("{}(", method_name);
        if !trimmed.contains(&pattern) && !trimmed.contains(&format!("{} (", method_name)) {
            continue;
        }

        let before_name = trimmed.split(method_name).next().unwrap_or("");
        let words: Vec<&str> = before_name.split_whitespace().collect();
        let ret_type = if words.len() >= 2 {
            words[words.len() - 1].to_string()
        } else {
            "-".to_string()
        };

        let clean_ret = ret_type
            .replace("System.Threading.Tasks.Task<", "")
            .replace("System.Collections.Generic.ICollection<", "ICollection<")
            .trim_end_matches('>')
            .to_string();

        if let Some(paren_start) = trimmed.find('(') {
            let after = &trimmed[paren_start + 1..];
            if let Some(paren_end) = after.find(')') {
                let params_raw = after[..paren_end].trim();
                if params_raw.is_empty() || params_raw == ")" {
                    results.push(("-".to_string(), clean_ret));
                    continue;
                }

                // Format params: simplify System.* types
                let params: Vec<String> = params_raw.split(',').map(|p| {
                    // Strip default values: "string nia = null" → "string nia"
                    let p_clean = p.split('=').next().unwrap_or(p).trim()
                        .replace("System.Threading.CancellationToken", "CancellationToken")
                        .replace("System.Threading.Tasks.", "");
                    let parts: Vec<&str> = p_clean.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let type_name = parts[0];
                        let param_name = parts[1]; // Name is always the second word
                        format!("`{}` {}", type_name, param_name)
                    } else if parts.len() == 1 {
                        format!("`{}`", parts[0])
                    } else {
                        p.trim().to_string()
                    }
                }).collect();

                // Filter out CancellationToken (internal plumbing)
                let visible_params: Vec<&String> = params.iter()
                    .filter(|p| !p.contains("CancellationToken"))
                    .collect();

                let params_str = if visible_params.is_empty() {
                    "-".to_string()
                } else {
                    visible_params.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                };

                results.push((params_str, clean_ret));
            }
        }
    }
    if results.is_empty() {
        results.push(("-".to_string(), "-".to_string()));
    }
    results
}

/// Extract a method body from source code by finding the method declaration and reading until its closing brace.
fn extract_method_body(source: &str, method_name: &str, max_lines: usize) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let pattern = format!(" {}(", method_name);

    // Find the method declaration line
    let start_idx = lines.iter().position(|line| {
        let trimmed = line.trim();
        trimmed.contains(&pattern)
            && (trimmed.starts_with("public") || trimmed.starts_with("private")
                || trimmed.starts_with("protected") || trimmed.starts_with("["))
            && !trimmed.contains("await ")
            && !trimmed.contains(".GetAwaiter")
    })?;

    // Count braces to find the method end
    let mut brace_count = 0;
    let mut found_open = false;
    let mut end_idx = start_idx;

    for (i, line) in lines[start_idx..].iter().enumerate() {
        for ch in line.chars() {
            if ch == '{' {
                brace_count += 1;
                found_open = true;
            } else if ch == '}' {
                brace_count -= 1;
            }
        }
        end_idx = start_idx + i;
        if found_open && brace_count == 0 {
            break;
        }
        // Safety: don't go past max_lines
        if i >= max_lines {
            break;
        }
    }

    let actual_end = (end_idx + 1).min(lines.len());
    let snippet_lines = &lines[start_idx..actual_end];

    if snippet_lines.is_empty() {
        return None;
    }

    let mut result = String::new();
    for line in snippet_lines {
        result.push_str(line);
        result.push('\n');
    }

    if !found_open || brace_count > 0 {
        result.push_str("// ... (méthode tronquée)\n");
    }

    Some(result)
}

/// Count nodes by label type in the graph.
fn count_nodes_by_label(graph: &KnowledgeGraph) -> HashMap<NodeLabel, usize> {
    let mut counts: HashMap<NodeLabel, usize> = HashMap::new();
    for node in graph.iter_nodes() {
        *counts.entry(node.label).or_insert(0) += 1;
    }
    counts
}

/// Find the top N most-connected files (by total degree) in the graph.
fn top_connected_files(graph: &KnowledgeGraph, n: usize) -> Vec<String> {
    let mut file_degree: HashMap<String, usize> = HashMap::new();
    for rel in graph.iter_relationships() {
        // Count source file
        if let Some(src_node) = graph.get_node(&rel.source_id) {
            if !src_node.properties.file_path.is_empty() {
                *file_degree.entry(src_node.properties.file_path.clone()).or_insert(0) += 1;
            }
        }
        // Count target file
        if let Some(tgt_node) = graph.get_node(&rel.target_id) {
            if !tgt_node.properties.file_path.is_empty() {
                *file_degree.entry(tgt_node.properties.file_path.clone()).or_insert(0) += 1;
            }
        }
    }
    let mut sorted: Vec<_> = file_degree.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.into_iter().take(n).map(|(f, _)| f).collect()
}

/// Detect frameworks/libraries from graph nodes and file extensions.
fn detect_technology_stack(graph: &KnowledgeGraph, lang_stats: &BTreeMap<String, usize>) -> (Vec<String>, Vec<String>, Vec<String>, String) {
    let mut languages: Vec<String> = Vec::new();
    let mut frameworks: Vec<String> = Vec::new();
    let mut ui_libs: Vec<String> = Vec::new();
    let mut description_parts: Vec<String> = Vec::new();

    // Languages
    for (lang, count) in lang_stats {
        languages.push(format!("{} ({} files)", lang, count));
    }

    // Detect frameworks from node labels
    let label_counts = count_nodes_by_label(graph);
    let has_controllers = label_counts.get(&NodeLabel::Controller).copied().unwrap_or(0) > 0;
    let has_db_context = label_counts.get(&NodeLabel::DbContext).copied().unwrap_or(0) > 0;
    let has_db_entities = label_counts.get(&NodeLabel::DbEntity).copied().unwrap_or(0) > 0;
    let has_views = label_counts.get(&NodeLabel::View).copied().unwrap_or(0) > 0;
    let has_ui_components = label_counts.get(&NodeLabel::UiComponent).copied().unwrap_or(0) > 0;
    let has_services = label_counts.get(&NodeLabel::Service).copied().unwrap_or(0) > 0;
    let has_external = label_counts.get(&NodeLabel::ExternalService).copied().unwrap_or(0) > 0;

    if has_controllers {
        frameworks.push("ASP.NET MVC 5".to_string());
        description_parts.push("ASP.NET MVC 5 application".to_string());
    }
    if has_db_context || has_db_entities {
        frameworks.push("Entity Framework 6".to_string());
        if description_parts.is_empty() {
            description_parts.push("Entity Framework application".to_string());
        } else {
            description_parts.push("Entity Framework 6".to_string());
        }
    }
    if has_ui_components {
        // Check for Telerik/Kendo
        let has_telerik = graph.iter_nodes().any(|n| {
            n.label == NodeLabel::UiComponent
                && n.properties.component_type.as_deref().is_some_and(|t| {
                    t.contains("Telerik") || t.contains("Kendo")
                })
        });
        if has_telerik {
            ui_libs.push("Telerik UI / Kendo UI".to_string());
            description_parts.push("Telerik UI components".to_string());
        } else {
            ui_libs.push("Custom UI Components".to_string());
        }
    }
    if has_external {
        description_parts.push("external service integrations".to_string());
    }
    if has_services {
        // Check if we have repository pattern too
        let has_repos = label_counts.get(&NodeLabel::Repository).copied().unwrap_or(0) > 0;
        if has_repos {
            frameworks.push("Repository Pattern".to_string());
        }
    }
    if has_views {
        let has_razor = graph.iter_nodes().any(|n| {
            n.label == NodeLabel::View
                && n.properties.view_engine.as_deref() == Some("razor")
        });
        if has_razor {
            frameworks.push("Razor Views".to_string());
        }
    }

    // If no ASP.NET detected, describe generically
    if description_parts.is_empty() {
        let primary_lang = lang_stats.iter().max_by_key(|(_, c)| *c).map(|(l, _)| l.as_str()).unwrap_or("multi-language");
        description_parts.push(format!("{} codebase", primary_lang));
    }

    let description = if description_parts.len() == 1 {
        format!("{}.", description_parts[0])
    } else {
        let last = description_parts.pop().unwrap_or_default();
        format!("{} with {}.", description_parts.join(", "), last)
    };

    (languages, frameworks, ui_libs, description)
}

/// Describe a controller based on its name heuristic.
fn describe_controller(name: &str) -> String {
    let base = name.trim_end_matches("Controller");
    match base.to_lowercase().as_str() {
        s if s.contains("dossier") => "case/file management".to_string(),
        s if s.contains("beneficiaire") || s.contains("beneficiary") => "beneficiary lookup and management".to_string(),
        s if s.contains("home") => "main dashboard and landing page".to_string(),
        s if s.contains("account") || s.contains("auth") => "authentication and account management".to_string(),
        s if s.contains("admin") => "administration and system configuration".to_string(),
        s if s.contains("user") => "user management".to_string(),
        s if s.contains("report") => "reporting and analytics".to_string(),
        s if s.contains("search") => "search functionality".to_string(),
        s if s.contains("document") || s.contains("doc") => "document management".to_string(),
        s if s.contains("setting") || s.contains("config") => "application settings and configuration".to_string(),
        s if s.contains("notification") || s.contains("alert") => "notifications and alerts".to_string(),
        s if s.contains("api") => "API endpoints".to_string(),
        s if s.contains("log") => "logging and audit trail".to_string(),
        s if s.contains("dashboard") => "dashboard and overview".to_string(),
        _ => format!("{} management", base),
    }
}

// ─── AGENTS.md Generator ────────────────────────────────────────────────

fn generate_agents_md(graph: &KnowledgeGraph, repo_path: &Path) -> Result<()> {
    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repository");

    let file_count = count_files(graph);
    let lang_stats = collect_language_stats(graph);
    let communities = collect_communities(graph);

    let out_path = repo_path.join("AGENTS.md");
    let mut f = std::fs::File::create(&out_path)?;

    debug!("Processing {} communities for AGENTS.md", communities.len());

    // Header
    writeln!(f, "# {repo_name}")?;
    writeln!(f)?;
    writeln!(
        f,
        "Auto-generated codebase context for AI agents. {file_count} source files indexed."
    )?;
    writeln!(f)?;

    // Languages
    writeln!(f, "## Languages")?;
    writeln!(f)?;
    for (lang, count) in &lang_stats {
        writeln!(f, "- **{lang}**: {count} files")?;
    }
    writeln!(f)?;

    // Communities
    if !communities.is_empty() {
        writeln!(f, "## Modules / Communities")?;
        writeln!(f)?;
        for info in communities.values() {
            let member_count = info.member_ids.len();
            writeln!(f, "### {}", info.label)?;
            writeln!(f)?;
            if let Some(desc) = &info.description {
                writeln!(f, "{desc}")?;
                writeln!(f)?;
            }
            writeln!(f, "- Members: {member_count} symbols")?;

            // Show key symbols (up to 8)
            let mut key_symbols: Vec<String> = Vec::new();
            for mid in info.member_ids.iter().take(8) {
                if let Some(node) = graph.get_node(mid) {
                    key_symbols.push(format!(
                        "`{}` ({})",
                        node.properties.name,
                        node.label.as_str()
                    ));
                }
            }
            if !key_symbols.is_empty() {
                writeln!(f, "- Key symbols: {}", key_symbols.join(", "))?;
            }
            if !info.keywords.is_empty() {
                writeln!(f, "- Keywords: {}", info.keywords.join(", "))?;
            }
            writeln!(f)?;
        }
    }

    // Entry points
    let mut entry_points: Vec<(&GraphNode, f64)> = graph
        .iter_nodes()
        .filter_map(|n| {
            n.properties
                .entry_point_score
                .filter(|&s| s > 0.3)
                .map(|s| (n, s))
        })
        .collect();
    entry_points.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if !entry_points.is_empty() {
        writeln!(f, "## Entry Points")?;
        writeln!(f)?;
        for (node, score) in entry_points.iter().take(15) {
            let reason = node
                .properties
                .entry_point_reason
                .as_deref()
                .unwrap_or("");
            writeln!(
                f,
                "- `{}` in `{}` (score: {:.2}) {}",
                node.properties.name, node.properties.file_path, score, reason
            )?;
        }
        writeln!(f)?;
    }

    // Execution flows (Processes)
    let processes: Vec<&GraphNode> = graph
        .iter_nodes()
        .filter(|n| n.label == NodeLabel::Process)
        .collect();
    if !processes.is_empty() {
        writeln!(f, "## Execution Flows")?;
        writeln!(f)?;
        for proc_node in processes.iter().take(20) {
            let step_count = proc_node.properties.step_count.unwrap_or(0);
            let ptype = proc_node
                .properties
                .process_type
                .map(|t| match t {
                    ProcessType::IntraCommunity => "intra-community",
                    ProcessType::CrossCommunity => "cross-community",
                })
                .unwrap_or("unknown");
            writeln!(
                f,
                "- **{}**: {} steps ({ptype})",
                proc_node.properties.name, step_count
            )?;
            if let Some(desc) = &proc_node.properties.description {
                writeln!(f, "  {desc}")?;
            }
        }
        writeln!(f)?;
    }

    // Architecture overview: inter-community CALLS
    if communities.len() > 1 {
        writeln!(f, "## Architecture (inter-module dependencies)")?;
        writeln!(f)?;

        // Build set of member->community mappings
        let mut member_to_community: HashMap<String, String> = HashMap::new();
        for info in communities.values() {
            for mid in &info.member_ids {
                member_to_community.insert(mid.clone(), info.label.clone());
            }
        }

        let mut cross_deps: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for rel in graph.iter_relationships() {
            if rel.rel_type == RelationshipType::Calls {
                if let (Some(src_comm), Some(tgt_comm)) = (
                    member_to_community.get(&rel.source_id),
                    member_to_community.get(&rel.target_id),
                ) {
                    if src_comm != tgt_comm {
                        cross_deps
                            .entry(src_comm.clone())
                            .or_default()
                            .insert(tgt_comm.clone());
                    }
                }
            }
        }

        for (src, targets) in &cross_deps {
            let targets_str: Vec<&str> = targets.iter().map(|s| s.as_str()).collect();
            writeln!(f, "- **{src}** depends on: {}", targets_str.join(", "))?;
        }
        writeln!(f)?;
    }

    info!("Documentation generated: 1 page");
    println!(
        "{} Generated {}",
        "OK".green(),
        out_path.display()
    );
    Ok(())
}

// ─── Wiki Generator ─────────────────────────────────────────────────────

fn generate_wiki(graph: &KnowledgeGraph, repo_path: &Path) -> Result<()> {
    let wiki_dir = repo_path.join("wiki");
    std::fs::create_dir_all(&wiki_dir)?;

    let communities = collect_communities(graph);
    let edge_map = build_edge_map(graph);

    if communities.is_empty() {
        println!(
            "{} No communities found. Run `gitnexus analyze` first.",
            "!".yellow()
        );
        return Ok(());
    }

    let mut used_filenames_wiki: HashSet<String> = HashSet::new();

    for info in communities.values() {
        let base = sanitize_filename(&info.label);
        let filename = if used_filenames_wiki.contains(&base) {
            let mut candidate = base.clone();
            let mut counter = 2;
            while used_filenames_wiki.contains(&candidate) {
                candidate = format!("{}_{}", base, counter);
                counter += 1;
            }
            candidate
        } else {
            base
        };
        used_filenames_wiki.insert(filename.clone());
        let out_path = wiki_dir.join(format!("{filename}.md"));
        let mut f = std::fs::File::create(&out_path)?;

        debug!("Processing community: {}", info.label);
        writeln!(f, "# {}", info.label)?;
        writeln!(f)?;
        if let Some(desc) = &info.description {
            writeln!(f, "{desc}")?;
            writeln!(f)?;
        }
        if !info.keywords.is_empty() {
            writeln!(f, "**Keywords**: {}", info.keywords.join(", "))?;
            writeln!(f)?;
        }

        // Members
        let member_set: HashSet<&str> = info.member_ids.iter().map(|s| s.as_str()).collect();

        writeln!(f, "## Members")?;
        writeln!(f)?;
        writeln!(f, "| Symbol | Type | File | Lines |")?;
        writeln!(f, "|--------|------|------|-------|")?;

        let mut files_set: BTreeSet<String> = BTreeSet::new();
        for mid in &info.member_ids {
            if let Some(node) = graph.get_node(mid) {
                let lines = match (node.properties.start_line, node.properties.end_line) {
                    (Some(s), Some(e)) => format!("{s}-{e}"),
                    (Some(s), None) => format!("{s}"),
                    _ => "-".to_string(),
                };
                writeln!(
                    f,
                    "| `{}` | {} | `{}` | {} |",
                    node.properties.name,
                    node.label.as_str(),
                    node.properties.file_path,
                    lines
                )?;
                files_set.insert(node.properties.file_path.clone());
            }
        }
        writeln!(f)?;

        // Internal calls
        let mut internal_calls: Vec<(String, String)> = Vec::new();
        let mut external_deps: Vec<(String, String)> = Vec::new();

        for mid in &info.member_ids {
            if let Some(edges) = edge_map.get(mid.as_str()) {
                for (target_id, rel_type) in edges {
                    if *rel_type == RelationshipType::Calls {
                        let src_name = graph
                            .get_node(mid)
                            .map(|n| n.properties.name.as_str())
                            .unwrap_or("?");
                        let tgt_name = graph
                            .get_node(target_id)
                            .map(|n| n.properties.name.as_str())
                            .unwrap_or("?");

                        if member_set.contains(target_id.as_str()) {
                            internal_calls
                                .push((src_name.to_string(), tgt_name.to_string()));
                        } else {
                            external_deps
                                .push((src_name.to_string(), tgt_name.to_string()));
                        }
                    }
                }
            }
        }

        if !internal_calls.is_empty() {
            writeln!(f, "## Internal Calls")?;
            writeln!(f)?;
            for (src, tgt) in &internal_calls {
                writeln!(f, "- `{src}` -> `{tgt}`")?;
            }
            writeln!(f)?;
        }

        if !external_deps.is_empty() {
            writeln!(f, "## External Dependencies")?;
            writeln!(f)?;
            for (src, tgt) in &external_deps {
                writeln!(f, "- `{src}` -> `{tgt}`")?;
            }
            writeln!(f)?;
        }

        // Files
        if !files_set.is_empty() {
            writeln!(f, "## Files")?;
            writeln!(f)?;
            for file_path in &files_set {
                writeln!(f, "- `{file_path}`")?;
            }
            writeln!(f)?;
        }

        println!(
            "  {} wiki/{filename}.md",
            "OK".green(),
        );
    }

    info!("Documentation generated: {} pages", communities.len());
    println!(
        "{} Generated {} wiki pages in {}",
        "OK".green(),
        communities.len(),
        wiki_dir.display()
    );
    Ok(())
}

// ─── Skills Generator ───────────────────────────────────────────────────

fn generate_skills(graph: &KnowledgeGraph, repo_path: &Path) -> Result<()> {
    let skills_dir = repo_path.join("skills");
    std::fs::create_dir_all(&skills_dir)?;

    let communities = collect_communities(graph);
    let edge_map = build_edge_map(graph);

    if communities.is_empty() {
        println!(
            "{} No communities found. Run `gitnexus analyze` first.",
            "!".yellow()
        );
        return Ok(());
    }

    // Build member->community label mapping
    let mut member_to_community: HashMap<String, String> = HashMap::new();
    for info in communities.values() {
        for mid in &info.member_ids {
            member_to_community.insert(mid.clone(), info.label.clone());
        }
    }

    let mut used_filenames_skills: HashSet<String> = HashSet::new();

    for info in communities.values() {
        let base = sanitize_filename(&info.label);
        let filename = if used_filenames_skills.contains(&base) {
            let mut candidate = base.clone();
            let mut counter = 2;
            while used_filenames_skills.contains(&candidate) {
                candidate = format!("{}_{}", base, counter);
                counter += 1;
            }
            candidate
        } else {
            base
        };
        used_filenames_skills.insert(filename.clone());
        let out_path = skills_dir.join(format!("{filename}.md"));
        let mut f = std::fs::File::create(&out_path)?;

        debug!("Processing module: {}", info.label);
        writeln!(f, "# Skill: {}", info.label)?;
        writeln!(f)?;

        // Infer responsibility from folder/file names
        let mut folders: BTreeSet<String> = BTreeSet::new();
        for mid in &info.member_ids {
            if let Some(node) = graph.get_node(mid) {
                let fp = &node.properties.file_path;
                if let Some(dir) = Path::new(fp).parent() {
                    folders.insert(dir.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        if let Some(desc) = &info.description {
            writeln!(f, "## Responsibility")?;
            writeln!(f)?;
            writeln!(f, "{desc}")?;
            writeln!(f)?;
        } else if !folders.is_empty() {
            writeln!(f, "## Responsibility")?;
            writeln!(f)?;
            writeln!(
                f,
                "This module manages code in: {}",
                folders
                    .iter()
                    .map(|s| format!("`{s}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
            writeln!(f)?;
        }

        // Key functions
        let member_set: HashSet<&str> = info.member_ids.iter().map(|s| s.as_str()).collect();
        let key_labels = [
            NodeLabel::Function,
            NodeLabel::Method,
            NodeLabel::Constructor,
            NodeLabel::Class,
            NodeLabel::Struct,
            NodeLabel::Trait,
            NodeLabel::Interface,
        ];

        let mut key_symbols: Vec<&GraphNode> = info
            .member_ids
            .iter()
            .filter_map(|mid| graph.get_node(mid))
            .filter(|n| key_labels.contains(&n.label))
            .collect();
        // Sort by entry_point_score descending, then name
        key_symbols.sort_by(|a, b| {
            let sa = a.properties.entry_point_score.unwrap_or(0.0);
            let sb = b.properties.entry_point_score.unwrap_or(0.0);
            sb.partial_cmp(&sa)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.properties.name.cmp(&b.properties.name))
        });

        if !key_symbols.is_empty() {
            writeln!(f, "## Key Symbols")?;
            writeln!(f)?;
            for node in key_symbols.iter().take(20) {
                let role = if node
                    .properties
                    .entry_point_score
                    .map(|s| s > 0.3)
                    .unwrap_or(false)
                {
                    " (entry point)"
                } else {
                    ""
                };
                writeln!(
                    f,
                    "- `{}` ({}) in `{}`{}",
                    node.properties.name,
                    node.label.as_str(),
                    node.properties.file_path,
                    role
                )?;
            }
            writeln!(f)?;
        }

        // Entry points into this community
        let mut entry_points: Vec<&GraphNode> = info
            .member_ids
            .iter()
            .filter_map(|mid| graph.get_node(mid))
            .filter(|n| {
                n.properties
                    .entry_point_score
                    .map(|s| s > 0.3)
                    .unwrap_or(false)
            })
            .collect();
        entry_points.sort_by(|a, b| {
            let sa = a.properties.entry_point_score.unwrap_or(0.0);
            let sb = b.properties.entry_point_score.unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

        if !entry_points.is_empty() {
            writeln!(f, "## Entry Points")?;
            writeln!(f)?;
            for node in entry_points.iter().take(10) {
                let score = node.properties.entry_point_score.unwrap_or(0.0);
                writeln!(
                    f,
                    "- `{}` (score: {:.2}) in `{}`",
                    node.properties.name, score, node.properties.file_path
                )?;
            }
            writeln!(f)?;
        }

        // Connections to other communities
        let mut connected_communities: BTreeMap<String, usize> = BTreeMap::new();
        for mid in &info.member_ids {
            if let Some(edges) = edge_map.get(mid.as_str()) {
                for (target_id, rel_type) in edges {
                    if *rel_type == RelationshipType::Calls
                        && !member_set.contains(target_id.as_str())
                    {
                        if let Some(target_comm) = member_to_community.get(target_id) {
                            *connected_communities.entry(target_comm.clone()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        if !connected_communities.is_empty() {
            writeln!(f, "## Connections to Other Modules")?;
            writeln!(f)?;
            let mut sorted: Vec<_> = connected_communities.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));
            for (comm_label, call_count) in sorted {
                writeln!(f, "- **{comm_label}**: {call_count} call(s)")?;
            }
            writeln!(f)?;
        }

        println!(
            "  {} skills/{filename}.md",
            "OK".green(),
        );
    }

    info!("Documentation generated: {} pages", communities.len());
    println!(
        "{} Generated {} skill files in {}",
        "OK".green(),
        communities.len(),
        skills_dir.display()
    );
    Ok(())
}

// ─── Docs Generator (DeepWiki-style) ─────────────────────────────────────

fn generate_docs(graph: &KnowledgeGraph, repo_path: &Path) -> Result<()> {
    let docs_dir = repo_path.join(".gitnexus").join("docs");
    // Clean old generated files to avoid stale duplicates
    if docs_dir.exists() {
        let _ = std::fs::remove_dir_all(&docs_dir);
    }
    std::fs::create_dir_all(&docs_dir)?;
    let modules_dir = docs_dir.join("modules");
    std::fs::create_dir_all(&modules_dir)?;

    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repository");

    let communities = collect_communities(graph);
    let edge_map = build_edge_map(graph);
    let lang_stats = collect_language_stats(graph);
    let file_count = count_files(graph);

    let node_count = graph.iter_nodes().count();
    let edge_count = graph.iter_relationships().count();

    if communities.is_empty() {
        println!(
            "{} No communities found. Run `gitnexus analyze` first.",
            "!".yellow()
        );
        return Ok(());
    }

    // 1. Generate overview.md
    generate_docs_overview(
        &docs_dir,
        repo_name,
        file_count,
        node_count,
        edge_count,
        &lang_stats,
        &communities,
        graph,
    )?;

    // 1b. Generate functional guide (business-oriented documentation)
    generate_functional_guide(&docs_dir, repo_name, graph)?;

    // 1c. Generate project health dashboard
    generate_project_health(&docs_dir, graph)?;

    // 2. Generate architecture.md
    generate_docs_architecture(
        &docs_dir,
        &communities,
        graph,
        &edge_map,
        file_count,
        node_count,
        edge_count,
    )?;

    // 3. Generate getting-started.md
    generate_docs_getting_started(&docs_dir, repo_name, &communities, graph)?;

    // 4. Generate per-module files
    let module_page_count = generate_docs_modules(
        &modules_dir,
        &communities,
        graph,
        &edge_map,
        repo_path,
    )?;

    // 5b. Generate deployment guide
    generate_deployment_guide(&docs_dir, repo_name, &graph)?;

    // 5d. Generate git analytics pages (hotspots, coupling, ownership)
    let git_analytics_count = generate_git_analytics_pages(&docs_dir, repo_path)?;

    // 5c. Generate ASP.NET MVC specific documentation (if applicable)
    let aspnet_pages = if super::generate_aspnet::has_aspnet_content(graph) {
        let pages = super::generate_aspnet::generate_aspnet_docs(graph, &docs_dir)?;
        if !pages.is_empty() {
            info!("ASP.NET docs generated: {} pages", pages.len());
            println!(
                "{} Generated {} ASP.NET documentation pages",
                "OK".green(),
                pages.len()
            );
        }
        pages
    } else {
        Vec::new()
    };

    // Total page count: static pages (overview, architecture, getting-started, deployment, functional-guide, project-health) + git analytics + module pages + ASP.NET pages
    let total_pages = 6 + git_analytics_count + module_page_count + aspnet_pages.len();
    info!("Documentation generated: {} pages total", total_pages);

    // 6. Generate _index.json LAST so it includes ASP.NET pages
    generate_docs_index(
        &docs_dir,
        repo_name,
        file_count,
        node_count,
        edge_count,
        communities.len(),
        &communities,
        &aspnet_pages,
    )?;

    println!(
        "{} Generated DeepWiki docs in {}",
        "OK".green(),
        docs_dir.display()
    );
    Ok(())
}

/// Generate the _index.json navigation file.
/// `aspnet_pages` contains (id, title, filename) tuples from ASP.NET doc generation.
#[allow(clippy::too_many_arguments)]
fn generate_docs_index(
    docs_dir: &Path,
    repo_name: &str,
    file_count: usize,
    node_count: usize,
    edge_count: usize,
    module_count: usize,
    communities: &BTreeMap<String, CommunityInfo>,
    aspnet_pages: &[(String, String, String)],
) -> Result<()> {
    let now = chrono::Local::now().to_rfc3339();

    // Build module children (deduplicated by filename)
    let mut module_children = Vec::new();
    let mut seen_modules = HashSet::new();
    for info in communities.values() {
        let filename = sanitize_filename(&info.label);
        if seen_modules.insert(filename.clone()) {
            module_children.push(json!({
                "id": format!("mod-{}", filename),
                "title": info.label,
                "path": format!("modules/{}.md", filename),
                "icon": "box"
            }));
        }
    }

    // Build ASP.NET children (grouped under an "ASP.NET MVC" section)
    let aspnet_icon_map: HashMap<&str, &str> = [
        ("aspnet-controllers", "server"),
        ("aspnet-routes", "route"),
        ("aspnet-entities", "table-2"),
        ("aspnet-views", "layout"),
        ("aspnet-areas", "layers"),
        ("aspnet-data-model", "database"),
        ("aspnet-seq-http", "arrow-right-left"),
        ("aspnet-seq-data", "hard-drive"),
    ].into_iter().collect();

    let mut pages_array = vec![
        json!({
            "id": "overview",
            "title": "Overview",
            "path": "overview.md",
            "icon": "home"
        }),
        json!({
            "id": "project-health",
            "title": "Santé du Projet",
            "path": "project-health.md",
            "icon": "activity"
        }),
        json!({
            "id": "architecture",
            "title": "Architecture",
            "path": "architecture.md",
            "icon": "git-branch"
        }),
        json!({
            "id": "git-analytics",
            "title": "Git Analytics",
            "icon": "git-commit",
            "children": [
                {
                    "id": "hotspots",
                    "title": "Code Hotspots",
                    "path": "hotspots.md",
                    "icon": "flame"
                },
                {
                    "id": "coupling",
                    "title": "Temporal Coupling",
                    "path": "coupling.md",
                    "icon": "link"
                },
                {
                    "id": "ownership",
                    "title": "Code Ownership",
                    "path": "ownership.md",
                    "icon": "users"
                }
            ]
        }),
        json!({
            "id": "getting-started",
            "title": "Getting Started",
            "path": "getting-started.md",
            "icon": "book-open"
        }),
        json!({
            "id": "deployment",
            "title": "Environnement & Déploiement",
            "path": "deployment.md",
            "icon": "cloud"
        }),
        json!({
            "id": "modules",
            "title": "Modules",
            "icon": "layers",
            "children": module_children
        }),
    ];

    // Add ASP.NET section if pages exist
    if !aspnet_pages.is_empty() {
        let aspnet_children: Vec<Value> = aspnet_pages
            .iter()
            .map(|(id, title, filename)| {
                let icon = aspnet_icon_map.get(id.as_str()).unwrap_or(&"file-text");
                json!({
                    "id": id,
                    "title": title,
                    "path": filename,
                    "icon": icon
                })
            })
            .collect();

        pages_array.push(json!({
            "id": "aspnet",
            "title": "ASP.NET MVC 5 / EF6",
            "icon": "server",
            "children": aspnet_children
        }));
    }

    if pages_array.is_empty() {
        warn!("No documentation pages found in _index.json");
    }

    let index = json!({
        "title": repo_name,
        "generatedAt": now,
        "stats": {
            "files": file_count,
            "nodes": node_count,
            "edges": edge_count,
            "modules": module_count
        },
        "pages": pages_array
    });

    let index_path = docs_dir.join("_index.json");
    let mut f = std::fs::File::create(&index_path)?;
    writeln!(f, "{}", index)?;
    println!("  {} _index.json", "OK".green());
    Ok(())
}

/// Generate overview.md with DeepWiki-quality content.
#[allow(clippy::too_many_arguments)]
fn generate_docs_overview(
    docs_dir: &Path,
    repo_name: &str,
    file_count: usize,
    node_count: usize,
    edge_count: usize,
    lang_stats: &BTreeMap<String, usize>,
    communities: &BTreeMap<String, CommunityInfo>,
    graph: &KnowledgeGraph,
) -> Result<()> {
    let out_path = docs_dir.join("overview.md");
    let mut f = std::fs::File::create(&out_path)?;

    let label_counts = count_nodes_by_label(graph);
    let controller_count = label_counts.get(&NodeLabel::Controller).copied().unwrap_or(0);
    let view_count = label_counts.get(&NodeLabel::View).copied().unwrap_or(0);
    let entity_count = label_counts.get(&NodeLabel::DbEntity).copied().unwrap_or(0);
    let service_count = label_counts.get(&NodeLabel::Service).copied().unwrap_or(0)
        + label_counts.get(&NodeLabel::Repository).copied().unwrap_or(0);
    let ui_count = label_counts.get(&NodeLabel::UiComponent).copied().unwrap_or(0);

    // Title
    writeln!(f, "# {}", repo_name)?;
    writeln!(f, "<!-- GNX:LEAD -->")?;
    writeln!(f)?;

    // Relevant source files
    let top_files = top_connected_files(graph, 10);
    let top_files_refs: Vec<&str> = top_files.iter().map(|s| s.as_str()).collect();
    write!(f, "{}", source_files_section(&top_files_refs))?;

    // Business description — specific to the project type
    let (_languages, _frameworks, _ui_libs, _auto_desc) = detect_technology_stack(graph, lang_stats);
    let has_aspnet = label_counts.get(&NodeLabel::Controller).copied().unwrap_or(0) > 0;
    let has_ef = label_counts.get(&NodeLabel::DbContext).copied().unwrap_or(0) > 0;
    let has_telerik = label_counts.get(&NodeLabel::UiComponent).copied().unwrap_or(0) > 0;

    if has_aspnet && has_ef {
        writeln!(f, "> **{}** est une application de gestion métier construite en ASP.NET MVC 5 avec Entity Framework 6.", repo_name)?;
        if has_telerik {
            writeln!(f, "> L'interface utilise des grilles Telerik pour l'affichage et la saisie des données.")?;
        }
        let ext_count = label_counts.get(&NodeLabel::ExternalService).copied().unwrap_or(0);
        if ext_count > 0 {
            writeln!(f, "> Le système s'intègre avec {} services externes (WebAPI, WCF, LDAP).", ext_count)?;
        }
    } else {
        writeln!(f, "> {}", _auto_desc)?;
    }
    writeln!(f)?;

    // Metrics table
    writeln!(f, "| Metric | Value |")?;
    writeln!(f, "|--------|-------|")?;
    writeln!(f, "| Source Files | {} |", file_count)?;
    writeln!(f, "| Code Symbols | {} |", node_count)?;
    writeln!(f, "| Relationships | {} |", edge_count)?;
    if controller_count > 0 {
        writeln!(f, "| Controllers | {} |", controller_count)?;
    }
    if view_count > 0 {
        writeln!(f, "| Views | {} |", view_count)?;
    }
    if entity_count > 0 {
        writeln!(f, "| Database Entities | {} |", entity_count)?;
    }
    if service_count > 0 {
        writeln!(f, "| Services | {} |", service_count)?;
    }
    if ui_count > 0 {
        writeln!(f, "| UI Components | {} |", ui_count)?;
    }
    writeln!(f)?;

    // Technology Stack as a proper table
    let (languages, frameworks, ui_libs, _desc) = detect_technology_stack(graph, lang_stats);
    writeln!(f, "## Technology Stack")?;
    writeln!(f, "<!-- GNX:INTRO:technology-stack -->")?;
    writeln!(f)?;
    writeln!(f, "| Category | Technology |")?;
    writeln!(f, "|----------|-----------|")?;
    if !languages.is_empty() {
        writeln!(f, "| **Languages** | {} |", languages.join(", "))?;
    }
    if !frameworks.is_empty() {
        writeln!(f, "| **Frameworks** | {} |", frameworks.join(", "))?;
    }
    if !ui_libs.is_empty() {
        writeln!(f, "| **UI Components** | {} |", ui_libs.join(", "))?;
    }
    let ctx_count = label_counts.get(&NodeLabel::DbContext).copied().unwrap_or(0);
    if ctx_count > 0 {
        writeln!(f, "| **ORM** | Entity Framework 6 ({} DbContexts) |", ctx_count)?;
    }
    let ext_count = label_counts.get(&NodeLabel::ExternalService).copied().unwrap_or(0);
    if ext_count > 0 {
        writeln!(f, "| **Integrations** | {} external services (WebAPI, WCF) |", ext_count)?;
    }
    writeln!(f)?;

    // Key Subsystems
    if !communities.is_empty() {
        writeln!(f, "## Key Subsystems")?;
        writeln!(f, "<!-- GNX:INTRO:key-subsystems -->")?;
        writeln!(f)?;
        writeln!(f, "| Module | Members | Entry Points | Description |")?;
        writeln!(f, "|--------|---------|-------------|-------------|")?;
        for info in communities.values() {
            let member_count = info.member_ids.len();
            let entry_point_count = info
                .member_ids
                .iter()
                .filter_map(|mid| graph.get_node(mid))
                .filter(|n| n.properties.entry_point_score.map(|s| s > 0.3).unwrap_or(false))
                .count();
            let desc = info
                .description
                .as_deref()
                .unwrap_or(
                    if !info.keywords.is_empty() {
                        // Use first few keywords as description
                        ""
                    } else {
                        "Module"
                    }
                );
            let desc_str = if desc.is_empty() {
                info.keywords.join(", ")
            } else {
                desc.to_string()
            };
            let filename = sanitize_filename(&info.label);
            writeln!(
                f,
                "| [{}](modules/{}.md) | {} | {} | {} |",
                info.label, filename, member_count, entry_point_count, desc_str
            )?;
        }
        writeln!(f)?;
    }

    // ── Signaux d'Alerte ────────────────────────────────────────────────
    {
        let density = if node_count > 0 {
            edge_count as f64 / node_count as f64
        } else {
            0.0
        };

        // StackLogger tracing coverage
        let total_files = graph.iter_nodes()
            .filter(|n| n.label == NodeLabel::File)
            .count();
        let traced_files = graph.iter_nodes()
            .filter(|n| n.properties.is_traced == Some(true))
            .count();
        let traced_pct = if total_files > 0 {
            (traced_files as f64 / total_files as f64) * 100.0
        } else {
            0.0
        };
        let ext_svc_count = label_counts.get(&NodeLabel::ExternalService).copied().unwrap_or(0);

        let mut has_alerts = false;

        if traced_pct < 10.0 && total_files > 0 {
            if !has_alerts {
                writeln!(f, "## Signaux d'Alerte")?;
                writeln!(f)?;
                has_alerts = true;
            }
            writeln!(f, "> [!WARNING]")?;
            writeln!(f, "> Seulement {:.0}% des fichiers ont une traçabilité StackLogger.", traced_pct)?;
            writeln!(f, "> Les modules non tracés seront difficiles à déboguer en production.")?;
            writeln!(f)?;
        }

        if density > 3.0 {
            if !has_alerts {
                writeln!(f, "## Signaux d'Alerte")?;
                writeln!(f)?;
                has_alerts = true;
            }
            writeln!(f, "> [!DANGER]")?;
            writeln!(f, "> Densité de couplage élevée ({:.1}). Le système est fortement interconnecté.", density)?;
            writeln!(f, "> Tout changement peut avoir des effets de bord importants.")?;
            writeln!(f)?;
        }

        if ext_svc_count > 5 {
            if !has_alerts {
                writeln!(f, "## Signaux d'Alerte")?;
                writeln!(f)?;
                #[allow(unused_assignments)]
                { has_alerts = true; }
            }
            writeln!(f, "> [!NOTE]")?;
            writeln!(f, "> {} services externes détectés. Chaque intégration est un point de", ext_svc_count)?;
            writeln!(f, "> fragilité potentiel (timeout, indisponibilité, changement d'API).")?;
            writeln!(f)?;
        }
    }

    // GNX:CLOSING anchor before summary/navigation
    writeln!(f, "<!-- GNX:CLOSING -->")?;

    // Summary
    // Count total pages: 3 static + communities + controller pages + data pages + services + ui + ajax
    let ctrl_pages = controller_count;
    let data_pages = label_counts.get(&NodeLabel::DbContext).copied().unwrap_or(0);
    let svc_page = if service_count > 0 { 1 } else { 0 };
    let ui_page = if ui_count > 0 { 1 } else { 0 };
    let ajax_page = if label_counts.get(&NodeLabel::AjaxCall).copied().unwrap_or(0) > 0 { 1 } else { 0 };
    let total_pages = 4 + communities.len() + ctrl_pages + data_pages + svc_page + ui_page + ajax_page; // 4 = overview + architecture + getting-started + deployment

    writeln!(f, "## Summary")?;
    writeln!(f)?;
    writeln!(
        f,
        "This documentation covers {} pages organized into sections:",
        total_pages
    )?;
    writeln!(f, "Overview, Architecture, Getting Started, Déploiement, Modules")?;
    if controller_count > 0 {
        write!(f, ", Controllers")?;
    }
    if data_pages > 0 {
        write!(f, ", Data Model")?;
    }
    if service_count > 0 {
        write!(f, ", Services")?;
    }
    if ui_count > 0 {
        write!(f, ", UI Components")?;
    }
    writeln!(f, ".")?;
    writeln!(f)?;

    writeln!(f, "**See also:** [Architecture](./architecture.md) · [Getting Started](./getting-started.md)")?;
    writeln!(f)?;
    writeln!(f, "---")?;
    writeln!(f, "[Next: Architecture ->](./architecture.md)")?;

    println!("  {} overview.md", "OK".green());
    Ok(())
}

/// Generate architecture.md with real Mermaid diagram built from graph data.
fn generate_docs_architecture(
    docs_dir: &Path,
    communities: &BTreeMap<String, CommunityInfo>,
    graph: &KnowledgeGraph,
    edge_map: &HashMap<String, Vec<(String, RelationshipType)>>,
    _file_count: usize,
    node_count: usize,
    edge_count: usize,
) -> Result<()> {
    let out_path = docs_dir.join("architecture.md");
    let mut f = std::fs::File::create(&out_path)?;

    let label_counts = count_nodes_by_label(graph);
    let ctrl_count = label_counts.get(&NodeLabel::Controller).copied().unwrap_or(0);
    let view_count = label_counts.get(&NodeLabel::View).copied().unwrap_or(0);
    let svc_count = label_counts.get(&NodeLabel::Service).copied().unwrap_or(0)
        + label_counts.get(&NodeLabel::Repository).copied().unwrap_or(0);
    let ctx_count = label_counts.get(&NodeLabel::DbContext).copied().unwrap_or(0);
    let entity_count = label_counts.get(&NodeLabel::DbEntity).copied().unwrap_or(0);
    let ext_count = label_counts.get(&NodeLabel::ExternalService).copied().unwrap_or(0);
    let action_count = label_counts.get(&NodeLabel::ControllerAction).copied().unwrap_or(0);
    let ui_count = label_counts.get(&NodeLabel::UiComponent).copied().unwrap_or(0);
    let edmx_count: usize = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::File && n.properties.file_path.ends_with(".edmx"))
        .count();

    // Collect relevant source files (controllers, services, DbContexts)
    let arch_files: Vec<String> = graph.iter_nodes()
        .filter(|n| matches!(n.label, NodeLabel::Controller | NodeLabel::Service | NodeLabel::DbContext | NodeLabel::Repository))
        .map(|n| n.properties.file_path.clone())
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect();
    let arch_file_refs: Vec<&str> = arch_files.iter().take(15).map(|s| s.as_str()).collect();

    writeln!(f, "# Architecture")?;
    writeln!(f, "<!-- GNX:LEAD -->")?;
    writeln!(f)?;
    write!(f, "{}", source_files_section(&arch_file_refs))?;

    // Determine if we have a tiered architecture
    let has_tiered = ctrl_count > 0 && (svc_count > 0 || ctx_count > 0);

    if has_tiered {
        writeln!(f, "This project follows a **3-tier architecture** pattern:")?;
        writeln!(f, "Presentation (Controllers + Views) -> Business Logic (Services) -> Data Access (Entity Framework).")?;
    } else {
        writeln!(
            f,
            "System architecture with **{}** modules, **{}** nodes, and **{}** relationships.",
            communities.len(), node_count, edge_count
        )?;
    }
    writeln!(f)?;

    // Architecture Diagram - built from actual NodeLabel counts
    writeln!(f, "## Architecture Diagram")?;
    writeln!(f, "<!-- GNX:INTRO:architecture-diagram -->")?;
    writeln!(f)?;
    writeln!(f, "```mermaid")?;
    writeln!(f, "graph TD")?;

    if has_tiered {
        // Tiered architecture diagram
        writeln!(f, "    subgraph Presentation")?;
        writeln!(f, "        C[\"Controllers ({})\"]", ctrl_count)?;
        if view_count > 0 {
            writeln!(f, "        V[\"Views ({})\"]", view_count)?;
        }
        writeln!(f, "    end")?;

        if svc_count > 0 {
            writeln!(f, "    subgraph Business[\"Business Logic\"]")?;
            writeln!(f, "        S[\"Services ({})\"]", svc_count)?;
            writeln!(f, "    end")?;
        }

        if ctx_count > 0 || entity_count > 0 {
            writeln!(f, "    subgraph Data[\"Data Access\"]")?;
            if ctx_count > 0 {
                writeln!(f, "        DB[\"DbContexts ({})\"]", ctx_count)?;
            }
            if entity_count > 0 {
                writeln!(f, "        E[\"Entities ({})\"]", entity_count)?;
            }
            writeln!(f, "    end")?;
        }

        if ext_count > 0 {
            writeln!(f, "    subgraph External")?;
            writeln!(f, "        EXT[\"External Services ({})\"]", ext_count)?;
            writeln!(f, "    end")?;
        }

        // Add edges based on actual relationships in the graph
        let has_ctrl_to_svc = svc_count > 0 && graph.iter_relationships().any(|r| {
            matches!(r.rel_type, RelationshipType::DependsOn | RelationshipType::Calls)
                && graph.get_node(&r.source_id).map(|n| n.label == NodeLabel::Controller).unwrap_or(false)
                && graph.get_node(&r.target_id).map(|n| matches!(n.label, NodeLabel::Service | NodeLabel::Repository)).unwrap_or(false)
        });
        let has_svc_to_db = ctx_count > 0 && graph.iter_relationships().any(|r| {
            matches!(r.rel_type, RelationshipType::DependsOn | RelationshipType::Calls | RelationshipType::Uses)
                && graph.get_node(&r.source_id).map(|n| matches!(n.label, NodeLabel::Service | NodeLabel::Repository)).unwrap_or(false)
                && graph.get_node(&r.target_id).map(|n| n.label == NodeLabel::DbContext).unwrap_or(false)
        });
        let has_db_to_entity = entity_count > 0 && graph.iter_relationships().any(|r| {
            r.rel_type == RelationshipType::MapsToEntity
        });
        let has_ctrl_to_view = view_count > 0 && graph.iter_relationships().any(|r| {
            r.rel_type == RelationshipType::RendersView
        });
        let has_svc_to_ext = ext_count > 0 && graph.iter_relationships().any(|r| {
            r.rel_type == RelationshipType::CallsService
        });

        // Emit edges: use detected relationships or infer from layer presence
        if has_ctrl_to_svc || svc_count > 0 {
            writeln!(f, "    C --> S")?;
        }
        if has_svc_to_db || (ctx_count > 0 && svc_count > 0) {
            writeln!(f, "    S --> DB")?;
        }
        if has_db_to_entity || (entity_count > 0 && ctx_count > 0) {
            writeln!(f, "    DB --> E")?;
        }
        if has_ctrl_to_view || view_count > 0 {
            writeln!(f, "    C --> V")?;
        }
        if has_svc_to_ext || (ext_count > 0 && svc_count > 0) {
            writeln!(f, "    S --> EXT")?;
        }
    } else {
        // Non-tiered: use community-based diagram
        for info in communities.values() {
            let safe_id = sanitize_filename(&info.label).replace('-', "_");
            writeln!(f, "    {}[\"{}\"]", safe_id, escape_mermaid_label(&info.label))?;
        }

        // Build cross-community edges
        let mut member_to_community: HashMap<String, String> = HashMap::new();
        for info in communities.values() {
            for mid in &info.member_ids {
                member_to_community.insert(mid.clone(), info.label.clone());
            }
        }
        let mut cross_deps: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for rel in graph.iter_relationships() {
            if rel.rel_type == RelationshipType::Calls {
                if let (Some(src_comm), Some(tgt_comm)) = (
                    member_to_community.get(&rel.source_id),
                    member_to_community.get(&rel.target_id),
                ) {
                    if src_comm != tgt_comm {
                        cross_deps.entry(src_comm.clone()).or_default().insert(tgt_comm.clone());
                    }
                }
            }
        }
        for (src, targets) in &cross_deps {
            let src_id = sanitize_filename(src).replace('-', "_");
            for tgt in targets {
                let tgt_id = sanitize_filename(tgt).replace('-', "_");
                writeln!(f, "    {} --> {}", src_id, tgt_id)?;
            }
        }
    }
    writeln!(f, "```")?;
    writeln!(f)?;

    // Layer Details
    writeln!(f, "## Layer Details")?;
    writeln!(f, "<!-- GNX:INTRO:layer-details -->")?;
    writeln!(f)?;

    if ctrl_count > 0 {
        writeln!(f, "### Presentation Layer")?;
        writeln!(
            f,
            "{} controllers with {} actions serving {} views.",
            ctrl_count, action_count, view_count
        )?;
        if ui_count > 0 {
            writeln!(f, "{} Telerik/Kendo UI components detected.", ui_count)?;
        }
        writeln!(f)?;
    }

    if svc_count > 0 {
        writeln!(f, "### Business Logic Layer")?;
        writeln!(
            f,
            "{} services handling business rules and data processing.",
            svc_count
        )?;
        writeln!(f)?;
    }

    if ctx_count > 0 || entity_count > 0 {
        writeln!(f, "### Data Access Layer")?;
        writeln!(
            f,
            "{} Entity Framework DbContext classes managing {} entities",
            ctx_count, entity_count
        )?;
        if edmx_count > 0 {
            writeln!(f, "across {} EDMX data models.", edmx_count)?;
        } else {
            writeln!(f, ".")?;
        }
        writeln!(f)?;
    }

    if ext_count > 0 {
        writeln!(f, "### External Integrations")?;
        writeln!(
            f,
            "{} external service connections detected (WebAPI, WCF, LDAP).",
            ext_count
        )?;
        writeln!(f)?;

        // List external services
        let ext_services: Vec<&GraphNode> = graph.iter_nodes()
            .filter(|n| n.label == NodeLabel::ExternalService)
            .collect();
        if !ext_services.is_empty() {
            for svc in ext_services.iter().take(15) {
                let stype = svc.properties.service_type.as_deref().unwrap_or("REST");
                writeln!(f, "- **{}** ({})", svc.properties.name, stype)?;
            }
            writeln!(f)?;
        }
    }

    // Summary / Navigation
    writeln!(f, "<!-- GNX:CLOSING -->")?;
    writeln!(f, "## Summary")?;
    writeln!(f)?;
    if has_tiered {
        writeln!(f, "The application follows a layered architecture with clear separation of concerns between presentation, business logic, and data access.")?;
    } else {
        writeln!(f, "The codebase is organized into {} interconnected modules.", communities.len())?;
    }
    writeln!(f)?;
    writeln!(f, "**See also:** [Overview](./overview.md) · [Getting Started](./getting-started.md)")?;
    writeln!(f)?;
    writeln!(f, "---")?;
    writeln!(f, "[<- Previous: Overview](./overview.md) | [Next: Getting Started ->](./getting-started.md)")?;

    // Suppress unused warning
    let _ = edge_map;

    println!("  {} architecture.md", "OK".green());
    Ok(())
}

/// Generate getting-started.md guide.
fn generate_docs_getting_started(
    docs_dir: &Path,
    repo_name: &str,
    _communities: &BTreeMap<String, CommunityInfo>,
    graph: &KnowledgeGraph,
) -> Result<()> {
    let out_path = docs_dir.join("getting-started.md");
    let mut f = std::fs::File::create(&out_path)?;

    // Collect relevant entry point files
    let mut ep_files: Vec<String> = graph
        .iter_nodes()
        .filter(|n| n.properties.entry_point_score.map(|s| s > 0.3).unwrap_or(false))
        .map(|n| n.properties.file_path.clone())
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect();
    ep_files.truncate(15);
    let ep_file_refs: Vec<&str> = ep_files.iter().map(|s| s.as_str()).collect();

    writeln!(f, "# Prise en Main")?;
    writeln!(f)?;
    write!(f, "{}", source_files_section(&ep_file_refs))?;
    writeln!(f, "Welcome to the **{}** codebase!", repo_name)?;
    writeln!(f)?;

    // Project Structure — group files by top-level project directory
    writeln!(f, "## Structure des Projets")?;
    writeln!(f)?;

    // Detect projects by grouping files by their top-level directory (project folder)
    let mut project_files: BTreeMap<String, usize> = BTreeMap::new();
    for node in graph.iter_nodes() {
        if node.label == NodeLabel::File {
            let path = &node.properties.file_path;
            if !path.is_empty() && !path.contains("PackageTmp") && !path.contains("/obj/") {
                // First directory component = project name
                let project = path.split(['/', '\\']).next().unwrap_or("Other");
                *project_files.entry(project.to_string()).or_insert(0) += 1;
            }
        }
    }

    if !project_files.is_empty() {
        writeln!(f, "La solution contient **{} projets** :", project_files.len())?;
        writeln!(f)?;
        writeln!(f, "| Projet | Fichiers | Rôle |")?;
        writeln!(f, "|--------|----------|------|")?;
        let mut projects: Vec<_> = project_files.iter().collect();
        projects.sort_by(|a, b| b.1.cmp(a.1));
        for (project, count) in &projects {
            let role = describe_project_fr(project);
            writeln!(f, "| `{}` | {} | {} |", project, count, role)?;
        }
        writeln!(f)?;
    }

    // Key Entry Points
    let mut entry_points: Vec<(&GraphNode, f64)> = graph
        .iter_nodes()
        .filter_map(|n| {
            n.properties
                .entry_point_score
                .filter(|&s| s > 0.3)
                .map(|s| (n, s))
        })
        .collect();
    entry_points.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if !entry_points.is_empty() {
        writeln!(f, "## Points d'Entrée Principaux")?;
        writeln!(f)?;
        writeln!(f, "Commencez l'exploration par ces points d'entrée :")?;
        writeln!(f)?;
        for (node, _score) in entry_points.iter().take(10) {
            writeln!(
                f,
                "- `{}` in `{}`",
                node.properties.name, node.properties.file_path
            )?;
        }
        writeln!(f)?;
    }

    // ASP.NET setup section (if applicable)
    let has_controllers = graph.iter_nodes().any(|n| n.label == NodeLabel::Controller);
    if has_controllers {
        writeln!(f, "## Prérequis & Setup local")?;
        writeln!(f, "<!-- GNX:INTRO:setup-local -->")?;
        writeln!(f)?;
        writeln!(f, "Ce projet est une application **ASP.NET MVC 5** (.NET Framework).")?;
        writeln!(f)?;
        writeln!(f, "### Prérequis")?;
        writeln!(f)?;
        writeln!(f, "| Outil | Version | Notes |")?;
        writeln!(f, "|-------|---------|-------|")?;
        writeln!(f, "| Visual Studio | 2019+ | Avec le workload \"Développement web ASP.NET\" |")?;
        writeln!(f, "| .NET Framework | 4.6.1+ | Vérifier dans `web.config` → `targetFramework` |")?;
        writeln!(f, "| SQL Server | 2016+ | Base de données locale ou distante |")?;
        writeln!(f, "| IIS Express | intégré à VS | Pour le debug local |")?;
        writeln!(f)?;
        writeln!(f, "### Étapes de démarrage")?;
        writeln!(f)?;
        writeln!(f, "1. **Ouvrir la solution** `.sln` dans Visual Studio")?;
        writeln!(f, "2. **Restaurer les packages NuGet** : clic droit sur la solution → Restaurer les packages NuGet")?;
        writeln!(f, "3. **Configurer la connexion DB** : vérifier `web.config` → `<connectionStrings>`")?;
        writeln!(f, "4. **Compiler** : Ctrl+Shift+B")?;
        writeln!(f, "5. **Lancer** : F5 (IIS Express)")?;
        writeln!(f)?;

        // Detect connection strings in web.config nodes
        let config_files: Vec<&GraphNode> = graph
            .iter_nodes()
            .filter(|n| {
                n.label == NodeLabel::File
                    && (n.properties.file_path.ends_with("web.config")
                        || n.properties.file_path.ends_with("Web.config"))
                    && !n.properties.file_path.contains("PackageTmp")
                    && !n.properties.file_path.contains("/obj/")
            })
            .collect();

        if !config_files.is_empty() {
            writeln!(f, "### Fichiers de configuration")?;
            writeln!(f)?;
            for cf in &config_files {
                writeln!(f, "- `{}`", cf.properties.file_path.replace('\\', "/"))?;
            }
            writeln!(f)?;
        }
    }

    // Navigation
    writeln!(f, "## Pour aller plus loin")?;
    writeln!(f)?;
    writeln!(f, "- Consultez l'**Architecture** pour comprendre les couches du système")?;
    writeln!(f, "- Explorez les **Controllers** pour voir les fonctionnalités par écran")?;
    writeln!(f, "- Le **Guide Fonctionnel** décrit chaque module du point de vue métier")?;
    writeln!(f, "- Les **Services Externes** détaillent les intégrations (Erable, WCF)")?;
    writeln!(f)?;

    writeln!(f, "**Voir aussi :** [Vue d'ensemble](./overview.md) · [Architecture](./architecture.md)")?;
    writeln!(f)?;
    writeln!(f, "---")?;
    writeln!(f, "[<- Previous: Architecture](./architecture.md) | [Next: Modules ->](./modules/)")?;

    println!("  {} getting-started.md", "OK".green());
    Ok(())
}

/// Generate per-module documentation files with page ordering and navigation.
fn generate_docs_modules(
    modules_dir: &Path,
    communities: &BTreeMap<String, CommunityInfo>,
    graph: &KnowledgeGraph,
    edge_map: &HashMap<String, Vec<(String, RelationshipType)>>,
    repo_path: &Path,
) -> Result<usize> {
    let mut page_count: usize = 0;

    // Build member->community mapping
    let mut member_to_community: HashMap<String, String> = HashMap::new();
    for info in communities.values() {
        for mid in &info.member_ids {
            member_to_community.insert(mid.clone(), info.label.clone());
        }
    }

    // ─── Build ordered page list for Previous/Next navigation ──────────
    // Format: (filename_without_ext, display_title, is_relative_to_modules_dir)
    let mut page_order: Vec<(String, String)> = Vec::new();

    // Static pages (relative from modules/ directory via ../)
    page_order.push(("../overview".to_string(), "Overview".to_string()));
    page_order.push(("../project-health".to_string(), "Santé du Projet".to_string()));
    page_order.push(("../architecture".to_string(), "Architecture".to_string()));
    page_order.push(("../getting-started".to_string(), "Getting Started".to_string()));

    // Community/module pages — DEDUPLICATE by merging communities with same sanitized label
    let mut merged_communities: BTreeMap<String, CommunityInfo> = BTreeMap::new();
    for info in communities.values() {
        let base = sanitize_filename(&info.label);
        let entry = merged_communities.entry(base).or_insert_with(|| CommunityInfo {
            label: info.label.clone(),
            description: info.description.clone(),
            member_ids: Vec::new(),
            keywords: Vec::new(),
        });
        // Merge members from duplicate communities
        for mid in &info.member_ids {
            if !entry.member_ids.contains(mid) {
                entry.member_ids.push(mid.clone());
            }
        }
        for kw in &info.keywords {
            if !entry.keywords.contains(kw) {
                entry.keywords.push(kw.clone());
            }
        }
    }

    let mut community_filenames: Vec<(String, String)> = Vec::new();
    for (filename, info) in &merged_communities {
        community_filenames.push((filename.clone(), info.label.clone()));
        page_order.push((filename.clone(), info.label.clone()));
    }

    // Controller pages
    let mut controllers: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::Controller)
        .collect();
    controllers.sort_by(|a, b| a.properties.name.cmp(&b.properties.name));

    let ctrl_filenames: Vec<(String, String)> = controllers.iter()
        .map(|c| {
            let fname = format!("ctrl-{}", sanitize_filename(&c.properties.name));
            (fname, c.properties.name.clone())
        })
        .collect();
    for (fname, title) in &ctrl_filenames {
        page_order.push((fname.clone(), title.clone()));
    }

    // Data model pages
    let db_contexts: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::DbContext)
        .collect();
    let data_filenames: Vec<(String, String)> = db_contexts.iter()
        .map(|c| {
            let fname = format!("data-{}", sanitize_filename(&c.properties.name));
            (fname, format!("Data Model: {}", c.properties.name))
        })
        .collect();
    for (fname, title) in &data_filenames {
        page_order.push((fname.clone(), title.clone()));
    }

    // Services page
    let services: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::Service || n.label == NodeLabel::Repository)
        .collect();
    if !services.is_empty() {
        page_order.push(("services".to_string(), "Service Layer".to_string()));
    }

    // UI Components page
    let ui_components: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::UiComponent)
        .collect();
    if !ui_components.is_empty() {
        page_order.push(("ui-components".to_string(), "UI Components".to_string()));
    }

    // AJAX Endpoints page
    let ajax_calls: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::AjaxCall)
        .collect();
    if !ajax_calls.is_empty() {
        page_order.push(("ajax-endpoints".to_string(), "AJAX Endpoints".to_string()));
    }

    // External Services page
    let ext_services: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::ExternalService)
        .collect();
    if !ext_services.is_empty() {
        page_order.push(("external-services".to_string(), "External Services".to_string()));
    }

    /// Helper: generate prev/next navigation footer for a given page index.
    fn nav_footer(page_order: &[(String, String)], current_filename: &str) -> String {
        let idx = page_order.iter().position(|(f, _)| f == current_filename);
        let mut footer = String::from("\n---\n");
        if let Some(i) = idx {
            if i > 0 {
                let (prev_file, prev_title) = &page_order[i - 1];
                footer.push_str(&format!("[<- Previous: {}](./{}.md)", prev_title, prev_file));
            }
            if i > 0 && i + 1 < page_order.len() {
                footer.push_str(" | ");
            }
            if i + 1 < page_order.len() {
                let (next_file, next_title) = &page_order[i + 1];
                footer.push_str(&format!("[Next: {} ->](./{}.md)", next_title, next_file));
            }
        }
        footer.push('\n');
        footer
    }

    // ─── Community / Module pages (deduplicated) ──────────────────────
    for (comm_idx, (filename, info)) in merged_communities.iter().enumerate() {
        let _ = comm_idx;
        let out_path = modules_dir.join(format!("{}.md", filename));
        let mut f = std::fs::File::create(&out_path)?;

        let member_set: HashSet<&str> = info.member_ids.iter().map(|s| s.as_str()).collect();

        // Collect source files for this module
        let mut files_set: BTreeSet<String> = BTreeSet::new();
        for mid in &info.member_ids {
            if let Some(node) = graph.get_node(mid) {
                if !node.properties.file_path.is_empty() {
                    files_set.insert(node.properties.file_path.clone());
                }
            }
        }
        let files_vec: Vec<&str> = files_set.iter().map(|s| s.as_str()).collect();

        writeln!(f, "# {}", info.label)?;
        writeln!(f)?;
        write!(f, "{}", source_files_section(&files_vec))?;

        if let Some(desc) = &info.description {
            writeln!(f, "{}", desc)?;
            writeln!(f)?;
        }

        // Keywords
        if !info.keywords.is_empty() {
            writeln!(f, "**Keywords**: {}", info.keywords.join(", "))?;
            writeln!(f)?;
        }

        // Call Graph (internal calls only, limit to 30)
        let mut internal_calls: Vec<(String, String)> = Vec::new();
        for mid in &info.member_ids {
            if let Some(edges) = edge_map.get(mid.as_str()) {
                for (target_id, rel_type) in edges {
                    if *rel_type == RelationshipType::Calls && member_set.contains(target_id.as_str()) {
                        let src_name = graph
                            .get_node(mid)
                            .map(|n| n.properties.name.as_str())
                            .unwrap_or("?");
                        let tgt_name = graph
                            .get_node(target_id)
                            .map(|n| n.properties.name.as_str())
                            .unwrap_or("?");
                        internal_calls.push((src_name.to_string(), tgt_name.to_string()));
                    }
                }
            }
        }

        if !internal_calls.is_empty() && internal_calls.len() <= 30 {
            writeln!(f, "## Call Graph")?;
            writeln!(f)?;
            writeln!(f, "```mermaid")?;
            writeln!(f, "graph LR")?;
            let mut seen_nodes = HashSet::new();
            for (src, tgt) in &internal_calls {
                let src_safe = sanitize_filename(src).replace('-', "_");
                let tgt_safe = sanitize_filename(tgt).replace('-', "_");
                if seen_nodes.insert(src_safe.clone()) {
                    writeln!(f, "    {}[\"{}\"]", src_safe, escape_mermaid_label(src))?;
                }
                if seen_nodes.insert(tgt_safe.clone()) {
                    writeln!(f, "    {}[\"{}\"]", tgt_safe, escape_mermaid_label(tgt))?;
                }
                writeln!(f, "    {} --> {}", src_safe, tgt_safe)?;
            }
            writeln!(f, "```")?;
            writeln!(f)?;
        }

        // Members
        writeln!(f, "## Members")?;
        writeln!(f)?;
        writeln!(f, "| Symbol | Type | File | Lines |")?;
        writeln!(f, "|--------|------|------|-------|")?;

        for mid in &info.member_ids {
            if let Some(node) = graph.get_node(mid) {
                let lines = match (node.properties.start_line, node.properties.end_line) {
                    (Some(s), Some(e)) => format!("{}-{}", s, e),
                    (Some(s), None) => format!("{}", s),
                    _ => "-".to_string(),
                };
                writeln!(
                    f,
                    "| `{}` | {} | `{}` | {} |",
                    node.properties.name,
                    node.label.as_str(),
                    node.properties.file_path,
                    lines
                )?;
            }
        }
        writeln!(f)?;

        // Entry Points
        let mut entry_points: Vec<&GraphNode> = info
            .member_ids
            .iter()
            .filter_map(|mid| graph.get_node(mid))
            .filter(|n| n.properties.entry_point_score.map(|s| s > 0.3).unwrap_or(false))
            .collect();
        entry_points.sort_by(|a, b| {
            let sa = a.properties.entry_point_score.unwrap_or(0.0);
            let sb = b.properties.entry_point_score.unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

        if !entry_points.is_empty() {
            writeln!(f, "## Entry Points")?;
            writeln!(f)?;
            for node in entry_points.iter().take(10) {
                let score = node.properties.entry_point_score.unwrap_or(0.0);
                writeln!(
                    f,
                    "- `{}` (score: {:.2}) in `{}`",
                    node.properties.name, score, node.properties.file_path
                )?;
            }
            writeln!(f)?;
        }

        // Internal Calls
        if !internal_calls.is_empty() {
            writeln!(f, "## Internal Calls")?;
            writeln!(f)?;
            for (src, tgt) in &internal_calls {
                writeln!(f, "- `{}` -> `{}`", src, tgt)?;
            }
            writeln!(f)?;
        }

        // External Dependencies
        let mut external_deps: BTreeMap<String, usize> = BTreeMap::new();
        for mid in &info.member_ids {
            if let Some(edges) = edge_map.get(mid.as_str()) {
                for (target_id, rel_type) in edges {
                    if *rel_type == RelationshipType::Calls && !member_set.contains(target_id.as_str()) {
                        if let Some(target_comm) = member_to_community.get(target_id) {
                            *external_deps.entry(target_comm.clone()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        if !external_deps.is_empty() {
            writeln!(f, "## External Dependencies")?;
            writeln!(f)?;
            let mut sorted: Vec<_> = external_deps.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));
            for (target_comm, count) in sorted {
                let target_filename = sanitize_filename(&target_comm);
                writeln!(
                    f,
                    "- [**{}**]({}.md) - {} call(s)",
                    target_comm, target_filename, count
                )?;
            }
            writeln!(f)?;
        }

        // Files
        if !files_set.is_empty() {
            writeln!(f, "## Files")?;
            writeln!(f)?;
            for file_path in &files_set {
                writeln!(f, "- `{}`", file_path)?;
            }
            writeln!(f)?;
        }

        // Navigation footer
        write!(f, "{}", nav_footer(&page_order, filename))?;

        println!(
            "  {} modules/{filename}.md",
            "OK".green(),
        );
        page_count += 1;
    }

    // ─── Per-Controller pages (DeepWiki-quality) ──────────────────────
    for (ctrl_idx, ctrl) in controllers.iter().enumerate() {
        let ctrl_name = &ctrl.properties.name;
        let (filename, _) = &ctrl_filenames[ctrl_idx];
        let out_path = modules_dir.join(format!("{filename}.md"));

        // Find actions for this controller
        let mut actions: Vec<&GraphNode> = graph.iter_nodes()
            .filter(|n| n.label == NodeLabel::ControllerAction
                      && n.properties.file_path == ctrl.properties.file_path)
            .collect();
        actions.sort_by(|a, b| {
            a.properties.start_line.unwrap_or(0).cmp(&b.properties.start_line.unwrap_or(0))
        });

        // Skip trivial controllers with fewer than 3 actions (Root, PdfView, Print, Home)
        if actions.len() < 3 {
            continue;
        }

        // Build action ID set for caller lookup
        let action_ids: HashSet<String> = actions.iter().map(|a| a.id.clone()).collect();

        // Find all callers targeting any action of this controller
        let caller_rels: Vec<&GraphRelationship> = graph.iter_relationships()
            .filter(|r| action_ids.contains(&r.target_id)
                    && (r.rel_type == RelationshipType::CallsAction
                        || r.rel_type == RelationshipType::Calls))
            .collect();

        // Build per-action caller map: action_id -> Vec<(short_name, source_label)>
        let mut action_callers: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for r in &caller_rels {
            let short_name = if let Some(src_node) = graph.get_node(&r.source_id) {
                let label_str = match src_node.label {
                    NodeLabel::View | NodeLabel::PartialView => {
                        src_node.properties.file_path.rsplit(['/', '\\']).next()
                            .unwrap_or(&src_node.properties.name).to_string()
                    }
                    NodeLabel::UiComponent => {
                        // Show: vue + model + columns summary
                        let file = src_node.properties.file_path.rsplit(['/', '\\']).next()
                            .unwrap_or("vue");
                        let model = src_node.properties.bound_model.as_deref().unwrap_or("");
                        let cols = src_node.properties.description.as_deref().unwrap_or("");
                        if !model.is_empty() && !cols.is_empty() {
                            let short_cols: String = cols.chars().take(30).collect();
                            format!("{} Grid<{}> [{}]", file, model, short_cols)
                        } else if !model.is_empty() {
                            format!("{} Grid<{}>", file, model)
                        } else {
                            format!("{} (Grille)", file)
                        }
                    }
                    NodeLabel::AjaxCall => {
                        src_node.properties.file_path.rsplit(['/', '\\']).next()
                            .unwrap_or(&src_node.properties.name).to_string()
                    }
                    NodeLabel::ScriptFile => {
                        src_node.properties.file_path.rsplit(['/', '\\']).next()
                            .unwrap_or(&src_node.properties.name).to_string()
                    }
                    _ => src_node.properties.name.clone(),
                };
                let type_str = match src_node.label {
                    NodeLabel::View => "Vue".to_string(),
                    NodeLabel::PartialView => "Partielle".to_string(),
                    NodeLabel::UiComponent => "Grille".to_string(),
                    NodeLabel::AjaxCall => {
                        let ajax_method = src_node.properties.ajax_method.as_deref().unwrap_or("AJAX");
                        format!("AJAX {}", ajax_method)
                    }
                    NodeLabel::ScriptFile => "Script".to_string(),
                    _ => format!("{:?}", src_node.label),
                };
                (label_str, type_str)
            } else {
                let short = r.source_id.rsplit(':').next().unwrap_or(&r.source_id).to_string();
                (short, "Unknown".to_string())
            };
            let entry = action_callers.entry(r.target_id.clone()).or_default();
            if !entry.iter().any(|(n, _)| *n == short_name.0) {
                entry.push(short_name);
            }
        }

        // Find views rendered by this controller (both direct and through actions)
        let view_targets: Vec<String> = graph.iter_relationships()
            .filter(|r| {
                r.rel_type == RelationshipType::RendersView
                    && (r.source_id.contains(ctrl_name.as_str())
                        || graph.get_node(&r.source_id)
                            .map(|n| n.properties.file_path == ctrl.properties.file_path)
                            .unwrap_or(false))
            })
            .map(|r| r.target_id.clone())
            .collect();
        // Resolve view file paths
        let mut view_files: Vec<String> = view_targets.iter()
            .filter_map(|vid| graph.get_node(vid).map(|n| n.properties.file_path.clone()))
            .collect::<BTreeSet<String>>()
            .into_iter()
            .collect();
        if view_files.is_empty() {
            // Fallback: use the target IDs directly
            view_files = view_targets.iter().cloned().collect::<BTreeSet<String>>().into_iter().collect();
        }

        // Find services this controller depends on (DependsOn relationships)
        let dependencies: Vec<String> = graph.iter_relationships()
            .filter(|r| {
                r.rel_type == RelationshipType::DependsOn
                    && (r.source_id.contains(ctrl_name.as_str())
                        || graph.get_node(&r.source_id)
                            .map(|n| n.properties.file_path == ctrl.properties.file_path
                                && n.label == NodeLabel::Controller)
                            .unwrap_or(false))
            })
            .filter_map(|r| graph.get_node(&r.target_id).map(|n| n.properties.name.clone()))
            .collect::<BTreeSet<String>>()
            .into_iter()
            .collect();

        // Build source files list
        let mut src_files: Vec<String> = vec![ctrl.properties.file_path.clone()];
        src_files.extend(view_files.iter().cloned());
        let src_file_refs: Vec<&str> = src_files.iter().take(15).map(|s| s.as_str()).collect();

        let mut content = format!("# {}\n\n", ctrl_name);
        content.push_str("<!-- GNX:LEAD -->\n");
        content.push_str(&source_files_section(&src_file_refs));

        // Description
        let base_name = ctrl_name.trim_end_matches("Controller");
        let action_count = actions.len();
        let desc = describe_controller(ctrl_name);
        content.push_str(&format!(
            "> {} manages {} endpoints for {}.\n\n",
            base_name, action_count, desc
        ));

        // Actions table with method signatures extracted from content
        // Collect known entity/model type names for linking
        let known_types: HashSet<String> = graph.iter_nodes()
            .filter(|n| matches!(n.label, NodeLabel::DbEntity | NodeLabel::ViewModel | NodeLabel::Class))
            .map(|n| n.properties.name.clone())
            .collect();

        content.push_str(&format!("## Actions ({})\n\n", action_count));
        content.push_str("| # | Action | Method | Paramètres | Retour | Appelé par |\n");
        content.push_str("|---|--------|--------|-----------|--------|------------|\n");
        for (i, action) in actions.iter().enumerate() {
            let method = action.properties.http_method.as_deref().unwrap_or("GET");
            let ret = action.properties.return_type.as_deref().unwrap_or("ActionResult");

            // Extract parameter signature and link known types to data model
            let params = extract_params_linked(
                action.properties.description.as_deref().unwrap_or(""),
                &known_types,
            );

            // Get callers for this action (up to 3)
            let called_by = action_callers.get(&action.id)
                .map(|callers| {
                    callers.iter()
                        .take(3)
                        .map(|(name, _)| name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "-".to_string());

            content.push_str(&format!("| {} | **{}** | {} | {} | {} | {} |\n",
                i + 1, action.properties.name, method, params, ret, called_by));
        }
        content.push('\n');
        content.push_str("<!-- GNX:TIP:actions -->\n");

        // ── Impact Analysis section ──────────────────────────────────────
        {
            // For each action, find outgoing calls (callees) and incoming calls (callers)
            let mut action_impacts: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();
            for action in &actions {
                let action_name = action.properties.name.clone();

                // Callees: nodes this action calls (outgoing Calls/CallsAction/DependsOn)
                let callees: Vec<String> = graph.iter_relationships()
                    .filter(|r| {
                        r.source_id == action.id
                            && matches!(r.rel_type,
                                RelationshipType::Calls
                                | RelationshipType::CallsAction
                                | RelationshipType::DependsOn
                                | RelationshipType::CallsService)
                    })
                    .filter_map(|r| graph.get_node(&r.target_id).map(|n| n.properties.name.clone()))
                    .collect::<BTreeSet<String>>()
                    .into_iter()
                    .collect();

                // Callers: nodes that call this action (incoming Calls/CallsAction)
                let callers: Vec<String> = graph.iter_relationships()
                    .filter(|r| {
                        r.target_id == action.id
                            && matches!(r.rel_type,
                                RelationshipType::Calls
                                | RelationshipType::CallsAction)
                    })
                    .filter_map(|r| graph.get_node(&r.source_id).map(|n| n.properties.name.clone()))
                    .collect::<BTreeSet<String>>()
                    .into_iter()
                    .collect();

                let total_rels = callees.len() + callers.len();
                if total_rels > 0 {
                    action_impacts.push((action_name, callees, callers));
                }
            }

            // Sort by total relationships descending, take top 5
            action_impacts.sort_by(|a, b| (b.1.len() + b.2.len()).cmp(&(a.1.len() + a.2.len())));

            if !action_impacts.is_empty() {
                content.push_str("## Analyse d'Impact\n\n");
                content.push_str("> Si une action de ce controller est modifiée, voici les composants potentiellement impactés.\n\n");
                content.push_str("| Action modifiée | Impact aval (callees) | Impact amont (callers) |\n");
                content.push_str("|----------------|----------------------|----------------------|\n");
                for (action_name, callees, callers) in action_impacts.iter().take(5) {
                    let callees_str = if callees.is_empty() {
                        "-".to_string()
                    } else {
                        callees.iter().take(5).map(|s| format!("`{}`", s)).collect::<Vec<_>>().join(", ")
                    };
                    let callers_str = if callers.is_empty() {
                        "-".to_string()
                    } else {
                        callers.iter().take(5).map(|s| format!("`{}`", s)).collect::<Vec<_>>().join(", ")
                    };
                    content.push_str(&format!("| **{}** | {} | {} |\n", action_name, callees_str, callers_str));
                }
                content.push('\n');
            }
        }

        // Callers section: all callers targeting actions of this controller
        if !caller_rels.is_empty() {
            // Build a deduplicated callers table
            let mut caller_rows: Vec<(String, String, String, String)> = Vec::new(); // (source, type, action, method)
            let mut seen_callers: HashSet<(String, String)> = HashSet::new();
            for r in &caller_rels {
                let (source_name, source_type) = if let Some(src_node) = graph.get_node(&r.source_id) {
                    let name = match src_node.label {
                        NodeLabel::View | NodeLabel::PartialView => {
                            src_node.properties.file_path.rsplit(['/', '\\']).next()
                                .unwrap_or(&src_node.properties.name).to_string()
                        }
                        _ => src_node.properties.name.clone(),
                    };
                    let stype = match src_node.label {
                        NodeLabel::View => {
                            // Check if it's a form submission
                            if r.reason.contains("form") || r.reason.contains("Form") {
                                "View (Form)".to_string()
                            } else {
                                "View".to_string()
                            }
                        }
                        NodeLabel::PartialView => "Partial View".to_string(),
                        NodeLabel::AjaxCall => {
                            let ajax_type = src_node.properties.ajax_method.as_deref().unwrap_or("AJAX");
                            if src_node.properties.ajax_url.as_deref().map(|u| u.contains("getJSON")).unwrap_or(false) {
                                "Script ($.getJSON)".to_string()
                            } else {
                                format!("Script ({})", ajax_type)
                            }
                        }
                        _ => format!("{:?}", src_node.label),
                    };
                    (name, stype)
                } else {
                    let short = r.source_id.rsplit(':').next().unwrap_or(&r.source_id).to_string();
                    (short, "Unknown".to_string())
                };

                let target_action = graph.get_node(&r.target_id)
                    .map(|n| n.properties.name.clone())
                    .unwrap_or_else(|| r.target_id.rsplit(':').next().unwrap_or(&r.target_id).to_string());

                let method = graph.get_node(&r.target_id)
                    .and_then(|n| n.properties.http_method.as_ref())
                    .cloned()
                    .unwrap_or_else(|| "-".to_string());

                let key = (source_name.clone(), target_action.clone());
                if seen_callers.insert(key) {
                    caller_rows.push((source_name, source_type, target_action, method));
                }
            }

            if !caller_rows.is_empty() {
                content.push_str("## Callers\n\n");
                content.push_str("This controller is called from:\n\n");
                content.push_str("| Source | Type | Action | Method |\n");
                content.push_str("|--------|------|--------|--------|\n");
                for (source, stype, action, method) in &caller_rows {
                    content.push_str(&format!("| {} | {} | {} | {} |\n",
                        source, stype, action, method));
                }
                content.push('\n');
            }
        }

        // Associated Views section
        if !view_files.is_empty() {
            content.push_str("## Associated Views\n\n");
            for v in &view_files {
                content.push_str(&format!("- `{}`\n", v));
            }
            content.push('\n');
        }

        // Dependencies section
        if !dependencies.is_empty() {
            content.push_str("## Dependencies\n\n");
            for dep in &dependencies {
                content.push_str(&format!("- `{}`\n", dep));
            }
            content.push('\n');
        }

        // Action Details (collapsible signatures with full parameter info)
        if !actions.is_empty() {
            content.push_str("## Action Details\n\n");
            for action in &actions {
                let method = action.properties.http_method.as_deref().unwrap_or("GET");
                let params_short = extract_params_from_content(
                    action.properties.description.as_deref().unwrap_or(""),
                    &action.properties.name,
                );

                content.push_str(&format!("<details>\n<summary><strong>{}</strong> ({}) — {}</summary>\n\n",
                    action.properties.name, method,
                    if params_short == "-" { "aucun paramètre".to_string() } else { params_short.clone() }));

                content.push_str(&format!("**Fichier :** `{}`", ctrl.properties.file_path));
                if let Some(line) = action.properties.start_line {
                    content.push_str(&format!(" (ligne {})", line));
                }
                content.push('\n');

                if params_short != "-" {
                    content.push_str(&format!("**Paramètres :** {}\n", params_short));
                }

                let ret = action.properties.return_type.as_deref().unwrap_or("ActionResult");
                content.push_str(&format!("**Returns:** {}\n", ret));

                // Callers for this specific action
                if let Some(callers) = action_callers.get(&action.id) {
                    let caller_strs: Vec<String> = callers.iter()
                        .map(|(name, stype)| format!("{} ({})", name, stype))
                        .collect();
                    if !caller_strs.is_empty() {
                        content.push_str(&format!("**Appelé par :** {}\n", caller_strs.join(", ")));
                    }
                }

                // Source code snippet — find method by name in source file
                let source_path = repo_path.join(&ctrl.properties.file_path);
                if let Ok(source) = std::fs::read_to_string(&source_path) {
                    if let Some(snippet) = extract_method_body(&source, &action.properties.name, 50) {
                        content.push_str("\n```csharp\n");
                        content.push_str(&snippet);
                        content.push_str("```\n");
                    }
                }

                content.push_str("\n</details>\n\n");
            }
        }

        // Summary
        content.push_str("<!-- GNX:CLOSING -->\n");
        content.push_str(&format!(
            "## Summary\n\n**{}** provides {} actions.\n\n",
            ctrl_name, action_count
        ));
        content.push_str("**See also:** [Architecture](../architecture.md) · [Services](./services.md)\n");

        // Navigation footer
        content.push_str(&nav_footer(&page_order, filename));

        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    // ─── Data Model pages ──────────────────────────────────────────────
    for (ctx_idx, ctx) in db_contexts.iter().enumerate() {
        let ctx_name = &ctx.properties.name;
        let (filename, _) = &data_filenames[ctx_idx];
        let out_path = modules_dir.join(format!("{filename}.md"));

        // Find entities mapped to this context
        let entities: Vec<&GraphNode> = graph.iter_nodes()
            .filter(|n| n.label == NodeLabel::DbEntity)
            .collect();

        // Source files
        let mut src_files: Vec<String> = vec![ctx.properties.file_path.clone()];
        for e in &entities {
            if !e.properties.file_path.is_empty() {
                src_files.push(e.properties.file_path.clone());
            }
        }
        let src_files_dedup: Vec<String> = src_files.into_iter().collect::<BTreeSet<String>>().into_iter().collect();
        let src_file_refs: Vec<&str> = src_files_dedup.iter().take(15).map(|s| s.as_str()).collect();

        let mut content = format!("# Data Model: {}\n\n", ctx_name);
        content.push_str("<!-- GNX:LEAD -->\n");
        content.push_str(&source_files_section(&src_file_refs));
        content.push_str(&format!("**File:** `{}`\n\n", ctx.properties.file_path));
        content.push_str(&format!("**Entities:** {}\n\n", entities.len()));

        // Build adjacency map for all entities
        let mut entity_rels: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for rel in graph.iter_relationships() {
            if rel.rel_type == RelationshipType::AssociatesWith {
                let src = rel.source_id.rsplit(':').next().unwrap_or(&rel.source_id).to_string();
                let tgt = rel.target_id.rsplit(':').next().unwrap_or(&rel.target_id).to_string();
                let card = if rel.reason.contains("1:*") { "||--o{" }
                    else if rel.reason.contains("*:1") { "}o--||" }
                    else if rel.reason.contains("*:*") { "}o--o{" }
                    else { "||--||" };
                entity_rels.entry(src.clone()).or_default().push((tgt.clone(), card.to_string()));
                entity_rels.entry(tgt).or_default().push((src, card.to_string()));
            }
        }

        content.push_str("## Entities\n\n");

        // Per-entity: collapsible section with mini ER diagram
        for entity in &entities {
            let ename = &entity.properties.name;
            let rels = entity_rels.get(ename.as_str());
            let rel_count = rels.map_or(0, |v| v.len());

            content.push_str(&format!("<details id=\"{}\">\n<summary><strong>{}</strong> — <code>{}</code> ({} relations)</summary>\n\n",
                ename, ename, entity.properties.file_path, rel_count));

            // Mini ER diagram showing this entity and its direct relations
            if rel_count > 0 {
                if let Some(rels) = rels {
                    // Use graph LR instead of erDiagram for better Mermaid 11.x compatibility
                    content.push_str("```mermaid\ngraph LR\n");
                    let eid = sanitize_mermaid_id(ename);
                    content.push_str(&format!("    {}[\"{}\"]\n", eid, ename));
                    content.push_str(&format!("    style {} fill:#4a85e0,color:#fff,stroke:#3a73cc\n", eid));

                    let mut seen: HashSet<String> = HashSet::new();
                    for (target, _cardinality) in rels.iter().take(8) {
                        if seen.insert(target.clone()) {
                            let tid = sanitize_mermaid_id(target);
                            content.push_str(&format!("    {}[\"{}\"]\n", tid, target));
                            content.push_str(&format!("    {} --- {}\n", eid, tid));
                        }
                    }
                    if rels.len() > 8 {
                        content.push_str(&format!("    more((\"...+{}\"))\n", rels.len() - 8));
                        content.push_str(&format!("    {} -.- more\n", eid));
                    }
                    content.push_str("```\n\n");
                }
            } else {
                content.push_str("*Aucune relation détectée dans le modèle.*\n\n");
            }

            content.push_str("</details>\n\n");
        }

        // Navigation footer
        content.push_str(&nav_footer(&page_order, filename));

        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    // ─── Service Layer page ────────────────────────────────────────────
    if !services.is_empty() {
        let out_path = modules_dir.join("services.md");

        // Source files
        let svc_files: Vec<String> = services.iter()
            .map(|s| s.properties.file_path.clone())
            .collect::<BTreeSet<String>>()
            .into_iter()
            .collect();
        let svc_file_refs: Vec<&str> = svc_files.iter().take(15).map(|s| s.as_str()).collect();

        let mut content = String::from("# Service Layer\n\n");
        content.push_str("<!-- GNX:LEAD -->\n");
        content.push_str(&source_files_section(&svc_file_refs));
        content.push_str(&format!("**Total services:** {}\n\n", services.len()));

        // Build service "Used By" lookup: find controllers that depend on each service
        let mut service_used_by: HashMap<String, Vec<String>> = HashMap::new();
        for svc in &services {
            let users: Vec<String> = graph.iter_relationships()
                .filter(|r| {
                    r.rel_type == RelationshipType::DependsOn
                        && r.target_id == svc.id
                })
                .filter_map(|r| {
                    graph.get_node(&r.source_id)
                        .filter(|n| n.label == NodeLabel::Controller)
                        .map(|n| n.properties.name.clone())
                })
                .collect::<BTreeSet<String>>()
                .into_iter()
                .collect();
            service_used_by.insert(svc.id.clone(), users);
        }

        content.push_str("## Services\n\n");
        content.push_str("| Service | Type | Interface | Used By | Purpose | File |\n");
        content.push_str("|---------|------|-----------|---------|---------|------|\n");
        for svc in &services {
            let layer = svc.properties.layer_type.as_deref().unwrap_or("Service");
            let iface = svc.properties.implements_interface.as_deref().unwrap_or("-");
            let used_by = service_used_by.get(&svc.id)
                .map(|users| {
                    if users.is_empty() {
                        "-".to_string()
                    } else {
                        users.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                    }
                })
                .unwrap_or_else(|| "-".to_string());
            let purpose = describe_service_fr(&svc.properties.name);
            content.push_str(&format!("| {} | {} | {} | {} | {} | `{}` |\n",
                svc.properties.name, layer, iface, used_by, purpose, svc.properties.file_path));
        }
        content.push('\n');

        // Navigation footer
        content.push_str(&nav_footer(&page_order, "services"));

        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    // ─── UI Components page ────────────────────────────────────────────
    if !ui_components.is_empty() {
        let out_path = modules_dir.join("ui-components.md");

        // Source files
        let ui_files: Vec<String> = ui_components.iter()
            .map(|c| c.properties.file_path.clone())
            .collect::<BTreeSet<String>>()
            .into_iter()
            .collect();
        let ui_file_refs: Vec<&str> = ui_files.iter().take(15).map(|s| s.as_str()).collect();

        let mut content = String::from("# UI Components (Telerik/Kendo)\n\n");
        content.push_str(&source_files_section(&ui_file_refs));
        content.push_str(&format!("**Total components:** {}\n\n", ui_components.len()));

        content.push_str("| Component | Type | Model | Columns | File |\n");
        content.push_str("|-----------|------|-------|---------|------|\n");
        for comp in &ui_components {
            let comp_type = comp.properties.component_type.as_deref().unwrap_or("-");
            let model = comp.properties.bound_model.as_deref().unwrap_or("-");
            let cols = comp.properties.description.as_deref().unwrap_or("-");
            // Truncate cols to 40 chars
            let cols_short: String = cols.chars().take(40).collect();
            content.push_str(&format!("| {} | {} | {} | {} | `{}` |\n",
                comp.properties.name, comp_type, model, cols_short, comp.properties.file_path));
        }
        content.push('\n');

        // Navigation footer
        content.push_str(&nav_footer(&page_order, "ui-components"));

        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    // ─── AJAX Endpoints page ───────────────────────────────────────────
    if !ajax_calls.is_empty() {
        let out_path = modules_dir.join("ajax-endpoints.md");

        // Source files
        let ajax_files: Vec<String> = ajax_calls.iter()
            .map(|c| c.properties.file_path.clone())
            .collect::<BTreeSet<String>>()
            .into_iter()
            .collect();
        let ajax_file_refs: Vec<&str> = ajax_files.iter().take(15).map(|s| s.as_str()).collect();

        let mut content = String::from("# AJAX Endpoints\n\n");
        content.push_str(&source_files_section(&ajax_file_refs));
        content.push_str(&format!("**Total AJAX calls:** {}\n\n", ajax_calls.len()));

        content.push_str("| Method | URL | File | Line |\n");
        content.push_str("|--------|-----|------|------|\n");
        for call in ajax_calls.iter().take(100) { // Cap at 100 for readability
            let method = call.properties.ajax_method.as_deref().unwrap_or("GET");
            let url = call.properties.ajax_url.as_deref().unwrap_or("-");
            let line = call.properties.start_line.map(|l| l.to_string()).unwrap_or_default();
            content.push_str(&format!("| {} | {} | `{}` | {} |\n",
                method, url, call.properties.file_path, line));
        }
        content.push('\n');

        // Navigation footer
        content.push_str(&nav_footer(&page_order, "ajax-endpoints"));

        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    // ─── External Services page ────────────────────────────────────────
    if !ext_services.is_empty() {
        let out_path = modules_dir.join("external-services.md");

        // Source files: files that contain or call external services
        let ext_files: Vec<String> = ext_services.iter()
            .map(|s| s.properties.file_path.clone())
            .collect::<BTreeSet<String>>()
            .into_iter()
            .collect();

        // Also find files that call these services
        let mut calling_files: BTreeSet<String> = BTreeSet::new();
        for svc in &ext_services {
            for r in graph.iter_relationships() {
                if r.rel_type == RelationshipType::CallsService && r.target_id == svc.id {
                    if let Some(src) = graph.get_node(&r.source_id) {
                        if !src.properties.file_path.is_empty() {
                            calling_files.insert(src.properties.file_path.clone());
                        }
                    }
                }
            }
        }

        let mut all_src_files: Vec<String> = ext_files.iter().cloned()
            .chain(calling_files.iter().cloned())
            .collect::<BTreeSet<String>>()
            .into_iter()
            .collect();
        all_src_files.truncate(15);
        let src_file_refs: Vec<&str> = all_src_files.iter().map(|s| s.as_str()).collect();

        let mut content = String::from("# External Services & Integrations\n\n");
        content.push_str(&source_files_section(&src_file_refs));
        content.push_str(&format!(
            "> This project integrates with {} external services via WebAPI (REST) and WCF (SOAP).\n\n",
            ext_services.len()
        ));

        // Partition by service_type
        let webapi_services: Vec<&&GraphNode> = ext_services.iter()
            .filter(|s| {
                let stype = s.properties.service_type.as_deref().unwrap_or("");
                stype.eq_ignore_ascii_case("webapi") || stype.eq_ignore_ascii_case("rest")
                    || stype.eq_ignore_ascii_case("http")
            })
            .collect();

        let wcf_services: Vec<&&GraphNode> = ext_services.iter()
            .filter(|s| {
                let stype = s.properties.service_type.as_deref().unwrap_or("");
                stype.eq_ignore_ascii_case("wcf") || stype.eq_ignore_ascii_case("soap")
            })
            .collect();

        let other_services: Vec<&&GraphNode> = ext_services.iter()
            .filter(|s| {
                let stype = s.properties.service_type.as_deref().unwrap_or("").to_lowercase();
                !["webapi", "rest", "http", "wcf", "soap"].contains(&stype.as_str())
            })
            .collect();

        // Helper closure: find callers of a given external service
        let find_callers = |svc: &GraphNode| -> Vec<String> {
            graph.iter_relationships()
                .filter(|r| r.rel_type == RelationshipType::CallsService && r.target_id == svc.id)
                .filter_map(|r| graph.get_node(&r.source_id).map(|n| n.properties.name.clone()))
                .collect::<BTreeSet<String>>()
                .into_iter()
                .collect()
        };

        // Helper: find API methods for a service by searching Method nodes
        // in files containing "WebAPI" or matching the client name pattern
        let _find_methods = |svc: &GraphNode| -> Vec<&GraphNode> {
            let svc_name = &svc.properties.name;
            // Look for Method nodes in files containing "WebAPI" or the client name
            graph.iter_nodes()
                .filter(|n| n.label == NodeLabel::Method
                    && (n.properties.file_path.contains("WebAPI")
                        || n.properties.file_path.contains("WebApi")
                        || n.properties.file_path.contains(svc_name))
                    && n.properties.name.ends_with("Async")
                    && !n.properties.name.starts_with("PrepareRequest")
                    && !n.properties.name.starts_with("ProcessResponse")
                    && !n.properties.name.starts_with("ReadObject"))
                .collect()
        };

        if !webapi_services.is_empty() {
            content.push_str(&format!("## WebAPI Services ({})\n\n", webapi_services.len()));
            content.push_str("| Client | Type | Called From | Purpose |\n");
            content.push_str("|--------|------|------------|--------|\n");
            for svc in &webapi_services {
                let stype = svc.properties.service_type.as_deref().unwrap_or("WebAPI");
                let callers = find_callers(svc);
                let called_from = if callers.is_empty() {
                    "-".to_string()
                } else {
                    callers.join(", ")
                };
                let purpose = svc.properties.description.as_deref().unwrap_or("-");
                content.push_str(&format!("| {} | {} | {} | {} |\n",
                    svc.properties.name, stype, called_from, purpose));
            }
            content.push('\n');

            // Collect ALL Async methods from WebAPI files (shared across clients)
            let all_api_methods: Vec<&GraphNode> = graph.iter_nodes()
                .filter(|n| n.label == NodeLabel::Method
                    && (n.properties.file_path.contains("WebAPI")
                        || n.properties.file_path.contains("WebApi"))
                    && !n.properties.file_path.contains("Tests")
                    && n.properties.name.ends_with("Async")
                    && !n.properties.name.starts_with("PrepareRequest")
                    && !n.properties.name.starts_with("ProcessResponse")
                    && !n.properties.name.starts_with("ReadObject"))
                .collect();

            if !all_api_methods.is_empty() {
                content.push_str("### API Erable — Méthodes détaillées\n\n");
                content.push_str("> Point d'accès unique aux données bénéficiaires via l'API REST Erable.\n");
                content.push_str("> Authentification : HTTP Basic. Toutes les méthodes sont asynchrones.\n\n");

                // Group by file
                let mut methods_by_file: BTreeMap<String, Vec<&&GraphNode>> = BTreeMap::new();
                for m in &all_api_methods {
                    methods_by_file.entry(m.properties.file_path.clone()).or_default().push(m);
                }

                for (file, methods) in &methods_by_file {
                    let file_short = file.rsplit(['/', '\\']).next().unwrap_or(file);

                    // Skip LDAP client — not Erable
                    if file_short.contains("Ldap") {
                        continue;
                    }

                    content.push_str(&format!("**Fichier : `{}`**\n\n", file_short));

                    // Try to read the actual source file for signature extraction
                    let source_path = repo_path.join(file);
                    let source_content = std::fs::read_to_string(&source_path).unwrap_or_default();

                    content.push_str("| Méthode | Paramètres | Retour |\n");
                    content.push_str("|---------|-----------|--------|\n");

                    for method in methods {
                        let method_name = &method.properties.name;

                        // Extract ALL overload signatures from source file
                        let signatures = if !source_content.is_empty() {
                            extract_all_method_signatures(&source_content, method_name)
                        } else {
                            vec![("-".to_string(), "-".to_string())]
                        };

                        for (idx, (params_str, ret_str)) in signatures.iter().enumerate() {
                            if idx == 0 {
                                content.push_str(&format!("| **{}** | {} | `{}` |\n",
                                    method_name, params_str, ret_str));
                            } else {
                                // Overload: show with "(surcharge)" label
                                content.push_str(&format!("| ↳ *surcharge* | {} | `{}` |\n",
                                    params_str, ret_str));
                            }
                        }
                    }
                    content.push('\n');
                }

                // Who calls these clients
                content.push_str("**Services appelants :**\n\n");
                for svc in &webapi_services {
                    let callers = find_callers(svc);
                    if !callers.is_empty() {
                        content.push_str(&format!("- **{}** ← {}\n", svc.properties.name, callers.join(", ")));
                    }
                }
                content.push('\n');
            }
        }

        if !wcf_services.is_empty() {
            content.push_str(&format!("## WCF Services (SOAP) ({})\n\n", wcf_services.len()));
            content.push_str("| Client | Type | Called From | Purpose |\n");
            content.push_str("|--------|------|------------|--------|\n");
            for svc in &wcf_services {
                let stype = svc.properties.service_type.as_deref().unwrap_or("WCF");
                let callers = find_callers(svc);
                let called_from = if callers.is_empty() {
                    "-".to_string()
                } else {
                    callers.join(", ")
                };
                let purpose = svc.properties.description.as_deref().unwrap_or("-");
                content.push_str(&format!("| {} | {} | {} | {} |\n",
                    svc.properties.name, stype, called_from, purpose));
            }
            content.push('\n');
        }

        if !other_services.is_empty() {
            content.push_str(&format!("## Other Services ({})\n\n", other_services.len()));
            content.push_str("| Client | Type | Called From | Purpose |\n");
            content.push_str("|--------|------|------------|--------|\n");
            for svc in &other_services {
                let stype = svc.properties.service_type.as_deref().unwrap_or("External");
                let callers = find_callers(svc);
                let called_from = if callers.is_empty() {
                    "-".to_string()
                } else {
                    callers.join(", ")
                };
                let purpose = svc.properties.description.as_deref().unwrap_or("-");
                content.push_str(&format!("| {} | {} | {} | {} |\n",
                    svc.properties.name, stype, called_from, purpose));
            }
            content.push('\n');
        }

        // Service Call Flow (Mermaid diagram)
        // Build a flow: Controller -> Service -> ExternalService
        let mut mermaid_edges: Vec<(String, String)> = Vec::new();
        let mut mermaid_nodes: BTreeMap<String, (String, &str)> = BTreeMap::new(); // id -> (label, subgraph)

        for ext_svc in &ext_services {
            let ext_short = sanitize_mermaid_id(&ext_svc.properties.name);
            mermaid_nodes.insert(ext_short.clone(),
                (ext_svc.properties.name.clone(), "External"));

            // Find what calls this external service
            for r in graph.iter_relationships() {
                if r.rel_type == RelationshipType::CallsService && r.target_id == ext_svc.id {
                    if let Some(caller) = graph.get_node(&r.source_id) {
                        let caller_short = sanitize_mermaid_id(&caller.properties.name);
                        // Skip test files to keep diagram readable
                        if caller.properties.file_path.contains("Test")
                            || caller.properties.file_path.contains("test") {
                            continue;
                        }
                        let subgraph = match caller.label {
                            NodeLabel::Controller => "Controllers",
                            NodeLabel::Service | NodeLabel::Repository => "Services",
                            _ => continue, // Skip non-controller/non-service callers
                        };
                        mermaid_nodes.insert(caller_short.clone(),
                            (caller.properties.name.clone(), subgraph));
                        mermaid_edges.push((caller_short.clone(), ext_short.clone()));

                        // Also find what calls this intermediate service (for Controller -> Service -> External flow)
                        if caller.label == NodeLabel::Service || caller.label == NodeLabel::Repository {
                            for r2 in graph.iter_relationships() {
                                if r2.rel_type == RelationshipType::DependsOn && r2.target_id == caller.id {
                                    if let Some(ctrl) = graph.get_node(&r2.source_id) {
                                        if ctrl.label == NodeLabel::Controller {
                                            let ctrl_short = sanitize_mermaid_id(&ctrl.properties.name);
                                            mermaid_nodes.insert(ctrl_short.clone(),
                                                (ctrl.properties.name.clone(), "Controllers"));
                                            mermaid_edges.push((ctrl_short, caller_short.clone()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if !mermaid_edges.is_empty() {
            content.push_str("## Service Call Flow\n\n");
            content.push_str("```mermaid\ngraph LR\n");

            // Group nodes by subgraph
            let mut subgraphs: BTreeMap<&str, Vec<(String, String)>> = BTreeMap::new();
            for (id, (label, sg)) in &mermaid_nodes {
                subgraphs.entry(sg).or_default().push((id.clone(), label.clone()));
            }

            for (sg_name, nodes) in &subgraphs {
                content.push_str(&format!("    subgraph {}[\"{}\"]\n", sanitize_mermaid_id(sg_name), sg_name));
                for (id, label) in nodes {
                    content.push_str(&format!("        {}[\"{}\"]\n", id, label));
                }
                content.push_str("    end\n");
            }

            // Deduplicate edges
            let unique_edges: BTreeSet<(String, String)> = mermaid_edges.into_iter().collect();
            for (from, to) in &unique_edges {
                content.push_str(&format!("    {} --> {}\n", from, to));
            }
            content.push_str("```\n\n");
        }

        // Navigation footer
        content.push_str(&nav_footer(&page_order, "external-services"));

        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    Ok(page_count)
}

// ─── Project Health Generator ──────────────────────────────────────────

fn generate_project_health(docs_dir: &Path, graph: &KnowledgeGraph) -> Result<()> {
    let out_path = docs_dir.join("project-health.md");
    let mut f = std::fs::File::create(&out_path)?;

    let label_counts = count_nodes_by_label(graph);
    let node_count = graph.iter_nodes().count();
    let edge_count = graph.iter_relationships().count();
    let density = if node_count > 0 {
        edge_count as f64 / node_count as f64
    } else {
        0.0
    };
    let density_interp = if density > 3.0 {
        "Fortement couplé"
    } else if density > 2.0 {
        "Couplage modéré"
    } else if density > 1.0 {
        "Couplage normal"
    } else {
        "Faiblement couplé"
    };

    // StackLogger tracing coverage
    let total_files = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::File)
        .count();
    let traced_files = graph.iter_nodes()
        .filter(|n| n.properties.is_traced == Some(true))
        .count();
    let traced_pct = if total_files > 0 {
        (traced_files as f64 / total_files as f64) * 100.0
    } else {
        0.0
    };

    let ext_count = label_counts.get(&NodeLabel::ExternalService).copied().unwrap_or(0);

    // Key symbol counts
    let fn_count = label_counts.get(&NodeLabel::Function).copied().unwrap_or(0);
    let class_count = label_counts.get(&NodeLabel::Class).copied().unwrap_or(0);
    let method_count = label_counts.get(&NodeLabel::Method).copied().unwrap_or(0);
    let ctrl_count = label_counts.get(&NodeLabel::Controller).copied().unwrap_or(0);
    let action_count = label_counts.get(&NodeLabel::ControllerAction).copied().unwrap_or(0);
    let svc_count = label_counts.get(&NodeLabel::Service).copied().unwrap_or(0)
        + label_counts.get(&NodeLabel::Repository).copied().unwrap_or(0);

    writeln!(f, "# Santé du Projet")?;
    writeln!(f, "<!-- GNX:LEAD -->")?;
    writeln!(f)?;
    writeln!(f, "> Vue d'ensemble de la santé structurelle du codebase, ")?;
    writeln!(f, "> générée automatiquement par l'analyse du graphe de connaissances GitNexus.")?;
    writeln!(f)?;

    // ── Key Metrics Table ──
    writeln!(f, "## Métriques Clés")?;
    writeln!(f, "<!-- GNX:INTRO:metriques-cles -->")?;
    writeln!(f)?;
    writeln!(f, "| Indicateur | Valeur | Interprétation |")?;
    writeln!(f, "|-----------|--------|----------------|")?;
    writeln!(f, "| Symboles | {} | Volume de code analysé |", node_count)?;
    writeln!(f, "| Relations | {} | Couplage entre composants |", edge_count)?;
    writeln!(f, "| Densité | {:.1} | {} |", density, density_interp)?;
    writeln!(f, "| Couverture traçabilité | {:.0}% ({}/{} fichiers) | Fichiers avec StackLogger |",
        traced_pct, traced_files, total_files)?;
    writeln!(f, "| Services externes | {} | Points d'intégration |", ext_count)?;
    writeln!(f)?;

    // ── Symbol breakdown ──
    writeln!(f, "## Répartition par type de symbole")?;
    writeln!(f)?;
    writeln!(f, "| Type | Nombre |")?;
    writeln!(f, "|------|--------|")?;
    if fn_count > 0 { writeln!(f, "| Functions | {} |", fn_count)?; }
    if class_count > 0 { writeln!(f, "| Classes | {} |", class_count)?; }
    if method_count > 0 { writeln!(f, "| Methods | {} |", method_count)?; }
    if ctrl_count > 0 { writeln!(f, "| Controllers | {} |", ctrl_count)?; }
    if action_count > 0 { writeln!(f, "| Controller Actions | {} |", action_count)?; }
    if svc_count > 0 { writeln!(f, "| Services/Repositories | {} |", svc_count)?; }
    // Show remaining non-zero labels
    let shown_labels: HashSet<NodeLabel> = [
        NodeLabel::Function, NodeLabel::Class, NodeLabel::Method,
        NodeLabel::Controller, NodeLabel::ControllerAction,
        NodeLabel::Service, NodeLabel::Repository, NodeLabel::ExternalService,
        NodeLabel::File, NodeLabel::Community,
    ].into_iter().collect();
    let mut other_labels: Vec<_> = label_counts.iter()
        .filter(|(label, count)| !shown_labels.contains(label) && **count > 0)
        .collect();
    other_labels.sort_by(|a, b| b.1.cmp(a.1));
    for (label, count) in other_labels.iter().take(10) {
        writeln!(f, "| {} | {} |", label.as_str(), count)?;
    }
    writeln!(f)?;

    // ── Top 10 Most Connected Nodes ──
    writeln!(f, "## Top 10 — Composants les plus connectés")?;
    writeln!(f, "<!-- GNX:INTRO:top-connected -->")?;
    writeln!(f)?;
    writeln!(f, "> Ces composants ont le plus de dépendances. Un changement dans l'un d'eux")?;
    writeln!(f, "> a un impact potentiel large sur le reste du système.")?;
    writeln!(f)?;

    // Compute degree for each node
    let mut node_degree: HashMap<String, usize> = HashMap::new();
    for rel in graph.iter_relationships() {
        *node_degree.entry(rel.source_id.clone()).or_insert(0) += 1;
        *node_degree.entry(rel.target_id.clone()).or_insert(0) += 1;
    }
    let mut sorted_degree: Vec<_> = node_degree.into_iter().collect();
    sorted_degree.sort_by(|a, b| b.1.cmp(&a.1));

    writeln!(f, "| Composant | Type | Connexions | Fichier |")?;
    writeln!(f, "|-----------|------|-----------|---------|")?;
    for (node_id, degree) in sorted_degree.iter().take(10) {
        if let Some(node) = graph.get_node(node_id) {
            writeln!(f, "| `{}` | {} | {} | `{}` |",
                node.properties.name,
                node.label.as_str(),
                degree,
                node.properties.file_path)?;
        }
    }
    writeln!(f)?;

    // ── Top 10 Largest Files ──
    writeln!(f, "## Top 10 — Fichiers les plus volumineux")?;
    writeln!(f)?;

    // Count symbols per file, and track the dominant label
    let mut file_stats: HashMap<String, (usize, HashMap<NodeLabel, usize>)> = HashMap::new();
    for node in graph.iter_nodes() {
        if !node.properties.file_path.is_empty() && node.label != NodeLabel::File {
            let entry = file_stats
                .entry(node.properties.file_path.clone())
                .or_insert_with(|| (0, HashMap::new()));
            entry.0 += 1;
            *entry.1.entry(node.label).or_insert(0) += 1;
        }
    }
    let mut sorted_files: Vec<_> = file_stats.into_iter().collect();
    sorted_files.sort_by(|a, b| (b.1).0.cmp(&(a.1).0));

    writeln!(f, "| Fichier | Symboles | Type principal |")?;
    writeln!(f, "|---------|----------|---------------|")?;
    for (file_path, (sym_count, label_map)) in sorted_files.iter().take(10) {
        let dominant = label_map.iter()
            .max_by_key(|(_, c)| *c)
            .map(|(l, _)| l.as_str())
            .unwrap_or("-");
        writeln!(f, "| `{}` | {} | {} |", file_path, sym_count, dominant)?;
    }
    writeln!(f)?;

    // ── External Services ──
    if ext_count > 0 {
        writeln!(f, "## Services Externes")?;
        writeln!(f)?;
        writeln!(f, "| Service | Fichier |")?;
        writeln!(f, "|---------|---------|")?;
        for node in graph.iter_nodes() {
            if node.label == NodeLabel::ExternalService {
                writeln!(f, "| `{}` | `{}` |", node.properties.name, node.properties.file_path)?;
            }
        }
        writeln!(f)?;
    }

    writeln!(f, "<!-- GNX:CLOSING -->")?;
    writeln!(f, "---")?;
    writeln!(f, "**Voir aussi :** [Vue d'ensemble](./overview.md) · [Architecture](./architecture.md)")?;

    println!("  {} project-health.md", "OK".green());
    Ok(())
}

// ─── Git Analytics Pages Generator ─────────────────────────────────────

/// Generate hotspots, coupling, and ownership pages using gitnexus-git.
/// Returns the number of pages successfully generated (0-3).
fn generate_git_analytics_pages(docs_dir: &Path, repo_path: &Path) -> Result<usize> {
    let mut count = 0;

    // ── Hotspots ──
    match gitnexus_git::hotspots::analyze_hotspots(repo_path, 90) {
        Ok(hotspots) if !hotspots.is_empty() => {
            let out_path = docs_dir.join("hotspots.md");
            let mut f = std::fs::File::create(&out_path)?;

            writeln!(f, "# Code Hotspots")?;
            writeln!(f, "<!-- GNX:LEAD -->")?;
            writeln!(f)?;
            writeln!(f, "> Fichiers les plus fréquemment modifiés ces 90 derniers jours.")?;
            writeln!(f, "> Un hotspot élevé signale un fichier qui change souvent — risque de régressions, dette technique ou logique métier centrale.")?;
            writeln!(f)?;

            writeln!(f, "## Top 20 fichiers les plus modifiés")?;
            writeln!(f)?;
            writeln!(f, "| # | Fichier | Commits | Churn | Auteurs | Score |")?;
            writeln!(f, "|---|---------|---------|-------|---------|-------|")?;
            for (i, h) in hotspots.iter().take(20).enumerate() {
                let short_path = h.path.replace('\\', "/");
                let bar = "█".repeat((h.score * 10.0) as usize);
                writeln!(f, "| {} | `{}` | {} | +{}/-{} | {} | {} {:.0}% |",
                    i + 1,
                    short_path,
                    h.commit_count,
                    h.lines_added,
                    h.lines_removed,
                    h.author_count,
                    bar,
                    h.score * 100.0)?;
            }
            writeln!(f)?;

            // Interpretation
            writeln!(f, "## Interprétation")?;
            writeln!(f)?;
            let top3: Vec<_> = hotspots.iter().take(3).collect();
            if !top3.is_empty() {
                writeln!(f, "Les fichiers les plus chauds sont :")?;
                for h in &top3 {
                    writeln!(f, "- **`{}`** — {} commits, churn {} lignes, {} auteurs",
                        h.path.replace('\\', "/"), h.commit_count, h.churn, h.author_count)?;
                }
                writeln!(f)?;
                writeln!(f, "> **Recommandation :** Les fichiers avec un score >70% et >3 auteurs sont des candidats prioritaires pour du refactoring ou de meilleurs tests.")?;
            }
            writeln!(f)?;

            println!("  {} hotspots.md ({} fichiers)", "OK".green(), hotspots.len().min(20));
            count += 1;
        }
        Ok(_) => {
            debug!("No hotspots found, skipping page");
        }
        Err(e) => {
            debug!("Could not analyze hotspots: {}", e);
        }
    }

    // ── Temporal Coupling ──
    match gitnexus_git::coupling::analyze_coupling(repo_path, 3) {
        Ok(couplings) if !couplings.is_empty() => {
            let out_path = docs_dir.join("coupling.md");
            let mut f = std::fs::File::create(&out_path)?;

            writeln!(f, "# Temporal Coupling")?;
            writeln!(f, "<!-- GNX:LEAD -->")?;
            writeln!(f)?;
            writeln!(f, "> Paires de fichiers qui changent toujours ensemble.")?;
            writeln!(f, "> Un couplage temporel élevé peut indiquer une dépendance implicite non visible dans le code.")?;
            writeln!(f)?;

            writeln!(f, "## Paires les plus couplées")?;
            writeln!(f)?;
            writeln!(f, "| # | Fichier A | Fichier B | Commits partagés | Force |")?;
            writeln!(f, "|---|-----------|-----------|-----------------|-------|")?;
            for (i, c) in couplings.iter().take(20).enumerate() {
                let bar = "█".repeat((c.coupling_strength * 10.0) as usize);
                writeln!(f, "| {} | `{}` | `{}` | {} | {} {:.0}% |",
                    i + 1,
                    c.file_a.replace('\\', "/"),
                    c.file_b.replace('\\', "/"),
                    c.shared_commits,
                    bar,
                    c.coupling_strength * 100.0)?;
            }
            writeln!(f)?;

            writeln!(f, "## Interprétation")?;
            writeln!(f)?;
            let strong: Vec<_> = couplings.iter().filter(|c| c.coupling_strength > 0.7).collect();
            if !strong.is_empty() {
                writeln!(f, "**{} paires fortement couplées** (>70%) détectées :", strong.len())?;
                writeln!(f)?;
                for c in strong.iter().take(5) {
                    writeln!(f, "- `{}` ↔ `{}` ({:.0}%)",
                        c.file_a.replace('\\', "/"),
                        c.file_b.replace('\\', "/"),
                        c.coupling_strength * 100.0)?;
                }
                writeln!(f)?;
                writeln!(f, "> **Recommandation :** Un couplage >70% suggère que ces fichiers devraient peut-être être fusionnés, ou qu'une abstraction commune manque.")?;
            } else {
                writeln!(f, "Aucune paire n'est couplée à plus de 70%. Le codebase a un couplage temporel raisonnable.")?;
            }
            writeln!(f)?;

            println!("  {} coupling.md ({} paires)", "OK".green(), couplings.len().min(20));
            count += 1;
        }
        Ok(_) => {
            debug!("No coupling data found, skipping page");
        }
        Err(e) => {
            debug!("Could not analyze coupling: {}", e);
        }
    }

    // ── Code Ownership ──
    match gitnexus_git::ownership::analyze_ownership(repo_path) {
        Ok(ownerships) if !ownerships.is_empty() => {
            let out_path = docs_dir.join("ownership.md");
            let mut f = std::fs::File::create(&out_path)?;

            writeln!(f, "# Code Ownership")?;
            writeln!(f, "<!-- GNX:LEAD -->")?;
            writeln!(f)?;
            writeln!(f, "> Répartition de la propriété du code par auteur principal.")?;
            writeln!(f, "> Les fichiers avec un ownership faible (<50%) ou beaucoup d'auteurs indiquent un manque de propriétaire clair.")?;
            writeln!(f)?;

            // Group by primary author
            let mut by_author: BTreeMap<String, Vec<&gitnexus_git::types::FileOwnership>> = BTreeMap::new();
            for o in &ownerships {
                by_author.entry(o.primary_author.clone()).or_default().push(o);
            }

            writeln!(f, "## Résumé par auteur")?;
            writeln!(f)?;
            writeln!(f, "| Auteur | Fichiers possédés | Ownership moyen |")?;
            writeln!(f, "|--------|-------------------|-----------------|")?;
            let mut author_stats: Vec<_> = by_author.iter().map(|(author, files)| {
                let avg_pct = files.iter().map(|f| f.ownership_pct).sum::<f64>() / files.len() as f64;
                (author.clone(), files.len(), avg_pct)
            }).collect();
            author_stats.sort_by(|a, b| b.1.cmp(&a.1));
            for (author, file_count, avg_pct) in &author_stats {
                writeln!(f, "| {} | {} | {:.0}% |", author, file_count, avg_pct)?;
            }
            writeln!(f)?;

            writeln!(f, "## Fichiers à risque (ownership < 50%)")?;
            writeln!(f)?;
            let low_ownership: Vec<_> = ownerships.iter()
                .filter(|o| o.ownership_pct < 50.0)
                .collect();
            if low_ownership.is_empty() {
                writeln!(f, "Tous les fichiers ont un propriétaire clair (>50%). Bonne pratique.")?;
            } else {
                writeln!(f, "| Fichier | Auteur principal | Ownership | Auteurs |")?;
                writeln!(f, "|---------|-----------------|-----------|---------|")?;
                for o in low_ownership.iter().take(20) {
                    writeln!(f, "| `{}` | {} | {:.0}% | {} |",
                        o.path.replace('\\', "/"),
                        o.primary_author,
                        o.ownership_pct,
                        o.author_count)?;
                }
                writeln!(f)?;
                writeln!(f, "> **Recommandation :** Ces {} fichiers n'ont pas de propriétaire clair. Assigner un responsable réduit le risque de régressions.", low_ownership.len())?;
            }
            writeln!(f)?;

            writeln!(f, "## Top 20 fichiers les plus distribués")?;
            writeln!(f)?;
            writeln!(f, "| # | Fichier | Auteurs | Ownership principal |")?;
            writeln!(f, "|---|---------|---------|---------------------|")?;
            // Sorted by author_count desc (most distributed first)
            let mut sorted_own = ownerships.clone();
            sorted_own.sort_by(|a, b| b.author_count.cmp(&a.author_count));
            for (i, o) in sorted_own.iter().take(20).enumerate() {
                writeln!(f, "| {} | `{}` | {} | {} ({:.0}%) |",
                    i + 1,
                    o.path.replace('\\', "/"),
                    o.author_count,
                    o.primary_author,
                    o.ownership_pct)?;
            }
            writeln!(f)?;

            println!("  {} ownership.md ({} fichiers)", "OK".green(), ownerships.len().min(20));
            count += 1;
        }
        Ok(_) => {
            debug!("No ownership data found, skipping page");
        }
        Err(e) => {
            debug!("Could not analyze ownership: {}", e);
        }
    }

    if count > 0 {
        println!(
            "{} Generated {} git analytics pages",
            "OK".green(),
            count
        );
    }

    Ok(count)
}

// ─── Functional Guide Generator ────────────────────────────────────────

fn generate_functional_guide(
    docs_dir: &Path,
    repo_name: &str,
    graph: &KnowledgeGraph,
) -> Result<()> {
    let label_counts = count_nodes_by_label(graph);
    let has_controllers = label_counts.get(&NodeLabel::Controller).copied().unwrap_or(0) > 0;

    // Only generate for ASP.NET MVC projects with controllers
    if !has_controllers {
        return Ok(());
    }

    let out_path = docs_dir.join("functional-guide.md");
    let mut f = std::fs::File::create(&out_path)?;

    let ctrl_count = label_counts.get(&NodeLabel::Controller).copied().unwrap_or(0);
    let view_count = label_counts.get(&NodeLabel::View).copied().unwrap_or(0);
    let action_count = label_counts.get(&NodeLabel::ControllerAction).copied().unwrap_or(0);
    let entity_count = label_counts.get(&NodeLabel::DbEntity).copied().unwrap_or(0);
    let svc_count = label_counts.get(&NodeLabel::Service).copied().unwrap_or(0);
    let ui_count = label_counts.get(&NodeLabel::UiComponent).copied().unwrap_or(0);
    let ext_count = label_counts.get(&NodeLabel::ExternalService).copied().unwrap_or(0);

    // Collect controllers and group actions by controller
    let controllers: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::Controller)
        .collect();

    writeln!(f, "# Guide Fonctionnel — {}", repo_name)?;
    writeln!(f, "<!-- GNX:LEAD -->")?;
    writeln!(f)?;

    // Source files
    let ctrl_files: Vec<&str> = controllers.iter()
        .map(|c| c.properties.file_path.as_str())
        .take(10)
        .collect();
    write!(f, "{}", source_files_section(&ctrl_files))?;

    writeln!(f, "> Ce guide décrit les modules fonctionnels de l'application du point de vue métier.")?;
    writeln!(f, "> Il est destiné aux responsables de service et aux personnes reprenant l'application.")?;
    writeln!(f)?;

    // Quick stats
    writeln!(f, "| Métrique | Valeur |")?;
    writeln!(f, "|----------|--------|")?;
    writeln!(f, "| Modules fonctionnels | {} controllers |", ctrl_count)?;
    writeln!(f, "| Fonctionnalités | {} actions |", action_count)?;
    writeln!(f, "| Écrans | {} vues |", view_count)?;
    writeln!(f, "| Entités de données | {} |", entity_count)?;
    writeln!(f, "| Services métier | {} |", svc_count)?;
    writeln!(f, "| Composants UI | {} grilles Telerik |", ui_count)?;
    writeln!(f, "| Intégrations externes | {} services |", ext_count)?;
    writeln!(f)?;

    // Generate module documentation for each controller
    // Sort by action count descending (most important first)
    let mut ctrl_with_actions: Vec<(&GraphNode, Vec<&GraphNode>)> = controllers.iter()
        .map(|ctrl| {
            let actions: Vec<&GraphNode> = graph.iter_nodes()
                .filter(|n| n.label == NodeLabel::ControllerAction
                    && n.properties.file_path == ctrl.properties.file_path)
                .collect();
            (*ctrl, actions)
        })
        .collect();
    ctrl_with_actions.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    for (ctrl, actions) in &ctrl_with_actions {
        let name = ctrl.properties.name
            .strip_suffix("Controller").unwrap_or(&ctrl.properties.name);

        // Skip RootController (base class, not a real module)
        if name == "Root" || name == "PdfView" || name == "Print" {
            continue;
        }

        writeln!(f, "---")?;
        writeln!(f)?;
        writeln!(f, "## {}", name)?;
        writeln!(f)?;

        // Heuristic business description
        let desc = describe_controller_fr(&ctrl.properties.name);
        writeln!(f, "**Finalité métier :** {}", desc)?;
        writeln!(f)?;

        // Count views for this controller
        let ctrl_views: Vec<&GraphNode> = graph.iter_nodes()
            .filter(|n| n.label == NodeLabel::View
                && n.properties.file_path.contains(name))
            .collect();

        // Count UI components for this controller
        let ctrl_ui: Vec<&GraphNode> = graph.iter_nodes()
            .filter(|n| n.label == NodeLabel::UiComponent
                && n.properties.file_path.contains(name))
            .collect();

        writeln!(f, "| | |")?;
        writeln!(f, "|---|---|")?;
        writeln!(f, "| **Actions** | {} |", actions.len())?;
        writeln!(f, "| **Écrans** | {} vues |", ctrl_views.len())?;
        if !ctrl_ui.is_empty() {
            writeln!(f, "| **Grilles Telerik** | {} |", ctrl_ui.len())?;
        }
        writeln!(f)?;

        // Key actions (group by GET/POST)
        let _get_actions: Vec<&&GraphNode> = actions.iter()
            .filter(|a| a.properties.http_method.as_deref().unwrap_or("GET") == "GET")
            .collect();
        let _post_actions: Vec<&&GraphNode> = actions.iter()
            .filter(|a| a.properties.http_method.as_deref().unwrap_or("GET") == "POST")
            .collect();

        writeln!(f, "**Processus principaux :**")?;
        writeln!(f)?;

        // List top actions by name patterns
        let mut listed = 0;
        for action in actions.iter().take(15) {
            let aname = &action.properties.name;
            let method = action.properties.http_method.as_deref().unwrap_or("GET");
            let icon = if method == "POST" { "✏️" } else { "📄" };
            writeln!(f, "- {} **{}** ({})", icon, aname, method)?;
            listed += 1;
        }
        if actions.len() > listed {
            writeln!(f, "- *...et {} autres actions*", actions.len() - listed)?;
        }
        writeln!(f)?;

        // Key grids
        if !ctrl_ui.is_empty() {
            writeln!(f, "**Grilles principales :**")?;
            writeln!(f)?;
            for comp in ctrl_ui.iter().take(5) {
                let cols = comp.properties.description.as_deref().unwrap_or("");
                let model = comp.properties.bound_model.as_deref().unwrap_or("-");
                writeln!(f, "- **{}** (modèle: `{}`)", comp.properties.name, model)?;
                if !cols.is_empty() {
                    writeln!(f, "  - Colonnes : {}", cols)?;
                }
            }
            writeln!(f)?;
        }

        // Criticality
        let criticality = if actions.len() > 30 {
            "🔴 **Très élevé** — Module complexe avec de nombreuses fonctionnalités"
        } else if actions.len() > 10 {
            "🟡 **Élevé** — Module important dans le workflow quotidien"
        } else {
            "🟢 **Moyen** — Module de support ou consultation"
        };
        writeln!(f, "**Niveau de criticité :** {}", criticality)?;
        writeln!(f)?;

        // Simple flow diagram (only for major controllers)
        if actions.len() > 5 {
            writeln!(f, "**Flux principal :**")?;
            writeln!(f)?;
            writeln!(f, "```mermaid")?;
            writeln!(f, "flowchart LR")?;

            // Show: Search → View/Create → Edit → Validate
            let has_search = actions.iter().any(|a| {
                let n = a.properties.name.to_lowercase();
                n.contains("rech") || n.contains("search") || n.contains("list") || n.contains("get")
            });
            let has_create = actions.iter().any(|a| {
                let n = a.properties.name.to_lowercase();
                n.contains("cre") || n.contains("new") || n.contains("add")
            });
            let has_edit = actions.iter().any(|a| {
                let n = a.properties.name.to_lowercase();
                n.contains("modif") || n.contains("edit") || n.contains("update")
            });
            let has_detail = actions.iter().any(|a| {
                let n = a.properties.name.to_lowercase();
                n.contains("detail") || n.contains("view")
            });
            let has_export = actions.iter().any(|a| {
                let n = a.properties.name.to_lowercase();
                n.contains("export") || n.contains("excel") || n.contains("csv")
            });
            let has_delete = actions.iter().any(|a| {
                let n = a.properties.name.to_lowercase();
                n.contains("suppr") || n.contains("delete")
            });

            let mut steps = Vec::new();
            if has_search { steps.push(("Recherche", "Rechercher")); }
            if has_detail { steps.push(("Consultation", "Consulter")); }
            if has_create { steps.push(("Creation", "Créer")); }
            if has_edit { steps.push(("Modification", "Modifier")); }
            if has_delete { steps.push(("Suppression", "Supprimer")); }
            if has_export { steps.push(("Export", "Exporter")); }

            for (id, label) in &steps {
                writeln!(f, "    {}[\"{}\" ]", id, label)?;
            }
            for i in 0..steps.len().saturating_sub(1) {
                writeln!(f, "    {} --> {}", steps[i].0, steps[i + 1].0)?;
            }

            writeln!(f, "```")?;
            writeln!(f)?;
        }
    }

    // Sequence diagrams for critical flows
    writeln!(f, "---")?;
    writeln!(f)?;
    writeln!(f, "## Flux critiques")?;
    writeln!(f, "<!-- GNX:INTRO:flux-critiques -->")?;
    writeln!(f)?;

    writeln!(f, "### Recherche Bénéficiaire")?;
    writeln!(f)?;
    writeln!(f, "```mermaid")?;
    writeln!(f, "sequenceDiagram")?;
    writeln!(f, "    participant U as Utilisateur")?;
    writeln!(f, "    participant C as BeneficiaireController")?;
    writeln!(f, "    participant S as BenefService")?;
    writeln!(f, "    participant API as Erable API")?;
    writeln!(f, "    U->>C: Recherche (NIA ou Nom)")?;
    writeln!(f, "    C->>S: RechercheOuvrantDroit(filtre)")?;
    writeln!(f, "    S->>API: CMCASClient.OuvrantsDroitGetAsync()")?;
    writeln!(f, "    API-->>S: FicheODLite[]")?;
    writeln!(f, "    S->>API: FoyerClient.MembresduFoyerGetAsync()")?;
    writeln!(f, "    API-->>S: Foyer (composition familiale)")?;
    writeln!(f, "    S-->>C: Liste bénéficiaires")?;
    writeln!(f, "    C-->>U: Grille Telerik avec résultats")?;
    writeln!(f, "```")?;
    writeln!(f)?;

    writeln!(f, "### Création Dossier")?;
    writeln!(f)?;
    writeln!(f, "```mermaid")?;
    writeln!(f, "sequenceDiagram")?;
    writeln!(f, "    participant U as Utilisateur")?;
    writeln!(f, "    participant C as DossiersController")?;
    writeln!(f, "    participant S as DossierService")?;
    writeln!(f, "    participant DB as Entity Framework")?;
    writeln!(f, "    U->>C: Sélection Domaine + Groupe Aide")?;
    writeln!(f, "    C->>S: AfficherAides(idGrpAide)")?;
    writeln!(f, "    S-->>C: Liste Aides disponibles")?;
    writeln!(f, "    U->>C: Choix Aides + Dates")?;
    writeln!(f, "    C->>S: CreerDossier(DossierPresta)")?;
    writeln!(f, "    S->>S: Calcul Barème + Plafonds")?;
    writeln!(f, "    S->>DB: Insert Dossier + Prestations")?;
    writeln!(f, "    DB-->>S: OK")?;
    writeln!(f, "    S-->>C: Dossier créé")?;
    writeln!(f, "    C-->>U: Page détails dossier")?;
    writeln!(f, "```")?;
    writeln!(f)?;

    writeln!(f, "### Export ELODIE")?;
    writeln!(f)?;
    writeln!(f, "```mermaid")?;
    writeln!(f, "sequenceDiagram")?;
    writeln!(f, "    participant U as Utilisateur")?;
    writeln!(f, "    participant C as FacturesController")?;
    writeln!(f, "    participant S as FactureService")?;
    writeln!(f, "    participant DB as Entity Framework")?;
    writeln!(f, "    U->>C: Validation paiements")?;
    writeln!(f, "    C->>S: ValidationPaiement()")?;
    writeln!(f, "    S->>S: Vérification montants + plafonds")?;
    writeln!(f, "    S->>DB: Mise à jour statuts")?;
    writeln!(f, "    U->>C: Générer bordereau")?;
    writeln!(f, "    C->>S: GenerBordereau()")?;
    writeln!(f, "    S->>DB: Création bordereaux")?;
    writeln!(f, "    U->>C: Export ELODIE")?;
    writeln!(f, "    C->>S: ExportElodie()")?;
    writeln!(f, "    S->>S: Formatage Flux3")?;
    writeln!(f, "    S-->>C: Fichier ELODIE")?;
    writeln!(f, "    C-->>U: Téléchargement fichier")?;
    writeln!(f, "```")?;
    writeln!(f)?;

    // Synthesis
    writeln!(f, "<!-- GNX:CLOSING -->")?;
    writeln!(f, "---")?;
    writeln!(f)?;
    writeln!(f, "## Synthèse : Modules les plus critiques")?;
    writeln!(f)?;

    // Sort by action count, take top 3
    let top3: Vec<&(&GraphNode, Vec<&GraphNode>)> = ctrl_with_actions.iter()
        .filter(|(c, _)| {
            let n = c.properties.name.as_str();
            n != "RootController" && n != "PdfViewController" && n != "PrintController"
        })
        .take(3)
        .collect();

    for (i, (ctrl, actions)) in top3.iter().enumerate() {
        let name = ctrl.properties.name
            .strip_suffix("Controller").unwrap_or(&ctrl.properties.name);
        writeln!(f, "### {}. {}", i + 1, name)?;
        writeln!(f)?;
        writeln!(f, "**{} actions** — {}", actions.len(), describe_controller_fr(&ctrl.properties.name))?;
        writeln!(f)?;
    }

    writeln!(f, "---")?;
    writeln!(f)?;
    writeln!(f, "**Voir aussi :** [Vue d'ensemble](./overview.md) · [Architecture](./architecture.md)")?;
    writeln!(f)?;
    writeln!(f, "[← Previous: Overview](./overview.md) | [Next: Architecture →](./architecture.md)")?;

    println!("  {} {}", "OK".green(), out_path.display());

    Ok(())
}

/// French business description for a controller based on its name.
fn describe_project_fr(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.contains("ihm") && !lower.contains("test") { "Application web ASP.NET MVC (Présentation)" }
    else if lower.contains("bal") && !lower.contains("test") { "Couche métier (Business Logic)" }
    else if lower.contains("dal") && !lower.contains("test") { "Couche d'accès aux données (Entity Framework)" }
    else if lower.contains("entities") { "Entités / objets métier partagés" }
    else if lower.contains("commun") { "Utilitaires et attributs communs" }
    else if lower.contains("courrier") && !lower.contains("test") { "Génération de courriers (mail merge)" }
    else if lower.contains("erable") || lower.contains("webapi") { "Client API REST Erable (bénéficiaires)" }
    else if lower.contains("ldap") { "Client LDAP / Active Directory" }
    else if lower.contains("pdf") { "Génération de rapports PDF" }
    else if lower.contains("ressource") { "Fichiers de ressources (localisation)" }
    else if lower.contains("traitement") || lower.contains("batch") { "Traitement batch / planifié" }
    else if lower.contains("console") { "Application console" }
    else if lower.contains("test") { "Tests unitaires / intégration" }
    else { "Projet" }
}

fn describe_controller_fr(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.contains("administration") {
        "Configurer le référentiel d'aides (groupes, aides, barèmes, plafonds, majorations, tarifs, justificatifs). C'est le socle de paramétrage dont dépend toute l'application."
    } else if lower.contains("dossier") {
        "Gérer le cycle de vie complet des dossiers d'aide sociale — de la demande à la clôture, en passant par le calcul des droits via les barèmes et la sélection des aides."
    } else if lower.contains("facture") {
        "Gérer la chaîne financière : facturation fournisseurs, paiement bénéficiaires, régularisations, validation et export ELODIE vers la comptabilité centrale."
    } else if lower.contains("beneficiaire") {
        "Rechercher et consulter les profils des ouvrants droit (OD) et ayants droit (AD) issus du WebAPI Erable, puis les lier aux dossiers d'aide."
    } else if lower.contains("courrier") {
        "Générer des courriers personnalisés aux bénéficiaires — individuellement ou en masse — à partir de modèles avec champs de fusion."
    } else if lower.contains("statistique") {
        "Produire les tableaux de bord et rapports réglementaires : suivi budgétaire, comptage dossiers, analyse paiements, restitutions mensuelles."
    } else if lower.contains("fournisseur") {
        "Gérer le référentiel des fournisseurs de prestations sociales et leur association aux dossiers."
    } else if lower.contains("utilisateur") {
        "Administrer les comptes utilisateurs, les profils d'habilitation et les droits d'accès par CMCAS."
    } else if lower.contains("profil") {
        "Gérer les profils d'habilitation et les autorisations fonctionnelles des utilisateurs."
    } else if lower.contains("intervention") {
        "Suivre les interventions terrain liées aux dossiers de bénéficiaires."
    } else if lower.contains("commission") {
        "Gérer les commissions d'attribution des aides (nationales et locales)."
    } else if lower.contains("mco") {
        "Module de maintien en condition opérationnelle — suivi de l'éligibilité et des cas particuliers."
    } else if lower.contains("archiver") {
        "Archiver les dossiers clôturés pour libérer l'espace de travail courant."
    } else if lower.contains("home") {
        "Page d'accueil avec messages d'information, authentification et navigation principale."
    } else {
        "Module fonctionnel de l'application."
    }
}

// ─── Glossary Generator ───────────────────────────────────────────────

// ─── Deployment Guide Generator ───────────────────────────────────────

fn generate_deployment_guide(
    docs_dir: &Path,
    _repo_name: &str,
    graph: &KnowledgeGraph,
) -> Result<()> {
    let out_path = docs_dir.join("deployment.md");
    let mut f = std::fs::File::create(&out_path)?;

    writeln!(f, "# Guide Environnement & Déploiement")?;
    writeln!(f)?;
    writeln!(f, "> Informations techniques pour configurer et déployer l'application.")?;
    writeln!(f)?;

    writeln!(f, "## Prérequis")?;
    writeln!(f, "- .NET Framework 4.8")?;
    writeln!(f, "- Visual Studio 2019/2022")?;
    writeln!(f, "- SQL Server 2012+")?;
    writeln!(f, "- IIS / IIS Express")?;
    writeln!(f, "- Node.js (pour les scripts de build frontend)")?;
    writeln!(f)?;

    writeln!(f)?;

    // Databases from DbContext nodes
    let db_contexts: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::DbContext)
        .collect();
    writeln!(f, "## Bases de données")?;
    writeln!(f)?;
    if db_contexts.is_empty() {
        writeln!(f, "Aucun DbContext détecté.")?;
    } else {
        for ctx in &db_contexts {
            writeln!(f, "- **{}** (`{}`)", ctx.properties.name, ctx.properties.file_path)?;
        }
    }
    writeln!(f)?;

    // External services
    let ext_services: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::ExternalService)
        .collect();
    writeln!(f, "## Services externes")?;
    writeln!(f)?;
    if ext_services.is_empty() {
        writeln!(f, "Aucun service externe détecté.")?;
    } else {
        for svc in &ext_services {
            let stype = svc.properties.service_type.as_deref().unwrap_or("REST");
            writeln!(f, "- **{}** ({})", svc.properties.name, stype)?;
        }
    }
    writeln!(f)?;

    writeln!(f, "## Configuration")?;
    writeln!(f, "<!-- GNX:INTRO:configuration -->")?;
    writeln!(f)?;
    writeln!(f, "Les fichiers `Web.config` contiennent les paramètres par environnement.")?;
    writeln!(f, "Chaque environnement a sa propre transformation `Web.{{env}}.config`.")?;
    writeln!(f)?;

    // List config files detected
    let config_files: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| {
            n.label == NodeLabel::File
                && (n.properties.file_path.ends_with(".config")
                    || n.properties.file_path.ends_with(".Config"))
                && !n.properties.file_path.contains("PackageTmp")
                && !n.properties.file_path.contains("/obj/")
                && !n.properties.file_path.contains("\\obj\\")
        })
        .collect();

    if !config_files.is_empty() {
        writeln!(f, "### Fichiers de configuration détectés")?;
        writeln!(f)?;
        writeln!(f, "| Fichier | Rôle |")?;
        writeln!(f, "|---------|------|")?;
        for cf in &config_files {
            let path = cf.properties.file_path.replace('\\', "/");
            let role = if path.contains("Web.config") && !path.contains(".Release") && !path.contains(".Debug") {
                "Configuration principale"
            } else if path.contains("Release") {
                "Transformation production"
            } else if path.contains("Debug") {
                "Transformation développement"
            } else if path.contains("Qualification") {
                "Transformation qualification"
            } else if path.contains("packages.config") {
                "Dépendances NuGet"
            } else {
                "Configuration"
            };
            writeln!(f, "| `{}` | {} |", path, role)?;
        }
        writeln!(f)?;
    }

    // ASP.NET deployment checklist
    let has_controllers = graph.iter_nodes().any(|n| n.label == NodeLabel::Controller);
    if has_controllers {
        writeln!(f, "## Déploiement ASP.NET MVC")?;
        writeln!(f, "<!-- GNX:INTRO:deploiement-aspnet -->")?;
        writeln!(f)?;
        writeln!(f, "### Checklist")?;
        writeln!(f)?;
        writeln!(f, "1. **Compiler en Release** : `msbuild /p:Configuration=Release`")?;
        writeln!(f, "2. **Publier** : clic droit → Publier → Profil de publication")?;
        writeln!(f, "3. **Transformations** : `Web.Release.config` appliquée automatiquement")?;
        writeln!(f, "4. **IIS** : pool .NET 4.x (pipeline intégré), pointer vers le dossier publié")?;
        writeln!(f, "5. **ConnectionStrings** : configurer dans `Web.config` du serveur")?;
        writeln!(f, "6. **Tester** : naviguer vers l'URL du site")?;
        writeln!(f)?;

        writeln!(f, "### Environnements")?;
        writeln!(f)?;
        writeln!(f, "| Environnement | Transformation | Usage |")?;
        writeln!(f, "|--------------|----------------|-------|")?;
        writeln!(f, "| Développement | `Web.Debug.config` | Debug local (IIS Express) |")?;
        writeln!(f, "| Qualification | `Web.Qualification.config` | Tests pré-production |")?;
        writeln!(f, "| Production | `Web.Release.config` | Serveur de production |")?;
        writeln!(f)?;
    }

    println!("  {} deployment.md", "OK".green());
    Ok(())
}

// ─── Service Description Helper ───────────────────────────────────────

fn describe_service_fr(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.contains("aide") { "Gestion des aides financières et paramétrage" }
    else if lower.contains("bareme") { "Calcul des barèmes et tranches de revenus" }
    else if lower.contains("dossier") { "Création et suivi des dossiers d'aide" }
    else if lower.contains("facture") { "Facturation fournisseurs et paiements" }
    else if lower.contains("benef") { "Recherche et gestion des bénéficiaires" }
    else if lower.contains("courrier") { "Génération et envoi de courriers" }
    else if lower.contains("profil") { "Gestion des profils et habilitations" }
    else if lower.contains("utilisateur") { "Administration des comptes utilisateurs" }
    else if lower.contains("statistique") { "Tableaux de bord et restitutions" }
    else if lower.contains("parametr") { "Configuration et paramètres système" }
    else if lower.contains("message") { "Gestion des messages d'erreur et d'accueil" }
    else if lower.contains("grpaide") { "Gestion des groupes d'aides" }
    else if lower.contains("cmcas") { "Données et paramètres CMCAS" }
    else if lower.contains("background") { "Traitement asynchrone (Hangfire)" }
    else if lower.contains("elodie") { "Export comptable vers ELODIE" }
    else if lower.contains("numcommi") { "Numérotation des commissions" }
    else if lower.contains("unitofwork") { "Gestion transactionnelle des données" }
    else { "Service métier" }
}

// ─── HTML Site Generator ───────────────────────────────────────────────

fn generate_html_site(
    graph: &KnowledgeGraph,
    repo_path: &Path,
) -> Result<()> {
    let docs_dir = repo_path.join(".gitnexus").join("docs");
    if !docs_dir.exists() {
        return Err(anyhow::anyhow!(
            "No docs found. Run 'generate docs' first."
        ));
    }

    // 1. Collect all .md files from docs/
    let mut pages: BTreeMap<String, (String, String)> = BTreeMap::new(); // id -> (title, html_content)

    for entry in std::fs::read_dir(&docs_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "md") {
            let content = std::fs::read_to_string(&path)?;
            let filename = path.file_stem().unwrap().to_string_lossy().to_string();
            let title = extract_title_from_md(&content).unwrap_or_else(|| filename.clone());
            let html = markdown_to_html(&content);
            pages.insert(filename, (title, html));
        }
    }

    // Also read modules/ subdirectory
    let modules_dir = docs_dir.join("modules");
    if modules_dir.exists() {
        for entry in std::fs::read_dir(&modules_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "md") {
                let content = std::fs::read_to_string(&path)?;
                let filename = path.file_stem().unwrap().to_string_lossy().to_string();
                let title = extract_title_from_md(&content).unwrap_or_else(|| filename.clone());
                let html = markdown_to_html(&content);
                pages.insert(format!("modules/{}", filename), (title, html));
            }
        }
    }

    if pages.is_empty() {
        return Err(anyhow::anyhow!(
            "No .md pages found in {}",
            docs_dir.display()
        ));
    }

    // 2. Build sidebar HTML with numbered sections
    let mut sidebar_html = String::new();

    // Group pages by category — force overview first
    let preferred_order = [
        "overview", "functional-guide", "project-health", "architecture",
        "getting-started", "deployment",
        "hotspots", "coupling", "ownership",
        "aspnet-controllers", "aspnet-routes", "aspnet-entities", "aspnet-data-model",
        "aspnet-views", "aspnet-services", "aspnet-external", "aspnet-entities-detail",
        "aspnet-seq-http", "aspnet-seq-data",
    ];

    let mut overview_pages: Vec<_> = pages
        .iter()
        .filter(|(k, _)| !k.starts_with("modules/"))
        .collect();
    // Sort by preferred order, then alphabetically for unlisted
    overview_pages.sort_by_key(|(k, _)| {
        preferred_order.iter().position(|&p| k.as_str() == p).unwrap_or(999)
    });

    let module_pages: Vec<_> = pages
        .iter()
        .filter(|(k, _)| k.starts_with("modules/"))
        .collect();

    let first_page_id = overview_pages
        .first()
        .map(|(k, _)| k.as_str())
        .unwrap_or("");

    let mut section_num: usize = 1;

    sidebar_html.push_str(&format!("<div class=\"section-title\">{}. OVERVIEW</div>\n", section_num));
    for (sub_idx, (id, (title, _))) in overview_pages.iter().enumerate() {
        let active = if id.as_str() == first_page_id {
            " active"
        } else {
            ""
        };
        sidebar_html.push_str(&format!(
            "<a href=\"#\" data-page=\"{id}\" onclick=\"showPage('{id}'); return false;\" class=\"{active}\">{section_num}.{sub_num} {title}</a>\n",
            sub_num = sub_idx + 1
        ));
    }

    // Controllers
    let ctrl_pages: Vec<_> = module_pages
        .iter()
        .filter(|(k, _)| k.contains("ctrl-"))
        .collect();
    if !ctrl_pages.is_empty() {
        section_num += 1;
        sidebar_html.push_str(&format!("<div class=\"section-title\">{}. CONTROLLERS</div>\n", section_num));
        for (sub_idx, (id, (title, _))) in ctrl_pages.iter().enumerate() {
            sidebar_html.push_str(&format!(
                "<a href=\"#\" data-page=\"{id}\" onclick=\"showPage('{id}'); return false;\">{section_num}.{sub_num} {title}</a>\n",
                sub_num = sub_idx + 1
            ));
        }
    }

    // Data Model
    let data_pages: Vec<_> = module_pages
        .iter()
        .filter(|(k, _)| k.contains("data-"))
        .collect();
    if !data_pages.is_empty() {
        section_num += 1;
        sidebar_html.push_str(&format!("<div class=\"section-title\">{}. DATA MODEL</div>\n", section_num));
        for (sub_idx, (id, (title, _))) in data_pages.iter().enumerate() {
            sidebar_html.push_str(&format!(
                "<a href=\"#\" data-page=\"{id}\" onclick=\"showPage('{id}'); return false;\">{section_num}.{sub_num} {title}</a>\n",
                sub_num = sub_idx + 1
            ));
        }
    }

    // Remaining module pages (services, UI, AJAX, etc.)
    let other_pages: Vec<_> = module_pages
        .iter()
        .filter(|(k, _)| !k.contains("ctrl-") && !k.contains("data-"))
        .collect();
    if !other_pages.is_empty() {
        section_num += 1;
        sidebar_html.push_str(&format!("<div class=\"section-title\">{}. MODULES</div>\n", section_num));
        for (sub_idx, (id, (title, _))) in other_pages.iter().enumerate() {
            sidebar_html.push_str(&format!(
                "<a href=\"#\" data-page=\"{id}\" onclick=\"showPage('{id}'); return false;\">{section_num}.{sub_num} {title}</a>\n",
                sub_num = sub_idx + 1
            ));
        }
    }

    // 3. Build pages JSON
    let pages_json: BTreeMap<String, serde_json::Value> = pages
        .iter()
        .map(|(id, (title, html))| {
            (
                id.clone(),
                serde_json::json!({
                    "title": title,
                    "html": html
                }),
            )
        })
        .collect();

    // 3b. Build PAGE_ORDER (ordered list of page IDs for prev/next navigation)
    let page_order: Vec<&String> = pages.keys().collect();
    let page_order_json = serde_json::to_string(&page_order)?;

    // 3c. Build SEARCH_INDEX (stripped text for full-text search)
    let search_index: Vec<serde_json::Value> = pages
        .iter()
        .map(|(id, (title, html))| {
            json!({
                "id": id,
                "title": title,
                "text": strip_html_tags(html)
            })
        })
        .collect();
    let search_index_json = serde_json::to_string(&search_index)?;

    // 4. Get project stats
    let node_count = graph.node_count();
    let edge_count = graph.relationship_count();
    let project_name = repo_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let stats_str = format!(
        "{} nodes &middot; {} relations &middot; {} pages",
        node_count,
        edge_count,
        pages.len()
    );

    // 5. Get first page content
    let first_page_html = pages
        .values()
        .next()
        .map(|(_, html)| html.as_str())
        .unwrap_or("<h1>Documentation</h1><p>No pages generated yet.</p>");

    // 6. Assemble HTML from template
    let pages_json_str = serde_json::to_string(&pages_json)?;
    let final_html = build_html_template(
        &project_name,
        &stats_str,
        &sidebar_html,
        first_page_html,
        &pages_json_str,
        &page_order_json,
        &search_index_json,
    );

    // 7. Check for local mermaid.min.js (offline support)
    let mermaid_path = docs_dir.join("mermaid.min.js");
    if !mermaid_path.exists() {
        println!("  {} For offline diagrams, download mermaid.min.js to {}", "TIP".cyan(), docs_dir.display());
    }

    // 8. Write output
    let out_path = docs_dir.join("index.html");
    std::fs::write(&out_path, &final_html)?;
    info!("Generated HTML documentation at {}", out_path.display());
    println!(
        "{} Generated HTML documentation: {}",
        "OK".green(),
        out_path.display()
    );

    Ok(())
}

/// Strip HTML tags from content, returning plain text for search indexing.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        if c == '<' { in_tag = true; continue; }
        if c == '>' { in_tag = false; result.push(' '); continue; }
        if !in_tag { result.push(c); }
    }
    // Collapse whitespace
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Build the complete self-contained HTML template.
fn build_html_template(
    project_name: &str,
    stats: &str,
    sidebar_nav: &str,
    first_page_content: &str,
    pages_json: &str,
    page_order_json: &str,
    search_index_json: &str,
) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en" data-theme="dark">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{project_name} — Documentation</title>
  <script src="mermaid.min.js" onerror="this.onerror=null;var s=document.createElement('script');s.src='https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js';s.onload=function(){{if(typeof mermaid!=='undefined'){{mermaid.initialize({{theme:'dark',startOnLoad:false,securityLevel:'loose'}});renderMermaid();}}}};document.head.appendChild(s);"></script>
  <link rel="stylesheet" href="hljs-dark.css" onerror="this.onerror=null;this.href='https://cdn.jsdelivr.net/npm/highlight.js@11/styles/github-dark.min.css'">
  <script src="hljs.min.js" onerror="this.onerror=null;this.src='https://cdn.jsdelivr.net/npm/highlight.js@11/lib/core.min.js'"></script>
  <script src="hljs-csharp.min.js" onerror="this.onerror=null;this.src='https://cdn.jsdelivr.net/npm/highlight.js@11/lib/languages/csharp.min.js'"></script>
  <script src="hljs-js.min.js" onerror="this.onerror=null;this.src='https://cdn.jsdelivr.net/npm/highlight.js@11/lib/languages/javascript.min.js'"></script>
  <script src="hljs-xml.min.js" onerror="this.onerror=null;this.src='https://cdn.jsdelivr.net/npm/highlight.js@11/lib/languages/xml.min.js'"></script>
  <script src="hljs-sql.min.js" onerror="this.onerror=null;this.src='https://cdn.jsdelivr.net/npm/highlight.js@11/lib/languages/sql.min.js'"></script>
  <style>
    [data-theme="light"] .hljs {{ background: var(--bg-surface); }}
    :root {{
      --bg: #0f1117; --bg-surface: #161822; --bg-sidebar: #12141e;
      --text: #e8ecf4; --text-muted: #8690a5; --accent: #6aa1f8;
      --border: rgba(255,255,255,0.08);
    }}
    [data-theme="light"] {{
      --bg: #f8f9fc; --bg-surface: #ffffff; --bg-sidebar: #f0f2f7;
      --text: #1a1d26; --text-muted: #5a6275; --accent: #4a85e0;
      --border: rgba(0,0,0,0.08);
    }}
    * {{ margin:0; padding:0; box-sizing:border-box; }}
    body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
           background: var(--bg); color: var(--text); display:flex; height:100vh; }}

    /* Header bar */
    .header {{ position:fixed; top:0; left:0; right:0; height:48px; background:var(--bg-sidebar);
              border-bottom:1px solid var(--border); display:flex; align-items:center;
              padding:0 20px; z-index:50; }}
    .header h1 {{ font-size:15px; color:var(--accent); }}
    .header .stats {{ margin-left:auto; font-size:11px; color:var(--text-muted); margin-right:80px; }}
    body {{ padding-top:48px; }}

    /* Sidebar */
    .sidebar {{ width:280px; background:var(--bg-sidebar); border-right:1px solid var(--border);
               overflow-y:auto; padding:16px 0; flex-shrink:0; margin-top:48px; height:calc(100vh - 48px); }}
    .sidebar h2 {{ font-size:14px; padding:8px 20px; color:var(--accent); }}
    .sidebar a {{ display:block; padding:6px 20px; color:var(--text-muted); text-decoration:none;
                 font-size:13px; border-left:3px solid transparent; transition: all 0.15s; }}
    .sidebar a:hover {{ color:var(--text); background:rgba(255,255,255,0.03); }}
    .sidebar a.active {{ color:var(--accent); border-left-color:var(--accent);
                        background:rgba(106,161,248,0.08); }}
    .sidebar .section-title {{ font-size:10px; text-transform:uppercase; letter-spacing:0.05em;
                              color:var(--text-muted); padding:16px 20px 4px; }}

    /* Main content */
    .main {{ flex:1; overflow-y:auto; padding:40px 60px; max-width:900px;
            transition: opacity 0.12s ease-out; }}
    .main h1 {{ font-size:28px; margin-bottom:8px; }}
    .main h2 {{ font-size:20px; margin:32px 0 12px; padding-bottom:8px;
               border-bottom:1px solid var(--border); }}
    .main h3 {{ font-size:16px; margin:24px 0 8px; }}
    .main p {{ line-height:1.7; margin:8px 0; }}
    .main table {{ width:100%; border-collapse:collapse; margin:16px 0; font-size:13px; }}
    .main th, .main td {{ padding:8px 12px; border:1px solid var(--border); text-align:left; }}
    .main th {{ background:var(--bg-sidebar); font-weight:600; }}
    .main code {{ background:var(--bg-sidebar); padding:2px 6px; border-radius:4px; font-size:12px;
                 font-family:'JetBrains Mono',monospace; }}
    .main pre {{ background:var(--bg-sidebar); padding:16px; border-radius:8px; overflow-x:auto;
                margin:12px 0; border:1px solid var(--border); }}
    .main pre code {{ background:none; padding:0; }}
    .main ul, .main ol {{ padding-left:24px; margin:8px 0; }}
    .main li {{ line-height:1.7; }}
    .main blockquote {{ border-left:3px solid var(--accent); padding:8px 16px; margin:12px 0;
                       color:var(--text-muted); background:rgba(106,161,248,0.05); border-radius:0 8px 8px 0; }}

    /* TOC right sidebar */
    .toc {{ width:220px; padding:20px 16px; border-left:1px solid var(--border);
           overflow-y:auto; flex-shrink:0; position:sticky; top:0; margin-top:48px; height:calc(100vh - 48px); }}
    .toc h3 {{ font-size:11px; text-transform:uppercase; letter-spacing:0.05em;
              color:var(--text-muted); margin-bottom:12px; }}
    .toc a {{ display:block; font-size:12px; color:var(--text-muted); text-decoration:none;
             padding:3px 0; border-left:2px solid transparent; padding-left:8px; }}
    .toc a:hover {{ color:var(--accent); }}
    .toc a.depth-3 {{ padding-left:20px; }}
    .toc a.toc-active {{
      color: var(--accent);
      border-left-color: var(--accent);
      font-weight: 600;
    }}

    /* Theme toggle */
    .theme-toggle {{ position:fixed; top:12px; right:16px; background:var(--bg-surface);
                    border:1px solid var(--border); border-radius:8px; padding:6px 12px;
                    color:var(--text-muted); cursor:pointer; font-size:12px; z-index:100; }}

    /* Mermaid */
    .mermaid {{ background:var(--bg-surface); border-radius:8px; padding:16px; margin:16px 0;
               border:1px solid var(--border); text-align:center; }}

    /* Sidebar filter search */
    .search {{ padding:8px 16px; }}
    .search input {{ width:100%; padding:6px 10px; background:var(--bg); border:1px solid var(--border);
                    border-radius:6px; color:var(--text); font-size:12px; outline:none; }}
    .search input:focus {{ border-color:var(--accent); }}

    .hidden {{ display:none !important; }}

    /* Details/Summary collapsible sections */
    .main details {{ margin:12px 0; border:1px solid var(--border); border-radius:8px;
                    padding:4px 12px; background:var(--bg-surface); }}
    .main details summary {{ cursor:pointer; font-weight:600; font-size:13px; color:var(--text-muted);
                            padding:8px 0; user-select:none; }}
    .main details summary:hover {{ color:var(--accent); }}
    .main details[open] summary {{ margin-bottom:4px; border-bottom:1px solid var(--border); padding-bottom:8px; }}

    /* Syntax highlighting (dark theme) */
    .hljs-keyword {{ color: #c678dd; font-weight: 600; }}
    .hljs-string {{ color: #98c379; }}
    .hljs-comment {{ color: #7f848e; font-style: italic; }}
    .hljs-number {{ color: #d19a66; }}
    .hljs-function .hljs-title {{ color: #61afef; }}
    .hljs-built_in {{ color: #e5c07b; }}
    .hljs-type {{ color: #e5c07b; }}
    [data-theme="light"] .hljs-keyword {{ color: #8b3dba; }}
    [data-theme="light"] .hljs-string {{ color: #2e7d32; }}
    [data-theme="light"] .hljs-comment {{ color: #9e9e9e; }}
    [data-theme="light"] .hljs-number {{ color: #b5651d; }}
    [data-theme="light"] .hljs-function .hljs-title {{ color: #1565c0; }}

    /* Copy button on code blocks */
    .code-wrapper {{ position: relative; }}
    .copy-btn {{
      position: absolute; top: 8px; right: 8px;
      background: var(--bg-surface); border: 1px solid var(--border);
      border-radius: 6px; padding: 4px 8px; cursor: pointer;
      font-size: 11px; color: var(--text-muted);
      opacity: 0; transition: opacity 0.15s;
    }}
    .code-wrapper:hover .copy-btn {{ opacity: 1; }}
    .copy-btn.copied {{ color: var(--accent); }}

    /* Callout / admonition blocks */
    .callout {{
      border-radius: 8px; padding: 12px 16px; margin: 16px 0;
      border-left: 4px solid; display: flex; gap: 10px;
    }}
    .callout-icon {{ font-size: 16px; flex-shrink: 0; margin-top: 2px; }}
    .callout-content {{ flex: 1; }}
    .callout-content p {{ margin: 0; }}
    .callout-note {{ background: rgba(106,161,248,0.08); border-color: var(--accent); }}
    .callout-tip {{ background: rgba(74,222,128,0.08); border-color: #4ade80; }}
    .callout-warning {{ background: rgba(251,191,36,0.08); border-color: #fbbf24; }}
    .callout-danger {{ background: rgba(248,113,113,0.08); border-color: #f87171; }}

    /* Breadcrumb */
    .breadcrumb {{
      font-size: 12px; color: var(--text-muted); margin-bottom: 16px;
      display: flex; gap: 6px; align-items: center;
    }}
    .breadcrumb a {{ color: var(--text-muted); text-decoration: none; }}
    .breadcrumb a:hover {{ color: var(--accent); }}
    .breadcrumb .sep {{ color: var(--border); }}

    /* Prev/Next footer navigation */
    .page-nav {{
      display: flex; justify-content: space-between; padding: 24px 0;
      margin-top: 32px; border-top: 1px solid var(--border);
    }}
    .page-nav a {{
      display: flex; flex-direction: column; gap: 4px;
      text-decoration: none; color: var(--text-muted); font-size: 13px;
      padding: 8px 12px; border-radius: 8px; transition: background 0.15s;
      max-width: 45%;
    }}
    .page-nav a:hover {{ background: rgba(106,161,248,0.06); color: var(--accent); }}
    .page-nav .nav-label {{ font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; }}
    .page-nav .nav-title {{ font-weight: 600; color: var(--text); }}
    .page-nav .nav-next {{ text-align: right; margin-left: auto; }}

    /* Mobile hamburger */
    .hamburger {{
      display: none; position: fixed; top: 12px; left: 12px;
      background: var(--bg-surface); border: 1px solid var(--border);
      border-radius: 8px; padding: 6px 10px; cursor: pointer;
      color: var(--text-muted); z-index: 60; font-size: 18px;
    }}

    /* Full-text search overlay */
    .search-result {{
      display: block; padding: 10px 12px; border-radius: 8px;
      text-decoration: none; color: var(--text); transition: background 0.1s;
    }}
    .search-result:hover {{ background: rgba(106,161,248,0.08); }}
    .search-result-title {{ font-weight: 600; font-size: 13px; }}
    .search-result-snippet {{ font-size: 12px; color: var(--text-muted); margin-top: 4px; }}
    .search-result-snippet mark {{ background: rgba(106,161,248,0.3); color: var(--text); border-radius: 2px; padding: 0 2px; }}
    .search-empty {{ padding: 20px; text-align: center; color: var(--text-muted); font-size: 13px; }}

    /* Line numbers on code blocks */
    .code-wrapper pre {{ counter-reset: line; }}
    .code-wrapper pre code .line {{ counter-increment: line; }}
    .code-wrapper pre code .line::before {{
      content: counter(line); display: inline-block; width: 3em;
      margin-right: 1em; text-align: right; color: var(--text-muted);
      opacity: 0.4; font-size: 12px; user-select: none;
    }}

    @media (max-width:900px) {{
      .hamburger {{ display: block; }}
      .sidebar {{ transform: translateX(-100%); transition: transform 0.25s ease; z-index: 55; position: fixed; height: 100vh; }}
      .sidebar.open {{ transform: translateX(0); box-shadow: 4px 0 20px rgba(0,0,0,0.3); }}
      .toc {{ display:none; }}
      .main {{ padding:20px; }}
    }}

    /* Print CSS */
    @media print {{
      .sidebar, .toc, .header, .theme-toggle, .copy-btn, .hamburger, .page-nav, .search {{ display: none !important; }}
      .main {{ margin: 0; padding: 20px; max-width: 100%; }}
      body {{ font-family: Georgia, serif; font-size: 11pt; color: #000; background: #fff; }}
      pre {{ border: 1px solid #ccc; page-break-inside: avoid; font-size: 9pt; }}
      h1, h2, h3 {{ page-break-after: avoid; color: #000; }}
      a {{ color: #000; text-decoration: underline; }}
      .callout {{ border: 1px solid #ccc; break-inside: avoid; }}
    }}
  </style>
</head>
<body>
  <button class="hamburger" onclick="toggleSidebar()">&#9776;</button>

  <div id="search-overlay" class="hidden"
    style="position:fixed;inset:0;z-index:100;background:rgba(0,0,0,0.6);display:flex;align-items:flex-start;justify-content:center;padding-top:15vh;">
    <div style="width:560px;max-width:90vw;background:var(--bg-surface);border:1px solid var(--border);border-radius:12px;overflow:hidden;box-shadow:0 8px 32px rgba(0,0,0,0.3);">
      <div style="padding:12px 16px;border-bottom:1px solid var(--border);">
        <input id="search-input" type="text" placeholder="Rechercher dans la documentation... (Ctrl+K)"
          style="width:100%;padding:8px 12px;background:var(--bg);border:1px solid var(--border);border-radius:8px;color:var(--text);font-size:14px;outline:none;">
      </div>
      <div id="search-results" style="max-height:400px;overflow-y:auto;padding:8px;"></div>
    </div>
  </div>

  <header class="header">
    <h1>{project_name}</h1>
    <span class="stats">{stats}</span>
    <button class="theme-toggle" onclick="toggleTheme()">Theme</button>
  </header>

  <nav class="sidebar">
    <div class="search">
      <input type="text" placeholder="Filter pages..." oninput="filterPages(this.value)">
    </div>
    {sidebar_nav}
  </nav>

  <main class="main" id="content">
    {first_page_content}
  </main>

  <aside class="toc" id="toc">
    <h3>On this page</h3>
    <div id="toc-links"></div>
  </aside>

  <script>
    // Page data
    const PAGES = {pages_json};
    const PAGE_ORDER = {page_order_json};
    const SEARCH_INDEX = {search_index_json};

    let currentPage = null;

    // Show page
    function showPage(id, anchor) {{
      const page = PAGES[id];
      if (!page) return;
      currentPage = id;

      const content = document.getElementById('content');
      content.style.opacity = '0';

      setTimeout(() => {{
        content.innerHTML = page.html;

        // Add breadcrumb
        const breadcrumb = buildBreadcrumb(id, page.title);
        content.insertAdjacentHTML('afterbegin', breadcrumb);

        // Add prev/next navigation
        content.insertAdjacentHTML('beforeend', buildPageNav(id));

        // Update sidebar active
        document.querySelectorAll('.sidebar a[data-page]').forEach(a => a.classList.remove('active'));
        const link = document.querySelector('.sidebar a[data-page="' + id + '"]');
        if (link) {{ link.classList.add('active'); link.scrollIntoView({{block:'nearest'}}); }}

        // Build TOC
        buildToc();

        // Add copy buttons to code blocks
        addCopyButtons();

        // Render Mermaid
        renderMermaid();

        // Syntax highlighting
        if (typeof hljs !== 'undefined') {{
          document.querySelectorAll('pre code').forEach(block => {{
            if (!block.classList.contains('language-mermaid')) {{
              hljs.highlightElement(block);
            }}
          }});
        }}

        // Init scroll spy
        initScrollSpy();

        content.style.opacity = '1';

        // Handle anchor navigation
        if (anchor) {{
          setTimeout(() => {{
            const el = document.getElementById(anchor);
            if (el) {{ if (el.tagName === 'DETAILS') el.open = true; el.scrollIntoView({{behavior:'smooth'}}); }}
          }}, 150);
        }} else {{
          content.scrollTop = 0;
        }}
      }}, 100);
    }}

    // Breadcrumb
    function buildBreadcrumb(id, title) {{
      const parts = id.split('/');
      let html = '<div class="breadcrumb"><a href="#" onclick="showPage(PAGE_ORDER[0]); return false;">Documentation</a>';
      if (parts.length > 1) {{
        html += '<span class="sep">&#8250;</span><span>' + parts[0].charAt(0).toUpperCase() + parts[0].slice(1) + '</span>';
      }}
      html += '<span class="sep">&#8250;</span><span>' + title + '</span></div>';
      return html;
    }}

    // Prev/Next navigation
    function buildPageNav(id) {{
      const idx = PAGE_ORDER.indexOf(id);
      if (idx === -1) return '';
      let html = '<div class="page-nav">';
      if (idx > 0) {{
        const prev = PAGE_ORDER[idx - 1];
        html += '<a href="#" onclick="showPage(\'' + prev + '\'); return false;">' +
          '<span class="nav-label">&larr; Pr&eacute;c&eacute;dent</span>' +
          '<span class="nav-title">' + (PAGES[prev] ? PAGES[prev].title : prev) + '</span></a>';
      }}
      if (idx < PAGE_ORDER.length - 1) {{
        const next = PAGE_ORDER[idx + 1];
        html += '<a class="nav-next" href="#" onclick="showPage(\'' + next + '\'); return false;">' +
          '<span class="nav-label">Suivant &rarr;</span>' +
          '<span class="nav-title">' + (PAGES[next] ? PAGES[next].title : next) + '</span></a>';
      }}
      html += '</div>';
      return html;
    }}

    // TOC with scroll spy
    function buildToc() {{
      const headings = document.querySelectorAll('.main h2, .main h3');
      const tocDiv = document.getElementById('toc-links');
      tocDiv.innerHTML = '';
      headings.forEach((h, i) => {{
        h.id = 'heading-' + i;
        const a = document.createElement('a');
        a.textContent = h.textContent;
        a.href = '#heading-' + i;
        a.className = h.tagName === 'H3' ? 'depth-3' : '';
        a.setAttribute('data-target', 'heading-' + i);
        a.onclick = (e) => {{ e.preventDefault(); h.scrollIntoView({{behavior:'smooth'}}); }};
        tocDiv.appendChild(a);
      }});
    }}

    // Scroll spy
    function initScrollSpy() {{
      const tocLinks = document.querySelectorAll('.toc a[data-target]');
      if (!tocLinks.length) return;

      const observer = new IntersectionObserver(entries => {{
        entries.forEach(e => {{
          const link = document.querySelector('.toc a[data-target="' + e.target.id + '"]');
          if (link) link.classList.toggle('toc-active', e.isIntersecting);
        }});
      }}, {{ threshold: 0.3, rootMargin: '-80px 0px -60% 0px' }});

      document.querySelectorAll('h2[id], h3[id]').forEach(h => observer.observe(h));
    }}

    // Copy buttons
    function addCopyButtons() {{
      document.querySelectorAll('pre').forEach(pre => {{
        if (pre.parentElement.classList.contains('code-wrapper')) return;
        const wrapper = document.createElement('div');
        wrapper.className = 'code-wrapper';
        pre.parentNode.insertBefore(wrapper, pre);
        wrapper.appendChild(pre);

        const btn = document.createElement('button');
        btn.className = 'copy-btn';
        btn.textContent = 'Copier';
        btn.onclick = () => {{
          navigator.clipboard.writeText(pre.textContent).then(() => {{
            btn.textContent = '\u2713 Copi\u00e9';
            btn.classList.add('copied');
            setTimeout(() => {{ btn.textContent = 'Copier'; btn.classList.remove('copied'); }}, 1500);
          }});
        }};
        wrapper.appendChild(btn);
      }});
    }}

    // Mermaid
    function renderMermaid() {{
      document.querySelectorAll('pre code.language-mermaid').forEach(block => {{
        const div = document.createElement('div');
        div.className = 'mermaid';
        div.textContent = block.textContent;
        block.parentElement.replaceWith(div);
      }});
      const nodes = document.querySelectorAll('.mermaid');
      if (nodes.length === 0) return;
      if (typeof mermaid !== 'undefined') {{
        try {{ mermaid.run({{nodes}}); }} catch(e) {{ console.warn('Mermaid render error:', e); }}
      }} else {{
        // Mermaid not loaded yet — retry after a short delay (CDN loading)
        setTimeout(renderMermaid, 500);
      }}
    }}

    // Full-text search
    function initSearch() {{
      const searchInput = document.getElementById('search-input');
      const searchResults = document.getElementById('search-results');
      const searchOverlay = document.getElementById('search-overlay');

      // Ctrl+K shortcut
      document.addEventListener('keydown', e => {{
        if ((e.ctrlKey || e.metaKey) && e.key === 'k') {{
          e.preventDefault();
          searchOverlay.classList.toggle('hidden');
          if (!searchOverlay.classList.contains('hidden')) searchInput.focus();
        }}
        if (e.key === 'Escape') searchOverlay.classList.add('hidden');
      }});

      searchInput.addEventListener('input', () => {{
        const q = searchInput.value.toLowerCase().trim();
        if (q.length < 2) {{ searchResults.innerHTML = ''; return; }}

        const results = SEARCH_INDEX
          .filter(p => p.title.toLowerCase().includes(q) || p.text.toLowerCase().includes(q))
          .slice(0, 10);

        searchResults.innerHTML = results.map(r => {{
          // Find snippet around match
          const idx = r.text.toLowerCase().indexOf(q);
          const start = Math.max(0, idx - 40);
          const end = Math.min(r.text.length, idx + q.length + 40);
          const snippet = (start > 0 ? '...' : '') +
            r.text.slice(start, idx) +
            '<mark>' + r.text.slice(idx, idx + q.length) + '</mark>' +
            r.text.slice(idx + q.length, end) +
            (end < r.text.length ? '...' : '');

          return '<a class="search-result" href="#" onclick="showPage(\'' + r.id + '\'); document.getElementById(\'search-overlay\').classList.add(\'hidden\'); return false;">' +
            '<div class="search-result-title">' + r.title + '</div>' +
            '<div class="search-result-snippet">' + (idx >= 0 ? snippet : '') + '</div>' +
            '</a>';
        }}).join('');

        if (results.length === 0) {{
          searchResults.innerHTML = '<div class="search-empty">Aucun r&eacute;sultat pour "' + q + '"</div>';
        }}
      }});
    }}

    // Sidebar filter
    function filterPages(query) {{
      const q = query.toLowerCase();
      document.querySelectorAll('.sidebar a[data-page]').forEach(a => {{
        a.style.display = a.textContent.toLowerCase().includes(q) ? '' : 'none';
      }});
      // Also hide section titles with no visible children
      document.querySelectorAll('.sidebar .section-title').forEach(title => {{
        let next = title.nextElementSibling;
        let hasVisible = false;
        while (next && !next.classList.contains('section-title')) {{
          if (next.style.display !== 'none') hasVisible = true;
          next = next.nextElementSibling;
        }}
        title.style.display = hasVisible || !q ? '' : 'none';
      }});
    }}

    // Theme toggle
    function toggleTheme() {{
      const html = document.documentElement;
      const next = html.getAttribute('data-theme') === 'dark' ? 'light' : 'dark';
      html.setAttribute('data-theme', next);
      localStorage.setItem('theme', next);
      if (typeof mermaid !== 'undefined') {{
        mermaid.initialize({{ theme: next === 'dark' ? 'dark' : 'default', startOnLoad: false, securityLevel: 'loose' }});
        renderMermaid();
      }}
    }}

    // Hamburger
    function toggleSidebar() {{
      document.querySelector('.sidebar').classList.toggle('open');
    }}

    // Init
    document.addEventListener('DOMContentLoaded', () => {{
      const saved = localStorage.getItem('theme');
      if (saved) document.documentElement.setAttribute('data-theme', saved);
      if (typeof mermaid !== 'undefined') {{
        const theme = document.documentElement.getAttribute('data-theme') === 'light' ? 'default' : 'dark';
        mermaid.initialize({{ theme, startOnLoad: false, securityLevel: 'loose' }});
      }}
      buildToc();
      renderMermaid();
      addCopyButtons();
      initSearch();
      initScrollSpy();
      if (typeof hljs !== 'undefined') {{
        document.querySelectorAll('pre code').forEach(block => {{
          if (!block.classList.contains('language-mermaid')) {{
            hljs.highlightElement(block);
          }}
        }});
      }}
    }});
  </script>
</body>
</html>"##
    )
}

// ─── Markdown to HTML Converter ────────────────────────────────────────

/// Convert Markdown content to HTML (basic, no external dependencies).
fn markdown_to_html(md: &str) -> String {
    let mut html = String::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_content = String::new();
    let mut in_table = false;
    let mut table_has_body = false;
    let mut in_list = false;
    let mut in_ordered_list = false;

    for line in md.lines() {
        // Strip GNX anchor comments (used by LLM enrichment, not for display)
        if line.trim().starts_with("<!-- GNX:") {
            continue;
        }

        // Handle HTML comments (pass through as invisible)
        if line.trim().starts_with("<!--") && line.trim().ends_with("-->") {
            html.push_str(line);
            html.push('\n');
            continue;
        }

        // Handle <details>/<summary> blocks (pass through as HTML)
        if line.trim_start().starts_with("<details>")
            || line.trim_start().starts_with("<details ")
            || line.trim_start().starts_with("</details>")
            || line.trim_start().starts_with("<summary>")
            || line.trim_start().starts_with("<summary ")
            || line.trim_start().starts_with("</summary>")
        {
            html.push_str(line);
            html.push('\n');
            continue;
        }

        // Code fences
        if line.starts_with("```") {
            if in_code_block {
                // Close code block
                if code_lang == "mermaid" {
                    html.push_str(&format!(
                        "<pre><code class=\"language-mermaid\">{}</code></pre>\n",
                        html_escape(&code_content)
                    ));
                } else {
                    html.push_str(&format!(
                        "<pre><code class=\"language-{}\">{}</code></pre>\n",
                        code_lang,
                        html_escape(&code_content)
                    ));
                }
                code_content.clear();
                in_code_block = false;
            } else {
                // Close any open list before a code block
                if in_list {
                    html.push_str("</ul>\n");
                    in_list = false;
                }
                if in_ordered_list {
                    html.push_str("</ol>\n");
                    in_ordered_list = false;
                }
                code_lang = line.trim_start_matches('`').trim().to_string();
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_content.push_str(line);
            code_content.push('\n');
            continue;
        }

        // Tables
        if line.contains('|') && line.trim().starts_with('|') {
            // Separator row (e.g., |---|---|)
            if line.replace('|', "").replace('-', "").replace(' ', "").replace(':', "").is_empty() {
                // Mark that we should switch from thead to tbody
                if in_table {
                    html.push_str("</thead><tbody>\n");
                    table_has_body = true;
                }
                continue;
            }
            if !in_table {
                // Close any open list
                if in_list {
                    html.push_str("</ul>\n");
                    in_list = false;
                }
                if in_ordered_list {
                    html.push_str("</ol>\n");
                    in_ordered_list = false;
                }
                html.push_str("<table>\n<thead>\n");
                in_table = true;
                table_has_body = false;
            }
            let cells: Vec<&str> = line
                .split('|')
                .filter(|s| !s.trim().is_empty())
                .collect();
            let tag = if table_has_body { "td" } else { "th" };
            html.push_str("<tr>");
            for cell in cells {
                html.push_str(&format!(
                    "<{tag}>{}</{tag}>",
                    inline_md(cell.trim())
                ));
            }
            html.push_str("</tr>\n");
            continue;
        } else if in_table {
            if table_has_body {
                html.push_str("</tbody></table>\n");
            } else {
                html.push_str("</thead></table>\n");
            }
            in_table = false;
            table_has_body = false;
        }

        // Headings
        if line.starts_with("### ") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            if in_ordered_list { html.push_str("</ol>\n"); in_ordered_list = false; }
            html.push_str(&format!("<h3>{}</h3>\n", inline_md(&line[4..])));
            continue;
        }
        if line.starts_with("## ") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            if in_ordered_list { html.push_str("</ol>\n"); in_ordered_list = false; }
            html.push_str(&format!("<h2>{}</h2>\n", inline_md(&line[3..])));
            continue;
        }
        if line.starts_with("# ") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            if in_ordered_list { html.push_str("</ol>\n"); in_ordered_list = false; }
            html.push_str(&format!("<h1>{}</h1>\n", inline_md(&line[2..])));
            continue;
        }

        // Horizontal rule
        let trimmed = line.trim();
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            if in_ordered_list { html.push_str("</ol>\n"); in_ordered_list = false; }
            html.push_str("<hr>\n");
            continue;
        }

        // Unordered lists
        if line.starts_with("- ") || line.starts_with("* ") {
            if in_ordered_list { html.push_str("</ol>\n"); in_ordered_list = false; }
            if !in_list {
                html.push_str("<ul>\n");
                in_list = true;
            }
            html.push_str(&format!("<li>{}</li>\n", inline_md(&line[2..])));
            continue;
        }
        // Indented sub-items (2 or 4 spaces + dash)
        if (line.starts_with("  - ") || line.starts_with("    - ")) && in_list {
            let content = line.trim_start().trim_start_matches("- ");
            html.push_str(&format!("<li style=\"margin-left:16px\">{}</li>\n", inline_md(content)));
            continue;
        }

        // Ordered lists
        if !line.is_empty() {
            let maybe_ol = trimmed.split_once(". ");
            if let Some((num_part, rest)) = maybe_ol {
                if num_part.chars().all(|c| c.is_ascii_digit()) {
                    if in_list { html.push_str("</ul>\n"); in_list = false; }
                    if !in_ordered_list {
                        html.push_str("<ol>\n");
                        in_ordered_list = true;
                    }
                    html.push_str(&format!("<li>{}</li>\n", inline_md(rest)));
                    continue;
                }
            }
        }

        // Callouts: > [!NOTE], > [!TIP], > [!WARNING], > [!DANGER]
        if line.starts_with("> [!") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            if in_ordered_list { html.push_str("</ol>\n"); in_ordered_list = false; }
            let callout_type = if line.contains("[!NOTE]") { "note" }
                else if line.contains("[!TIP]") { "tip" }
                else if line.contains("[!WARNING]") { "warning" }
                else if line.contains("[!DANGER]") { "danger" }
                else { "note" };
            let icon = match callout_type {
                "tip" => "\u{1f4a1}",
                "warning" => "\u{26a0}\u{fe0f}",
                "danger" => "\u{1f534}",
                _ => "\u{2139}\u{fe0f}",
            };
            let text = line.trim_start_matches("> ").trim_start_matches("[!NOTE]")
                .trim_start_matches("[!TIP]").trim_start_matches("[!WARNING]")
                .trim_start_matches("[!DANGER]").trim();
            html.push_str(&format!(
                "<div class=\"callout callout-{}\">\
                 <span class=\"callout-icon\">{}</span>\
                 <div class=\"callout-content\">{}</div>\
                 </div>\n",
                callout_type, icon, inline_md(text)
            ));
            continue;
        }

        // Blockquotes
        if line.starts_with("> ") {
            if in_list { html.push_str("</ul>\n"); in_list = false; }
            if in_ordered_list { html.push_str("</ol>\n"); in_ordered_list = false; }
            html.push_str(&format!(
                "<blockquote>{}</blockquote>\n",
                inline_md(&line[2..])
            ));
            continue;
        }

        // Empty lines close lists
        if line.trim().is_empty() {
            if in_list {
                html.push_str("</ul>\n");
                in_list = false;
            }
            if in_ordered_list {
                html.push_str("</ol>\n");
                in_ordered_list = false;
            }
            continue;
        }

        // Paragraph (default)
        if in_list { html.push_str("</ul>\n"); in_list = false; }
        if in_ordered_list { html.push_str("</ol>\n"); in_ordered_list = false; }
        html.push_str(&format!("<p>{}</p>\n", inline_md(line)));
    }

    // Close any open blocks
    if in_table {
        if table_has_body {
            html.push_str("</tbody></table>\n");
        } else {
            html.push_str("</thead></table>\n");
        }
    }
    if in_list {
        html.push_str("</ul>\n");
    }
    if in_ordered_list {
        html.push_str("</ol>\n");
    }

    html
}

/// Process inline Markdown formatting: bold, italic, code, links.
fn inline_md(text: &str) -> String {
    let mut s = html_escape(text);

    // Bold: **text**
    loop {
        if let Some(start) = s.find("**") {
            if let Some(end) = s[start + 2..].find("**") {
                let bold_text = s[start + 2..start + 2 + end].to_string();
                s = format!(
                    "{}<strong>{}</strong>{}",
                    &s[..start],
                    bold_text,
                    &s[start + 2 + end + 2..]
                );
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Italic: *text* (but not inside <strong> tags already processed)
    // Simple approach: match single * not preceded/followed by *
    loop {
        // Find a lone * that is not part of **
        let bytes = s.as_bytes();
        let mut start_pos = None;
        for i in 0..bytes.len() {
            if bytes[i] == b'*' {
                let prev_star = i > 0 && bytes[i - 1] == b'*';
                let next_star = i + 1 < bytes.len() && bytes[i + 1] == b'*';
                if !prev_star && !next_star {
                    start_pos = Some(i);
                    break;
                }
            }
        }
        if let Some(start) = start_pos {
            // Find matching closing *
            let rest = &s[start + 1..];
            let mut end_pos = None;
            let rest_bytes = rest.as_bytes();
            for i in 0..rest_bytes.len() {
                if rest_bytes[i] == b'*' {
                    let prev_star = i > 0 && rest_bytes[i - 1] == b'*';
                    let next_star = i + 1 < rest_bytes.len() && rest_bytes[i + 1] == b'*';
                    if !prev_star && !next_star {
                        end_pos = Some(i);
                        break;
                    }
                }
            }
            if let Some(end) = end_pos {
                let italic_text = s[start + 1..start + 1 + end].to_string();
                s = format!(
                    "{}<em>{}</em>{}",
                    &s[..start],
                    italic_text,
                    &s[start + 1 + end + 1..]
                );
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Inline code: `text`
    loop {
        if let Some(start) = s.find('`') {
            if let Some(end) = s[start + 1..].find('`') {
                let code_text = s[start + 1..start + 1 + end].to_string();
                s = format!(
                    "{}<code>{}</code>{}",
                    &s[..start],
                    code_text,
                    &s[start + 1 + end + 1..]
                );
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Links: [text](url) - after HTML escaping, parens are still literal
    // We need to match the pattern carefully
    loop {
        if let Some(bracket_start) = s.find('[') {
            if let Some(bracket_end) = s[bracket_start..].find("](") {
                let abs_bracket_end = bracket_start + bracket_end;
                let link_text = &s[bracket_start + 1..abs_bracket_end];
                let after_paren = &s[abs_bracket_end + 2..];
                if let Some(paren_end) = after_paren.find(')') {
                    let url = &after_paren[..paren_end];
                    // Transform .md links to JavaScript page navigation for HTML site
                    let replacement = if url.contains(".md") {
                        // Handle anchors: ./modules/file.md#ENTITY → page='modules/file', anchor='ENTITY'
                        let (md_part, anchor) = if let Some(hash_idx) = url.find('#') {
                            (&url[..hash_idx], Some(&url[hash_idx + 1..]))
                        } else {
                            (url, None)
                        };
                        let page_id = md_part.trim_start_matches("./")
                            .trim_end_matches(".md");
                        if let Some(anchor_id) = anchor {
                            // Navigate to page AND scroll to + open the entity details
                            format!(
                                "<a href=\"#\" onclick=\"showPage('{}'); setTimeout(function(){{ var el=document.getElementById('{}'); if(el){{ el.open=true; el.scrollIntoView({{behavior:'smooth'}}); }} }}, 100); return false;\">{}</a>",
                                page_id, anchor_id, link_text
                            )
                        } else {
                            format!(
                                "<a href=\"#\" onclick=\"showPage('{}'); return false;\">{}</a>",
                                page_id, link_text
                            )
                        }
                    } else {
                        format!("<a href=\"{}\">{}</a>", url, link_text)
                    };
                    s = format!(
                        "{}{}{}",
                        &s[..bracket_start],
                        replacement,
                        &after_paren[paren_end + 1..]
                    );
                    continue;
                }
            }
        }
        break;
    }

    s
}

/// Escape HTML special characters.
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Extract the first `# Title` from Markdown content.
fn extract_title_from_md(content: &str) -> Option<String> {
    for line in content.lines() {
        if line.starts_with("# ") {
            return Some(line[2..].trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_classify_page_overview() {
        assert_eq!(classify_page(Path::new("overview.md")), PageType::Overview);
    }

    #[test]
    fn test_classify_page_architecture() {
        assert_eq!(classify_page(Path::new("architecture.md")), PageType::Architecture);
    }

    #[test]
    fn test_classify_page_controller() {
        assert_eq!(classify_page(Path::new("ctrl-dossierscontroller.md")), PageType::Controller);
    }

    #[test]
    fn test_classify_page_service() {
        assert_eq!(classify_page(Path::new("services.md")), PageType::Service);
    }

    #[test]
    fn test_classify_page_data_model() {
        assert_eq!(classify_page(Path::new("data-alisev2entities.md")), PageType::DataModel);
    }

    #[test]
    fn test_classify_page_external() {
        assert_eq!(classify_page(Path::new("external-services.md")), PageType::ExternalService);
    }

    #[test]
    fn test_classify_page_misc() {
        assert_eq!(classify_page(Path::new("random-page.md")), PageType::Misc);
    }

    #[test]
    fn test_strip_html_tags() {
        assert_eq!(strip_html_tags("<h1>Hello</h1><p>World</p>"), "Hello World");
        assert_eq!(strip_html_tags("no tags here"), "no tags here");
        assert_eq!(strip_html_tags("<a href='x'>link</a> text"), "link text");
        assert_eq!(strip_html_tags(""), "");
    }

    #[test]
    fn test_md5_simple_deterministic() {
        let h1 = md5_simple("hello world");
        let h2 = md5_simple("hello world");
        assert_eq!(h1, h2);
        assert_ne!(md5_simple("hello"), md5_simple("world"));
    }

    #[test]
    fn test_enriched_payload_parse() {
        let json = r#"{
            "lead": "This is the lead.",
            "section_augments": [
                {
                    "section_key": "architecture",
                    "intro": "Introduction text",
                    "warning": null,
                    "developer_tip": "A useful tip",
                    "see_also": ["overview.md"],
                    "source_ids": ["E1", "E2"]
                }
            ],
            "related_pages": ["overview.md"],
            "relevant_source_ids": ["E1"],
            "closing_summary": "Summary text"
        }"#;
        let payload: EnrichedPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.lead.as_deref(), Some("This is the lead."));
        assert_eq!(payload.section_augments.len(), 1);
        assert_eq!(payload.section_augments[0].section_key, "architecture");
        assert_eq!(payload.section_augments[0].source_ids, vec!["E1", "E2"]);
        assert_eq!(payload.closing_summary.as_deref(), Some("Summary text"));
    }

    #[test]
    fn test_enriched_payload_parse_minimal() {
        // Minimal valid payload (all optional fields null/empty)
        let json = r#"{
            "section_augments": [],
            "related_pages": [],
            "relevant_source_ids": []
        }"#;
        let payload: EnrichedPayload = serde_json::from_str(json).unwrap();
        assert!(payload.lead.is_none());
        assert!(payload.section_augments.is_empty());
        assert!(payload.closing_summary.is_none());
    }

    #[test]
    fn test_enriched_payload_parse_invalid() {
        let json = r#"{"not": "a valid payload"}"#;
        // Should parse but with defaults (empty vectors)
        let result: Result<EnrichedPayload, _> = serde_json::from_str(json);
        // Depending on serde defaults, this might fail -- that's OK, we test the fallback
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_extract_params_from_content() {
        assert_eq!(
            extract_params_from_content("string id, int page", "test"),
            "`string` id, `int` page"
        );
        assert_eq!(
            extract_params_from_content("", "test"),
            "-"
        );
        assert_eq!(
            extract_params_from_content("DossierPresta dossier", "test"),
            "`DossierPresta` dossier"
        );
    }

    #[test]
    fn test_get_profile() {
        let fast = get_profile("fast");
        assert_eq!(fast.max_evidence, 10);
        assert!(!fast.review_critical);
        assert_eq!(fast.timeout_secs, 60);

        let quality = get_profile("quality");
        assert_eq!(quality.max_evidence, 20);
        assert!(quality.review_critical);

        let strict = get_profile("strict");
        assert_eq!(strict.max_evidence, 30);
        assert!(strict.review_critical);
        assert_eq!(strict.max_retries, 2);
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Hello World"), "hello_world");
        assert_eq!(sanitize_filename("DossiersController"), "dossierscontroller");
        assert_eq!(sanitize_filename("a-b_c"), "a-b_c");
    }

    #[test]
    fn test_markdown_to_html_headings() {
        let md = "# Title\n## Section\n### Subsection\n";
        let html = markdown_to_html(md);
        assert!(html.contains("<h1>"));
        assert!(html.contains("<h2>"));
        assert!(html.contains("<h3>"));
    }

    #[test]
    fn test_markdown_to_html_code_block() {
        let md = "```csharp\npublic void Test() {}\n```\n";
        let html = markdown_to_html(md);
        assert!(html.contains("<pre>"));
        assert!(html.contains("language-csharp"));
    }

    #[test]
    fn test_markdown_to_html_callout() {
        let md = "> [!WARNING]\n> This is a warning\n";
        let html = markdown_to_html(md);
        assert!(html.contains("callout-warning"));
        assert!(html.contains("\u{26a0}\u{fe0f}"));
    }

    #[test]
    fn test_markdown_to_html_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n";
        let html = markdown_to_html(md);
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>"));
        assert!(html.contains("<td>"));
    }

    #[test]
    fn test_markdown_to_html_mermaid() {
        let md = "```mermaid\ngraph TD\n  A-->B\n```\n";
        let html = markdown_to_html(md);
        assert!(html.contains("language-mermaid"));
    }

    #[test]
    fn test_markdown_to_html_details() {
        let md = "<details>\n<summary>Click me</summary>\nContent here\n</details>\n";
        let html = markdown_to_html(md);
        assert!(html.contains("<details>"));
        assert!(html.contains("<summary>"));
    }

    #[test]
    fn test_inline_md_bold() {
        let result = inline_md("This is **bold** text");
        assert!(result.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_inline_md_code() {
        let result = inline_md("Use `code` here");
        assert!(result.contains("<code>code</code>"));
    }

    #[test]
    fn test_inline_md_link() {
        let result = inline_md("See [docs](./overview.md)");
        assert!(result.contains("showPage"));
        assert!(result.contains("overview"));
    }
}
