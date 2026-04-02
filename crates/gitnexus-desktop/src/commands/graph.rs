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
    let (graph, indexes, _fts, _repo_path) = state.get_repo(None).await?;

    let max_nodes = filter.max_nodes.unwrap_or(200);

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

    // Collect filtered nodes
    let filtered_nodes: Vec<_> = graph
        .iter_nodes()
        .filter(|node| {
            let label_ok = if let Some(ref custom) = custom_labels {
                custom.contains(&node.label)
            } else {
                allowed_labels.contains(&node.label)
            };
            if !label_ok {
                return false;
            }
            // File path filter
            if let Some(ref paths) = filter.file_paths {
                if !paths.iter().any(|p| node.properties.file_path.starts_with(p)) {
                    return false;
                }
            }
            true
        })
        .collect();

    // Compute importance score for each node and sort descending
    let mut scored_nodes: Vec<(f64, &gitnexus_core::graph::types::GraphNode)> = filtered_nodes
        .into_iter()
        .map(|node| {
            let mut score: f64 = 0.0;

            // Connectivity (from indexes)
            let indegree = indexes.incoming.get(node.id.as_str()).map_or(0, |v| v.len());
            let outdegree = indexes.outgoing.get(node.id.as_str()).map_or(0, |v| v.len());
            score += (indegree + outdegree) as f64 * 2.0;

            // Entry point bonus
            if let Some(eps) = node.properties.entry_point_score {
                score += eps * 10.0;
            }

            // Exported symbols
            if node.properties.is_exported == Some(true) {
                score += 5.0;
            }

            // Traced symbols
            if node.properties.is_traced == Some(true) {
                score += 3.0;
            }

            // High-level types get priority
            match node.label {
                NodeLabel::Controller | NodeLabel::Service => score += 20.0,
                NodeLabel::Class | NodeLabel::Interface => score += 10.0,
                NodeLabel::Module | NodeLabel::Package => score += 15.0,
                _ => {}
            }

            (score, node)
        })
        .collect();

    scored_nodes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Take top N nodes
    let nodes: Vec<CytoNode> = scored_nodes
        .iter()
        .take(max_nodes)
        .map(|(_, node)| node_to_cyto(node))
        .collect();

    // Collect node ID set for edge filtering — only edges between selected nodes
    let node_ids: std::collections::HashSet<&str> =
        nodes.iter().map(|n| n.id.as_str()).collect();

    // Collect edges between selected nodes
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
