//! BerryEditor - 100% Rust Code Editor
//!
//! A fully-featured code editor built entirely in Rust using Leptos and WASM.
//! No JavaScript required!

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlElement};

mod components;
pub mod components_tauri;
pub mod editor;
pub mod editor_lsp;
pub mod file_tree;
pub mod file_tree_tauri;
mod syntax;
pub mod buffer;
mod lsp;
pub mod lsp_client;
mod cursor;
mod minimap;
mod search;
mod git;

// Common utilities (zero duplication)
pub mod common;

// Tauri bindings for native file access
pub mod tauri_bindings;
pub mod tauri_bindings_search;

// ✅ Web Workers for background processing
pub mod web_worker;
pub mod syntax_worker;  // ✅ Strategy 1: Non-blocking syntax analysis
pub mod tree_sitter_engine;  // ✅ Strategy 2: Deep contextual analysis
// pub mod webgpu_renderer;  // ✅ Strategy 4: GPU-accelerated DOM-free rendering (requires web-sys WebGPU support)

// Core modules (Editor Engine)
pub mod core;

// Phase 2: Search functionality
pub mod search_panel;

// Phase 1: High-performance rendering
pub mod virtual_scroll;
pub mod debounce;
pub mod canvas_renderer;
pub mod highlight_job;  // ✅ IntelliJ Pro: Async syntax highlighting

// Phase 1: LSP UI Integration
pub mod lsp_ui;
pub mod completion_widget;
pub mod diagnostics_panel;
pub mod hover_tooltip;

// Phase 5: UX Polishing
pub mod command_palette;

// Phase 2: Debugger Integration
pub mod debugger;

// Phase 3: Refactoring Integration
pub mod refactoring;

// Phase 4: Git UI Integration (disabled in WASM - requires std::time)
#[cfg(not(target_arch = "wasm32"))]
pub mod git_ui;

use components::EditorApp;
use components_tauri::EditorAppTauri;
use file_tree::get_mock_file_tree;

/// Test helper: Get mock file tree data for testing
#[wasm_bindgen]
pub fn get_test_file_tree() -> JsValue {
    let files = get_mock_file_tree();
    serde_wasm_bindgen::to_value(&files).unwrap()
}

/// Get document from global scope (fallback for test environments)
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = globalThis, js_name = document)]
    static DOCUMENT: web_sys::Document;
}

/// Initialize the BerryEditor WASM application
/// This is called automatically when WASM loads (via #[wasm_bindgen(start)])
#[wasm_bindgen(start)]
pub fn init_berry_editor() {
    // Set up better panic messages in development
    console_error_panic_hook::set_once();


    // Get the root element
    // In test environments (like jsdom), web_sys::window() might not work properly
    // So we try to get the document directly from JavaScript global scope
    let document = match window() {
        Some(win) => win.document().expect("no document"),
        None => {
            // Fallback for test environments: access document from global scope
            DOCUMENT.clone()
        }
    };

    let root = document
        .get_element_by_id("berry-editor-wasm-root")
        .expect("berry-editor-wasm-root element not found")
        .dyn_into::<HtmlElement>()
        .expect("root element is not an HtmlElement");


    // Clear loading message
    root.set_inner_html("");


    // Mount the Leptos app to the specific element
    // Use Tauri version if available, otherwise use Web version
    let mount_handle = leptos::mount::mount_to(root.clone(), || {

        if tauri_bindings::is_tauri_context() {
            view! { <EditorAppTauri/> }.into_any()
        } else {
            view! { <EditorApp/> }.into_any()
        }
    });

    mount_handle.forget();

}
