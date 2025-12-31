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

#[wasm_bindgen_test]
async fn test_tab_close_button_exists() {
    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // Open a file
    selected_file.set(Some(("/test.rs".to_string(), "fn main() {}".to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();

    // Check close button exists
    let close_button = document
        .query_selector(".berry-tab.active button")
        .unwrap();

    assert!(close_button.is_some(), "Close button should exist in active tab");

    let button = close_button.unwrap();
    let button_text = button.text_content().unwrap_or_default();
    assert_eq!(button_text.trim(), "×", "Close button should display ×");

    leptos::logging::log!("✅ Tab close button test completed");
}
