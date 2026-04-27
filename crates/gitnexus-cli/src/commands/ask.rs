//! The `ask` command: ask questions about the codebase using graph + LLM.
//!
//! Retrieval uses the same BM25 → hybrid RRF (when `embeddings.bin` exists) →
//! LLM rerank pipeline as `gitnexus query --hybrid --rerank`.  The primitive
//! substring-scoring loop has been replaced by this pipeline to address the
//! "isolated semantic search / no reranking" failure modes identified in the
//! agile-up.com 2026 RAG postmortem.

use std::path::Path;

use anyhow::Result;
use colored::Colorize;
use gitnexus_core::graph::{types::GraphNode, KnowledgeGraph};
use gitnexus_db::inmemory::fts::{FtsIndex, FtsResult};
use gitnexus_db::snapshot;
use gitnexus_search::bm25::BM25SearchResult;
use gitnexus_search::embeddings::{generate_embeddings, load_embeddings, search_semantic, EmbeddingConfig};
use gitnexus_search::hybrid;
use gitnexus_search::reranker::{Candidate, LlmReranker, Reranker};

/// Number of nodes passed as context to the LLM.
const CONTEXT_LIMIT: usize = 10;

/// Retrieval pool size — pull a larger set before reranking so the LLM can
/// reorder a meaningful candidate pool before we truncate to CONTEXT_LIMIT.
const RERANK_CANDIDATE_POOL: usize = 20;

pub fn run(question: &str, path: Option<&str>) -> Result<()> {
    let (answer, top_nodes) = ask_question(
        question,
        path,
        Some(Box::new(|delta| {
            print!("{}", delta);
            use std::io::Write;
            std::io::stdout().flush().unwrap();
        })),
    )?;

    if answer.is_empty() && top_nodes.is_empty() {
        return Ok(());
    }

    println!("\n\n{}", "\u{2500}".repeat(60));

    // Show sources
    println!("\n{}", "Sources:".dimmed());
    for (node, _) in top_nodes.iter().take(5) {
        println!(
            "  {} `{}` in {}",
            "->".dimmed(),
            node.properties.name,
            node.properties.file_path
        );
    }

    Ok(())
}

pub type StreamCallback = Box<dyn Fn(&str) + Send>;

pub fn ask_question(
    question: &str,
    path: Option<&str>,
    stream_cb: Option<StreamCallback>,
) -> Result<(String, Vec<(GraphNode, f64)>)> {
    let repo_path = if let Some(p) = path {
        std::path::PathBuf::from(p)
    } else {
        std::env::current_dir()?
    };

    // Load config
    let config = super::generate::load_llm_config();
    let config = match config {
        Some(c) => c,
        None => {
            return Err(anyhow::anyhow!(
                "No LLM configured. Create ~/.gitnexus/chat-config.json"
            ));
        }
    };

    // Load graph
    let storage_path = repo_path.join(".gitnexus");
    let snap_path = storage_path.join("graph.bin");
    if !snap_path.exists() {
        return Err(anyhow::anyhow!(
            "No index found. Run 'gitnexus analyze' first."
        ));
    }

    let graph = snapshot::load_snapshot(&snap_path)
        .map_err(|e| anyhow::anyhow!("Failed to load graph: {}", e))?;

    // --- BM25 → optional hybrid RRF (replaces the old substring-scoring loop) ---
    let fused = retrieve_bm25_hybrid(question, &graph, &storage_path, CONTEXT_LIMIT)?;

    if fused.is_empty() {
        return Ok((String::new(), Vec::new()));
    }

    // --- LLM rerank (graceful fallback if endpoint is unreachable) ---
    let candidates = match run_reranker(question, &fused, &config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Warning: reranker failed, falling back to pre-rerank order: {e}");
            fts_to_candidates(&fused)
        }
    };

    // Resolve candidates to (GraphNode, score), honouring CONTEXT_LIMIT.
    let top_nodes: Vec<(GraphNode, f64)> = candidates
        .into_iter()
        .take(CONTEXT_LIMIT)
        .filter_map(|c| graph.get_node(&c.node_id).map(|n| (n.clone(), c.score)))
        .collect();

    if top_nodes.is_empty() {
        return Ok((String::new(), Vec::new()));
    }

    // Build context from top nodes
    let mut context = String::new();
    for (node, _score) in &top_nodes {
        context.push_str(&format!(
            "**{}** ({}) in `{}`\n",
            node.properties.name,
            node.label.as_str(),
            node.properties.file_path
        ));

        if let Some(content) = &node.properties.content {
            context.push_str("```markdown\n");
            context.push_str(content);
            context.push_str("\n```\n\n");
            continue;
        }

        let source_path = repo_path.join(&node.properties.file_path);
        if let Ok(source) = std::fs::read_to_string(&source_path) {
            let lines: Vec<&str> = source.lines().collect();
            let start = node
                .properties
                .start_line
                .map(|l| l as usize)
                .unwrap_or(1)
                .saturating_sub(1)
                .min(lines.len());
            let end = (start + 15).min(lines.len());
            context.push_str("```\n");
            for line in &lines[start..end] {
                context.push_str(line);
                context.push('\n');
            }
            context.push_str("```\n\n");
        }
    }

    // Call LLM
    let messages = vec![
        serde_json::json!({
            "role": "system",
            "content": "Tu es un expert en analyse de code. Réponds de façon précise et concise en te basant UNIQUEMENT sur le contexte fourni. Ne fais pas de suppositions."
        }),
        serde_json::json!({
            "role": "user",
            "content": format!("Question : {}\n\nContexte :\n{}", question, context)
        }),
    ];

    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
    let mut body = serde_json::json!({
        "model": config.model,
        "messages": messages,
        "max_tokens": config.max_tokens,
        "temperature": 0.3,
        "stream": stream_cb.is_some()
    });

    let effort = config.reasoning_effort.trim().to_lowercase();
    if !effort.is_empty() && effort != "none" {
        body["reasoning_effort"] = serde_json::Value::String(effort);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let mut request = client.post(&url).json(&body);
    if !config.api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", config.api_key));
    }

    let response = request.send()?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("LLM error: {}", response.status()));
    }

    use std::io::{BufRead, BufReader};

    let mut full_answer = String::new();
    let reader = BufReader::new(response);
    for line in reader.lines() {
        let line = line?;
        if let Some(data) = line.strip_prefix("data: ") {
            if data.trim() == "[DONE]" {
                break;
            }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(delta) = json
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("delta"))
                    .and_then(|d| d.get("content"))
                    .and_then(|v| v.as_str())
                {
                    if let Some(cb) = &stream_cb {
                        cb(delta);
                    }
                    full_answer.push_str(delta);
                }
            }
        } else if stream_cb.is_none() {
            // Non-streaming response body parsing if stream is false
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(content) = json
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|v| v.as_str())
                {
                    full_answer.push_str(content);
                }
            }
        }
    }

    Ok((full_answer, top_nodes))
}

// ─── Retrieval helpers ──────────────────────────────────────────────────

/// BM25 → optional semantic RRF fusion.
///
/// Returns the fused FtsResult list (BM25-only on any embeddings error).
/// Exposed for testing: callers that need retrieval without the LLM context
/// step can call this directly.
pub fn retrieve_bm25_hybrid(
    query: &str,
    graph: &KnowledgeGraph,
    storage_path: &Path,
    limit: usize,
) -> anyhow::Result<Vec<FtsResult>> {
    let fts = FtsIndex::build(graph);
    let pool = limit.max(RERANK_CANDIDATE_POOL);
    let bm25 = fts.search(graph, query, None, pool);
    match run_hybrid(query, &bm25, graph, storage_path, pool) {
        Ok(r) => Ok(r),
        Err(_) => Ok(bm25),
    }
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

fn run_hybrid(
    query: &str,
    bm25: &[FtsResult],
    graph: &KnowledgeGraph,
    storage_path: &Path,
    top_k: usize,
) -> anyhow::Result<Vec<FtsResult>> {
    let emb_path = storage_path.join("embeddings.bin");
    let meta_path = storage_path.join("embeddings.meta.json");
    if !emb_path.exists() || !meta_path.exists() {
        return Err(anyhow::anyhow!("embeddings not found"));
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

    let stored: Vec<(String, Vec<f32>)> = store.entries;
    let mut semantic_results = search_semantic(&q_vec, &stored, top_k);
    for s in &mut semantic_results {
        if let Some(n) = graph.get_node(&s.node_id) {
            s.file_path = n.properties.file_path.clone();
            s.name = n.properties.name.clone();
            s.label = format!("{:?}", n.label);
            s.start_line = n.properties.start_line;
            s.end_line = n.properties.end_line;
        }
    }

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

fn run_reranker(
    query: &str,
    fts: &[FtsResult],
    config: &super::generate::LlmConfig,
) -> anyhow::Result<Vec<Candidate>> {
    let candidates = fts_to_candidates(fts);
    let reranker = LlmReranker::new(
        config.base_url.clone(),
        config.model.clone(),
        Some(config.api_key.clone()),
    )
    .with_max_candidates(RERANK_CANDIDATE_POOL);
    reranker.rerank(query, candidates)
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use gitnexus_core::graph::{types::*, KnowledgeGraph};

    fn make_node(id: &str, name: &str, file: &str, label: NodeLabel) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            label,
            properties: NodeProperties {
                name: name.to_string(),
                file_path: file.to_string(),
                description: Some(format!("{name} description")),
                start_line: Some(1),
                end_line: Some(10),
                ..Default::default()
            },
        }
    }

    fn fixture_graph() -> KnowledgeGraph {
        let mut g = KnowledgeGraph::new();
        g.add_node(make_node(
            "Function:src/auth.rs:authenticate",
            "authenticate",
            "src/auth.rs",
            NodeLabel::Function,
        ));
        g.add_node(make_node(
            "Function:src/auth.rs:validate_token",
            "validate_token",
            "src/auth.rs",
            NodeLabel::Function,
        ));
        g.add_node(make_node(
            "Function:src/db.rs:query_users",
            "query_users",
            "src/db.rs",
            NodeLabel::Function,
        ));
        g.add_node(make_node(
            "Class:src/user.rs:UserService",
            "UserService",
            "src/user.rs",
            NodeLabel::Class,
        ));
        g
    }

    #[test]
    fn retrieve_bm25_returns_relevant_nodes() {
        let graph = fixture_graph();
        // Use a temp dir with no embeddings.bin so it falls back to BM25-only.
        let tmp = tempfile::tempdir().expect("tempdir");
        let results = retrieve_bm25_hybrid("authenticate user token", &graph, tmp.path(), 5)
            .expect("retrieve should not fail");

        // BM25 should surface auth-related nodes before unrelated ones.
        assert!(
            !results.is_empty(),
            "expected at least one result for 'authenticate user token'"
        );
        let top_name = &results[0].name;
        assert!(
            top_name == "authenticate" || top_name == "validate_token",
            "expected auth node at top, got '{top_name}'"
        );
    }

    #[test]
    fn retrieve_bm25_no_embeddings_does_not_error() {
        let graph = fixture_graph();
        let tmp = tempfile::tempdir().expect("tempdir");
        // Should degrade gracefully to BM25 when embeddings.bin is absent.
        let result = retrieve_bm25_hybrid("query database", &graph, tmp.path(), 10);
        assert!(result.is_ok());
    }

    #[test]
    fn retrieve_bm25_empty_graph_returns_empty() {
        let graph = KnowledgeGraph::new();
        let tmp = tempfile::tempdir().expect("tempdir");
        let results = retrieve_bm25_hybrid("anything", &graph, tmp.path(), 5).unwrap();
        assert!(results.is_empty());
    }
}
