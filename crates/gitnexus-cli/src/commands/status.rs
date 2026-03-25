//! The `status` command: check GitNexus index status for the current directory.

use gitnexus_core::storage::{git, repo_manager};

pub fn run() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let storage_paths = repo_manager::get_storage_paths(&cwd);

    println!("GitNexus Status");
    println!("  Directory: {}", cwd.display());

    // Check if indexed
    if !repo_manager::has_index(&cwd) {
        println!("  Status: NOT INDEXED");
        println!();
        println!("Run `gitnexus analyze` to index this repository.");
        return Ok(());
    }

    // Load and display metadata
    match repo_manager::load_meta(&storage_paths.storage_path)? {
        Some(meta) => {
            println!("  Status: INDEXED");
            println!("  Indexed at: {}", meta.indexed_at);
            println!("  Commit: {}", meta.last_commit);
            println!("  Storage: {}", storage_paths.storage_path.display());

            // Check if index is stale
            let current_commit = git::current_commit(&cwd);
            match current_commit {
                Some(ref commit) if commit != &meta.last_commit => {
                    println!();
                    println!("  WARNING: Index is stale!");
                    println!("    Indexed commit: {}", meta.last_commit);
                    println!("    Current commit: {commit}");
                    println!("    Run `gitnexus analyze` to update.");
                }
                None => {
                    println!("  Git: not available or not a git repo");
                }
                _ => {
                    println!("  Index is up-to-date.");
                }
            }

            if let Some(stats) = &meta.stats {
                println!();
                println!("  Statistics:");
                if let Some(n) = stats.files {
                    println!("    Files:       {n}");
                }
                if let Some(n) = stats.nodes {
                    println!("    Nodes:       {n}");
                }
                if let Some(n) = stats.edges {
                    println!("    Edges:       {n}");
                }
                if let Some(n) = stats.communities {
                    println!("    Communities: {n}");
                }
                if let Some(n) = stats.processes {
                    println!("    Processes:   {n}");
                }
                if let Some(n) = stats.embeddings {
                    println!("    Embeddings:  {n}");
                }
            }
        }
        None => {
            println!("  Status: INDEX CORRUPTED (meta.json missing)");
            println!("  Run `gitnexus analyze --force` to re-index.");
        }
    }

    Ok(())
}
