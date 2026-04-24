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
