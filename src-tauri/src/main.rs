// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod fs_commands;
mod search_commands;
mod lsp;
mod git;
mod indexer;  // ✅ IntelliJ Pro: Background symbol indexing
mod syntax_highlighter;  // ✅ Parallel syntax highlighting with rayon
mod streaming;  // ✅ Async streaming for large files
mod hyper_search;  // ✅ Strategy 3: Zero-memory parallel search

use lsp::LspManager;
use git::GitManager;
use indexer::SymbolIndex;
use std::sync::{Arc, Mutex};
use tauri::Manager;

fn main() {
    // Create LSP manager
    let lsp_manager = Arc::new(Mutex::new(LspManager::new()));

    // Create Git manager
    let git_manager = GitManager::new();

    // ✅ IntelliJ Pro: Create Symbol Index for background indexing
    let symbol_index = Arc::new(Mutex::new(SymbolIndex::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(lsp_manager)
        .manage(git_manager)
        .manage(symbol_index)  // ✅ IntelliJ Pro: Manage symbol index state
        .invoke_handler(tauri::generate_handler![
            fs_commands::read_file,
            fs_commands::read_file_partial,  // ✅ IntelliJ Pro: Lazy file loading
            fs_commands::read_file_chunk,    // ✅ IntelliJ Pro: Streaming large files
            fs_commands::write_file,
            fs_commands::read_dir,
            fs_commands::create_file,
            fs_commands::delete_file,
            fs_commands::rename_file,
            fs_commands::get_file_metadata,
            search_commands::search_in_files,
            lsp::commands::lsp_initialize,
            lsp::commands::lsp_get_completions,
            lsp::commands::lsp_get_hover,
            lsp::commands::lsp_goto_definition,
            lsp::commands::lsp_shutdown,
            git::commands::git_set_repo_path,
            git::commands::git_status,
            git::commands::git_list_branches,
            git::commands::git_current_branch,
            git::commands::git_stage_file,
            git::commands::git_unstage_file,
            git::commands::git_stage_all,
            git::commands::git_commit,
            git::commands::git_checkout_branch,
            git::commands::git_create_branch,
            git::commands::git_delete_branch,
            git::commands::git_log,
            git::commands::git_diff_file,
            git::commands::git_blame,
            // ✅ IntelliJ Pro: Background indexing commands
            indexer::commands::index_workspace,
            indexer::commands::search_symbols,
            indexer::commands::index_file,
            indexer::commands::get_symbol_count,
            // ✅ Parallel syntax highlighting commands
            syntax_highlighter::highlight_file_parallel,
            syntax_highlighter::invalidate_syntax_cache,
            syntax_highlighter::get_syntax_cache_stats,
            // ✅ Streaming commands for large files
            streaming::stream_large_file,
            streaming::read_file_auto,
            // ✅ Hyper-parallel search commands
            hyper_search::hyper_search,
            hyper_search::hyper_replace,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
