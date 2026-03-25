//! The `generate` command: produces AI context files (AGENTS.md, wiki/, skills/) from the knowledge graph.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use colored::Colorize;

use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_core::storage::repo_manager;
use gitnexus_db::snapshot;

pub fn run(what: &str, path: Option<&str>) -> Result<()> {
    let repo_path = Path::new(path.unwrap_or(".")).canonicalize()?;
    let storage = repo_manager::get_storage_paths(&repo_path);
    let snap_path = snapshot::snapshot_path(&storage.storage_path);
    let graph = snapshot::load_snapshot(&snap_path)?;

    match what {
        "context" | "agents" => generate_agents_md(&graph, &repo_path)?,
        "wiki" => generate_wiki(&graph, &repo_path)?,
        "skills" => generate_skills(&graph, &repo_path)?,
        "all" => {
            generate_agents_md(&graph, &repo_path)?;
            generate_wiki(&graph, &repo_path)?;
            generate_skills(&graph, &repo_path)?;
        }
        _ => {
            eprintln!(
                "Unknown target: {}. Use: context, wiki, skills, all",
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
            } else if c == ' ' || c == '/' || c == '\\' {
                '_'
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_lowercase()
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
        for (_id, info) in &communities {
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
        for (_cid, info) in &communities {
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

    for (_cid, info) in &communities {
        let filename = sanitize_filename(&info.label);
        let out_path = wiki_dir.join(format!("{filename}.md"));
        let mut f = std::fs::File::create(&out_path)?;

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
            "  {} wiki/{}",
            "OK".green(),
            format!("{filename}.md")
        );
    }

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
    for (_, info) in &communities {
        for mid in &info.member_ids {
            member_to_community.insert(mid.clone(), info.label.clone());
        }
    }

    for (_cid, info) in &communities {
        let filename = sanitize_filename(&info.label);
        let out_path = skills_dir.join(format!("{filename}.md"));
        let mut f = std::fs::File::create(&out_path)?;

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
            "  {} skills/{}",
            "OK".green(),
            format!("{filename}.md")
        );
    }

    println!(
        "{} Generated {} skill files in {}",
        "OK".green(),
        communities.len(),
        skills_dir.display()
    );
    Ok(())
}
