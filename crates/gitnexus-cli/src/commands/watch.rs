//! The `watch` command: monitors a repository for changes and incrementally updates the graph.
//!
//! Uses the incremental engine from [`gitnexus_ingest::incremental`] to
//! efficiently update the knowledge graph when files change, rather than
//! re-indexing the entire repository.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use colored::Colorize;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};

use gitnexus_core::config::languages::SupportedLanguage;
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

        // Save initial manifest alongside snapshot
        let file_entries = gitnexus_ingest::phases::structure::walk_repository(&repo_path)?;
        let manifest = gitnexus_ingest::manifest::build_manifest_from_entries(&file_entries);
        let manifest_file = gitnexus_ingest::manifest::manifest_path(&storage.storage_path);
        gitnexus_ingest::manifest::save_manifest(&manifest, &manifest_file)?;

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
                // Check if any source files changed
                let mut has_source_changes = false;
                for event in &events {
                    if event.kind == DebouncedEventKind::Any {
                        let path = &event.path;
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
                                    has_source_changes = true;
                                    break;
                                }
                            }
                        }
                    }
                }

                if has_source_changes {
                    println!(
                        "\n{} Change detected, running incremental update...",
                        ">>>".yellow().bold(),
                    );

                    // Use the incremental engine
                    let update_result = {
                        let mut g = graph.lock().unwrap();
                        gitnexus_ingest::incremental::incremental_update(
                            &repo_path,
                            &storage.storage_path,
                            &mut g,
                        )
                    };

                    match update_result {
                        Ok(result) => {
                            if result.total_changed() > 0 {
                                println!(
                                    "    {} +{} added, ~{} modified, -{} removed",
                                    "delta".cyan(),
                                    result.added,
                                    result.modified,
                                    result.removed,
                                );
                                println!(
                                    "    {} Removed {} nodes, added {} nodes",
                                    "graph".cyan(),
                                    result.nodes_removed,
                                    result.nodes_added,
                                );

                                // Save updated snapshot
                                let g = graph.lock().unwrap();
                                snapshot::save_snapshot(&g, &snap_path).ok();
                                println!(
                                    "    {} Graph: {} nodes, {} edges total",
                                    "=".cyan(),
                                    g.node_count(),
                                    g.relationship_count()
                                );
                                println!("    {} Snapshot saved", "OK".green());
                            } else {
                                println!(
                                    "    {} No effective changes detected",
                                    "=".dimmed(),
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "    {} Incremental update failed: {}",
                                "!".red(),
                                e
                            );
                        }
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
