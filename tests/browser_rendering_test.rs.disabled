//! Browser Rendering Measurement Tests
//!
//! CRITICAL: These tests measure ACTUAL browser rendering
//! and compare it with our Rust calculations.
//!
//! WHY THIS IS NEEDED:
//! - Our Rust code assumes CHAR_WIDTH_ASCII = 7.8125px
//! - But the actual browser might render it as 7.9px or 7.7px
//! - This 0.1px difference accumulates and causes cursor drift
//!
//! This test uses Canvas2D.measureText() to get the REAL width
//! from the browser's rendering engine.

use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

wasm_bindgen_test_configure!(run_in_browser);

// Our assumed constants
const CHAR_WIDTH_ASCII: f64 = 7.8125;
const CHAR_WIDTH_WIDE: f64 = 15.625;

/// Get actual rendered width from browser
fn measure_text_width(text: &str, font: &str) -> f64 {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document
        .create_element("canvas")
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()
        .unwrap();

    context.set_font(font);
    context.measure_text(text).unwrap().width()
}

// ========================================
// Reality Check Tests
// ========================================

#[wasm_bindgen_test]
fn test_ascii_width_matches_browser() {
    let font = "13px 'JetBrains Mono', monospace";

    // Measure single ASCII character
    let actual_width = measure_text_width("a", font);

    wasm_bindgen_test::console_log!("Expected ASCII width: {}", CHAR_WIDTH_ASCII);
    wasm_bindgen_test::console_log!("Actual browser width: {}", actual_width);

    let diff = (actual_width - CHAR_WIDTH_ASCII).abs();
    assert!(
        diff < 0.5,
        "ASCII width mismatch! Expected: {}, Actual: {}, Diff: {}",
        CHAR_WIDTH_ASCII,
        actual_width,
        diff
    );
}

#[wasm_bindgen_test]
fn test_wide_width_matches_browser() {
    let font = "13px 'JetBrains Mono', monospace";

    // Measure single wide character
    let actual_width = measure_text_width("あ", font);

    wasm_bindgen_test::console_log!("Expected wide width: {}", CHAR_WIDTH_WIDE);
    wasm_bindgen_test::console_log!("Actual browser width: {}", actual_width);

    let diff = (actual_width - CHAR_WIDTH_WIDE).abs();
    assert!(
        diff < 0.5,
        "Wide character width mismatch! Expected: {}, Actual: {}, Diff: {}",
        CHAR_WIDTH_WIDE,
        actual_width,
        diff
    );
}

#[wasm_bindgen_test]
fn test_line_width_accumulation() {
    let font = "13px 'JetBrains Mono', monospace";
    let line = "fn main() {";

    // Our calculation
    let our_width: f64 = line.chars().map(|ch| {
        if ch as u32 > 255 { CHAR_WIDTH_WIDE } else { CHAR_WIDTH_ASCII }
    }).sum();

    // Browser's actual rendering
    let actual_width = measure_text_width(line, font);

    wasm_bindgen_test::console_log!("Line: {:?}", line);
    wasm_bindgen_test::console_log!("Our calculation: {}", our_width);
    wasm_bindgen_test::console_log!("Browser rendering: {}", actual_width);
    wasm_bindgen_test::console_log!("Difference: {}", (actual_width - our_width).abs());

    let diff = (actual_width - our_width).abs();
    assert!(
        diff < line.len() as f64 * 0.5,  // Allow 0.5px error per character
        "Line width accumulation error too large! Our: {}, Actual: {}, Diff: {}",
        our_width,
        actual_width,
        diff
    );
}

#[wasm_bindgen_test]
fn test_mixed_line_accuracy() {
    let font = "13px 'JetBrains Mono', monospace";
    let line = "Hello 世界";

    let our_width: f64 = line.chars().map(|ch| {
        if ch as u32 > 255 { CHAR_WIDTH_WIDE } else { CHAR_WIDTH_ASCII }
    }).sum();

    let actual_width = measure_text_width(line, font);

    wasm_bindgen_test::console_log!("Mixed line: {:?}", line);
    wasm_bindgen_test::console_log!("Our calculation: {}", our_width);
    wasm_bindgen_test::console_log!("Browser rendering: {}", actual_width);

    let diff_percentage = ((actual_width - our_width).abs() / actual_width) * 100.0;

    wasm_bindgen_test::console_log!("Difference: {}%", diff_percentage);

    assert!(
        diff_percentage < 5.0,
        "Mixed character line has >5% error! Our: {}, Actual: {}, Diff: {}%",
        our_width,
        actual_width,
        diff_percentage
    );
}

// ========================================
// Device Pixel Ratio Test
// ========================================

#[wasm_bindgen_test]
fn test_device_pixel_ratio_awareness() {
    let window = web_sys::window().unwrap();
    let dpr = window.device_pixel_ratio();

    wasm_bindgen_test::console_log!("Device Pixel Ratio: {}", dpr);

    // On Retina displays (dpr=2), we might need to adjust calculations
    assert!(
        dpr > 0.0 && dpr <= 3.0,
        "Unexpected device pixel ratio: {}",
        dpr
    );
}
