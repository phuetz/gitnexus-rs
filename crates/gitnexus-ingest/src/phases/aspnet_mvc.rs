//! Phase: ASP.NET MVC 5 / EF6 enrichment.
//!
//! This phase runs after Heritage and before Communities. It:
//! 1. Scans all C# files for ASP.NET controllers and promotes Class → Controller
//! 2. Extracts action methods and creates ControllerAction/ApiEndpoint nodes
//! 3. Detects DbContext classes and promotes them to DbContext nodes
//! 4. Identifies entity classes referenced in DbSets → DbEntity nodes
//! 5. Parses .edmx files for entity model details (properties, associations)
//! 6. Processes .cshtml files as View nodes with @model/@layout metadata
//! 7. Creates ASP.NET-specific relationships (RendersView, BelongsToArea, etc.)

use gitnexus_core::graph::types::{
    GraphNode, GraphRelationship, NodeLabel, NodeProperties, RelationshipType,
};
use gitnexus_core::graph::KnowledgeGraph;
use gitnexus_lang::route_extractors::csharp::{
    self, ControllerInfo, DbContextInfo,
};
use gitnexus_lang::route_extractors::edmx;

use super::structure::FileEntry;
use std::collections::{HashMap, HashSet};
use regex::Regex;

/// Result statistics from the ASP.NET MVC enrichment phase.
#[derive(Debug, Default)]
pub struct AspNetMvcStats {
    pub controllers: usize,
    pub actions: usize,
    pub api_endpoints: usize,
    pub views: usize,
    pub db_contexts: usize,
    pub db_entities: usize,
    pub areas: usize,
    pub edmx_models: usize,
    pub filters: usize,
    pub webconfigs: usize,
    pub partial_views: usize,
}

/// Run the ASP.NET MVC enrichment phase.
///
/// Returns the count of ASP.NET-specific nodes created.
pub fn enrich_aspnet_mvc(
    graph: &mut KnowledgeGraph,
    file_entries: &[FileEntry],
) -> Result<AspNetMvcStats, crate::IngestError> {
    // Quick check: does this project have any C# files?
    let has_csharp = file_entries
        .iter()
        .any(|f| f.path.ends_with(".cs") || f.path.ends_with(".cshtml") || f.path.ends_with(".edmx"));

    if !has_csharp {
        return Ok(AspNetMvcStats::default());
    }

    let mut stats = AspNetMvcStats::default();
    let mut known_entity_types: HashSet<String> = HashSet::new();
    let mut area_nodes: HashMap<String, String> = HashMap::new(); // area name → node ID

    // ──────────────────────────────────────────────────────────────────────
    // Pass 1: Parse .edmx files for entity model information
    // ──────────────────────────────────────────────────────────────────────
    let edmx_files: Vec<&FileEntry> = file_entries
        .iter()
        .filter(|f| f.path.ends_with(".edmx"))
        .collect();

    let mut edmx_models: Vec<edmx::EdmxModel> = Vec::new();
    for entry in &edmx_files {
        let model = edmx::parse_edmx(&entry.content);
        // Collect known entity types
        for et in &model.entity_types {
            known_entity_types.insert(et.name.clone());
        }
        for es in &model.entity_sets {
            let clean = edmx::clean_entity_type_name(&es.entity_type);
            known_entity_types.insert(clean.to_string());
        }
        edmx_models.push(model);
        stats.edmx_models += 1;
    }

    // ──────────────────────────────────────────────────────────────────────
    // Pass 1b: Parse Web.config files
    // ──────────────────────────────────────────────────────────────────────
    let re_auth = Regex::new(r#"<authentication\s+mode="([^"]+)""#).expect("regex pattern must compile");
    let re_conn = Regex::new(r#"<add\s+name="[^"]+"\s+connectionString="#).expect("regex pattern must compile");
    let re_appsettings = Regex::new(r#"<appSettings>[\s\S]*?<add\s+key="#).expect("regex pattern must compile");
    let re_binding = Regex::new(r#"<bindingRedirect\s+oldVersion="#).expect("regex pattern must compile");

    for entry in file_entries {
        if entry.path.ends_with("Web.config") || entry.path.ends_with("web.config") {
            let webconfig_id = format!("WebConfig:{}", entry.path);

            let mut description_parts = Vec::new();

            if let Some(auth_match) = re_auth.captures(&entry.content) {
                description_parts.push(format!("auth: {}", &auth_match[1]));
            }

            let conn_string_count = re_conn.find_iter(&entry.content).count();
            if conn_string_count > 0 {
                description_parts.push(format!("{} connection strings", conn_string_count));
            }

            let app_settings_count = re_appsettings.find_iter(&entry.content).count();
            if app_settings_count > 0 {
                description_parts.push(format!("{} app settings", app_settings_count));
            }

            let binding_redirects = re_binding.find_iter(&entry.content).count();
            if binding_redirects > 0 {
                description_parts.push(format!("{} binding redirects", binding_redirects));
            }

            let description = if description_parts.is_empty() {
                None
            } else {
                Some(description_parts.join("; "))
            };

            graph.add_node(GraphNode {
                id: webconfig_id,
                label: NodeLabel::WebConfig,
                properties: NodeProperties {
                    name: "Web.config".to_string(),
                    file_path: entry.path.clone(),
                    description,
                    ..Default::default()
                },
            });

            stats.webconfigs += 1;
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    // Pass 1c: Group partial classes by name for content merging
    // ──────────────────────────────────────────────────────────────────────
    let partial_class_regex = Regex::new(r"partial\s+class\s+(\w+)").expect("regex pattern must compile");
    let mut partial_classes: HashMap<String, Vec<String>> = HashMap::new();
    let cs_files_for_partial: Vec<&FileEntry> = file_entries
        .iter()
        .filter(|f| f.path.ends_with(".cs"))
        .collect();

    for entry in &cs_files_for_partial {
        for cap in partial_class_regex.captures_iter(&entry.content) {
            let class_name = &cap[1];
            partial_classes
                .entry(class_name.to_string())
                .or_default()
                .push(entry.content.clone());
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    // Pass 2: Scan C# files for controllers and DbContexts
    // ──────────────────────────────────────────────────────────────────────
    let cs_files: Vec<&FileEntry> = file_entries
        .iter()
        .filter(|f| f.path.ends_with(".cs"))
        .collect();

    let mut all_controllers: Vec<(String, ControllerInfo)> = Vec::new(); // (file_path, info)
    let mut all_db_contexts: Vec<(String, DbContextInfo)> = Vec::new();

    for entry in &cs_files {
        // Extract controllers
        let controllers = csharp::extract_controllers(&entry.content);
        for ctrl in controllers {
            all_controllers.push((entry.path.clone(), ctrl));
        }

        // Extract DbContexts
        let contexts = csharp::extract_db_contexts(&entry.content);
        for ctx in contexts {
            // Add entity types from DbSet<T> to known types
            for es in &ctx.entity_sets {
                known_entity_types.insert(es.entity_type.clone());
            }
            all_db_contexts.push((entry.path.clone(), ctx));
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    // Pass 3: Extract entities from C# files (using known types from DbContext + .edmx)
    // ──────────────────────────────────────────────────────────────────────
    let known_types: Vec<String> = known_entity_types.iter().cloned().collect();
    let mut entity_file_map: HashMap<String, String> = HashMap::new(); // entity name → node ID

    for entry in &cs_files {
        let entities = csharp::extract_entities(&entry.content, &known_types);
        for entity in entities {
            let node_id = format!("DbEntity:{}:{}", entry.path, entity.class_name);

            // Create or promote the node
            let annotations: Vec<String> = entity
                .property_annotations
                .values()
                .flat_map(|v| v.iter().cloned())
                .collect();

            graph.add_node(GraphNode {
                id: node_id.clone(),
                label: NodeLabel::DbEntity,
                properties: NodeProperties {
                    name: entity.class_name.clone(),
                    file_path: entry.path.clone(),
                    db_table_name: entity.table_name,
                    data_annotations: if annotations.is_empty() {
                        None
                    } else {
                        Some(annotations)
                    },
                    ..Default::default()
                },
            });

            entity_file_map.insert(entity.class_name.clone(), node_id.clone());

            // Create navigation property relationships
            for nav in &entity.navigation_properties {
                // These will be linked in a later pass when all entities are known
                // For now, record them
                let target_id = format!("DbEntity:*:{}", nav.target_type);
                let rel_id = format!(
                    "assoc:{}:{}:{}",
                    entity.class_name, nav.name, nav.target_type
                );
                let cardinality = if nav.is_collection { "1:*" } else { "*:1" };

                graph.add_relationship(GraphRelationship {
                    id: rel_id,
                    source_id: node_id.clone(),
                    target_id,
                    rel_type: RelationshipType::AssociatesWith,
                    confidence: 0.8,
                    reason: format!("navigation_property:{}", cardinality),
                    step: None,
                });
            }

            stats.db_entities += 1;
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    // Pass 4: Create DbContext nodes and MapsToEntity relationships
    // ──────────────────────────────────────────────────────────────────────
    for (file_path, ctx) in &all_db_contexts {
        let ctx_id = format!("DbContext:{}:{}", file_path, ctx.class_name);

        graph.add_node(GraphNode {
            id: ctx_id.clone(),
            label: NodeLabel::DbContext,
            properties: NodeProperties {
                name: ctx.class_name.clone(),
                file_path: file_path.clone(),
                connection_string_name: ctx.connection_string_name.clone(),
                ..Default::default()
            },
        });

        // Create MAPS_TO_ENTITY relationships
        for es in &ctx.entity_sets {
            let entity_node_id = entity_file_map
                .get(&es.entity_type)
                .cloned()
                .unwrap_or_else(|| format!("DbEntity:*:{}", es.entity_type));

            graph.add_relationship(GraphRelationship {
                id: format!("maps:{}:{}", ctx.class_name, es.entity_type),
                source_id: ctx_id.clone(),
                target_id: entity_node_id,
                rel_type: RelationshipType::MapsToEntity,
                confidence: 1.0,
                reason: format!("DbSet<{}>:{}", es.entity_type, es.property_name),
                step: None,
            });
        }

        stats.db_contexts += 1;
    }

    // ──────────────────────────────────────────────────────────────────────
    // Pass 5: Create Controller nodes, Action nodes, and relationships
    // ──────────────────────────────────────────────────────────────────────
    for (file_path, ctrl) in &all_controllers {
        let ctrl_id = format!("Controller:{}:{}", file_path, ctrl.class_name);

        // Create Area node if needed
        if let Some(area_name) = &ctrl.area_name {
            if !area_nodes.contains_key(area_name) {
                let area_id = format!("Area:{}", area_name);
                graph.add_node(GraphNode {
                    id: area_id.clone(),
                    label: NodeLabel::Area,
                    properties: NodeProperties {
                        name: area_name.clone(),
                        file_path: String::new(),
                        area_name: Some(area_name.clone()),
                        ..Default::default()
                    },
                });
                area_nodes.insert(area_name.clone(), area_id);
                stats.areas += 1;
            }
        }

        // Also detect area from path: Areas/<Name>/Controllers/...
        let inferred_area = infer_area_from_path(file_path);
        let effective_area = ctrl.area_name.as_ref().or(inferred_area.as_ref());

        graph.add_node(GraphNode {
            id: ctrl_id.clone(),
            label: NodeLabel::Controller,
            properties: NodeProperties {
                name: ctrl.class_name.clone(),
                file_path: file_path.clone(),
                area_name: effective_area.cloned(),
                route_template: ctrl.route_prefix.clone(),
                ..Default::default()
            },
        });

        // BELONGS_TO_AREA relationship
        if let Some(area_name) = effective_area {
            if !area_nodes.contains_key(area_name) {
                let area_id = format!("Area:{}", area_name);
                graph.add_node(GraphNode {
                    id: area_id.clone(),
                    label: NodeLabel::Area,
                    properties: NodeProperties {
                        name: area_name.clone(),
                        file_path: String::new(),
                        area_name: Some(area_name.clone()),
                        ..Default::default()
                    },
                });
                area_nodes.insert(area_name.clone(), area_id);
                stats.areas += 1;
            }

            if let Some(area_id) = area_nodes.get(area_name) {
                graph.add_relationship(GraphRelationship {
                    id: format!("area:{}:{}", ctrl.class_name, area_name),
                    source_id: ctrl_id.clone(),
                    target_id: area_id.clone(),
                    rel_type: RelationshipType::BelongsToArea,
                    confidence: 1.0,
                    reason: "area_attribute".to_string(),
                    step: None,
                });
            }
        }

        // Create action nodes
        for action in &ctrl.actions {
            let label = if ctrl.is_api_controller {
                NodeLabel::ApiEndpoint
            } else {
                NodeLabel::ControllerAction
            };

            let action_id = format!(
                "{}:{}:{}:{}",
                if ctrl.is_api_controller {
                    "ApiEndpoint"
                } else {
                    "ControllerAction"
                },
                file_path,
                ctrl.class_name,
                action.name
            );

            // Build route template by combining controller prefix + action route
            let full_route = build_full_route(
                ctrl.route_prefix.as_deref(),
                action.route_template.as_deref(),
                &ctrl.class_name,
                &action.name,
            );

            graph.add_node(GraphNode {
                id: action_id.clone(),
                label,
                properties: NodeProperties {
                    name: action.name.clone(),
                    file_path: file_path.clone(),
                    start_line: action.start_line,
                    http_method: Some(action.http_method.clone()),
                    route_template: Some(full_route),
                    model_type: action.model_type.clone(),
                    return_type: action.return_type.clone(),
                    ..Default::default()
                },
            });

            // HAS_ACTION relationship
            graph.add_relationship(GraphRelationship {
                id: format!("action:{}:{}", ctrl.class_name, action.name),
                source_id: ctrl_id.clone(),
                target_id: action_id.clone(),
                rel_type: RelationshipType::HasAction,
                confidence: 1.0,
                reason: action.http_method.clone(),
                step: None,
            });

            // BINDS_MODEL relationship
            if let Some(model_type) = &action.model_type {
                let target_id = entity_file_map
                    .get(model_type)
                    .cloned()
                    .unwrap_or_else(|| format!("ViewModel:*:{}", model_type));

                graph.add_relationship(GraphRelationship {
                    id: format!("binds:{}:{}:{}", ctrl.class_name, action.name, model_type),
                    source_id: action_id.clone(),
                    target_id,
                    rel_type: RelationshipType::BindsModel,
                    confidence: 0.9,
                    reason: "parameter_binding".to_string(),
                    step: None,
                });
            }

            if ctrl.is_api_controller {
                stats.api_endpoints += 1;
            } else {
                stats.actions += 1;
            }
        }

        stats.controllers += 1;
    }

    // ──────────────────────────────────────────────────────────────────────
    // Pass 5b: Extract filter attributes from controllers
    // ──────────────────────────────────────────────────────────────────────
    let filter_regex = Regex::new(
        r"\[(?:(?:System\.Web\.Mvc\.)?)(Authorize|ValidateAntiForgeryToken|OutputCache|HandleError|AllowAnonymous|RequireHttps|ActionFilter|ExceptionFilter|ResultFilter)(?:\(([^)]*)\))?\]"
    ).expect("regex pattern must compile");

    for (file_path, ctrl) in &all_controllers {
        if let Ok(content) = std::fs::read_to_string(file_path) {
            let mut seen_filters: HashSet<String> = HashSet::new();

            for cap in filter_regex.captures_iter(&content) {
                let filter_name = &cap[1];
                let filter_params = cap.get(2).map_or("", |m| m.as_str());

                let filter_key = if filter_params.is_empty() {
                    filter_name.to_string()
                } else {
                    format!("{}({})", filter_name, filter_params)
                };

                if !seen_filters.contains(&filter_key) {
                    let filter_id = format!("Filter:{}", filter_name);

                    if graph.get_node(&filter_id).is_none() {
                        let mut props = NodeProperties {
                            name: filter_name.to_string(),
                            file_path: String::new(),
                            ..Default::default()
                        };
                        if !filter_params.is_empty() {
                            props.description = Some(filter_params.to_string());
                        }
                        graph.add_node(GraphNode {
                            id: filter_id.clone(),
                            label: NodeLabel::Filter,
                            properties: props,
                        });
                    }

                    let ctrl_id = format!("Controller:{}:{}", file_path, ctrl.class_name);
                    graph.add_relationship(GraphRelationship {
                        id: format!("filter:{}:{}", ctrl.class_name, filter_name),
                        source_id: ctrl_id,
                        target_id: filter_id,
                        rel_type: RelationshipType::HasFilter,
                        confidence: 0.95,
                        reason: if filter_params.is_empty() {
                            "attribute".to_string()
                        } else {
                            format!("attribute:{}", filter_params)
                        },
                        step: None,
                    });

                    seen_filters.insert(filter_key);
                    stats.filters += 1;
                }
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    // Pass 6: Process Razor views (.cshtml)
    // ──────────────────────────────────────────────────────────────────────
    let view_files: Vec<&FileEntry> = file_entries
        .iter()
        .filter(|f| f.path.ends_with(".cshtml"))
        .collect();

    for entry in &view_files {
        let view_info = csharp::extract_view_info(&entry.path, &entry.content);
        let view_id = format!("View:{}", entry.path);

        graph.add_node(GraphNode {
            id: view_id.clone(),
            label: NodeLabel::View,
            properties: NodeProperties {
                name: entry
                    .path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&entry.path)
                    .to_string(),
                file_path: entry.path.clone(),
                model_type: view_info.model_type.clone(),
                layout_path: view_info.layout_path,
                area_name: view_info.area_name.clone(),
                view_engine: Some(if view_info.is_partial {
                    "partial".to_string()
                } else {
                    "razor".to_string()
                }),
                ..Default::default()
            },
        });

        // Try to link view to controller via convention:
        // Views/<ControllerName>/<ActionName>.cshtml → Controller:*:<ControllerName>Controller
        if let Some((ctrl_name, _action_name)) = infer_controller_from_view_path(&entry.path) {
            let ctrl_class = format!("{}Controller", ctrl_name);
            // Find matching controller
            for (_, ctrl) in &all_controllers {
                if ctrl.class_name == ctrl_class {
                    let ctrl_id = all_controllers
                        .iter()
                        .find(|(_, c)| c.class_name == ctrl_class)
                        .map(|(fp, c)| format!("Controller:{}:{}", fp, c.class_name));

                    if let Some(ctrl_id) = ctrl_id {
                        graph.add_relationship(GraphRelationship {
                            id: format!("renders:{}", entry.path),
                            source_id: ctrl_id,
                            target_id: view_id.clone(),
                            rel_type: RelationshipType::RendersView,
                            confidence: 0.85,
                            reason: "convention_based".to_string(),
                            step: None,
                        });
                    }
                    break;
                }
            }
        }

        // Link to Area if in Areas/ directory
        if let Some(area_name) = &view_info.area_name {
            if let Some(area_id) = area_nodes.get(area_name) {
                graph.add_relationship(GraphRelationship {
                    id: format!("viewarea:{}", entry.path),
                    source_id: view_id.clone(),
                    target_id: area_id.clone(),
                    rel_type: RelationshipType::BelongsToArea,
                    confidence: 1.0,
                    reason: "path_convention".to_string(),
                    step: None,
                });
            }
        }

        stats.views += 1;
    }

    // ──────────────────────────────────────────────────────────────────────
    // Pass 6b: Detect @Html.Partial and @Html.RenderAction calls in views
    // ──────────────────────────────────────────────────────────────────────
    let partial_regex = Regex::new(r#"@?\s*Html\.(Partial|RenderPartial|RenderAction)\("([^"]+)""#).expect("regex pattern must compile");

    for entry in &view_files {
        let view_id = format!("View:{}", entry.path);

        let mut seen_partials: HashSet<String> = HashSet::new();

        for cap in partial_regex.captures_iter(&entry.content) {
            let method = &cap[1];
            let partial_name = &cap[2];

            if !seen_partials.contains(partial_name) {
                let partial_view_id = format!("PartialView:{}", partial_name);

                if graph.get_node(&partial_view_id).is_none() {
                    graph.add_node(GraphNode {
                        id: partial_view_id.clone(),
                        label: NodeLabel::PartialView,
                        properties: NodeProperties {
                            name: partial_name.to_string(),
                            file_path: String::new(),
                            view_engine: Some("partial".to_string()),
                            ..Default::default()
                        },
                    });
                }

                graph.add_relationship(GraphRelationship {
                    id: format!("partial:{}:{}", entry.path, partial_name),
                    source_id: view_id.clone(),
                    target_id: partial_view_id,
                    rel_type: RelationshipType::UsesPartial,
                    confidence: 0.95,
                    reason: method.to_string(),
                    step: None,
                });

                seen_partials.insert(partial_name.to_string());
                stats.partial_views += 1;
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    // Pass 7: Enrich from .edmx models (associations with cardinality)
    // ──────────────────────────────────────────────────────────────────────
    for model in &edmx_models {
        for assoc in &model.associations {
            let entity1 = edmx::clean_entity_type_name(&assoc.end1.entity_type);
            let entity2 = edmx::clean_entity_type_name(&assoc.end2.entity_type);

            let source_id = entity_file_map
                .get(entity1)
                .cloned()
                .unwrap_or_else(|| format!("DbEntity:*:{}", entity1));
            let target_id = entity_file_map
                .get(entity2)
                .cloned()
                .unwrap_or_else(|| format!("DbEntity:*:{}", entity2));

            let cardinality = edmx::cardinality_str(assoc);

            graph.add_relationship(GraphRelationship {
                id: format!("edmx_assoc:{}", assoc.name),
                source_id,
                target_id,
                rel_type: RelationshipType::AssociatesWith,
                confidence: 1.0,
                reason: format!("edmx:{}", cardinality),
                step: None,
            });
        }
    }

    Ok(stats)
}

// ─── Helpers ─────────────────────────────────────────────────────────────

/// Infer area name from file path: Areas/<Name>/Controllers/...
fn infer_area_from_path(path: &str) -> Option<String> {
    let lower = path.to_lowercase().replace('\\', "/");
    let (idx, offset) = if let Some(idx) = lower.find("/areas/") {
        (idx, idx + 7)
    } else if lower.starts_with("areas/") {
        (0, 6)
    } else {
        return None;
    };
    let _ = idx;
    let after = &path[offset..];
    after.split('/').next().map(|s| s.to_string())
}

/// Infer controller and action names from view path.
/// Views/<Controller>/<Action>.cshtml → (Controller, Action)
fn infer_controller_from_view_path(path: &str) -> Option<(String, String)> {
    let normalized = path.replace('\\', "/");
    let segments: Vec<&str> = normalized.split('/').collect();

    // Look for "Views" directory
    for (i, seg) in segments.iter().enumerate() {
        if seg.eq_ignore_ascii_case("Views") && i + 2 < segments.len() {
            let controller = segments[i + 1].to_string();
            let action = segments[i + 2]
                .strip_suffix(".cshtml")
                .unwrap_or(segments[i + 2])
                .to_string();

            // Skip shared views
            if controller.eq_ignore_ascii_case("Shared") {
                return None;
            }

            return Some((controller, action));
        }
    }
    None
}

/// Build full route template by combining controller prefix + action route.
fn build_full_route(
    prefix: Option<&str>,
    action_route: Option<&str>,
    controller_name: &str,
    action_name: &str,
) -> String {
    match (prefix, action_route) {
        (Some(p), Some(a)) => format!("{}/{}", p.trim_end_matches('/'), a.trim_start_matches('/')),
        (Some(p), None) => format!("{}/{}", p.trim_end_matches('/'), action_name),
        (None, Some(a)) => a.to_string(),
        (None, None) => {
            // Convention-based: /{Controller}/{Action}
            let ctrl = controller_name
                .strip_suffix("Controller")
                .unwrap_or(controller_name);
            format!("{}/{}", ctrl, action_name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_area_from_path() {
        assert_eq!(
            infer_area_from_path("Areas/Admin/Controllers/ProductsController.cs"),
            Some("Admin".to_string())
        );
        assert_eq!(
            infer_area_from_path("src/Controllers/HomeController.cs"),
            None
        );
    }

    #[test]
    fn test_infer_controller_from_view_path() {
        let (ctrl, action) =
            infer_controller_from_view_path("Views/Products/Index.cshtml").unwrap();
        assert_eq!(ctrl, "Products");
        assert_eq!(action, "Index");

        assert!(infer_controller_from_view_path("Views/Shared/_Layout.cshtml").is_none());
    }

    #[test]
    fn test_build_full_route() {
        assert_eq!(
            build_full_route(Some("api/products"), Some("{id}"), "ProductsController", "Get"),
            "api/products/{id}"
        );
        assert_eq!(
            build_full_route(Some("api/products"), None, "ProductsController", "GetAll"),
            "api/products/GetAll"
        );
        assert_eq!(
            build_full_route(None, None, "ProductsController", "Index"),
            "Products/Index"
        );
    }
}
