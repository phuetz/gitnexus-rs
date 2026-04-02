use tauri::State;

use gitnexus_core::graph::types::{NodeLabel, RelationshipType};

use crate::state::AppState;
use crate::types::*;

/// Get graph data filtered by zoom level.
#[tauri::command]
pub async fn get_graph_data(
    state: State<'_, AppState>,
    filter: GraphFilter,
) -> Result<GraphPayload, String> {
    let (graph, _indexes, _fts, _repo_path) = state.get_repo(None).await?;

    let max_nodes = filter.max_nodes.unwrap_or(500);

    // Determine which node labels to include based on zoom level
    let allowed_labels = match filter.zoom_level {
        ZoomLevel::Package => vec![
            NodeLabel::Folder,
            NodeLabel::Package,
            NodeLabel::Project,
        ],
        ZoomLevel::Module => vec![
            NodeLabel::File,
            NodeLabel::Module,
            NodeLabel::Folder,
        ],
        ZoomLevel::Symbol => vec![
            NodeLabel::Function,
            NodeLabel::Class,
            NodeLabel::Method,
            NodeLabel::Interface,
            NodeLabel::Struct,
            NodeLabel::Trait,
            NodeLabel::Enum,
            NodeLabel::Variable,
            NodeLabel::Type,
            NodeLabel::Const,
            NodeLabel::Constructor,
            NodeLabel::Property,
            NodeLabel::Namespace,
            NodeLabel::Route,
            NodeLabel::Tool,
        ],
    };

    let allowed_rel_types = match filter.zoom_level {
        ZoomLevel::Package => vec![RelationshipType::Contains],
        ZoomLevel::Module => vec![RelationshipType::Contains, RelationshipType::Imports],
        ZoomLevel::Symbol => vec![
            RelationshipType::Calls,
            RelationshipType::Uses,
            RelationshipType::Imports,
            RelationshipType::Inherits,
            RelationshipType::Implements,
            RelationshipType::Extends,
        ],
    };

    // Custom label filter
    let custom_labels: Option<Vec<NodeLabel>> = filter.labels.as_ref().map(|labels| {
        labels
            .iter()
            .filter_map(|l| NodeLabel::from_str_label(l))
            .collect()
    });

    // Collect nodes
    let mut nodes = Vec::new();
    for node in graph.iter_nodes() {
        let label_ok = if let Some(ref custom) = custom_labels {
            custom.contains(&node.label)
        } else {
            allowed_labels.contains(&node.label)
        };

        if !label_ok {
            continue;
        }

        // File path filter
        if let Some(ref paths) = filter.file_paths {
            if !paths.iter().any(|p| node.properties.file_path.starts_with(p)) {
                continue;
            }
        }

        nodes.push(node_to_cyto(node));

        if nodes.len() >= max_nodes {
            break;
        }
    }

    // Collect node ID set for edge filtering
    let node_ids: std::collections::HashSet<&str> =
        nodes.iter().map(|n| n.id.as_str()).collect();

    // Collect edges
    let mut edges = Vec::new();
    for rel in graph.iter_relationships() {
        if !allowed_rel_types.contains(&rel.rel_type) {
            continue;
        }
        if node_ids.contains(rel.source_id.as_str())
            && node_ids.contains(rel.target_id.as_str())
        {
            edges.push(rel_to_cyto(rel));
        }
    }

    let truncated = nodes.len() >= max_nodes;
    let stats = GraphStats {
        node_count: nodes.len(),
        edge_count: edges.len(),
        truncated,
    };

    Ok(GraphPayload {
        nodes,
        edges,
        stats,
    })
}

/// Get a subgraph centered on a node, expanding to a given depth.
#[tauri::command]
pub async fn get_subgraph(
    state: State<'_, AppState>,
    center_node_id: String,
    depth: Option<u32>,
) -> Result<GraphPayload, String> {
    let (graph, indexes, _fts, _repo_path) = state.get_repo(None).await?;
    let max_depth = depth.unwrap_or(2);

    // BFS to collect neighborhood
    let mut visited = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();

    visited.insert(center_node_id.clone());
    queue.push_back((center_node_id, 0u32));

    while let Some((node_id, d)) = queue.pop_front() {
        if d >= max_depth {
            continue;
        }

        // Outgoing neighbors
        if let Some(outs) = indexes.outgoing.get(&node_id) {
            for (target, _) in outs {
                if visited.insert(target.clone()) {
                    queue.push_back((target.clone(), d + 1));
                }
            }
        }

        // Incoming neighbors
        if let Some(ins) = indexes.incoming.get(&node_id) {
            for (source, _) in ins {
                if visited.insert(source.clone()) {
                    queue.push_back((source.clone(), d + 1));
                }
            }
        }
    }

    // Build payload
    let mut nodes = Vec::new();
    for id in &visited {
        if let Some(node) = graph.get_node(id) {
            nodes.push(node_to_cyto(node));
        }
    }

    let mut edges = Vec::new();
    for rel in graph.iter_relationships() {
        if visited.contains(&rel.source_id) && visited.contains(&rel.target_id) {
            edges.push(rel_to_cyto(rel));
        }
    }

    let stats = GraphStats {
        node_count: nodes.len(),
        edge_count: edges.len(),
        truncated: false,
    };

    Ok(GraphPayload {
        nodes,
        edges,
        stats,
    })
}

// ─── Helpers ─────────────────────────────────────────────────────────────

fn node_to_cyto(node: &gitnexus_core::graph::types::GraphNode) -> CytoNode {
    CytoNode {
        id: node.id.clone(),
        label: node.label.as_str().to_string(),
        name: node.properties.name.clone(),
        file_path: node.properties.file_path.clone(),
        start_line: node.properties.start_line,
        end_line: node.properties.end_line,
        is_exported: node.properties.is_exported,
        community: node.properties.heuristic_label.clone(),
        language: node.properties.language.map(|l| format!("{:?}", l)),
        description: node.properties.description.clone(),
        parameter_count: node.properties.parameter_count,
        return_type: node.properties.return_type.clone(),
        layer_type: node.properties.layer_type.clone(),
        entry_point_score: node.properties.entry_point_score,
        entry_point_reason: node.properties.entry_point_reason.clone(),
        is_traced: node.properties.is_traced,
        trace_call_count: node.properties.trace_call_count,
    }
}

fn rel_to_cyto(rel: &gitnexus_core::graph::types::GraphRelationship) -> CytoEdge {
    CytoEdge {
        id: rel.id.clone(),
        source: rel.source_id.clone(),
        target: rel.target_id.clone(),
        rel_type: rel.rel_type.as_str().to_string(),
        confidence: rel.confidence,
    }
}
