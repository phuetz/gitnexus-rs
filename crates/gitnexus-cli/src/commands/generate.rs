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
        }
        _ => {
            eprintln!(
                "Unknown target: {}. Use: context, wiki, skills, docs, docx, all",
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

/// Generate overview.md with architecture diagram.
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

    writeln!(f, "# {}", repo_name)?;
    writeln!(f)?;
    writeln!(
        f,
        "This project contains **{file_count}** source files with **{node_count}** nodes and **{edge_count}** relationships in the code graph."
    )?;
    writeln!(f)?;

    // Language Distribution
    writeln!(f, "## Language Distribution")?;
    writeln!(f)?;
    let mut lang_vec: Vec<_> = lang_stats.iter().collect();
    lang_vec.sort_by(|a, b| b.1.cmp(a.1));
    for (lang, count) in lang_vec {
        writeln!(f, "- **{}**: {} files", lang, count)?;
    }
    writeln!(f)?;

    // Architecture Overview with Mermaid diagram
    if communities.len() > 1 {
        writeln!(f, "## Architecture Overview")?;
        writeln!(f)?;

        // Build member->community mappings and cross-community calls
        let mut member_to_community: HashMap<String, String> = HashMap::new();
        for info in communities.values() {
            for mid in &info.member_ids {
                member_to_community.insert(mid.clone(), info.label.clone());
            }
        }

        let mut cross_deps: BTreeMap<String, (usize, BTreeSet<String>)> = BTreeMap::new();
        for rel in graph.iter_relationships() {
            if rel.rel_type == RelationshipType::Calls {
                if let (Some(src_comm), Some(tgt_comm)) = (
                    member_to_community.get(&rel.source_id),
                    member_to_community.get(&rel.target_id),
                ) {
                    if src_comm != tgt_comm {
                        let entry = cross_deps
                            .entry(src_comm.clone())
                            .or_insert_with(|| (0, BTreeSet::new()));
                        entry.0 += 1;
                        entry.1.insert(tgt_comm.clone());
                    }
                }
            }
        }

        writeln!(f, "```mermaid")?;
        writeln!(f, "graph TD")?;
        for info in communities.values() {
            let safe_id = sanitize_filename(&info.label).replace('-', "_");
            writeln!(f, "    {}[\"{}\"]", safe_id, escape_mermaid_label(&info.label))?;
        }
        for (src, (_count, targets)) in &cross_deps {
            let src_id = sanitize_filename(src).replace('-', "_");
            for tgt in targets {
                let tgt_id = sanitize_filename(tgt).replace('-', "_");
                writeln!(f, "    {} --> {}", src_id, tgt_id)?;
            }
        }
        writeln!(f, "```")?;
        writeln!(f)?;
    }

    // Modules
    writeln!(f, "## Modules")?;
    writeln!(f)?;
    for info in communities.values() {
        let filename = sanitize_filename(&info.label);
        writeln!(
            f,
            "- **[{}](modules/{}.md)** - {} members",
            info.label, filename, info.member_ids.len()
        )?;
        if let Some(desc) = &info.description {
            writeln!(f, "  - {}", desc)?;
        }
    }
    writeln!(f)?;

    println!("  {} overview.md", "OK".green());
    Ok(())
}

/// Generate architecture.md with detailed module info.
fn generate_docs_architecture(
    docs_dir: &Path,
    communities: &BTreeMap<String, CommunityInfo>,
    graph: &KnowledgeGraph,
    _edge_map: &HashMap<String, Vec<(String, RelationshipType)>>,
    _file_count: usize,
    node_count: usize,
    edge_count: usize,
) -> Result<()> {
    let out_path = docs_dir.join("architecture.md");
    let mut f = std::fs::File::create(&out_path)?;

    writeln!(f, "# Architecture")?;
    writeln!(f)?;
    writeln!(
        f,
        "System architecture with **{}** modules, **{}** nodes, and **{}** relationships.",
        communities.len(),
        node_count,
        edge_count
    )?;
    writeln!(f)?;

    // Module Dependency Graph
    if communities.len() > 1 {
        writeln!(f, "## Module Dependency Graph")?;
        writeln!(f)?;

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

        writeln!(f, "```mermaid")?;
        writeln!(f, "graph TD")?;
        for info in communities.values() {
            let safe_id = sanitize_filename(&info.label).replace('-', "_");
            writeln!(f, "    {}[\"{}\"]", safe_id, escape_mermaid_label(&info.label))?;
        }
        for (src, targets) in &cross_deps {
            let src_id = sanitize_filename(src).replace('-', "_");
            for tgt in targets {
                let tgt_id = sanitize_filename(tgt).replace('-', "_");
                writeln!(f, "    {} --> {}", src_id, tgt_id)?;
            }
        }
        writeln!(f, "```")?;
        writeln!(f)?;
    }

    // Module Details
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

        // Entry points in this community
        let entry_points: Vec<&GraphNode> = info
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
        if !entry_points.is_empty() {
            writeln!(f, "- **Entry Points**: {}", entry_points.len())?;
        }
        writeln!(f)?;
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
            let reason = node
                .properties
                .entry_point_reason
                .as_deref()
                .unwrap_or("");
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

    writeln!(f, "# Getting Started")?;
    writeln!(f)?;
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

    println!("  {} getting-started.md", "OK".green());
    Ok(())
}

/// Generate per-module documentation files.
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

    // Track used filenames to avoid collisions
    let mut used_filenames: HashSet<String> = HashSet::new();

    for info in communities.values() {
        let base = sanitize_filename(&info.label);
        let filename = if used_filenames.contains(&base) {
            let mut candidate = base.clone();
            let mut counter = 2;
            while used_filenames.contains(&candidate) {
                candidate = format!("{}_{}", base, counter);
                counter += 1;
            }
            candidate
        } else {
            base
        };
        used_filenames.insert(filename.clone());
        let out_path = modules_dir.join(format!("{}.md", filename));
        let mut f = std::fs::File::create(&out_path)?;

        let member_set: HashSet<&str> = info.member_ids.iter().map(|s| s.as_str()).collect();

        writeln!(f, "# {}", info.label)?;
        writeln!(f)?;

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

        let mut files_set: BTreeSet<String> = BTreeSet::new();
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
                files_set.insert(node.properties.file_path.clone());
            }
        }
        writeln!(f)?;

        // Entry Points
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

        println!(
            "  {} modules/{filename}.md",
            "OK".green(),
        );
        page_count += 1;
    }

    // ─── Per-Controller pages ──────────────────────────────────────────
    let controllers: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::Controller)
        .collect();

    for ctrl in &controllers {
        let ctrl_name = &ctrl.properties.name;
        let filename = format!("ctrl-{}", sanitize_filename(ctrl_name));
        let out_path = modules_dir.join(format!("{filename}.md"));

        // Find actions for this controller
        let actions: Vec<&GraphNode> = graph.iter_nodes()
            .filter(|n| n.label == NodeLabel::ControllerAction
                      && n.properties.file_path == ctrl.properties.file_path)
            .collect();

        // Find views rendered by this controller
        let views: Vec<String> = graph.iter_relationships()
            .filter(|r| r.source_id.contains(&ctrl.properties.name)
                      && r.rel_type == RelationshipType::RendersView)
            .map(|r| r.target_id.clone())
            .collect();

        let mut content = format!("# {}\n\n", ctrl_name);
        content.push_str(&format!("**File:** `{}`\n\n", ctrl.properties.file_path));

        // Actions table
        content.push_str("## Actions\n\n");
        content.push_str("| Action | HTTP Method | Route | Return Type |\n");
        content.push_str("|--------|------------|-------|-------------|\n");
        for action in &actions {
            let method = action.properties.http_method.as_deref().unwrap_or("GET");
            let route = action.properties.route_template.as_deref().unwrap_or("-");
            let ret = action.properties.return_type.as_deref().unwrap_or("ActionResult");
            content.push_str(&format!("| {} | {} | {} | {} |\n",
                action.properties.name, method, route, ret));
        }
        content.push('\n');

        // Views section
        if !views.is_empty() {
            content.push_str("## Views\n\n");
            for v in &views {
                content.push_str(&format!("- `{}`\n", v));
            }
            content.push('\n');
        }

        // Stats
        content.push_str(&format!("\n---\n*{} actions*\n", actions.len()));

        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    // ─── Data Model pages ──────────────────────────────────────────────
    let db_contexts: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::DbContext)
        .collect();

    for ctx in &db_contexts {
        let ctx_name = &ctx.properties.name;
        let filename = format!("data-{}", sanitize_filename(ctx_name));
        let out_path = modules_dir.join(format!("{filename}.md"));

        // Find entities mapped to this context
        let entities: Vec<&GraphNode> = graph.iter_nodes()
            .filter(|n| n.label == NodeLabel::DbEntity)
            .collect(); // TODO: filter by context relationship

        let mut content = format!("# Data Model: {}\n\n", ctx_name);
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

        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    // ─── Service Layer page ────────────────────────────────────────────
    let services: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::Service || n.label == NodeLabel::Repository)
        .collect();

    if !services.is_empty() {
        let out_path = modules_dir.join("services.md");

        let mut content = String::from("# Service Layer\n\n");
        content.push_str(&format!("**Total services:** {}\n\n", services.len()));

        content.push_str("## Services\n\n");
        content.push_str("| Service | Type | Interface | File |\n");
        content.push_str("|---------|------|-----------|------|\n");
        for svc in &services {
            let layer = svc.properties.layer_type.as_deref().unwrap_or("Service");
            let iface = svc.properties.implements_interface.as_deref().unwrap_or("-");
            content.push_str(&format!("| {} | {} | {} | `{}` |\n",
                svc.properties.name, layer, iface, svc.properties.file_path));
        }

        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    // ─── UI Components page ────────────────────────────────────────────
    let ui_components: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::UiComponent)
        .collect();

    if !ui_components.is_empty() {
        let out_path = modules_dir.join("ui-components.md");

        let mut content = String::from("# UI Components (Telerik/Kendo)\n\n");
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

        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    // ─── AJAX Endpoints page ───────────────────────────────────────────
    let ajax_calls: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::AjaxCall)
        .collect();

    if !ajax_calls.is_empty() {
        let out_path = modules_dir.join("ajax-endpoints.md");

        let mut content = String::from("# AJAX Endpoints\n\n");
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

        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    Ok(page_count)
}
