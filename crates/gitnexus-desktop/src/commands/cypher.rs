use tauri::State;

use gitnexus_db::inmemory::cypher;

use crate::state::AppState;

/// Execute a raw Cypher query (read-only).
#[tauri::command]
pub async fn execute_cypher(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<serde_json::Value>, String> {
    let (graph, indexes, fts_index, _repo_path) = state.get_repo(None).await?;

    // Check for write operations
    let upper = query.to_uppercase();
    let write_keywords = ["CREATE", "DELETE", "MERGE", "REMOVE", "DROP"];
    for kw in &write_keywords {
        if upper.contains(kw) {
            return Err("Only read-only queries are allowed".to_string());
        }
    }
    // SET with any whitespace after it
    if upper.contains("SET") {
        let set_idx = upper.find("SET").unwrap();
        let after = &upper[set_idx + 3..];
        if after.starts_with(|c: char| c.is_whitespace()) {
            return Err("Only read-only queries are allowed".to_string());
        }
    }

    // Parse and execute directly against the graph references
    let stmt = cypher::parse(&query).map_err(|e| format!("Parse error: {}", e))?;
    cypher::execute(&stmt, &graph, &indexes, &fts_index)
        .map_err(|e| format!("Query error: {}", e))
}
