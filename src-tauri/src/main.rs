// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod fs_commands;
mod search_commands;
mod lsp;
mod git;

use lsp::LspManager;
use git::GitManager;
use std::sync::{Arc, Mutex};
use tauri::Manager;

fn main() {
    // Create LSP manager
    let lsp_manager = Arc::new(Mutex::new(LspManager::new()));

    // Create Git manager
    let git_manager = GitManager::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(lsp_manager)
        .manage(git_manager)
        .invoke_handler(tauri::generate_handler![
            fs_commands::read_file,
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
