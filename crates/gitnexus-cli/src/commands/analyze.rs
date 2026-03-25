//! The `analyze` command: indexes a repository into a knowledge graph.

use std::path::Path;

use indicatif::{ProgressBar, ProgressStyle};

use gitnexus_core::storage::{git, repo_manager};

pub async fn run(
    path: &str,
    force: bool,
    embeddings: bool,
    verbose: bool,
    skip_git: bool,
) -> anyhow::Result<()> {
    let repo_path = Path::new(path)
        .canonicalize()
        .unwrap_or_else(|_| Path::new(path).to_path_buf());

    println!("Indexing repository: {}", repo_path.display());

    // Check if already indexed
    if !force && repo_manager::has_index(&repo_path) {
        println!("Repository already indexed. Use --force to re-index.");
        return Ok(());
    }

    // Create progress bar
    let style = ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] {msg}")
        .unwrap();
    let pb = ProgressBar::new_spinner();
    pb.set_style(style);
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    // Create progress channel
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<gitnexus_core::pipeline::PipelineProgress>();

    // Spawn progress handler
    let pb_clone = pb.clone();
    let progress_handle = tokio::spawn(async move {
        while let Some(progress) = rx.recv().await {
            pb_clone.set_message(format!(
                "[{:>12}] {:.0}% {}",
                progress.phase.as_str(),
                progress.percent,
                progress.message
            ));
        }
    });

    // Run pipeline
    let options = gitnexus_ingest::pipeline::PipelineOptions {
        force,
        embeddings,
        verbose,
        skip_git,
    };

    let result = gitnexus_ingest::pipeline::run_pipeline(&repo_path, Some(tx), options).await;

    // Wait for progress handler to finish
    let _ = progress_handle.await;
    pb.finish_and_clear();

    match result {
        Ok(result) => {
            println!("\nIndexing complete!");
            println!("  Files:       {}", result.total_file_count);
            println!("  Nodes:       {}", result.graph.node_count());
            println!("  Edges:       {}", result.graph.relationship_count());
            println!("  Communities: {}", result.community_count);
            println!("  Processes:   {}", result.process_count);

            // Save metadata
            let commit = git::current_commit(&repo_path)
                .unwrap_or_else(|| "unknown".to_string());
            let meta = repo_manager::RepoMeta {
                repo_path: repo_path.display().to_string(),
                last_commit: commit,
                indexed_at: chrono_now(),
                stats: Some(repo_manager::RepoStats {
                    files: Some(result.total_file_count),
                    nodes: Some(result.graph.node_count()),
                    edges: Some(result.graph.relationship_count()),
                    communities: Some(result.community_count),
                    processes: Some(result.process_count),
                    embeddings: None,
                }),
            };

            let storage_paths = repo_manager::get_storage_paths(&repo_path);
            repo_manager::save_meta(&storage_paths.storage_path, &meta)?;
            repo_manager::register_repo(&repo_path, &meta)?;

            // Save binary snapshot for fast reload (REPL, MCP, CLI queries)
            let snap_path = gitnexus_db::snapshot::snapshot_path(&storage_paths.storage_path);
            gitnexus_db::snapshot::save_snapshot(&result.graph, &snap_path)?;
            println!("  Graph snapshot saved ({} bytes)", std::fs::metadata(&snap_path).map(|m| m.len()).unwrap_or(0));

            // Generate CSV and save
            println!("  Saving CSVs...");
            let csv_dir = storage_paths.storage_path.join("csv");
            std::fs::create_dir_all(&csv_dir)?;
            gitnexus_db::csv_generator::generate_all_csvs(&result.graph, &repo_path, &csv_dir)?;

            // Load CSVs into KuzuDB (when the kuzu-backend feature is enabled)
            #[cfg(feature = "kuzu-backend")]
            {
                println!("  Loading into KuzuDB...");
                let mut db = gitnexus_db::adapter::DbAdapter::new_kuzu();
                db.open(&storage_paths.lbug_path)?;
                db.create_schema()?;
                db.bulk_load_csv(&csv_dir)?;
                db.close()?;
                println!("  KuzuDB loaded successfully.");
            }

            println!("  Done! Run 'gitnexus mcp' to start the MCP server.");
            Ok(())
        }
        Err(e) => {
            eprintln!("Pipeline failed: {e}");
            Err(e.into())
        }
    }
}

fn chrono_now() -> String {
    // Simple ISO 8601 timestamp without chrono dependency
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}Z", now.as_secs())
}
