//! The `query` command: search the knowledge graph via the in-memory snapshot.

use std::path::Path;

use gitnexus_core::storage::repo_manager;
use gitnexus_db::inmemory::fts::{FtsIndex, FtsResult};
use gitnexus_search::reranker::{Candidate, LlmReranker, Reranker};

/// When `--rerank` is active, we pull a larger BM25 top-K to give the LLM a
/// broader pool to reorder, then truncate to `limit` after reranking.
const RERANK_CANDIDATE_POOL: usize = 20;

pub async fn run(
    query: &str,
    repo: Option<&str>,
    limit: usize,
    rerank: bool,
) -> anyhow::Result<()> {
    let repo_path = resolve_repo_path(repo)?;
    let storage = repo_manager::get_storage_paths(&repo_path);
    let snap = gitnexus_db::snapshot::snapshot_path(&storage.storage_path);

    if !snap.exists() {
        return Err(anyhow::anyhow!(
            "No graph snapshot found. Run 'gitnexus analyze' first."
        ));
    }

    let graph = gitnexus_db::snapshot::load_snapshot(&snap)?;
    let fts = FtsIndex::build(&graph);

    // Pull a larger pool when reranking so the LLM has room to reorder.
    let pool = if rerank {
        limit.max(RERANK_CANDIDATE_POOL)
    } else {
        limit
    };
    let bm25 = fts.search(&graph, query, None, pool);

    if bm25.is_empty() {
        println!("No results for '{query}'.");
        return Ok(());
    }

    let results = if rerank {
        match run_reranker(query, &bm25).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Warning: reranker failed, falling back to BM25 order: {e}");
                bm25_to_candidates(&bm25)
            }
        }
    } else {
        bm25_to_candidates(&bm25)
    };

    let display = &results[..results.len().min(limit)];
    println!("Found {} results for '{}':", display.len(), query);
    if rerank {
        println!("  (reranked by LLM from top-{} BM25 pool)", bm25.len());
    }
    println!();
    for (i, r) in display.iter().enumerate() {
        let loc = match (r.start_line, r.end_line) {
            (Some(s), Some(e)) => format!("{}:{}-{}", r.file_path, s, e),
            (Some(s), None) => format!("{}:{}", r.file_path, s),
            _ => r.file_path.clone(),
        };
        println!(
            "  {:>3}. [{:<10}] {:<30}  {}",
            i + 1,
            r.label,
            r.name,
            loc
        );
    }

    Ok(())
}

fn bm25_to_candidates(bm25: &[FtsResult]) -> Vec<Candidate> {
    bm25.iter()
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
        .collect()
}

async fn run_reranker(query: &str, bm25: &[FtsResult]) -> anyhow::Result<Vec<Candidate>> {
    let config = super::generate::load_llm_config().ok_or_else(|| {
        anyhow::anyhow!(
            "--rerank requires an LLM config at ~/.gitnexus/chat-config.json. \
             Run 'gitnexus config test' to see the expected format."
        )
    })?;

    let candidates = bm25_to_candidates(bm25);
    let reranker = LlmReranker::new(config.base_url, config.model, Some(config.api_key))
        .with_max_candidates(RERANK_CANDIDATE_POOL);

    // Reqwest blocking call — run on a blocking pool so we don't lock the
    // tokio runtime on a slow LLM endpoint.
    let q = query.to_string();
    let result = tokio::task::spawn_blocking(move || reranker.rerank(&q, candidates)).await??;
    Ok(result)
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
