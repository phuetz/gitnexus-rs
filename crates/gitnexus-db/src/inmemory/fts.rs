//! In-memory full-text search index with BM25 scoring.

use std::collections::HashMap;

use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_core::graph::types::NodeLabel;

/// BM25 tuning constants.
const BM25_K1: f64 = 1.2;
const BM25_B: f64 = 0.75;

/// Labels that are indexed for full-text search.
const FTS_LABELS: &[NodeLabel] = &[
    NodeLabel::Function,
    NodeLabel::Class,
    NodeLabel::Method,
    NodeLabel::Interface,
    NodeLabel::File,
    NodeLabel::Struct,
    NodeLabel::Trait,
    NodeLabel::Enum,
    NodeLabel::Variable,
    NodeLabel::Type,
    NodeLabel::Module,
    NodeLabel::Route,
    NodeLabel::Tool,
];

/// A single FTS search result.
#[derive(Debug, Clone)]
pub struct FtsResult {
    pub node_id: String,
    pub score: f64,
    pub name: String,
    pub file_path: String,
    pub label: String,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
}

/// In-memory inverted index with BM25 scoring.
pub struct FtsIndex {
    /// term -> Vec<(node_id, term_frequency)>
    inverted: HashMap<String, Vec<(String, u32)>>,
    /// node_id -> document length (number of tokens)
    doc_lengths: HashMap<String, u32>,
    /// Total number of indexed documents.
    doc_count: usize,
    /// Average document length across the corpus.
    avg_doc_len: f64,
}

impl FtsIndex {
    /// Create an empty FTS index.
    pub fn new() -> Self {
        Self {
            inverted: HashMap::new(),
            doc_lengths: HashMap::new(),
            doc_count: 0,
            avg_doc_len: 0.0,
        }
    }

    /// Build an FTS index from a `KnowledgeGraph`.
    ///
    /// Indexes the `name` and `file_path` properties for nodes whose labels
    /// are in `FTS_LABELS`.
    pub fn build(graph: &KnowledgeGraph) -> Self {
        let mut inverted: HashMap<String, Vec<(String, u32)>> = HashMap::new();
        let mut doc_lengths: HashMap<String, u32> = HashMap::new();
        let mut doc_count: usize = 0;
        let mut total_tokens: usize = 0;

        for node in graph.iter_nodes() {
            if !FTS_LABELS.contains(&node.label) {
                continue;
            }

            // Build document text from name + file_path
            let text = format!("{} {}", node.properties.name, node.properties.file_path);
            let tokens = tokenize(&text);
            let doc_len = tokens.len() as u32;

            doc_lengths.insert(node.id.clone(), doc_len);
            doc_count += 1;
            total_tokens += tokens.len();

            // Count term frequencies for this document
            let mut tf_map: HashMap<&str, u32> = HashMap::new();
            for token in &tokens {
                *tf_map.entry(token.as_str()).or_insert(0) += 1;
            }

            for (term, tf) in tf_map {
                inverted
                    .entry(term.to_string())
                    .or_default()
                    .push((node.id.clone(), tf));
            }
        }

        let avg_doc_len = if doc_count > 0 {
            (total_tokens as f64 / doc_count as f64).max(1.0)
        } else {
            1.0
        };

        Self {
            inverted,
            doc_lengths,
            doc_count,
            avg_doc_len,
        }
    }

    /// Search the index with a query string.
    ///
    /// `table_filter` can be a label name (e.g. `"Function"`) to restrict results.
    /// Returns up to `limit` results sorted by BM25 score descending.
    pub fn search(
        &self,
        graph: &KnowledgeGraph,
        query: &str,
        table_filter: Option<&str>,
        limit: usize,
    ) -> Vec<FtsResult> {
        let query_tokens = tokenize(query);
        if query_tokens.is_empty() {
            return Vec::new();
        }

        // Accumulate BM25 scores per document
        let mut scores: HashMap<&str, f64> = HashMap::new();

        for token in &query_tokens {
            if let Some(postings) = self.inverted.get(token.as_str()) {
                let df = postings.len() as f64;
                let idf = ((self.doc_count as f64 - df + 0.5) / (df + 0.5) + 1.0).ln();

                for (doc_id, tf) in postings {
                    let doc_len = *self.doc_lengths.get(doc_id.as_str()).unwrap_or(&1) as f64;
                    let tf_f = *tf as f64;
                    let numerator = tf_f * (BM25_K1 + 1.0);
                    let denominator =
                        tf_f + BM25_K1 * (1.0 - BM25_B + BM25_B * doc_len / self.avg_doc_len);
                    let bm25 = idf * numerator / denominator;

                    *scores.entry(doc_id.as_str()).or_insert(0.0) += bm25;
                }
            }
        }

        // Collect and filter
        let mut results: Vec<(&str, f64)> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut output = Vec::new();
        for (node_id, score) in results {
            if output.len() >= limit {
                break;
            }

            if let Some(node) = graph.get_node(node_id) {
                // Apply table filter
                if let Some(filter) = table_filter {
                    if node.label.as_str() != filter {
                        continue;
                    }
                }

                output.push(FtsResult {
                    node_id: node_id.to_string(),
                    score,
                    name: node.properties.name.clone(),
                    file_path: node.properties.file_path.clone(),
                    label: node.label.as_str().to_string(),
                    start_line: node.properties.start_line,
                    end_line: node.properties.end_line,
                });
            }
        }

        output
    }
}

impl Default for FtsIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Tokenize text: lowercase, split on non-alphanumeric characters, filter empty tokens.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Extract a table/label filter from FTS table names like `"fts_Function"`.
pub fn parse_fts_table_filter(table_name: &str) -> Option<String> {
    table_name.strip_prefix("fts_").map(|stripped| stripped.to_string())
}

/// Convert an `FtsResult` to a `serde_json::Value` row.
pub fn fts_result_to_json(r: &FtsResult) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert("node_id".to_string(), serde_json::Value::String(r.node_id.clone()));
    map.insert("score".to_string(), serde_json::json!(r.score));
    map.insert("name".to_string(), serde_json::Value::String(r.name.clone()));
    map.insert(
        "filePath".to_string(),
        serde_json::Value::String(r.file_path.clone()),
    );
    map.insert("label".to_string(), serde_json::Value::String(r.label.clone()));
    if let Some(sl) = r.start_line {
        map.insert("startLine".to_string(), serde_json::json!(sl));
    }
    if let Some(el) = r.end_line {
        map.insert("endLine".to_string(), serde_json::json!(el));
    }
    serde_json::Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gitnexus_core::graph::types::*;

    fn make_test_graph() -> KnowledgeGraph {
        let mut g = KnowledgeGraph::new();
        g.add_node(GraphNode {
            id: "Function:src/auth.ts:handleLogin".to_string(),
            label: NodeLabel::Function,
            properties: NodeProperties {
                name: "handleLogin".to_string(),
                file_path: "src/auth.ts".to_string(),
                start_line: Some(10),
                end_line: Some(30),
                ..Default::default()
            },
        });
        g.add_node(GraphNode {
            id: "Function:src/auth.ts:validateToken".to_string(),
            label: NodeLabel::Function,
            properties: NodeProperties {
                name: "validateToken".to_string(),
                file_path: "src/auth.ts".to_string(),
                start_line: Some(35),
                end_line: Some(50),
                ..Default::default()
            },
        });
        g.add_node(GraphNode {
            id: "Class:src/user.ts:UserService".to_string(),
            label: NodeLabel::Class,
            properties: NodeProperties {
                name: "UserService".to_string(),
                file_path: "src/user.ts".to_string(),
                start_line: Some(1),
                end_line: Some(100),
                ..Default::default()
            },
        });
        g
    }

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("handleLogin src/auth.ts");
        assert_eq!(tokens, vec!["handlelogin", "src", "auth", "ts"]);
    }

    #[test]
    fn test_build_and_search() {
        let graph = make_test_graph();
        let index = FtsIndex::build(&graph);

        assert!(index.doc_count > 0);

        let results = index.search(&graph, "handleLogin", None, 10);
        assert!(!results.is_empty());
        assert_eq!(results[0].name, "handleLogin");
    }

    #[test]
    fn test_search_with_table_filter() {
        let graph = make_test_graph();
        let index = FtsIndex::build(&graph);

        let results = index.search(&graph, "auth", Some("Function"), 10);
        for r in &results {
            assert_eq!(r.label, "Function");
        }
    }

    #[test]
    fn test_parse_fts_table_filter() {
        assert_eq!(parse_fts_table_filter("fts_Function"), Some("Function".to_string()));
        assert_eq!(parse_fts_table_filter("fts_Class"), Some("Class".to_string()));
        assert_eq!(parse_fts_table_filter("other"), None);
    }

    #[test]
    fn test_empty_query() {
        let graph = make_test_graph();
        let index = FtsIndex::build(&graph);
        let results = index.search(&graph, "", None, 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_bm25_scoring() {
        let graph = make_test_graph();
        let index = FtsIndex::build(&graph);

        // A more specific query should rank the exact match higher
        let results = index.search(&graph, "handleLogin", None, 10);
        assert!(!results.is_empty());
        assert!(results[0].score > 0.0);
    }
}
