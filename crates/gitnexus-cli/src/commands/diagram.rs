//! The `diagram` command: generate Mermaid diagrams from the knowledge graph.

use std::collections::{HashSet, VecDeque};
use std::io::Write;
use anyhow::Result;
use colored::Colorize;

use gitnexus_core::graph::types::NodeLabel;
use gitnexus_db::snapshot;

pub fn run(target: &str, diagram_type: &str, path: Option<&str>, output: Option<&str>) -> Result<()> {
    let repo_path = if let Some(p) = path {
        std::path::PathBuf::from(p)
    } else {
        std::env::current_dir()?
    };

    let snap_path = repo_path.join(".gitnexus").join("graph.bin");
    if !snap_path.exists() {
        println!("{} No index found. Run 'gitnexus analyze' first.", "ERROR".red());
        return Ok(());
    }

    let graph = snapshot::load_snapshot(&snap_path)
        .map_err(|e| anyhow::anyhow!("Failed to load graph: {}", e))?;

    // Find the target symbol
    let target_lower = target.to_lowercase();
    let start_node = graph.iter_nodes().find(|n| {
        n.properties.name.to_lowercase() == target_lower
    });

    let start_node = match start_node {
        Some(n) => n,
        None => {
            println!("{} Symbol '{}' not found.", "ERROR".red(), target);
            return Ok(());
        }
    };

    let mermaid = match diagram_type {
        "sequence" => generate_sequence(&graph, start_node),
        "class" => generate_class_diagram(&graph, start_node),
        _ => generate_flowchart(&graph, start_node),
    };

    if let Some(out_path) = output {
        let mut f = std::fs::File::create(out_path)?;
        writeln!(f, "```mermaid")?;
        writeln!(f, "{}", mermaid)?;
        writeln!(f, "```")?;
        println!("{} Diagram written to {}", "OK".green(), out_path);
    } else {
        println!("{}", mermaid);
    }

    Ok(())
}

fn safe_id(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

fn safe_label(name: &str) -> String {
    name.replace('"', "'").replace('`', "'")
}

fn generate_flowchart(
    graph: &gitnexus_core::graph::KnowledgeGraph,
    start: &gitnexus_core::graph::types::GraphNode,
) -> String {
    let mut lines = vec!["graph TD".to_string()];
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    visited.insert(start.id.clone());
    queue.push_back((start.id.clone(), 0usize));

    let mut edges = Vec::new();
    let mut nodes = HashSet::new();
    let max_nodes = 30;

    while let Some((node_id, depth)) = queue.pop_front() {
        if depth >= 5 || nodes.len() >= max_nodes {
            break;
        }

        for rel in graph.iter_relationships() {
            if rel.source_id != node_id {
                continue;
            }
            if visited.contains(&rel.target_id) {
                continue;
            }

            if let Some(target) = graph.get_node(&rel.target_id) {
                // Skip File and Folder nodes for cleaner diagrams
                if target.label == NodeLabel::File || target.label == NodeLabel::Folder {
                    continue;
                }

                visited.insert(rel.target_id.clone());
                nodes.insert(rel.target_id.clone());

                let src_id = safe_id(&graph.get_node(&node_id).map(|n| n.properties.name.as_str()).unwrap_or("?"));
                let tgt_id = safe_id(&target.properties.name);
                let rel_label = rel.rel_type.as_str();

                edges.push(format!(
                    "    {} -->|{}| {}",
                    src_id, rel_label, tgt_id
                ));

                queue.push_back((rel.target_id.clone(), depth + 1));
            }
        }
    }

    // Emit node declarations with labels
    let start_safe = safe_id(&start.properties.name);
    lines.push(format!(
        "    {}[\"{}\\n({})\"]",
        start_safe,
        safe_label(&start.properties.name),
        start.label.as_str()
    ));
    lines.push(format!("    style {} fill:#7aa2f7,color:#fff", start_safe));

    for nid in &nodes {
        if let Some(node) = graph.get_node(nid) {
            let nid_safe = safe_id(&node.properties.name);
            let shape = match node.label {
                NodeLabel::Service | NodeLabel::Repository => format!(
                    "    {}[/\"{}\\n({})\"/]",
                    nid_safe, safe_label(&node.properties.name), node.label.as_str()
                ),
                NodeLabel::DbEntity => format!(
                    "    {}[(\"{}\")]",
                    nid_safe, safe_label(&node.properties.name)
                ),
                NodeLabel::View => format!(
                    "    {}{{\"{}\"}}",
                    nid_safe, safe_label(&node.properties.name)
                ),
                _ => format!(
                    "    {}[\"{}\"]",
                    nid_safe, safe_label(&node.properties.name)
                ),
            };
            lines.push(shape);
        }
    }

    lines.extend(edges);
    lines.join("\n")
}

fn generate_sequence(
    graph: &gitnexus_core::graph::KnowledgeGraph,
    start: &gitnexus_core::graph::types::GraphNode,
) -> String {
    let mut lines = vec!["sequenceDiagram".to_string()];
    let mut participants = Vec::new();
    let mut interactions = Vec::new();
    let mut seen_participants = HashSet::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    // Add start as first participant
    let start_alias = safe_id(&start.properties.name);
    participants.push(format!(
        "    participant {} as {}",
        start_alias, safe_label(&start.properties.name)
    ));
    seen_participants.insert(start_alias.clone());

    visited.insert(start.id.clone());
    queue.push_back((start.id.clone(), start_alias.clone(), 0usize));

    while let Some((node_id, caller_alias, depth)) = queue.pop_front() {
        if depth >= 4 || interactions.len() >= 20 {
            break;
        }

        for rel in graph.iter_relationships() {
            if rel.source_id != node_id {
                continue;
            }
            // Only follow call-like relationships for sequence diagrams
            let rel_str = rel.rel_type.as_str();
            if !rel_str.contains("Call") && !rel_str.contains("Renders") && !rel_str.contains("DependsOn") {
                continue;
            }

            if let Some(target) = graph.get_node(&rel.target_id) {
                if target.label == NodeLabel::File || target.label == NodeLabel::Folder {
                    continue;
                }

                let target_alias = safe_id(&target.properties.name);

                if !seen_participants.contains(&target_alias) {
                    participants.push(format!(
                        "    participant {} as {}",
                        target_alias,
                        safe_label(&target.properties.name)
                    ));
                    seen_participants.insert(target_alias.clone());
                }

                let arrow = if rel_str.contains("Renders") {
                    "-->>"
                } else {
                    "->>"
                };

                interactions.push(format!(
                    "    {}{}{}: {}",
                    caller_alias, arrow, target_alias, rel_str
                ));

                if !visited.contains(&rel.target_id) {
                    visited.insert(rel.target_id.clone());
                    queue.push_back((rel.target_id.clone(), target_alias, depth + 1));
                }
            }
        }
    }

    lines.extend(participants);
    lines.extend(interactions);
    lines.join("\n")
}

fn generate_class_diagram(
    graph: &gitnexus_core::graph::KnowledgeGraph,
    start: &gitnexus_core::graph::types::GraphNode,
) -> String {
    let mut lines = vec!["classDiagram".to_string()];

    let class_name = safe_id(&start.properties.name);

    // Collect methods and properties in the same file
    let mut methods = Vec::new();
    let mut properties = Vec::new();

    for node in graph.iter_nodes() {
        if node.properties.file_path == start.properties.file_path {
            match node.label {
                NodeLabel::Method => {
                    let ret = node.properties.return_type.as_deref().unwrap_or("void");
                    methods.push(format!("        +{}() {}", node.properties.name, ret));
                }
                NodeLabel::Property => {
                    let ptype = node.properties.return_type.as_deref().unwrap_or("object");
                    properties.push(format!("        +{} : {}", node.properties.name, ptype));
                }
                NodeLabel::Constructor => {
                    methods.push(format!("        +{}()", node.properties.name));
                }
                _ => {}
            }
        }
    }

    lines.push(format!("    class {} {{", class_name));
    for p in properties.iter().take(20) {
        lines.push(p.clone());
    }
    for m in methods.iter().take(20) {
        lines.push(m.clone());
    }
    lines.push("    }".to_string());

    // Find inheritance
    for rel in graph.iter_relationships() {
        if rel.source_id == start.id && rel.rel_type.as_str() == "Inherits" {
            if let Some(parent) = graph.get_node(&rel.target_id) {
                let parent_id = safe_id(&parent.properties.name);
                lines.push(format!("    {} <|-- {}", parent_id, class_name));
                lines.push(format!("    class {} {{", parent_id));
                lines.push("    }".to_string());
            }
        }
    }

    // Find implementations
    for rel in graph.iter_relationships() {
        if rel.source_id == start.id && rel.rel_type.as_str() == "Implements" {
            if let Some(iface) = graph.get_node(&rel.target_id) {
                let iface_id = safe_id(&iface.properties.name);
                lines.push(format!("    {} <|.. {}", iface_id, class_name));
                lines.push(format!("    class {} {{", iface_id));
                lines.push(format!("        <<interface>>"));
                lines.push("    }".to_string());
            }
        }
    }

    lines.join("\n")
}
