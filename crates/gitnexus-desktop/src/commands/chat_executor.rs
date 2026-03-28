//! Chat Executor — executes research plans step by step.
//!
//! Each step runs a specific tool (search, context, impact, cypher, file read)
//! and collects results that feed into subsequent steps.

use std::path::PathBuf;
use std::sync::Mutex;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use once_cell::sync::Lazy;
use tauri::State;

use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_db::inmemory::fts::FtsIndex;

use crate::state::AppState;
use crate::types::*;
use crate::commands::chat::{self};
use crate::commands::chat_planner;

// ─── In-Memory Plan Store ───────────────────────────────────────────

/// Stores plans with insertion-order tracking for proper FIFO eviction.
struct PlanStore {
    plans: HashMap<String, ResearchPlan>,
    insertion_order: VecDeque<String>,
}

static ACTIVE_PLANS: Lazy<Mutex<PlanStore>> =
    Lazy::new(|| Mutex::new(PlanStore { plans: HashMap::new(), insertion_order: VecDeque::new() }));

fn store_plan(plan: &ResearchPlan) {
    if let Ok(mut store) = ACTIVE_PLANS.lock() {
        if store.plans.contains_key(&plan.id) {
            store.insertion_order.retain(|id| id != &plan.id);
        }
        store.plans.insert(plan.id.clone(), plan.clone());
        store.insertion_order.push_back(plan.id.clone());
        // Evict oldest plans beyond capacity
        while store.plans.len() > 10 {
            if let Some(oldest_id) = store.insertion_order.pop_front() {
                store.plans.remove(&oldest_id);
            }
        }
    }
}

fn get_plan(plan_id: &str) -> Option<ResearchPlan> {
    ACTIVE_PLANS.lock().ok().and_then(|store| store.plans.get(plan_id).cloned())
}

fn update_plan(plan: &ResearchPlan) {
    if let Ok(mut store) = ACTIVE_PLANS.lock() {
        store.plans.insert(plan.id.clone(), plan.clone());
    }
}

// ─── Execute Single Step ────────────────────────────────────────────

/// Execute a single step of a research plan.
#[tauri::command]
pub async fn chat_execute_step(
    state: State<'_, AppState>,
    plan_id: String,
    step_id: String,
) -> Result<StepResult, String> {
    let (graph, _indexes, fts_index, repo_path_str) = state.get_repo(None).await?;
    let repo_path = PathBuf::from(&repo_path_str);

    let mut plan = get_plan(&plan_id).ok_or_else(|| format!("Plan {} not found", plan_id))?;

    let step_idx = plan.steps.iter().position(|s| s.id == step_id)
        .ok_or_else(|| format!("Step {} not found in plan {}", step_id, plan_id))?;

    // Check dependencies are completed
    let depends = plan.steps[step_idx].depends_on.clone();
    for dep_id in &depends {
        let dep_step = plan.steps.iter().find(|s| s.id == *dep_id);
        if let Some(dep) = dep_step {
            if dep.status != StepStatus::Completed {
                return Err(format!("Dependency step {} is not completed yet", dep_id));
            }
        }
    }

    // Mark step as running
    plan.steps[step_idx].status = StepStatus::Running;
    plan.status = PlanStatus::Running;
    update_plan(&plan);

    let start = Instant::now();

    // Collect results from dependency steps
    let dep_results: Vec<&StepResult> = plan.steps.iter()
        .filter(|s| depends.contains(&s.id))
        .filter_map(|s| s.result.as_ref())
        .collect();

    // Execute the tool
    let result = execute_tool(
        &plan.steps[step_idx],
        &dep_results,
        &graph,
        &fts_index,
        &repo_path,
    );

    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(mut step_result) => {
            step_result.duration_ms = duration_ms;
            plan.steps[step_idx].status = StepStatus::Completed;
            plan.steps[step_idx].result = Some(step_result.clone());

            // Check if all steps are done
            if plan.steps.iter().all(|s| s.status == StepStatus::Completed || s.status == StepStatus::Skipped) {
                plan.status = PlanStatus::Completed;
            }

            update_plan(&plan);
            Ok(step_result)
        }
        Err(e) => {
            plan.steps[step_idx].status = StepStatus::Failed;
            plan.steps[step_idx].result = Some(StepResult {
                summary: format!("Failed: {}", e),
                sources: vec![],
                data: None,
                duration_ms,
            });

            // Don't fail the whole plan — skip dependent steps
            for i in 0..plan.steps.len() {
                if plan.steps[i].depends_on.contains(&step_id) {
                    plan.steps[i].status = StepStatus::Skipped;
                }
            }

            update_plan(&plan);
            Err(e)
        }
    }
}

/// Execute a tool for a research step.
fn execute_tool(
    step: &ResearchStep,
    dep_results: &[&StepResult],
    graph: &KnowledgeGraph,
    fts_index: &FtsIndex,
    repo_path: &PathBuf,
) -> Result<StepResult, String> {
    match step.tool.as_str() {
        "search_symbols" => execute_search(step, graph, fts_index, repo_path),
        "get_symbol_context" => execute_context(step, dep_results, graph),
        "get_impact_analysis" => execute_impact(step, dep_results, graph),
        "read_file_content" => execute_read_file(step, dep_results, graph, repo_path),
        "execute_cypher" => execute_cypher_step(step, graph, fts_index),
        _ => Err(format!("Unknown tool: {}", step.tool)),
    }
}

// ─── Tool Executors ─────────────────────────────────────────────────

fn execute_search(
    step: &ResearchStep,
    graph: &KnowledgeGraph,
    fts_index: &FtsIndex,
    repo_path: &PathBuf,
) -> Result<StepResult, String> {
    let query = step.params["query"].as_str().unwrap_or("");
    let limit = step.params["limit"].as_u64().unwrap_or(15) as usize;

    // Apply filters if present
    let filters: Option<ChatContextFilter> = step.params.get("filters")
        .and_then(|f| serde_json::from_value(f.clone()).ok());

    let fts_results = fts_index.search(graph, query, None, limit * 2);

    let mut results: Vec<(String, f64)> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for fts_result in fts_results {
        if !seen.insert(fts_result.node_id.clone()) {
            continue;
        }

        // Apply filters
        if let Some(ref f) = filters {
            if let Some(node) = graph.get_node(&fts_result.node_id) {
                if !f.files.is_empty() && !f.files.iter().any(|fp| {
                    node.properties.file_path == *fp || node.properties.file_path.ends_with(&format!("/{}", fp))
                }) {
                    continue;
                }
                if !f.labels.is_empty() && !f.labels.iter().any(|l| node.label.as_str() == l.as_str()) {
                    continue;
                }
                if !f.languages.is_empty() {
                    let ext = std::path::Path::new(&node.properties.file_path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    if !f.languages.iter().any(|l| l == ext) {
                        continue;
                    }
                }
            }
        }

        results.push((fts_result.node_id, fts_result.score));
        if results.len() >= limit {
            break;
        }
    }

    // Build sources
    let sources = build_sources_from_results(&results, graph, repo_path);

    let summary = if sources.is_empty() {
        format!("No symbols found matching '{}'", query)
    } else {
        format!("Found {} symbols matching '{}': {}",
            sources.len(), query,
            sources.iter().take(5).map(|s| format!("`{}`", s.symbol_name)).collect::<Vec<_>>().join(", ")
        )
    };

    Ok(StepResult {
        summary,
        sources,
        data: Some(serde_json::json!({ "result_count": results.len() })),
        duration_ms: 0,
    })
}

fn execute_context(
    step: &ResearchStep,
    dep_results: &[&StepResult],
    graph: &KnowledgeGraph,
) -> Result<StepResult, String> {
    let top_n = step.params["top_n"].as_u64().unwrap_or(5) as usize;

    // Get node IDs from dependency results
    let node_ids: Vec<String> = dep_results.iter()
        .flat_map(|r| r.sources.iter().map(|s| s.node_id.clone()))
        .take(top_n)
        .collect();

    if node_ids.is_empty() {
        return Ok(StepResult {
            summary: "No symbols to get context for".to_string(),
            sources: vec![],
            data: None,
            duration_ms: 0,
        });
    }

    let mut context_data = Vec::new();
    let mut all_related_names = Vec::new();

    for node_id in &node_ids {
        if let Some(node) = graph.get_node(node_id) {
            let mut callers = Vec::new();
            let mut callees = Vec::new();
            let mut imports = Vec::new();

            for rel in graph.iter_relationships() {
                if rel.source_id == *node_id {
                    if let Some(target) = graph.get_node(&rel.target_id) {
                        match rel.rel_type {
                            RelationshipType::Calls => callees.push(target.properties.name.clone()),
                            RelationshipType::Imports => imports.push(target.properties.name.clone()),
                            _ => {}
                        }
                    }
                } else if rel.target_id == *node_id && rel.rel_type == RelationshipType::Calls {
                    if let Some(source) = graph.get_node(&rel.source_id) {
                        callers.push(source.properties.name.clone());
                    }
                }
            }

            all_related_names.extend(callers.iter().cloned());
            all_related_names.extend(callees.iter().cloned());

            context_data.push(serde_json::json!({
                "symbol": node.properties.name,
                "label": node.label.as_str(),
                "file": node.properties.file_path,
                "callers": callers,
                "callees": callees,
                "imports": imports,
            }));
        }
    }

    let summary = format!(
        "Retrieved context for {} symbols, found {} unique callers/callees",
        node_ids.len(),
        all_related_names.len()
    );

    // Keep the sources from dependency steps
    let sources: Vec<ChatSource> = dep_results.iter()
        .flat_map(|r| r.sources.clone())
        .take(top_n)
        .collect();

    Ok(StepResult {
        summary,
        sources,
        data: Some(serde_json::json!({ "contexts": context_data })),
        duration_ms: 0,
    })
}

fn execute_impact(
    step: &ResearchStep,
    dep_results: &[&StepResult],
    graph: &KnowledgeGraph,
) -> Result<StepResult, String> {
    let direction = step.params["direction"].as_str().unwrap_or("both");
    let max_depth = step.params["max_depth"].as_u64().unwrap_or(3) as u32;

    // Get the first symbol from dependencies
    let target_id = dep_results.iter()
        .flat_map(|r| r.sources.iter())
        .next()
        .map(|s| s.node_id.clone())
        .ok_or_else(|| "No target symbol for impact analysis".to_string())?;

    let target_node = graph.get_node(&target_id)
        .ok_or_else(|| format!("Node {} not found", target_id))?;

    // BFS for upstream and downstream
    let mut upstream = Vec::new();
    let mut downstream = Vec::new();
    let mut affected_files = std::collections::HashSet::new();

    affected_files.insert(target_node.properties.file_path.clone());

    // Downstream: what does this symbol call/affect? (BFS)
    if direction == "both" || direction == "downstream" {
        let mut queue = VecDeque::from(vec![(target_id.clone(), 0u32)]);
        let mut visited = std::collections::HashSet::new();
        visited.insert(target_id.clone());

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for rel in graph.iter_relationships() {
                if rel.source_id == current_id && (rel.rel_type == RelationshipType::Calls || rel.rel_type == RelationshipType::Imports) {
                    if visited.insert(rel.target_id.clone()) {
                        if let Some(node) = graph.get_node(&rel.target_id) {
                            affected_files.insert(node.properties.file_path.clone());
                            downstream.push(node.properties.name.clone());
                            queue.push_back((rel.target_id.clone(), depth + 1));
                        }
                    }
                }
            }
        }
    }

    // Upstream: what calls/uses this symbol? (BFS)
    if direction == "both" || direction == "upstream" {
        let mut queue = VecDeque::from(vec![(target_id.clone(), 0u32)]);
        let mut visited = std::collections::HashSet::new();
        visited.insert(target_id.clone());

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for rel in graph.iter_relationships() {
                if rel.target_id == current_id && (rel.rel_type == RelationshipType::Calls || rel.rel_type == RelationshipType::Imports) {
                    if visited.insert(rel.source_id.clone()) {
                        if let Some(node) = graph.get_node(&rel.source_id) {
                            affected_files.insert(node.properties.file_path.clone());
                            upstream.push(node.properties.name.clone());
                            queue.push_back((rel.source_id.clone(), depth + 1));
                        }
                    }
                }
            }
        }
    }

    let summary = format!(
        "Impact of `{}`: {} upstream, {} downstream, {} affected files",
        target_node.properties.name,
        upstream.len(),
        downstream.len(),
        affected_files.len()
    );

    let sources = dep_results.iter()
        .flat_map(|r| r.sources.clone())
        .collect();

    Ok(StepResult {
        summary,
        sources,
        data: Some(serde_json::json!({
            "target": target_node.properties.name,
            "upstream": upstream,
            "downstream": downstream,
            "affected_files": affected_files.iter().collect::<Vec<_>>(),
        })),
        duration_ms: 0,
    })
}

fn execute_read_file(
    step: &ResearchStep,
    dep_results: &[&StepResult],
    graph: &KnowledgeGraph,
    repo_path: &PathBuf,
) -> Result<StepResult, String> {
    let max_files = step.params["max_files"].as_u64().unwrap_or(5) as usize;

    // Collect unique file paths from dependency sources
    let mut file_paths: Vec<String> = dep_results.iter()
        .flat_map(|r| r.sources.iter().map(|s| s.file_path.clone()))
        .collect();

    // Deduplicate
    file_paths.sort();
    file_paths.dedup();
    file_paths.truncate(max_files);

    let mut snippets = Vec::new();
    let mut updated_sources: Vec<ChatSource> = dep_results.iter()
        .flat_map(|r| r.sources.clone())
        .collect();

    for fp in &file_paths {
        let full_path = repo_path.join(fp);
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            let lines: Vec<&str> = content.lines().collect();
            let total_lines = lines.len();

            // Find symbols in this file from sources
            let file_symbols: Vec<&ChatSource> = updated_sources.iter()
                .filter(|s| s.file_path == *fp)
                .collect();

            if file_symbols.is_empty() {
                // Show first 50 lines
                let end = std::cmp::min(50, lines.len());
                let preview: String = lines[..end].join("\n");
                snippets.push(serde_json::json!({
                    "file": fp,
                    "total_lines": total_lines,
                    "snippet": preview,
                }));
            } else {
                // Show code around each symbol
                for sym in file_symbols {
                    if let (Some(start), Some(end)) = (sym.start_line, sym.end_line) {
                        let s = std::cmp::min((start.saturating_sub(1)) as usize, lines.len());
                        let e = std::cmp::min(end as usize, lines.len());
                        let e = std::cmp::min(e, s + 60); // Max 60 lines per snippet
                        if s >= e { continue; }
                        let snippet: String = lines[s..e].join("\n");
                        snippets.push(serde_json::json!({
                            "file": fp,
                            "symbol": sym.symbol_name,
                            "start_line": start,
                            "end_line": end,
                            "snippet": snippet,
                        }));
                    }
                }
            }
        }
    }

    // Update source snippets
    for source in &mut updated_sources {
        let full_path = repo_path.join(&source.file_path);
        if source.snippet.is_none() {
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                let lines: Vec<&str> = content.lines().collect();
                if let (Some(start), Some(end)) = (source.start_line, source.end_line) {
                    let s = std::cmp::min((start.saturating_sub(1)) as usize, lines.len());
                    let e = std::cmp::min(end as usize, lines.len());
                    let e = std::cmp::min(e, s + 50);
                    if s < e {
                        source.snippet = Some(lines[s..e].join("\n"));
                    }
                }
            }
        }
    }

    let summary = format!("Read {} files, extracted {} code snippets", file_paths.len(), snippets.len());

    Ok(StepResult {
        summary,
        sources: updated_sources,
        data: Some(serde_json::json!({ "snippets": snippets })),
        duration_ms: 0,
    })
}

fn execute_cypher_step(
    step: &ResearchStep,
    graph: &KnowledgeGraph,
    fts_index: &FtsIndex,
) -> Result<StepResult, String> {
    let query = step.params["query"].as_str()
        .ok_or_else(|| "No Cypher query provided".to_string())?;

    // Parse the Cypher query
    let stmt = gitnexus_db::inmemory::cypher::parse(query)
        .map_err(|e| format!("Cypher parse error: {}", e))?;

    // Build indexes needed for the Cypher executor
    let indexes = gitnexus_db::inmemory::cypher::GraphIndexes::build(graph);

    // Execute the parsed statement
    let results = gitnexus_db::inmemory::cypher::execute(&stmt, graph, &indexes, fts_index)
        .map_err(|e| format!("Cypher execution failed: {}", e))?;

    let row_count = results.len();
    let summary = format!("Cypher query returned {} rows", row_count);

    Ok(StepResult {
        summary,
        sources: vec![],
        data: Some(serde_json::json!({
            "query": query,
            "rows": results,
            "row_count": row_count,
        })),
        duration_ms: 0,
    })
}

// ─── Full Plan Execution ────────────────────────────────────────────

/// Execute a full research plan and generate an LLM-powered answer.
#[tauri::command]
pub async fn chat_execute_plan(
    state: State<'_, AppState>,
    request: ChatSmartRequest,
) -> Result<ChatSmartResponse, String> {
    let config = chat::load_config_pub();
    let (graph, _indexes, fts_index, repo_path_str) = state.get_repo(None).await?;
    let repo_path = PathBuf::from(&repo_path_str);

    // 1. Analyze the query
    let analysis = chat_planner::analyze_query_impl(
        &request.question, &request.filters, &graph, &fts_index
    )?;

    // 2. For simple queries or no deep research, use the standard chat path
    if analysis.complexity == QueryComplexity::Simple && !request.deep_research {
        let search_results = crate::commands::chat::search_relevant_context_pub(
            &request.question, &graph, &fts_index, 10
        );
        let sources = build_sources_from_results(&search_results, &graph, &repo_path);

        // Build and call LLM
        let answer = if config.api_key.is_empty() && !config.base_url.contains("localhost") {
            build_simple_answer(&sources)
        } else {
            let system_prompt = build_research_prompt(&request.question, &sources, &[]);
            let messages = build_llm_messages(&system_prompt, &request.history, &request.question);
            chat::call_llm_pub(&config, &messages).await?
        };

        return Ok(ChatSmartResponse {
            answer,
            sources,
            model: Some(config.model.clone()),
            plan: None,
            complexity: analysis.complexity,
        });
    }

    // 3. For medium/complex queries, build and execute a research plan
    let plan_id = format!("plan-{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis());

    let steps = chat_planner::build_research_steps_pub(
        &plan_id, &request.question, &analysis, &request.filters, &graph, &fts_index
    );

    let mut plan = ResearchPlan {
        id: plan_id.clone(),
        query: request.question.clone(),
        analysis: analysis.clone(),
        steps,
        status: PlanStatus::Running,
    };

    store_plan(&plan);

    // Execute steps in dependency order
    let step_order: Vec<(usize, Vec<String>)> = plan.steps.iter()
        .enumerate()
        .map(|(i, s)| (i, s.depends_on.clone()))
        .collect();

    for (idx, depends) in &step_order {
        // Check all dependencies are completed
        let deps_ok = depends.iter().all(|dep_id| {
            plan.steps.iter().find(|s| s.id == *dep_id)
                .map_or(false, |s| s.status == StepStatus::Completed)
        });

        if !deps_ok && !depends.is_empty() {
            plan.steps[*idx].status = StepStatus::Skipped;
            continue;
        }

        plan.steps[*idx].status = StepStatus::Running;
        update_plan(&plan);

        let start = Instant::now();

        let dep_results: Vec<&StepResult> = plan.steps.iter()
            .filter(|s| depends.contains(&s.id))
            .filter_map(|s| s.result.as_ref())
            .collect();

        let result = execute_tool(
            &plan.steps[*idx],
            &dep_results,
            &graph,
            &fts_index,
            &repo_path,
        );

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(mut r) => {
                r.duration_ms = duration_ms;
                plan.steps[*idx].status = StepStatus::Completed;
                plan.steps[*idx].result = Some(r);
            }
            Err(e) => {
                plan.steps[*idx].status = StepStatus::Failed;
                plan.steps[*idx].result = Some(StepResult {
                    summary: format!("Failed: {}", e),
                    sources: vec![],
                    data: None,
                    duration_ms,
                });
            }
        }

        update_plan(&plan);
    }

    plan.status = PlanStatus::Completed;
    update_plan(&plan);

    // 4. Collect all sources from completed steps
    let all_sources: Vec<ChatSource> = plan.steps.iter()
        .filter(|s| s.status == StepStatus::Completed)
        .filter_map(|s| s.result.as_ref())
        .flat_map(|r| r.sources.clone())
        .collect();

    // Deduplicate sources by node_id
    let mut seen = std::collections::HashSet::new();
    let unique_sources: Vec<ChatSource> = all_sources.into_iter()
        .filter(|s| seen.insert(s.node_id.clone()))
        .collect();

    // 5. Collect step summaries for the LLM context
    let step_summaries: Vec<String> = plan.steps.iter()
        .filter(|s| s.status == StepStatus::Completed)
        .filter_map(|s| s.result.as_ref().map(|r| format!("- {}: {}", s.description, r.summary)))
        .collect();

    // 6. Generate final answer with LLM
    let answer = if config.api_key.is_empty() && !config.base_url.contains("localhost") {
        build_research_answer(&unique_sources, &step_summaries)
    } else {
        let system_prompt = build_research_prompt(&request.question, &unique_sources, &step_summaries);
        let messages = build_llm_messages(&system_prompt, &request.history, &request.question);
        chat::call_llm_pub(&config, &messages).await?
    };

    Ok(ChatSmartResponse {
        answer,
        sources: unique_sources,
        model: Some(config.model.clone()),
        plan: Some(plan),
        complexity: analysis.complexity,
    })
}

// ─── Helpers ────────────────────────────────────────────────────────

fn build_sources_from_results(
    results: &[(String, f64)],
    graph: &KnowledgeGraph,
    repo_path: &PathBuf,
) -> Vec<ChatSource> {
    let mut sources = Vec::new();

    for (node_id, score) in results {
        let node = match graph.get_node(node_id) {
            Some(n) => n,
            None => continue,
        };

        match node.label {
            NodeLabel::Function | NodeLabel::Method | NodeLabel::Constructor |
            NodeLabel::Class | NodeLabel::Struct | NodeLabel::Trait |
            NodeLabel::Interface | NodeLabel::Enum | NodeLabel::TypeAlias => {}
            _ => continue,
        }

        let snippet = read_snippet(repo_path, &node.properties.file_path, node.properties.start_line, node.properties.end_line);

        let mut callers = Vec::new();
        let mut callees = Vec::new();
        for rel in graph.iter_relationships() {
            if rel.rel_type == RelationshipType::Calls {
                if rel.source_id == *node_id {
                    if let Some(target) = graph.get_node(&rel.target_id) {
                        callees.push(target.properties.name.clone());
                    }
                } else if rel.target_id == *node_id {
                    if let Some(source) = graph.get_node(&rel.source_id) {
                        callers.push(source.properties.name.clone());
                    }
                }
            }
        }

        let community = graph.iter_relationships()
            .find(|r| r.rel_type == RelationshipType::MemberOf && r.source_id == *node_id)
            .and_then(|r| graph.get_node(&r.target_id))
            .map(|c| c.properties.heuristic_label.clone().unwrap_or_else(|| c.properties.name.clone()));

        sources.push(ChatSource {
            node_id: node_id.clone(),
            symbol_name: node.properties.name.clone(),
            symbol_type: node.label.as_str().to_string(),
            file_path: node.properties.file_path.clone(),
            start_line: node.properties.start_line,
            end_line: node.properties.end_line,
            snippet,
            callers: if callers.is_empty() { None } else { Some(callers) },
            callees: if callees.is_empty() { None } else { Some(callees) },
            community,
            relevance_score: *score,
        });
    }

    sources
}

fn read_snippet(repo_path: &PathBuf, file_path: &str, start: Option<u32>, end: Option<u32>) -> Option<String> {
    let full_path = repo_path.join(file_path);
    let content = std::fs::read_to_string(&full_path).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    match (start, end) {
        (Some(s), Some(e)) => {
            let s = std::cmp::min((s.saturating_sub(1)) as usize, lines.len());
            let e = std::cmp::min(e as usize, lines.len());
            let e = std::cmp::min(e, s + 50);
            if s >= e { return None; }
            Some(lines[s..e].join("\n"))
        }
        (Some(s), None) => {
            let s = std::cmp::min((s.saturating_sub(1)) as usize, lines.len());
            let e = std::cmp::min(s + 20, lines.len());
            if s >= e { return None; }
            Some(lines[s..e].join("\n"))
        }
        _ => {
            let e = std::cmp::min(30, lines.len());
            Some(lines[..e].join("\n"))
        }
    }
}

fn build_research_prompt(question: &str, sources: &[ChatSource], step_summaries: &[String]) -> String {
    let mut prompt = String::from(
        "You are an expert code analyst. You have performed a multi-step research plan to answer the developer's question.\n\n"
    );

    if !step_summaries.is_empty() {
        prompt.push_str("## Research Steps Completed\n\n");
        for summary in step_summaries {
            prompt.push_str(summary);
            prompt.push('\n');
        }
        prompt.push('\n');
    }

    prompt.push_str("## Relevant Code Context\n\n");
    for (i, source) in sources.iter().enumerate().take(10) {
        prompt.push_str(&format!(
            "### {} — `{}` ({}) in `{}`\n",
            i + 1, source.symbol_name, source.symbol_type, source.file_path
        ));
        if let Some(community) = &source.community {
            prompt.push_str(&format!("**Module**: {}\n", community));
        }
        if let Some(callers) = &source.callers {
            prompt.push_str(&format!("**Called by**: {}\n", callers.join(", ")));
        }
        if let Some(callees) = &source.callees {
            prompt.push_str(&format!("**Calls**: {}\n", callees.join(", ")));
        }
        if let Some(snippet) = &source.snippet {
            let lang = detect_lang_from_path(&source.file_path);
            prompt.push_str(&format!("\n```{}\n{}\n```\n\n", lang, snippet));
        }
    }

    prompt.push_str(
        "## Instructions\n\n\
         - Synthesize findings from all research steps into a comprehensive answer.\n\
         - Reference specific symbols, files, and line numbers.\n\
         - Use markdown code blocks with the correct language.\n\
         - Be concise but thorough.\n\
         - Respond in the same language as the user's question.\n"
    );

    prompt
}

fn build_llm_messages(
    system_prompt: &str,
    history: &[ChatMessage],
    question: &str,
) -> Vec<serde_json::Value> {
    let mut messages = vec![serde_json::json!({
        "role": "system",
        "content": system_prompt
    })];

    for msg in history.iter().rev().take(10).rev() {
        messages.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content
        }));
    }

    messages.push(serde_json::json!({
        "role": "user",
        "content": question
    }));

    messages
}

fn build_simple_answer(sources: &[ChatSource]) -> String {
    let mut answer = String::from("## Results\n\n*No LLM configured. Showing graph search results.*\n\n");
    for source in sources.iter().take(10) {
        answer.push_str(&format!("- **`{}`** ({}) in `{}`", source.symbol_name, source.symbol_type, source.file_path));
        if let Some(community) = &source.community {
            answer.push_str(&format!(" — module: {}", community));
        }
        answer.push('\n');
    }
    answer
}

fn build_research_answer(sources: &[ChatSource], step_summaries: &[String]) -> String {
    let mut answer = String::from("## Research Results\n\n*No LLM configured. Showing research plan results.*\n\n");

    if !step_summaries.is_empty() {
        answer.push_str("### Steps Completed\n\n");
        for summary in step_summaries {
            answer.push_str(summary);
            answer.push('\n');
        }
        answer.push('\n');
    }

    answer.push_str("### Relevant Symbols\n\n");
    for source in sources.iter().take(10) {
        answer.push_str(&format!("- **`{}`** ({}) in `{}`\n", source.symbol_name, source.symbol_type, source.file_path));
    }
    answer
}

fn detect_lang_from_path(file_path: &str) -> &str {
    match file_path.rsplit('.').next() {
        Some("rs") => "rust",
        Some("js" | "mjs" | "cjs") => "javascript",
        Some("ts" | "mts" | "cts") => "typescript",
        Some("tsx") => "tsx",
        Some("jsx") => "jsx",
        Some("py" | "pyi") => "python",
        Some("java") => "java",
        Some("cs") => "csharp",
        Some("go") => "go",
        Some("rb") => "ruby",
        Some("php") => "php",
        Some("kt" | "kts") => "kotlin",
        Some("swift") => "swift",
        Some("c" | "h") => "c",
        Some("cpp" | "hpp" | "cc" | "hh" | "cxx" | "hxx") => "cpp",
        Some("razor" | "cshtml") => "razor",
        Some("json") => "json",
        Some("toml") => "toml",
        Some("yaml" | "yml") => "yaml",
        Some("md") => "markdown",
        _ => "",
    }
}
