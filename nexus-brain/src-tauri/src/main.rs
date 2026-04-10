// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use std::path::{Path, PathBuf};
use regex::Regex;
use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize)]
struct VaultEntry {
    name: String,
    path: String,
    is_dir: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GraphNode {
    id: String,
    label: String,
    group: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GraphEdge {
    source: String,
    target: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct VaultGraph {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

#[tauri::command]
async fn list_vault(path: String) -> Result<Vec<VaultEntry>, String> {
    let mut entries = Vec::new();
    let root = PathBuf::from(&path);
    
    if !root.exists() {
        return Err("Vault path does not exist".into());
    }

    for entry in WalkDir::new(&root).max_depth(3).into_iter().filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        let relative_path = entry.path().strip_prefix(&root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| name.clone());
            
        entries.push(VaultEntry {
            name,
            path: relative_path,
            is_dir: entry.file_type().is_dir(),
        });
    }
    Ok(entries)
}

#[tauri::command]
async fn get_vault_graph(vault_path: String) -> Result<VaultGraph, String> {
    let root = PathBuf::from(&vault_path);
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut seen_nodes = HashSet::new();

    let re_link = Regex::new(r"\[\[([^\]|]+)(?:\|[^\]]+)?\]\]").unwrap();

    for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "md") {
            let rel_path = entry.path().strip_prefix(&root).unwrap().to_string_lossy().to_string();
            let name = entry.path().file_stem().unwrap().to_string_lossy().to_string();
            
            let group = if rel_path.contains("Modules") { "module" }
                        else if rel_path.contains("Processus") { "process" }
                        else if rel_path.contains("Symboles") { "symbol" }
                        else { "file" };

            if seen_nodes.insert(name.clone()) {
                nodes.push(GraphNode {
                    id: name.clone(),
                    label: name.clone(),
                    group: group.to_string(),
                });
            }

            // Extract links
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                for cap in re_link.captures_iter(&content) {
                    let target = cap[1].trim();
                    // Clean target from path (Obsidian style)
                    let target_name = Path::new(target).file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| target.to_string());

                    edges.push(GraphEdge {
                        source: name.clone(),
                        target: target_name,
                    });
                }
            }
        }
    }

    Ok(VaultGraph { nodes, edges })
}

#[tauri::command]
async fn read_note(vault_path: String, note_path: String) -> Result<String, String> {
    let full_path = PathBuf::from(vault_path).join(note_path);
    std::fs::read_to_string(full_path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_note(vault_path: String, note_path: String, content: String) -> Result<(), String> {
    let full_path = PathBuf::from(vault_path).join(note_path);
    std::fs::write(full_path, content).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            list_vault,
            read_note,
            save_note,
            get_vault_graph
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
