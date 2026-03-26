use std::collections::BTreeMap;
use std::path::Path;

use tauri::State;

use gitnexus_core::config::languages::SupportedLanguage;
use gitnexus_core::graph::types::NodeLabel;

use crate::state::AppState;
use crate::types::{FileContent, FileTreeNode};

/// Build a file tree from the graph's file nodes.
#[tauri::command]
pub async fn get_file_tree(state: State<'_, AppState>) -> Result<Vec<FileTreeNode>, String> {
    let (graph, _indexes, _fts, _repo_path) = state.get_repo(None).await?;

    // Collect all unique file paths from File/Folder nodes
    let mut file_paths: Vec<String> = Vec::new();

    for node in graph.iter_nodes() {
        if node.label == NodeLabel::File {
            file_paths.push(node.properties.file_path.clone());
        }
    }

    file_paths.sort();

    // Build tree structure
    let tree = build_tree_from_paths(&file_paths);
    Ok(tree)
}

/// Read file content from disk.
#[tauri::command]
pub async fn read_file_content(
    state: State<'_, AppState>,
    file_path: String,
    start_line: Option<u32>,
    end_line: Option<u32>,
) -> Result<FileContent, String> {
    let (_graph, _indexes, _fts, repo_path) = state.get_repo(None).await?;

    let full_path = Path::new(&repo_path).join(&file_path);

    // Security: prevent path traversal outside the repo
    let full_canonical = full_path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve path '{}': {}", file_path, e))?;
    let repo_canonical = Path::new(&repo_path)
        .canonicalize()
        .map_err(|e| format!("Failed to resolve repo path: {}", e))?;
    if !full_canonical.starts_with(&repo_canonical) {
        return Err("Access denied: path is outside the repository".to_string());
    }

    let content = std::fs::read_to_string(&full_canonical)
        .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;

    let total_lines = content.lines().count();

    // If line range specified, extract subset
    let content = match (start_line, end_line) {
        (Some(start), Some(end)) => {
            let start = (start as usize).saturating_sub(1);
            let end = end as usize;
            content
                .lines()
                .skip(start)
                .take(end - start)
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => content,
    };

    // Detect language from extension
    let language = Path::new(&file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| SupportedLanguage::from_extension(&format!(".{}", ext)))
        .map(|l| format!("{:?}", l).to_lowercase());

    Ok(FileContent {
        path: file_path,
        content,
        language,
        total_lines,
    })
}

fn build_tree_from_paths(paths: &[String]) -> Vec<FileTreeNode> {
    // Normalize Windows backslashes to forward slashes
    let paths: Vec<String> = paths.iter().map(|p| p.replace('\\', "/")).collect();
    let paths_ref: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();

    build_tree_impl(&paths_ref)
}

fn build_tree_impl(paths: &[&str]) -> Vec<FileTreeNode> {
    let mut dir_children: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut root_files = Vec::new();

    for path in paths {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() > 1 {
            dir_children
                .entry(parts[0].to_string())
                .or_default()
                .push(parts[1..].join("/"));
        } else {
            root_files.push(path.to_string());
        }
    }

    let mut result = Vec::new();

    // Add directories
    for (dir_name, child_paths) in &dir_children {
        let child_refs: Vec<&str> = child_paths.iter().map(|s| s.as_str()).collect();
        let children = build_tree_impl(&child_refs);
        result.push(FileTreeNode {
            name: dir_name.clone(),
            path: dir_name.clone(),
            is_dir: true,
            children,
        });
    }

    // Add files at this level
    for file_path in &root_files {
        result.push(FileTreeNode {
            name: file_path.clone(),
            path: file_path.clone(),
            is_dir: false,
            children: Vec::new(),
        });
    }

    result
}
