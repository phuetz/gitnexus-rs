//! The `watch` command: monitors a repository for changes and incrementally updates the graph.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use colored::Colorize;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};

use gitnexus_core::config::languages::SupportedLanguage;
use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_core::storage::repo_manager;
use gitnexus_db::snapshot;

pub async fn run(path: Option<&str>) -> Result<()> {
    let repo_path = path.map(Path::new).unwrap_or(Path::new("."));
    let repo_path = repo_path.canonicalize()?;

    println!("{}", "GitNexus Watch Mode".green().bold());
    println!("Watching: {}", repo_path.display());
    println!("Press Ctrl+C to stop\n");

    // Load existing graph or analyze
    let storage = repo_manager::get_storage_paths(&repo_path);
    let snap_path = snapshot::snapshot_path(&storage.storage_path);

    let graph = if snapshot::snapshot_exists(&snap_path) {
        println!("{}", "Loading existing graph...".dimmed());
        Arc::new(Mutex::new(snapshot::load_snapshot(&snap_path)?))
    } else {
        println!(
            "{}",
            "No existing index. Running initial analysis...".yellow()
        );
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let result = gitnexus_ingest::pipeline::run_pipeline(
            &repo_path,
            Some(tx),
            Default::default(),
        )
        .await?;
        snapshot::save_snapshot(&result.graph, &snap_path)?;
        Arc::new(Mutex::new(result.graph))
    };

    let graph_count = {
        let g = graph.lock().unwrap();
        (g.node_count(), g.relationship_count())
    };
    println!(
        "Graph loaded: {} nodes, {} edges",
        graph_count.0, graph_count.1
    );
    println!("{}\n", "Watching for changes...".green());

    // Setup file watcher with 500ms debounce
    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(500), tx)?;

    debouncer
        .watcher()
        .watch(repo_path.as_ref(), notify::RecursiveMode::Recursive)?;

    // Watch loop
    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                let mut changed_files: Vec<String> = Vec::new();

                for event in &events {
                    if event.kind == DebouncedEventKind::Any {
                        let path = &event.path;
                        // Only care about source files
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if SupportedLanguage::from_extension(&format!(".{ext}")).is_some() {
                                let rel = path
                                    .strip_prefix(&repo_path)
                                    .unwrap_or(path)
                                    .to_string_lossy()
                                    .replace('\\', "/");
                                if !rel.starts_with(".gitnexus")
                                    && !rel.contains("/target/")
                                {
                                    changed_files.push(rel);
                                }
                            }
                        }
                    }
                }

                changed_files.sort();
                changed_files.dedup();

                if !changed_files.is_empty() {
                    println!(
                        "\n{} {} file(s) changed:",
                        ">>>".yellow().bold(),
                        changed_files.len()
                    );
                    for f in &changed_files {
                        println!("    {} {}", "~".yellow(), f);
                    }

                    // Incremental update: remove old nodes for changed files, re-parse them
                    {
                        let mut g = graph.lock().unwrap();
                        for file in &changed_files {
                            let removed = g.remove_nodes_by_file(file);
                            if removed > 0 {
                                println!(
                                    "    {} Removed {} old nodes from {}",
                                    "x".red(),
                                    removed,
                                    file
                                );
                            }
                        }
                    }

                    // Re-parse changed files
                    let file_entries: Vec<_> = changed_files
                        .iter()
                        .filter_map(|rel_path| {
                            let abs = repo_path.join(rel_path);
                            let content = std::fs::read_to_string(&abs).ok()?;
                            let lang = SupportedLanguage::from_filename(rel_path)?;
                            Some(gitnexus_ingest::phases::structure::FileEntry {
                                path: rel_path.clone(),
                                content,
                                size: abs.metadata().map(|m| m.len() as usize).unwrap_or(0),
                                language: Some(lang),
                            })
                        })
                        .collect();

                    if !file_entries.is_empty() {
                        let mut temp_graph = KnowledgeGraph::new();
                        // Add file nodes
                        gitnexus_ingest::phases::structure::create_structure_nodes(
                            &mut temp_graph,
                            &file_entries,
                        );
                        // Parse
                        let _extracted = gitnexus_ingest::phases::parsing::parse_files(
                            &mut temp_graph,
                            &file_entries,
                            None,
                        )
                        .unwrap_or_default();

                        // Merge new nodes/edges into main graph
                        let mut g = graph.lock().unwrap();
                        let mut added_nodes = 0;
                        let mut added_edges = 0;
                        temp_graph.for_each_node(|node| {
                            g.add_node(node.clone());
                            added_nodes += 1;
                        });
                        temp_graph.for_each_relationship(|rel| {
                            g.add_relationship(rel.clone());
                            added_edges += 1;
                        });

                        println!(
                            "    {} Added {} nodes, {} edges",
                            "+".green(),
                            added_nodes,
                            added_edges
                        );
                        println!(
                            "    {} Graph: {} nodes, {} edges total",
                            "=".cyan(),
                            g.node_count(),
                            g.relationship_count()
                        );

                        // Save snapshot
                        snapshot::save_snapshot(&g, &snap_path).ok();
                        println!("    {} Snapshot saved", "OK".green());
                    }

                    println!("{}", "\nWatching for changes...".dimmed());
                }
            }
            Ok(Err(e)) => {
                eprintln!("{} Watch error: {}", "!".red(), e);
            }
            Err(e) => {
                eprintln!("{} Channel error: {}", "!".red(), e);
                break;
            }
        }
    }

    Ok(())
}
