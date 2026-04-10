//! Obsidian Vault generator.
//! Produces a Markdown-based knowledge base compatible with Obsidian.md.

use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use colored::Colorize;

use gitnexus_core::graph::types::{NodeLabel, RelationshipType, GraphNode};
use gitnexus_core::graph::KnowledgeGraph;

/// Context for community information
pub struct CommunityInfo {
    pub label: String,
    pub description: Option<String>,
    pub member_ids: Vec<String>,
}

pub fn generate_obsidian_vault(
    graph: &KnowledgeGraph,
    output_dir: &Path,
    communities: &BTreeMap<String, CommunityInfo>,
) -> Result<String> {
    let vault_dir = output_dir.join("obsidian_vault");
    if vault_dir.exists() {
        std::fs::remove_dir_all(&vault_dir)?;
    }
    std::fs::create_dir_all(&vault_dir)?;

    let nodes_dir = vault_dir.join("Symboles");
    let comms_dir = vault_dir.join("Modules");
    let proc_dir = vault_dir.join("Processus");
    let files_dir = vault_dir.join("Fichiers");

    std::fs::create_dir_all(&nodes_dir)?;
    std::fs::create_dir_all(&comms_dir)?;
    std::fs::create_dir_all(&proc_dir)?;
    std::fs::create_dir_all(&files_dir)?;

    let mut edge_map: HashMap<String, Vec<(String, RelationshipType)>> = HashMap::new();
    for rel in graph.iter_relationships() {
        edge_map.entry(rel.source_id.clone()).or_default().push((rel.target_id.clone(), rel.rel_type));
    }

    // 1. Generate Index.md
    generate_index(&vault_dir, graph, communities)?;

    // 1b. Generate Log.md
    generate_log(&vault_dir, graph)?;

    // 1c. Generate .claudecode
    generate_claudecode_instructions(&vault_dir)?;

    // 2. Generate File pages
    for node in graph.iter_nodes().filter(|n| n.label == NodeLabel::File) {
        generate_file_page(&files_dir, node, graph, &edge_map)?;
    }

    // 3. Generate Community pages
    for (id, info) in communities {
        generate_community_page(&comms_dir, id, info, graph, &edge_map)?;
    }

    // 4. Generate Node pages
    for node in graph.iter_nodes().filter(|n| n.label != NodeLabel::File && n.label != NodeLabel::Community && n.label != NodeLabel::Process && n.label != NodeLabel::Document && n.label != NodeLabel::DocChunk) {
        generate_node_page(&nodes_dir, node, graph, &edge_map)?;
    }

    // 5. Generate Process pages
    for node in graph.iter_nodes().filter(|n| n.label == NodeLabel::Process) {
        generate_process_page(&proc_dir, node, graph, &edge_map)?;
    }

    Ok(vault_dir.to_string_lossy().to_string())
}

fn generate_index(dir: &Path, graph: &KnowledgeGraph, communities: &BTreeMap<String, CommunityInfo>) -> Result<()> {
    let out_path = dir.join("Index.md");
    let mut f = std::fs::File::create(&out_path)?;

    let project_name = dir.parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Project".to_string());

    writeln!(f, "# 🧠 Cerveau Numérique - {}", project_name)?;
    writeln!(f)?;
    writeln!(f, "> Ce coffre-fort Obsidian contient une cartographie complète du code source, générée par GitNexus.")?;
    writeln!(f)?;
    
    writeln!(f, "## 📊 Statistiques")?;
    
    let mut counts = HashMap::new();
    for node in graph.iter_nodes() {
        *counts.entry(node.label).or_insert(0) += 1;
    }

    writeln!(f, "- **Fichiers** : {}", counts.get(&NodeLabel::File).unwrap_or(&0))?;
    writeln!(f, "- **Classes** : {}", counts.get(&NodeLabel::Class).unwrap_or(&0))?;
    writeln!(f, "- **Fonctions/Méthodes** : {}", counts.get(&NodeLabel::Function).unwrap_or(&0))?;
    writeln!(f, "- **Modules Fonctionnels** : {}", communities.len())?;
    writeln!(f)?;

    writeln!(f, "## 🧱 Modules Principaux")?;
    for info in communities.values() {
        writeln!(f, "- [[Modules/{}|{}]]", sanitize_obsidian_filename(&info.label), info.label)?;
    }
    writeln!(f)?;

    writeln!(f, "## ⚙️ Processus Métier")?;
    for node in graph.iter_nodes().filter(|n| n.label == NodeLabel::Process) {
        writeln!(f, "- [[Processus/{}|{}]]", sanitize_obsidian_filename(&node.properties.name), node.properties.name)?;
    }
    writeln!(f)?;

    writeln!(f, "## 🔍 Vérification du Cerveau (Lint)")?;
    writeln!(f, "> État de santé de la base de connaissances.")?;
    writeln!(f)?;
    
    let mut edge_map: HashMap<String, Vec<(String, RelationshipType)>> = HashMap::new();
    let mut incoming_count: HashMap<String, usize> = HashMap::new();
    for rel in graph.iter_relationships() {
        edge_map.entry(rel.source_id.clone()).or_default().push((rel.target_id.clone(), rel.rel_type));
        *incoming_count.entry(rel.target_id.clone()).or_insert(0) += 1;
    }

    let orphans: Vec<_> = graph.iter_nodes()
        .filter(|n| n.label != NodeLabel::File && n.label != NodeLabel::Community && n.label != NodeLabel::Document && n.label != NodeLabel::DocChunk)
        .filter(|n| !edge_map.contains_key(&n.id) && !incoming_count.contains_key(&n.id))
        .take(5)
        .collect();

    if orphans.is_empty() {
        writeln!(f, "- ✅ Aucun symbole orphelin détecté. Le graphe est bien connecté.")?;
    } else {
        writeln!(f, "- ⚠️ {} symboles isolés détectés (ex: [[Symboles/{}/{}|{}]])", 
            orphans.len(), 
            orphans[0].label.as_str(),
            sanitize_obsidian_filename(&orphans[0].properties.name),
            orphans[0].properties.name
        )?;
    }

    writeln!(f)?;
    writeln!(f, "## 📑 Journal")?;
    writeln!(f, "- [[Log|Consulter le journal d'ingestion]]")?;
    writeln!(f)?;

    writeln!(f, "## 🤖 IA & Chat")?;
    writeln!(f, "Vous pouvez interroger ce cerveau numérique :")?;
    writeln!(f, "1. **Via CLI** : Lancez `gitnexus ask \"votre question\"`.")?;
    writeln!(f, "2. **Via Obsidian** : Utilisez les extensions *Claude Code* ou *Smart Connections* sur ce dossier.")?;
    writeln!(f, "3. **Via le Wiki HTML** : Lancez `gitnexus serve` et utilisez le widget de chat intégré.")?;

    Ok(())
}

fn generate_log(dir: &Path, graph: &KnowledgeGraph) -> Result<()> {
    let out_path = dir.join("Log.md");
    let mut f = std::fs::File::create(&out_path)?;

    writeln!(f, "# 📑 Journal d'Ingestion")?;
    writeln!(f)?;
    writeln!(f, "| Date | Événement | Détails |")?;
    writeln!(f, "|------|-----------|---------|")?;
    
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    writeln!(f, "| {} | Génération initiale | Export complet du graphe GitNexus |", now)?;
    
    let files: Vec<_> = graph.iter_nodes()
        .filter(|n| n.label == NodeLabel::File)
        .take(10)
        .collect();
        
    for node in files {
        writeln!(f, "| {} | Indexation | Fichier [[Fichiers/{}|{}]] traité |", now, sanitize_obsidian_filename(&node.properties.name), node.properties.name)?;
    }

    Ok(())
}

fn generate_claudecode_instructions(dir: &Path) -> Result<()> {
    let out_path = dir.join(".claudecode");
    let mut f = std::fs::File::create(&out_path)?;

    writeln!(f, "You are an AI expert assistant working inside an Obsidian Vault generated by GitNexus.")?;
    writeln!(f, "This vault represents the 'Digital Brain' of a software project.")?;
    writeln!(f)?;
    writeln!(f, "Structure:")?;
    writeln!(f, "- Modules/: High-level functional areas")?;
    writeln!(f, "- Processus/: Business processes and workflows")?;
    writeln!(f, "- Symboles/: Classes, Functions, Methods organized by type")?;
    writeln!(f, "- Fichiers/: Source code files mapping")?;
    writeln!(f)?;
    writeln!(f, "Guidelines:")?;
    writeln!(f, "1. Always check 'Index.md' first to understand the project structure.")?;
    writeln!(f, "2. Use wikilinks [[Path/To/Note]] to navigate between entities.")?;
    writeln!(f, "3. When asked about a feature, look into 'Modules/' or 'Processus/'.")?;
    writeln!(f, "4. When asked about implementation, look into 'Symboles/'.")?;
    writeln!(f, "5. If you find a broken link or orphan info, notify the user so they can run 'gitnexus analyze'.")?;

    Ok(())
}

fn generate_node_page(dir: &Path, node: &GraphNode, graph: &KnowledgeGraph, edge_map: &HashMap<String, Vec<(String, RelationshipType)>>) -> Result<()> {
    let label_dir = dir.join(node.label.as_str());
    if !label_dir.exists() {
        std::fs::create_dir_all(&label_dir)?;
    }

    let filename = format!("{}.md", sanitize_obsidian_filename(&node.properties.name));
    let out_path = label_dir.join(&filename);
    let mut f = std::fs::File::create(&out_path)?;

    // Frontmatter
    writeln!(f, "---")?;
    writeln!(f, "type: {:?}", node.label)?;
    writeln!(f, "name: \"{}\"", node.properties.name)?;
    writeln!(f, "file: \"{}\"", node.properties.file_path)?;
    if let Some(s) = node.properties.start_line { writeln!(f, "start_line: {s}")?; }
    writeln!(f, "---")?;
    writeln!(f)?;

    writeln!(f, "# {}", node.properties.name)?;
    writeln!(f)?;
    writeln!(f, "Type: **{:?}**", node.label)?;
    writeln!(f, "Fichier: [[Fichiers/{}|{}]]", sanitize_obsidian_filename(&node.properties.file_path), node.properties.file_path)?;
    writeln!(f)?;

    // Outgoing
    if let Some(edges) = edge_map.get(&node.id) {
        writeln!(f, "## 🔗 Dépendances (Sortant)")?;
        for (target_id, rel) in edges {
            if let Some(target) = graph.get_node(target_id) {
                let rel_str = format!("{:?}", rel);
                let folder = if target.label == NodeLabel::File { "Fichiers" } 
                            else if target.label == NodeLabel::Community { "Modules" }
                            else { "Symboles" };
                
                let subfolder = if folder == "Symboles" { format!("{}/", target.label.as_str()) } else { "".to_string() };

                writeln!(f, "- {}: [[{}/{}{}|{}]]", 
                    rel_str, 
                    folder, 
                    subfolder,
                    sanitize_obsidian_filename(&target.properties.name), 
                    target.properties.name
                )?;
            }
        }
        writeln!(f)?;
    }

    // Incoming (simple scan)
    writeln!(f, "## 📥 Utilisé par (Entrant)")?;
    for (src_id, edges) in edge_map {
        for (target_id, rel) in edges {
            if target_id == &node.id {
                if let Some(src) = graph.get_node(src_id) {
                    let folder = if src.label == NodeLabel::File { "Fichiers" } 
                                else if src.label == NodeLabel::Community { "Modules" }
                                else { "Symboles" };
                    let subfolder = if folder == "Symboles" { format!("{}/", src.label.as_str()) } else { "".to_string() };

                    writeln!(f, "- [[{}/{}{}|{}]] ({:?})", 
                        folder, 
                        subfolder,
                        sanitize_obsidian_filename(&src.properties.name), 
                        src.properties.name,
                        rel
                    )?;
                }
            }
        }
    }

    Ok(())
}

fn generate_file_page(dir: &Path, node: &GraphNode, graph: &KnowledgeGraph, edge_map: &HashMap<String, Vec<(String, RelationshipType)>>) -> Result<()> {
    let filename = format!("{}.md", sanitize_obsidian_filename(&node.properties.name));
    let out_path = dir.join(&filename);
    let mut f = std::fs::File::create(&out_path)?;

    writeln!(f, "---")?;
    writeln!(f, "type: File")?;
    writeln!(f, "path: \"{}\"", node.properties.file_path)?;
    if let Some(l) = &node.properties.language { writeln!(f, "language: {:?}", l)?; }
    writeln!(f, "---")?;
    writeln!(f)?;

    writeln!(f, "# Fichier: {}", node.properties.name)?;
    writeln!(f)?;

    writeln!(f, "## 🧩 Contenu")?;
    if let Some(edges) = edge_map.get(&node.id) {
        for (target_id, rel) in edges {
            if *rel == RelationshipType::Defines {
                if let Some(target) = graph.get_node(target_id) {
                    writeln!(f, "- [[Symboles/{}/{}|{}]]", target.label.as_str(), sanitize_obsidian_filename(&target.properties.name), target.properties.name)?;
                }
            }
        }
    }

    Ok(())
}

fn generate_community_page(dir: &Path, _id: &str, info: &CommunityInfo, graph: &KnowledgeGraph, _edge_map: &HashMap<String, Vec<(String, RelationshipType)>>) -> Result<()> {
    let filename = format!("{}.md", sanitize_obsidian_filename(&info.label));
    let out_path = dir.join(&filename);
    let mut f = std::fs::File::create(&out_path)?;

    writeln!(f, "---")?;
    writeln!(f, "type: Module")?;
    writeln!(f, "---")?;
    writeln!(f)?;

    writeln!(f, "# Module: {}", info.label)?;
    writeln!(f)?;
    if let Some(d) = &info.description {
        writeln!(f, "{d}")?;
        writeln!(f)?;
    }

    writeln!(f, "## 👥 Membres")?;
    for mid in &info.member_ids {
        if let Some(m) = graph.get_node(mid) {
            let folder = if m.label == NodeLabel::File { "Fichiers" } else { "Symboles" };
            let subfolder = if folder == "Symboles" { format!("{}/", m.label.as_str()) } else { "".to_string() };
            writeln!(f, "- [[{}/{}{}|{}]]", folder, subfolder, sanitize_obsidian_filename(&m.properties.name), m.properties.name)?;
        }
    }

    Ok(())
}

fn generate_process_page(dir: &Path, node: &GraphNode, _graph: &KnowledgeGraph, _edge_map: &HashMap<String, Vec<(String, RelationshipType)>>) -> Result<()> {
    let filename = format!("{}.md", sanitize_obsidian_filename(&node.properties.name));
    let out_path = dir.join(&filename);
    let mut f = std::fs::File::create(&out_path)?;

    writeln!(f, "---")?;
    writeln!(f, "type: Process")?;
    writeln!(f, "---")?;
    writeln!(f)?;

    writeln!(f, "# Processus: {}", node.properties.name)?;
    writeln!(f)?;

    if let Some(desc) = &node.properties.description {
        writeln!(f, "{desc}")?;
        writeln!(f)?;
    }

    // Sequence/Flow
    if let Some(entry_id) = &node.properties.entry_point_id {
        writeln!(f, "Point d'entrée: [[Symboles/Function/{}|{}]]", sanitize_obsidian_filename(entry_id), entry_id)?;
    }

    Ok(())
}

fn sanitize_obsidian_filename(name: &str) -> String {
    name.replace('/', "_")
        .replace('\\', "_")
        .replace(':', "_")
        .replace('*', "_")
        .replace('?', "_")
        .replace('"', "_")
        .replace('<', "_")
        .replace('>', "_")
        .replace('|', "_")
        .replace('#', "_")
        .replace('^', "_")
        .replace('[', "_")
        .replace(']', "_")
}
