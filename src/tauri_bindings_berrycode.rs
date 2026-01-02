//! Tauri bindings for BerryCode commands
//!
//! This module provides Rust wrappers around Tauri commands for BerryCode AI functionality.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub provider: String,
    pub description: String,
}

/// Initialize BerryCode session
pub async fn berrycode_init(
    model: Option<String>,
    mode: Option<String>,
    project_root: Option<String>,
) -> Result<String, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "model": model,
        "mode": mode,
        "projectRoot": project_root,
    }))
    .map_err(|e| format!("Serialization error: {}", e))?;

    let result = invoke("berrycode_init", args).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Send chat message
pub async fn berrycode_chat(message: String) -> Result<String, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "message": message,
    }))
    .map_err(|e| format!("Serialization error: {}", e))?;

    let result = invoke("berrycode_chat", args).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Add file to context
pub async fn berrycode_add_file(file_path: String) -> Result<String, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "filePath": file_path,
    }))
    .map_err(|e| format!("Serialization error: {}", e))?;

    let result = invoke("berrycode_add_file", args).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Remove file from context
pub async fn berrycode_drop_file(file_path: String) -> Result<String, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
        "filePath": file_path,
    }))
    .map_err(|e| format!("Serialization error: {}", e))?;

    let result = invoke("berrycode_drop_file", args).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// List context files
pub async fn berrycode_list_files() -> Result<Vec<String>, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let result = invoke("berrycode_list_files", JsValue::NULL).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// List available models
pub async fn berrycode_list_models() -> Result<Vec<ModelInfo>, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let result = invoke("berrycode_list_models", JsValue::NULL).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Get chat history
pub async fn berrycode_get_history() -> Result<Vec<ChatMessage>, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let result = invoke("berrycode_get_history", JsValue::NULL).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}

/// Clear chat history
pub async fn berrycode_clear_history() -> Result<String, String> {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    }

    let result = invoke("berrycode_clear_history", JsValue::NULL).await;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Deserialization error: {}", e))
}
