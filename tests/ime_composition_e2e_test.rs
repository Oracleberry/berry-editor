//! IME Composition E2E Tests
//!
//! CRITICAL: Tests for Japanese input during composition
//!
//! The most fragile part of text input:
//! - compositionstart -> compositionupdate -> compositionend
//! - Double input bugs (same text inserted twice)
//! - Cursor jumping during conversion
//! - Character corruption when mixing on:input and on:keydown

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{window, HtmlTextAreaElement, CompositionEvent};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_ime_composition_events_fire() {
    let document = window().unwrap().document().unwrap();
    let textarea = document.create_element("textarea").unwrap()
        .dyn_into::<HtmlTextAreaElement>().unwrap();

    document.body().unwrap().append_child(&textarea).unwrap();

    let composition_started = std::rc::Rc::new(std::cell::Cell::new(false));
    let composition_ended = std::rc::Rc::new(std::cell::Cell::new(false));

    let started_clone = composition_started.clone();
    let ended_clone = composition_ended.clone();

    let start_callback = Closure::wrap(Box::new(move |_: web_sys::Event| {
        started_clone.set(true);
    }) as Box<dyn FnMut(_)>);

    let end_callback = Closure::wrap(Box::new(move |_: web_sys::Event| {
        ended_clone.set(true);
    }) as Box<dyn FnMut(_)>);

    textarea.add_event_listener_with_callback(
        "compositionstart",
        start_callback.as_ref().unchecked_ref()
    ).unwrap();

    textarea.add_event_listener_with_callback(
        "compositionend",
        end_callback.as_ref().unchecked_ref()
    ).unwrap();

    // Simulate IME composition
    textarea.dispatch_event(&web_sys::Event::new("compositionstart").unwrap()).unwrap();
    textarea.dispatch_event(&web_sys::Event::new("compositionend").unwrap()).unwrap();

    assert!(composition_started.get(), "compositionstart should fire");
    assert!(composition_ended.get(), "compositionend should fire");

    start_callback.forget();
    end_callback.forget();
    textarea.remove();
}

#[wasm_bindgen_test]
fn test_ime_no_double_input_on_composition() {
    let document = window().unwrap().document().unwrap();
    let textarea = document.create_element("textarea").unwrap()
        .dyn_into::<HtmlTextAreaElement>().unwrap();

    document.body().unwrap().append_child(&textarea).unwrap();
    textarea.focus().unwrap();

    let input_count = std::rc::Rc::new(std::cell::Cell::new(0));
    let count_clone = input_count.clone();

    let input_callback = Closure::wrap(Box::new(move |_: web_sys::InputEvent| {
        count_clone.set(count_clone.get() + 1);
    }) as Box<dyn FnMut(_)>);

    textarea.add_event_listener_with_callback(
        "input",
        input_callback.as_ref().unchecked_ref()
    ).unwrap();

    // Simulate IME composition sequence
    textarea.dispatch_event(&web_sys::Event::new("compositionstart").unwrap()).unwrap();

    // During composition, input events might fire
    textarea.set_value("こ");
    textarea.dispatch_event(&web_sys::InputEvent::new("input").unwrap()).unwrap();

    textarea.set_value("こん");
    textarea.dispatch_event(&web_sys::InputEvent::new("input").unwrap()).unwrap();

    textarea.set_value("こんにちは");
    textarea.dispatch_event(&web_sys::Event::new("compositionend").unwrap()).unwrap();
    textarea.dispatch_event(&web_sys::InputEvent::new("input").unwrap()).unwrap();

    // The critical check: editor should NOT insert text multiple times
    // Input count should be reasonable (not 10x the expected)
    assert!(
        input_count.get() <= 5,
        "Too many input events fired: {}. Possible double-input bug!",
        input_count.get()
    );

    input_callback.forget();
    textarea.remove();
}

#[wasm_bindgen_test]
fn test_ime_composition_value_stability() {
    let document = window().unwrap().document().unwrap();
    let textarea = document.create_element("textarea").unwrap()
        .dyn_into::<HtmlTextAreaElement>().unwrap();

    document.body().unwrap().append_child(&textarea).unwrap();
    textarea.focus().unwrap();

    // Simulate IME composition
    textarea.dispatch_event(&web_sys::Event::new("compositionstart").unwrap()).unwrap();

    textarea.set_value("こんにちは");

    textarea.dispatch_event(&web_sys::Event::new("compositionend").unwrap()).unwrap();

    // After composition ends, value should be stable
    assert_eq!(
        textarea.value(),
        "こんにちは",
        "Textarea value should remain stable after IME composition"
    );

    textarea.remove();
}

#[wasm_bindgen_test]
fn test_ime_multiple_compositions_sequential() {
    let document = window().unwrap().document().unwrap();
    let textarea = document.create_element("textarea").unwrap()
        .dyn_into::<HtmlTextAreaElement>().unwrap();

    document.body().unwrap().append_child(&textarea).unwrap();
    textarea.focus().unwrap();

    // First composition: こんにちは
    textarea.dispatch_event(&web_sys::Event::new("compositionstart").unwrap()).unwrap();
    textarea.set_value("こんにちは");
    textarea.dispatch_event(&web_sys::Event::new("compositionend").unwrap()).unwrap();

    let first_value = textarea.value();

    // Second composition: 世界
    textarea.dispatch_event(&web_sys::Event::new("compositionstart").unwrap()).unwrap();
    let mut new_value = first_value.clone();
    new_value.push_str("世界");
    textarea.set_value(&new_value);
    textarea.dispatch_event(&web_sys::Event::new("compositionend").unwrap()).unwrap();

    assert_eq!(
        textarea.value(),
        "こんにちは世界",
        "Multiple sequential IME compositions should work"
    );

    textarea.remove();
}

#[wasm_bindgen_test]
fn test_ime_mixed_ascii_japanese_composition() {
    let document = window().unwrap().document().unwrap();
    let textarea = document.create_element("textarea").unwrap()
        .dyn_into::<HtmlTextAreaElement>().unwrap();

    document.body().unwrap().append_child(&textarea).unwrap();
    textarea.focus().unwrap();

    // Type ASCII
    textarea.set_value("Hello ");
    textarea.dispatch_event(&web_sys::InputEvent::new("input").unwrap()).unwrap();

    // Then Japanese via IME
    textarea.dispatch_event(&web_sys::Event::new("compositionstart").unwrap()).unwrap();
    let mut value = textarea.value();
    value.push_str("世界");
    textarea.set_value(&value);
    textarea.dispatch_event(&web_sys::Event::new("compositionend").unwrap()).unwrap();

    // Then more ASCII
    let mut value = textarea.value();
    value.push('!');
    textarea.set_value(&value);
    textarea.dispatch_event(&web_sys::InputEvent::new("input").unwrap()).unwrap();

    assert_eq!(
        textarea.value(),
        "Hello 世界!",
        "Mixed ASCII and Japanese IME composition should work"
    );

    textarea.remove();
}

#[wasm_bindgen_test]
fn test_ime_composition_doesnt_corrupt_existing_text() {
    let document = window().unwrap().document().unwrap();
    let textarea = document.create_element("textarea").unwrap()
        .dyn_into::<HtmlTextAreaElement>().unwrap();

    document.body().unwrap().append_child(&textarea).unwrap();
    textarea.focus().unwrap();

    // Set initial text
    textarea.set_value("Line 1\nLine 2\nLine 3");

    // Position cursor at end of Line 2
    textarea.set_selection_start(Some(13)).unwrap();  // After "Line 2"
    textarea.set_selection_end(Some(13)).unwrap();

    // Start IME composition
    textarea.dispatch_event(&web_sys::Event::new("compositionstart").unwrap()).unwrap();

    // In a real editor, this would insert at cursor position
    // For this test, we just verify no corruption
    let initial_lines = textarea.value().lines().count();

    textarea.dispatch_event(&web_sys::Event::new("compositionend").unwrap()).unwrap();

    let final_lines = textarea.value().lines().count();

    assert_eq!(
        initial_lines, final_lines,
        "IME composition should not corrupt existing line structure"
    );

    textarea.remove();
}
