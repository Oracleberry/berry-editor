//! Tauri Commands for LSP
//! Exposes LSP functionality to the WASM frontend

use super::{LspManager, protocol::*};
use std::sync::{Arc, Mutex};
use tauri::State;

/// Register all LSP commands
pub fn register_lsp_commands<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        lsp_initialize,
        lsp_get_completions,
        lsp_get_hover,
        lsp_goto_definition,
        lsp_shutdown,
    ])
}

/// Initialize LSP for a language
#[tauri::command]
pub async fn lsp_initialize(
    language: String,
    root_uri: String,
    manager: State<'_, Arc<Mutex<LspManager>>>,
) -> Result<bool, String> {
    let mgr = manager.lock().unwrap();
    mgr.initialize_client(language, root_uri)?;
    Ok(true)
}

/// Get completions at position
#[tauri::command]
pub async fn lsp_get_completions(
    language: String,
    file_path: String,
    line: u32,
    character: u32,
    manager: State<'_, Arc<Mutex<LspManager>>>,
) -> Result<Vec<CompletionItem>, String> {
    let mgr = manager.lock().unwrap();

    let client_arc = mgr
        .get_client(&language)
        .ok_or_else(|| format!("LSP not initialized for {}", language))?;

    let mut client = client_arc.lock().unwrap();

    // Convert file path to URI
    let file_uri = if file_path.starts_with("file://") {
        file_path
    } else {
        format!("file://{}", file_path)
    };

    client.get_completions(&file_uri, line, character)
}

/// Get hover information at position
#[tauri::command]
pub async fn lsp_get_hover(
    language: String,
    file_path: String,
    line: u32,
    character: u32,
    manager: State<'_, Arc<Mutex<LspManager>>>,
) -> Result<Option<Hover>, String> {
    let mgr = manager.lock().unwrap();

    let client_arc = mgr
        .get_client(&language)
        .ok_or_else(|| format!("LSP not initialized for {}", language))?;

    let mut client = client_arc.lock().unwrap();

    // Convert file path to URI
    let file_uri = if file_path.starts_with("file://") {
        file_path
    } else {
        format!("file://{}", file_path)
    };

    client.get_hover(&file_uri, line, character)
}

/// Go to definition
#[tauri::command]
pub async fn lsp_goto_definition(
    language: String,
    file_path: String,
    line: u32,
    character: u32,
    _manager: State<'_, Arc<Mutex<LspManager>>>,
) -> Result<Vec<Location>, String> {
    // Simplified implementation for now
    // In real implementation, call client.get_definition()
    Ok(Vec::new())
}

/// Shutdown LSP for a language
#[tauri::command]
pub async fn lsp_shutdown(
    language: String,
    manager: State<'_, Arc<Mutex<LspManager>>>,
) -> Result<bool, String> {
    let mgr = manager.lock().unwrap();
    mgr.shutdown_client(&language)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_uri_conversion() {
        let path = "/path/to/file.rs";
        let uri = format!("file://{}", path);
        assert_eq!(uri, "file:///path/to/file.rs");
    }

    #[test]
    fn test_file_uri_already_formatted() {
        let path = "file:///path/to/file.rs";
        let uri = if path.starts_with("file://") {
            path.to_string()
        } else {
            format!("file://{}", path)
        };
        assert_eq!(uri, "file:///path/to/file.rs");
    }
}
