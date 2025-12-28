//! Web Worker Integration for Background Processing
//!
//! Runs heavy computations (indexing, parsing) in background threads
//! without blocking the UI

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Worker, MessageEvent};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerMessage {
    IndexWorkspace { path: String, api_endpoint: Option<String> },
    SearchSymbols { query: String, api_endpoint: Option<String> },
    GetStatus,
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerResponse {
    Ready,
    Progress { data: ProgressData },
    Status { data: StatusData },
    SearchResult { symbols: Vec<crate::tauri_bindings::Symbol> },
    CallTauri { command: String, args: serde_json::Value },
    Error { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressData {
    pub status: String,
    pub message: String,
    pub is_indexing: bool,
    pub total_symbols: usize,
    pub processed_files: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusData {
    pub is_indexing: bool,
    pub total_symbols: usize,
    pub processed_files: usize,
    pub errors: Vec<ErrorData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorData {
    pub message: String,
    pub timestamp: u64,
}

/// Web Worker wrapper for background indexing
pub struct IndexerWorker {
    worker: Worker,
    on_progress: RwSignal<Option<ProgressData>>,
    on_error: RwSignal<Option<String>>,
}

impl IndexerWorker {
    /// Create a new worker instance
    pub fn new(
        on_progress: RwSignal<Option<ProgressData>>,
        on_error: RwSignal<Option<String>>,
    ) -> Result<Self, JsValue> {
        let worker = Worker::new("/indexer-worker.js")?;

        Ok(Self {
            worker,
            on_progress,
            on_error,
        })
    }

    /// Set up message handler
    pub fn setup_handlers(&self) {
        let on_progress = self.on_progress;
        let on_error = self.on_error;

        let onmessage_callback = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Ok(response) = serde_wasm_bindgen::from_value::<WorkerResponse>(event.data()) {
                match response {
                    WorkerResponse::Ready => {
                        web_sys::console::log_1(&"Worker ready".into());
                    }
                    WorkerResponse::Progress { data } => {
                        on_progress.set(Some(data));
                    }
                    WorkerResponse::Status { .. } => {
                        // Handle status if needed
                    }
                    WorkerResponse::SearchResult { .. } => {
                        // Handle search results if needed
                    }
                    WorkerResponse::CallTauri { command: _, args: _ } => {
                        // Note: Tauri calls handled separately via events
                        web_sys::console::log_1(&"Tauri call requested".into());
                    }
                    WorkerResponse::Error { error } => {
                        on_error.set(Some(error));
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);

        self.worker.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();
    }

    /// Send message to worker
    pub fn post_message(&self, message: WorkerMessage) -> Result<(), JsValue> {
        let js_message = serde_wasm_bindgen::to_value(&message)?;
        self.worker.post_message(&js_message)
    }

    /// Send result back to worker
    pub fn send_result(&self, result: serde_json::Value) -> Result<(), JsValue> {
        let js_value = serde_wasm_bindgen::to_value(&result)?;
        self.worker.post_message(&js_value)
    }

    /// Index workspace
    pub fn index_workspace(&self, path: String) -> Result<(), JsValue> {
        self.post_message(WorkerMessage::IndexWorkspace {
            path,
            api_endpoint: None,
        })
    }

    /// Search symbols
    pub fn search_symbols(&self, query: String) -> Result<(), JsValue> {
        self.post_message(WorkerMessage::SearchSymbols {
            query,
            api_endpoint: None,
        })
    }

    /// Get status
    pub fn get_status(&self) -> Result<(), JsValue> {
        self.post_message(WorkerMessage::GetStatus)
    }

    /// Cancel indexing
    pub fn cancel(&self) -> Result<(), JsValue> {
        self.post_message(WorkerMessage::Cancel)
    }

    /// Terminate worker
    pub fn terminate(&self) {
        self.worker.terminate();
    }
}
