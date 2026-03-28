use tauri::State;

use crate::state::AppState;
use crate::types::SearchResult;

/// Full-text search using BM25.
#[tauri::command]
pub async fn search_symbols(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, String> {
    let (graph, _indexes, fts_index, _repo_path) = state.get_repo(None).await?;
    let max_results = limit.unwrap_or(20);

    let results = fts_index.search(&graph, &query, None, max_results);

    Ok(results
        .into_iter()
        .map(|r| SearchResult {
            node_id: r.node_id,
            name: r.name,
            label: r.label,
            file_path: r.file_path,
            score: r.score,
            start_line: r.start_line,
            end_line: r.end_line,
        })
        .collect())
}

/// Autocomplete search using the name index.
#[tauri::command]
pub async fn search_autocomplete(
    state: State<'_, AppState>,
    prefix: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, String> {
    let (graph, _indexes, _fts, _repo_path) = state.get_repo(None).await?;
    let max_results = limit.unwrap_or(10);
    let prefix_lower = prefix.to_lowercase();

    let mut results = Vec::new();

    for node in graph.iter_nodes() {
        if node.properties.name.to_lowercase().starts_with(&prefix_lower) {
            results.push(SearchResult {
                node_id: node.id.clone(),
                name: node.properties.name.clone(),
                label: node.label.as_str().to_string(),
                file_path: node.properties.file_path.clone(),
                score: 1.0,
                start_line: node.properties.start_line,
                end_line: node.properties.end_line,
            });
            if results.len() >= max_results {
                return Ok(results);
            }
        }
    }

    Ok(results)
}
