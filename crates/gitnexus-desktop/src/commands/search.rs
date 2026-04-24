use std::collections::HashMap;

use gitnexus_db::inmemory::fts::FtsResult;
use gitnexus_search::reranker::{Candidate, LlmReranker, Reranker};
use tauri::State;

use crate::state::AppState;
use crate::types::SearchResult;

/// Full-text search using BM25, with optional LLM reranker on the top-20 pool.
///
/// When `rerank` is true, the command pulls a wider BM25 pool (min 20), sends
/// it to the configured LLM as a reranking prompt, and returns the reordered
/// top-`limit`. If the reranker is unavailable (no config, HTTP error, parse
/// failure) the command silently falls back to the raw BM25 order so the UI
/// never sees an empty response.
#[tauri::command]
pub async fn search_symbols(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
    rerank: Option<bool>,
) -> Result<Vec<SearchResult>, String> {
    let (graph, _indexes, fts_index, _repo_path) = state.get_repo(None).await?;
    let max_results = limit.unwrap_or(20);
    let use_rerank = rerank.unwrap_or(false);

    let pool_size = if use_rerank { max_results.max(20) } else { max_results };
    let mut results = fts_index.search(&graph, &query, None, pool_size);

    if use_rerank && results.len() > 1 {
        results = maybe_rerank_with_chat_config(&state, &query, results).await;
    }
    results.truncate(max_results);

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

async fn maybe_rerank_with_chat_config(
    state: &State<'_, AppState>,
    query: &str,
    fts_results: Vec<FtsResult>,
) -> Vec<FtsResult> {
    let config = state.chat_config().await;
    let (base_url, model, api_key) = match config {
        Some(c) if !c.api_key.is_empty() => (c.base_url, c.model, c.api_key),
        _ => {
            tracing::warn!(
                "search rerank requested but chat config missing or API key empty; using BM25 order"
            );
            return fts_results;
        }
    };

    let candidates: Vec<Candidate> = fts_results
        .iter()
        .enumerate()
        .map(|(i, r)| Candidate {
            node_id: r.node_id.clone(),
            name: r.name.clone(),
            label: r.label.clone(),
            file_path: r.file_path.clone(),
            start_line: r.start_line,
            end_line: r.end_line,
            score: r.score,
            rank: i + 1,
            snippet: None,
        })
        .collect();

    let reranker = LlmReranker::new(base_url, model, Some(api_key));
    let q = query.to_string();
    let reranked = match tokio::task::spawn_blocking(move || reranker.rerank(&q, candidates)).await
    {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "search reranker failed; using BM25 order");
            return fts_results;
        }
        Err(e) => {
            tracing::warn!(error = %e, "search reranker task panicked; using BM25 order");
            return fts_results;
        }
    };

    let mut by_id: HashMap<String, FtsResult> = fts_results
        .into_iter()
        .map(|r| (r.node_id.clone(), r))
        .collect();
    let mut out = Vec::with_capacity(by_id.len());
    for c in reranked {
        if let Some(r) = by_id.remove(&c.node_id) {
            out.push(r);
        }
    }
    out.extend(by_id.into_values());
    out
}
