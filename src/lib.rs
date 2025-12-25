//! BerryEditor - 100% Rust Code Editor
//!
//! A fully-featured code editor built entirely in Rust using Leptos and WASM.
//! No JavaScript required!

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlElement};

mod components;
mod editor;
mod file_tree;
mod syntax;
mod buffer;
mod lsp;
mod cursor;
mod minimap;
mod search;
mod git;

// Common utilities (zero duplication)
pub mod common;

// Phase 1: High-performance rendering
pub mod virtual_scroll;
pub mod debounce;
pub mod canvas_renderer;

// Phase 1: LSP UI Integration
pub mod lsp_ui;
pub mod completion_widget;
pub mod diagnostics_panel;

// Phase 2: Debugger Integration
pub mod debugger;

use components::EditorApp;

/// Initialize the BerryEditor WASM application
#[wasm_bindgen]
pub fn init_berry_editor() {
    // Set up better panic messages in development
    console_error_panic_hook::set_once();

    // Get the root element
    let document = window()
        .expect("no window")
        .document()
        .expect("no document");

    let root = document
        .get_element_by_id("berry-editor-wasm-root")
        .expect("berry-editor-wasm-root element not found")
        .dyn_into::<HtmlElement>()
        .expect("root element is not an HtmlElement");

    // Clear loading message
    root.set_inner_html("");

    // Mount the Leptos app to the specific element
    leptos::mount::mount_to(root, || view! { <EditorApp/> }).forget();
}
