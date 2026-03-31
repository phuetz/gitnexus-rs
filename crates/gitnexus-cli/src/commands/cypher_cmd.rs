//! The `cypher` command: execute raw Cypher queries against the knowledge graph.

use anyhow::Result;
use colored::Colorize;

use gitnexus_db::snapshot;
use gitnexus_db::inmemory::cypher::{self, GraphIndexes};
use gitnexus_db::inmemory::fts::FtsIndex;

pub async fn run(query: &str, repo: Option<&str>) -> Result<()> {
    let repo_path = if let Some(p) = repo {
        std::path::PathBuf::from(p)
    } else {
        std::env::current_dir()?
    };

    let snap_path = repo_path.join(".gitnexus").join("graph.bin");
    if !snap_path.exists() {
        println!(
            "{} No index found. Run 'gitnexus analyze' first.",
            "ERROR".red()
        );
        return Ok(());
    }

    let graph = snapshot::load_snapshot(&snap_path)
        .map_err(|e| anyhow::anyhow!("Failed to load graph: {}", e))?;

    // Block write operations
    let upper = query.to_uppercase();
    for kw in &["CREATE", "DELETE", "MERGE", "REMOVE", "DROP"] {
        if upper.contains(kw) {
            println!("{} Only read-only queries are allowed.", "ERROR".red());
            return Ok(());
        }
    }

    // Build indexes and FTS
    let indexes = GraphIndexes::build(&graph);
    let fts_index = FtsIndex::build(&graph);

    // Parse and execute
    let stmt = cypher::parse(query)
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    let results = cypher::execute(&stmt, &graph, &indexes, &fts_index)
        .map_err(|e| anyhow::anyhow!("Query error: {}", e))?;

    if results.is_empty() {
        println!("{} No results.", "WARN".yellow());
        return Ok(());
    }

    // Print results as formatted JSON
    println!("{} {} result{}", "OK".green(), results.len(), if results.len() == 1 { "" } else { "s" });
    println!();
    println!("{}", serde_json::to_string_pretty(&results)?);

    Ok(())
}
