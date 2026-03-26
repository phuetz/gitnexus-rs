#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use gitnexus_desktop::commands;
use gitnexus_desktop::state::AppState;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // Repos
            commands::repos::list_repos,
            commands::repos::open_repo,
            commands::repos::get_active_repo,
            commands::repos::analyze_repo,
            commands::repos::generate_docs,
            // Graph
            commands::graph::get_graph_data,
            commands::graph::get_subgraph,
            commands::graph::get_neighbors,
            // Search
            commands::search::search_symbols,
            commands::search::search_autocomplete,
            // Context
            commands::context::get_symbol_context,
            // Impact
            commands::impact::get_impact_analysis,
            // Files
            commands::files::get_file_tree,
            commands::files::read_file_content,
            // Cypher
            commands::cypher::execute_cypher,
        ])
        .run(tauri::generate_context!())
        .expect("error while running GitNexus desktop application");
}
