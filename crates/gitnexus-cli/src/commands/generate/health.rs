//! Project health dashboard generator.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use colored::Colorize;

use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;

use super::utils::*;

pub(super) fn generate_project_health(docs_dir: &Path, graph: &KnowledgeGraph) -> Result<()> {
    let out_path = docs_dir.join("project-health.md");
    let mut f = std::fs::File::create(&out_path)?;

    let label_counts = count_nodes_by_label(graph);
    let node_count = graph.iter_nodes().count();
    let edge_count = graph.iter_relationships().count();
    let density = if node_count > 0 {
        edge_count as f64 / node_count as f64
    } else {
        0.0
    };
    let density_interp = if density > 3.0 {
        "Fortement couplé"
    } else if density > 2.0 {
        "Couplage modéré"
    } else if density > 1.0 {
        "Couplage normal"
    } else {
        "Faiblement couplé"
    };

    // StackLogger tracing coverage
    let total_files = graph
        .iter_nodes()
        .filter(|n| n.label == NodeLabel::File)
        .count();
    // Restrict the numerator to File nodes — `is_traced` is also set on
    // Method nodes by `extract_tracing_info`, so without this filter the
    // count would mix methods + files and the displayed percentage could
    // exceed 100%.
    let traced_files = graph
        .iter_nodes()
        .filter(|n| n.label == NodeLabel::File && n.properties.is_traced == Some(true))
        .count();
    let traced_pct = if total_files > 0 {
        (traced_files as f64 / total_files as f64) * 100.0
    } else {
        0.0
    };

    // Dead code detection
    let total_methods = graph
        .iter_nodes()
        .filter(|n| matches!(n.label, NodeLabel::Method | NodeLabel::Function))
        .count();
    let dead_methods = graph
        .iter_nodes()
        .filter(|n| n.properties.is_dead_candidate == Some(true))
        .count();
    let dead_pct = if total_methods > 0 {
        (dead_methods as f64 / total_methods as f64) * 100.0
    } else {
        0.0
    };

    let ext_count = label_counts
        .get(&NodeLabel::ExternalService)
        .copied()
        .unwrap_or(0);

    // Key symbol counts
    let fn_count = label_counts.get(&NodeLabel::Function).copied().unwrap_or(0);
    let class_count = label_counts.get(&NodeLabel::Class).copied().unwrap_or(0);
    let method_count = label_counts.get(&NodeLabel::Method).copied().unwrap_or(0);
    let ctrl_count = label_counts
        .get(&NodeLabel::Controller)
        .copied()
        .unwrap_or(0);
    let action_count = label_counts
        .get(&NodeLabel::ControllerAction)
        .copied()
        .unwrap_or(0);
    let svc_count = label_counts.get(&NodeLabel::Service).copied().unwrap_or(0)
        + label_counts
            .get(&NodeLabel::Repository)
            .copied()
            .unwrap_or(0);

    writeln!(f, "# Santé du Projet")?;
    writeln!(f, "<!-- GNX:LEAD -->")?;
    writeln!(f)?;
    writeln!(f, "> Vue d'ensemble de la santé structurelle du codebase, ")?;
    writeln!(
        f,
        "> générée automatiquement par l'analyse du graphe de connaissances GitNexus."
    )?;
    writeln!(f)?;

    // ── Key Metrics Table ──
    writeln!(f, "## Métriques Clés")?;
    writeln!(f, "<!-- GNX:INTRO:metriques-cles -->")?;
    writeln!(f)?;
    writeln!(f, "| Indicateur | Valeur | Interprétation |")?;
    writeln!(f, "|-----------|--------|----------------|")?;
    writeln!(f, "| Symboles | {} | Volume de code analysé |", node_count)?;
    writeln!(
        f,
        "| Relations | {} | Couplage entre composants |",
        edge_count
    )?;
    writeln!(f, "| Densité | {:.1} | {} |", density, density_interp)?;
    writeln!(
        f,
        "| Couverture traçabilité | {:.0}% ({}/{} fichiers) | Fichiers avec StackLogger |",
        traced_pct, traced_files, total_files
    )?;
    if dead_methods > 0 {
        writeln!(
            f,
            "| Code mort potentiel | {:.0}% ({} méthodes) | Méthodes sans appelants |",
            dead_pct, dead_methods
        )?;
    }
    writeln!(
        f,
        "| Services externes | {} | Points d'intégration |",
        ext_count
    )?;

    // Test file coverage
    let test_files = graph
        .iter_nodes()
        .filter(|n| {
            n.label == NodeLabel::File
                && (n.properties.file_path.contains("Test")
                    || n.properties.file_path.ends_with(".test.cs")
                    || n.properties.file_path.ends_with("_test.cs")
                    || n.properties.file_path.ends_with("_tests.cs"))
        })
        .count();
    let test_ratio = if total_files > 0 {
        (test_files as f64 / total_files as f64 * 100.0) as u32
    } else {
        0
    };
    writeln!(
        f,
        "| Tests | {} fichiers ({} %) | Ratio couverture test |",
        test_files, test_ratio
    )?;

    // LLM smells
    let smelly_nodes: Vec<_> = graph
        .iter_nodes()
        .filter(|n| {
            n.properties
                .llm_smells
                .as_ref()
                .map(|s| !s.is_empty())
                .unwrap_or(false)
        })
        .collect();
    if !smelly_nodes.is_empty() {
        let risk_sum: u32 = smelly_nodes
            .iter()
            .filter_map(|n| n.properties.llm_risk_score)
            .sum();
        let avg_risk = risk_sum / smelly_nodes.len() as u32;
        writeln!(
            f,
            "| Code Smells LLM | {} symboles | Risque moyen : {} / 100 |",
            smelly_nodes.len(),
            avg_risk
        )?;
    }
    writeln!(f)?;

    // ── Symbol breakdown ──
    writeln!(f, "## Répartition par type de symbole")?;
    writeln!(f)?;
    writeln!(f, "| Type | Nombre |")?;
    writeln!(f, "|------|--------|")?;
    if fn_count > 0 {
        writeln!(f, "| Functions | {} |", fn_count)?;
    }
    if class_count > 0 {
        writeln!(f, "| Classes | {} |", class_count)?;
    }
    if method_count > 0 {
        writeln!(f, "| Methods | {} |", method_count)?;
    }
    if ctrl_count > 0 {
        writeln!(f, "| Controllers | {} |", ctrl_count)?;
    }
    if action_count > 0 {
        writeln!(f, "| Controller Actions | {} |", action_count)?;
    }
    if svc_count > 0 {
        writeln!(f, "| Services/Repositories | {} |", svc_count)?;
    }
    // Show remaining non-zero labels
    let shown_labels: HashSet<NodeLabel> = [
        NodeLabel::Function,
        NodeLabel::Class,
        NodeLabel::Method,
        NodeLabel::Controller,
        NodeLabel::ControllerAction,
        NodeLabel::Service,
        NodeLabel::Repository,
        NodeLabel::ExternalService,
        NodeLabel::File,
        NodeLabel::Community,
    ]
    .into_iter()
    .collect();
    let mut other_labels: Vec<_> = label_counts
        .iter()
        .filter(|(label, count)| !shown_labels.contains(label) && **count > 0)
        .collect();
    other_labels.sort_by(|a, b| b.1.cmp(a.1));
    for (label, count) in other_labels.iter().take(10) {
        writeln!(f, "| {} | {} |", label.as_str(), count)?;
    }
    writeln!(f)?;

    // ── Top 10 Most Connected Nodes ──
    writeln!(f, "## Top 10 — Composants les plus connectés")?;
    writeln!(f, "<!-- GNX:INTRO:top-connected -->")?;
    writeln!(f)?;
    writeln!(
        f,
        "> Ces composants ont le plus de dépendances. Un changement dans l'un d'eux"
    )?;
    writeln!(f, "> a un impact potentiel large sur le reste du système.")?;
    writeln!(f)?;

    // Compute degree for each node
    let mut node_degree: HashMap<String, usize> = HashMap::new();
    for rel in graph.iter_relationships() {
        *node_degree.entry(rel.source_id.clone()).or_insert(0) += 1;
        *node_degree.entry(rel.target_id.clone()).or_insert(0) += 1;
    }
    let mut sorted_degree: Vec<_> = node_degree.into_iter().collect();
    sorted_degree.sort_by(|a, b| b.1.cmp(&a.1));

    writeln!(f, "| Composant | Type | Connexions | Fichier |")?;
    writeln!(f, "|-----------|------|-----------|---------|")?;
    for (node_id, degree) in sorted_degree.iter().take(10) {
        if let Some(node) = graph.get_node(node_id) {
            writeln!(
                f,
                "| `{}` | {} | {} | `{}` |",
                node.properties.name,
                node.label.as_str(),
                degree,
                node.properties.file_path
            )?;
        }
    }
    writeln!(f)?;

    // ── Top 10 Largest Files ──
    writeln!(f, "## Top 10 — Fichiers les plus volumineux")?;
    writeln!(f)?;

    // Count symbols per file, and track the dominant label
    let mut file_stats: HashMap<String, (usize, HashMap<NodeLabel, usize>)> = HashMap::new();
    for node in graph.iter_nodes() {
        if !node.properties.file_path.is_empty() && node.label != NodeLabel::File {
            let entry = file_stats
                .entry(node.properties.file_path.clone())
                .or_insert_with(|| (0, HashMap::new()));
            entry.0 += 1;
            *entry.1.entry(node.label).or_insert(0) += 1;
        }
    }
    let mut sorted_files: Vec<_> = file_stats.into_iter().collect();
    sorted_files.sort_by(|a, b| (b.1).0.cmp(&(a.1).0));

    writeln!(f, "| Fichier | Symboles | Type principal |")?;
    writeln!(f, "|---------|----------|---------------|")?;
    for (file_path, (sym_count, label_map)) in sorted_files.iter().take(10) {
        let dominant = label_map
            .iter()
            .max_by_key(|(_, c)| *c)
            .map(|(l, _)| l.as_str())
            .unwrap_or("-");
        writeln!(f, "| `{}` | {} | {} |", file_path, sym_count, dominant)?;
    }
    writeln!(f)?;

    // ── External Services ──
    if ext_count > 0 {
        writeln!(f, "## Services Externes")?;
        writeln!(f)?;
        writeln!(f, "| Service | Fichier |")?;
        writeln!(f, "|---------|---------|")?;
        for node in graph.iter_nodes() {
            if node.label == NodeLabel::ExternalService {
                writeln!(
                    f,
                    "| `{}` | `{}` |",
                    node.properties.name, node.properties.file_path
                )?;
            }
        }
        writeln!(f)?;
    }

    // ── Most Complex Functions ──
    {
        let mut complex_fns: Vec<(&str, &str, u32)> = graph
            .iter_nodes()
            .filter(|n| {
                matches!(
                    n.label,
                    NodeLabel::Method | NodeLabel::Function | NodeLabel::Constructor
                )
            })
            .filter_map(|n| {
                n.properties.complexity.map(|cc| {
                    (
                        n.properties.name.as_str(),
                        n.properties.file_path.as_str(),
                        cc,
                    )
                })
            })
            .collect();
        complex_fns.sort_by(|a, b| b.2.cmp(&a.2));

        if !complex_fns.is_empty() && complex_fns[0].2 > 1 {
            writeln!(f, "## Fonctions les plus complexes")?;
            writeln!(f)?;
            writeln!(f, "> Complexité cyclomatique (CC) : 1 = linéaire, >10 = complexe, >20 = très complexe.")?;
            writeln!(f)?;

            writeln!(f, "| Fonction | Fichier | CC |")?;
            writeln!(f, "|----------|---------|-----|")?;
            for (name, file_path, cc) in complex_fns.iter().take(15) {
                writeln!(f, "| `{}` | `{}` | {} |", name, file_path, cc)?;
            }
            writeln!(f)?;
        }
    }

    // ── Dead Code Candidates ──
    if dead_methods > 0 {
        writeln!(f, "## Code mort potentiel")?;
        writeln!(f)?;
        writeln!(
            f,
            "> **{} méthodes** ({:.1}%) n'ont aucun appelant détecté dans le code source.",
            dead_methods, dead_pct
        )?;
        writeln!(
            f,
            "> Ces méthodes sont potentiellement inutilisées et candidates à la suppression."
        )?;
        writeln!(f, "> Les constructeurs, méthodes de test, entry points ASP.NET, et méthodes d'interface sont exclus.")?;
        writeln!(f)?;

        // Group dead methods by file
        let mut dead_by_file: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        for node in graph.iter_nodes() {
            if node.properties.is_dead_candidate == Some(true) {
                dead_by_file
                    .entry(node.properties.file_path.clone())
                    .or_default()
                    .push(node.properties.name.clone());
            }
        }

        // Show top 15 files with most dead code
        let mut files_sorted: Vec<_> = dead_by_file.iter().collect();
        files_sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        writeln!(f, "| Fichier | Méthodes mortes | Exemples |")?;
        writeln!(f, "|---------|----------------|----------|")?;
        for (file, methods) in files_sorted.iter().take(15) {
            let examples: Vec<&str> = methods.iter().take(3).map(|s| s.as_str()).collect();
            writeln!(
                f,
                "| `{}` | {} | {} |",
                file,
                methods.len(),
                examples.join(", ")
            )?;
        }
        writeln!(f)?;
    }

    // ── Architecture Analysis ──
    let arch_result = gitnexus_ingest::phases::architecture::analyze_architecture(graph);

    if !arch_result.circular_deps.is_empty() {
        writeln!(f, "## Dépendances circulaires")?;
        writeln!(f)?;
        writeln!(f, "> [!WARNING]")?;
        writeln!(
            f,
            "> **{} cycle(s)** détecté(s) dans les imports entre fichiers.",
            arch_result.circular_deps.len()
        )?;
        writeln!(f)?;
        for (i, cycle) in arch_result.circular_deps.iter().enumerate().take(10) {
            writeln!(f, "**Cycle {}:** `{}`", i + 1, cycle.cycle.join("` → `"))?;
            writeln!(f)?;
        }
    }

    if !arch_result.layer_violations.is_empty() {
        writeln!(f, "## Violations de couche architecturale")?;
        writeln!(f)?;
        writeln!(f, "> [!DANGER]")?;
        writeln!(
            f,
            "> **{} violation(s)** : couche présentation accède directement à la couche données.",
            arch_result.layer_violations.len()
        )?;
        writeln!(f)?;
        writeln!(f, "| Source | Couche | Cible | Couche |")?;
        writeln!(f, "|--------|--------|-------|--------|")?;
        for v in arch_result.layer_violations.iter().take(20) {
            writeln!(
                f,
                "| `{}` | {} | `{}` | {} |",
                v.source_name, v.source_layer, v.target_name, v.target_layer
            )?;
        }
        writeln!(f)?;
    }

    // ── Technical Debt: TodoMarker nodes ──
    let todos: Vec<_> = graph
        .iter_nodes()
        .filter(|n| n.label == NodeLabel::TodoMarker)
        .collect();
    if !todos.is_empty() {
        writeln!(f, "\n## Marqueurs de dette technique")?;
        writeln!(f, "<!-- GNX:INTRO:dette-technique -->")?;
        writeln!(f)?;
        let mut by_kind: std::collections::HashMap<&str, Vec<_>> = Default::default();
        for t in &todos {
            by_kind
                .entry(t.properties.todo_kind.as_deref().unwrap_or("TODO"))
                .or_default()
                .push(t);
        }
        for kind in &["FIXME", "HACK", "TODO", "XXX"] {
            if let Some(items) = by_kind.get(kind) {
                writeln!(f, "### {} ({})", kind, items.len())?;
                writeln!(f, "| Fichier | Ligne | Note |")?;
                writeln!(f, "|---------|-------|------|")?;
                for item in items.iter().take(20) {
                    let file = item
                        .properties
                        .file_path
                        .split(['/', '\\'])
                        .next_back()
                        .unwrap_or("");
                    let line = item
                        .properties
                        .start_line
                        .map(|l| l.to_string())
                        .unwrap_or_else(|| "-".to_string());
                    let text = item.properties.todo_text.as_deref().unwrap_or("-");
                    writeln!(f, "| `{}` | {} | {} |", file, line, text)?;
                }
                if items.len() > 20 {
                    writeln!(f, "| … | | *+{} autres* |", items.len() - 20)?;
                }
                writeln!(f)?;
            }
        }
    }

    writeln!(f, "<!-- GNX:CLOSING -->")?;
    writeln!(f, "---")?;
    writeln!(
        f,
        "**Voir aussi :** [Vue d'ensemble](./overview.md) · [Architecture](./architecture.md)"
    )?;

    println!("  {} project-health.md", "OK".green());
    Ok(())
}
