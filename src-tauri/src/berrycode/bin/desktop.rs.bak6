//! BerryCode Desktop Application (Tauri)
//!
//! This binary launches BerryCode as a native desktop application using Tauri.
//! It starts the Axum web server in the background and opens a window.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::net::TcpListener;
use std::path::PathBuf;
use std::fs;

/// Tauri command to open folder picker dialog
#[tauri::command]
fn pick_folder() -> Option<String> {
    use rfd::FileDialog;

    FileDialog::new()
        .set_title("プロジェクトフォルダを選択")
        .pick_folder()
        .map(|path| path.to_string_lossy().to_string())
}

/// Tauri command to open settings window
#[tauri::command]
fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    // Check if settings window already exists
    if let Some(window) = app.get_webview_window("settings") {
        // Focus existing window
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Create new settings window with Leptos UI
    tauri::WebviewWindowBuilder::new(
        &app,
        "settings",
        tauri::WebviewUrl::App("settings.html".into())
    )
    .title("BerryCode Settings")
    .inner_size(900.0, 700.0)
    .min_inner_size(800.0, 600.0)
    .center()
    .resizable(true)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Tauri command to open the new Rust editor
#[tauri::command]
fn open_editor(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    // Check if editor window already exists
    if let Some(window) = app.get_webview_window("berry-editor") {
        // Focus existing window
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Create new editor window using web server
    // Try common ports until we find the running server
    let port = find_running_port();
    let url = format!("http://localhost:{}/editor", port);

    tauri::WebviewWindowBuilder::new(
        &app,
        "berry-editor",
        tauri::WebviewUrl::External(url.parse().unwrap())
    )
    .title("BerryEditor - Pure Rust Code Editor")
    .inner_size(1400.0, 900.0)
    .min_inner_size(1000.0, 700.0)
    .center()
    .resizable(true)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Find an available port
fn find_available_port() -> u16 {
    // Try ports from 7770 to 7800
    for port in 7770..=7800 {
        if let Ok(listener) = TcpListener::bind(format!("127.0.0.1:{}", port)) {
            drop(listener);
            return port;
        }
    }
    // Fallback to 7778
    7778
}

/// Find the port where the web server is running
fn find_running_port() -> u16 {
    // Try to connect to common ports to find the running server
    for port in 7770..=7800 {
        if let Ok(_stream) = TcpListener::bind(format!("127.0.0.1:{}", port)) {
            // Port is available, not in use
            continue;
        } else {
            // Port is in use, this is likely our server
            return port;
        }
    }
    // Fallback
    7770
}

/// Get BerryCode data directory in user's home directory
fn get_data_directory() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir()
        .ok_or("Failed to get home directory")?;

    let data_dir = home_dir.join(".berrycode");

    // Create directory if it doesn't exist
    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)?;
        tracing::info!("Created BerryCode data directory: {:?}", data_dir);
    }

    Ok(data_dir)
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load .env file
    match dotenv::dotenv() {
        Ok(path) => tracing::info!("Loaded .env from: {:?}", path),
        Err(e) => tracing::warn!("Failed to load .env file: {}", e),
    }

    // Get BerryCode data directory
    let data_dir = match get_data_directory() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Failed to get data directory: {}", e);
            return;
        }
    };

    // Get current executable directory and find project root (templates directory)
    let _exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    // Try to find templates directory
    let templates_dir = if let Ok(current_dir) = std::env::current_dir() {
        let templates = current_dir.join("templates");
        if templates.exists() {
            Some(templates)
        } else {
            // Try parent directories (for when running from target/debug)
            current_dir.parent()
                .and_then(|p| p.parent())
                .map(|p| p.join("templates"))
                .filter(|p| p.exists())
        }
    } else {
        None
    };

    if let Some(ref dir) = templates_dir {
        tracing::info!("Templates directory: {:?}", dir);
    } else {
        tracing::warn!("Templates directory not found, using default search path");
    }

    // Find available port
    let port = find_available_port();
    tracing::info!("BerryCode Desktop starting on port {}", port);
    tracing::info!("Data directory: {:?}", data_dir);

    // Prepare database URL (For AnyPool, use sqlite: format)
    let database_path = data_dir.join("berrycode.db");
    let database_url = format!("sqlite:{}?mode=rwc", database_path.display());
    tracing::info!("Database path: {:?}", database_path);
    tracing::info!("Database URL: {}", database_url);

    // Set environment variable to indicate desktop mode (no authentication required)
    std::env::set_var("BERRYCODE_DESKTOP_MODE", "true");

    // Start Axum server in background
    tokio::spawn(async move {
        tracing::info!("Starting BerryCode web server...");

        // Run the web server with templates directory and database
        if let Err(e) = berrycode::web::run_server(port, templates_dir, database_url).await {
            tracing::error!("Server error: {}", e);
        }
    });

    // Wait for server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    // TODO: CLI auto-launch is temporarily disabled
    // Users can manually launch CLI with: cargo run --bin berrycode

    // Build Tauri application
    tauri::Builder::default()
        .setup(move |app| {
            // Create main window (BerryEditor)
            let url = format!("http://localhost:{}/editor", port);
            tracing::info!("Opening BerryEditor at {}", url);

            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::External(url.parse().unwrap())
            )
            .title("BerryCode - AI Pair Programming")
            .inner_size(1400.0, 900.0)
            .min_inner_size(1000.0, 700.0)
            .center()
            .build()
            .map_err(|e| {
                tracing::error!("Failed to create window: {}", e);
                e
            })?;

            tracing::info!("BerryCode desktop application started successfully");

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Hide instead of closing on macOS (keep in dock)
                #[cfg(target_os = "macos")]
                {
                    if let Err(e) = window.hide() {
                        tracing::error!("Failed to hide window: {}", e);
                    } else {
                        api.prevent_close();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            pick_folder,
            open_settings,
            open_editor,
            // Editor commands
            berrycode::editor_backend::editor_read_dir,
            berrycode::editor_backend::editor_read_file,
            berrycode::editor_backend::editor_write_file,
            berrycode::editor_backend::editor_create_file,
            berrycode::editor_backend::editor_delete_file,
            berrycode::editor_backend::editor_rename_file,
            berrycode::editor_backend::editor_create_directory,
            berrycode::editor_backend::editor_get_file_info,
            berrycode::editor_backend::editor_search_in_files,
            // LSP commands
            berrycode::editor_backend::lsp_initialize,
            berrycode::editor_backend::lsp_get_completions,
            berrycode::editor_backend::lsp_get_diagnostics,
            berrycode::editor_backend::lsp_hover,
            berrycode::editor_backend::lsp_goto_definition,
            berrycode::editor_backend::lsp_find_references,
            berrycode::editor_backend::lsp_code_actions,
            // Debug commands (Phase 2)
            berrycode::editor_backend::debug_start_session,
            berrycode::editor_backend::debug_stop_session,
            berrycode::editor_backend::debug_set_breakpoint,
            berrycode::editor_backend::debug_remove_breakpoint,
            berrycode::editor_backend::debug_continue,
            berrycode::editor_backend::debug_step_over,
            berrycode::editor_backend::debug_step_into,
            berrycode::editor_backend::debug_step_out,
            berrycode::editor_backend::debug_get_stack_trace,
            berrycode::editor_backend::debug_get_variables,
            berrycode::editor_backend::debug_evaluate,
        ])
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
