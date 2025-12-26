//! Tauri LSP Command Bindings

use crate::lsp::{CompletionItem, Location};
use super::HoverInfo;
use wasm_bindgen::prelude::*;
use crate::tauri_bindings::is_tauri_context;

/// Initialize LSP for a language
pub async fn lsp_initialize(language: String, root_uri: String) -> Result<bool, String> {
    if !is_tauri_context() {
        return Err("Not running in Tauri context".to_string());
    }

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "language": language,
        "rootUri": root_uri
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    match tauri_invoke("lsp_initialize", args).await {
        val => serde_wasm_bindgen::from_value(val)
            .map_err(|e| format!("Failed to parse result: {}", e)),
    }
}

/// Get completions from LSP
pub async fn lsp_get_completions(
    language: String,
    file_path: String,
    line: u32,
    character: u32,
) -> Result<Vec<CompletionItem>, String> {
    if !is_tauri_context() {
        return Err("Not running in Tauri context".to_string());
    }

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "language": language,
        "filePath": file_path,
        "line": line,
        "character": character
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    match tauri_invoke("lsp_get_completions", args).await {
        val => serde_wasm_bindgen::from_value(val)
            .map_err(|e| format!("Failed to parse completions: {}", e)),
    }
}

/// Get hover information from LSP
pub async fn lsp_get_hover(
    language: String,
    file_path: String,
    line: u32,
    character: u32,
) -> Result<Option<HoverInfo>, String> {
    if !is_tauri_context() {
        return Err("Not running in Tauri context".to_string());
    }

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "language": language,
        "filePath": file_path,
        "line": line,
        "character": character
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    match tauri_invoke("lsp_get_hover", args).await {
        val => serde_wasm_bindgen::from_value(val)
            .map_err(|e| format!("Failed to parse hover: {}", e)),
    }
}

/// Go to definition
pub async fn lsp_goto_definition(
    language: String,
    file_path: String,
    line: u32,
    character: u32,
) -> Result<Vec<Location>, String> {
    if !is_tauri_context() {
        return Err("Not running in Tauri context".to_string());
    }

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "language": language,
        "filePath": file_path,
        "line": line,
        "character": character
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    match tauri_invoke("lsp_goto_definition", args).await {
        val => serde_wasm_bindgen::from_value(val)
            .map_err(|e| format!("Failed to parse locations: {}", e)),
    }
}

/// Shutdown LSP for a language
pub async fn lsp_shutdown(language: String) -> Result<bool, String> {
    if !is_tauri_context() {
        return Err("Not running in Tauri context".to_string());
    }

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "language": language
    }))
    .map_err(|e| format!("Failed to serialize args: {}", e))?;

    match tauri_invoke("lsp_shutdown", args).await {
        val => serde_wasm_bindgen::from_value(val)
            .map_err(|e| format!("Failed to parse result: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    // Note: tokio is not available in WASM target, so these tests are commented out
    // These would need to be tested in the Tauri backend instead

    // use super::*;
    //
    // #[tokio::test]
    // async fn test_lsp_functions_not_in_tauri() {
    //     // These should all fail when not in Tauri context
    //     let result = lsp_initialize("rust".to_string(), "/test".to_string()).await;
    //     assert!(result.is_err());
    //
    //     let result2 = lsp_get_completions("rust".to_string(), "/test.rs".to_string(), 0, 0).await;
    //     assert!(result2.is_err());
    // }
}
