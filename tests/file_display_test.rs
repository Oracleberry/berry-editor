//! File Display Integration Test
//!
//! Tests the complete flow from file selection to visible rendering
//! Run with: wasm-pack test --headless --firefox

use berry_editor::core::virtual_editor::VirtualEditorPanel;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::HtmlElement;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_file_selection_creates_visible_content() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Simulate file selection
    let test_content = "fn main() {\n    println!(\"Hello, world!\");\n}\n";
    selected_file.set(Some(("/test.rs".to_string(), test_content.to_string())));

    // Wait for effects to propagate
    wait_for_render().await;
    wait_for_render().await;
    wait_for_render().await;

    let document = web_sys::window().unwrap().document().unwrap();
    let window = web_sys::window().unwrap();

    // Step 1: Verify tab was created
    let tab_bar = document.query_selector(".berry-editor-tab-bar").unwrap();
    assert!(tab_bar.is_some(), "❌ Tab bar not found");

    let tab = document.query_selector(".berry-editor-tab").unwrap();
    assert!(tab.is_some(), "❌ Tab not created after file selection");

    web_sys::console::log_1(&"✅ Tab created successfully".into());

    // Step 2: Verify editor pane exists
    let pane = document.query_selector(".berry-editor-pane").unwrap();
    assert!(pane.is_some(), "❌ Editor pane not found");

    let pane_el = pane.unwrap().dyn_into::<HtmlElement>().unwrap();
    let pane_rect = pane_el.get_bounding_client_rect();

    web_sys::console::log_1(&format!("✅ Editor pane: top={}, height={}",
        pane_rect.top(), pane_rect.height()).into());

    assert!(pane_rect.height() > 0.0, "❌ Editor pane has zero height");

    // Step 3: Verify line elements were created
    let lines = document.get_elements_by_class_name("berry-editor-line");
    let line_count = lines.length();

    assert!(line_count > 0, "❌ No line elements created. Expected at least 1, got 0");
    web_sys::console::log_1(&format!("✅ Created {} line elements", line_count).into());

    // Step 4: Verify first line has content
    let first_line = document.query_selector(".berry-editor-line").unwrap();
    assert!(first_line.is_some(), "❌ First line element not found");

    let first_line_el = first_line.unwrap();
    let text_content = first_line_el.text_content().unwrap_or_default();

    assert!(!text_content.is_empty(), "❌ First line has no text content");
    web_sys::console::log_1(&format!("✅ First line text: '{}'", text_content).into());

    // Step 5: CRITICAL - Verify lines are in visible viewport
    let first_line_html = first_line_el.dyn_into::<HtmlElement>().unwrap();
    let line_rect = first_line_html.get_bounding_client_rect();

    web_sys::console::log_1(&format!(
        "Line position - top: {}, left: {}, width: {}, height: {}",
        line_rect.top(), line_rect.left(), line_rect.width(), line_rect.height()
    ).into());

    // Check if line is within viewport
    let viewport_height = window.inner_height().unwrap().as_f64().unwrap();

    assert!(line_rect.top() >= 0.0 && line_rect.top() < viewport_height,
        "❌ CRITICAL: First line is positioned at {}px, outside visible viewport (0-{}px)",
        line_rect.top(), viewport_height);

    assert!(line_rect.width() > 0.0, "❌ Line has zero width");
    assert!(line_rect.height() > 0.0, "❌ Line has zero height");

    // Step 6: Verify visibility styles
    let styles = window.get_computed_style(&first_line_html).unwrap().unwrap();
    let display = styles.get_property_value("display").unwrap();
    let visibility = styles.get_property_value("visibility").unwrap();

    assert_ne!(display, "none", "❌ Line has display:none");
    assert_ne!(visibility, "hidden", "❌ Line has visibility:hidden");

    web_sys::console::log_1(&"✅ ALL TESTS PASSED - File content is visible!".into());
}

#[wasm_bindgen_test]
async fn test_viewport_position() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    let test_content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
    selected_file.set(Some(("/test.txt".to_string(), test_content.to_string())));

    wait_for_render().await;
    wait_for_render().await;
    wait_for_render().await;

    let document = web_sys::window().unwrap().document().unwrap();

    // Check viewport element
    let viewport = document.query_selector(".berry-editor-viewport").unwrap();
    assert!(viewport.is_some(), "❌ Viewport element not found");

    let viewport_el = viewport.unwrap().dyn_into::<HtmlElement>().unwrap();
    let viewport_rect = viewport_el.get_bounding_client_rect();

    web_sys::console::log_1(&format!(
        "Viewport position - top: {}, left: {}, width: {}, height: {}",
        viewport_rect.top(), viewport_rect.left(), viewport_rect.width(), viewport_rect.height()
    ).into());

    // Viewport should start at top:0 when scroll is 0
    assert!(viewport_rect.top() >= 0.0 && viewport_rect.top() < 50.0,
        "❌ Viewport is at {}px, expected near 0px", viewport_rect.top());

    web_sys::console::log_1(&"✅ Viewport positioned correctly".into());
}

#[wasm_bindgen_test]
async fn test_scroll_container_structure() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    let test_content = "Test Line 1\nTest Line 2\nTest Line 3";
    selected_file.set(Some(("/test.txt".to_string(), test_content.to_string())));

    wait_for_render().await;
    wait_for_render().await;

    let document = web_sys::window().unwrap().document().unwrap();

    // Check scroll container
    let container = document.query_selector(".berry-editor-scroll-container").unwrap();
    assert!(container.is_some(), "❌ Scroll container not found");

    let container_el = container.unwrap().dyn_into::<HtmlElement>().unwrap();
    let container_rect = container_el.get_bounding_client_rect();

    web_sys::console::log_1(&format!(
        "Scroll container - top: {}, left: {}, width: {}, height: {}",
        container_rect.top(), container_rect.left(), container_rect.width(), container_rect.height()
    ).into());

    // Check if container has children
    let child_count = container_el.children().length();
    web_sys::console::log_1(&format!("Scroll container has {} children", child_count).into());

    assert!(child_count > 0, "❌ Scroll container has no children");

    web_sys::console::log_1(&"✅ Scroll container structure correct".into());
}

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
