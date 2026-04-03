//! The `impact` command: blast radius analysis via in-memory snapshot.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use gitnexus_core::graph::types::{NodeLabel, RelationshipType};
use gitnexus_core::storage::repo_manager;

pub async fn run(target: &str, repo: Option<&str>, direction: &str) -> anyhow::Result<()> {
    let repo_path = resolve_repo_path(repo)?;
    let storage = repo_manager::get_storage_paths(&repo_path);
    let snap = gitnexus_db::snapshot::snapshot_path(&storage.storage_path);

    if !snap.exists() {
        return Err(anyhow::anyhow!("No graph snapshot found. Run 'gitnexus analyze' first."));
    }

    let graph = gitnexus_db::snapshot::load_snapshot(&snap)?;
    let lower = target.to_lowercase();

    // Find the target node — prefer Controller/Class/Service over Constructor/File
    let mut matches: Vec<_> = graph.iter_nodes()
        .filter(|n| n.properties.name.to_lowercase() == lower)
        .collect();
    if matches.is_empty() {
        matches = graph.iter_nodes()
            .filter(|n| n.properties.name.to_lowercase().contains(&lower))
            .collect();
    }

    if matches.is_empty() {
        println!("Symbol '{}' not found.", target);
        return Ok(());
    }

    // Sort by priority: Controller > Class > Service > Method > File > others
    matches.sort_by_key(|n| match n.label {
        NodeLabel::Controller => 0,
        NodeLabel::Class => 1,
        NodeLabel::Service => 2,
        NodeLabel::Method => 5,
        NodeLabel::File => 8,
        _ => 10,
    });

    let start_id = &matches[0].id;
    let start_label = matches[0].label;
    let start_name = graph
        .get_node(start_id)
        .map(|n| n.properties.name.clone())
        .unwrap_or_else(|| target.to_string());
    let start_file = matches[0].properties.file_path.clone();

    // Build adjacency index for dependency-related edges
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();

    for rel in graph.iter_relationships() {
        // Skip structural relationships that don't represent functional dependencies
        if matches!(rel.rel_type, RelationshipType::Contains | RelationshipType::Imports
            | RelationshipType::StepInProcess | RelationshipType::Defines
            | RelationshipType::MemberOf | RelationshipType::Inherits
            | RelationshipType::Implements | RelationshipType::Extends
            | RelationshipType::BelongsToArea) {
            continue;
        }
        outgoing
            .entry(rel.source_id.clone())
            .or_default()
            .push(rel.target_id.clone());
        incoming
            .entry(rel.target_id.clone())
            .or_default()
            .push(rel.source_id.clone());
    }

    // Collect BFS seed IDs: start node + child methods (for Class/Service/Controller)
    let mut seed_ids: Vec<String> = vec![start_id.clone()];
    if matches!(start_label, NodeLabel::Class | NodeLabel::Service
        | NodeLabel::Interface | NodeLabel::Controller)
    {
        // For Controllers, also include the sibling Class node's children
        let mut source_ids = vec![start_id.clone()];
        if start_label == NodeLabel::Controller {
            for n in graph.iter_nodes() {
                if n.label == NodeLabel::Class
                    && n.properties.name == start_name
                    && n.properties.file_path == start_file
                {
                    source_ids.push(n.id.clone());
                }
            }
        }

        for rel in graph.iter_relationships() {
            if source_ids.contains(&rel.source_id)
                && matches!(rel.rel_type, RelationshipType::HasMethod
                    | RelationshipType::HasProperty | RelationshipType::HasAction)
            {
                seed_ids.push(rel.target_id.clone());
            }
        }
    }

    let use_downstream = direction == "downstream" || direction == "both";
    let use_upstream = direction == "upstream" || direction == "both";

    println!("Impact Analysis for '{}' (direction: {})", start_name, direction);
    println!("{}", "-".repeat(50));

    let max_depth = 5;

    if use_downstream {
        println!("\nDownstream (symbols affected by changes):");
        bfs_print(&graph, &seed_ids, &outgoing, max_depth);
    }

    if use_upstream {
        println!("\nUpstream (symbols that affect this):");
        bfs_print(&graph, &seed_ids, &incoming, max_depth);
    }

    Ok(())
}

fn bfs_print(
    graph: &gitnexus_core::graph::KnowledgeGraph,
    seeds: &[String],
    adjacency: &HashMap<String, Vec<String>>,
    max_depth: usize,
) {
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();

    // Initialize BFS with all seed nodes
    for seed in seeds {
        if visited.insert(seed.clone()) {
            queue.push_back((seed.clone(), 0));
        }
    }

    let mut total = 0;
    let mut levels: Vec<Vec<String>> = vec![Vec::new(); max_depth];

    while let Some((node_id, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        if let Some(neighbors) = adjacency.get(&node_id) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    // Skip obj/ artifacts and Community noise nodes
                    if let Some(node) = graph.get_node(neighbor) {
                        if node.label == NodeLabel::Community
                            || node.properties.file_path.contains("/obj/")
                            || node.properties.file_path.contains("\\obj\\")
                        {
                            continue;
                        }
                    }
                    visited.insert(neighbor.clone());
                    levels[depth].push(neighbor.clone());
                    total += 1;
                    queue.push_back((neighbor.clone(), depth + 1));
                }
            }
        }
    }

    for (depth, ids) in levels.iter().enumerate() {
        if ids.is_empty() {
            continue;
        }
        println!("  Depth {} ({} nodes):", depth + 1, ids.len());
        for id in ids.iter().take(10) {
            if let Some(node) = graph.get_node(id) {
                let loc = match node.properties.start_line {
                    Some(l) => format!("{}:{}", node.properties.file_path, l),
                    None => node.properties.file_path.clone(),
                };
                println!(
                    "    {} {} ({})",
                    node.label.as_str(),
                    node.properties.name,
                    loc
                );
            }
        }
        if ids.len() > 10 {
            println!("    ... and {} more", ids.len() - 10);
        }
    }

    println!("  Total affected: {} symbols", total);
}

fn resolve_repo_path(repo: Option<&str>) -> anyhow::Result<std::path::PathBuf> {
    match repo {
        Some(r) => {
            let p = Path::new(r);
            Ok(p.canonicalize().unwrap_or_else(|_| p.to_path_buf()))
        }
        None => Ok(std::env::current_dir()?),
    }
}
