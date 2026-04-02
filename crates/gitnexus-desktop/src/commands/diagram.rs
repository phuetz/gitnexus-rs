use serde::Serialize;
use tauri::State;

use gitnexus_core::graph::types::{NodeLabel, RelationshipType};

use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramResult {
    pub mermaid: String,
    pub target_name: String,
    pub target_label: String,
    pub diagram_type: String,
}

#[tauri::command]
pub async fn get_diagram(
    state: State<'_, AppState>,
    target: String,
    diagram_type: Option<String>,
) -> Result<DiagramResult, String> {
    let (graph, _, _, _) = state.get_repo(None).await?;
    let dtype = diagram_type.as_deref().unwrap_or("flowchart");

    // Find the target symbol
    let target_lower = target.to_lowercase();
    let mut candidates: Vec<_> = graph
        .iter_nodes()
        .filter(|n| n.properties.name.to_lowercase() == target_lower)
        .collect();
    candidates.sort_by_key(|n| match n.label {
        NodeLabel::Controller => 0,
        NodeLabel::Class => 1,
        NodeLabel::Service => 2,
        NodeLabel::Interface => 3,
        _ => 10,
    });

    let start_node = candidates
        .first()
        .ok_or_else(|| format!("Symbol '{}' not found", target))?;

    let mut lines = vec!["graph TD".to_string()];
    let node_id = &start_node.id;
    let node_name = &start_node.properties.name;
    lines.push(format!(
        "    {}[\"{}\"]",
        sanitize(node_id),
        node_name
    ));

    // Collect methods of this class/controller
    let methods: Vec<_> = graph
        .iter_relationships()
        .filter(|r| {
            &r.source_id == node_id
                && matches!(
                    r.rel_type,
                    RelationshipType::HasMethod | RelationshipType::HasAction
                )
        })
        .collect();

    for method_rel in &methods {
        if let Some(method) = graph.get_node(&method_rel.target_id) {
            lines.push(format!(
                "    {} --> {}[\"{}\"]",
                sanitize(node_id),
                sanitize(&method_rel.target_id),
                method.properties.name,
            ));

            // Find outgoing calls from this method
            for call in graph.iter_relationships() {
                if call.source_id == method_rel.target_id
                    && matches!(
                        call.rel_type,
                        RelationshipType::Calls
                            | RelationshipType::CallsAction
                            | RelationshipType::CallsService
                    )
                {
                    if let Some(callee) = graph.get_node(&call.target_id) {
                        lines.push(format!(
                            "    {} --> {}[\"{}\"]",
                            sanitize(&call.source_id),
                            sanitize(&call.target_id),
                            callee.properties.name,
                        ));
                    }
                }
            }
        }
    }

    // If no methods found, show direct relationships
    if methods.is_empty() {
        for rel in graph.iter_relationships() {
            if &rel.source_id == node_id
                && matches!(
                    rel.rel_type,
                    RelationshipType::Calls
                        | RelationshipType::Imports
                        | RelationshipType::DependsOn
                )
            {
                if let Some(target_node) = graph.get_node(&rel.target_id) {
                    lines.push(format!(
                        "    {} -->|{}| {}[\"{}\"]",
                        sanitize(node_id),
                        rel.rel_type.as_str(),
                        sanitize(&rel.target_id),
                        target_node.properties.name,
                    ));
                }
            }
        }
    }

    Ok(DiagramResult {
        mermaid: lines.join("\n"),
        target_name: node_name.clone(),
        target_label: start_node.label.as_str().to_string(),
        diagram_type: dtype.to_string(),
    })
}

fn sanitize(id: &str) -> String {
    id.replace([':', '/', '.', ' ', '<', '>', '(', ')', '{', '}'], "_")
}
