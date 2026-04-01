//! Dead code detection phase: marks methods with no incoming Calls edges.

use std::collections::HashSet;

use gitnexus_core::graph::types::{NodeLabel, RelationshipType};
use gitnexus_core::graph::KnowledgeGraph;

/// Mark methods with no incoming Calls edges as dead code candidates.
///
/// Entry points (ControllerAction, methods named Main/Application_Start) are excluded.
/// This runs as a post-indexing phase after all Calls edges have been resolved.
pub fn mark_dead_code(graph: &mut KnowledgeGraph) {
    // Build set of method IDs that have at least one incoming Calls edge
    let mut has_incoming_call: HashSet<String> = HashSet::new();
    for rel in graph.iter_relationships() {
        if matches!(
            rel.rel_type,
            RelationshipType::Calls | RelationshipType::CallsAction | RelationshipType::CallsService
        ) {
            has_incoming_call.insert(rel.target_id.clone());
        }
    }

    // Entry point method names that should never be flagged as dead
    let entry_point_names: HashSet<&str> = [
        "Main",
        "Application_Start",
        "Application_End",
        "Application_Error",
        "Configuration",
        "ConfigureServices",
        "Dispose",
    ]
    .into_iter()
    .collect();

    // Collect method IDs to mark (avoid borrow conflict with get_node_mut)
    let method_ids: Vec<(String, bool)> = graph
        .iter_nodes()
        .filter(|n| {
            matches!(
                n.label,
                NodeLabel::Method | NodeLabel::Constructor | NodeLabel::Function
            )
        })
        .map(|n| {
            let is_entry = matches!(n.label, NodeLabel::ControllerAction)
                || entry_point_names.contains(n.properties.name.as_str());
            (n.id.clone(), is_entry)
        })
        .collect();

    let mut dead_count = 0u32;
    let mut live_count = 0u32;

    for (method_id, is_entry) in &method_ids {
        if *is_entry {
            continue;
        }
        if has_incoming_call.contains(method_id) {
            live_count += 1;
        } else {
            if let Some(node) = graph.get_node_mut(method_id) {
                node.properties.is_dead_candidate = Some(true);
                dead_count += 1;
            }
        }
    }

    tracing::info!(
        dead_candidates = dead_count,
        live_methods = live_count,
        total_methods = method_ids.len(),
        "Dead code detection complete"
    );
}
