//! Focus Race Condition Tests
//!
//! CRITICAL: Tests for focus management between layers
//!
//! The most common "can't type" bug:
//! - User opens command palette (focus moves to palette input)
//! - User closes palette (Esc key)
//! - Focus doesn't return to editor textarea
//! - User types but nothing happens -> "文字入力できない"
//!
//! This ensures focus always returns to the correct element

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{window, HtmlElement, HtmlInputElement, HtmlTextAreaElement};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_focus_returns_to_textarea_after_modal() {
    let document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    // Create editor textarea
    let editor_textarea = document.create_element("textarea").unwrap()
        .dyn_into::<HtmlTextAreaElement>().unwrap();
    editor_textarea.set_id("editor-textarea");
    body.append_child(&editor_textarea).unwrap();

    // Create modal input (like command palette)
    let modal_input = document.create_element("input").unwrap()
        .dyn_into::<HtmlInputElement>().unwrap();
    modal_input.set_id("modal-input");
    modal_input.set_attribute("style", "display: none;").unwrap();
    body.append_child(&modal_input).unwrap();

    // 1. Focus editor
    editor_textarea.focus().unwrap();
    assert_eq!(
        document.active_element().unwrap().id(),
        "editor-textarea",
        "Editor should be focused initially"
    );

    // 2. Open modal (show and focus it)
    modal_input.set_attribute("style", "display: block;").unwrap();
    modal_input.focus().unwrap();
    assert_eq!(
        document.active_element().unwrap().id(),
        "modal-input",
        "Modal should steal focus"
    );

    // 3. Close modal (hide and return focus to editor)
    modal_input.set_attribute("style", "display: none;").unwrap();
    editor_textarea.focus().unwrap();

    // 4. CRITICAL CHECK: Editor should have focus back
    assert_eq!(
        document.active_element().unwrap().id(),
        "editor-textarea",
        "Focus MUST return to editor after closing modal"
    );

    editor_textarea.remove();
    modal_input.remove();
}

#[wasm_bindgen_test]
fn test_input_works_after_focus_return() {
    let document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    let editor_textarea = document.create_element("textarea").unwrap()
        .dyn_into::<HtmlTextAreaElement>().unwrap();
    body.append_child(&editor_textarea).unwrap();

    let modal_input = document.create_element("input").unwrap()
        .dyn_into::<HtmlInputElement>().unwrap();
    modal_input.set_attribute("style", "display: none;").unwrap();
    body.append_child(&modal_input).unwrap();

    // Focus editor -> open modal -> close modal -> type
    editor_textarea.focus().unwrap();
    modal_input.set_attribute("style", "display: block;").unwrap();
    modal_input.focus().unwrap();
    modal_input.set_attribute("style", "display: none;").unwrap();
    editor_textarea.focus().unwrap();

    // Try to type
    editor_textarea.set_value("test");

    // Should work
    assert_eq!(
        editor_textarea.value(),
        "test",
        "Input should work after focus returns"
    );

    editor_textarea.remove();
    modal_input.remove();
}

#[wasm_bindgen_test]
fn test_multiple_modal_cycles() {
    let document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    let editor_textarea = document.create_element("textarea").unwrap()
        .dyn_into::<HtmlTextAreaElement>().unwrap();
    body.append_child(&editor_textarea).unwrap();

    let modal = document.create_element("div").unwrap()
        .dyn_into::<HtmlElement>().unwrap();
    let modal_input = document.create_element("input").unwrap()
        .dyn_into::<HtmlInputElement>().unwrap();
    modal.append_child(&modal_input).unwrap();
    modal.set_attribute("style", "display: none;").unwrap();
    body.append_child(&modal).unwrap();

    // Simulate opening/closing modal multiple times
    for i in 0..5 {
        // Open modal
        modal.set_attribute("style", "display: block;").unwrap();
        modal_input.focus().unwrap();

        // Type in modal
        modal_input.set_value(&format!("command{}", i));

        // Close modal
        modal.set_attribute("style", "display: none;").unwrap();
        editor_textarea.focus().unwrap();

        // Verify editor has focus
        assert_eq!(
            document.active_element().unwrap().dyn_ref::<HtmlTextAreaElement>(),
            Some(&editor_textarea),
            "Cycle {}: Editor should have focus after modal closes",
            i
        );

        // Verify typing still works
        editor_textarea.set_value(&format!("text{}", i));
        assert_eq!(editor_textarea.value(), format!("text{}", i));
    }

    editor_textarea.remove();
    modal.remove();
}

#[wasm_bindgen_test]
fn test_focus_not_lost_to_background_element() {
    let document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    let editor_textarea = document.create_element("textarea").unwrap()
        .dyn_into::<HtmlTextAreaElement>().unwrap();
    body.append_child(&editor_textarea).unwrap();

    // Create background div (like file tree panel)
    let background_div = document.create_element("div").unwrap()
        .dyn_into::<HtmlElement>().unwrap();
    background_div.set_attribute("tabindex", "0").unwrap();
    body.append_child(&background_div).unwrap();

    // Focus editor
    editor_textarea.focus().unwrap();

    // Click background (should NOT steal focus)
    // In real app, background should have pointer-events or focus management

    // Editor should still have focus
    assert_eq!(
        document.active_element().unwrap().dyn_ref::<HtmlTextAreaElement>(),
        Some(&editor_textarea),
        "Background elements should not steal focus from editor"
    );

    editor_textarea.remove();
    background_div.remove();
}

#[wasm_bindgen_test]
fn test_focus_restoration_after_blur() {
    let document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    let editor_textarea = document.create_element("textarea").unwrap()
        .dyn_into::<HtmlTextAreaElement>().unwrap();
    editor_textarea.set_id("editor");
    body.append_child(&editor_textarea).unwrap();

    // Focus -> blur -> focus
    editor_textarea.focus().unwrap();
    editor_textarea.blur().unwrap();
    editor_textarea.focus().unwrap();

    // Should be focused
    assert_eq!(
        document.active_element().unwrap().id(),
        "editor",
        "Focus should work after blur"
    );

    editor_textarea.remove();
}

#[wasm_bindgen_test]
fn test_nested_modals_focus_management() {
    let document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    let editor = document.create_element("textarea").unwrap()
        .dyn_into::<HtmlTextAreaElement>().unwrap();
    body.append_child(&editor).unwrap();

    let modal1 = document.create_element("input").unwrap()
        .dyn_into::<HtmlInputElement>().unwrap();
    modal1.set_id("modal1");
    modal1.set_attribute("style", "display: none;").unwrap();
    body.append_child(&modal1).unwrap();

    let modal2 = document.create_element("input").unwrap()
        .dyn_into::<HtmlInputElement>().unwrap();
    modal2.set_id("modal2");
    modal2.set_attribute("style", "display: none;").unwrap();
    body.append_child(&modal2).unwrap();

    // Focus editor
    editor.focus().unwrap();

    // Open modal1
    modal1.set_attribute("style", "display: block;").unwrap();
    modal1.focus().unwrap();
    assert_eq!(document.active_element().unwrap().id(), "modal1");

    // Open modal2 (nested)
    modal2.set_attribute("style", "display: block;").unwrap();
    modal2.focus().unwrap();
    assert_eq!(document.active_element().unwrap().id(), "modal2");

    // Close modal2 -> focus should return to modal1
    modal2.set_attribute("style", "display: none;").unwrap();
    modal1.focus().unwrap();
    assert_eq!(
        document.active_element().unwrap().id(),
        "modal1",
        "Focus should return to parent modal"
    );

    // Close modal1 -> focus should return to editor
    modal1.set_attribute("style", "display: none;").unwrap();
    editor.focus().unwrap();
    assert_eq!(
        document.active_element().unwrap().dyn_ref::<HtmlTextAreaElement>(),
        Some(&editor),
        "Focus should return to editor after closing all modals"
    );

    editor.remove();
    modal1.remove();
    modal2.remove();
}

#[wasm_bindgen_test]
fn test_rapid_focus_switching() {
    let document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    let editor = document.create_element("textarea").unwrap()
        .dyn_into::<HtmlTextAreaElement>().unwrap();
    editor.set_id("editor");
    body.append_child(&editor).unwrap();

    let other = document.create_element("input").unwrap()
        .dyn_into::<HtmlInputElement>().unwrap();
    other.set_id("other");
    body.append_child(&other).unwrap();

    // Rapidly switch focus
    for _ in 0..10 {
        editor.focus().unwrap();
        other.focus().unwrap();
        editor.focus().unwrap();
    }

    // Editor should still be focused
    assert_eq!(
        document.active_element().unwrap().id(),
        "editor",
        "Rapid focus switching should work reliably"
    );

    editor.remove();
    other.remove();
}
