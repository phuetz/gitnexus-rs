//! Chat Planner — query analysis and research plan generation.
//!
//! Analyzes natural language questions to determine:
//! - Query complexity (simple / medium / complex)
//! - Required tools (search, context, impact, cypher, file read)
//! - Optimal execution plan as a step DAG

use tauri::State;

use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_db::inmemory::fts::FtsIndex;

use crate::state::AppState;
use crate::types::*;

// ─── Helpers ────────────────────────────────────────────────────────

/// Sanitize a string for safe interpolation into Cypher query literals.
/// Escapes single quotes and backslashes to prevent injection.
fn sanitize_cypher_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

// ─── Query Analysis ─────────────────────────────────────────────────

/// Internal query analysis implementation.
pub fn analyze_query_impl(
    question: &str,
    filters: &Option<ChatContextFilter>,
    graph: &KnowledgeGraph,
    fts_index: &FtsIndex,
) -> Result<QueryAnalysis, String> {
    let q = question.to_lowercase();
    let keywords = extract_keywords(&q);

    // Detect intent patterns
    let is_definition = q.contains("what is") || q.contains("qu'est-ce que") || q.contains("explain") || q.contains("describe") || q.contains("définir");
    let is_usage = q.contains("who calls") || q.contains("where is") || q.contains("used by") || q.contains("utilisé") || q.contains("how is") || q.contains("qui appelle");
    let is_impact = q.contains("impact") || q.contains("affect") || q.contains("depends on") || q.contains("dependency") || q.contains("blast radius") || q.contains("dépendance");
    let is_architecture = q.contains("architecture") || q.contains("structure") || q.contains("how do") || q.contains("how does") || q.contains("overview") || q.contains("vue d'ensemble");
    let is_comparison = q.contains("difference") || q.contains("compare") || q.contains("vs") || q.contains("between") || q.contains("comparer");
    let is_refactor = q.contains("refactor") || q.contains("improve") || q.contains("clean up") || q.contains("simplify") || q.contains("améliorer");
    let is_flow = q.contains("flow") || q.contains("pipeline") || q.contains("process") || q.contains("chain") || q.contains("lifecycle") || q.contains("flux");

    // Check if query targets specific symbols
    let has_symbol_match = !keywords.is_empty() && {
        let results = fts_index.search(graph, &keywords.join(" "), None, 3);
        !results.is_empty()
    };

    // Has active filters => more focused, potentially simpler
    let has_filters = filters.as_ref().is_some_and(|f| {
        !f.files.is_empty() || !f.symbols.is_empty() || !f.modules.is_empty()
    });

    // Determine complexity
    let (complexity, mut tools, reasoning) = if is_definition && has_symbol_match {
        (
            QueryComplexity::Simple,
            vec!["search_symbols".to_string(), "get_symbol_context".to_string()],
            "Direct symbol lookup — single search + context retrieval".to_string(),
        )
    } else if is_usage && has_symbol_match {
        (
            QueryComplexity::Medium,
            vec![
                "search_symbols".to_string(),
                "get_symbol_context".to_string(),
                "read_file_content".to_string(),
            ],
            "Usage analysis — find symbol, get context with callers/callees, read source".to_string(),
        )
    } else if is_impact {
        (
            QueryComplexity::Complex,
            vec![
                "search_symbols".to_string(),
                "get_impact_analysis".to_string(),
                "get_symbol_context".to_string(),
                "read_file_content".to_string(),
            ],
            "Impact analysis — find target, compute blast radius, read affected files".to_string(),
        )
    } else if is_architecture || is_flow {
        (
            QueryComplexity::Complex,
            vec![
                "search_symbols".to_string(),
                "execute_cypher".to_string(),
                "get_symbol_context".to_string(),
                "read_file_content".to_string(),
            ],
            "Architecture/flow analysis — multi-step exploration of modules, dependencies, and call chains".to_string(),
        )
    } else if is_comparison {
        (
            QueryComplexity::Medium,
            vec![
                "search_symbols".to_string(),
                "get_symbol_context".to_string(),
                "read_file_content".to_string(),
            ],
            "Comparison — look up both subjects, compare their structure and relationships".to_string(),
        )
    } else if is_refactor {
        (
            QueryComplexity::Complex,
            vec![
                "search_symbols".to_string(),
                "get_symbol_context".to_string(),
                "get_impact_analysis".to_string(),
                "read_file_content".to_string(),
                "execute_cypher".to_string(),
            ],
            "Refactoring analysis — find symbol, analyze dependencies, assess impact, review code".to_string(),
        )
    } else if has_filters && has_symbol_match {
        (
            QueryComplexity::Simple,
            vec!["search_symbols".to_string(), "read_file_content".to_string()],
            "Filtered search — scoped to specific files/symbols".to_string(),
        )
    } else if has_symbol_match {
        (
            QueryComplexity::Medium,
            vec![
                "search_symbols".to_string(),
                "get_symbol_context".to_string(),
                "read_file_content".to_string(),
            ],
            "General question with matching symbols — search, get context, read source".to_string(),
        )
    } else {
        (
            QueryComplexity::Medium,
            vec![
                "search_symbols".to_string(),
                "execute_cypher".to_string(),
            ],
            "Broad question — full-text search + graph query exploration".to_string(),
        )
    };

    // If filtered, add read_file_content if not already there
    if has_filters && !tools.contains(&"read_file_content".to_string()) {
        tools.push("read_file_content".to_string());
    }

    let estimated_steps = match complexity {
        QueryComplexity::Simple => 1 + if has_filters { 1 } else { 0 },
        QueryComplexity::Medium => 2 + if has_filters { 1 } else { 0 },
        QueryComplexity::Complex => tools.len() as u32,
    };

    Ok(QueryAnalysis {
        complexity,
        suggested_tools: tools,
        estimated_steps,
        reasoning,
        keywords,
        needs_cross_file: is_architecture || is_flow || is_comparison || is_impact,
        needs_impact: is_impact || is_refactor,
    })
}

// ─── Research Plan Generation ───────────────────────────────────────

/// Public wrapper for the executor.
pub fn build_research_steps_pub(
    plan_id: &str,
    question: &str,
    analysis: &QueryAnalysis,
    filters: &Option<ChatContextFilter>,
    graph: &KnowledgeGraph,
    fts_index: &FtsIndex,
) -> Vec<ResearchStep> {
    build_research_steps(plan_id, question, analysis, filters, graph, fts_index)
}

/// Build the sequence of research steps based on analysis.
fn build_research_steps(
    plan_id: &str,
    question: &str,
    analysis: &QueryAnalysis,
    filters: &Option<ChatContextFilter>,
    graph: &KnowledgeGraph,
    _fts_index: &FtsIndex,
) -> Vec<ResearchStep> {
    let mut steps = Vec::new();
    let mut order = 0u32;

    // Step 1: Always start with a search
    let search_query = if analysis.keywords.is_empty() {
        question.to_string()
    } else {
        analysis.keywords.join(" ")
    };

    let search_step_id = format!("{}-search", plan_id);
    steps.push(ResearchStep {
        id: search_step_id.clone(),
        order,
        tool: "search_symbols".to_string(),
        description: format!("Search for symbols matching: {}", search_query),
        params: serde_json::json!({
            "query": search_query,
            "limit": 15,
            "filters": filters
        }),
        depends_on: vec![],
        status: StepStatus::Pending,
        result: None,
    });
    order += 1;

    // Step 2: If specific symbols are targeted, get their context
    if analysis.suggested_tools.contains(&"get_symbol_context".to_string()) {
        let ctx_step_id = format!("{}-context", plan_id);
        steps.push(ResearchStep {
            id: ctx_step_id.clone(),
            order,
            tool: "get_symbol_context".to_string(),
            description: "Get 360° context (callers, callees, imports, heritage) for top symbols".to_string(),
            params: serde_json::json!({
                "top_n": 5
            }),
            depends_on: vec![search_step_id.clone()],
            status: StepStatus::Pending,
            result: None,
        });
        order += 1;

        // Step 3: Read file content for the most relevant symbols
        if analysis.suggested_tools.contains(&"read_file_content".to_string()) {
            let read_step_id = format!("{}-read", plan_id);
            steps.push(ResearchStep {
                id: read_step_id.clone(),
                order,
                tool: "read_file_content".to_string(),
                description: "Read source code of the most relevant symbols".to_string(),
                params: serde_json::json!({
                    "from_step": ctx_step_id,
                    "max_files": 5
                }),
                depends_on: vec![ctx_step_id.clone()],
                status: StepStatus::Pending,
                result: None,
            });
            order += 1;
        }
    }

    // Step 4: If impact analysis is needed
    if analysis.needs_impact {
        let impact_step_id = format!("{}-impact", plan_id);
        steps.push(ResearchStep {
            id: impact_step_id.clone(),
            order,
            tool: "get_impact_analysis".to_string(),
            description: "Analyze blast radius and dependency impact of the target symbol".to_string(),
            params: serde_json::json!({
                "direction": "both",
                "max_depth": 3
            }),
            depends_on: vec![search_step_id.clone()],
            status: StepStatus::Pending,
            result: None,
        });
        order += 1;
    }

    // Step 5: If architecture/flow query, add a Cypher exploration step
    if analysis.suggested_tools.contains(&"execute_cypher".to_string()) {
        let cypher_step_id = format!("{}-cypher", plan_id);
        let cypher_query = build_cypher_for_question(question, analysis, graph);

        if let Some(cypher) = cypher_query {
            steps.push(ResearchStep {
                id: cypher_step_id,
                order,
                tool: "execute_cypher".to_string(),
                description: format!("Graph query: {}", cypher),
                params: serde_json::json!({
                    "query": cypher
                }),
                depends_on: vec![search_step_id.clone()],
                status: StepStatus::Pending,
                result: None,
            });
        }
    }

    steps
}

/// Build a Cypher query tailored to the question's intent.
fn build_cypher_for_question(
    question: &str,
    analysis: &QueryAnalysis,
    _graph: &KnowledgeGraph,
) -> Option<String> {
    let q = question.to_lowercase();

    if q.contains("architecture") || q.contains("overview") || q.contains("vue d'ensemble") {
        // Parseur supporte: MATCH (n:Label) RETURN ... — pas de IN ni COUNT(*) AS
        Some("MATCH (n:Module) RETURN n.name, n.file_path LIMIT 30".to_string())
    } else if q.contains("entry point") || q.contains("point d'entrée") {
        // Parseur ne supporte pas IN ni NOT — retourne les fonctions pour tri côté app
        Some("MATCH (n:Function) RETURN n.name, n.file_path LIMIT 30".to_string())
    } else if q.contains("most called") || q.contains("most used") || q.contains("plus utilisé") {
        // Parseur ne supporte pas COUNT(r) AS alias — retourne les appels bruts
        Some("MATCH (n)-[r:CALLS]->(m) RETURN m.name, m.file_path LIMIT 30".to_string())
    } else if q.contains("depend") || q.contains("import") {
        if let Some(keyword) = analysis.keywords.first() {
            let sanitized = sanitize_cypher_string(keyword);
            // Parseur ne supporte pas OR — on filtre sur une seule condition
            Some(format!(
                "MATCH (n)-[r:IMPORTS]->(m) WHERE n.name CONTAINS '{}' RETURN n.name, m.name, n.file_path LIMIT 30",
                sanitized
            ))
        } else {
            Some("MATCH (n)-[r:IMPORTS]->(m) RETURN n.name, m.name LIMIT 30".to_string())
        }
    } else if q.contains("class") || q.contains("inherit") || q.contains("héritage") {
        Some("MATCH (n)-[r:INHERITS]->(m) RETURN n.name, m.name, n.file_path LIMIT 30".to_string())
    } else if !analysis.keywords.is_empty() {
        let kw = sanitize_cypher_string(&analysis.keywords[0]);
        Some(format!(
            "MATCH (n) WHERE n.name CONTAINS '{}' RETURN n.name, n.label, n.file_path LIMIT 20",
            kw
        ))
    } else {
        None
    }
}

// ─── Quick Pick Commands ────────────────────────────────────────────

/// IDE-style file picker — fuzzy search across all indexed files.
#[tauri::command]
pub async fn chat_pick_files(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<FileQuickPick>, String> {
    let (graph, _indexes, _fts_index, _repo_path) = state.get_repo(None).await?;
    let limit = limit.unwrap_or(20);
    let query_lower = query.to_lowercase();

    let mut results: Vec<FileQuickPick> = Vec::new();

    // Collect unique file paths from the graph
    let mut seen_files = std::collections::HashSet::new();
    for node in graph.iter_nodes() {
        let fp = &node.properties.file_path;
        if fp.is_empty() || !seen_files.insert(fp.clone()) {
            continue;
        }

        // Fuzzy match: filename or path contains query
        let fp_lower = fp.to_lowercase();
        let name = std::path::Path::new(fp)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(fp);

        if query.is_empty() || fp_lower.contains(&query_lower) || name.to_lowercase().contains(&query_lower) {
            // Count symbols in this file
            let symbol_count = graph.iter_nodes()
                .filter(|n| n.properties.file_path == *fp && matches!(n.label,
                    NodeLabel::Function | NodeLabel::Method | NodeLabel::Class |
                    NodeLabel::Struct | NodeLabel::Enum | NodeLabel::Interface |
                    NodeLabel::Trait | NodeLabel::TypeAlias | NodeLabel::Constructor
                ))
                .count() as u32;

            // Detect language from extension
            let language = std::path::Path::new(fp)
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_string());

            results.push(FileQuickPick {
                path: fp.clone(),
                name: name.to_string(),
                language,
                symbol_count,
            });
        }
    }

    // Sort: exact filename match first, then by symbol count
    results.sort_by(|a, b| {
        let a_exact = a.name.to_lowercase() == query_lower;
        let b_exact = b.name.to_lowercase() == query_lower;
        b_exact.cmp(&a_exact)
            .then_with(|| b.symbol_count.cmp(&a.symbol_count))
    });

    results.truncate(limit);
    Ok(results)
}

/// IDE-style symbol picker — fuzzy search across all symbols.
#[tauri::command]
pub async fn chat_pick_symbols(
    state: State<'_, AppState>,
    query: String,
    file_filter: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SymbolQuickPick>, String> {
    let (graph, _indexes, fts_index, _repo_path) = state.get_repo(None).await?;
    let limit = limit.unwrap_or(30);

    let mut results: Vec<SymbolQuickPick> = Vec::new();

    if query.is_empty() {
        // No query — list all symbols (optionally filtered by file)
        for node in graph.iter_nodes() {
            if !is_code_symbol(&node.label) {
                continue;
            }
            if let Some(ref ff) = file_filter {
                if !node.properties.file_path.contains(ff.as_str()) {
                    continue;
                }
            }
            results.push(node_to_symbol_pick(node, &graph));
            if results.len() >= limit {
                break;
            }
        }
    } else {
        // Use FTS for fuzzy search
        let fts_results = fts_index.search(&graph, &query, None, limit * 2);

        for fts_result in fts_results {
            if let Some(node) = graph.get_node(&fts_result.node_id) {
                if !is_code_symbol(&node.label) {
                    continue;
                }
                if let Some(ref ff) = file_filter {
                    if !node.properties.file_path.contains(ff.as_str()) {
                        continue;
                    }
                }
                results.push(node_to_symbol_pick(node, &graph));
                if results.len() >= limit {
                    break;
                }
            }
        }

        // Also do exact name match
        let query_lower = query.to_lowercase();
        for node in graph.iter_nodes() {
            if !is_code_symbol(&node.label) {
                continue;
            }
            if node.properties.name.to_lowercase().contains(&query_lower) {
                if let Some(ref ff) = file_filter {
                    if !node.properties.file_path.contains(ff.as_str()) {
                        continue;
                    }
                }
                let pick = node_to_symbol_pick(node, &graph);
                if !results.iter().any(|r| r.node_id == pick.node_id) {
                    results.push(pick);
                }
            }
            if results.len() >= limit {
                break;
            }
        }
    }

    results.truncate(limit);
    Ok(results)
}

/// Module/community picker — search across detected communities.
#[tauri::command]
pub async fn chat_pick_modules(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<ModuleQuickPick>, String> {
    let (graph, _indexes, _fts_index, _repo_path) = state.get_repo(None).await?;
    let limit = limit.unwrap_or(20);
    let query_lower = query.to_lowercase();

    let mut results: Vec<ModuleQuickPick> = Vec::new();

    for node in graph.iter_nodes() {
        if node.label != NodeLabel::Community {
            continue;
        }
        let name = node.properties.heuristic_label.clone()
            .unwrap_or_else(|| node.properties.name.clone());

        if !query.is_empty() && !name.to_lowercase().contains(&query_lower) {
            continue;
        }

        // Count members
        let member_count = graph.iter_relationships()
            .filter(|r| r.rel_type == RelationshipType::MemberOf && r.target_id == node.id)
            .count() as u32;

        results.push(ModuleQuickPick {
            community_id: node.id.clone(),
            name,
            member_count,
            description: node.properties.description.clone(),
        });
    }

    results.sort_by(|a, b| b.member_count.cmp(&a.member_count));
    results.truncate(limit);
    Ok(results)
}

// ─── Helpers ────────────────────────────────────────────────────────

/// Extract meaningful keywords from a question.
fn extract_keywords(question: &str) -> Vec<String> {
    let stop_words: std::collections::HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "shall",
        "should", "may", "might", "must", "can", "could", "to", "of", "in",
        "for", "on", "with", "at", "by", "from", "as", "into", "through",
        "during", "before", "after", "above", "below", "between", "out",
        "this", "that", "these", "those", "it", "its", "my", "your", "his",
        "her", "their", "our", "what", "which", "who", "whom", "where",
        "when", "why", "how", "all", "each", "every", "both", "few",
        "more", "most", "other", "some", "such", "no", "not", "only",
        "and", "but", "or", "if", "about", "up", "there", "than", "very",
        // French stop words
        "le", "la", "les", "un", "une", "des", "du", "de", "et", "est",
        "en", "que", "qui", "dans", "ce", "il", "ne", "sur", "se", "pas",
        "plus", "par", "je", "avec", "tout", "faire", "son", "sont", "comme",
        "mais", "ou", "nous", "vous", "aux", "été", "aussi", "peut",
        "entre", "quoi", "quel", "quelle", "comment", "pourquoi",
    ].iter().copied().collect();

    question
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-' && c != '.')
        .filter(|w| w.len() > 2 && !stop_words.contains(w))
        .map(|w| w.to_string())
        .collect()
}

fn is_code_symbol(label: &NodeLabel) -> bool {
    matches!(label,
        NodeLabel::Function | NodeLabel::Method | NodeLabel::Constructor |
        NodeLabel::Class | NodeLabel::Struct | NodeLabel::Trait |
        NodeLabel::Interface | NodeLabel::Enum | NodeLabel::TypeAlias |
        NodeLabel::Variable | NodeLabel::Const
    )
}

fn node_to_symbol_pick(node: &gitnexus_core::graph::types::GraphNode, graph: &KnowledgeGraph) -> SymbolQuickPick {
    // Try to find container (class/module containing this symbol)
    // Contains: source=parent, target=child → look for target_id == node.id
    // MemberOf: source=member, target=community → look for source_id == node.id
    let container = graph.iter_relationships()
        .find(|r| {
            (r.rel_type == RelationshipType::Contains && r.target_id == node.id)
                || (r.rel_type == RelationshipType::MemberOf && r.source_id == node.id)
        })
        .and_then(|r| {
            // For Contains, the parent is the source; for MemberOf, the community is the target
            let container_id = if r.rel_type == RelationshipType::Contains {
                &r.source_id
            } else {
                &r.target_id
            };
            graph.get_node(container_id)
        })
        .map(|n| n.properties.name.clone());

    SymbolQuickPick {
        node_id: node.id.clone(),
        name: node.properties.name.clone(),
        kind: node.label.as_str().to_string(),
        file_path: node.properties.file_path.clone(),
        container,
        start_line: node.properties.start_line,
    }
}
