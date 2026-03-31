use tauri::State;

use gitnexus_core::graph::types::RelationshipType;

use crate::state::AppState;
use crate::types::*;

/// Get 360-degree context for a symbol.
#[tauri::command]
pub async fn get_symbol_context(
    state: State<'_, AppState>,
    node_id: String,
) -> Result<SymbolContext, String> {
    let (graph, indexes, _fts, _repo_path) = state.get_repo(None).await?;

    let node = graph
        .get_node(&node_id)
        .ok_or_else(|| format!("Node '{}' not found", node_id))?;

    let cyto_node = CytoNode {
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
    };

    // Collect related nodes by relationship type
    let mut callers = Vec::new();
    let mut callees = Vec::new();
    let mut imports = Vec::new();
    let mut imported_by = Vec::new();
    let mut inherits = Vec::new();
    let mut inherited_by = Vec::new();

    // Outgoing relationships
    if let Some(outs) = indexes.outgoing.get(&node_id) {
        for (target_id, rel_type) in outs {
            if let Some(target) = graph.get_node(target_id) {
                let related = RelatedNode {
                    id: target.id.clone(),
                    name: target.properties.name.clone(),
                    label: target.label.as_str().to_string(),
                    file_path: target.properties.file_path.clone(),
                };
                match rel_type {
                    RelationshipType::Calls => callees.push(related),
                    RelationshipType::Imports => imports.push(related),
                    RelationshipType::Inherits
                    | RelationshipType::Extends
                    | RelationshipType::Implements => inherits.push(related),
                    _ => {}
                }
            }
        }
    }

    // Incoming relationships
    if let Some(ins) = indexes.incoming.get(&node_id) {
        for (source_id, rel_type) in ins {
            if let Some(source) = graph.get_node(source_id) {
                let related = RelatedNode {
                    id: source.id.clone(),
                    name: source.properties.name.clone(),
                    label: source.label.as_str().to_string(),
                    file_path: source.properties.file_path.clone(),
                };
                match rel_type {
                    RelationshipType::Calls => callers.push(related),
                    RelationshipType::Imports => imported_by.push(related),
                    RelationshipType::Inherits
                    | RelationshipType::Extends
                    | RelationshipType::Implements => inherited_by.push(related),
                    _ => {}
                }
            }
        }
    }

    // Find community membership
    let community = find_community(&graph, &indexes, &node_id);

    Ok(SymbolContext {
        node: cyto_node,
        callers,
        callees,
        imports,
        imported_by,
        inherits,
        inherited_by,
        community,
    })
}

fn find_community(
    graph: &gitnexus_core::graph::KnowledgeGraph,
    indexes: &gitnexus_db::inmemory::cypher::GraphIndexes,
    node_id: &str,
) -> Option<CommunityInfo> {
    // Check incoming MEMBER_OF relationships
    if let Some(ins) = indexes.incoming.get(node_id) {
        for (source_id, rel_type) in ins {
            if *rel_type == RelationshipType::MemberOf {
                if let Some(community_node) = graph.get_node(source_id) {
                    return Some(CommunityInfo {
                        id: community_node.id.clone(),
                        name: community_node.properties.name.clone(),
                        description: community_node.properties.description.clone(),
                        member_count: community_node.properties.symbol_count,
                        cohesion: community_node.properties.cohesion,
                    });
                }
            }
        }
    }

    // Check outgoing MEMBER_OF relationships
    if let Some(outs) = indexes.outgoing.get(node_id) {
        for (target_id, rel_type) in outs {
            if *rel_type == RelationshipType::MemberOf {
                if let Some(community_node) = graph.get_node(target_id) {
                    return Some(CommunityInfo {
                        id: community_node.id.clone(),
                        name: community_node.properties.name.clone(),
                        description: community_node.properties.description.clone(),
                        member_count: community_node.properties.symbol_count,
                        cohesion: community_node.properties.cohesion,
                    });
                }
            }
        }
    }

    None
}
