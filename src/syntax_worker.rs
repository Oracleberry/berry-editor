//! Syntax Worker Integration
//!
//! Web Worker wrapper for non-blocking syntax analysis
//! Guarantees 144fps UI by isolating parsing to separate thread

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Worker, MessageEvent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyntaxWorkerMessage {
    HighlightLines {
        lines: Vec<LineToHighlight>,
        language: Option<String>,
    },
    HighlightSingleLine {
        line_number: usize,
        text: String,
        language: Option<String>,
    },
    ClearCache,
    SetLanguage {
        language: String,
    },
    GetCacheStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineToHighlight {
    #[serde(rename = "lineNumber")]
    pub line_number: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyntaxWorkerResponse {
    Ready,
    HighlightResult {
        results: Vec<HighlightedLine>,
    },
    SingleLineResult {
        #[serde(rename = "lineNumber")]
        line_number: usize,
        html: String,
    },
    CacheCleared,
    LanguageSet {
        language: String,
    },
    CacheStats {
        size: usize,
        #[serde(rename = "isAnalyzing")]
        is_analyzing: bool,
    },
    Error {
        error: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightedLine {
    #[serde(rename = "lineNumber")]
    pub line_number: usize,
    pub html: String,
}

/// Syntax Worker wrapper
pub struct SyntaxWorker {
    worker: Worker,
    on_result: RwSignal<Option<HashMap<usize, String>>>,
    on_error: RwSignal<Option<String>>,
}

impl SyntaxWorker {
    /// Create a new syntax worker
    pub fn new(
        on_result: RwSignal<Option<HashMap<usize, String>>>,
        on_error: RwSignal<Option<String>>,
    ) -> Result<Self, JsValue> {
        let worker = Worker::new("/syntax-worker.js")?;

        Ok(Self {
            worker,
            on_result,
            on_error,
        })
    }

    /// Set up message handler
    pub fn setup_handlers(&self) {
        let on_result = self.on_result;
        let on_error = self.on_error;

        let onmessage_callback = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Ok(response) = serde_wasm_bindgen::from_value::<SyntaxWorkerResponse>(event.data()) {
                match response {
                    SyntaxWorkerResponse::Ready => {
                        web_sys::console::log_1(&"Syntax worker ready".into());
                    }
                    SyntaxWorkerResponse::HighlightResult { results } => {
                        let mut map = HashMap::new();
                        for line in results {
                            map.insert(line.line_number, line.html);
                        }
                        on_result.set(Some(map));
                    }
                    SyntaxWorkerResponse::SingleLineResult { line_number, html } => {
                        let mut map = HashMap::new();
                        map.insert(line_number, html);
                        on_result.set(Some(map));
                    }
                    SyntaxWorkerResponse::CacheCleared => {
                        web_sys::console::log_1(&"Cache cleared".into());
                    }
                    SyntaxWorkerResponse::LanguageSet { language } => {
                        web_sys::console::log_1(&format!("Language set to: {}", language).into());
                    }
                    SyntaxWorkerResponse::CacheStats { size, is_analyzing } => {
                        web_sys::console::log_1(&format!("Cache: {} items, analyzing: {}", size, is_analyzing).into());
                    }
                    SyntaxWorkerResponse::Error { error } => {
                        on_error.set(Some(error));
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);

        self.worker.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();
    }

    /// Send message to worker
    pub fn post_message(&self, message: SyntaxWorkerMessage) -> Result<(), JsValue> {
        let js_message = serde_wasm_bindgen::to_value(&message)?;
        self.worker.post_message(&js_message)
    }

    /// Highlight multiple lines (batch)
    pub fn highlight_lines(&self, lines: Vec<(usize, String)>, language: Option<String>) -> Result<(), JsValue> {
        let lines_to_highlight: Vec<LineToHighlight> = lines
            .into_iter()
            .map(|(line_number, text)| LineToHighlight { line_number, text })
            .collect();

        self.post_message(SyntaxWorkerMessage::HighlightLines {
            lines: lines_to_highlight,
            language,
        })
    }

    /// Highlight single line (for real-time typing)
    pub fn highlight_single_line(&self, line_number: usize, text: String, language: Option<String>) -> Result<(), JsValue> {
        self.post_message(SyntaxWorkerMessage::HighlightSingleLine {
            line_number,
            text,
            language,
        })
    }

    /// Clear cache
    pub fn clear_cache(&self) -> Result<(), JsValue> {
        self.post_message(SyntaxWorkerMessage::ClearCache)
    }

    /// Set language
    pub fn set_language(&self, language: String) -> Result<(), JsValue> {
        self.post_message(SyntaxWorkerMessage::SetLanguage { language })
    }

    /// Get cache stats
    pub fn get_cache_stats(&self) -> Result<(), JsValue> {
        self.post_message(SyntaxWorkerMessage::GetCacheStats)
    }

    /// Terminate worker
    pub fn terminate(&self) {
        self.worker.terminate();
    }
}
