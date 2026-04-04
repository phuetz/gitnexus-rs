use tauri::State;

use gitnexus_core::graph::types::{NodeLabel, RelationshipType};

use crate::state::AppState;
use crate::types::*;
use crate::commands::shared::{node_to_cyto, node_to_cyto_with_depth};

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

    scored_nodes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or_else(|| {
        // NaN values sort to the end
        if a.0.is_nan() { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Less }
    }));

    // Take top N nodes
    let nodes: Vec<CytoNode> = scored_nodes
        .iter()
        .take(max_nodes)
        .map(|(_, node)| node_to_cyto(node))
        .collect();

    // Collect node ID set for edge filtering — only edges between selected nodes
    let node_ids: std::collections::HashSet<&str> =
        nodes.iter().map(|n| n.id.as_str()).collect();

    // Collect edges between selected nodes.
    // NOTE (M13): The outgoing index stores (target_id, Vec<RelationshipType>) but not
    // relationship IDs or confidence scores, so CytoEdge cannot be reconstructed from the
    // index alone. We keep the O(total_edges) scan here. To enable an O(selected * degree)
    // approach the index would need to store full relationship metadata (id, confidence).
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
    let max_depth = depth.unwrap_or(2).min(5); // Cap depth to prevent explosion
    const MAX_NODES: usize = 500; // Cap total nodes

    // BFS to collect neighborhood, tracking depth per node
    let mut visited = std::collections::HashSet::new();
    let mut depth_map = std::collections::HashMap::new();
    let mut queue = std::collections::VecDeque::new();

    visited.insert(center_node_id.clone());
    depth_map.insert(center_node_id.clone(), 0u32);
    queue.push_back((center_node_id, 0u32));

    while let Some((node_id, d)) = queue.pop_front() {
        if d >= max_depth || visited.len() >= MAX_NODES {
            continue;
        }

        // Outgoing neighbors
        if let Some(outs) = indexes.outgoing.get(&node_id) {
            for (target, _) in outs {
                if visited.insert(target.clone()) {
                    depth_map.insert(target.clone(), d + 1);
                    queue.push_back((target.clone(), d + 1));
                }
            }
        }

        // Incoming neighbors
        if let Some(ins) = indexes.incoming.get(&node_id) {
            for (source, _) in ins {
                if visited.insert(source.clone()) {
                    depth_map.insert(source.clone(), d + 1);
                    queue.push_back((source.clone(), d + 1));
                }
            }
        }
    }

    // Build payload
    let mut nodes = Vec::new();
    for id in &visited {
        if let Some(node) = graph.get_node(id) {
            let d = depth_map.get(id).copied();
            nodes.push(node_to_cyto_with_depth(node, d));
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

/// Get all detected features/communities with stats.
#[tauri::command]
pub async fn get_features(
    state: State<'_, AppState>,
) -> Result<Vec<crate::types::FeatureInfo>, String> {
    let (graph, _, _, _) = state.get_repo(None).await?;

    let mut features: Vec<crate::types::FeatureInfo> = Vec::new();

    for node in graph.iter_nodes() {
        if node.label == NodeLabel::Community {
            let community_name = &node.properties.name;

            // Use symbol_count from the Community node if available,
            // otherwise count nodes whose heuristic_label matches this community name.
            let member_count = node.properties.symbol_count.unwrap_or_else(|| {
                graph
                    .iter_nodes()
                    .filter(|n| n.properties.heuristic_label.as_deref() == Some(community_name))
                    .count() as u32
            });

            features.push(crate::types::FeatureInfo {
                id: node.id.clone(),
                name: node.properties.name.clone(),
                description: node.properties.description.clone(),
                member_count,
                cohesion: node.properties.cohesion,
            });
        }
    }

    // Sort by member count descending
    features.sort_by(|a, b| b.member_count.cmp(&a.member_count));

    // Limit to top 50
    features.truncate(50);

    Ok(features)
}

// ─── Helpers ─────────────────────────────────────────────────────────────

fn rel_to_cyto(rel: &gitnexus_core::graph::types::GraphRelationship) -> CytoEdge {
    CytoEdge {
        id: rel.id.clone(),
        source: rel.source_id.clone(),
        target: rel.target_id.clone(),
        rel_type: rel.rel_type.as_str().to_string(),
        confidence: rel.confidence,
    }
}
