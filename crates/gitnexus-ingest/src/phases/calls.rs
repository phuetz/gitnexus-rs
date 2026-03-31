use std::collections::HashMap;

use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_core::resolution::context::ResolutionContext;
use gitnexus_core::resolution::types::*;
use gitnexus_core::symbol::SymbolTable;

use crate::phases::parsing::ExtractedData;
use crate::IngestError;

/// Build a map of (file_path, field_name) → interface_type from constructor parameters.
/// Scans .cs files for class constructors with DI-injected parameters.
fn build_field_type_map(
    file_entries: &[crate::phases::structure::FileEntry],
) -> HashMap<(String, String), String> {
    let mut map = HashMap::new();
    let mut class_count = 0u32;

    // Pattern 1: Field declarations like "CourriersService courriersService = null;"
    let field_re = regex::Regex::new(
        r"(?:private|protected|public|internal)?\s*([A-Z]\w+(?:Service|Repository|Manager|Helper|Provider|Client|Handler))\s+(\w+)\s*[=;]"
    ).ok();

    // Pattern 2: Constructor DI params like "public Foo(ICourriersService courriersService)"
    let class_re = regex::Regex::new(
        r"(?:public|internal|private)?\s*(?:partial\s+)?class\s+(\w+)"
    ).ok();

    for file in file_entries {
        if !file.path.ends_with(".cs") {
            continue;
        }
        let fp = file.path.clone();

        // Extract field declarations (legacy ASP.NET pattern without DI)
        if let Some(ref re) = field_re {
            for cap in re.captures_iter(&file.content) {
                if let (Some(type_name), Some(field_name)) = (cap.get(1), cap.get(2)) {
                    let type_name = type_name.as_str().to_string();
                    let field_name = field_name.as_str().to_string();
                    map.insert((fp.clone(), field_name.clone()), type_name.clone());
                    // Also with _ prefix
                    map.insert((fp.clone(), format!("_{}", field_name)), type_name.clone());
                    class_count += 1;
                }
            }
        }

        // Extract constructor DI params (modern pattern)
        if let Some(ref re) = class_re {
            for cap in re.captures_iter(&file.content) {
                if let Some(class_name) = cap.get(1) {
                    let deps = gitnexus_lang::route_extractors::csharp::extract_constructor_dependencies(
                        &file.content,
                        class_name.as_str(),
                    );
                    for (iface_type, param_name) in deps {
                        map.insert((fp.clone(), param_name.clone()), iface_type.clone());
                        map.insert((fp.clone(), format!("_{}", param_name)), iface_type.clone());
                    }
                }
            }
        }
    }

    tracing::debug!("Built field type map: {} entries from {} classes with DI", map.len(), class_count);
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
    file_entries: &[crate::phases::structure::FileEntry],
) -> Result<(), IngestError> {
    let mut ctx = ResolutionContext::new(
        symbol_table,
        import_map,
        package_map,
        named_import_map,
        module_alias_map,
    );

    // Build field→type map for receiver-aware resolution (C# DI)
    let field_type_map = build_field_type_map(file_entries);
    let mut receiver_resolved = 0u32;

    let mut edge_count = 0;

    // Debug: count how many calls have receivers
    let with_receiver = extracted.calls.iter().filter(|c| c.receiver_name.is_some()).count();
    let cs_calls = extracted.calls.iter().filter(|c| c.file_path.ends_with(".cs")).count();
    tracing::debug!("Calls: {} total, {} C#, {} with receiver", extracted.calls.len(), cs_calls, with_receiver);

    for call in &extracted.calls {
        ctx.enable_cache(&call.file_path);

        // Tier 0: Field-type-aware resolution for C# files
        // If the calling file has declared "CourriersService courriersService" as a field,
        // and the called method "CreerCourrier" exists in CourriersService.cs,
        // create a high-confidence Calls edge.
        if call.file_path.ends_with(".cs") {
            // Get all service types declared as fields in this file
            let file_types: Vec<&String> = field_type_map
                .iter()
                .filter(|((fp, _), _)| fp == &call.file_path)
                .map(|(_, type_name)| type_name)
                .collect();

            if !file_types.is_empty() {
                // Check if the called method exists in any of these service types' files
                if let Some(candidates) = symbol_table.lookup_global(&call.called_name) {
                    let target = candidates.iter().find(|def| {
                        (def.symbol_type == NodeLabel::Method || def.symbol_type == NodeLabel::Function)
                            && file_types.iter().any(|svc_type| {
                                let impl_name = svc_type.strip_prefix('I').unwrap_or(svc_type);
                                def.file_path.contains(impl_name)
                                    || def.owner_id.as_deref()
                                        .map(|o| o.contains(impl_name))
                                        .unwrap_or(false)
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
                                confidence: 0.85,
                                reason: format!("field-type:{}", call.called_name),
                                step: None,
                            });
                            edge_count += 1;
                            receiver_resolved += 1;
                        }
                        continue;
                    }
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
