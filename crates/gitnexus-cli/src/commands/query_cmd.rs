//! The `query` command: search the knowledge graph via the in-memory snapshot.

use std::path::{Path, PathBuf};

use gitnexus_core::storage::repo_manager;
use gitnexus_db::inmemory::fts::{FtsIndex, FtsResult};
use gitnexus_search::embeddings::{
    generate_embeddings, load_embeddings, search_semantic, EmbeddingConfig,
};
use gitnexus_search::bm25::BM25SearchResult;
use gitnexus_search::hybrid;
use gitnexus_search::reranker::{Candidate, LlmReranker, Reranker};

/// When `--rerank` is active, we pull a larger BM25 top-K to give the LLM a
/// broader pool to reorder, then truncate to `limit` after reranking.
const RERANK_CANDIDATE_POOL: usize = 20;

pub async fn run(
    query: &str,
    repo: Option<&str>,
    limit: usize,
    rerank: bool,
    hybrid_mode: bool,
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

    // Pull a larger pool when reranking or fusing so there's room to reorder.
    let pool = if rerank || hybrid_mode {
        limit.max(RERANK_CANDIDATE_POOL)
    } else {
        limit
    };
    let bm25 = fts.search(&graph, query, None, pool);

    if bm25.is_empty() && !hybrid_mode {
        println!("No results for '{query}'.");
        return Ok(());
    }

    // Hybrid: fuse BM25 with semantic via RRF BEFORE any LLM rerank.
    let fused: Vec<FtsResult> = if hybrid_mode {
        match run_hybrid(query, &bm25, &graph, Path::new(&storage.storage_path), pool) {
            Ok(r) => r,
            Err(e) => {
                eprintln!(
                    "Warning: hybrid path failed ({e}); falling back to BM25-only."
                );
                bm25.clone()
            }
        }
    } else {
        bm25.clone()
    };

    if fused.is_empty() {
        println!("No results for '{query}'.");
        return Ok(());
    }

    let candidates = if rerank {
        match run_reranker(query, &fused).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Warning: reranker failed, falling back to pre-rerank order: {e}");
                fts_to_candidates(&fused)
            }
        }
    } else {
        fts_to_candidates(&fused)
    };

    let display = &candidates[..candidates.len().min(limit)];
    println!("Found {} results for '{}':", display.len(), query);
    let mut mods: Vec<&str> = Vec::new();
    if hybrid_mode {
        mods.push("hybrid BM25+semantic RRF");
    }
    if rerank {
        mods.push("LLM rerank");
    }
    if !mods.is_empty() {
        println!("  ({}, pool={})", mods.join(" + "), pool);
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

fn fts_to_candidates(bm25: &[FtsResult]) -> Vec<Candidate> {
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

/// Perform BM25+semantic RRF fusion. Loads `.gitnexus/embeddings.bin` and
/// `embeddings.meta.json`, embeds the query with the same model, takes
/// top-K cosine matches, and fuses via hybrid::merge_with_rrf.
///
/// Returns the fused results as FtsResult (so downstream rerank/display
/// don't need a different branch). The `score` field on each result is
/// the RRF score (0–1 range), not the BM25 or cosine score.
fn run_hybrid(
    query: &str,
    bm25: &[FtsResult],
    graph: &gitnexus_core::graph::KnowledgeGraph,
    storage_path: &Path,
    top_k: usize,
) -> anyhow::Result<Vec<FtsResult>> {
    let emb_path = storage_path.join("embeddings.bin");
    let meta_path = storage_path.join("embeddings.meta.json");
    if !emb_path.exists() || !meta_path.exists() {
        return Err(anyhow::anyhow!(
            "embeddings not found — run 'gitnexus embed --model <path>' first (expected {} and {})",
            emb_path.display(),
            meta_path.display()
        ));
    }
    let cfg: EmbeddingConfig = serde_json::from_str(&std::fs::read_to_string(&meta_path)?)?;
    let store = load_embeddings(&emb_path)?;
    if store.header.dimension != cfg.dimension {
        return Err(anyhow::anyhow!(
            "embeddings.bin dim {} differs from meta dim {}",
            store.header.dimension,
            cfg.dimension
        ));
    }

    // Embed the query with the same model.
    let q_vecs = generate_embeddings(&[query.to_string()], &cfg);
    let q_vec = q_vecs
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("generate_embeddings returned no output"))?;
    if q_vec.iter().all(|&v| v == 0.0) {
        return Err(anyhow::anyhow!(
            "query embedding is all zeros — model path or tokenizer likely missing"
        ));
    }

    // Convert stored entries to the shape search_semantic expects, then
    // enrich with graph metadata before passing to merge_with_rrf.
    let stored: Vec<(String, Vec<f32>)> = store.entries;
    let mut semantic_results = search_semantic(&q_vec, &stored, top_k);
    // search_semantic populates node_id + score + rank only; fill the rest.
    for s in &mut semantic_results {
        if let Some(n) = graph.get_node(&s.node_id) {
            s.file_path = n.properties.file_path.clone();
            s.name = n.properties.name.clone();
            s.label = format!("{:?}", n.label);
            s.start_line = n.properties.start_line;
            s.end_line = n.properties.end_line;
        }
    }

    // Wrap BM25 in BM25SearchResult (same field set).
    let bm25_wrapped: Vec<BM25SearchResult> = bm25
        .iter()
        .enumerate()
        .map(|(i, r)| BM25SearchResult {
            file_path: r.file_path.clone(),
            score: r.score,
            rank: i + 1,
            node_id: r.node_id.clone(),
            name: r.name.clone(),
            label: r.label.clone(),
            start_line: r.start_line,
            end_line: r.end_line,
        })
        .collect();

    let fused = hybrid::merge_with_rrf(&bm25_wrapped, &semantic_results, top_k);

    // Convert back to FtsResult so the outer display loop treats it uniformly.
    Ok(fused
        .into_iter()
        .map(|h| FtsResult {
            node_id: h.node_id,
            score: h.score,
            name: h.name,
            file_path: h.file_path,
            label: h.label,
            start_line: h.start_line,
            end_line: h.end_line,
        })
        .collect())
}

async fn run_reranker(query: &str, fts: &[FtsResult]) -> anyhow::Result<Vec<Candidate>> {
    let config = super::generate::load_llm_config().ok_or_else(|| {
        anyhow::anyhow!(
            "--rerank requires an LLM config at ~/.gitnexus/chat-config.json. \
             Run 'gitnexus config test' to see the expected format."
        )
    })?;

    let candidates = fts_to_candidates(fts);
    let reranker = LlmReranker::new(config.base_url, config.model, Some(config.api_key))
        .with_max_candidates(RERANK_CANDIDATE_POOL);

    let q = query.to_string();
    let result = tokio::task::spawn_blocking(move || reranker.rerank(&q, candidates)).await??;
    Ok(result)
}

fn resolve_repo_path(repo: Option<&str>) -> anyhow::Result<PathBuf> {
    match repo {
        Some(r) => {
            let p = Path::new(r);
            Ok(p.canonicalize().unwrap_or_else(|_| p.to_path_buf()))
        }
        None => Ok(std::env::current_dir()?),
    }
}
