//! The `cypher` command: execute raw Cypher queries.
//! Currently a placeholder until the Cypher engine is implemented.

pub async fn run(query: &str, _repo: Option<&str>) -> anyhow::Result<()> {
    eprintln!("Cypher query engine is not yet available.");
    eprintln!("Query: {}", query);
    eprintln!("Use 'gitnexus query' or 'gitnexus shell' for graph exploration.");
    Ok(())
}
