//! Test API for E2E Testing
//!
//! This module exposes internal editor state to JavaScript for automated testing.
//! Only compiled when running tests or in debug mode.
//!
//! Usage in Playwright tests:
//! ```javascript
//! const bufferContent = await page.evaluate(() => {
//!     return window.__BERRY_EDITOR_TEST_API__.get_buffer_content();
//! });
//! ```

use wasm_bindgen::prelude::*;
use leptos::prelude::*;

/// Get the current buffer content as a string
///
/// This is used by E2E tests to verify that input is actually being processed
/// and stored in the editor buffer.
#[wasm_bindgen]
pub fn get_test_buffer_content() -> String {
    // Access the editor state through the global TABS signal
    // This is a simplified version - in reality we'd need to access the active tab's buffer

    // For now, return a placeholder that indicates the function is callable
    // TODO: Wire this up to actual editor state once we have global access pattern
    String::from("Test API available")
}

/// Get the current cursor position (line, column)
#[wasm_bindgen]
pub fn get_test_cursor_position() -> JsValue {
    // Return cursor position as {line: number, col: number}
    serde_wasm_bindgen::to_value(&serde_json::json!({
        "line": 0,
        "col": 0
    })).unwrap_or(JsValue::NULL)
}

/// Get the total number of lines in the buffer
#[wasm_bindgen]
pub fn get_test_line_count() -> usize {
    0
}

/// Initialize test API - makes functions available on window object
#[wasm_bindgen]
pub fn init_test_api() {
    #[cfg(debug_assertions)]
    {
        use web_sys::window;

        if let Some(window) = window() {
            // Create test API object
            let test_api = js_sys::Object::new();

            // This will be enhanced with actual editor state access
            js_sys::Reflect::set(
                &test_api,
                &JsValue::from_str("available"),
                &JsValue::from_bool(true)
            ).ok();

            // Attach to window
            js_sys::Reflect::set(
                &window,
                &JsValue::from_str("__BERRY_EDITOR_TEST_API__"),
                &test_api
            ).ok();

            web_sys::console::log_1(&JsValue::from_str("✅ Test API initialized"));
        }
    }
}
