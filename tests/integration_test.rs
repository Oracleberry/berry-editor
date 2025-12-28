use wasm_bindgen_test::*;
use web_sys::window;

wasm_bindgen_test_configure!(run_in_browser);

fn setup_root_element() {
    let document = window().unwrap().document().unwrap();

    // Remove existing root if present
    if let Some(existing) = document.get_element_by_id("berry-editor-wasm-root") {
        existing.remove();
    }

    let root = document.create_element("div").unwrap();
    root.set_id("berry-editor-wasm-root");
    document.body().unwrap().append_child(&root).unwrap();
}

#[wasm_bindgen_test]
fn test_init_berry_editor_mounts_ui() {
    // Setup: Create root element
    setup_root_element();

    // Execute: Call init_berry_editor
    berry_editor::init_berry_editor();

    // Verify: Check that UI was mounted
    let document = window().unwrap().document().unwrap();
    let root_element = document
        .get_element_by_id("berry-editor-wasm-root")
        .expect("Root element should exist");

    let inner_html = root_element.inner_html();

    // Should NOT be empty after mounting
    assert!(!inner_html.is_empty(), "Root element should not be empty after init_berry_editor()");

    // Should contain the main container
    assert!(
        inner_html.contains("berry-editor-container"),
        "Should contain berry-editor-container class"
    );
}

#[wasm_bindgen_test]
fn test_root_element_not_found() {
    // This test verifies the error case when root element doesn't exist
    // The function should panic with a clear error message

    // Remove root element if it exists
    let document = window().unwrap().document().unwrap();
    if let Some(root) = document.get_element_by_id("berry-editor-wasm-root") {
        root.remove();
    }

    // This should panic
    // Note: wasm-bindgen-test doesn't support should_panic yet, so we skip this test
}

#[wasm_bindgen_test]
fn test_file_tree_renders() {
    // Setup
    setup_root_element();

    // Execute
    berry_editor::init_berry_editor();

    // Verify file tree exists
    let document = window().unwrap().document().unwrap();
    let root_element = document.get_element_by_id("berry-editor-wasm-root").unwrap();
    let inner_html = root_element.inner_html();

    assert!(
        inner_html.contains("FILE EXPLORER") || inner_html.contains("EXPLORER"),
        "Should contain file explorer header"
    );
}

#[wasm_bindgen_test]
fn test_editor_panel_renders() {
    // Setup
    setup_root_element();

    // Execute
    berry_editor::init_berry_editor();

    // Verify editor panel exists
    let document = window().unwrap().document().unwrap();
    let root_element = document.get_element_by_id("berry-editor-wasm-root").unwrap();
    let inner_html = root_element.inner_html();

    assert!(
        inner_html.contains("berry-editor-main-area") || inner_html.contains("berry-editor-main"),
        "Should contain editor main area"
    );
}

#[wasm_bindgen_test]
fn test_status_bar_renders() {
    // Setup
    setup_root_element();

    // Execute
    berry_editor::init_berry_editor();

    // Verify status bar exists
    let document = window().unwrap().document().unwrap();
    let root_element = document.get_element_by_id("berry-editor-wasm-root").unwrap();
    let inner_html = root_element.inner_html();

    assert!(
        inner_html.contains("berry-editor-status-bar"),
        "Should contain status bar"
    );

    assert!(
        inner_html.contains("BerryEditor") || inner_html.contains("100% Rust"),
        "Status bar should contain BerryEditor branding"
    );
}

#[wasm_bindgen_test]
fn test_file_tree_contains_mock_files() {
    // Setup
    setup_root_element();

    // Execute
    berry_editor::init_berry_editor();

    // Wait a bit for Leptos to render
    let document = window().unwrap().document().unwrap();
    let root_element = document.get_element_by_id("berry-editor-wasm-root").unwrap();
    let inner_html = root_element.inner_html();

    web_sys::console::log_1(&format!("HTML length: {}", inner_html.len()).into());
    web_sys::console::log_1(&format!("First 500 chars: {}", &inner_html[..inner_html.len().min(500)]).into());

    // Verify mock files are present
    assert!(
        inner_html.contains("src"),
        "File tree should contain 'src' folder. HTML: {}",
        &inner_html[..inner_html.len().min(1000)]
    );

    assert!(
        inner_html.contains("Cargo.toml"),
        "File tree should contain 'Cargo.toml' file. HTML: {}",
        &inner_html[..inner_html.len().min(1000)]
    );

    assert!(
        inner_html.contains("index.html"),
        "File tree should contain 'index.html' file. HTML: {}",
        &inner_html[..inner_html.len().min(1000)]
    );

    assert!(
        inner_html.contains("README.md"),
        "File tree should contain 'README.md' file. HTML: {}",
        &inner_html[..inner_html.len().min(1000)]
    );
}

#[wasm_bindgen_test]
fn test_file_tree_has_icons() {
    // Setup
    setup_root_element();

    // Execute
    berry_editor::init_berry_editor();

    // Verify file icon elements exist (icon content is rendered via reactive closures)
    let document = window().unwrap().document().unwrap();
    let root_element = document.get_element_by_id("berry-editor-wasm-root").unwrap();
    let inner_html = root_element.inner_html();

    // Check for file icon span elements (the actual emojis are rendered by reactive closures)
    let has_icon_spans = inner_html.contains("berry-editor-file-icon");

    assert!(
        has_icon_spans,
        "File tree should contain file icon elements. HTML: {}",
        &inner_html[..inner_html.len().min(1000)]
    );
}
