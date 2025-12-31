//! Tab Display Integration Test
//!
//! Tests that file tabs are properly displayed when files are opened

use berry_editor::core::virtual_editor::VirtualEditorPanel;
use leptos::prelude::*;
use wasm_bindgen_test::*;

mod test_helpers;
use test_helpers::{get_test_document, wait_for_render};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_tab_displays_when_file_opened() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    let document = get_test_document();

    // Initially, "No file open" should be displayed
    let tab_bar = document
        .query_selector(".berry-editor-tabs")
        .unwrap()
        .expect("Tab bar should exist");

    let initial_text = tab_bar.text_content().unwrap_or_default();
    assert!(
        initial_text.contains("No file open"),
        "Should show 'No file open' when no file is selected"
    );

    // Open a file
    selected_file.set(Some(("/src/main.rs".to_string(), "fn main() {}".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    // Tab should now display the file name
    let tab = document
        .query_selector(".berry-tab.active")
        .unwrap()
        .expect("Active tab should exist");

    let tab_text = tab.text_content().unwrap_or_default();
    assert_eq!(tab_text, "main.rs", "Tab should display file name");

    leptos::logging::log!("✅ Tab display test completed");
}


#[wasm_bindgen_test]
async fn test_tab_bar_exists() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    let document = get_test_document();

    // Check tab bar exists
    let tab_bar = document
        .query_selector(".berry-editor-tabs")
        .unwrap();

    assert!(tab_bar.is_some(), "Tab bar should exist");

    leptos::logging::log!("✅ Tab bar existence test completed");
}

