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
            commands::repos::analyze_repo,
            commands::repos::generate_docs,
            // Graph
            commands::graph::get_graph_data,
            commands::graph::get_subgraph,
            commands::graph::get_features,
            // Search
            commands::search::search_symbols,
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
            // Chat Q&A
            commands::chat::chat_ask,
            commands::chat::chat_get_config,
            commands::chat::chat_set_config,
            // Chat Intelligence (Planner & Executor)
            commands::chat_planner::chat_pick_files,
            commands::chat_planner::chat_pick_symbols,
            commands::chat_planner::chat_pick_modules,
            commands::chat_executor::chat_execute_step,
            commands::chat_executor::chat_execute_plan,
            // Feature-Dev (3-phase artifact pipeline, absorbs Claude's feature-dev skill)
            commands::feature_dev::feature_dev_run,
            // Code-Review (absorbs Claude's code-review skill — pre-commit review)
            commands::code_review::code_review_run,
            // Simplify (absorbs Claude's simplify skill — refactor proposals)
            commands::simplify::simplify_run,
            // Rename refactor (multi-file, graph-confirmed)
            commands::rename::rename_run,
            // Bookmarks (per-repo persistent node bookmarks)
            commands::bookmarks::bookmarks_list,
            commands::bookmarks::bookmarks_add,
            commands::bookmarks::bookmarks_remove,
            commands::bookmarks::bookmarks_clear,
            // Comments (per-node threads)
            commands::comments::comments_for_node,
            commands::comments::comments_add,
            commands::comments::comments_remove,
            // Saved Cypher queries
            commands::saved_queries::saved_queries_list,
            commands::saved_queries::saved_queries_save,
            commands::saved_queries::saved_queries_delete,
            // Interactive HTML export (self-contained shareable graph)
            commands::html_export::export_interactive_html,
            // Wiki generation (Markdown pages per community)
            commands::wiki::wiki_generate,
            // Cypher notebooks
            commands::notebooks::notebook_list,
            commands::notebooks::notebook_load,
            commands::notebooks::notebook_save,
            commands::notebooks::notebook_delete,
            // Multi-repo overview dashboard
            commands::repos_overview::repos_overview,
            // Repo activity history (timeline of analyze runs)
            commands::activity::activity_record,
            commands::activity::activity_list,
            commands::activity::activity_clear,
            // Snapshot history + diff (B3 full + B4)
            commands::snapshots::snapshot_create,
            commands::snapshots::snapshot_list,
            commands::snapshots::snapshot_delete,
            commands::snapshots::snapshot_diff,
            // Custom dashboards (E)
            commands::dashboards::dashboard_list,
            commands::dashboards::dashboard_load,
            commands::dashboards::dashboard_save,
            commands::dashboards::dashboard_delete,
            // Workflow editor (E)
            commands::workflows::workflow_list,
            commands::workflows::workflow_load,
            commands::workflows::workflow_save,
            commands::workflows::workflow_delete,
            commands::workflows::workflow_run,
            // User-defined slash commands (E light)
            commands::user_commands::user_commands_list,
            commands::user_commands::user_commands_save,
            commands::user_commands::user_commands_delete,
            commands::user_commands::user_command_resolve,
            // User data bundle export/import
            commands::user_bundle::user_bundle_export,
            commands::user_bundle::user_bundle_import,
            // Cypher
            commands::cypher::execute_cypher,
            // Export
            commands::export::export_docs_docx,
            commands::export::export_obsidian_vault,
            commands::export::get_aspnet_stats,            // Process Flows
            commands::process::get_process_flows,
            // Git Analytics
            commands::git_analytics::get_hotspots,
            commands::git_analytics::get_coupling,
            commands::git_analytics::get_ownership,
            // Code Health
            commands::health::get_code_health,
            // Coverage & Diagrams
            commands::coverage::get_coverage_stats,
            commands::diagram::get_diagram,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            tracing::error!("Failed to start GitNexus desktop: {}", e);
            eprintln!("Fatal: GitNexus failed to start: {e}");
            std::process::exit(1);
        });
}
