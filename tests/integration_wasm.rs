//! WASM Integration Tests
//!
//! Minimal UI verification tests that run in the browser.
//! These tests verify critical UI behaviors without testing implementation details.
//!
//! Run with: wasm-pack test --headless --firefox

use berry_editor::core::virtual_editor::VirtualEditorPanel;
use berry_editor::buffer::TextBuffer;
use leptos::prelude::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ========================================
// Core Editor Initialization
// ========================================

#[wasm_bindgen_test]
async fn test_editor_initialization() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    // Wait for render
    wait_for_render().await;

    let document = web_sys::window().unwrap().document().unwrap();

    // Verify main structure exists
    assert!(
        document.query_selector(".berry-editor-main").unwrap().is_some(),
        "Main editor container should exist"
    );
}

// ========================================
// File Loading
// ========================================

#[wasm_bindgen_test]
async fn test_file_loading_creates_tab() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Load a file
    selected_file.set(Some(("/test.rs".to_string(), "fn main() {}".to_string())));

    // Wait for effect to execute
    wait_for_render().await;

    let document = web_sys::window().unwrap().document().unwrap();

    // Verify tab was created (check for tab bar, not specific tab structure)
    let tab_bar = document.query_selector(".berry-editor-tab-bar").unwrap();
    assert!(tab_bar.is_some(), "Tab bar should exist after loading file");
}

// ========================================
// Virtual Scrolling
// ========================================

#[wasm_bindgen_test]
async fn test_virtual_scrolling_with_large_file() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Create a large file (1000 lines)
    let large_content = (0..1000)
        .map(|i| format!("Line {} content", i + 1))
        .collect::<Vec<_>>()
        .join("\n");

    selected_file.set(Some(("/large.txt".to_string(), large_content)));

    wait_for_render().await;

    // Virtual scrolling should be active - we don't verify exact DOM structure,
    // just that the component rendered
    let document = web_sys::window().unwrap().document().unwrap();
    assert!(
        document.query_selector(".berry-editor-pane").unwrap().is_some(),
        "Editor pane should exist for large file"
    );
}

// ========================================
// Buffer Operations (Unit Tests)
// ========================================

#[wasm_bindgen_test]
fn test_buffer_basic_operations() {
    let content = "Line 1\nLine 2\nLine 3";
    let buffer = TextBuffer::from_str(content);

    assert_eq!(buffer.len_lines(), 3);
    assert_eq!(buffer.to_string(), content);
}

#[wasm_bindgen_test]
fn test_buffer_insert_text() {
    let mut buffer = TextBuffer::from_str("Hello");

    buffer.insert(5, " World");
    assert_eq!(buffer.to_string(), "Hello World");
}

#[wasm_bindgen_test]
fn test_buffer_delete_text() {
    let mut buffer = TextBuffer::from_str("Hello World");

    buffer.remove(5, 11);
    assert_eq!(buffer.to_string(), "Hello");
}

#[wasm_bindgen_test]
fn test_buffer_line_operations() {
    let content = "First\nSecond\nThird";
    let buffer = TextBuffer::from_str(content);

    assert_eq!(buffer.line(0), Some("First\n".into()));
    assert_eq!(buffer.line(1), Some("Second\n".into()));
    assert_eq!(buffer.line(2), Some("Third".into()));
    assert_eq!(buffer.line(3), None);
}

// ========================================
// Helper Functions
// ========================================

async fn wait_for_render() {
    wasm_bindgen_futures::JsFuture::from(js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 100)
            .unwrap();
    }))
    .await
    .unwrap();
}

// ========================================
// Cursor Position Tests (Logic Only)
// ========================================

#[wasm_bindgen_test]
fn test_cursor_position_calculation() {
    const LINE_HEIGHT: f64 = 20.0;
    const CHAR_WIDTH: f64 = 8.4;

    // Test line calculation
    let click_y = 150.0;
    let scroll_top = 100.0;
    let y_absolute = click_y + scroll_top;
    let line = (y_absolute / LINE_HEIGHT).floor() as usize;

    assert_eq!(line, 12); // (250 / 20) = 12

    // Test column calculation
    let click_x = 100.0_f64;
    let padding = 10.0_f64;
    let col = ((click_x - padding).max(0.0) / CHAR_WIDTH).round() as usize;

    assert_eq!(col, 11); // ((100 - 10) / 8.4) ≈ 10.7 → 11
}

#[wasm_bindgen_test]
fn test_cursor_position_with_scroll() {
    const LINE_HEIGHT: f64 = 20.0;

    let scroll_top = 1000.0;
    let click_y = 10.0;
    let y_absolute = click_y + scroll_top;
    let line = (y_absolute / LINE_HEIGHT).floor() as usize;

    assert_eq!(line, 50); // (1010 / 20) = 50
}

// ========================================
// Edit Mode Tests (Logic Only)
// ========================================

#[wasm_bindgen_test]
fn test_edit_mode_cursor_calculation() {
    let content = "fn main() {\n    println!(\"Hello\");\n}";
    let buffer = TextBuffer::from_str(content);

    // Simulate click on line 1, column 4
    const LINE_HEIGHT: f64 = 20.0;
    const CHAR_WIDTH: f64 = 8.4;

    let target_line = 1;
    let target_col = 4;
    let click_y = (target_line as f64) * LINE_HEIGHT;
    let click_x = 50.0_f64 + (target_col as f64) * CHAR_WIDTH; // 50px line numbers

    let calculated_line = (click_y / LINE_HEIGHT).floor() as usize;
    let calculated_col = ((click_x - 50.0_f64).max(0.0) / CHAR_WIDTH).round() as usize;

    assert_eq!(calculated_line, target_line);
    assert_eq!(calculated_col, target_col);

    // Verify line exists in buffer
    assert!(buffer.line(calculated_line).is_some());
}
