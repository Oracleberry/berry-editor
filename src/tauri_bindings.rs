//! Tauri command bindings for WASM frontend

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileNode>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetadata {
    pub size: u64,
    pub modified: Option<u64>,
    pub is_readonly: bool,
}

#[wasm_bindgen(module = "/src/tauri-bindings.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    pub async fn tauri_read_file(path: String) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn tauri_write_file(path: String, contents: String) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn tauri_read_dir(path: String, max_depth: Option<usize>) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn tauri_create_file(path: String, contents: Option<String>) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn tauri_delete_file(path: String) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn tauri_rename_file(old_path: String, new_path: String) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    pub async fn tauri_get_file_metadata(path: String) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = "isTauriContext")]
    pub fn is_tauri_context() -> bool;
}

/// Read file contents
pub async fn read_file(path: &str) -> Result<String, String> {
    if !is_tauri_context() {
        return Err("Not running in Tauri context".to_string());
    }

    match tauri_read_file(path.to_string()).await {
        Ok(val) => {
            serde_wasm_bindgen::from_value(val)
                .map_err(|e| format!("Failed to deserialize: {}", e))
        }
        Err(e) => Err(format!("{:?}", e)),
    }
}

/// Write file contents
pub async fn write_file(path: &str, contents: &str) -> Result<(), String> {
    if !is_tauri_context() {
        return Err("Not running in Tauri context".to_string());
    }

    tauri_write_file(path.to_string(), contents.to_string())
        .await
        .map(|_| ())
        .map_err(|e| format!("{:?}", e))
}

/// Read directory contents
pub async fn read_dir(path: &str, max_depth: Option<usize>) -> Result<Vec<FileNode>, String> {
    if !is_tauri_context() {
        return Err("Not running in Tauri context".to_string());
    }

    match tauri_read_dir(path.to_string(), max_depth).await {
        Ok(val) => {
            serde_wasm_bindgen::from_value(val)
                .map_err(|e| format!("Failed to deserialize: {}", e))
        }
        Err(e) => Err(format!("{:?}", e)),
    }
}

/// Create a new file
pub async fn create_file(path: &str, contents: Option<String>) -> Result<(), String> {
    if !is_tauri_context() {
        return Err("Not running in Tauri context".to_string());
    }

    tauri_create_file(path.to_string(), contents)
        .await
        .map(|_| ())
        .map_err(|e| format!("{:?}", e))
}

/// Delete a file or directory
pub async fn delete_file(path: &str) -> Result<(), String> {
    if !is_tauri_context() {
        return Err("Not running in Tauri context".to_string());
    }

    tauri_delete_file(path.to_string())
        .await
        .map(|_| ())
        .map_err(|e| format!("{:?}", e))
}

/// Rename/move a file or directory
pub async fn rename_file(old_path: &str, new_path: &str) -> Result<(), String> {
    if !is_tauri_context() {
        return Err("Not running in Tauri context".to_string());
    }

    tauri_rename_file(old_path.to_string(), new_path.to_string())
        .await
        .map(|_| ())
        .map_err(|e| format!("{:?}", e))
}

/// Get file metadata
pub async fn get_file_metadata(path: &str) -> Result<FileMetadata, String> {
    if !is_tauri_context() {
        return Err("Not running in Tauri context".to_string());
    }

    match tauri_get_file_metadata(path.to_string()).await {
        Ok(val) => {
            serde_wasm_bindgen::from_value(val)
                .map_err(|e| format!("Failed to deserialize: {}", e))
        }
        Err(e) => Err(format!("{:?}", e)),
    }
}
