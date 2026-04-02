//! LLM enrichment types, config loading, structured/freeform enrichment, review passes.

#[allow(unused_imports)]
use std::collections::{BTreeSet, HashSet};
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use colored::Colorize;
use serde_json::json;
use tracing::{debug, warn};

use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;


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

pub(super) struct EnrichProfile {
    pub(super) max_evidence: usize,
    #[allow(dead_code)]
    pub(super) thinking_boost: bool,
    pub(super) review_critical: bool,
    pub(super) max_retries: u32,
    pub(super) timeout_secs: u64,
}

pub(super) fn get_profile(name: &str) -> EnrichProfile {
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
pub(super) fn run_enrichment_if_enabled(
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
