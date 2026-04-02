//! Cross-reference linking between documentation pages.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;

use gitnexus_core::graph::types::*;
use gitnexus_core::graph::KnowledgeGraph;

use super::utils::sanitize_filename;

/// Post-processing step that adds cross-reference links between documentation pages.
/// Runs after all pages are generated (and optionally enriched) but before HTML generation.
pub(super) fn apply_cross_references(docs_dir: &Path, graph: &KnowledgeGraph) -> Result<usize> {
    // 1. Build a map of known names -> page links
    let mut known_names: Vec<(String, String)> = Vec::new(); // (name, link)

    // Controllers, Services, Repositories, DbEntities, ExternalServices
    for node in graph.iter_nodes() {
        match node.label {
            NodeLabel::Controller => {
                let name = &node.properties.name;
                let filename = format!("ctrl-{}", sanitize_filename(name));
                known_names.push((name.clone(), format!("./modules/{}.md", filename)));
            }
            NodeLabel::Service | NodeLabel::Repository => {
                known_names.push((
                    node.properties.name.clone(),
                    "./modules/services.md".to_string(),
                ));
            }
            NodeLabel::DbEntity => {
                known_names.push((
                    node.properties.name.clone(),
                    format!("./modules/data-entities.md#{}", node.properties.name),
                ));
            }
            NodeLabel::ExternalService => {
                known_names.push((
                    node.properties.name.clone(),
                    "./modules/external-services.md".to_string(),
                ));
            }
            _ => {}
        }
    }

    // Sort by length descending (longest match first, avoid partial matches)
    known_names.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    // Filter out names shorter than 5 chars (too generic)
    known_names.retain(|(name, _)| name.len() >= 5);

    // 2. Process each .md file
    let mut total_links = 0;
    let mut files_to_process: Vec<PathBuf> = Vec::new();

    for entry in std::fs::read_dir(docs_dir)?.flatten() {
        if entry.path().extension().map_or(false, |e| e == "md") {
            files_to_process.push(entry.path());
        }
    }
    let modules_dir = docs_dir.join("modules");
    if modules_dir.exists() {
        for entry in std::fs::read_dir(&modules_dir)?.flatten() {
            if entry.path().extension().map_or(false, |e| e == "md") {
                files_to_process.push(entry.path());
            }
        }
    }

    for file_path in &files_to_process {
        let content = std::fs::read_to_string(file_path)?;
        let mut modified = content.clone();
        let mut linked_names: HashSet<String> = HashSet::new();
        let mut page_links = 0;

        for (name, link) in &known_names {
            // Skip if already linked on this page
            if linked_names.contains(name) {
                continue;
            }

            // Skip self-references (don't link to the current page)
            if link.contains(
                file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(""),
            ) {
                continue;
            }

            // Find FIRST occurrence that's not inside a code block, heading, or existing link
            if let Some(idx) = modified.find(name.as_str()) {
                // Check context: skip if inside code block or already linked
                let before = &modified[..idx];

                let in_code = before.matches("```").count() % 2 == 1;
                let in_inline_code = before.ends_with('`');
                let in_link = before.ends_with('[') || before.ends_with("](");
                let in_heading = before
                    .lines()
                    .last()
                    .map_or(false, |l| l.starts_with('#'));

                if !in_code && !in_inline_code && !in_link && !in_heading {
                    // Replace first occurrence with link
                    modified = format!(
                        "{}[{}]({}){}", &modified[..idx], name, link,
                        &modified[idx + name.len()..]
                    );
                    linked_names.insert(name.clone());
                    page_links += 1;
                }
            }
        }

        if page_links > 0 {
            std::fs::write(file_path, &modified)?;
            total_links += page_links;
        }
    }

    Ok(total_links)
}
