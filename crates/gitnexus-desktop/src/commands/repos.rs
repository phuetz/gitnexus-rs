use std::path::Path;

use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;

use gitnexus_core::pipeline::types::PipelineProgress;
use gitnexus_core::storage::{git, repo_manager};
use gitnexus_db::csv_generator;
use gitnexus_db::snapshot;
use gitnexus_ingest::pipeline::{run_pipeline, PipelineOptions};

use crate::state::AppState;
use crate::types::RepoInfo;

#[tauri::command]
pub async fn list_repos(state: State<'_, AppState>) -> Result<Vec<RepoInfo>, String> {
    let entries = state.load_registry().await?;
    let repos = entries
        .into_iter()
        .map(|e| RepoInfo {
            name: e.name,
            path: e.path,
            indexed_at: e.indexed_at,
            last_commit: e.last_commit,
            files: e.stats.as_ref().and_then(|s| s.files),
            nodes: e.stats.as_ref().and_then(|s| s.nodes),
            edges: e.stats.as_ref().and_then(|s| s.edges),
            communities: e.stats.as_ref().and_then(|s| s.communities),
        })
        .collect();
    Ok(repos)
}

#[tauri::command]
pub async fn open_repo(state: State<'_, AppState>, name: String) -> Result<RepoInfo, String> {
    state.open_repo(&name).await?;

    let registry = state.registry().await;
    let entry = registry
        .iter()
        .find(|e| e.name == name)
        .ok_or_else(|| format!("Repository '{}' not found", name))?;

    Ok(RepoInfo {
        name: entry.name.clone(),
        path: entry.path.clone(),
        indexed_at: entry.indexed_at.clone(),
        last_commit: entry.last_commit.clone(),
        files: entry.stats.as_ref().and_then(|s| s.files),
        nodes: entry.stats.as_ref().and_then(|s| s.nodes),
        edges: entry.stats.as_ref().and_then(|s| s.edges),
        communities: entry.stats.as_ref().and_then(|s| s.communities),
    })
}

/// Index a repository using the Rust pipeline directly (no subprocess).
/// Emits "pipeline-progress" events to the frontend for real-time progress tracking.
#[tauri::command]
pub async fn analyze_repo(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    let repo_path = Path::new(&path)
        .canonicalize()
        .map_err(|e| format!("Invalid path '{}': {}", path, e))?;

    let options = PipelineOptions {
        force: true,
        embeddings: false,
        verbose: false,
        skip_git: false,
        ..Default::default()
    };

    // Create a progress channel and forward events to the frontend via Tauri events
    let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<PipelineProgress>();

    let app_handle = app.clone();
    tokio::spawn(async move {
        while let Some(progress) = progress_rx.recv().await {
            let _ = app_handle.emit("pipeline-progress", &progress);
        }
    });

    // Run the ingestion pipeline with progress reporting
    let result = run_pipeline(&repo_path, Some(progress_tx), options)
        .await
        .map_err(|e| format!("Pipeline failed: {}", e))?;

    let file_count = result.total_file_count;
    let node_count = result.graph.node_count();
    let edge_count = result.graph.relationship_count();
    let community_count = result.community_count;
    let process_count = result.process_count;

    // Save metadata
    let commit = git::current_commit(&repo_path).unwrap_or_else(|| "unknown".to_string());
    let meta = repo_manager::RepoMeta {
        repo_path: repo_path.display().to_string(),
        last_commit: commit,
        indexed_at: chrono_now(),
        stats: Some(repo_manager::RepoStats {
            files: Some(file_count),
            nodes: Some(node_count),
            edges: Some(edge_count),
            communities: Some(community_count),
            processes: Some(process_count),
            embeddings: None,
        }),
    };

    let storage_paths = repo_manager::get_storage_paths(&repo_path);
    repo_manager::save_meta(&storage_paths.storage_path, &meta)
        .map_err(|e| format!("Failed to save metadata: {}", e))?;
    repo_manager::register_repo(&repo_path, &meta)
        .map_err(|e| format!("Failed to register repo: {}", e))?;

    // Save graph snapshot
    let snap_path = snapshot::snapshot_path(&storage_paths.storage_path);
    snapshot::save_snapshot(&result.graph, &snap_path)
        .map_err(|e| format!("Failed to save snapshot: {}", e))?;

    // Generate CSVs
    let csv_dir = storage_paths.storage_path.join("csv");
    std::fs::create_dir_all(&csv_dir)
        .map_err(|e| format!("Failed to create CSV dir: {}", e))?;
    csv_generator::generate_all_csvs(&result.graph, &repo_path, &csv_dir)
        .map_err(|e| format!("Failed to generate CSVs: {}", e))?;

    // Reload the repo in AppState so the UI picks up new data
    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Refresh registry and reload
    state.load_registry().await?;
    let _ = state.reload_repo(&repo_name).await;

    // Emit completion event
    let _ = app.emit(
        "pipeline-progress",
        &PipelineProgress {
            phase: gitnexus_core::pipeline::types::PipelinePhase::Complete,
            percent: 100.0,
            message: format!(
                "Indexed successfully: {} files, {} nodes, {} edges, {} communities",
                file_count, node_count, edge_count, community_count
            ),
            detail: None,
            stats: Some(gitnexus_core::pipeline::types::PipelineStats {
                files_processed: file_count,
                total_files: file_count,
                nodes_created: node_count,
            }),
        },
    );

    Ok(format!(
        "Indexed successfully: {} files, {} nodes, {} edges, {} communities",
        file_count, node_count, edge_count, community_count
    ))
}

/// Generate docs (wiki, context, skills) using the Rust CLI binary.
/// Finds the gitnexus binary next to the desktop binary, then falls back to PATH.
#[tauri::command]
pub async fn generate_docs(what: String, path: String) -> Result<String, String> {
    let valid = ["context", "wiki", "skills", "docs", "all"];
    if !valid.contains(&what.as_str()) {
        return Err(format!(
            "Invalid target '{}'. Must be one of: {}",
            what,
            valid.join(", ")
        ));
    }

    let gitnexus_bin = find_gitnexus_binary();

    let output = std::process::Command::new(&gitnexus_bin)
        .args(["generate", &what, "--path", &path])
        .output()
        .map_err(|e| {
            format!(
                "Failed to run '{}'. Error: {}",
                gitnexus_bin, e
            )
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(format!(
            "generate {} failed: {}",
            what,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// Find the gitnexus CLI binary.
/// 1. Look next to the current executable (same build output dir)
/// 2. Fall back to "gitnexus" in PATH
fn find_gitnexus_binary() -> String {
    // In dev/debug, the desktop binary is at target/debug/gitnexus-desktop.exe
    // and the CLI binary is at target/debug/gitnexus.exe (same directory)
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            let sibling = dir.join(if cfg!(windows) { "gitnexus.exe" } else { "gitnexus" });
            if sibling.exists() {
                return sibling.display().to_string();
            }
        }
    }
    // Fallback: hope it's in PATH
    "gitnexus".to_string()
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}Z", now.as_secs())
}
