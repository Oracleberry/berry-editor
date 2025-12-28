//! Editor Integration E2E Tests (Simplified)
//!
//! These tests verify the ACTUAL editor component behavior

use wasm_bindgen_test::*;
use web_sys::{window, HtmlTextAreaElement};
use wasm_bindgen::JsCast;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_editor_textarea_exists_in_real_app() {
    // This test assumes the editor is already mounted in index.html
    let document = window().unwrap().document().unwrap();

    // Try to find the hidden textarea that should exist
    let textarea_result = document.query_selector("textarea");

    assert!(
        textarea_result.is_ok(),
        "Should be able to query for textarea"
    );

    if let Some(textarea) = textarea_result.unwrap() {
        let textarea_el = textarea.dyn_into::<HtmlTextAreaElement>().unwrap();

        // Verify textarea can be focused
        textarea_el.focus().unwrap();

        // Verify it's now active
        let active = document.active_element().unwrap();
        assert_eq!(
            active.dyn_ref::<HtmlTextAreaElement>(),
            Some(&textarea_el),
            "Textarea should be focusable"
        );
    } else {
        panic!("No textarea found - editor may not be mounted properly");
    }
}

#[wasm_bindgen_test]
fn test_editor_textarea_input_clears_value() {
    let document = window().unwrap().document().unwrap();
    let textarea_opt = document.query_selector("textarea").unwrap();

    if let Some(textarea) = textarea_opt {
        let textarea_el = textarea.dyn_into::<HtmlTextAreaElement>().unwrap();

        // Focus the textarea
        textarea_el.focus().unwrap();

        // Set a value (simulate typing)
        textarea_el.set_value("test");

        // Trigger input event
        let input_event = web_sys::InputEvent::new("input").unwrap();
        textarea_el.dispatch_event(&input_event).unwrap();

        // Wait a bit for processing
        std::thread::sleep(std::time::Duration::from_millis(50));

        // The editor should clear the textarea after processing input
        // This is the critical behavior that indicates input is being handled
        assert_eq!(
            textarea_el.value(),
            "",
            "Textarea should be cleared after input processing (indicates input handler is working)"
        );
    } else {
        panic!("No textarea found");
    }
}

#[wasm_bindgen_test]
fn test_editor_japanese_input_clears() {
    let document = window().unwrap().document().unwrap();
    let textarea_opt = document.query_selector("textarea").unwrap();

    if let Some(textarea) = textarea_opt {
        let textarea_el = textarea.dyn_into::<HtmlTextAreaElement>().unwrap();

        textarea_el.focus().unwrap();

        // Simulate Japanese input
        textarea_el.set_value("こんにちは");
        let input_event = web_sys::InputEvent::new("input").unwrap();
        textarea_el.dispatch_event(&input_event).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(50));

        // Should be cleared, indicating Japanese input was processed
        assert_eq!(
            textarea_el.value(),
            "",
            "Japanese input should be processed and cleared"
        );
    } else {
        panic!("No textarea found");
    }
}

#[wasm_bindgen_test]
fn test_editor_sequential_inputs() {
    let document = window().unwrap().document().unwrap();
    let textarea_opt = document.query_selector("textarea").unwrap();

    if let Some(textarea) = textarea_opt {
        let textarea_el = textarea.dyn_into::<HtmlTextAreaElement>().unwrap();

        textarea_el.focus().unwrap();

        // Input 1
        textarea_el.set_value("a");
        textarea_el.dispatch_event(&web_sys::InputEvent::new("input").unwrap()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));

        // Input 2
        textarea_el.set_value("b");
        textarea_el.dispatch_event(&web_sys::InputEvent::new("input").unwrap()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));

        // Input 3 (Japanese)
        textarea_el.set_value("あ");
        textarea_el.dispatch_event(&web_sys::InputEvent::new("input").unwrap()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));

        // All should have been processed
        assert_eq!(
            textarea_el.value(),
            "",
            "All sequential inputs should be processed"
        );
    } else {
        panic!("No textarea found");
    }
}

#[wasm_bindgen_test]
fn test_editor_textarea_z_index() {
    let document = window().unwrap().document().unwrap();
    let textarea_opt = document.query_selector("textarea").unwrap();

    if let Some(textarea) = textarea_opt {
        let textarea_el = textarea.dyn_into::<HtmlTextAreaElement>().unwrap();

        let style = textarea_el.get_attribute("style").unwrap();

        // Verify z-index is high (999)
        assert!(
            style.contains("999"),
            "Textarea should have z-index 999. Style: {}",
            style
        );
    } else {
        panic!("No textarea found");
    }
}

#[wasm_bindgen_test]
fn test_editor_textarea_width() {
    let document = window().unwrap().document().unwrap();
    let textarea_opt = document.query_selector("textarea").unwrap();

    if let Some(textarea) = textarea_opt {
        let textarea_el = textarea.dyn_into::<HtmlTextAreaElement>().unwrap();

        let style = textarea_el.get_attribute("style").unwrap();

        // Verify width is sufficient (200px)
        assert!(
            style.contains("200px"),
            "Textarea should have width 200px. Style: {}",
            style
        );
    } else {
        panic!("No textarea found");
    }
}
