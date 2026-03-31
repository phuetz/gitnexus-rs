use std::collections::HashMap;

use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_core::resolution::context::ResolutionContext;
use gitnexus_core::resolution::types::*;
use gitnexus_core::symbol::SymbolTable;

use crate::phases::parsing::ExtractedData;
use crate::IngestError;

/// Build a map of (file_path, field_name) → interface_type from constructor parameters.
/// Reads .cs files from graph nodes and extracts constructor DI parameters.
fn build_field_type_map(
    graph: &KnowledgeGraph,
    repo_path: &std::path::Path,
) -> HashMap<(String, String), String> {
    let mut map = HashMap::new();

    // Collect all Class/Service/Repository/Controller nodes with file paths
    let class_nodes: Vec<(&str, &str)> = graph
        .iter_nodes()
        .filter(|n| {
            matches!(
                n.label,
                NodeLabel::Class | NodeLabel::Controller | NodeLabel::Service | NodeLabel::Repository
            ) && n.properties.file_path.ends_with(".cs")
        })
        .map(|n| (n.properties.name.as_str(), n.properties.file_path.as_str()))
        .collect();

    for (class_name, file_path) in &class_nodes {
        // Read the source file
        let full_path = repo_path.join(file_path);
        let content = match std::fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let deps = gitnexus_lang::route_extractors::csharp::extract_constructor_dependencies(
            &content,
            class_name,
        );

        for (iface_type, param_name) in deps {
            let fp = file_path.to_string();
            // Constructor param: courrierService
            map.insert((fp.clone(), param_name.clone()), iface_type.clone());
            // Prefixed with _: _courrierService
            map.insert((fp.clone(), format!("_{}", param_name)), iface_type.clone());
            // Lowercase variant
            let lower = param_name[..1].to_lowercase() + &param_name[1..];
            if lower != param_name {
                map.insert((fp.clone(), lower), iface_type.clone());
            }
        }
    }

    tracing::debug!("Built field type map: {} entries from {} classes", map.len(), class_nodes.len());
    map
}

/// Resolve all extracted calls and create CALLS edges.
///
/// Resolution tiers:
/// 0. Receiver-aware: _service.Method() → resolve via DI type map (C# only)
/// 1. Same-file exact match
/// 2a. Named import binding chain
/// 2b. Package-scoped fuzzy match
/// 3. Global fuzzy match
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
    repo_path: &std::path::Path,
) -> Result<(), IngestError> {
    let mut ctx = ResolutionContext::new(
        symbol_table,
        import_map,
        package_map,
        named_import_map,
        module_alias_map,
    );

    // Build field→type map for receiver-aware resolution (C# DI)
    let field_type_map = build_field_type_map(graph, repo_path);
    let mut receiver_resolved = 0u32;

    let mut edge_count = 0;

    for call in &extracted.calls {
        ctx.enable_cache(&call.file_path);

        // Tier 0: Receiver-aware resolution (C# DI pattern)
        // If we have _courrierService.CreerCourrier(), resolve via constructor DI type
        if let Some(ref receiver) = call.receiver_name {
            let key = (call.file_path.clone(), receiver.clone());
            if let Some(iface_type) = field_type_map.get(&key) {
                // Strip leading 'I' to get implementation class name
                // ICourriersService → CourriersService
                let impl_name = iface_type.strip_prefix('I').unwrap_or(iface_type);

                // Find methods matching called_name in class files containing impl_name
                let target = symbol_table.lookup_global(&call.called_name)
                    .and_then(|defs| {
                        defs.iter().find(|def| {
                            (def.symbol_type == NodeLabel::Method
                                || def.symbol_type == NodeLabel::Function)
                                && (def.file_path.contains(impl_name)
                                    || def.owner_id.as_deref()
                                        .map(|o| o.contains(impl_name))
                                        .unwrap_or(false))
                        })
                    });

                if let Some(target_def) = target {
                    let edge_id = format!("calls_di_{}_{}", call.source_id, target_def.node_id);
                    if graph.get_relationship(&edge_id).is_none() {
                        graph.add_relationship(GraphRelationship {
                            id: edge_id,
                            source_id: call.source_id.clone(),
                            target_id: target_def.node_id.clone(),
                            rel_type: RelationshipType::Calls,
                            confidence: 0.9,
                            reason: format!("DI:{}->{}.{}", receiver, iface_type, call.called_name),
                            step: None,
                        });
                        edge_count += 1;
                        receiver_resolved += 1;
                    }
                    continue; // Skip normal resolution, we found a DI match
                }
            }
        }

        // Tiers 1-3: Standard resolution
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

    tracing::info!("Resolved {} call edges ({} via DI receiver)", edge_count, receiver_resolved);
    Ok(())
}
