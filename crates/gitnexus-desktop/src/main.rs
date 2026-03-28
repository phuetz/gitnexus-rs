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
        .plugin(tauri_plugin_dialog::init())
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
            // Docs
            commands::docs::get_doc_index,
            commands::docs::read_doc,
            commands::docs::has_docs,
            // Chat Q&A
            commands::chat::chat_ask,
            commands::chat::chat_get_config,
            commands::chat::chat_set_config,
            commands::chat::chat_search_context,
            // Chat Intelligence (Planner & Executor)
            commands::chat_planner::chat_analyze_query,
            commands::chat_planner::chat_plan_research,
            commands::chat_planner::chat_pick_files,
            commands::chat_planner::chat_pick_symbols,
            commands::chat_planner::chat_pick_modules,
            commands::chat_executor::chat_execute_step,
            commands::chat_executor::chat_execute_plan,
            // Cypher
            commands::cypher::execute_cypher,
            // Export
            commands::export::export_docs_docx,
            commands::export::get_aspnet_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running GitNexus desktop application");
}
