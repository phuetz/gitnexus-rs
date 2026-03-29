//! The `generate` command: produces AI context files (AGENTS.md, wiki/, skills/) from the knowledge graph.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::Write;
use std::path::Path;

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

pub fn run(what: &str, path: Option<&str>) -> Result<()> {
    let repo_path = Path::new(path.unwrap_or(".")).canonicalize()?;
    let storage = repo_manager::get_storage_paths(&repo_path);
    let snap_path = snapshot::snapshot_path(&storage.storage_path);
    let graph = snapshot::load_snapshot(&snap_path)?;

    info!("Generating {} for {}", what, repo_path.display());

    match what {
        TARGET_CONTEXT | TARGET_AGENTS => generate_agents_md(&graph, &repo_path)?,
        TARGET_WIKI => generate_wiki(&graph, &repo_path)?,
        TARGET_SKILLS => generate_skills(&graph, &repo_path)?,
        TARGET_DOCS => generate_docs(&graph, &repo_path)?,
        TARGET_DOCX => {
            // Generate Markdown first, then convert to DOCX
            generate_docs(&graph, &repo_path)?;
            let docs_dir = repo_path.join(".gitnexus").join("docs");
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
            // Generate Markdown first, then convert to HTML site
            generate_docs(&graph, &repo_path)?;
            generate_html_site(&graph, &repo_path)?;
        }
        TARGET_ALL => {
            generate_agents_md(&graph, &repo_path)?;
            generate_wiki(&graph, &repo_path)?;
            generate_skills(&graph, &repo_path)?;
            generate_docs(&graph, &repo_path)?;
            // Also generate DOCX
            let docs_dir = repo_path.join(".gitnexus").join("docs");
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
    )?;

    // 5. Generate ASP.NET MVC specific documentation (if applicable)
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

    // Total page count: 3 static pages + module pages + ASP.NET pages
    let total_pages = 3 + module_page_count + aspnet_pages.len();
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

    // Build module children
    let mut module_children = Vec::new();
    for info in communities.values() {
        let filename = sanitize_filename(&info.label);
        module_children.push(json!({
            "id": format!("mod-{}", filename),
            "title": info.label,
            "path": format!("modules/{}.md", filename),
            "icon": "box"
        }));
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
            "id": "architecture",
            "title": "Architecture",
            "path": "architecture.md",
            "icon": "git-branch"
        }),
        json!({
            "id": "getting-started",
            "title": "Getting Started",
            "path": "getting-started.md",
            "icon": "book-open"
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

    // Summary
    // Count total pages: 3 static + communities + controller pages + data pages + services + ui + ajax
    let ctrl_pages = controller_count;
    let data_pages = label_counts.get(&NodeLabel::DbContext).copied().unwrap_or(0);
    let svc_page = if service_count > 0 { 1 } else { 0 };
    let ui_page = if ui_count > 0 { 1 } else { 0 };
    let ajax_page = if label_counts.get(&NodeLabel::AjaxCall).copied().unwrap_or(0) > 0 { 1 } else { 0 };
    let total_pages = 3 + communities.len() + ctrl_pages + data_pages + svc_page + ui_page + ajax_page;

    writeln!(f, "## Summary")?;
    writeln!(f)?;
    writeln!(
        f,
        "This documentation covers {} pages organized into sections:",
        total_pages
    )?;
    writeln!(f, "Overview, Architecture, Getting Started, Modules")?;
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

    // Module Details (communities) - always show
    if !communities.is_empty() {
        writeln!(f, "## Module Details")?;
        writeln!(f)?;
        for info in communities.values() {
            writeln!(f, "### {}", info.label)?;
            if let Some(desc) = &info.description {
                writeln!(f, "{}", desc)?;
            } else {
                writeln!(f, "Module with {} members.", info.member_ids.len())?;
            }
            writeln!(f)?;
            writeln!(f, "- **Members**: {}", info.member_ids.len())?;

            let entry_points: Vec<&GraphNode> = info
                .member_ids
                .iter()
                .filter_map(|mid| graph.get_node(mid))
                .filter(|n| n.properties.entry_point_score.map(|s| s > 0.3).unwrap_or(false))
                .collect();
            if !entry_points.is_empty() {
                writeln!(f, "- **Entry Points**: {}", entry_points.len())?;
            }
            writeln!(f)?;
        }
    }

    // Key Entry Points
    let mut all_entry_points: Vec<(&GraphNode, f64)> = graph
        .iter_nodes()
        .filter_map(|n| {
            n.properties
                .entry_point_score
                .filter(|&s| s > 0.3)
                .map(|s| (n, s))
        })
        .collect();
    all_entry_points.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if !all_entry_points.is_empty() {
        writeln!(f, "## Key Entry Points")?;
        writeln!(f)?;
        for (node, score) in all_entry_points.iter().take(20) {
            let reason = node.properties.entry_point_reason.as_deref().unwrap_or("");
            writeln!(
                f,
                "- **{}** in `{}` (score: {:.2})",
                node.properties.name, node.properties.file_path, score
            )?;
            if !reason.is_empty() {
                writeln!(f, "  - {}", reason)?;
            }
        }
        writeln!(f)?;
    }

    // Execution Flows
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
                "- **{}**: {} steps ({})",
                proc_node.properties.name, step_count, ptype
            )?;
            if let Some(desc) = &proc_node.properties.description {
                writeln!(f, "  - {}", desc)?;
            }
        }
        writeln!(f)?;
    }

    // Summary / Navigation
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
    communities: &BTreeMap<String, CommunityInfo>,
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

    writeln!(f, "# Getting Started")?;
    writeln!(f)?;
    write!(f, "{}", source_files_section(&ep_file_refs))?;
    writeln!(f, "Welcome to the **{}** codebase!", repo_name)?;
    writeln!(f)?;

    // Project Structure
    writeln!(f, "## Project Structure")?;
    writeln!(f)?;

    // Infer folder structure from module file paths
    let mut folder_info: BTreeMap<String, usize> = BTreeMap::new();
    for node in graph.iter_nodes() {
        if node.label == NodeLabel::File {
            let path = &node.properties.file_path;
            if !path.is_empty() {
                if let Some(parent) = Path::new(path).parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    *folder_info.entry(parent_str).or_insert(0) += 1;
                }
            }
        }
    }

    writeln!(f, "The codebase is organized as follows:")?;
    writeln!(f)?;
    let mut folders: Vec<_> = folder_info.iter().collect();
    folders.sort();
    for (folder, count) in folders.iter().take(10) {
        writeln!(f, "- `{}` - {} files", folder, count)?;
    }
    writeln!(f)?;

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
        writeln!(f, "## Key Entry Points")?;
        writeln!(f)?;
        writeln!(f, "Start exploring from these main entry points:")?;
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

    // Module Map
    writeln!(f, "## Module Map")?;
    writeln!(f)?;
    for info in communities.values() {
        let filename = sanitize_filename(&info.label);
        writeln!(f, "- **[{}](modules/{}.md)** - {} members", info.label, filename, info.member_ids.len())?;
        if let Some(desc) = &info.description {
            writeln!(f, "  - {}", desc)?;
        }
    }
    writeln!(f)?;

    // Navigation Tips
    writeln!(f, "## Navigation Tips")?;
    writeln!(f)?;
    writeln!(f, "- Use the **Modules** section in the navigation to explore specific components")?;
    writeln!(f, "- Check the **Architecture** page to understand module dependencies")?;
    writeln!(f, "- Each module page shows entry points, call graphs, and file locations")?;
    writeln!(f, "- Look for symbols with high entry point scores as starting points for understanding flows")?;
    writeln!(f)?;

    writeln!(f, "**See also:** [Overview](./overview.md) · [Architecture](./architecture.md)")?;
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
                        // Extract just the filename from file_path
                        src_node.properties.file_path.rsplit(['/', '\\']).next()
                            .unwrap_or(&src_node.properties.name).to_string()
                    }
                    _ => src_node.properties.name.clone(),
                };
                let type_str = match src_node.label {
                    NodeLabel::View => "View".to_string(),
                    NodeLabel::PartialView => "Partial".to_string(),
                    NodeLabel::AjaxCall => {
                        let ajax_method = src_node.properties.ajax_method.as_deref().unwrap_or("AJAX");
                        format!("Script ({})", ajax_method)
                    }
                    _ => format!("{:?}", src_node.label),
                };
                (label_str, type_str)
            } else {
                let short = r.source_id.rsplit(':').next().unwrap_or(&r.source_id).to_string();
                (short, "Unknown".to_string())
            };
            action_callers.entry(r.target_id.clone()).or_default().push(short_name);
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
        content.push_str(&format!("## Actions ({})\n\n", action_count));
        content.push_str("| # | Action | Method | Paramètres | Retour | Appelé par |\n");
        content.push_str("|---|--------|--------|-----------|--------|------------|\n");
        for (i, action) in actions.iter().enumerate() {
            let method = action.properties.http_method.as_deref().unwrap_or("GET");
            let ret = action.properties.return_type.as_deref().unwrap_or("ActionResult");

            // Extract parameter signature from content (the actual method source code)
            let params = extract_params_from_content(
                action.properties.description.as_deref().unwrap_or(""),
                &action.properties.name,
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
                        content.push_str(&format!("**Called by:** {}\n", caller_strs.join(", ")));
                    }
                }

                content.push_str("\n</details>\n\n");
            }
        }

        // Summary
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
        content.push_str(&source_files_section(&src_file_refs));
        content.push_str(&format!("**File:** `{}`\n\n", ctx.properties.file_path));
        content.push_str(&format!("**Entities:** {}\n\n", entities.len()));

        content.push_str("## Entities\n\n");
        content.push_str("| Entity | File | Properties |\n");
        content.push_str("|--------|------|------------|\n");
        for entity in &entities {
            let props = entity.properties.description.as_deref().unwrap_or("-");
            content.push_str(&format!("| {} | `{}` | {} |\n",
                entity.properties.name, entity.properties.file_path, props));
        }
        content.push('\n');

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
        content.push_str("| Service | Type | Interface | Used By | File |\n");
        content.push_str("|---------|------|-----------|---------|------|\n");
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
            content.push_str(&format!("| {} | {} | {} | {} | `{}` |\n",
                svc.properties.name, layer, iface, used_by, svc.properties.file_path));
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
        let get_actions: Vec<&&GraphNode> = actions.iter()
            .filter(|a| a.properties.http_method.as_deref().unwrap_or("GET") == "GET")
            .collect();
        let post_actions: Vec<&&GraphNode> = actions.iter()
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

    // Synthesis
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
    writeln!(f, "**See also:** [Overview](./overview.md) · [Architecture](./architecture.md)")?;
    writeln!(f)?;
    writeln!(f, "[← Previous: Overview](./overview.md) | [Next: Architecture →](./architecture.md)")?;

    println!("  {} {}", "OK".green(), out_path.display());

    Ok(())
}

/// French business description for a controller based on its name.
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

    // Group pages by category
    let overview_pages: Vec<_> = pages
        .iter()
        .filter(|(k, _)| !k.starts_with("modules/"))
        .collect();
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
    );

    // 7. Write output
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

/// Build the complete self-contained HTML template.
fn build_html_template(
    project_name: &str,
    stats: &str,
    sidebar_nav: &str,
    first_page_content: &str,
    pages_json: &str,
) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en" data-theme="dark">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{project_name} — Documentation</title>
  <script src="https://cdn.jsdelivr.net/npm/mermaid/dist/mermaid.min.js"></script>
  <style>
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
    .main {{ flex:1; overflow-y:auto; padding:40px 60px; max-width:900px; }}
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

    /* Theme toggle */
    .theme-toggle {{ position:fixed; top:12px; right:16px; background:var(--bg-surface);
                    border:1px solid var(--border); border-radius:8px; padding:6px 12px;
                    color:var(--text-muted); cursor:pointer; font-size:12px; z-index:100; }}

    /* Mermaid */
    .mermaid {{ background:var(--bg-surface); border-radius:8px; padding:16px; margin:16px 0;
               border:1px solid var(--border); text-align:center; }}

    /* Search */
    .search {{ padding:8px 16px; }}
    .search input {{ width:100%; padding:6px 10px; background:var(--bg); border:1px solid var(--border);
                    border-radius:6px; color:var(--text); font-size:12px; outline:none; }}
    .search input:focus {{ border-color:var(--accent); }}

    .hidden {{ display:none; }}

    /* Details/Summary collapsible sections */
    .main details {{ margin:12px 0; border:1px solid var(--border); border-radius:8px;
                    padding:4px 12px; background:var(--bg-surface); }}
    .main details summary {{ cursor:pointer; font-weight:600; font-size:13px; color:var(--text-muted);
                            padding:8px 0; user-select:none; }}
    .main details summary:hover {{ color:var(--accent); }}
    .main details[open] summary {{ margin-bottom:4px; border-bottom:1px solid var(--border); padding-bottom:8px; }}

    @media (max-width:900px) {{
      .sidebar {{ display:none; }}
      .toc {{ display:none; }}
      .main {{ padding:20px; }}
    }}
  </style>
</head>
<body>
  <header class="header">
    <h1>{project_name}</h1>
    <span class="stats">{stats}</span>
    <button class="theme-toggle" onclick="toggleTheme()">Theme</button>
  </header>

  <nav class="sidebar">
    <div class="search">
      <input type="text" placeholder="Search pages..." oninput="filterPages(this.value)">
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
    // Page data embedded as JSON
    const PAGES = {pages_json};

    // Navigation
    function showPage(id) {{
      const page = PAGES[id];
      if (!page) return;
      document.getElementById('content').innerHTML = page.html;

      // Update active sidebar link
      document.querySelectorAll('.sidebar a').forEach(a => a.classList.remove('active'));
      const link = document.querySelector('.sidebar a[data-page="' + id + '"]');
      if (link) link.classList.add('active');

      // Build TOC
      buildToc();

      // Render Mermaid diagrams
      renderMermaid();

      // Scroll to top
      document.getElementById('content').scrollTop = 0;
    }}

    // TOC builder
    function buildToc() {{
      const headings = document.querySelectorAll('.main h2, .main h3');
      const toc = document.getElementById('toc-links');
      toc.innerHTML = '';
      headings.forEach((h, i) => {{
        h.id = 'heading-' + i;
        const a = document.createElement('a');
        a.textContent = h.textContent;
        a.href = '#heading-' + i;
        a.className = h.tagName === 'H3' ? 'depth-3' : '';
        a.onclick = (e) => {{ e.preventDefault(); h.scrollIntoView({{behavior:'smooth'}}); }};
        toc.appendChild(a);
      }});
    }}

    // Mermaid rendering
    function renderMermaid() {{
      document.querySelectorAll('pre code.language-mermaid').forEach(block => {{
        const div = document.createElement('div');
        div.className = 'mermaid';
        div.textContent = block.textContent;
        block.parentElement.replaceWith(div);
      }});
      if (typeof mermaid !== 'undefined') {{
        mermaid.init(undefined, '.mermaid');
      }}
    }}

    // Theme toggle
    function toggleTheme() {{
      const html = document.documentElement;
      const current = html.getAttribute('data-theme');
      const next = current === 'dark' ? 'light' : 'dark';
      html.setAttribute('data-theme', next);
      localStorage.setItem('theme', next);
      if (typeof mermaid !== 'undefined') {{
        mermaid.initialize({{ theme: next === 'dark' ? 'dark' : 'default', startOnLoad: false }});
        renderMermaid();
      }}
    }}

    // Search filter
    function filterPages(query) {{
      const q = query.toLowerCase();
      document.querySelectorAll('.sidebar a[data-page]').forEach(a => {{
        const text = a.textContent.toLowerCase();
        a.style.display = text.includes(q) ? '' : 'none';
      }});
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
                    let replacement = format!("<a href=\"{}\">{}</a>", url, link_text);
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
