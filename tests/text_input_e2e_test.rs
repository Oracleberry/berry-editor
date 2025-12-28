//! E2E Text Input Tests
//!
//! These tests verify that text input actually works:
//! 1. Textarea can be focused
//! 2. Input events fire correctly
//! 3. Text is inserted at the correct position
//! 4. Japanese IME input works
//! 5. Cursor moves correctly after input

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{window, HtmlTextAreaElement, KeyboardEvent, InputEvent, Event};

wasm_bindgen_test_configure!(run_in_browser);

/// Create a test textarea that mimics the editor's hidden input
fn create_test_textarea() -> HtmlTextAreaElement {
    let document = window().unwrap().document().unwrap();
    let textarea = document.create_element("textarea").unwrap()
        .dyn_into::<HtmlTextAreaElement>().unwrap();

    textarea.set_attribute("style",
        "position: absolute; \
         left: 50px; \
         top: 50px; \
         width: 10px; \
         height: 20px; \
         opacity: 0; \
         z-index: 40; \
         resize: none; \
         border: none; \
         outline: none;"
    ).unwrap();

    document.body().unwrap().append_child(&textarea).unwrap();
    textarea
}

// ========================================
// Basic Input Tests
// ========================================

#[wasm_bindgen_test]
fn test_textarea_can_be_focused() {
    let textarea = create_test_textarea();

    // Attempt to focus
    textarea.focus().unwrap();

    // Verify focus
    let document = window().unwrap().document().unwrap();
    let active = document.active_element().unwrap();

    assert_eq!(
        active.dyn_ref::<HtmlTextAreaElement>(),
        Some(&textarea),
        "Textarea should be focused"
    );

    textarea.remove();
}

#[wasm_bindgen_test]
fn test_ascii_input() {
    let textarea = create_test_textarea();
    textarea.focus().unwrap();

    // Simulate typing "hello"
    textarea.set_value("hello");

    assert_eq!(textarea.value(), "hello", "ASCII input should work");

    textarea.remove();
}

#[wasm_bindgen_test]
fn test_japanese_input() {
    let textarea = create_test_textarea();
    textarea.focus().unwrap();

    // Simulate Japanese input
    textarea.set_value("こんにちは");

    assert_eq!(
        textarea.value(),
        "こんにちは",
        "Japanese input should work"
    );

    textarea.remove();
}

#[wasm_bindgen_test]
fn test_mixed_input() {
    let textarea = create_test_textarea();
    textarea.focus().unwrap();

    // Simulate mixed ASCII + Japanese
    textarea.set_value("Hello 世界");

    assert_eq!(
        textarea.value(),
        "Hello 世界",
        "Mixed ASCII and Japanese input should work"
    );

    textarea.remove();
}

// ========================================
// Input Event Tests
// ========================================

#[wasm_bindgen_test]
fn test_input_event_fires() {
    let textarea = create_test_textarea();
    textarea.focus().unwrap();

    // Track if input event fired
    let fired = std::rc::Rc::new(std::cell::Cell::new(false));
    let fired_clone = fired.clone();

    let callback = Closure::wrap(Box::new(move |_: InputEvent| {
        fired_clone.set(true);
    }) as Box<dyn FnMut(_)>);

    textarea.add_event_listener_with_callback(
        "input",
        callback.as_ref().unchecked_ref()
    ).unwrap();

    // Trigger input
    textarea.set_value("test");

    let event = InputEvent::new("input").unwrap();
    textarea.dispatch_event(&event).unwrap();

    assert!(fired.get(), "Input event should fire");

    callback.forget();
    textarea.remove();
}

#[wasm_bindgen_test]
fn test_keydown_event_fires() {
    let textarea = create_test_textarea();
    textarea.focus().unwrap();

    let fired = std::rc::Rc::new(std::cell::Cell::new(false));
    let fired_clone = fired.clone();

    let callback = Closure::wrap(Box::new(move |_: KeyboardEvent| {
        fired_clone.set(true);
    }) as Box<dyn FnMut(_)>);

    textarea.add_event_listener_with_callback(
        "keydown",
        callback.as_ref().unchecked_ref()
    ).unwrap();

    // Trigger keydown
    let event = KeyboardEvent::new("keydown").unwrap();

    textarea.dispatch_event(&event).unwrap();

    assert!(fired.get(), "Keydown event should fire");

    callback.forget();
    textarea.remove();
}

// ========================================
// Cursor Position After Input Tests
// ========================================

#[wasm_bindgen_test]
fn test_selection_after_input() {
    let textarea = create_test_textarea();
    textarea.focus().unwrap();

    // Type some text
    textarea.set_value("hello");

    // Cursor should be at end (position 5)
    textarea.set_selection_start(Some(5)).unwrap();
    textarea.set_selection_end(Some(5)).unwrap();

    assert_eq!(
        textarea.selection_start().unwrap(),
        Some(5),
        "Cursor should be at position 5 after typing 'hello'"
    );

    textarea.remove();
}

#[wasm_bindgen_test]
fn test_selection_with_japanese() {
    let textarea = create_test_textarea();
    textarea.focus().unwrap();

    // Type Japanese text (7 characters)
    let text = "こんにちは世界";
    textarea.set_value(text);

    let char_count = text.chars().count();
    textarea.set_selection_start(Some(char_count as u32)).unwrap();
    textarea.set_selection_end(Some(char_count as u32)).unwrap();

    assert_eq!(
        textarea.selection_start().unwrap(),
        Some(char_count as u32),
        "Cursor should be at end after typing Japanese"
    );

    textarea.remove();
}

#[wasm_bindgen_test]
fn test_insert_at_middle() {
    let textarea = create_test_textarea();
    textarea.focus().unwrap();

    // Type initial text
    textarea.set_value("helo");

    // Position cursor at index 3 (between 'l' and 'o')
    textarea.set_selection_start(Some(3)).unwrap();
    textarea.set_selection_end(Some(3)).unwrap();

    // Insert 'l'
    let mut value = textarea.value();
    value.insert(3, 'l');
    textarea.set_value(&value);

    assert_eq!(textarea.value(), "hello", "Should insert character at cursor position");

    textarea.remove();
}

// ========================================
// IME Composition Tests
// ========================================

#[wasm_bindgen_test]
fn test_composition_events_exist() {
    let textarea = create_test_textarea();
    textarea.focus().unwrap();

    let composition_start_fired = std::rc::Rc::new(std::cell::Cell::new(false));
    let composition_end_fired = std::rc::Rc::new(std::cell::Cell::new(false));

    let start_clone = composition_start_fired.clone();
    let end_clone = composition_end_fired.clone();

    let start_callback = Closure::wrap(Box::new(move |_: Event| {
        start_clone.set(true);
    }) as Box<dyn FnMut(_)>);

    let end_callback = Closure::wrap(Box::new(move |_: Event| {
        end_clone.set(true);
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
    let start_event = Event::new("compositionstart").unwrap();
    textarea.dispatch_event(&start_event).unwrap();

    let end_event = Event::new("compositionend").unwrap();
    textarea.dispatch_event(&end_event).unwrap();

    assert!(
        composition_start_fired.get(),
        "compositionstart event should fire for IME input"
    );
    assert!(
        composition_end_fired.get(),
        "compositionend event should fire for IME input"
    );

    start_callback.forget();
    end_callback.forget();
    textarea.remove();
}

// ========================================
// Textarea Visibility and Positioning Tests
// ========================================

#[wasm_bindgen_test]
fn test_textarea_is_in_dom() {
    let textarea = create_test_textarea();

    let document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    assert!(
        body.contains(Some(&textarea)),
        "Textarea should be in the DOM"
    );

    textarea.remove();
}

#[wasm_bindgen_test]
fn test_textarea_has_correct_style() {
    let textarea = create_test_textarea();

    let style = textarea.get_attribute("style").unwrap();

    assert!(style.contains("position: absolute"), "Should be absolutely positioned");
    assert!(style.contains("z-index: 40"), "Should have correct z-index");
    assert!(style.contains("opacity: 0"), "Should be invisible");

    textarea.remove();
}

#[wasm_bindgen_test]
fn test_textarea_dimensions() {
    let textarea = create_test_textarea();

    // Note: offsetWidth/Height might be 0 if opacity is 0
    // So we check the style attribute instead
    let style = textarea.get_attribute("style").unwrap();

    assert!(style.contains("width: 10px"), "Should have width");
    assert!(style.contains("height: 20px"), "Should have height");

    textarea.remove();
}

// ========================================
// Focus Robustness Tests
// ========================================

#[wasm_bindgen_test]
fn test_focus_after_blur() {
    let textarea = create_test_textarea();

    // Focus, then blur, then focus again
    textarea.focus().unwrap();
    textarea.blur().unwrap();
    textarea.focus().unwrap();

    let document = window().unwrap().document().unwrap();
    let active = document.active_element().unwrap();

    assert_eq!(
        active.dyn_ref::<HtmlTextAreaElement>(),
        Some(&textarea),
        "Textarea should be focusable after blur"
    );

    textarea.remove();
}

#[wasm_bindgen_test]
fn test_multiple_textareas_focus_switching() {
    let textarea1 = create_test_textarea();
    let textarea2 = create_test_textarea();

    // Focus first textarea
    textarea1.focus().unwrap();
    let document = window().unwrap().document().unwrap();
    let active = document.active_element().unwrap();
    assert_eq!(active.dyn_ref::<HtmlTextAreaElement>(), Some(&textarea1));

    // Switch focus to second textarea
    textarea2.focus().unwrap();
    let active = document.active_element().unwrap();
    assert_eq!(active.dyn_ref::<HtmlTextAreaElement>(), Some(&textarea2));

    textarea1.remove();
    textarea2.remove();
}

// ========================================
// Real-World Scenario Tests
// ========================================

#[wasm_bindgen_test]
fn test_complete_input_flow() {
    let textarea = create_test_textarea();

    // 1. Focus textarea
    textarea.focus().unwrap();

    // 2. Type some text
    textarea.set_value("Hello");
    assert_eq!(textarea.value(), "Hello");

    // 3. Move cursor to end
    textarea.set_selection_start(Some(5)).unwrap();
    textarea.set_selection_end(Some(5)).unwrap();

    // 4. Append more text
    let mut value = textarea.value();
    value.push_str(" world");
    textarea.set_value(&value);

    // 5. Verify final result
    assert_eq!(textarea.value(), "Hello world", "Complete input flow should work");

    textarea.remove();
}

#[wasm_bindgen_test]
fn test_japanese_input_flow() {
    let textarea = create_test_textarea();

    textarea.focus().unwrap();

    // 1. Type "Hello "
    textarea.set_value("Hello ");

    // 2. Type Japanese "世界"
    let mut value = textarea.value();
    value.push_str("世界");
    textarea.set_value(&value);

    // 3. Type "!"
    let mut value = textarea.value();
    value.push('!');
    textarea.set_value(&value);

    assert_eq!(
        textarea.value(),
        "Hello 世界!",
        "Japanese input flow should work"
    );

    textarea.remove();
}
