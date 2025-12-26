//! Tauri bindings for search functionality

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: String,
    pub line_number: usize,
    pub column: usize,
    pub line_text: String,
    pub match_start: usize,
    pub match_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub use_regex: bool,
    pub whole_word: bool,
    pub include_pattern: Option<String>,
    pub exclude_pattern: Option<String>,
    pub max_results: Option<usize>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            use_regex: false,
            whole_word: false,
            include_pattern: None,
            exclude_pattern: None,
            max_results: Some(1000),
        }
    }
}

#[wasm_bindgen(module = "/src/tauri-bindings.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = "tauri_search_in_files")]
    pub async fn tauri_search_in_files(
        query: String,
        root_path: String,
        options: JsValue,
    ) -> Result<JsValue, JsValue>;
}

/// Search in files using Tauri command
pub async fn search_in_files(
    query: &str,
    root_path: &str,
    options: Option<SearchOptions>,
) -> Result<Vec<SearchResult>, String> {
    if !crate::tauri_bindings::is_tauri_context() {
        return Err("Not running in Tauri context".to_string());
    }

    let opts = options.unwrap_or_default();
    let opts_js = serde_wasm_bindgen::to_value(&opts)
        .map_err(|e| format!("Failed to serialize options: {}", e))?;

    match tauri_search_in_files(query.to_string(), root_path.to_string(), opts_js).await {
        Ok(val) => {
            serde_wasm_bindgen::from_value(val)
                .map_err(|e| format!("Failed to deserialize results: {}", e))
        }
        Err(e) => Err(format!("{:?}", e)),
    }
}
