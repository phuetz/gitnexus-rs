use std::collections::HashMap;

use serde::Serialize;
use tauri::State;

use gitnexus_core::graph::types::{NodeLabel, RelationshipType};

use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverageStats {
    pub total_methods: usize,
    pub traced_methods: usize,
    pub dead_code_candidates: usize,
    pub coverage_pct: f64,
    pub dead_methods: Vec<DeadMethod>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeadMethod {
    pub name: String,
    pub file_path: String,
    pub class_name: Option<String>,
    pub node_id: String,
}

#[tauri::command]
pub async fn get_coverage_stats(
    state: State<'_, AppState>,
) -> Result<CoverageStats, String> {
    let (graph, _, _, _) = state.get_repo(None).await?;

    // Build incoming calls index
    let mut incoming_calls: HashMap<String, Vec<String>> = HashMap::new();
    let mut method_class: HashMap<String, String> = HashMap::new();

    for rel in graph.iter_relationships() {
        match rel.rel_type {
            RelationshipType::Calls | RelationshipType::CallsAction => {
                incoming_calls
                    .entry(rel.target_id.clone())
                    .or_default()
                    .push(rel.source_id.clone());
            }
            RelationshipType::HasMethod => {
                method_class.insert(rel.target_id.clone(), rel.source_id.clone());
            }
            _ => {}
        }
    }

    let all_methods: Vec<_> = graph
        .iter_nodes()
        .filter(|n| n.label == NodeLabel::Method || n.label == NodeLabel::Function)
        .collect();

    let total = all_methods.len();
    let traced = all_methods
        .iter()
        .filter(|n| n.properties.is_traced == Some(true))
        .count();

    let mut dead_methods = Vec::new();
    for method in &all_methods {
        if !incoming_calls.contains_key(&method.id) {
            let class_name = method_class
                .get(&method.id)
                .and_then(|cid| graph.get_node(cid))
                .map(|n| n.properties.name.clone());
            dead_methods.push(DeadMethod {
                name: method.properties.name.clone(),
                file_path: method.properties.file_path.clone(),
                class_name,
                node_id: method.id.clone(),
            });
        }
    }

    // Sort dead methods by file path for consistent output
    dead_methods.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    // Limit to top 50
    dead_methods.truncate(50);

    Ok(CoverageStats {
        total_methods: total,
        traced_methods: traced,
        dead_code_candidates: dead_methods.len(),
        coverage_pct: if total > 0 {
            (traced as f64 / total as f64 * 100.0).round()
        } else {
            0.0
        },
        dead_methods,
    })
}
