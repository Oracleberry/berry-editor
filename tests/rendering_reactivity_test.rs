//! Rendering Reactivity Tests
//!
//! Tests for the reactive rendering system that was causing the "blank screen" bug.
//! These tests verify that Leptos signals properly trigger re-renders.

use berry_editor::buffer::TextBuffer;
use leptos::prelude::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ========================================
// Helper Functions
// ========================================

/// Calculate visible line range based on scroll position
fn calculate_visible_range(scroll_top: f64, line_height: f64, viewport_height: f64, total_lines: usize) -> (usize, usize) {
    let start_line = (scroll_top / line_height).floor() as usize;
    let visible_count = (viewport_height / line_height).ceil() as usize + 10; // +10 for overscan
    let end_line = (start_line + visible_count).min(total_lines);
    (start_line, end_line)
}

/// Calculate character offset for cursor positioning
fn calculate_char_offset(line: &str, char_count: usize) -> f64 {
    line.chars().take(char_count).map(|ch| {
        if ch as u32 > 255 { 15.625 } else { 7.8125 }
    }).sum()
}

// ========================================
// Scroll Range Calculation Tests
// ========================================

#[wasm_bindgen_test]
fn test_visible_range_at_top() {
    let scroll_top = 0.0;
    let line_height = 20.0;
    let viewport_height = 600.0;
    let total_lines = 1000;

    let (start, end) = calculate_visible_range(scroll_top, line_height, viewport_height, total_lines);

    assert_eq!(start, 0, "Should start at line 0 when scroll is 0");
    assert!(end > 30, "Should show at least 30 lines (viewport height 600 / line height 20)");
    assert!(end <= 50, "Should not exceed overscan limit");
}

#[wasm_bindgen_test]
fn test_visible_range_mid_scroll() {
    let scroll_top = 1000.0; // Scrolled down 1000px = 50 lines
    let line_height = 20.0;
    let viewport_height = 600.0;
    let total_lines = 1000;

    let (start, end) = calculate_visible_range(scroll_top, line_height, viewport_height, total_lines);

    assert_eq!(start, 50, "Should start at line 50 (1000px / 20px)");
    assert!(end > 80, "Should show lines beyond the start");
    assert!(end <= 100, "Should respect overscan limit");
}

#[wasm_bindgen_test]
fn test_visible_range_at_bottom() {
    let scroll_top = 19000.0; // Near bottom (950 lines * 20px)
    let line_height = 20.0;
    let viewport_height = 600.0;
    let total_lines = 1000;

    let (start, end) = calculate_visible_range(scroll_top, line_height, viewport_height, total_lines);

    assert_eq!(start, 950, "Should start at line 950");
    // visible_count = (600/20).ceil() + 10 = 30 + 10 = 40
    // end = min(950 + 40, 1000) = min(990, 1000) = 990
    assert_eq!(end, 990, "Should show 40 lines from start");
    assert!(end <= total_lines, "Should not exceed total_lines");
}

#[wasm_bindgen_test]
fn test_visible_range_small_file() {
    let scroll_top = 0.0;
    let line_height = 20.0;
    let viewport_height = 600.0;
    let total_lines = 10; // Small file

    let (start, end) = calculate_visible_range(scroll_top, line_height, viewport_height, total_lines);

    assert_eq!(start, 0);
    assert_eq!(end, 10, "Should not exceed total_lines for small files");
}

// ========================================
// Reactive Signal Tests
// ========================================

#[wasm_bindgen_test]
async fn test_signal_triggers_recalculation() {
    // Create a reactive signal for scroll position
    let scroll_top = RwSignal::new(0.0);

    // Create a reactive memo that calculates visible range
    let visible_range = Memo::new(move |_| {
        let scroll: f64 = scroll_top.get();
        let start = (scroll / 20.0).floor() as usize;
        let end = start + 30;
        (start, end)
    });

    // Initial state
    assert_eq!(visible_range.get(), (0, 30));

    // Update scroll position
    scroll_top.set(200.0);

    // Wait for reactive update
    wait_for_render().await;

    // Verify range was recalculated
    assert_eq!(visible_range.get(), (10, 40), "Range should update when scroll changes");
}

#[wasm_bindgen_test]
async fn test_tabs_update_triggers_rerender() {
    #[derive(Clone)]
    struct Tab {
        line_count: usize,
    }

    let tabs = RwSignal::new(Vec::<Tab>::new());

    // Create a memo that depends on tabs
    let total_height = Memo::new(move |_| {
        tabs.with(|t| {
            if let Some(tab) = t.first() {
                tab.line_count as f64 * 20.0
            } else {
                0.0
            }
        })
    });

    // Initial: no tabs
    assert_eq!(total_height.get(), 0.0);

    // Add a tab (simulating file load)
    tabs.update(|t| {
        t.push(Tab { line_count: 100 });
    });

    wait_for_render().await;

    // Verify height was recalculated
    assert_eq!(total_height.get(), 2000.0, "Height should update when tab is added");

    // Update tab line count
    tabs.update(|t| {
        if let Some(tab) = t.first_mut() {
            tab.line_count = 200;
        }
    });

    wait_for_render().await;

    assert_eq!(total_height.get(), 4000.0, "Height should update when tab content changes");
}

// ========================================
// Cursor Position Calculation Tests
// ========================================

#[wasm_bindgen_test]
fn test_cursor_offset_ascii_only() {
    let line = "fn main() {";
    let char_count = 5;

    let offset = calculate_char_offset(line, char_count);

    // 5 ASCII chars * 7.8125px = 39.0625px
    assert!((offset - 39.0625).abs() < 0.01, "ASCII offset should be ~39px, got {}", offset);
}

#[wasm_bindgen_test]
fn test_cursor_offset_japanese() {
    let line = "日本語";
    let char_count = 2;

    let offset = calculate_char_offset(line, char_count);

    // 2 Japanese chars * 15.625px = 31.25px
    assert!((offset - 31.25).abs() < 0.01, "Japanese offset should be ~31.25px, got {}", offset);
}

#[wasm_bindgen_test]
fn test_cursor_offset_mixed() {
    let line = "Hello 世界";
    let char_count = 8;

    let offset = calculate_char_offset(line, char_count);

    // "Hello " = 6 ASCII * 7.8125 = 46.875
    // "世界" = 2 Japanese * 15.625 = 31.25
    // Total = 78.125
    assert!((offset - 78.125).abs() < 0.01, "Mixed offset should be ~78.125px, got {}", offset);
}

#[wasm_bindgen_test]
fn test_cursor_offset_beyond_line_end() {
    let line = "short";
    let char_count = 100; // Beyond line length

    let offset = calculate_char_offset(line, char_count);

    // Should only count actual characters (5)
    let expected = 5.0 * 7.8125; // 39.0625
    assert!((offset - expected).abs() < 0.01, "Should clamp to line length");
}

// ========================================
// Buffer Line Access Tests
// ========================================

#[wasm_bindgen_test]
fn test_buffer_line_access() {
    let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
    let buffer = TextBuffer::from_str(content);

    assert_eq!(buffer.len_lines(), 5);
    assert_eq!(buffer.line(0), Some("Line 1\n".to_string()));
    assert_eq!(buffer.line(2), Some("Line 3\n".to_string()));
    assert_eq!(buffer.line(4), Some("Line 5".to_string())); // Last line has no \n
}

#[wasm_bindgen_test]
fn test_buffer_empty_lines() {
    let content = "A\n\nC";
    let buffer = TextBuffer::from_str(content);

    assert_eq!(buffer.len_lines(), 3);
    assert_eq!(buffer.line(0), Some("A\n".to_string()));
    assert_eq!(buffer.line(1), Some("\n".to_string())); // Empty line
    assert_eq!(buffer.line(2), Some("C".to_string()));
}

// ========================================
// Integration: Scroll + Buffer
// ========================================

#[wasm_bindgen_test]
fn test_visible_lines_from_buffer() {
    let content = (0..1000)
        .map(|i| format!("Line {}", i + 1))
        .collect::<Vec<_>>()
        .join("\n");

    let buffer = TextBuffer::from_str(&content);
    let scroll_top = 500.0; // 25 lines down
    let (start, end) = calculate_visible_range(scroll_top, 20.0, 600.0, buffer.len_lines());

    assert_eq!(start, 25);

    // Verify we can access all visible lines
    for line_idx in start..end {
        let line = buffer.line(line_idx);
        assert!(line.is_some(), "Line {} should exist", line_idx);
        assert!(line.unwrap().contains(&format!("Line {}", line_idx + 1)));
    }
}

// ========================================
// Helper Functions
// ========================================

async fn wait_for_render() {
    wasm_bindgen_futures::JsFuture::from(js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 50)
            .unwrap();
    }))
    .await
    .unwrap();
}
