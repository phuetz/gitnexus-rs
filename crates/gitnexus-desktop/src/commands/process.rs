use serde::Serialize;
use tauri::State;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessFlow {
    pub id: String,
    pub name: String,
    pub process_type: String,
    pub step_count: u32,
    pub steps: Vec<ProcessStep>,
    pub mermaid: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessStep {
    pub node_id: String,
    pub name: String,
    pub label: String,
    pub file_path: String,
}

#[tauri::command]
pub async fn get_process_flows(
    state: State<'_, AppState>,
) -> Result<Vec<ProcessFlow>, String> {
    let (graph, indexes, _, _) = state.get_repo(None).await?;

    let mut flows = Vec::new();

    // Find Process-labeled nodes
    for node in graph.iter_nodes() {
        if node.label != gitnexus_core::graph::types::NodeLabel::Process {
            continue;
        }

        let name = node.properties.name.clone();
        let process_type = node
            .properties
            .process_type
            .as_ref()
            .map(|pt| format!("{:?}", pt))
            .unwrap_or_else(|| "unknown".to_string());
        let step_count = node.properties.step_count.unwrap_or(0);

        // Follow CALLS relationships from entry_point_id to build step chain
        let mut steps = Vec::new();
        let entry_id = node
            .properties
            .entry_point_id
            .clone()
            .unwrap_or_else(|| node.id.clone());

        // BFS to collect ordered steps
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(entry_id.clone());

        while let Some(current_id) = queue.pop_front() {
            if !visited.insert(current_id.clone()) {
                continue;
            }
            if steps.len() >= 20 {
                break; // safety limit
            }

            if let Some(step_node) = graph.get_node(&current_id) {
                steps.push(ProcessStep {
                    node_id: step_node.id.clone(),
                    name: step_node.properties.name.clone(),
                    label: step_node.label.as_str().to_string(),
                    file_path: step_node.properties.file_path.clone(),
                });

                // Follow outgoing CALLS edges
                if let Some(outs) = indexes.outgoing.get(&current_id) {
                    for (target_id, rel_type) in outs {
                        if rel_type.as_str().contains("Calls")
                            || rel_type.as_str() == "CALLS"
                            || rel_type.as_str() == "CallsAction"
                            || rel_type.as_str() == "CallsService"
                        {
                            if !visited.contains(target_id) {
                                queue.push_back(target_id.clone());
                            }
                        }
                    }
                }
            }
        }

        // Generate Mermaid flowchart
        let mut mermaid = String::from("graph TD\n");
        for (i, step) in steps.iter().enumerate() {
            let safe_name = step.name.replace('"', "'").replace('`', "'");
            let node_id = format!("S{}", i);
            mermaid.push_str(&format!(
                "    {}[\"{} ({})\"]",
                node_id, safe_name, step.label
            ));
            mermaid.push('\n');

            if i + 1 < steps.len() {
                mermaid.push_str(&format!("    {} --> S{}\n", node_id, i + 1));
            }
        }

        if !steps.is_empty() {
            flows.push(ProcessFlow {
                id: node.id.clone(),
                name,
                process_type,
                step_count,
                steps,
                mermaid,
            });
        }
    }

    // Sort by step count desc
    flows.sort_by(|a, b| b.step_count.cmp(&a.step_count));

    Ok(flows)
}
