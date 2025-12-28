//! Platform Abstraction Layer Tests
//!
//! These tests verify that the platform abstraction works correctly
//! across different environments without requiring DOM or browser APIs.
//!
//! Run with: cargo test --test platform_abstraction_test

use berry_editor::common::storage::{EditorStorage, MockStorage, create_mock_storage};
use berry_editor::common::platform::Platform;
use berry_editor::common::events::{PointerPosition, ScrollPosition, matches_modifier};

// ========================================
// Storage Abstraction Tests
// ========================================

#[test]
fn test_storage_trait_via_mock() {
    let storage = MockStorage::new();

    // Basic operations
    storage.set_item("editor_theme", "dark").unwrap();
    assert_eq!(
        storage.get_item("editor_theme").unwrap(),
        Some("dark".to_string())
    );

    // Update
    storage.set_item("editor_theme", "light").unwrap();
    assert_eq!(
        storage.get_item("editor_theme").unwrap(),
        Some("light".to_string())
    );

    // Remove
    storage.remove_item("editor_theme").unwrap();
    assert_eq!(storage.get_item("editor_theme").unwrap(), None);
}

#[test]
fn test_storage_persistence_simulation() {
    let storage = MockStorage::new();

    // Simulate saving editor state
    storage.set_item("cursor_line", "42").unwrap();
    storage.set_item("cursor_col", "15").unwrap();
    storage.set_item("scroll_top", "1200.5").unwrap();

    // Verify persistence
    let line = storage.get_item("cursor_line").unwrap().unwrap();
    let col = storage.get_item("cursor_col").unwrap().unwrap();
    let scroll = storage.get_item("scroll_top").unwrap().unwrap();

    assert_eq!(line.parse::<usize>().unwrap(), 42);
    assert_eq!(col.parse::<usize>().unwrap(), 15);
    assert_eq!(scroll.parse::<f64>().unwrap(), 1200.5);
}

#[test]
fn test_storage_clear_operation() {
    let storage = MockStorage::new();

    // Add multiple items
    storage.set_item("item1", "value1").unwrap();
    storage.set_item("item2", "value2").unwrap();
    storage.set_item("item3", "value3").unwrap();

    // Clear all
    storage.clear().unwrap();

    // Verify all cleared
    assert_eq!(storage.get_item("item1").unwrap(), None);
    assert_eq!(storage.get_item("item2").unwrap(), None);
    assert_eq!(storage.get_item("item3").unwrap(), None);
}

#[test]
fn test_storage_factory_creates_mock() {
    let storage = create_mock_storage();

    storage.set_item("test_key", "test_value").unwrap();
    assert_eq!(
        storage.get_item("test_key").unwrap(),
        Some("test_value".to_string())
    );
}

// ========================================
// Platform Detection Tests
// ========================================

#[test]
fn test_platform_current_detection() {
    let platform = Platform::current();

    // Verify it returns a valid platform
    match platform {
        Platform::Web | Platform::Desktop | Platform::iOS | Platform::Android => {
            // Valid platform detected
        }
    }
}

#[test]
fn test_platform_categorization() {
    // Test each platform's categorization
    assert!(Platform::iOS.is_mobile());
    assert!(Platform::Android.is_mobile());
    assert!(!Platform::Web.is_mobile());
    assert!(!Platform::Desktop.is_mobile());

    assert!(Platform::Web.is_web());
    assert!(!Platform::Desktop.is_web());
    assert!(!Platform::iOS.is_web());

    assert!(Platform::Desktop.is_desktop());
    assert!(!Platform::Web.is_desktop());
    assert!(!Platform::iOS.is_desktop());
}

#[test]
fn test_platform_names() {
    assert_eq!(Platform::Web.name(), "Web");
    assert_eq!(Platform::Desktop.name(), "Desktop");
    assert_eq!(Platform::iOS.name(), "iOS");
    assert_eq!(Platform::Android.name(), "Android");
}

// ========================================
// Event Abstraction Tests
// ========================================

#[test]
fn test_pointer_position_creation() {
    let pos = PointerPosition::new(100.0, 200.0);

    assert_eq!(pos.client_x, 100.0);
    assert_eq!(pos.client_y, 200.0);
    assert_eq!(pos.page_x, 100.0); // Should default to client when not specified
    assert_eq!(pos.page_y, 200.0);
}

#[test]
fn test_pointer_position_with_scroll() {
    let pos = PointerPosition::with_page(100.0, 200.0, 100.0, 400.0);

    assert_eq!(pos.client_x, 100.0);
    assert_eq!(pos.client_y, 200.0);
    assert_eq!(pos.page_x, 100.0);
    assert_eq!(pos.page_y, 400.0); // page_y includes scroll offset
}

#[test]
fn test_scroll_position() {
    let scroll = ScrollPosition::new(50.0, 1200.0);

    assert_eq!(scroll.x, 50.0);
    assert_eq!(scroll.y, 1200.0);
}

#[test]
fn test_modifier_key_matching() {
    // Test exact match
    assert!(matches_modifier(
        true, false, false, false, // event: shift only
        true, false, false, false  // required: shift only
    ));

    // Test mismatch
    assert!(!matches_modifier(
        true, false, false, false, // event: shift only
        false, true, false, false  // required: ctrl only
    ));

    // Test multiple modifiers
    assert!(matches_modifier(
        true, true, false, false,  // event: shift + ctrl
        true, true, false, false   // required: shift + ctrl
    ));

    // Test no modifiers
    assert!(matches_modifier(
        false, false, false, false, // event: no modifiers
        false, false, false, false  // required: no modifiers
    ));
}

// ========================================
// Cross-Platform Logic Tests
// ========================================

#[test]
fn test_splitter_size_persistence_logic() {
    // Simulate splitter size persistence without DOM
    let storage = MockStorage::new();

    // Initial size
    let initial_size = 250.0;
    storage.set_item("splitter_size", &initial_size.to_string()).unwrap();

    // Load size
    let loaded_size = storage
        .get_item("splitter_size")
        .unwrap()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(200.0);

    assert_eq!(loaded_size, 250.0);

    // Update size
    let new_size = 300.0;
    storage.set_item("splitter_size", &new_size.to_string()).unwrap();

    let updated_size = storage
        .get_item("splitter_size")
        .unwrap()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap();

    assert_eq!(updated_size, 300.0);
}

#[test]
fn test_cursor_position_calculation_logic() {
    // Test cursor position calculation without DOM
    const LINE_HEIGHT: f64 = 20.0;
    const CHAR_WIDTH: f64 = 8.4;

    let click_y = 150.0;
    let scroll_top = 100.0;
    let y_absolute = click_y + scroll_top;

    let line = (y_absolute / LINE_HEIGHT).floor() as usize;
    assert_eq!(line, 12); // (250 / 20) = 12.5, floor = 12

    let click_x = 100.0_f64;
    let padding = 10.0_f64;
    let col = ((click_x - padding).max(0.0) / CHAR_WIDTH).round() as usize;
    assert_eq!(col, 11); // ((100 - 10) / 8.4) = 10.7, round = 11
}

#[test]
fn test_viewport_scroll_logic() {
    // Test scroll range calculation without DOM
    const LINE_HEIGHT: f64 = 20.0;
    const VIEWPORT_HEIGHT: f64 = 600.0;

    let scroll_top = 1000.0;
    let buffer = 5; // Extra lines above/below

    let start_line = ((scroll_top / LINE_HEIGHT).floor() as usize).saturating_sub(buffer);
    let visible_lines = (VIEWPORT_HEIGHT / LINE_HEIGHT).ceil() as usize;
    let end_line = start_line + visible_lines + (buffer * 2);

    assert_eq!(start_line, 45); // (1000 / 20) - 5 = 45
    assert!(end_line > start_line);
    assert!(end_line <= start_line + visible_lines + (buffer * 2));
}

// ========================================
// Platform-Specific Behavior Tests
// ========================================

#[test]
fn test_platform_dependent_logic() {
    let platform = Platform::current();

    match platform {
        Platform::Web => {
            // Web-specific assertions
            assert!(platform.is_web());
            assert!(!platform.is_mobile());
        }
        Platform::Desktop => {
            // Desktop-specific assertions
            assert!(platform.is_desktop());
            assert!(!platform.is_mobile());
        }
        Platform::iOS | Platform::Android => {
            // Mobile-specific assertions
            assert!(platform.is_mobile());
            assert!(!platform.is_desktop());
        }
    }
}

#[test]
fn test_storage_key_sanitization() {
    let storage = MockStorage::new();

    // Test that storage can handle various key formats
    let test_keys = vec![
        "simple_key",
        "key-with-dashes",
        "key.with.dots",
        "key_with_underscores",
        "CamelCaseKey",
    ];

    for key in test_keys {
        storage.set_item(key, "test_value").unwrap();
        assert_eq!(
            storage.get_item(key).unwrap(),
            Some("test_value".to_string())
        );
        storage.remove_item(key).unwrap();
    }
}
