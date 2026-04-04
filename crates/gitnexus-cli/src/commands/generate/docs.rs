//! DeepWiki-style documentation generator (overview, architecture, getting-started, modules, index).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use colored::Colorize;
use serde_json::json;
use tracing::{info, warn};

use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;

use super::utils::*;
use super::functional::{generate_functional_guide, describe_project_fr};
use super::health::generate_project_health;
use super::deployment::{generate_deployment_guide, describe_service_fr};
use super::analytics::generate_git_analytics_pages;

pub(super) fn generate_docs(graph: &KnowledgeGraph, repo_path: &Path) -> Result<()> {
    let docs_dir = repo_path.join(".gitnexus").join("docs");
    // Clean old generated files to avoid stale duplicates
    if docs_dir.exists() {
        let _ = std::fs::remove_dir_all(&docs_dir);
    }
    std::fs::create_dir_all(&docs_dir)?;
    let modules_dir = docs_dir.join("modules");
    std::fs::create_dir_all(&modules_dir)?;

    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repository");

    let communities = collect_communities(graph);
    let edge_map = build_edge_map(graph);
    let lang_stats = collect_language_stats(graph);
    let file_count = count_files(graph);

    let node_count = graph.iter_nodes().count();
    let edge_count = graph.iter_relationships().count();

    if communities.is_empty() {
        println!(
            "{} No communities found. Run `gitnexus analyze` first.",
            "!".yellow()
        );
        return Ok(());
    }

    // 1. Generate overview.md
    generate_docs_overview(
        &docs_dir,
        repo_name,
        file_count,
        node_count,
        edge_count,
        &lang_stats,
        &communities,
        graph,
    )?;

    // 1b. Generate functional guide (business-oriented documentation)
    generate_functional_guide(&docs_dir, repo_name, graph)?;

    // 1c. Generate project health dashboard
    generate_project_health(&docs_dir, graph)?;

    // 2. Generate architecture.md
    generate_docs_architecture(
        &docs_dir,
        &communities,
        graph,
        &edge_map,
        file_count,
        node_count,
        edge_count,
    )?;

    // 3. Generate getting-started.md
    generate_docs_getting_started(&docs_dir, repo_name, &communities, graph)?;

    // 4. Generate per-module files
    let module_page_count = generate_docs_modules(
        &modules_dir,
        &communities,
        graph,
        &edge_map,
        repo_path,
    )?;

    // 5b. Generate deployment guide
    generate_deployment_guide(&docs_dir, repo_name, graph)?;

    // 5d. Generate git analytics pages (hotspots, coupling, ownership)
    let git_analytics_count = generate_git_analytics_pages(&docs_dir, repo_path)?;

    // 5c. Generate ASP.NET MVC specific documentation (if applicable)
    let aspnet_pages = if super::super::generate_aspnet::has_aspnet_content(graph) {
        let pages = super::super::generate_aspnet::generate_aspnet_docs(graph, &docs_dir)?;
        if !pages.is_empty() {
            info!("ASP.NET docs generated: {} pages", pages.len());
            println!(
                "{} Generated {} ASP.NET documentation pages",
                "OK".green(),
                pages.len()
            );
        }
        pages
    } else {
        Vec::new()
    };

    // Total page count: static pages (overview, architecture, getting-started, deployment, functional-guide, project-health) + git analytics + module pages + ASP.NET pages
    let total_pages = 6 + git_analytics_count + module_page_count + aspnet_pages.len();
    info!("Documentation generated: {} pages total", total_pages);

    // 6. Generate _index.json LAST so it includes ASP.NET pages
    generate_docs_index(
        &docs_dir,
        repo_name,
        file_count,
        node_count,
        edge_count,
        communities.len(),
        &communities,
        &aspnet_pages,
    )?;

    println!(
        "{} Generated DeepWiki docs in {}",
        "OK".green(),
        docs_dir.display()
    );
    Ok(())
}

/// Generate the _index.json navigation file.
/// `aspnet_pages` contains (id, title, filename) tuples from ASP.NET doc generation.
#[allow(clippy::too_many_arguments)]
fn generate_docs_index(
    docs_dir: &Path,
    repo_name: &str,
    file_count: usize,
    node_count: usize,
    edge_count: usize,
    module_count: usize,
    communities: &BTreeMap<String, CommunityInfo>,
    aspnet_pages: &[(String, String, String)],
) -> Result<()> {
    let now = chrono::Local::now().to_rfc3339();

    // Build module children (deduplicated by filename)
    let mut module_children = Vec::new();
    let mut seen_modules = HashSet::new();
    for info in communities.values() {
        let filename = sanitize_filename(&info.label);
        if seen_modules.insert(filename.clone()) {
            module_children.push(json!({
                "id": format!("mod-{}", filename),
                "title": info.label,
                "path": format!("modules/{}.md", filename),
                "icon": "box"
            }));
        }
    }

    // Build ASP.NET children (grouped under an "ASP.NET MVC" section)
    let aspnet_icon_map: HashMap<&str, &str> = [
        ("aspnet-controllers", "server"),
        ("aspnet-routes", "route"),
        ("aspnet-entities", "table-2"),
        ("aspnet-views", "layout"),
        ("aspnet-areas", "layers"),
        ("aspnet-data-model", "database"),
        ("aspnet-seq-http", "arrow-right-left"),
        ("aspnet-seq-data", "hard-drive"),
    ].into_iter().collect();

    let mut pages_array = vec![
        json!({
            "id": "overview",
            "title": "Overview",
            "path": "overview.md",
            "icon": "home"
        }),
        json!({
            "id": "project-health",
            "title": "Santé du Projet",
            "path": "project-health.md",
            "icon": "activity"
        }),
        json!({
            "id": "architecture",
            "title": "Architecture",
            "path": "architecture.md",
            "icon": "git-branch"
        }),
        json!({
            "id": "git-analytics",
            "title": "Git Analytics",
            "icon": "git-commit",
            "children": [
                {
                    "id": "hotspots",
                    "title": "Code Hotspots",
                    "path": "hotspots.md",
                    "icon": "flame"
                },
                {
                    "id": "coupling",
                    "title": "Temporal Coupling",
                    "path": "coupling.md",
                    "icon": "link"
                },
                {
                    "id": "ownership",
                    "title": "Code Ownership",
                    "path": "ownership.md",
                    "icon": "users"
                }
            ]
        }),
        json!({
            "id": "getting-started",
            "title": "Getting Started",
            "path": "getting-started.md",
            "icon": "book-open"
        }),
        json!({
            "id": "deployment",
            "title": "Environnement & Déploiement",
            "path": "deployment.md",
            "icon": "cloud"
        }),
        json!({
            "id": "modules",
            "title": "Modules",
            "icon": "layers",
            "children": module_children
        }),
    ];

    // Add ASP.NET section if pages exist
    if !aspnet_pages.is_empty() {
        let aspnet_children: Vec<serde_json::Value> = aspnet_pages
            .iter()
            .map(|(id, title, filename)| {
                let icon = aspnet_icon_map.get(id.as_str()).unwrap_or(&"file-text");
                json!({
                    "id": id,
                    "title": title,
                    "path": filename,
                    "icon": icon
                })
            })
            .collect();

        pages_array.push(json!({
            "id": "aspnet",
            "title": "ASP.NET MVC 5 / EF6",
            "icon": "server",
            "children": aspnet_children
        }));
    }

    if pages_array.is_empty() {
        warn!("No documentation pages found in _index.json");
    }

    let index = json!({
        "title": repo_name,
        "generatedAt": now,
        "stats": {
            "files": file_count,
            "nodes": node_count,
            "edges": edge_count,
            "modules": module_count
        },
        "pages": pages_array
    });

    let index_path = docs_dir.join("_index.json");
    let mut f = std::fs::File::create(&index_path)?;
    writeln!(f, "{}", index)?;
    println!("  {} _index.json", "OK".green());
    Ok(())
}

/// Generate overview.md with DeepWiki-quality content.
#[allow(clippy::too_many_arguments)]
fn generate_docs_overview(
    docs_dir: &Path,
    repo_name: &str,
    file_count: usize,
    node_count: usize,
    edge_count: usize,
    lang_stats: &BTreeMap<String, usize>,
    communities: &BTreeMap<String, CommunityInfo>,
    graph: &KnowledgeGraph,
) -> Result<()> {
    let out_path = docs_dir.join("overview.md");
    let mut f = std::fs::File::create(&out_path)?;

    let label_counts = count_nodes_by_label(graph);
    let controller_count = label_counts.get(&NodeLabel::Controller).copied().unwrap_or(0);
    let view_count = label_counts.get(&NodeLabel::View).copied().unwrap_or(0);
    let entity_count = label_counts.get(&NodeLabel::DbEntity).copied().unwrap_or(0);
    let service_count = label_counts.get(&NodeLabel::Service).copied().unwrap_or(0)
        + label_counts.get(&NodeLabel::Repository).copied().unwrap_or(0);
    let ui_count = label_counts.get(&NodeLabel::UiComponent).copied().unwrap_or(0);

    // Title
    writeln!(f, "# {}", repo_name)?;
    writeln!(f, "<!-- GNX:LEAD -->")?;
    writeln!(f)?;

    // Relevant source files
    let top_files = top_connected_files(graph, 10);
    let top_files_refs: Vec<&str> = top_files.iter().map(|s| s.as_str()).collect();
    write!(f, "{}", source_files_section(&top_files_refs))?;

    // Business description — specific to the project type
    let (_languages, _frameworks, _ui_libs, _auto_desc) = detect_technology_stack(graph, lang_stats);
    let has_aspnet = label_counts.get(&NodeLabel::Controller).copied().unwrap_or(0) > 0;
    let has_ef = label_counts.get(&NodeLabel::DbContext).copied().unwrap_or(0) > 0;
    let has_telerik = label_counts.get(&NodeLabel::UiComponent).copied().unwrap_or(0) > 0;

    if has_aspnet && has_ef {
        writeln!(f, "> **{}** est une application de gestion métier construite en ASP.NET MVC 5 avec Entity Framework 6.", repo_name)?;
        if has_telerik {
            writeln!(f, "> L'interface utilise des grilles Telerik pour l'affichage et la saisie des données.")?;
        }
        let ext_count = label_counts.get(&NodeLabel::ExternalService).copied().unwrap_or(0);
        if ext_count > 0 {
            writeln!(f, "> Le système s'intègre avec {} services externes (WebAPI, WCF, LDAP).", ext_count)?;
        }
    } else {
        writeln!(f, "> {}", _auto_desc)?;
    }
    writeln!(f)?;

    // Metrics table
    writeln!(f, "| Metric | Value |")?;
    writeln!(f, "|--------|-------|")?;
    writeln!(f, "| Source Files | {} |", file_count)?;
    writeln!(f, "| Code Symbols | {} |", node_count)?;
    writeln!(f, "| Relationships | {} |", edge_count)?;
    if controller_count > 0 {
        writeln!(f, "| Controllers | {} |", controller_count)?;
    }
    if view_count > 0 {
        writeln!(f, "| Views | {} |", view_count)?;
    }
    if entity_count > 0 {
        writeln!(f, "| Database Entities | {} |", entity_count)?;
    }
    if service_count > 0 {
        writeln!(f, "| Services | {} |", service_count)?;
    }
    if ui_count > 0 {
        writeln!(f, "| UI Components | {} |", ui_count)?;
    }
    writeln!(f)?;

    // Technology Stack as a proper table
    let (languages, frameworks, ui_libs, _desc) = detect_technology_stack(graph, lang_stats);
    writeln!(f, "## Technology Stack")?;
    writeln!(f, "<!-- GNX:INTRO:technology-stack -->")?;
    writeln!(f)?;
    writeln!(f, "| Category | Technology |")?;
    writeln!(f, "|----------|-----------|")?;
    if !languages.is_empty() {
        writeln!(f, "| **Languages** | {} |", languages.join(", "))?;
    }
    if !frameworks.is_empty() {
        writeln!(f, "| **Frameworks** | {} |", frameworks.join(", "))?;
    }
    if !ui_libs.is_empty() {
        writeln!(f, "| **UI Components** | {} |", ui_libs.join(", "))?;
    }
    let ctx_count = label_counts.get(&NodeLabel::DbContext).copied().unwrap_or(0);
    if ctx_count > 0 {
        writeln!(f, "| **ORM** | Entity Framework 6 ({} DbContexts) |", ctx_count)?;
    }
    let ext_count = label_counts.get(&NodeLabel::ExternalService).copied().unwrap_or(0);
    if ext_count > 0 {
        writeln!(f, "| **Integrations** | {} external services (WebAPI, WCF) |", ext_count)?;
    }
    writeln!(f)?;

    // Key Subsystems
    if !communities.is_empty() {
        writeln!(f, "## Key Subsystems")?;
        writeln!(f, "<!-- GNX:INTRO:key-subsystems -->")?;
        writeln!(f)?;
        writeln!(f, "| Module | Members | Entry Points | Description |")?;
        writeln!(f, "|--------|---------|-------------|-------------|")?;
        for info in communities.values() {
            let member_count = info.member_ids.len();
            let entry_point_count = info
                .member_ids
                .iter()
                .filter_map(|mid| graph.get_node(mid))
                .filter(|n| n.properties.entry_point_score.map(|s| s > 0.3).unwrap_or(false))
                .count();
            let desc = info
                .description
                .as_deref()
                .unwrap_or(
                    if !info.keywords.is_empty() {
                        ""
                    } else {
                        "Module"
                    }
                );
            let desc_str = if desc.is_empty() {
                info.keywords.join(", ")
            } else {
                desc.to_string()
            };
            let filename = sanitize_filename(&info.label);
            writeln!(
                f,
                "| [{}](modules/{}.md) | {} | {} | {} |",
                info.label, filename, member_count, entry_point_count, desc_str
            )?;
        }
        writeln!(f)?;
    }

    // ── Signaux d'Alerte ────────────────────────────────────────────────
    {
        let density = if node_count > 0 {
            edge_count as f64 / node_count as f64
        } else {
            0.0
        };

        let total_files = graph.iter_nodes()
            .filter(|n| n.label == NodeLabel::File)
            .count();
        let traced_files = graph.iter_nodes()
            .filter(|n| n.properties.is_traced == Some(true))
            .count();
        let traced_pct = if total_files > 0 {
            (traced_files as f64 / total_files as f64) * 100.0
        } else {
            0.0
        };
        let ext_svc_count = label_counts.get(&NodeLabel::ExternalService).copied().unwrap_or(0);

        // Dead code stats
        let dead_count = graph.iter_nodes()
            .filter(|n| n.properties.is_dead_candidate == Some(true))
            .count();

        let mut has_alerts = false;

        if dead_count > 0 {
            let total_m = graph.iter_nodes()
                .filter(|n| matches!(n.label, NodeLabel::Method | NodeLabel::Function))
                .count();
            let dead_pct = if total_m > 0 { (dead_count as f64 / total_m as f64) * 100.0 } else { 0.0 };
            if !has_alerts {
                writeln!(f, "## Signaux d'Alerte")?;
                writeln!(f)?;
                has_alerts = true;
            }
            if dead_pct > 20.0 {
                writeln!(f, "> [!DANGER]")?;
            } else if dead_pct > 10.0 {
                writeln!(f, "> [!WARNING]")?;
            } else {
                writeln!(f, "> [!NOTE]")?;
            }
            writeln!(f, "> **{} méthodes** ({:.1}%) détectées comme code mort potentiel (aucun appelant).", dead_count, dead_pct)?;
            writeln!(f, "> Voir la page [Santé du Projet](project-health.md) pour le détail.")?;
            writeln!(f)?;
        }

        if traced_pct < 10.0 && total_files > 0 {
            if !has_alerts {
                writeln!(f, "## Signaux d'Alerte")?;
                writeln!(f)?;
                has_alerts = true;
            }
            writeln!(f, "> [!WARNING]")?;
            writeln!(f, "> Seulement {:.0}% des fichiers ont une traçabilité StackLogger.", traced_pct)?;
            writeln!(f, "> Les modules non tracés seront difficiles à déboguer en production.")?;
            writeln!(f)?;
        }

        if density > 3.0 {
            if !has_alerts {
                writeln!(f, "## Signaux d'Alerte")?;
                writeln!(f)?;
                has_alerts = true;
            }
            writeln!(f, "> [!DANGER]")?;
            writeln!(f, "> Densité de couplage élevée ({:.1}). Le système est fortement interconnecté.", density)?;
            writeln!(f, "> Tout changement peut avoir des effets de bord importants.")?;
            writeln!(f)?;
        }

        if ext_svc_count > 5 {
            if !has_alerts {
                writeln!(f, "## Signaux d'Alerte")?;
                writeln!(f)?;
                #[allow(unused_assignments)]
                { has_alerts = true; }
            }
            writeln!(f, "> [!NOTE]")?;
            writeln!(f, "> {} services externes détectés. Chaque intégration est un point de", ext_svc_count)?;
            writeln!(f, "> fragilité potentiel (timeout, indisponibilité, changement d'API).")?;
            writeln!(f)?;
        }
    }

    // GNX:CLOSING anchor before summary/navigation
    writeln!(f, "<!-- GNX:CLOSING -->")?;

    // Summary
    let ctrl_pages = controller_count;
    let data_pages = label_counts.get(&NodeLabel::DbContext).copied().unwrap_or(0);
    let svc_page = if service_count > 0 { 1 } else { 0 };
    let ui_page = if ui_count > 0 { 1 } else { 0 };
    let ajax_page = if label_counts.get(&NodeLabel::AjaxCall).copied().unwrap_or(0) > 0 { 1 } else { 0 };
    let total_pages = 4 + communities.len() + ctrl_pages + data_pages + svc_page + ui_page + ajax_page;

    writeln!(f, "## Summary")?;
    writeln!(f)?;
    writeln!(
        f,
        "This documentation covers {} pages organized into sections:",
        total_pages
    )?;
    writeln!(f, "Overview, Architecture, Getting Started, Déploiement, Modules")?;
    if controller_count > 0 {
        write!(f, ", Controllers")?;
    }
    if data_pages > 0 {
        write!(f, ", Data Model")?;
    }
    if service_count > 0 {
        write!(f, ", Services")?;
    }
    if ui_count > 0 {
        write!(f, ", UI Components")?;
    }
    writeln!(f, ".")?;
    writeln!(f)?;

    writeln!(f, "**See also:** [Architecture](./architecture.md) · [Getting Started](./getting-started.md)")?;
    writeln!(f)?;
    writeln!(f, "---")?;
    writeln!(f, "[Next: Architecture ->](./architecture.md)")?;

    println!("  {} overview.md", "OK".green());
    Ok(())
}

/// Generate architecture.md with real Mermaid diagram built from graph data.
fn generate_docs_architecture(
    docs_dir: &Path,
    communities: &BTreeMap<String, CommunityInfo>,
    graph: &KnowledgeGraph,
    edge_map: &HashMap<String, Vec<(String, RelationshipType)>>,
    _file_count: usize,
    node_count: usize,
    edge_count: usize,
) -> Result<()> {
    let out_path = docs_dir.join("architecture.md");
    let mut f = std::fs::File::create(&out_path)?;

    let label_counts = count_nodes_by_label(graph);
    let ctrl_count = label_counts.get(&NodeLabel::Controller).copied().unwrap_or(0);
    let view_count = label_counts.get(&NodeLabel::View).copied().unwrap_or(0);
    let svc_count = label_counts.get(&NodeLabel::Service).copied().unwrap_or(0)
        + label_counts.get(&NodeLabel::Repository).copied().unwrap_or(0);
    let ctx_count = label_counts.get(&NodeLabel::DbContext).copied().unwrap_or(0);
    let entity_count = label_counts.get(&NodeLabel::DbEntity).copied().unwrap_or(0);
    let ext_count = label_counts.get(&NodeLabel::ExternalService).copied().unwrap_or(0);
    let action_count = label_counts.get(&NodeLabel::ControllerAction).copied().unwrap_or(0);
    let ui_count = label_counts.get(&NodeLabel::UiComponent).copied().unwrap_or(0);
    let edmx_count: usize = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::File && n.properties.file_path.ends_with(".edmx"))
        .count();

    let arch_files: Vec<String> = graph.iter_nodes()
        .filter(|n| matches!(n.label, NodeLabel::Controller | NodeLabel::Service | NodeLabel::DbContext | NodeLabel::Repository))
        .map(|n| n.properties.file_path.clone())
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect();
    let arch_file_refs: Vec<&str> = arch_files.iter().take(15).map(|s| s.as_str()).collect();

    writeln!(f, "# Architecture")?;
    writeln!(f, "<!-- GNX:LEAD -->")?;
    writeln!(f)?;
    write!(f, "{}", source_files_section(&arch_file_refs))?;

    let has_tiered = ctrl_count > 0 && (svc_count > 0 || ctx_count > 0);

    if has_tiered {
        writeln!(f, "This project follows a **3-tier architecture** pattern:")?;
        writeln!(f, "Presentation (Controllers + Views) -> Business Logic (Services) -> Data Access (Entity Framework).")?;
    } else {
        writeln!(
            f,
            "System architecture with **{}** modules, **{}** nodes, and **{}** relationships.",
            communities.len(), node_count, edge_count
        )?;
    }
    writeln!(f)?;

    writeln!(f, "## Architecture Diagram")?;
    writeln!(f, "<!-- GNX:INTRO:architecture-diagram -->")?;
    writeln!(f)?;
    writeln!(f, "```mermaid")?;
    writeln!(f, "graph TD")?;

    if has_tiered {
        writeln!(f, "    subgraph Presentation")?;
        writeln!(f, "        C[\"Controllers ({})\"]", ctrl_count)?;
        if view_count > 0 {
            writeln!(f, "        V[\"Views ({})\"]", view_count)?;
        }
        writeln!(f, "    end")?;

        if svc_count > 0 {
            writeln!(f, "    subgraph Business[\"Business Logic\"]")?;
            writeln!(f, "        S[\"Services ({})\"]", svc_count)?;
            writeln!(f, "    end")?;
        }

        if ctx_count > 0 || entity_count > 0 {
            writeln!(f, "    subgraph Data[\"Data Access\"]")?;
            if ctx_count > 0 {
                writeln!(f, "        DB[\"DbContexts ({})\"]", ctx_count)?;
            }
            if entity_count > 0 {
                writeln!(f, "        E[\"Entities ({})\"]", entity_count)?;
            }
            writeln!(f, "    end")?;
        }

        if ext_count > 0 {
            writeln!(f, "    subgraph External")?;
            writeln!(f, "        EXT[\"External Services ({})\"]", ext_count)?;
            writeln!(f, "    end")?;
        }

        let has_ctrl_to_svc = svc_count > 0 && graph.iter_relationships().any(|r| {
            matches!(r.rel_type, RelationshipType::DependsOn | RelationshipType::Calls)
                && graph.get_node(&r.source_id).map(|n| n.label == NodeLabel::Controller).unwrap_or(false)
                && graph.get_node(&r.target_id).map(|n| matches!(n.label, NodeLabel::Service | NodeLabel::Repository)).unwrap_or(false)
        });
        let has_svc_to_db = ctx_count > 0 && graph.iter_relationships().any(|r| {
            matches!(r.rel_type, RelationshipType::DependsOn | RelationshipType::Calls | RelationshipType::Uses)
                && graph.get_node(&r.source_id).map(|n| matches!(n.label, NodeLabel::Service | NodeLabel::Repository)).unwrap_or(false)
                && graph.get_node(&r.target_id).map(|n| n.label == NodeLabel::DbContext).unwrap_or(false)
        });
        let has_db_to_entity = entity_count > 0 && graph.iter_relationships().any(|r| {
            r.rel_type == RelationshipType::MapsToEntity
        });
        let has_ctrl_to_view = view_count > 0 && graph.iter_relationships().any(|r| {
            r.rel_type == RelationshipType::RendersView
        });
        let has_svc_to_ext = ext_count > 0 && graph.iter_relationships().any(|r| {
            r.rel_type == RelationshipType::CallsService
        });

        if has_ctrl_to_svc || svc_count > 0 {
            writeln!(f, "    C --> S")?;
        }
        if has_svc_to_db || (ctx_count > 0 && svc_count > 0) {
            writeln!(f, "    S --> DB")?;
        }
        if has_db_to_entity || (entity_count > 0 && ctx_count > 0) {
            writeln!(f, "    DB --> E")?;
        }
        if has_ctrl_to_view || view_count > 0 {
            writeln!(f, "    C --> V")?;
        }
        if has_svc_to_ext || (ext_count > 0 && svc_count > 0) {
            writeln!(f, "    S --> EXT")?;
        }
    } else {
        for info in communities.values() {
            let safe_id = sanitize_filename(&info.label).replace('-', "_");
            writeln!(f, "    {}[\"{}\"]", safe_id, escape_mermaid_label(&info.label))?;
        }

        let mut member_to_community: HashMap<String, String> = HashMap::new();
        for info in communities.values() {
            for mid in &info.member_ids {
                member_to_community.insert(mid.clone(), info.label.clone());
            }
        }
        let mut cross_deps: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for rel in graph.iter_relationships() {
            if rel.rel_type == RelationshipType::Calls {
                if let (Some(src_comm), Some(tgt_comm)) = (
                    member_to_community.get(&rel.source_id),
                    member_to_community.get(&rel.target_id),
                ) {
                    if src_comm != tgt_comm {
                        cross_deps.entry(src_comm.clone()).or_default().insert(tgt_comm.clone());
                    }
                }
            }
        }
        for (src, targets) in &cross_deps {
            let src_id = sanitize_filename(src).replace('-', "_");
            for tgt in targets {
                let tgt_id = sanitize_filename(tgt).replace('-', "_");
                writeln!(f, "    {} --> {}", src_id, tgt_id)?;
            }
        }
    }
    writeln!(f, "```")?;
    writeln!(f)?;

    writeln!(f, "## Layer Details")?;
    writeln!(f, "<!-- GNX:INTRO:layer-details -->")?;
    writeln!(f)?;

    if ctrl_count > 0 {
        writeln!(f, "### Presentation Layer")?;
        writeln!(f, "{} controllers with {} actions serving {} views.", ctrl_count, action_count, view_count)?;
        if ui_count > 0 {
            writeln!(f, "{} Telerik/Kendo UI components detected.", ui_count)?;
        }
        writeln!(f)?;
    }

    if svc_count > 0 {
        writeln!(f, "### Business Logic Layer")?;
        writeln!(f, "{} services handling business rules and data processing.", svc_count)?;
        writeln!(f)?;
    }

    if ctx_count > 0 || entity_count > 0 {
        writeln!(f, "### Data Access Layer")?;
        writeln!(f, "{} Entity Framework DbContext classes managing {} entities", ctx_count, entity_count)?;
        if edmx_count > 0 {
            writeln!(f, "across {} EDMX data models.", edmx_count)?;
        } else {
            writeln!(f, ".")?;
        }
        writeln!(f)?;
    }

    if ext_count > 0 {
        writeln!(f, "### External Integrations")?;
        writeln!(f, "{} external service connections detected (WebAPI, WCF, LDAP).", ext_count)?;
        writeln!(f)?;

        let ext_services: Vec<&GraphNode> = graph.iter_nodes()
            .filter(|n| n.label == NodeLabel::ExternalService)
            .collect();
        if !ext_services.is_empty() {
            for svc in ext_services.iter().take(15) {
                let stype = svc.properties.service_type.as_deref().unwrap_or("REST");
                writeln!(f, "- **{}** ({})", svc.properties.name, stype)?;
            }
            writeln!(f)?;
        }
    }

    writeln!(f, "<!-- GNX:CLOSING -->")?;
    writeln!(f, "## Summary")?;
    writeln!(f)?;
    if has_tiered {
        writeln!(f, "The application follows a layered architecture with clear separation of concerns between presentation, business logic, and data access.")?;
    } else {
        writeln!(f, "The codebase is organized into {} interconnected modules.", communities.len())?;
    }
    writeln!(f)?;
    writeln!(f, "**See also:** [Overview](./overview.md) · [Getting Started](./getting-started.md)")?;
    writeln!(f)?;
    writeln!(f, "---")?;
    writeln!(f, "[<- Previous: Overview](./overview.md) | [Next: Getting Started ->](./getting-started.md)")?;

    let _ = edge_map;

    println!("  {} architecture.md", "OK".green());
    Ok(())
}

/// Generate getting-started.md guide.
fn generate_docs_getting_started(
    docs_dir: &Path,
    repo_name: &str,
    _communities: &BTreeMap<String, CommunityInfo>,
    graph: &KnowledgeGraph,
) -> Result<()> {
    let out_path = docs_dir.join("getting-started.md");
    let mut f = std::fs::File::create(&out_path)?;

    let mut ep_files: Vec<String> = graph
        .iter_nodes()
        .filter(|n| n.properties.entry_point_score.map(|s| s > 0.3).unwrap_or(false))
        .map(|n| n.properties.file_path.clone())
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect();
    ep_files.truncate(15);
    let ep_file_refs: Vec<&str> = ep_files.iter().map(|s| s.as_str()).collect();

    writeln!(f, "# Prise en Main")?;
    writeln!(f)?;
    write!(f, "{}", source_files_section(&ep_file_refs))?;
    writeln!(f, "Welcome to the **{}** codebase!", repo_name)?;
    writeln!(f)?;

    writeln!(f, "## Structure des Projets")?;
    writeln!(f)?;

    let mut project_files: BTreeMap<String, usize> = BTreeMap::new();
    for node in graph.iter_nodes() {
        if node.label == NodeLabel::File {
            let path = &node.properties.file_path;
            if !path.is_empty() && !path.contains("PackageTmp") && !path.contains("/obj/") {
                let project = path.split(['/', '\\']).next().unwrap_or("Other");
                *project_files.entry(project.to_string()).or_insert(0) += 1;
            }
        }
    }

    if !project_files.is_empty() {
        writeln!(f, "La solution contient **{} projets** :", project_files.len())?;
        writeln!(f)?;
        writeln!(f, "| Projet | Fichiers | Rôle |")?;
        writeln!(f, "|--------|----------|------|")?;
        let mut projects: Vec<_> = project_files.iter().collect();
        projects.sort_by(|a, b| b.1.cmp(a.1));
        for (project, count) in &projects {
            let role = describe_project_fr(project);
            writeln!(f, "| `{}` | {} | {} |", project, count, role)?;
        }
        writeln!(f)?;
    }

    let mut entry_points: Vec<(&GraphNode, f64)> = graph
        .iter_nodes()
        .filter_map(|n| {
            n.properties
                .entry_point_score
                .filter(|&s| s > 0.3)
                .map(|s| (n, s))
        })
        .collect();
    entry_points.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if !entry_points.is_empty() {
        writeln!(f, "## Points d'Entrée Principaux")?;
        writeln!(f)?;
        writeln!(f, "Commencez l'exploration par ces points d'entrée :")?;
        writeln!(f)?;
        for (node, _score) in entry_points.iter().take(10) {
            writeln!(f, "- `{}` in `{}`", node.properties.name, node.properties.file_path)?;
        }
        writeln!(f)?;
    }

    let has_controllers = graph.iter_nodes().any(|n| n.label == NodeLabel::Controller);
    if has_controllers {
        writeln!(f, "## Prérequis & Setup local")?;
        writeln!(f, "<!-- GNX:INTRO:setup-local -->")?;
        writeln!(f)?;
        writeln!(f, "Ce projet est une application **ASP.NET MVC 5** (.NET Framework).")?;
        writeln!(f)?;
        writeln!(f, "### Prérequis")?;
        writeln!(f)?;
        writeln!(f, "| Outil | Version | Notes |")?;
        writeln!(f, "|-------|---------|-------|")?;
        writeln!(f, "| Visual Studio | 2019+ | Avec le workload \"Développement web ASP.NET\" |")?;
        writeln!(f, "| .NET Framework | 4.6.1+ | Vérifier dans `web.config` \u{2192} `targetFramework` |")?;
        writeln!(f, "| SQL Server | 2016+ | Base de données locale ou distante |")?;
        writeln!(f, "| IIS Express | intégré à VS | Pour le debug local |")?;
        writeln!(f)?;
        writeln!(f, "### Étapes de démarrage")?;
        writeln!(f)?;
        writeln!(f, "1. **Ouvrir la solution** `.sln` dans Visual Studio")?;
        writeln!(f, "2. **Restaurer les packages NuGet** : clic droit sur la solution \u{2192} Restaurer les packages NuGet")?;
        writeln!(f, "3. **Configurer la connexion DB** : vérifier `web.config` \u{2192} `<connectionStrings>`")?;
        writeln!(f, "4. **Compiler** : Ctrl+Shift+B")?;
        writeln!(f, "5. **Lancer** : F5 (IIS Express)")?;
        writeln!(f)?;

        let config_files: Vec<&GraphNode> = graph
            .iter_nodes()
            .filter(|n| {
                n.label == NodeLabel::File
                    && (n.properties.file_path.ends_with("web.config")
                        || n.properties.file_path.ends_with("Web.config"))
                    && !n.properties.file_path.contains("PackageTmp")
                    && !n.properties.file_path.contains("/obj/")
            })
            .collect();

        if !config_files.is_empty() {
            writeln!(f, "### Fichiers de configuration")?;
            writeln!(f)?;
            for cf in &config_files {
                writeln!(f, "- `{}`", cf.properties.file_path.replace('\\', "/"))?;
            }
            writeln!(f)?;
        }
    }

    writeln!(f, "## Pour aller plus loin")?;
    writeln!(f)?;
    writeln!(f, "- Consultez l'**Architecture** pour comprendre les couches du système")?;
    writeln!(f, "- Explorez les **Controllers** pour voir les fonctionnalités par écran")?;
    writeln!(f, "- Le **Guide Fonctionnel** décrit chaque module du point de vue métier")?;
    writeln!(f, "- Les **Services Externes** détaillent les intégrations (Erable, WCF)")?;
    writeln!(f)?;

    writeln!(f, "**Voir aussi :** [Vue d'ensemble](./overview.md) · [Architecture](./architecture.md)")?;
    writeln!(f)?;
    writeln!(f, "---")?;
    writeln!(f, "[<- Previous: Architecture](./architecture.md) | [Next: Modules ->](./modules/)")?;

    println!("  {} getting-started.md", "OK".green());
    Ok(())
}

// The generate_docs_modules function is extremely large (lines 3226-4484 in original).
// It is included in full below to preserve all functionality.

/// Generate per-module documentation files with page ordering and navigation.
fn generate_docs_modules(
    modules_dir: &Path,
    communities: &BTreeMap<String, CommunityInfo>,
    graph: &KnowledgeGraph,
    edge_map: &HashMap<String, Vec<(String, RelationshipType)>>,
    repo_path: &Path,
) -> Result<usize> {
    let mut page_count: usize = 0;

    let mut member_to_community: HashMap<String, String> = HashMap::new();
    for info in communities.values() {
        for mid in &info.member_ids {
            member_to_community.insert(mid.clone(), info.label.clone());
        }
    }

    let mut page_order: Vec<(String, String)> = vec![
        ("../overview".to_string(), "Overview".to_string()),
        ("../project-health".to_string(), "Santé du Projet".to_string()),
        ("../architecture".to_string(), "Architecture".to_string()),
        ("../getting-started".to_string(), "Getting Started".to_string()),
    ];

    let mut merged_communities: BTreeMap<String, CommunityInfo> = BTreeMap::new();
    for info in communities.values() {
        let base = sanitize_filename(&info.label);
        let entry = merged_communities.entry(base).or_insert_with(|| CommunityInfo {
            label: info.label.clone(),
            description: info.description.clone(),
            member_ids: Vec::new(),
            keywords: Vec::new(),
        });
        for mid in &info.member_ids {
            if !entry.member_ids.contains(mid) {
                entry.member_ids.push(mid.clone());
            }
        }
        for kw in &info.keywords {
            if !entry.keywords.contains(kw) {
                entry.keywords.push(kw.clone());
            }
        }
    }

    let mut community_filenames: Vec<(String, String)> = Vec::new();
    for (filename, info) in &merged_communities {
        community_filenames.push((filename.clone(), info.label.clone()));
        page_order.push((filename.clone(), info.label.clone()));
    }

    let mut controllers: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::Controller)
        .collect();
    controllers.sort_by(|a, b| a.properties.name.cmp(&b.properties.name));

    let ctrl_filenames: Vec<(String, String)> = controllers.iter()
        .map(|c| {
            let fname = format!("ctrl-{}", sanitize_filename(&c.properties.name));
            (fname, c.properties.name.clone())
        })
        .collect();
    for (fname, title) in &ctrl_filenames {
        page_order.push((fname.clone(), title.clone()));
    }

    let db_contexts: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::DbContext)
        .collect();
    let data_filenames: Vec<(String, String)> = db_contexts.iter()
        .map(|c| {
            let fname = format!("data-{}", sanitize_filename(&c.properties.name));
            (fname, format!("Data Model: {}", c.properties.name))
        })
        .collect();
    for (fname, title) in &data_filenames {
        page_order.push((fname.clone(), title.clone()));
    }

    let services: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::Service || n.label == NodeLabel::Repository)
        .collect();
    if !services.is_empty() {
        page_order.push(("services".to_string(), "Service Layer".to_string()));
    }

    let ui_components: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::UiComponent)
        .collect();
    if !ui_components.is_empty() {
        page_order.push(("ui-components".to_string(), "UI Components".to_string()));
    }

    let ajax_calls: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::AjaxCall)
        .collect();
    if !ajax_calls.is_empty() {
        page_order.push(("ajax-endpoints".to_string(), "AJAX Endpoints".to_string()));
    }

    let ext_services: Vec<&GraphNode> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::ExternalService)
        .collect();
    if !ext_services.is_empty() {
        page_order.push(("external-services".to_string(), "External Services".to_string()));
    }

    fn nav_footer(page_order: &[(String, String)], current_filename: &str) -> String {
        let idx = page_order.iter().position(|(f, _)| f == current_filename);
        let mut footer = String::from("\n---\n");
        if let Some(i) = idx {
            if i > 0 {
                let (prev_file, prev_title) = &page_order[i - 1];
                footer.push_str(&format!("[<- Previous: {}](./{}.md)", prev_title, prev_file));
            }
            if i > 0 && i + 1 < page_order.len() {
                footer.push_str(" | ");
            }
            if i + 1 < page_order.len() {
                let (next_file, next_title) = &page_order[i + 1];
                footer.push_str(&format!("[Next: {} ->](./{}.md)", next_title, next_file));
            }
        }
        footer.push('\n');
        footer
    }

    // The rest of this function generates community pages, controller pages, data model pages,
    // service layer page, UI components page, AJAX endpoints page, and external services page.
    // It is extremely large but must be included in full for correctness.

    // ─── Community / Module pages (deduplicated) ──────────────────────
    for (comm_idx, (filename, info)) in merged_communities.iter().enumerate() {
        let _ = comm_idx;
        let out_path = modules_dir.join(format!("{}.md", filename));
        let mut f = std::fs::File::create(&out_path)?;

        let member_set: HashSet<&str> = info.member_ids.iter().map(|s| s.as_str()).collect();

        let mut files_set: BTreeSet<String> = BTreeSet::new();
        for mid in &info.member_ids {
            if let Some(node) = graph.get_node(mid) {
                if !node.properties.file_path.is_empty() {
                    files_set.insert(node.properties.file_path.clone());
                }
            }
        }
        let files_vec: Vec<&str> = files_set.iter().map(|s| s.as_str()).collect();

        writeln!(f, "# {}", info.label)?;
        writeln!(f)?;
        write!(f, "{}", source_files_section(&files_vec))?;

        if let Some(desc) = &info.description {
            writeln!(f, "{}", desc)?;
            writeln!(f)?;
        }

        if !info.keywords.is_empty() {
            writeln!(f, "**Keywords**: {}", info.keywords.join(", "))?;
            writeln!(f)?;
        }

        let mut internal_calls: Vec<(String, String)> = Vec::new();
        for mid in &info.member_ids {
            if let Some(edges) = edge_map.get(mid.as_str()) {
                for (target_id, rel_type) in edges {
                    if *rel_type == RelationshipType::Calls && member_set.contains(target_id.as_str()) {
                        let src_name = graph.get_node(mid).map(|n| n.properties.name.as_str()).unwrap_or("?");
                        let tgt_name = graph.get_node(target_id).map(|n| n.properties.name.as_str()).unwrap_or("?");
                        internal_calls.push((src_name.to_string(), tgt_name.to_string()));
                    }
                }
            }
        }

        if !internal_calls.is_empty() && internal_calls.len() <= 30 {
            writeln!(f, "## Call Graph")?;
            writeln!(f)?;
            writeln!(f, "```mermaid")?;
            writeln!(f, "graph LR")?;
            let mut seen_nodes = HashSet::new();
            for (src, tgt) in &internal_calls {
                let src_safe = sanitize_filename(src).replace('-', "_");
                let tgt_safe = sanitize_filename(tgt).replace('-', "_");
                if seen_nodes.insert(src_safe.clone()) {
                    writeln!(f, "    {}[\"{}\"]", src_safe, escape_mermaid_label(src))?;
                }
                if seen_nodes.insert(tgt_safe.clone()) {
                    writeln!(f, "    {}[\"{}\"]", tgt_safe, escape_mermaid_label(tgt))?;
                }
                writeln!(f, "    {} --> {}", src_safe, tgt_safe)?;
            }
            writeln!(f, "```")?;
            writeln!(f)?;
        }

        writeln!(f, "## Members")?;
        writeln!(f)?;
        writeln!(f, "| Symbol | Type | File | Lines |")?;
        writeln!(f, "|--------|------|------|-------|")?;

        for mid in &info.member_ids {
            if let Some(node) = graph.get_node(mid) {
                let lines = match (node.properties.start_line, node.properties.end_line) {
                    (Some(s), Some(e)) => format!("{}-{}", s, e),
                    (Some(s), None) => format!("{}", s),
                    _ => "-".to_string(),
                };
                writeln!(f, "| `{}` | {} | `{}` | {} |", node.properties.name, node.label.as_str(), node.properties.file_path, lines)?;
            }
        }
        writeln!(f)?;

        let mut entry_points: Vec<&GraphNode> = info
            .member_ids.iter()
            .filter_map(|mid| graph.get_node(mid))
            .filter(|n| n.properties.entry_point_score.map(|s| s > 0.3).unwrap_or(false))
            .collect();
        entry_points.sort_by(|a, b| {
            let sa = a.properties.entry_point_score.unwrap_or(0.0);
            let sb = b.properties.entry_point_score.unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

        if !entry_points.is_empty() {
            writeln!(f, "## Entry Points")?;
            writeln!(f)?;
            for node in entry_points.iter().take(10) {
                let score = node.properties.entry_point_score.unwrap_or(0.0);
                writeln!(f, "- `{}` (score: {:.2}) in `{}`", node.properties.name, score, node.properties.file_path)?;
            }
            writeln!(f)?;
        }

        if !internal_calls.is_empty() {
            writeln!(f, "## Internal Calls")?;
            writeln!(f)?;
            for (src, tgt) in &internal_calls {
                writeln!(f, "- `{}` -> `{}`", src, tgt)?;
            }
            writeln!(f)?;
        }

        let mut external_deps: BTreeMap<String, usize> = BTreeMap::new();
        for mid in &info.member_ids {
            if let Some(edges) = edge_map.get(mid.as_str()) {
                for (target_id, rel_type) in edges {
                    if *rel_type == RelationshipType::Calls && !member_set.contains(target_id.as_str()) {
                        if let Some(target_comm) = member_to_community.get(target_id) {
                            *external_deps.entry(target_comm.clone()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        if !external_deps.is_empty() {
            writeln!(f, "## External Dependencies")?;
            writeln!(f)?;
            let mut sorted: Vec<_> = external_deps.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));
            for (target_comm, count) in sorted {
                let target_filename = sanitize_filename(&target_comm);
                writeln!(f, "- [**{}**]({}.md) - {} call(s)", target_comm, target_filename, count)?;
            }
            writeln!(f)?;
        }

        if !files_set.is_empty() {
            writeln!(f, "## Files")?;
            writeln!(f)?;
            for file_path in &files_set {
                writeln!(f, "- `{}`", file_path)?;
            }
            writeln!(f)?;
        }

        write!(f, "{}", nav_footer(&page_order, filename))?;
        println!("  {} modules/{filename}.md", "OK".green());
        page_count += 1;
    }

    // ─── Per-Controller pages ──────────────────────────────────────────
    // This section is very large. Including the full controller page generation
    // with action tables, callers, views, dependencies, impact analysis, action details.
    // Due to the extreme size, the controller, data model, services, UI, AJAX, and
    // external services page generation code follows the exact same logic from the original.

    for (ctrl_idx, ctrl) in controllers.iter().enumerate() {
        let ctrl_name = &ctrl.properties.name;
        let (filename, _) = &ctrl_filenames[ctrl_idx];
        let out_path = modules_dir.join(format!("{filename}.md"));

        let mut actions: Vec<&GraphNode> = graph.iter_nodes()
            .filter(|n| n.label == NodeLabel::ControllerAction && n.properties.file_path == ctrl.properties.file_path)
            .collect();
        actions.sort_by(|a, b| a.properties.start_line.unwrap_or(0).cmp(&b.properties.start_line.unwrap_or(0)));

        if actions.len() < 3 { continue; }

        let action_ids: HashSet<String> = actions.iter().map(|a| a.id.clone()).collect();
        let caller_rels: Vec<&GraphRelationship> = graph.iter_relationships()
            .filter(|r| action_ids.contains(&r.target_id) && (r.rel_type == RelationshipType::CallsAction || r.rel_type == RelationshipType::Calls))
            .collect();

        let mut action_callers: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for r in &caller_rels {
            let short_name = if let Some(src_node) = graph.get_node(&r.source_id) {
                let label_str = match src_node.label {
                    NodeLabel::View | NodeLabel::PartialView => src_node.properties.file_path.rsplit(['/', '\\']).next().unwrap_or(&src_node.properties.name).to_string(),
                    NodeLabel::UiComponent => {
                        let file = src_node.properties.file_path.rsplit(['/', '\\']).next().unwrap_or("vue");
                        let model = src_node.properties.bound_model.as_deref().unwrap_or("");
                        let cols = src_node.properties.description.as_deref().unwrap_or("");
                        if !model.is_empty() && !cols.is_empty() {
                            let short_cols: String = cols.chars().take(30).collect();
                            format!("{} Grid<{}> [{}]", file, model, short_cols)
                        } else if !model.is_empty() { format!("{} Grid<{}>", file, model) }
                        else { format!("{} (Grille)", file) }
                    }
                    NodeLabel::AjaxCall | NodeLabel::ScriptFile => src_node.properties.file_path.rsplit(['/', '\\']).next().unwrap_or(&src_node.properties.name).to_string(),
                    _ => src_node.properties.name.clone(),
                };
                let type_str = match src_node.label {
                    NodeLabel::View => "Vue".to_string(),
                    NodeLabel::PartialView => "Partielle".to_string(),
                    NodeLabel::UiComponent => "Grille".to_string(),
                    NodeLabel::AjaxCall => { let m = src_node.properties.ajax_method.as_deref().unwrap_or("AJAX"); format!("AJAX {}", m) }
                    NodeLabel::ScriptFile => "Script".to_string(),
                    _ => format!("{:?}", src_node.label),
                };
                (label_str, type_str)
            } else {
                let short = r.source_id.rsplit(':').next().unwrap_or(&r.source_id).to_string();
                (short, "Unknown".to_string())
            };
            let entry = action_callers.entry(r.target_id.clone()).or_default();
            if !entry.iter().any(|(n, _)| *n == short_name.0) { entry.push(short_name); }
        }

        let view_targets: Vec<String> = graph.iter_relationships()
            .filter(|r| r.rel_type == RelationshipType::RendersView && (r.source_id.contains(ctrl_name.as_str()) || graph.get_node(&r.source_id).map(|n| n.properties.file_path == ctrl.properties.file_path).unwrap_or(false)))
            .map(|r| r.target_id.clone()).collect();
        let mut view_files: Vec<String> = view_targets.iter()
            .filter_map(|vid| graph.get_node(vid).map(|n| n.properties.file_path.clone()))
            .collect::<BTreeSet<String>>().into_iter().collect();
        if view_files.is_empty() { view_files = view_targets.iter().cloned().collect::<BTreeSet<String>>().into_iter().collect(); }

        let dependencies: Vec<String> = graph.iter_relationships()
            .filter(|r| r.rel_type == RelationshipType::DependsOn && (r.source_id.contains(ctrl_name.as_str()) || graph.get_node(&r.source_id).map(|n| n.properties.file_path == ctrl.properties.file_path && n.label == NodeLabel::Controller).unwrap_or(false)))
            .filter_map(|r| graph.get_node(&r.target_id).map(|n| n.properties.name.clone()))
            .collect::<BTreeSet<String>>().into_iter().collect();

        let mut src_files: Vec<String> = vec![ctrl.properties.file_path.clone()];
        src_files.extend(view_files.iter().cloned());
        let src_file_refs: Vec<&str> = src_files.iter().take(15).map(|s| s.as_str()).collect();

        let known_types: HashSet<String> = graph.iter_nodes()
            .filter(|n| matches!(n.label, NodeLabel::DbEntity | NodeLabel::ViewModel | NodeLabel::Class))
            .map(|n| n.properties.name.clone()).collect();

        let mut content = format!("# {}\n\n", ctrl_name);
        content.push_str("<!-- GNX:LEAD -->\n");
        content.push_str(&source_files_section(&src_file_refs));

        let base_name = ctrl_name.trim_end_matches("Controller");
        let action_count = actions.len();
        let desc = describe_controller(ctrl_name);
        content.push_str(&format!("> {} manages {} endpoints for {}.\n\n", base_name, action_count, desc));

        content.push_str(&format!("## Actions ({})\n\n", action_count));
        content.push_str("| # | Action | Method | Paramètres | Retour | Appelé par |\n");
        content.push_str("|---|--------|--------|-----------|--------|------------|\n");
        for (i, action) in actions.iter().enumerate() {
            let method = action.properties.http_method.as_deref().unwrap_or("GET");
            let ret = action.properties.return_type.as_deref().unwrap_or("ActionResult");
            let params = extract_params_linked(action.properties.description.as_deref().unwrap_or(""), &known_types);
            let called_by = action_callers.get(&action.id)
                .map(|callers| callers.iter().take(3).map(|(name, _)| name.as_str()).collect::<Vec<_>>().join(", "))
                .unwrap_or_else(|| "-".to_string());
            content.push_str(&format!("| {} | **{}** | {} | {} | {} | {} |\n", i + 1, action.properties.name, method, params, ret, called_by));
        }
        content.push('\n');
        content.push_str("<!-- GNX:TIP:actions -->\n");

        // Impact Analysis
        {
            let mut action_impacts: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();
            for action in &actions {
                let action_name = action.properties.name.clone();
                let callees: Vec<String> = graph.iter_relationships()
                    .filter(|r| r.source_id == action.id && matches!(r.rel_type, RelationshipType::Calls | RelationshipType::CallsAction | RelationshipType::DependsOn | RelationshipType::CallsService))
                    .filter_map(|r| graph.get_node(&r.target_id).map(|n| n.properties.name.clone()))
                    .collect::<BTreeSet<String>>().into_iter().collect();
                let callers: Vec<String> = graph.iter_relationships()
                    .filter(|r| r.target_id == action.id && matches!(r.rel_type, RelationshipType::Calls | RelationshipType::CallsAction))
                    .filter_map(|r| graph.get_node(&r.source_id).map(|n| n.properties.name.clone()))
                    .collect::<BTreeSet<String>>().into_iter().collect();
                if callees.len() + callers.len() > 0 { action_impacts.push((action_name, callees, callers)); }
            }
            action_impacts.sort_by(|a, b| (b.1.len() + b.2.len()).cmp(&(a.1.len() + a.2.len())));
            if !action_impacts.is_empty() {
                content.push_str("## Analyse d'Impact\n\n> Si une action de ce controller est modifiée, voici les composants potentiellement impactés.\n\n");
                content.push_str("| Action modifiée | Impact aval (callees) | Impact amont (callers) |\n|----------------|----------------------|----------------------|\n");
                for (action_name, callees, callers) in action_impacts.iter().take(5) {
                    let callees_str = if callees.is_empty() { "-".to_string() } else { callees.iter().take(5).map(|s| format!("`{}`", s)).collect::<Vec<_>>().join(", ") };
                    let callers_str = if callers.is_empty() { "-".to_string() } else { callers.iter().take(5).map(|s| format!("`{}`", s)).collect::<Vec<_>>().join(", ") };
                    content.push_str(&format!("| **{}** | {} | {} |\n", action_name, callees_str, callers_str));
                }
                content.push('\n');
            }
        }

        // Callers section
        if !caller_rels.is_empty() {
            let mut caller_rows: Vec<(String, String, String, String)> = Vec::new();
            let mut seen_callers: HashSet<(String, String)> = HashSet::new();
            for r in &caller_rels {
                let (source_name, source_type) = if let Some(src_node) = graph.get_node(&r.source_id) {
                    let name = match src_node.label { NodeLabel::View | NodeLabel::PartialView => src_node.properties.file_path.rsplit(['/', '\\']).next().unwrap_or(&src_node.properties.name).to_string(), _ => src_node.properties.name.clone() };
                    let stype = match src_node.label {
                        NodeLabel::View => { if r.reason.contains("form") || r.reason.contains("Form") { "View (Form)".to_string() } else { "View".to_string() } }
                        NodeLabel::PartialView => "Partial View".to_string(),
                        NodeLabel::AjaxCall => { let ajax_type = src_node.properties.ajax_method.as_deref().unwrap_or("AJAX"); if src_node.properties.ajax_url.as_deref().map(|u| u.contains("getJSON")).unwrap_or(false) { "Script ($.getJSON)".to_string() } else { format!("Script ({})", ajax_type) } }
                        _ => format!("{:?}", src_node.label),
                    };
                    (name, stype)
                } else { let short = r.source_id.rsplit(':').next().unwrap_or(&r.source_id).to_string(); (short, "Unknown".to_string()) };
                let target_action = graph.get_node(&r.target_id).map(|n| n.properties.name.clone()).unwrap_or_else(|| r.target_id.rsplit(':').next().unwrap_or(&r.target_id).to_string());
                let method = graph.get_node(&r.target_id).and_then(|n| n.properties.http_method.as_ref()).cloned().unwrap_or_else(|| "-".to_string());
                let key = (source_name.clone(), target_action.clone());
                if seen_callers.insert(key) { caller_rows.push((source_name, source_type, target_action, method)); }
            }
            if !caller_rows.is_empty() {
                content.push_str("## Callers\n\nThis controller is called from:\n\n| Source | Type | Action | Method |\n|--------|------|--------|--------|\n");
                for (source, stype, action, method) in &caller_rows {
                    content.push_str(&format!("| {} | {} | {} | {} |\n", source, stype, action, method));
                }
                content.push('\n');
            }
        }

        if !view_files.is_empty() { content.push_str("## Associated Views\n\n"); for v in &view_files { content.push_str(&format!("- `{}`\n", v)); } content.push('\n'); }
        if !dependencies.is_empty() { content.push_str("## Dependencies\n\n"); for dep in &dependencies { content.push_str(&format!("- `{}`\n", dep)); } content.push('\n'); }

        // Action Details
        if !actions.is_empty() {
            content.push_str("## Action Details\n\n");
            for action in &actions {
                let method = action.properties.http_method.as_deref().unwrap_or("GET");
                let params_short = extract_params_from_content(action.properties.description.as_deref().unwrap_or(""), &action.properties.name);
                content.push_str(&format!("<details>\n<summary><strong>{}</strong> ({}) — {}</summary>\n\n", action.properties.name, method, if params_short == "-" { "aucun paramètre".to_string() } else { params_short.clone() }));
                content.push_str(&format!("**Fichier :** `{}`", ctrl.properties.file_path));
                if let Some(line) = action.properties.start_line { content.push_str(&format!(" (ligne {})", line)); }
                content.push('\n');
                if params_short != "-" { content.push_str(&format!("**Paramètres :** {}\n", params_short)); }
                let ret = action.properties.return_type.as_deref().unwrap_or("ActionResult");
                content.push_str(&format!("**Returns:** {}\n", ret));
                if let Some(callers) = action_callers.get(&action.id) {
                    let caller_strs: Vec<String> = callers.iter().map(|(name, stype)| format!("{} ({})", name, stype)).collect();
                    if !caller_strs.is_empty() { content.push_str(&format!("**Appelé par :** {}\n", caller_strs.join(", "))); }
                }
                let source_path = repo_path.join(&ctrl.properties.file_path);
                if let Ok(source) = std::fs::read_to_string(&source_path) {
                    if let Some(snippet) = extract_method_body(&source, &action.properties.name, 50) {
                        content.push_str("\n```csharp\n"); content.push_str(&snippet); content.push_str("```\n");
                    }
                }
                content.push_str("\n</details>\n\n");
            }
        }

        content.push_str("<!-- GNX:CLOSING -->\n");
        content.push_str(&format!("## Summary\n\n**{}** provides {} actions.\n\n", ctrl_name, action_count));
        content.push_str("**See also:** [Architecture](../architecture.md) · [Services](./services.md)\n");
        content.push_str(&nav_footer(&page_order, filename));

        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    // ─── Data Model pages ──────────────────────────────────────────────
    for (ctx_idx, ctx) in db_contexts.iter().enumerate() {
        let ctx_name = &ctx.properties.name;
        let (filename, _) = &data_filenames[ctx_idx];
        let out_path = modules_dir.join(format!("{filename}.md"));
        let entities: Vec<&GraphNode> = graph.iter_nodes().filter(|n| n.label == NodeLabel::DbEntity).collect();
        let mut src_files: Vec<String> = vec![ctx.properties.file_path.clone()];
        for e in &entities { if !e.properties.file_path.is_empty() { src_files.push(e.properties.file_path.clone()); } }
        let src_files_dedup: Vec<String> = src_files.into_iter().collect::<BTreeSet<String>>().into_iter().collect();
        let src_file_refs: Vec<&str> = src_files_dedup.iter().take(15).map(|s| s.as_str()).collect();

        let mut content = format!("# Data Model: {}\n\n<!-- GNX:LEAD -->\n", ctx_name);
        content.push_str(&source_files_section(&src_file_refs));
        content.push_str(&format!("**File:** `{}`\n\n**Entities:** {}\n\n", ctx.properties.file_path, entities.len()));

        let mut entity_rels: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for rel in graph.iter_relationships() {
            if rel.rel_type == RelationshipType::AssociatesWith {
                let src = rel.source_id.rsplit(':').next().unwrap_or(&rel.source_id).to_string();
                let tgt = rel.target_id.rsplit(':').next().unwrap_or(&rel.target_id).to_string();
                let card = if rel.reason.contains("1:*") { "||--o{" } else if rel.reason.contains("*:1") { "}o--||" } else if rel.reason.contains("*:*") { "}o--o{" } else { "||--||" };
                entity_rels.entry(src.clone()).or_default().push((tgt.clone(), card.to_string()));
                entity_rels.entry(tgt).or_default().push((src, card.to_string()));
            }
        }

        content.push_str("## Entities\n\n");
        for entity in &entities {
            let ename = &entity.properties.name;
            let rels = entity_rels.get(ename.as_str());
            let rel_count = rels.map_or(0, |v| v.len());
            content.push_str(&format!("<details id=\"{}\">\n<summary><strong>{}</strong> — <code>{}</code> ({} relations)</summary>\n\n", ename, ename, entity.properties.file_path, rel_count));
            if rel_count > 0 {
                if let Some(rels) = rels {
                    content.push_str("```mermaid\ngraph LR\n");
                    let eid = sanitize_mermaid_id(ename);
                    content.push_str(&format!("    {}[\"{}\"]\n    style {} fill:#4a85e0,color:#fff,stroke:#3a73cc\n", eid, ename, eid));
                    let mut seen: HashSet<String> = HashSet::new();
                    for (target, _) in rels.iter().take(8) {
                        if seen.insert(target.clone()) {
                            let tid = sanitize_mermaid_id(target);
                            content.push_str(&format!("    {}[\"{}\"]\n    {} --- {}\n", tid, target, eid, tid));
                        }
                    }
                    if rels.len() > 8 { content.push_str(&format!("    more((\"...+{}\"))\n    {} -.- more\n", rels.len() - 8, eid)); }
                    content.push_str("```\n\n");
                }
            } else { content.push_str("*Aucune relation détectée dans le modèle.*\n\n"); }
            content.push_str("</details>\n\n");
        }
        content.push_str(&nav_footer(&page_order, filename));
        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    // ─── Service Layer page ────────────────────────────────────────────
    if !services.is_empty() {
        let out_path = modules_dir.join("services.md");
        let svc_files: Vec<String> = services.iter().map(|s| s.properties.file_path.clone()).collect::<BTreeSet<String>>().into_iter().collect();
        let svc_file_refs: Vec<&str> = svc_files.iter().take(15).map(|s| s.as_str()).collect();
        let mut content = String::from("# Service Layer\n\n<!-- GNX:LEAD -->\n");
        content.push_str(&source_files_section(&svc_file_refs));
        content.push_str(&format!("**Total services:** {}\n\n", services.len()));
        let mut service_used_by: HashMap<String, Vec<String>> = HashMap::new();
        for svc in &services {
            let users: Vec<String> = graph.iter_relationships().filter(|r| r.rel_type == RelationshipType::DependsOn && r.target_id == svc.id).filter_map(|r| graph.get_node(&r.source_id).filter(|n| n.label == NodeLabel::Controller).map(|n| n.properties.name.clone())).collect::<BTreeSet<String>>().into_iter().collect();
            service_used_by.insert(svc.id.clone(), users);
        }
        content.push_str("## Services\n\n| Service | Type | Interface | Used By | Purpose | File |\n|---------|------|-----------|---------|---------|------|\n");
        for svc in &services {
            let layer = svc.properties.layer_type.as_deref().unwrap_or("Service");
            let iface = svc.properties.implements_interface.as_deref().unwrap_or("-");
            let used_by = service_used_by.get(&svc.id).map(|users| if users.is_empty() { "-".to_string() } else { users.iter().take(3).cloned().collect::<Vec<_>>().join(", ") }).unwrap_or_else(|| "-".to_string());
            let purpose = describe_service_fr(&svc.properties.name);
            content.push_str(&format!("| {} | {} | {} | {} | {} | `{}` |\n", svc.properties.name, layer, iface, used_by, purpose, svc.properties.file_path));
        }
        content.push('\n');
        content.push_str(&nav_footer(&page_order, "services"));
        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    // ─── UI Components page ────────────────────────────────────────────
    if !ui_components.is_empty() {
        let out_path = modules_dir.join("ui-components.md");
        let ui_files: Vec<String> = ui_components.iter().map(|c| c.properties.file_path.clone()).collect::<BTreeSet<String>>().into_iter().collect();
        let ui_file_refs: Vec<&str> = ui_files.iter().take(15).map(|s| s.as_str()).collect();
        let mut content = String::from("# UI Components (Telerik/Kendo)\n\n");
        content.push_str(&source_files_section(&ui_file_refs));
        content.push_str(&format!("**Total components:** {}\n\n", ui_components.len()));
        content.push_str("| Component | Type | Model | Columns | File |\n|-----------|------|-------|---------|------|\n");
        for comp in &ui_components {
            let comp_type = comp.properties.component_type.as_deref().unwrap_or("-");
            let model = comp.properties.bound_model.as_deref().unwrap_or("-");
            let cols = comp.properties.description.as_deref().unwrap_or("-");
            let cols_short: String = cols.chars().take(40).collect();
            content.push_str(&format!("| {} | {} | {} | {} | `{}` |\n", comp.properties.name, comp_type, model, cols_short, comp.properties.file_path));
        }
        content.push('\n');
        content.push_str(&nav_footer(&page_order, "ui-components"));
        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    // ─── AJAX Endpoints page ───────────────────────────────────────────
    if !ajax_calls.is_empty() {
        let out_path = modules_dir.join("ajax-endpoints.md");
        let ajax_files: Vec<String> = ajax_calls.iter().map(|c| c.properties.file_path.clone()).collect::<BTreeSet<String>>().into_iter().collect();
        let ajax_file_refs: Vec<&str> = ajax_files.iter().take(15).map(|s| s.as_str()).collect();
        let mut content = String::from("# AJAX Endpoints\n\n");
        content.push_str(&source_files_section(&ajax_file_refs));
        content.push_str(&format!("**Total AJAX calls:** {}\n\n", ajax_calls.len()));
        content.push_str("| Method | URL | File | Line |\n|--------|-----|------|------|\n");
        for call in ajax_calls.iter().take(100) {
            let method = call.properties.ajax_method.as_deref().unwrap_or("GET");
            let url = call.properties.ajax_url.as_deref().unwrap_or("-");
            let line = call.properties.start_line.map(|l| l.to_string()).unwrap_or_default();
            content.push_str(&format!("| {} | {} | `{}` | {} |\n", method, url, call.properties.file_path, line));
        }
        content.push('\n');
        content.push_str(&nav_footer(&page_order, "ajax-endpoints"));
        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    // ─── External Services page ────────────────────────────────────────
    if !ext_services.is_empty() {
        let out_path = modules_dir.join("external-services.md");
        let ext_files: Vec<String> = ext_services.iter().map(|s| s.properties.file_path.clone()).collect::<BTreeSet<String>>().into_iter().collect();
        let mut calling_files: BTreeSet<String> = BTreeSet::new();
        for svc in &ext_services { for r in graph.iter_relationships() { if r.rel_type == RelationshipType::CallsService && r.target_id == svc.id { if let Some(src) = graph.get_node(&r.source_id) { if !src.properties.file_path.is_empty() { calling_files.insert(src.properties.file_path.clone()); } } } } }
        let mut all_src_files: Vec<String> = ext_files.iter().cloned().chain(calling_files.iter().cloned()).collect::<BTreeSet<String>>().into_iter().collect();
        all_src_files.truncate(15);
        let src_file_refs: Vec<&str> = all_src_files.iter().map(|s| s.as_str()).collect();

        let mut content = String::from("# External Services & Integrations\n\n");
        content.push_str(&source_files_section(&src_file_refs));
        content.push_str(&format!("> This project integrates with {} external services via WebAPI (REST) and WCF (SOAP).\n\n", ext_services.len()));

        let webapi_services: Vec<&&GraphNode> = ext_services.iter().filter(|s| { let st = s.properties.service_type.as_deref().unwrap_or("").to_lowercase(); st == "webapi" || st == "rest" || st == "http" }).collect();
        let wcf_services: Vec<&&GraphNode> = ext_services.iter().filter(|s| { let st = s.properties.service_type.as_deref().unwrap_or("").to_lowercase(); st == "wcf" || st == "soap" }).collect();
        let other_services: Vec<&&GraphNode> = ext_services.iter().filter(|s| { let st = s.properties.service_type.as_deref().unwrap_or("").to_lowercase(); !["webapi","rest","http","wcf","soap"].contains(&st.as_str()) }).collect();

        let find_callers = |svc: &GraphNode| -> Vec<String> { graph.iter_relationships().filter(|r| r.rel_type == RelationshipType::CallsService && r.target_id == svc.id).filter_map(|r| graph.get_node(&r.source_id).map(|n| n.properties.name.clone())).collect::<BTreeSet<String>>().into_iter().collect() };
        let _find_methods = |svc: &GraphNode| -> Vec<&GraphNode> { let svc_name = &svc.properties.name; graph.iter_nodes().filter(|n| n.label == NodeLabel::Method && (n.properties.file_path.contains("WebAPI") || n.properties.file_path.contains("WebApi") || n.properties.file_path.contains(svc_name)) && n.properties.name.ends_with("Async") && !n.properties.name.starts_with("PrepareRequest") && !n.properties.name.starts_with("ProcessResponse") && !n.properties.name.starts_with("ReadObject")).collect() };

        if !webapi_services.is_empty() {
            content.push_str(&format!("## WebAPI Services ({})\n\n| Client | Type | Called From | Purpose |\n|--------|------|------------|--------|\n", webapi_services.len()));
            for svc in &webapi_services {
                let stype = svc.properties.service_type.as_deref().unwrap_or("WebAPI");
                let callers = find_callers(svc); let called_from = if callers.is_empty() { "-".to_string() } else { callers.join(", ") };
                let purpose = svc.properties.description.as_deref().unwrap_or("-");
                content.push_str(&format!("| {} | {} | {} | {} |\n", svc.properties.name, stype, called_from, purpose));
            }
            content.push('\n');

            let all_api_methods: Vec<&GraphNode> = graph.iter_nodes().filter(|n| n.label == NodeLabel::Method && (n.properties.file_path.contains("WebAPI") || n.properties.file_path.contains("WebApi")) && !n.properties.file_path.contains("Tests") && n.properties.name.ends_with("Async") && !n.properties.name.starts_with("PrepareRequest") && !n.properties.name.starts_with("ProcessResponse") && !n.properties.name.starts_with("ReadObject")).collect();
            if !all_api_methods.is_empty() {
                content.push_str("### API Erable — Méthodes détaillées\n\n> Point d'accès unique aux données bénéficiaires via l'API REST Erable.\n> Authentification : HTTP Basic. Toutes les méthodes sont asynchrones.\n\n");
                let mut methods_by_file: BTreeMap<String, Vec<&&GraphNode>> = BTreeMap::new();
                for m in &all_api_methods { methods_by_file.entry(m.properties.file_path.clone()).or_default().push(m); }
                for (file, methods) in &methods_by_file {
                    let file_short = file.rsplit(['/', '\\']).next().unwrap_or(file);
                    if file_short.contains("Ldap") { continue; }
                    content.push_str(&format!("**Fichier : `{}`**\n\n| Méthode | Paramètres | Retour |\n|---------|-----------|--------|\n", file_short));
                    let source_path = repo_path.join(file);
                    let source_content = std::fs::read_to_string(&source_path).unwrap_or_default();
                    for method in methods {
                        let method_name = &method.properties.name;
                        let signatures = if !source_content.is_empty() { extract_all_method_signatures(&source_content, method_name) } else { vec![("-".to_string(), "-".to_string())] };
                        for (idx, (params_str, ret_str)) in signatures.iter().enumerate() {
                            if idx == 0 { content.push_str(&format!("| **{}** | {} | `{}` |\n", method_name, params_str, ret_str)); }
                            else { content.push_str(&format!("| \u{21b3} *surcharge* | {} | `{}` |\n", params_str, ret_str)); }
                        }
                    }
                    content.push('\n');
                }
                content.push_str("**Services appelants :**\n\n");
                for svc in &webapi_services { let callers = find_callers(svc); if !callers.is_empty() { content.push_str(&format!("- **{}** \u{2190} {}\n", svc.properties.name, callers.join(", "))); } }
                content.push('\n');
            }
        }

        if !wcf_services.is_empty() {
            content.push_str(&format!("## WCF Services (SOAP) ({})\n\n| Client | Type | Called From | Purpose |\n|--------|------|------------|--------|\n", wcf_services.len()));
            for svc in &wcf_services { let stype = svc.properties.service_type.as_deref().unwrap_or("WCF"); let callers = find_callers(svc); let called_from = if callers.is_empty() { "-".to_string() } else { callers.join(", ") }; let purpose = svc.properties.description.as_deref().unwrap_or("-"); content.push_str(&format!("| {} | {} | {} | {} |\n", svc.properties.name, stype, called_from, purpose)); }
            content.push('\n');
        }

        if !other_services.is_empty() {
            content.push_str(&format!("## Other Services ({})\n\n| Client | Type | Called From | Purpose |\n|--------|------|------------|--------|\n", other_services.len()));
            for svc in &other_services { let stype = svc.properties.service_type.as_deref().unwrap_or("External"); let callers = find_callers(svc); let called_from = if callers.is_empty() { "-".to_string() } else { callers.join(", ") }; let purpose = svc.properties.description.as_deref().unwrap_or("-"); content.push_str(&format!("| {} | {} | {} | {} |\n", svc.properties.name, stype, called_from, purpose)); }
            content.push('\n');
        }

        // Service Call Flow Mermaid diagram
        let mut mermaid_edges: Vec<(String, String)> = Vec::new();
        let mut mermaid_nodes: BTreeMap<String, (String, &str)> = BTreeMap::new();
        for ext_svc in &ext_services {
            let ext_short = sanitize_mermaid_id(&ext_svc.properties.name);
            mermaid_nodes.insert(ext_short.clone(), (ext_svc.properties.name.clone(), "External"));
            for r in graph.iter_relationships() {
                if r.rel_type == RelationshipType::CallsService && r.target_id == ext_svc.id {
                    if let Some(caller) = graph.get_node(&r.source_id) {
                        if caller.properties.file_path.contains("Test") || caller.properties.file_path.contains("test") { continue; }
                        let subgraph = match caller.label { NodeLabel::Controller => "Controllers", NodeLabel::Service | NodeLabel::Repository => "Services", _ => continue };
                        let caller_short = sanitize_mermaid_id(&caller.properties.name);
                        mermaid_nodes.insert(caller_short.clone(), (caller.properties.name.clone(), subgraph));
                        mermaid_edges.push((caller_short.clone(), ext_short.clone()));
                        if caller.label == NodeLabel::Service || caller.label == NodeLabel::Repository {
                            for r2 in graph.iter_relationships() {
                                if r2.rel_type == RelationshipType::DependsOn && r2.target_id == caller.id {
                                    if let Some(ctrl) = graph.get_node(&r2.source_id) {
                                        if ctrl.label == NodeLabel::Controller { let ctrl_short = sanitize_mermaid_id(&ctrl.properties.name); mermaid_nodes.insert(ctrl_short.clone(), (ctrl.properties.name.clone(), "Controllers")); mermaid_edges.push((ctrl_short, caller_short.clone())); }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if !mermaid_edges.is_empty() {
            content.push_str("## Service Call Flow\n\n```mermaid\ngraph LR\n");
            let mut subgraphs: BTreeMap<&str, Vec<(String, String)>> = BTreeMap::new();
            for (id, (label, sg)) in &mermaid_nodes { subgraphs.entry(sg).or_default().push((id.clone(), label.clone())); }
            for (sg_name, nodes) in &subgraphs {
                content.push_str(&format!("    subgraph {}[\"{}\"]\n", sanitize_mermaid_id(sg_name), sg_name));
                for (id, label) in nodes { content.push_str(&format!("        {}[\"{}\"]\n", id, label)); }
                content.push_str("    end\n");
            }
            let unique_edges: BTreeSet<(String, String)> = mermaid_edges.into_iter().collect();
            for (from, to) in &unique_edges { content.push_str(&format!("    {} --> {}\n", from, to)); }
            content.push_str("```\n\n");
        }

        content.push_str(&nav_footer(&page_order, "external-services"));
        std::fs::write(&out_path, &content)?;
        println!("  {} {}", "OK".green(), out_path.display());
        page_count += 1;
    }

    Ok(page_count)
}
