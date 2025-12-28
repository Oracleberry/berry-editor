//! Complete Diagnostic Test
//!
//! This test diagnoses every step from file selection to rendering
//! Run with: wasm-pack test --headless --chrome --test diagnostic_test

use berry_editor::core::virtual_editor::VirtualEditorPanel;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::HtmlElement;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn diagnostic_full_flow() {
    web_sys::console::log_1(&"========== DIAGNOSTIC TEST START ==========".into());

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    let document = web_sys::window().unwrap().document().unwrap();
    let window = web_sys::window().unwrap();

    // ========== STEP 1: Check Initial State ==========
    web_sys::console::log_1(&"STEP 1: Checking initial state...".into());

    let editor_main = document.query_selector(".berry-editor-main").unwrap();
    if editor_main.is_some() {
        web_sys::console::log_1(&"✅ Editor main container exists".into());
    } else {
        web_sys::console::log_1(&"❌ Editor main container NOT found".into());
        panic!("Editor main container not found");
    }

    let tab_bar = document.query_selector(".berry-editor-tab-bar").unwrap();
    if tab_bar.is_some() {
        web_sys::console::log_1(&"✅ Tab bar exists".into());
    } else {
        web_sys::console::log_1(&"❌ Tab bar NOT found".into());
    }

    let pane = document.query_selector(".berry-editor-pane").unwrap();
    if pane.is_some() {
        let pane_el = pane.clone().unwrap().dyn_into::<HtmlElement>().unwrap();
        let rect = pane_el.get_bounding_client_rect();
        web_sys::console::log_1(&format!("✅ Editor pane exists: top={}, height={}", rect.top(), rect.height()).into());
    } else {
        web_sys::console::log_1(&"❌ Editor pane NOT found".into());
    }

    // ========== STEP 2: Simulate File Selection ==========
    web_sys::console::log_1(&"STEP 2: Simulating file selection...".into());

    let test_content = "fn main() {\n    println!(\"Hello, world!\");\n}\n";
    web_sys::console::log_1(&format!("Setting file: /test.rs with {} chars", test_content.len()).into());

    selected_file.set(Some(("/test.rs".to_string(), test_content.to_string())));

    web_sys::console::log_1(&"Waiting for effects to propagate...".into());
    wait_for_render().await;
    wait_for_render().await;
    wait_for_render().await;

    // ========== STEP 3: Check Tab Creation ==========
    web_sys::console::log_1(&"STEP 3: Checking tab creation...".into());

    let tabs = document.get_elements_by_class_name("berry-editor-tab");
    let tab_count = tabs.length();
    web_sys::console::log_1(&format!("Tab count: {}", tab_count).into());

    if tab_count > 0 {
        web_sys::console::log_1(&"✅ Tab was created".into());
        let first_tab = tabs.item(0).unwrap();
        let tab_text = first_tab.text_content().unwrap_or_default();
        web_sys::console::log_1(&format!("Tab text: '{}'", tab_text).into());
    } else {
        web_sys::console::log_1(&"❌ NO TABS CREATED!".into());
    }

    // ========== STEP 4: Check Editor Pane Content ==========
    web_sys::console::log_1(&"STEP 4: Checking editor pane content...".into());

    let pane = document.query_selector(".berry-editor-pane").unwrap();
    if let Some(pane_el) = pane {
        let pane_html = pane_el.dyn_into::<HtmlElement>().unwrap();
        let inner_html = pane_html.inner_html();
        web_sys::console::log_1(&format!("Pane inner HTML length: {}", inner_html.len()).into());

        if inner_html.len() < 100 {
            web_sys::console::log_1(&format!("Pane HTML: {}", inner_html).into());
        } else {
            web_sys::console::log_1(&format!("Pane HTML (first 200 chars): {}", &inner_html[..200]).into());
        }
    }

    // ========== STEP 5: Check Scroll Container ==========
    web_sys::console::log_1(&"STEP 5: Checking scroll container...".into());

    let scroll_container = document.query_selector(".berry-editor-scroll-container").unwrap();
    if let Some(container) = scroll_container {
        let container_el = container.dyn_into::<HtmlElement>().unwrap();
        let rect = container_el.get_bounding_client_rect();
        web_sys::console::log_1(&format!("✅ Scroll container: top={}, left={}, width={}, height={}",
            rect.top(), rect.left(), rect.width(), rect.height()).into());

        let child_count = container_el.children().length();
        web_sys::console::log_1(&format!("Scroll container children: {}", child_count).into());
    } else {
        web_sys::console::log_1(&"❌ Scroll container NOT found".into());
    }

    // ========== STEP 6: Check Viewport ==========
    web_sys::console::log_1(&"STEP 6: Checking viewport...".into());

    let viewport = document.query_selector(".berry-editor-viewport").unwrap();
    if let Some(viewport_el) = viewport {
        let vp = viewport_el.dyn_into::<HtmlElement>().unwrap();
        let rect = vp.get_bounding_client_rect();
        let styles = window.get_computed_style(&vp).unwrap().unwrap();
        let transform = styles.get_property_value("transform").unwrap();

        web_sys::console::log_1(&format!("✅ Viewport: top={}, left={}, width={}, height={}",
            rect.top(), rect.left(), rect.width(), rect.height()).into());
        web_sys::console::log_1(&format!("Viewport transform: {}", transform).into());

        let child_count = vp.children().length();
        web_sys::console::log_1(&format!("Viewport children: {}", child_count).into());
    } else {
        web_sys::console::log_1(&"❌ Viewport NOT found".into());
    }

    // ========== STEP 7: Check Line Elements ==========
    web_sys::console::log_1(&"STEP 7: Checking line elements...".into());

    let lines = document.get_elements_by_class_name("berry-editor-line");
    let line_count = lines.length();
    web_sys::console::log_1(&format!("Line element count: {}", line_count).into());

    if line_count > 0 {
        web_sys::console::log_1(&"✅ Line elements created".into());

        // Check first line
        let first_line = lines.item(0).unwrap().dyn_into::<HtmlElement>().unwrap();
        let rect = first_line.get_bounding_client_rect();
        let text = first_line.text_content().unwrap_or_default();
        let styles = window.get_computed_style(&first_line).unwrap().unwrap();

        web_sys::console::log_1(&format!("First line position: top={}, left={}, width={}, height={}",
            rect.top(), rect.left(), rect.width(), rect.height()).into());
        web_sys::console::log_1(&format!("First line text: '{}'", text).into());
        web_sys::console::log_1(&format!("First line display: {}", styles.get_property_value("display").unwrap()).into());
        web_sys::console::log_1(&format!("First line visibility: {}", styles.get_property_value("visibility").unwrap()).into());

        // Check if visible in viewport
        let viewport_height = window.inner_height().unwrap().as_f64().unwrap();
        if rect.top() >= 0.0 && rect.top() < viewport_height {
            web_sys::console::log_1(&format!("✅ First line IS in viewport (0-{}px)", viewport_height).into());
        } else {
            web_sys::console::log_1(&format!("❌ First line is OUTSIDE viewport! Position: {}px, Viewport: 0-{}px",
                rect.top(), viewport_height).into());
        }
    } else {
        web_sys::console::log_1(&"❌ NO LINE ELEMENTS CREATED!".into());
    }

    // ========== STEP 8: DOM Tree Structure ==========
    web_sys::console::log_1(&"STEP 8: Checking DOM tree structure...".into());

    if let Some(pane) = document.query_selector(".berry-editor-pane").unwrap() {
        let pane_el = pane.dyn_into::<HtmlElement>().unwrap();
        print_dom_tree(&pane_el, 0);
    }

    web_sys::console::log_1(&"========== DIAGNOSTIC TEST COMPLETE ==========".into());

    // Final assertion
    assert!(line_count > 0, "❌ CRITICAL: No line elements were created");
}

fn print_dom_tree(element: &HtmlElement, depth: usize) {
    let indent = "  ".repeat(depth);
    let tag_name = element.tag_name().to_lowercase();
    let class_name = element.class_name();

    let info = if class_name.is_empty() {
        format!("{}<{}>", indent, tag_name)
    } else {
        format!("{}<{} class='{}'>", indent, tag_name, class_name)
    };

    web_sys::console::log_1(&info.into());

    if depth < 3 {  // Limit depth to avoid too much output
        let children = element.children();
        for i in 0..children.length().min(5) {  // Show first 5 children
            if let Some(child) = children.item(i) {
                if let Ok(child_el) = child.dyn_into::<HtmlElement>() {
                    print_dom_tree(&child_el, depth + 1);
                }
            }
        }
        if children.length() > 5 {
            web_sys::console::log_1(&format!("{}  ... and {} more children", indent, children.length() - 5).into());
        }
    }
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
