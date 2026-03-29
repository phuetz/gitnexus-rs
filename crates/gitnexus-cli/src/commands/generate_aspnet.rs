//! ASP.NET MVC 5 / EF6 documentation generators.
//!
//! Generates documentation pages specific to ASP.NET projects:
//! - **controllers.md** — All controllers with routes, actions, model bindings
//! - **entities.md** — Entity model diagram with properties, associations, cardinality
//! - **views.md** — View inventory with model bindings, layouts, areas
//! - **routes.md** — API route table (sorted by path)
//! - **areas.md** — MVC Areas breakdown
//! - **data-model.md** — Full entity relationship diagram (Mermaid ER diagram)
//! - **seq-http.md** — Sequence diagrams: HTTP request lifecycle per controller
//! - **seq-data.md** — Sequence diagrams: Data access flow (Controller → DbContext → Entity)

use std::collections::{BTreeMap, HashSet};
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;

/// Check if the graph contains any ASP.NET-specific nodes.
pub fn has_aspnet_content(graph: &KnowledgeGraph) -> bool {
    graph.iter_nodes().any(|n| {
        matches!(
            n.label,
            NodeLabel::Controller
                | NodeLabel::ControllerAction
                | NodeLabel::ApiEndpoint
                | NodeLabel::DbContext
                | NodeLabel::DbEntity
                | NodeLabel::View
                | NodeLabel::Area
        )
    })
}

/// Generate all ASP.NET documentation pages.
pub fn generate_aspnet_docs(
    graph: &KnowledgeGraph,
    docs_dir: &Path,
) -> Result<Vec<(String, String, String)>> {
    // Returns: Vec<(id, title, filename)> for _index.json integration
    let mut pages = Vec::new();

    // Collect ASP.NET nodes
    let controllers = collect_by_label(graph, NodeLabel::Controller);
    let actions = collect_by_label(graph, NodeLabel::ControllerAction);
    let api_endpoints = collect_by_label(graph, NodeLabel::ApiEndpoint);
    let views = collect_by_label(graph, NodeLabel::View);
    let entities = collect_by_label(graph, NodeLabel::DbEntity);
    let db_contexts = collect_by_label(graph, NodeLabel::DbContext);
    let areas = collect_by_label(graph, NodeLabel::Area);

    // 1. Controllers documentation
    if !controllers.is_empty() {
        generate_controllers_doc(docs_dir, &controllers, &actions, &api_endpoints, graph)?;
        pages.push((
            "aspnet-controllers".to_string(),
            "Controllers & Actions".to_string(),
            "aspnet-controllers.md".to_string(),
        ));
    }

    // 2. Routes table
    if !actions.is_empty() || !api_endpoints.is_empty() {
        generate_routes_doc(docs_dir, &controllers, &actions, &api_endpoints)?;
        pages.push((
            "aspnet-routes".to_string(),
            "API & Route Table".to_string(),
            "aspnet-routes.md".to_string(),
        ));
    }

    // 3. Entity model
    if !entities.is_empty() {
        generate_entities_doc(docs_dir, &entities, &db_contexts, graph)?;
        pages.push((
            "aspnet-entities".to_string(),
            "Entity Data Model".to_string(),
            "aspnet-entities.md".to_string(),
        ));
    }

    // 4. Views
    if !views.is_empty() {
        generate_views_doc(docs_dir, &views, &controllers)?;
        pages.push((
            "aspnet-views".to_string(),
            "Views & Templates".to_string(),
            "aspnet-views.md".to_string(),
        ));
    }

    // 5. Areas
    if !areas.is_empty() {
        generate_areas_doc(docs_dir, &areas, &controllers, &views)?;
        pages.push((
            "aspnet-areas".to_string(),
            "MVC Areas".to_string(),
            "aspnet-areas.md".to_string(),
        ));
    }

    // 6. Data model ER diagram
    if !entities.is_empty() {
        generate_er_diagram_doc(docs_dir, &entities, graph)?;
        pages.push((
            "aspnet-data-model".to_string(),
            "Entity Relationship Diagram".to_string(),
            "aspnet-data-model.md".to_string(),
        ));
    }

    // 7. Sequence diagrams — HTTP request flow (per controller)
    if !controllers.is_empty() && (!actions.is_empty() || !views.is_empty()) {
        generate_sequence_http_doc(docs_dir, &controllers, &actions, &views, &db_contexts, graph)?;
        pages.push((
            "aspnet-seq-http".to_string(),
            "Sequence: HTTP Request Flow".to_string(),
            "aspnet-seq-http.md".to_string(),
        ));
    }

    // 8. Sequence diagrams — Data access flow (DbContext → Entities)
    if !db_contexts.is_empty() && !entities.is_empty() {
        generate_sequence_data_doc(docs_dir, &controllers, &db_contexts, &entities, graph)?;
        pages.push((
            "aspnet-seq-data".to_string(),
            "Sequence: Data Access Flow".to_string(),
            "aspnet-seq-data.md".to_string(),
        ));
    }

    Ok(pages)
}

// ─── Node Collection ─────────────────────────────────────────────────────

fn collect_by_label(graph: &KnowledgeGraph, label: NodeLabel) -> Vec<&GraphNode> {
    let mut nodes: Vec<&GraphNode> = graph
        .iter_nodes()
        .filter(|n| n.label == label)
        .collect();
    nodes.sort_by(|a, b| a.properties.name.cmp(&b.properties.name));
    nodes
}

// ─── Controllers Documentation ──────────────────────────────────────────

fn generate_controllers_doc(
    docs_dir: &Path,
    controllers: &[&GraphNode],
    actions: &[&GraphNode],
    api_endpoints: &[&GraphNode],
    graph: &KnowledgeGraph,
) -> Result<()> {
    let path = docs_dir.join("aspnet-controllers.md");
    let mut f = std::fs::File::create(path)?;

    writeln!(f, "# Controllers & Actions")?;
    writeln!(f)?;
    writeln!(
        f,
        "This project contains **{} controllers** with **{} MVC actions** and **{} API endpoints**.",
        controllers.len(),
        actions.len(),
        api_endpoints.len()
    )?;
    writeln!(f)?;

    for ctrl in controllers {
        let area = ctrl.properties.area_name.as_deref().unwrap_or("");
        let route = ctrl.properties.route_template.as_deref().unwrap_or("(convention)");

        writeln!(f, "## {}", ctrl.properties.name)?;
        writeln!(f)?;
        writeln!(f, "- **File:** `{}`", ctrl.properties.file_path)?;
        if !area.is_empty() {
            writeln!(f, "- **Area:** {}", area)?;
        }
        writeln!(f, "- **Route Prefix:** `{}`", route)?;
        writeln!(f)?;

        // Find actions for this controller
        let ctrl_actions: Vec<&&GraphNode> = actions
            .iter()
            .chain(api_endpoints.iter())
            .filter(|a| a.properties.file_path == ctrl.properties.file_path)
            .collect();

        if !ctrl_actions.is_empty() {
            writeln!(f, "### Actions")?;
            writeln!(f)?;
            writeln!(f, "| Method | Action | Route | Model | Return Type |")?;
            writeln!(f, "|--------|--------|-------|-------|-------------|")?;

            for action in ctrl_actions {
                let http = action
                    .properties
                    .http_method
                    .as_deref()
                    .unwrap_or("GET");
                let route = action
                    .properties
                    .route_template
                    .as_deref()
                    .unwrap_or("-");
                let model = action.properties.model_type.as_deref().unwrap_or("-");
                let ret = action.properties.return_type.as_deref().unwrap_or("-");

                writeln!(
                    f,
                    "| `{}` | **{}** | `{}` | {} | {} |",
                    http, action.properties.name, route, model, ret
                )?;
            }
            writeln!(f)?;
        }

        // Controller dependency diagram
        let rels: Vec<&GraphRelationship> = graph
            .iter_relationships()
            .filter(|r| r.source_id == ctrl.id || r.target_id == ctrl.id)
            .collect();

        // Only show high-level dependencies (Services, DbContexts, ExternalServices)
        // Skip HAS_ACTION, RENDERS_VIEW, etc. to keep diagrams readable
        let dep_rels: Vec<&&GraphRelationship> = rels.iter()
            .filter(|r| matches!(r.rel_type,
                RelationshipType::DependsOn | RelationshipType::CallsService |
                RelationshipType::Calls))
            .filter(|r| {
                let other_id = if r.source_id == ctrl.id { &r.target_id } else { &r.source_id };
                graph.get_node(other_id).map_or(false, |n| matches!(n.label,
                    NodeLabel::Service | NodeLabel::DbContext | NodeLabel::ExternalService))
            })
            .collect();

        if !dep_rels.is_empty() {
            writeln!(f, "### Dependencies")?;
            writeln!(f)?;
            writeln!(f, "```mermaid")?;
            writeln!(f, "graph LR")?;
            writeln!(f, "  {}[\"{}\"]", sanitize_mermaid(&ctrl.properties.name), escape_mermaid_label(&ctrl.properties.name))?;
            let mut seen_targets: HashSet<String> = HashSet::new();
            for rel in &dep_rels {
                let other_id = if rel.source_id == ctrl.id { &rel.target_id } else { &rel.source_id };
                if let Some(other) = graph.get_node(other_id) {
                    if seen_targets.insert(other.properties.name.clone()) {
                        writeln!(f, "  {} --> {}[\"{}\"]",
                            sanitize_mermaid(&ctrl.properties.name),
                            sanitize_mermaid(&other.properties.name),
                            escape_mermaid_label(&other.properties.name)
                        )?;
                    }
                }
            }
            writeln!(f, "```")?;
            writeln!(f)?;
        }
    }

    Ok(())
}

// ─── Routes Documentation ───────────────────────────────────────────────

fn generate_routes_doc(
    docs_dir: &Path,
    controllers: &[&GraphNode],
    actions: &[&GraphNode],
    api_endpoints: &[&GraphNode],
) -> Result<()> {
    let path = docs_dir.join("aspnet-routes.md");
    let mut f = std::fs::File::create(path)?;

    writeln!(f, "# API & Route Table")?;
    writeln!(f)?;

    // Collect all routes
    let mut routes: Vec<(&str, &str, &str, &str, &str)> = Vec::new(); // (method, route, action, controller, type)

    for action in actions.iter().chain(api_endpoints.iter()) {
        let method = action.properties.http_method.as_deref().unwrap_or("GET");
        let route = action.properties.route_template.as_deref().unwrap_or("-");
        let controller = controllers
            .iter()
            .find(|c| c.properties.file_path == action.properties.file_path)
            .map(|c| c.properties.name.as_str())
            .unwrap_or("-");
        let kind = if action.label == NodeLabel::ApiEndpoint {
            "API"
        } else {
            "MVC"
        };
        routes.push((method, route, &action.properties.name, controller, kind));
    }

    routes.sort_by(|a, b| a.1.cmp(b.1));

    writeln!(f, "| Method | Route | Action | Controller | Type |")?;
    writeln!(f, "|--------|-------|--------|------------|------|")?;
    for (method, route, action, controller, kind) in &routes {
        writeln!(
            f,
            "| `{}` | `{}` | {} | {} | {} |",
            method, route, action, controller, kind
        )?;
    }
    writeln!(f)?;

    // Summary by HTTP method
    writeln!(f, "## Summary")?;
    writeln!(f)?;
    let mut method_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for (method, _, _, _, _) in &routes {
        *method_counts.entry(method).or_default() += 1;
    }
    for (method, count) in &method_counts {
        writeln!(f, "- **{}**: {} routes", method, count)?;
    }

    Ok(())
}

// ─── Entities Documentation ─────────────────────────────────────────────

fn generate_entities_doc(
    docs_dir: &Path,
    entities: &[&GraphNode],
    db_contexts: &[&GraphNode],
    graph: &KnowledgeGraph,
) -> Result<()> {
    let path = docs_dir.join("aspnet-entities.md");
    let mut f = std::fs::File::create(path)?;

    writeln!(f, "# Entity Data Model")?;
    writeln!(f)?;
    writeln!(
        f,
        "This project defines **{} entities** across **{} DbContext(s)**.",
        entities.len(),
        db_contexts.len()
    )?;
    writeln!(f)?;

    // DbContexts
    for ctx in db_contexts {
        writeln!(f, "## {}", ctx.properties.name)?;
        writeln!(f)?;
        writeln!(f, "- **File:** `{}`", ctx.properties.file_path)?;
        if let Some(cs) = &ctx.properties.connection_string_name {
            writeln!(f, "- **Connection String:** `{}`", cs)?;
        }
        writeln!(f)?;

        // Find entities mapped through this context
        let mapped: Vec<&GraphRelationship> = graph
            .iter_relationships()
            .filter(|r| r.source_id == ctx.id && r.rel_type == RelationshipType::MapsToEntity)
            .collect();

        if !mapped.is_empty() {
            writeln!(f, "### Entity Sets")?;
            writeln!(f)?;
            writeln!(f, "| Entity | DbSet Property |")?;
            writeln!(f, "|--------|---------------|")?;
            for rel in &mapped {
                let entity_name = rel
                    .reason
                    .split(':')
                    .nth(1)
                    .unwrap_or(&rel.target_id);
                let dbset_info = &rel.reason;
                writeln!(f, "| {} | `{}` |", entity_name, dbset_info)?;
            }
            writeln!(f)?;
        }
    }

    // Individual entities
    for entity in entities {
        writeln!(f, "## {}", entity.properties.name)?;
        writeln!(f)?;
        writeln!(f, "- **File:** `{}`", entity.properties.file_path)?;
        if let Some(table) = &entity.properties.db_table_name {
            writeln!(f, "- **Table:** `{}`", table)?;
        }
        writeln!(f)?;

        // Data annotations
        if let Some(annotations) = &entity.properties.data_annotations {
            if !annotations.is_empty() {
                writeln!(f, "**Data Annotations:** {}", annotations.join(", "))?;
                writeln!(f)?;
            }
        }

        // Associations
        let associations: Vec<&GraphRelationship> = graph
            .iter_relationships()
            .filter(|r| {
                r.source_id == entity.id && r.rel_type == RelationshipType::AssociatesWith
            })
            .collect();

        if !associations.is_empty() {
            writeln!(f, "### Relationships")?;
            writeln!(f)?;
            writeln!(f, "| Target | Cardinality | Source |")?;
            writeln!(f, "|--------|------------|--------|")?;
            for rel in &associations {
                let target_name = rel.target_id.rsplit(':').next().unwrap_or(&rel.target_id);
                let card = &rel.reason;
                writeln!(f, "| {} | {} | {} |", target_name, card, rel.reason)?;
            }
            writeln!(f)?;
        }
    }

    Ok(())
}

// ─── Views Documentation ────────────────────────────────────────────────

fn generate_views_doc(
    docs_dir: &Path,
    views: &[&GraphNode],
    _controllers: &[&GraphNode],
) -> Result<()> {
    let path = docs_dir.join("aspnet-views.md");
    let mut f = std::fs::File::create(path)?;

    // Filter out PackageTmp/obj duplicates — keep only source views
    let source_views: Vec<&&GraphNode> = views.iter()
        .filter(|v| {
            let p = &v.properties.file_path;
            !p.contains("PackageTmp") && !p.contains("/obj/") && !p.contains("\\obj\\")
        })
        .collect();

    writeln!(f, "# Views & Templates")?;
    writeln!(f)?;
    writeln!(f, "Total: **{} vues** (hors copies de déploiement)", source_views.len())?;
    writeln!(f)?;

    // Group views by controller folder (extract from path: Views/{Controller}/xxx.cshtml)
    let mut grouped: BTreeMap<String, Vec<&&&GraphNode>> = BTreeMap::new();
    for view in &source_views {
        let path_str = &view.properties.file_path;
        // Extract folder name from Views/{FolderName}/file.cshtml
        let folder = if let Some(views_idx) = path_str.to_lowercase().find("views/") {
            let after = &path_str[views_idx + 6..];
            after.split(['/', '\\']).next().unwrap_or("Shared")
        } else if let Some(views_idx) = path_str.to_lowercase().find("views\\") {
            let after = &path_str[views_idx + 6..];
            after.split(['/', '\\']).next().unwrap_or("Shared")
        } else {
            "Autres"
        };
        grouped.entry(folder.to_string()).or_default().push(view);
    }

    for (folder, folder_views) in &grouped {
        writeln!(f, "## {} ({} vues)", folder, folder_views.len())?;
        writeln!(f)?;
        writeln!(f, "| Vue | Modèle | Layout |")?;
        writeln!(f, "|-----|--------|--------|")?;

        for view in folder_views {
            // Extract just the filename
            let filename = view.properties.file_path
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(&view.properties.file_path);
            let model = view.properties.model_type.as_deref().unwrap_or("-");
            let layout = view.properties.layout_path.as_deref().unwrap_or("-");

            writeln!(f, "| `{}` | {} | {} |", filename, model, layout)?;
        }
        writeln!(f)?;
    }

    Ok(())
}

// ─── Areas Documentation ────────────────────────────────────────────────

fn generate_areas_doc(
    docs_dir: &Path,
    areas: &[&GraphNode],
    controllers: &[&GraphNode],
    views: &[&GraphNode],
) -> Result<()> {
    let path = docs_dir.join("aspnet-areas.md");
    let mut f = std::fs::File::create(path)?;

    writeln!(f, "# MVC Areas")?;
    writeln!(f)?;
    writeln!(f, "This project is organized into **{} areas**.", areas.len())?;
    writeln!(f)?;

    for area in areas {
        let area_name = area.properties.area_name.as_deref().unwrap_or(&area.properties.name);
        writeln!(f, "## Area: {}", area_name)?;
        writeln!(f)?;

        // Controllers in this area
        let area_controllers: Vec<&&GraphNode> = controllers
            .iter()
            .filter(|c| c.properties.area_name.as_deref() == Some(area_name))
            .collect();

        if !area_controllers.is_empty() {
            writeln!(f, "### Controllers ({})", area_controllers.len())?;
            writeln!(f)?;
            for ctrl in &area_controllers {
                writeln!(f, "- **{}** (`{}`)", ctrl.properties.name, ctrl.properties.file_path)?;
            }
            writeln!(f)?;
        }

        // Views in this area
        let area_views: Vec<&&GraphNode> = views
            .iter()
            .filter(|v| v.properties.area_name.as_deref() == Some(area_name))
            .collect();

        if !area_views.is_empty() {
            writeln!(f, "### Views ({})", area_views.len())?;
            writeln!(f)?;
            for view in &area_views {
                writeln!(f, "- `{}`", view.properties.file_path)?;
            }
            writeln!(f)?;
        }
    }

    Ok(())
}

// ─── ER Diagram Documentation ───────────────────────────────────────────

fn generate_er_diagram_doc(
    docs_dir: &Path,
    entities: &[&GraphNode],
    graph: &KnowledgeGraph,
) -> Result<()> {
    let path = docs_dir.join("aspnet-data-model.md");
    let mut f = std::fs::File::create(path)?;

    writeln!(f, "# Modèle de Données")?;
    writeln!(f)?;
    writeln!(f, "> {} entités détectées dans le modèle Entity Framework.", entities.len())?;
    writeln!(f)?;

    // Build adjacency map for entities
    let entity_names: HashSet<String> = entities.iter()
        .map(|e| e.properties.name.clone())
        .collect();

    let mut adjacency: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new(); // entity -> [(related, cardinality)]
    for rel in graph.iter_relationships() {
        if rel.rel_type == RelationshipType::AssociatesWith {
            let source = rel.source_id.rsplit(':').next().unwrap_or(&rel.source_id).to_string();
            let target = rel.target_id.rsplit(':').next().unwrap_or(&rel.target_id).to_string();
            let card = if rel.reason.contains("1:*") { "1:N" }
                else if rel.reason.contains("*:1") { "N:1" }
                else if rel.reason.contains("*:*") { "N:N" }
                else { "1:1" };
            adjacency.entry(source.clone()).or_default().push((target.clone(), card.to_string()));
            adjacency.entry(target).or_default().push((source, card.to_string()));
        }
    }

    // Group entities by business domain (heuristic based on name)
    let domains: Vec<(&str, Vec<&str>)> = vec![
        ("Aides & Barèmes", vec!["AIDEFINANCE", "GROUPEAIDE", "BAREME", "TRANCHEBAREME", "TARIFBASE_AIDE", "PARAMAIDE", "PARAMAIDJUST", "MAJOAIDE", "PLAFOND", "PLAFOND_DOSSIER", "REF_UNITETAR", "REF_UNITEMAJ", "REFFOND", "BUDGET"]),
        ("Dossiers & Prestations", vec!["DOSSIERPRESTA", "DOSSIERELIGIBLE", "ODDEMANDEUR", "BENEFPRESTA", "DSTRGTPOSSIBLE", "JUSTIFICATIFS", "STATDOSSIER"]),
        ("Paiements & Factures", vec!["REGLEMENT", "REGLEMENTLIGNE", "REGULLIGNE", "REF_STATREG"]),
        ("Utilisateurs & Habilitations", vec!["UTILISATEUR", "PROFILS", "HABILITATION", "AUTORISATION", "CMCASUTILPROF", "CMCAS", "CMCASPARAM", "PARAMCMCAS", "REF_FONCTION"]),
        ("Référentiels", vec!["REFTYPEBENEF", "REFTYPEDST", "REFTYPEODAD", "REFTYPEPENSION", "REFTYPEMDLCOUR", "REFPUBLIC", "PARAMPUBLIC", "REFFORMAT", "REFMESSAGE", "REF_UNITE", "REF_CONFIG", "REF_NUM", "REF_STATUTS"]),
        ("Courriers & Documents", vec!["MODELECOURRIER", "MODCOURAIDE", "MODCOURGRP", "DOCUMENT", "EXPORT"]),
        ("Audit", vec!["AUDIT", "Audit", "Auditligne"]),
    ];

    // Entity table with file paths
    writeln!(f, "## Liste des entités\n")?;
    writeln!(f, "| Entité | Fichier | Relations |")?;
    writeln!(f, "|--------|---------|-----------|")?;
    for entity in entities {
        let rel_count = adjacency.get(&entity.properties.name).map_or(0, |v| v.len());
        writeln!(f, "| **{}** | `{}` | {} |",
            entity.properties.name, entity.properties.file_path, rel_count)?;
    }
    writeln!(f)?;

    // Per-domain diagrams
    for (domain_name, domain_entities) in &domains {
        // Filter to entities that actually exist in the graph
        let existing: Vec<&&str> = domain_entities.iter()
            .filter(|name| entity_names.contains(**name))
            .collect();

        if existing.is_empty() {
            continue;
        }

        writeln!(f, "## {}\n", domain_name)?;

        // Small ER diagram for this domain only
        writeln!(f, "```mermaid")?;
        writeln!(f, "erDiagram")?;

        let domain_set: HashSet<&str> = existing.iter().map(|n| **n).collect();

        // Entity blocks
        for name in &existing {
            writeln!(f, "  {} {{}}", sanitize_mermaid(name))?;
        }

        // Relationships within this domain
        let mut seen_rels: HashSet<String> = HashSet::new();
        for name in &existing {
            if let Some(rels) = adjacency.get(**name) {
                for (target, card) in rels {
                    if domain_set.contains(target.as_str()) {
                        let key = if **name < target.as_str() {
                            format!("{}:{}", name, target)
                        } else {
                            format!("{}:{}", target, name)
                        };
                        if seen_rels.insert(key) {
                            let mermaid_rel = match card.as_str() {
                                "1:N" => "||--o{",
                                "N:1" => "}o--||",
                                "N:N" => "}o--o{",
                                _ => "||--||",
                            };
                            writeln!(f, "  {} {} {}",
                                sanitize_mermaid(name), mermaid_rel, sanitize_mermaid(target))?;
                        }
                    }
                }
            }
        }

        writeln!(f, "```\n")?;
    }

    // Entities not in any domain
    let all_domain_entities: HashSet<&str> = domains.iter()
        .flat_map(|(_, ents)| ents.iter().copied())
        .collect();
    let unclassified: Vec<&&GraphNode> = entities.iter()
        .filter(|e| !all_domain_entities.contains(e.properties.name.as_str()))
        .collect();

    if !unclassified.is_empty() {
        writeln!(f, "## Autres entités\n")?;
        for entity in &unclassified {
            writeln!(f, "- **{}** (`{}`)", entity.properties.name, entity.properties.file_path)?;
        }
        writeln!(f)?;
    }

    writeln!(f, "```")?;
    writeln!(f)?;
    writeln!(
        f,
        "> This diagram is auto-generated from the knowledge graph. Entity properties are extracted from data annotations and .edmx models."
    )?;

    Ok(())
}

// ─── Sequence Diagram: HTTP Request Flow ─────────────────────────────────

/// Generate sequence diagrams showing the HTTP request lifecycle per controller.
/// Client → Router → Controller → Action → View (→ ViewModel)
fn generate_sequence_http_doc(
    docs_dir: &Path,
    controllers: &[&GraphNode],
    actions: &[&GraphNode],
    views: &[&GraphNode],
    db_contexts: &[&GraphNode],
    graph: &KnowledgeGraph,
) -> Result<()> {
    let path = docs_dir.join("aspnet-seq-http.md");
    let mut f = std::fs::File::create(path)?;

    writeln!(f, "# Sequence Diagrams: HTTP Request Flow")?;
    writeln!(f)?;
    writeln!(
        f,
        "These diagrams show the lifecycle of HTTP requests through the ASP.NET MVC pipeline, from client to response."
    )?;
    writeln!(f)?;

    // Build lookup: controller ID → actions
    let mut ctrl_actions: BTreeMap<String, Vec<&GraphNode>> = BTreeMap::new();
    for rel in graph.iter_relationships() {
        if rel.rel_type == RelationshipType::HasAction {
            for action in actions.iter() {
                if action.id == rel.target_id && rel.source_id.contains("Controller") {
                    ctrl_actions
                        .entry(rel.source_id.clone())
                        .or_default()
                        .push(action);
                }
            }
        }
    }

    // Build lookup: action ID → rendered views
    let mut action_views: BTreeMap<String, Vec<&GraphNode>> = BTreeMap::new();
    for rel in graph.iter_relationships() {
        if rel.rel_type == RelationshipType::RendersView {
            for view in views.iter() {
                if view.id == rel.target_id {
                    action_views
                        .entry(rel.source_id.clone())
                        .or_default()
                        .push(view);
                }
            }
        }
    }

    // Build lookup: action ID → bound model
    let mut action_models: BTreeMap<String, String> = BTreeMap::new();
    for rel in graph.iter_relationships() {
        if rel.rel_type == RelationshipType::BindsModel {
            if let Some(target) = graph.get_node(&rel.target_id) {
                action_models.insert(rel.source_id.clone(), target.properties.name.clone());
            }
        }
    }

    // Find if any DbContext is used (for data access participants)
    let has_db = !db_contexts.is_empty();
    let db_name = db_contexts
        .first()
        .map(|d| d.properties.name.as_str())
        .unwrap_or("DbContext");

    // Generate diagrams for top 5 controllers (by action count) to keep docs readable
    let mut sorted_ctrls: Vec<&&GraphNode> = controllers.iter().collect();
    sorted_ctrls.sort_by(|a, b| {
        let a_count = ctrl_actions.get(&a.id).map_or(0, |v| v.len());
        let b_count = ctrl_actions.get(&b.id).map_or(0, |v| v.len());
        b_count.cmp(&a_count)
    });
    let top_ctrls: Vec<&&GraphNode> = sorted_ctrls.into_iter().take(5).collect();

    if controllers.len() > 5 {
        writeln!(f, "> Showing request flows for the **5 most active controllers** (out of {}). Each diagram shows the typical HTTP request lifecycle.\n", controllers.len())?;
    }

    for ctrl in top_ctrls {
        let ctrl_name = &ctrl.properties.name;
        let area = ctrl
            .properties
            .area_name
            .as_deref()
            .unwrap_or("");
        let area_prefix = if area.is_empty() {
            String::new()
        } else {
            format!(" (Area: {})", area)
        };

        writeln!(f, "## {}{}", ctrl_name, area_prefix)?;
        writeln!(f)?;

        let Some(my_actions) = ctrl_actions.get(&ctrl.id).filter(|a| !a.is_empty()) else {
            writeln!(f, "_No actions detected for this controller._")?;
            writeln!(f)?;
            continue;
        };

        writeln!(f, "```mermaid")?;
        writeln!(f, "sequenceDiagram")?;
        writeln!(f, "    participant Client")?;
        writeln!(f, "    participant Router as ASP.NET Router")?;
        writeln!(
            f,
            "    participant Ctrl as {}",
            escape_mermaid_label(ctrl_name)
        )?;

        // Show max 3 representative actions to keep diagram readable
        let sample_actions: Vec<&&GraphNode> = my_actions.iter().take(3).collect();
        if my_actions.len() > 3 {
            writeln!(f, "    Note over Client: Showing {}/{} actions", sample_actions.len(), my_actions.len())?;
        }

        // Collect unique views and models for sampled actions
        let mut view_participants: BTreeMap<String, bool> = BTreeMap::new();
        let mut model_participants: BTreeMap<String, bool> = BTreeMap::new();

        for action in &sample_actions {
            if let Some(views_list) = action_views.get(&action.id) {
                for v in views_list {
                    view_participants.insert(v.properties.name.clone(), true);
                }
            }
            if let Some(model) = action_models.get(&action.id) {
                model_participants.insert(model.clone(), true);
            }
        }

        // Declare view participants
        for view_name in view_participants.keys() {
            writeln!(
                f,
                "    participant {} as {}",
                sanitize_mermaid(view_name),
                escape_mermaid_label(view_name)
            )?;
        }

        // Declare model participants if any
        for model_name in model_participants.keys() {
            writeln!(
                f,
                "    participant {} as {}",
                sanitize_mermaid(model_name),
                escape_mermaid_label(model_name)
            )?;
        }

        // Declare DbContext if used
        if has_db {
            writeln!(
                f,
                "    participant DB as {}",
                escape_mermaid_label(db_name)
            )?;
        }

        writeln!(f)?;

        // Generate interactions for sampled actions only
        for action in &sample_actions {
            let action_name = &action.properties.name;
            let http_method = action
                .properties
                .http_method
                .as_deref()
                .unwrap_or("GET");
            let route = action
                .properties
                .route_template
                .as_deref()
                .unwrap_or("/?");

            // Client → Router
            writeln!(
                f,
                "    Client->>Router: {} {}",
                http_method,
                escape_mermaid_label(route)
            )?;

            // Router → Controller
            writeln!(
                f,
                "    Router->>Ctrl: {}()",
                escape_mermaid_label(action_name)
            )?;

            // If action binds a model, show model binding
            if let Some(model_name) = action_models.get(&action.id) {
                writeln!(
                    f,
                    "    Note right of Ctrl: Model binding: {}",
                    escape_mermaid_label(model_name)
                )?;
            }

            // If action uses DbContext (heuristic: action has BindsModel to a DbEntity)
            if has_db {
                if let Some(model_name) = action_models.get(&action.id) {
                    // Check if the model is a DbEntity
                    let is_entity = graph.iter_nodes().any(|n| {
                        n.label == NodeLabel::DbEntity && n.properties.name == *model_name
                    });
                    if is_entity {
                        writeln!(
                            f,
                            "    Ctrl->>DB: Query {}",
                            escape_mermaid_label(model_name)
                        )?;
                        writeln!(
                            f,
                            "    DB-->>Ctrl: {}[]",
                            escape_mermaid_label(model_name)
                        )?;
                    }
                }
            }

            // Controller → View
            if let Some(views_list) = action_views.get(&action.id) {
                for v in views_list {
                    let vn = &v.properties.name;
                    writeln!(
                        f,
                        "    Ctrl->>{}: Render",
                        sanitize_mermaid(vn)
                    )?;
                    writeln!(
                        f,
                        "    {}-->>Client: HTML Response",
                        sanitize_mermaid(vn)
                    )?;
                }
            } else {
                // No view: likely returns JSON or redirect
                writeln!(f, "    Ctrl-->>Client: Response (JSON/Redirect)")?;
            }

            writeln!(f)?;
        }

        writeln!(f, "```")?;
        writeln!(f)?;
    }

    Ok(())
}

// ─── Sequence Diagram: Data Access Flow ──────────────────────────────────

/// Generate sequence diagrams showing the data access patterns.
/// Controller → DbContext → Entity (showing which controllers access which entities through which DbContexts)
fn generate_sequence_data_doc(
    docs_dir: &Path,
    controllers: &[&GraphNode],
    db_contexts: &[&GraphNode],
    _entities: &[&GraphNode],
    graph: &KnowledgeGraph,
) -> Result<()> {
    let path = docs_dir.join("aspnet-seq-data.md");
    let mut f = std::fs::File::create(path)?;

    writeln!(f, "# Sequence Diagrams: Data Access Flow")?;
    writeln!(f)?;
    writeln!(
        f,
        "These diagrams show how controllers access data through Entity Framework DbContexts and entities."
    )?;
    writeln!(f)?;

    // Build: DbContext ID → entity names it exposes (via MapsToEntity)
    let mut ctx_entities: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for rel in graph.iter_relationships() {
        if rel.rel_type == RelationshipType::MapsToEntity {
            if let Some(entity) = graph.get_node(&rel.target_id) {
                ctx_entities
                    .entry(rel.source_id.clone())
                    .or_default()
                    .push(entity.properties.name.clone());
            }
        }
    }

    // Build: Controller → set of entity names it touches (via BindsModel on its actions)
    let mut ctrl_entities: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    // ctrl_id → { entity_name → [action_names] }
    for rel in graph.iter_relationships() {
        if rel.rel_type == RelationshipType::HasAction {
            let ctrl_id = &rel.source_id;
            let action_id = &rel.target_id;

            // Find what this action binds to
            for bind_rel in graph.iter_relationships() {
                if bind_rel.rel_type == RelationshipType::BindsModel
                    && bind_rel.source_id == *action_id
                {
                    if let Some(target) = graph.get_node(&bind_rel.target_id) {
                        // Only include if target is a DbEntity
                        if target.label == NodeLabel::DbEntity {
                            if let Some(action_node) = graph.get_node(action_id) {
                                ctrl_entities
                                    .entry(ctrl_id.clone())
                                    .or_default()
                                    .entry(target.properties.name.clone())
                                    .or_default()
                                    .push(action_node.properties.name.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // Overview diagram: all DbContexts with their entity sets
    writeln!(f, "## DbContext Overview")?;
    writeln!(f)?;
    writeln!(f, "```mermaid")?;
    writeln!(f, "sequenceDiagram")?;
    writeln!(f, "    participant App as Application")?;

    for ctx in db_contexts {
        writeln!(
            f,
            "    participant {} as {}",
            sanitize_mermaid(&ctx.properties.name),
            escape_mermaid_label(&ctx.properties.name)
        )?;
    }

    writeln!(f, "    participant DB as SQL Server")?;
    writeln!(f)?;

    for ctx in db_contexts {
        let ctx_safe = sanitize_mermaid(&ctx.properties.name);
        let ctx_label = escape_mermaid_label(&ctx.properties.name);

        if let Some(entity_names) = ctx_entities.get(&ctx.id) {
            writeln!(f, "    App->>{}: Open connection", ctx_safe)?;

            for entity_name in entity_names {
                writeln!(
                    f,
                    "    {}-->>DB: DbSet<{}>",
                    ctx_safe,
                    escape_mermaid_label(entity_name)
                )?;
            }

            writeln!(f, "    Note over {},DB: {} entities managed", ctx_label, entity_names.len())?;
            writeln!(f)?;
        }
    }

    writeln!(f, "```")?;
    writeln!(f)?;

    // Per-controller data flow diagrams
    for ctrl in controllers {
        let ctrl_name = &ctrl.properties.name;
        let Some(entity_map) = ctrl_entities.get(&ctrl.id).filter(|m| !m.is_empty()) else {
            continue;
        };

        writeln!(f, "## {} — Data Access", ctrl_name)?;
        writeln!(f)?;

        // Find which DbContext serves these entities
        let mut relevant_ctx: Option<&GraphNode> = None;
        for ctx in db_contexts {
            if let Some(ctx_ents) = ctx_entities.get(&ctx.id) {
                if entity_map
                    .keys()
                    .any(|e| ctx_ents.iter().any(|ce| ce == e))
                {
                    relevant_ctx = Some(ctx);
                    break;
                }
            }
        }

        let ctx_name = relevant_ctx
            .map(|c| c.properties.name.as_str())
            .unwrap_or("DbContext");

        writeln!(f, "```mermaid")?;
        writeln!(f, "sequenceDiagram")?;
        writeln!(
            f,
            "    participant Ctrl as {}",
            escape_mermaid_label(ctrl_name)
        )?;
        writeln!(
            f,
            "    participant Ctx as {}",
            escape_mermaid_label(ctx_name)
        )?;

        // Declare entity participants
        for entity_name in entity_map.keys() {
            writeln!(
                f,
                "    participant {} as {}",
                sanitize_mermaid(entity_name),
                escape_mermaid_label(entity_name)
            )?;
        }

        writeln!(f, "    participant DB as SQL Server")?;
        writeln!(f)?;

        for (entity_name, action_names) in entity_map {
            let ent_safe = sanitize_mermaid(entity_name);

            // Show which actions trigger this entity access
            let actions_str = action_names.join(", ");
            writeln!(
                f,
                "    Note right of Ctrl: {}()",
                escape_mermaid_label(&actions_str)
            )?;

            writeln!(
                f,
                "    Ctrl->>Ctx: Query {}",
                escape_mermaid_label(entity_name)
            )?;
            writeln!(
                f,
                "    Ctx->>DB: SELECT FROM {}",
                escape_mermaid_label(
                    entity_name
                )
            )?;
            writeln!(f, "    DB-->>Ctx: ResultSet")?;
            writeln!(
                f,
                "    Ctx-->>{}: Materialize",
                ent_safe
            )?;
            writeln!(f, "    {}-->>Ctrl: Entity data", ent_safe)?;
            writeln!(f)?;
        }

        writeln!(f, "```")?;
        writeln!(f)?;
    }

    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────

/// Sanitize a name for use as a Mermaid node ID.
fn sanitize_mermaid(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

/// Escape a label for safe use inside Mermaid `["..."]` quoted strings.
fn escape_mermaid_label(label: &str) -> String {
    label
        .replace('&', "#amp;")
        .replace('"', "#quot;")
        .replace('<', "#lt;")
        .replace('>', "#gt;")
        .replace('\n', " ")
        .replace('\r', "")
}
