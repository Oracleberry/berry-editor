//! DOM Rendering Verification Tests
//!
//! Tests that verify the actual DOM structure and visibility of rendered content.
//! Run with: wasm-pack test --headless --firefox

use berry_editor::core::virtual_editor::VirtualEditorPanel;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::HtmlElement;

wasm_bindgen_test_configure!(run_in_browser);

// ========================================
// DOM Structure Tests
// ========================================

#[wasm_bindgen_test]
async fn test_editor_pane_exists() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    let document = web_sys::window().unwrap().document().unwrap();
    let pane = document.query_selector(".berry-editor-pane").unwrap();

    assert!(pane.is_some(), "❌ Editor pane does not exist");

    let pane_el = pane.unwrap().dyn_into::<HtmlElement>().unwrap();
    assert!(pane_el.offset_height() > 0, "❌ Editor pane has zero height");
}

#[wasm_bindgen_test]
async fn test_file_creates_line_elements() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Load a test file
    let test_content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
    selected_file.set(Some(("/test.txt".to_string(), test_content.to_string())));

    // Wait for effects to run
    wait_for_render().await;
    wait_for_render().await; // Double wait for nested effects

    let document = web_sys::window().unwrap().document().unwrap();

    // Check for line elements - use getElementsByClassName instead
    let lines = document.get_elements_by_class_name("berry-editor-line");
    let line_count = lines.length();

    assert!(line_count > 0, "❌ No line elements created! Expected at least 1, got 0");

    web_sys::console::log_1(&format!("✅ Created {} line elements", line_count).into());
}

#[wasm_bindgen_test]
async fn test_line_elements_have_content() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Load a test file
    let test_content = "Hello World\nSecond Line";
    selected_file.set(Some(("/test.txt".to_string(), test_content.to_string())));

    wait_for_render().await;
    wait_for_render().await;

    let document = web_sys::window().unwrap().document().unwrap();

    // Get first line element
    let first_line = document.query_selector(".berry-editor-line").unwrap();
    assert!(first_line.is_some(), "❌ First line element does not exist");

    let first_line_el = first_line.unwrap();
    let text_content = first_line_el.text_content().unwrap_or_default();

    assert!(!text_content.is_empty(), "❌ First line has no text content");
    assert!(text_content.contains("Hello") || text_content.contains("Second"),
            "❌ Line content doesn't match. Got: '{}'", text_content);

    web_sys::console::log_1(&format!("✅ First line text: '{}'", text_content).into());
}

#[wasm_bindgen_test]
async fn test_line_elements_are_visible() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    let test_content = "Visible Line 1\nVisible Line 2\nVisible Line 3";
    selected_file.set(Some(("/test.txt".to_string(), test_content.to_string())));

    wait_for_render().await;
    wait_for_render().await;

    let document = web_sys::window().unwrap().document().unwrap();
    let window = web_sys::window().unwrap();

    let first_line = document.query_selector(".berry-editor-line").unwrap();
    assert!(first_line.is_some(), "❌ No line element found");

    let line_el = first_line.unwrap().dyn_into::<HtmlElement>().unwrap();

    // Check computed styles
    let styles = window.get_computed_style(&line_el).unwrap().unwrap();

    let display = styles.get_property_value("display").unwrap();
    let opacity = styles.get_property_value("opacity").unwrap();
    let visibility = styles.get_property_value("visibility").unwrap();
    let height = styles.get_property_value("height").unwrap();

    web_sys::console::log_1(&format!("Line styles - display: {}, opacity: {}, visibility: {}, height: {}",
                                     display, opacity, visibility, height).into());

    assert_ne!(display, "none", "❌ Line element has display:none");
    assert_ne!(visibility, "hidden", "❌ Line element has visibility:hidden");

    // Check position
    let rect = line_el.get_bounding_client_rect();
    web_sys::console::log_1(&format!("Line position - top: {}, left: {}, width: {}, height: {}",
                                     rect.top(), rect.left(), rect.width(), rect.height()).into());

    assert!(rect.width() > 0.0, "❌ Line has zero width: {}", rect.width());
    assert!(rect.height() > 0.0, "❌ Line has zero height: {}", rect.height());
}

#[wasm_bindgen_test]
async fn test_scroll_container_structure() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    let test_content = "Line 1\nLine 2\nLine 3";
    selected_file.set(Some(("/test.txt".to_string(), test_content.to_string())));

    wait_for_render().await;
    wait_for_render().await;

    let document = web_sys::window().unwrap().document().unwrap();

    // Check for scroll container
    let container = document.query_selector(".berry-editor-scroll-container").unwrap();
    assert!(container.is_some(), "❌ Scroll container does not exist");

    let container_el = container.unwrap().dyn_into::<HtmlElement>().unwrap();
    let child_count = container_el.children().length();

    web_sys::console::log_1(&format!("Scroll container has {} children", child_count).into());

    assert!(child_count > 0, "❌ Scroll container has no children");
}

#[wasm_bindgen_test]
async fn test_editor_pane_hierarchy() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    let test_content = "Test content";
    selected_file.set(Some(("/test.txt".to_string(), test_content.to_string())));

    wait_for_render().await;
    wait_for_render().await;

    let document = web_sys::window().unwrap().document().unwrap();

    // Check hierarchy: pane -> main -> scroll-container -> lines
    let pane = document.query_selector(".berry-editor-pane").unwrap();
    assert!(pane.is_some(), "❌ Editor pane missing");

    let pane_el = pane.unwrap();
    let pane_html = pane_el.inner_html();

    web_sys::console::log_1(&format!("Pane HTML (first 500 chars): {}",
                                     &pane_html.chars().take(500).collect::<String>()).into());

    // Verify expected elements exist in hierarchy
    assert!(pane_html.contains("berry-editor-main") ||
            pane_html.contains("empty-screen") ||
            pane_html.contains("berry-editor-scroll"),
            "❌ Unexpected pane structure");
}

#[wasm_bindgen_test]
async fn test_line_color_and_background() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    let test_content = "Colored Text";
    selected_file.set(Some(("/test.txt".to_string(), test_content.to_string())));

    wait_for_render().await;
    wait_for_render().await;

    let document = web_sys::window().unwrap().document().unwrap();
    let window = web_sys::window().unwrap();

    let first_line = document.query_selector(".berry-editor-line").unwrap();
    if let Some(line_el) = first_line {
        let line_html = line_el.dyn_into::<HtmlElement>().unwrap();
        let styles = window.get_computed_style(&line_html).unwrap().unwrap();

        let color = styles.get_property_value("color").unwrap();
        let bg_color = styles.get_property_value("background-color").unwrap();

        web_sys::console::log_1(&format!("Line color: {}, background: {}", color, bg_color).into());

        // Ensure color and background are different
        assert_ne!(color, bg_color, "❌ Text color same as background - text invisible!");
    }
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
