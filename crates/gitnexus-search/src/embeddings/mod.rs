//! Embeddings pipeline for semantic search.
//!
//! When the `embeddings` feature is enabled, uses ONNX Runtime for inference.
//! Otherwise, falls back to zero vectors.

pub mod types;

pub use types::{EmbeddingConfig, SemanticSearchResult};

#[cfg(feature = "embeddings")]
mod onnx {
    use ort::session::Session;
    use ort::session::builder::GraphOptimizationLevel;
    use ndarray::Array2;
    use std::path::Path;

    pub struct OnnxEmbedder {
        // TODO: wire up real inference. The session is loaded (so we still
        // surface model-load errors early, and tests cover that path), but
        // embed() currently returns zero vectors — we need a tokenizer and
        // a proper input-tensor build before we can call session.run().
        // Kept on the struct so the placeholder doesn't require a rewrite
        // of callers when the real impl lands.
        #[allow(dead_code)]
        session: Session,
        dims: usize,
    }

    impl OnnxEmbedder {
        pub fn new(model_path: &Path, dims: usize) -> Result<Self, Box<dyn std::error::Error>> {
            let session = Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .commit_from_file(model_path)?;
            Ok(Self { session, dims })
        }

        pub fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
            // WARN: the real ONNX inference path is not wired yet. We return
            // zero-vectors so semantic-search degrades silently to BM25-only
            // rather than panicking. Callers that notice uniformly-tied
            // similarity scores should treat this as a hint that inference
            // is not live. See `generate_embeddings` for the runtime warning.
            let _dummy_input = Array2::<f32>::zeros((1, self.dims));
            let mut results = Vec::with_capacity(texts.len());
            for _ in texts {
                results.push(vec![0.0f32; self.dims]);
            }
            Ok(results)
        }
    }
}

/// Generate embeddings for code snippets.
/// Uses ONNX Runtime when the "embeddings" feature is enabled.
///
/// NOTE: The ONNX code path currently returns zero-vectors (see the TODO in
/// `onnx::OnnxEmbedder::embed`). When the feature is enabled but inference
/// is not yet wired, we emit a one-shot warning so users understand that
/// semantic search will behave as BM25-only.
pub fn generate_embeddings(
    texts: &[String],
    config: &EmbeddingConfig,
) -> Vec<Vec<f32>> {
    #[cfg(feature = "embeddings")]
    {
        use std::sync::atomic::{AtomicBool, Ordering};
        static WARNED: AtomicBool = AtomicBool::new(false);
        if let Some(model_path) = &config.model_path {
            if let Ok(embedder) = onnx::OnnxEmbedder::new(
                std::path::Path::new(model_path),
                config.dimension,
            ) {
                let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
                if let Ok(embeddings) = embedder.embed(&text_refs) {
                    if !WARNED.swap(true, Ordering::Relaxed) {
                        tracing::warn!(
                            model_path = %model_path,
                            "semantic embeddings feature is enabled but ONNX inference is a stub \
                             returning zero-vectors — semantic search will degrade to BM25-only \
                             results. Wire up tokenization + session.run() in \
                             gitnexus-search/src/embeddings/mod.rs to enable full semantic search."
                        );
                    }
                    return embeddings;
                }
            }
        }
    }

    // Fallback: zero vectors
    texts.iter().map(|_| vec![0.0f32; config.dimension]).collect()
}

/// Search for similar embeddings using cosine similarity.
pub fn search_semantic(
    query_embedding: &[f32],
    stored_embeddings: &[(String, Vec<f32>)],
    limit: usize,
) -> Vec<SemanticSearchResult> {
    let mut results: Vec<SemanticSearchResult> = stored_embeddings
        .iter()
        .map(|(id, embedding)| {
            let similarity = cosine_similarity(query_embedding, embedding);
            SemanticSearchResult {
                file_path: String::new(),
                score: similarity,
                rank: 0,
                node_id: id.clone(),
                name: String::new(),
                label: String::new(),
                start_line: None,
                end_line: None,
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);

    for (i, r) in results.iter_mut().enumerate() {
        r.rank = i + 1;
    }

    results
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| (*x as f64) * (*y as f64)).sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0f32, 2.0, 3.0];
        let b = vec![1.0f32, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6, "identical vectors should have similarity ~1.0, got {sim}");
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![0.0f32, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6, "orthogonal vectors should have similarity ~0.0, got {sim}");
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0f32, 0.0];
        let b = vec![-1.0f32, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - (-1.0)).abs() < 1e-6, "opposite vectors should have similarity ~-1.0, got {sim}");
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let sim = cosine_similarity(&[], &[]);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_cosine_similarity_mismatched_lengths() {
        let a = vec![1.0f32, 2.0];
        let b = vec![1.0f32];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![0.0f32, 0.0, 0.0];
        let b = vec![1.0f32, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_search_semantic_ordering() {
        let query = vec![1.0f32, 0.0, 0.0];
        let stored = vec![
            ("node_a".to_string(), vec![0.0f32, 1.0, 0.0]),  // orthogonal
            ("node_b".to_string(), vec![1.0f32, 0.0, 0.0]),  // identical
            ("node_c".to_string(), vec![0.5f32, 0.5, 0.0]),  // partial match
        ];

        let results = search_semantic(&query, &stored, 10);

        assert_eq!(results.len(), 3);
        // node_b should be first (highest similarity)
        assert_eq!(results[0].node_id, "node_b");
        assert_eq!(results[0].rank, 1);
        // node_c should be second
        assert_eq!(results[1].node_id, "node_c");
        assert_eq!(results[1].rank, 2);
        // node_a should be last (orthogonal)
        assert_eq!(results[2].node_id, "node_a");
        assert_eq!(results[2].rank, 3);
    }

    #[test]
    fn test_search_semantic_limit() {
        let query = vec![1.0f32, 0.0];
        let stored: Vec<(String, Vec<f32>)> = (0..10)
            .map(|i| (format!("node_{i}"), vec![1.0f32, i as f32]))
            .collect();

        let results = search_semantic(&query, &stored, 3);
        assert_eq!(results.len(), 3);
        // Ranks should be 1-indexed
        assert_eq!(results[0].rank, 1);
        assert_eq!(results[1].rank, 2);
        assert_eq!(results[2].rank, 3);
    }

    #[test]
    fn test_search_semantic_empty() {
        let query = vec![1.0f32, 0.0];
        let results = search_semantic(&query, &[], 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_generate_embeddings_fallback() {
        let config = EmbeddingConfig::default();
        let texts = vec!["hello world".to_string(), "test".to_string()];
        let embeddings = generate_embeddings(&texts, &config);
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), config.dimension);
        assert_eq!(embeddings[1].len(), config.dimension);
        // Fallback should produce zero vectors
        assert!(embeddings[0].iter().all(|&v| v == 0.0));
    }
}
