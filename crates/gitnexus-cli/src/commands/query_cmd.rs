//! The `query` command: search the knowledge graph via the in-memory snapshot.

use std::path::Path;

use gitnexus_core::storage::repo_manager;
use gitnexus_db::inmemory::fts::FtsIndex;

pub async fn run(query: &str, repo: Option<&str>, limit: usize) -> anyhow::Result<()> {
    let repo_path = resolve_repo_path(repo)?;
    let storage = repo_manager::get_storage_paths(&repo_path);
    let snap = gitnexus_db::snapshot::snapshot_path(&storage.storage_path);

    if !snap.exists() {
        return Err(anyhow::anyhow!(
            "No graph snapshot found. Run 'gitnexus analyze' first."
        ));
    }

    let graph = gitnexus_db::snapshot::load_snapshot(&snap)?;

    // Build FTS index and search
    let fts = FtsIndex::build(&graph);
    let results = fts.search(&graph, query, None, limit);

    if results.is_empty() {
        println!("No results for '{query}'.");
        return Ok(());
    }

    println!("Found {} results for '{}':", results.len(), query);
    println!();

    for (i, r) in results.iter().enumerate() {
        let loc = match (r.start_line, r.end_line) {
            (Some(s), Some(e)) => format!("{}:{}-{}", r.file_path, s, e),
            (Some(s), None) => format!("{}:{}", r.file_path, s),
            _ => r.file_path.clone(),
        };
        println!("  {:>3}. [{:<10}] {:<30}  {}", i + 1, r.label, r.name, loc);
    }

    Ok(())
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
