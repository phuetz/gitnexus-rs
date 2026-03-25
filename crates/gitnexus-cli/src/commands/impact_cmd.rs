//! The `impact` command: blast radius analysis via in-memory snapshot.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use gitnexus_core::graph::types::RelationshipType;
use gitnexus_core::storage::repo_manager;

pub async fn run(target: &str, repo: Option<&str>, direction: &str) -> anyhow::Result<()> {
    let repo_path = resolve_repo_path(repo)?;
    let storage = repo_manager::get_storage_paths(&repo_path);
    let snap = gitnexus_db::snapshot::snapshot_path(&storage.storage_path);

    if !snap.exists() {
        eprintln!("No graph snapshot found. Run 'gitnexus analyze' first.");
        std::process::exit(1);
    }

    let graph = gitnexus_db::snapshot::load_snapshot(&snap)?;
    let lower = target.to_lowercase();

    // Find the target node
    let mut matches = Vec::new();
    for node in graph.iter_nodes() {
        if node.properties.name.to_lowercase() == lower {
            matches.push(node.id.clone());
        }
    }
    if matches.is_empty() {
        for node in graph.iter_nodes() {
            if node.properties.name.to_lowercase().contains(&lower) {
                matches.push(node.id.clone());
            }
        }
    }

    if matches.is_empty() {
        println!("Symbol '{}' not found.", target);
        return Ok(());
    }

    let start_id = &matches[0];

    // Build adjacency index for CALLS edges
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();

    for rel in graph.iter_relationships() {
        if rel.rel_type == RelationshipType::Calls {
            outgoing
                .entry(rel.source_id.clone())
                .or_default()
                .push(rel.target_id.clone());
            incoming
                .entry(rel.target_id.clone())
                .or_default()
                .push(rel.source_id.clone());
        }
    }

    let use_downstream = direction == "downstream" || direction == "both";
    let use_upstream = direction == "upstream" || direction == "both";

    let start_name = graph
        .get_node(start_id)
        .map(|n| n.properties.name.clone())
        .unwrap_or_else(|| target.to_string());

    println!("Impact Analysis for '{}' (direction: {})", start_name, direction);
    println!("{}", "-".repeat(50));

    let max_depth = 5;

    if use_downstream {
        println!("\nDownstream (symbols affected by changes):");
        bfs_print(&graph, start_id, &outgoing, max_depth);
    }

    if use_upstream {
        println!("\nUpstream (symbols that affect this):");
        bfs_print(&graph, start_id, &incoming, max_depth);
    }

    Ok(())
}

fn bfs_print(
    graph: &gitnexus_core::graph::KnowledgeGraph,
    start: &str,
    adjacency: &HashMap<String, Vec<String>>,
    max_depth: usize,
) {
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(start.to_string());
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    queue.push_back((start.to_string(), 0));
    let mut total = 0;

    let mut levels: Vec<Vec<String>> = vec![Vec::new(); max_depth];

    while let Some((node_id, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        if let Some(neighbors) = adjacency.get(&node_id) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
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
