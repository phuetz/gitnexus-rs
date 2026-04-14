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
    let (graph, indexes, _, _) = state.get_repo(None).await?;
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
        escape_label(node_name)
    ));

    // Collect methods of this class/controller via the outgoing index (O(degree)).
    // Previously this scanned the full relationship list inside an N*E loop.
    let empty_outgoing: Vec<(String, RelationshipType)> = Vec::new();
    let outgoing = indexes.outgoing.get(node_id).unwrap_or(&empty_outgoing);
    let methods: Vec<String> = outgoing
        .iter()
        .filter(|(_, rel_type)| {
            matches!(
                rel_type,
                RelationshipType::HasMethod | RelationshipType::HasAction
            )
        })
        .map(|(target_id, _)| target_id.clone())
        .collect();

    for method_id in &methods {
        if let Some(method) = graph.get_node(method_id) {
            lines.push(format!(
                "    {} --> {}[\"{}\"]",
                sanitize(node_id),
                sanitize(method_id),
                escape_label(&method.properties.name),
            ));

            // Find outgoing calls from this method via the index
            if let Some(method_outgoing) = indexes.outgoing.get(method_id) {
                for (callee_id, rel_type) in method_outgoing {
                    if !matches!(
                        rel_type,
                        RelationshipType::Calls
                            | RelationshipType::CallsAction
                            | RelationshipType::CallsService
                    ) {
                        continue;
                    }
                    if let Some(callee) = graph.get_node(callee_id) {
                        lines.push(format!(
                            "    {} --> {}[\"{}\"]",
                            sanitize(method_id),
                            sanitize(callee_id),
                            escape_label(&callee.properties.name),
                        ));
                    }
                }
            }
        }
    }

    // If no methods found, show direct relationships from the start node.
    if methods.is_empty() {
        for (target_id, rel_type) in outgoing {
            if !matches!(
                rel_type,
                RelationshipType::Calls
                    | RelationshipType::Imports
                    | RelationshipType::DependsOn
            ) {
                continue;
            }
            if let Some(target_node) = graph.get_node(target_id) {
                lines.push(format!(
                    "    {} -->|{}| {}[\"{}\"]",
                    sanitize(node_id),
                    rel_type.as_str(),
                    sanitize(target_id),
                    escape_label(&target_node.properties.name),
                ));
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

/// Escape a string for inclusion inside a mermaid `["..."]` label.
/// Mermaid does not understand `\"`, so we replace problematic characters with
/// HTML entities (which mermaid renders correctly inside quoted labels).
fn escape_label(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('[', "&#91;")
        .replace(']', "&#93;")
        .replace('(', "&#40;")
        .replace(')', "&#41;")
        .replace('`', "&#96;")
        .replace('{', "&#123;")
        .replace('}', "&#125;")
}
