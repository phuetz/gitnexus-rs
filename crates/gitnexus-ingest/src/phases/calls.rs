use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_core::resolution::context::ResolutionContext;
use gitnexus_core::resolution::types::*;
use gitnexus_core::symbol::SymbolTable;

use crate::phases::parsing::ExtractedData;
use crate::IngestError;

/// Resolve all extracted calls and create CALLS edges.
///
/// For each extracted call:
/// 1. Tier 1: Same-file exact match
/// 2. Tier 2a: Named import binding chain
/// 3. Tier 2a: Import-scoped fuzzy match
/// 4. Tier 2b: Package-scoped fuzzy match (Go/C#)
/// 5. Tier 3: Global fuzzy match
///
/// Creates CALLS edges in the graph with confidence based on resolution tier.
pub fn resolve_calls(
    graph: &mut KnowledgeGraph,
    extracted: &ExtractedData,
    symbol_table: &SymbolTable,
    import_map: &ImportMap,
    named_import_map: &NamedImportMap,
    package_map: &PackageMap,
    module_alias_map: &ModuleAliasMap,
) -> Result<(), IngestError> {
    let mut ctx = ResolutionContext::new(
        symbol_table,
        import_map,
        package_map,
        named_import_map,
        module_alias_map,
    );

    let mut edge_count = 0;

    for call in &extracted.calls {
        ctx.enable_cache(&call.file_path);

        // Try to resolve the called name
        if let Some(resolved) = ctx.resolve(&call.called_name, &call.file_path) {
            let confidence = resolved.tier.confidence();
            let reason = resolved.tier.as_str().to_string();

            // Pick best candidate (first match, or arity-filtered)
            let target = if let Some(arg_count) = call.arg_count {
                // Arity filtering
                resolved
                    .candidates
                    .iter()
                    .find(|c| {
                        let param_count = c.parameter_count.unwrap_or(0);
                        let required = c.required_parameter_count.unwrap_or(0);
                        arg_count >= required && arg_count <= param_count
                    })
                    .or(resolved.candidates.first())
            } else {
                resolved.candidates.first()
            };

            if let Some(target_def) = target {
                let edge_id = format!("calls_{}_{}", call.source_id, target_def.node_id);
                // Skip if this exact edge already exists
                if graph.get_relationship(&edge_id).is_none() {
                    graph.add_relationship(GraphRelationship {
                        id: edge_id,
                        source_id: call.source_id.clone(),
                        target_id: target_def.node_id.clone(),
                        rel_type: RelationshipType::Calls,
                        confidence,
                        reason,
                        step: None,
                    });
                    edge_count += 1;
                }
            }
        }
    }

    tracing::info!("Resolved {} call edges", edge_count);
    Ok(())
}
