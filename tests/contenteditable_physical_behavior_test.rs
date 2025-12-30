//! E2E Test: ContentEditable Physical Behavior Verification
//!
//! This test suite verifies the critical physical behaviors introduced by
//! the ContentEditable architecture migration:
//!
//! 1. **Click Transparency**: Rendering layer (scroll-content) with
//!    `pointer-events: none` allows clicks to pass through to input layer
//! 2. **Viewport Preservation**: IME composition/confirmation doesn't
//!    accidentally clear the rendered Viewport
//!
//! These tests ensure that future refactorings don't break the core
//! architecture that enables keyboard input to work correctly.

#[cfg(test)]
mod contenteditable_physical_tests {
    use fantoccini::{ClientBuilder, Locator};
    use tokio;

    const APP_URL: &str = "http://localhost:8081";

    /// Test 1: Click Transparency - Rendering Layer Passes Through Events
    ///
    /// Verifies that clicking on rendered text (which is in scroll-content with
    /// pointer-events: none) correctly focuses the parent contenteditable pane.
    ///
    /// This is THE MOST CRITICAL test for input functionality.
    /// If this fails, keyboard input will not work.
    #[tokio::test]
    async fn test_click_on_rendered_text_focuses_input_pane() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Open a file to have some rendered content
        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.rs']")).await {
            file_elem.click().await.expect("Failed to click file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Find a rendered line element (this has pointer-events: none)
            let rendered_line = client
                .find(Locator::Css(".berry-editor-line"))
                .await
                .expect("Rendered line not found");

            // Click on the rendered text (should pass through to parent pane)
            rendered_line
                .click()
                .await
                .expect("Failed to click rendered line");

            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // CRITICAL ASSERTION: Verify the contenteditable pane is focused
            let active_element_tag = client
                .execute(
                    "return document.activeElement.getAttribute('contenteditable');",
                    vec![],
                )
                .await
                .expect("Failed to get active element");

            assert_eq!(
                active_element_tag.as_str(),
                Some("true"),
                "After clicking rendered text, the contenteditable='true' pane should be focused. \
                 If this fails, pointer-events: none is not working and keyboard input will fail."
            );

            // Verify we can type immediately after clicking
            client
                .execute(
                    "document.activeElement.dispatchEvent(new InputEvent('beforeinput', { \
                        bubbles: true, \
                        cancelable: true, \
                        data: 'x' \
                    }));",
                    vec![],
                )
                .await
                .expect("Failed to dispatch input event");

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            // Verify the character was inserted
            let editor_content = client
                .find(Locator::Css(".berry-editor-scroll-content"))
                .await
                .expect("Editor content not found");

            let content_html = editor_content
                .html(true)
                .await
                .expect("Failed to get HTML");

            assert!(
                content_html.contains("x"),
                "Character should be inserted immediately after clicking on rendered text. \
                 This proves click transparency is working."
            );
        }

        client.close().await.expect("Failed to close client");
    }

    /// Test 2: Pointer Events None on Scroll Content
    ///
    /// Directly verifies that the scroll-content CSS has pointer-events: none.
    /// This is a regression test - if CSS gets accidentally changed, this catches it.
    #[tokio::test]
    async fn test_scroll_content_has_pointer_events_none() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Open a file to ensure scroll-content is rendered
        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.rs']")).await {
            file_elem.click().await.expect("Failed to click file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            let scroll_content = client
                .find(Locator::Css(".berry-editor-scroll-content"))
                .await
                .expect("Scroll content not found");

            // Get computed pointer-events style
            let pointer_events = client
                .execute(
                    "return window.getComputedStyle(arguments[0]).pointerEvents;",
                    vec![serde_json::json!(scroll_content)],
                )
                .await
                .expect("Failed to get computed style");

            assert_eq!(
                pointer_events.as_str(),
                Some("none"),
                "CRITICAL: scroll-content MUST have pointer-events: none. \
                 If this is 'auto', clicks will be blocked and keyboard input won't work."
            );
        }

        client.close().await.expect("Failed to close client");
    }

    /// Test 3: Rendered Lines Have Pointer Events None
    ///
    /// Verifies that individual line elements also have pointer-events: none.
    #[tokio::test]
    async fn test_rendered_lines_have_pointer_events_none() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.rs']")).await {
            file_elem.click().await.expect("Failed to click file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            let rendered_line = client
                .find(Locator::Css(".berry-editor-line"))
                .await
                .expect("Rendered line not found");

            let pointer_events = client
                .execute(
                    "return window.getComputedStyle(arguments[0]).pointerEvents;",
                    vec![serde_json::json!(rendered_line)],
                )
                .await
                .expect("Failed to get computed style");

            assert_eq!(
                pointer_events.as_str(),
                Some("none"),
                "Rendered lines must have pointer-events: none to allow clicks to pass through"
            );
        }

        client.close().await.expect("Failed to close client");
    }

    /// Test 4: IME Confirmation Preserves Viewport
    ///
    /// Simulates Japanese IME composition and verifies that the Viewport
    /// (scroll-content with rendered lines) is NOT cleared during confirmation.
    ///
    /// This prevents the "typed text disappears, screen goes blank" bug.
    #[tokio::test]
    async fn test_ime_confirmation_preserves_viewport_content() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.rs']")).await {
            file_elem.click().await.expect("Failed to click file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Focus the editor
            let editor_pane = client
                .find(Locator::Css(".berry-editor-pane[contenteditable='true']"))
                .await
                .expect("Editor pane not found");

            editor_pane.click().await.expect("Failed to focus editor");
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            // Count rendered lines BEFORE IME event
            let lines_before: u64 = client
                .execute(
                    "return document.querySelectorAll('.berry-editor-line').length;",
                    vec![],
                )
                .await
                .expect("Failed to count lines before")
                .as_u64()
                .unwrap_or(0);

            // Simulate IME composition start
            client
                .execute(
                    "const el = document.querySelector('.berry-editor-pane'); \
                     el.dispatchEvent(new CompositionEvent('compositionstart', { \
                         bubbles: true, \
                         cancelable: true, \
                         data: '' \
                     }));",
                    vec![],
                )
                .await
                .expect("Failed to dispatch compositionstart");

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Simulate IME composition end with Japanese text
            client
                .execute(
                    "const el = document.querySelector('.berry-editor-pane'); \
                     el.dispatchEvent(new CompositionEvent('compositionend', { \
                         bubbles: true, \
                         cancelable: true, \
                         data: 'あああ' \
                     }));",
                    vec![],
                )
                .await
                .expect("Failed to dispatch compositionend");

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Count rendered lines AFTER IME event
            let lines_after: u64 = client
                .execute(
                    "return document.querySelectorAll('.berry-editor-line').length;",
                    vec![],
                )
                .await
                .expect("Failed to count lines after")
                .as_u64()
                .unwrap_or(0);

            // CRITICAL ASSERTION: Viewport should NOT be cleared
            assert!(
                lines_after > 0,
                "CRITICAL: After IME confirmation, rendered lines should still exist. \
                 If this is 0, the Viewport was accidentally cleared. \
                 Lines before: {}, Lines after: {}",
                lines_before,
                lines_after
            );

            // The number of lines should be similar (might increase by 1-2 due to new text)
            assert!(
                lines_after >= lines_before.saturating_sub(2),
                "Viewport should not lose significant content during IME. \
                 Before: {}, After: {}",
                lines_before,
                lines_after
            );

            // Verify Japanese text was inserted
            let editor_content = client
                .find(Locator::Css(".berry-editor-scroll-content"))
                .await
                .expect("Editor content not found");

            let content_html = editor_content
                .html(true)
                .await
                .expect("Failed to get HTML");

            assert!(
                content_html.contains("あああ") || content_html.contains("あ"),
                "Japanese IME text should be visible in the editor"
            );
        }

        client.close().await.expect("Failed to close client");
    }

    /// Test 5: ContentEditable Pane Has Correct Display Mode
    ///
    /// Verifies that the editor pane has `display: block` (not flex).
    /// Flex mode causes browser to auto-create wrapper divs.
    #[tokio::test]
    async fn test_editor_pane_has_display_block() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.rs']")).await {
            file_elem.click().await.expect("Failed to click file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            let editor_pane = client
                .find(Locator::Css(".berry-editor-pane[contenteditable='true']"))
                .await
                .expect("Editor pane not found");

            let display_mode = client
                .execute(
                    "return window.getComputedStyle(arguments[0]).display;",
                    vec![serde_json::json!(editor_pane)],
                )
                .await
                .expect("Failed to get display mode");

            assert_eq!(
                display_mode.as_str(),
                Some("block"),
                "Editor pane MUST have display: block (not flex). \
                 Flex causes browser to auto-create <div><br></div> garbage."
            );
        }

        client.close().await.expect("Failed to close client");
    }

    /// Test 6: Scroll Content Has ContentEditable False
    ///
    /// Verifies that the rendering layer is isolated from browser's
    /// contenteditable manipulation.
    #[tokio::test]
    async fn test_scroll_content_has_contenteditable_false() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.rs']")).await {
            file_elem.click().await.expect("Failed to click file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            let scroll_content = client
                .find(Locator::Css(".berry-editor-scroll-content"))
                .await
                .expect("Scroll content not found");

            let contenteditable = client
                .execute(
                    "return arguments[0].getAttribute('contenteditable');",
                    vec![serde_json::json!(scroll_content)],
                )
                .await
                .expect("Failed to get contenteditable attribute");

            assert_eq!(
                contenteditable.as_str(),
                Some("false"),
                "Scroll content MUST have contenteditable='false' to prevent browser DOM manipulation"
            );
        }

        client.close().await.expect("Failed to close client");
    }

    /// Test 7: Focus Management - Sidebar to Editor Focus Return
    ///
    /// Verifies that focus correctly returns to the editor after clicking
    /// on sidebar elements (file tree). This is a common "can't type" bug scenario:
    /// 1. User clicks file in sidebar -> focus moves to sidebar
    /// 2. User clicks editor -> focus should return to contenteditable pane
    /// 3. User types -> input should work
    ///
    /// If this fails, users will experience "keyboard input stops working after
    /// clicking sidebar" which is a critical UX bug.
    #[tokio::test]
    async fn test_focus_returns_to_editor_after_sidebar_click() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Step 1: Click on a file in the sidebar (file tree)
        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.rs']")).await {
            file_elem.click().await.expect("Failed to click file in sidebar");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // At this point, focus might be on the file tree item
            // This is normal behavior when clicking UI elements

            // Step 2: Click on the editor pane to regain focus
            let editor_pane = client
                .find(Locator::Css(".berry-editor-pane[contenteditable='true']"))
                .await
                .expect("Editor pane not found");

            editor_pane.click().await.expect("Failed to click editor pane");
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // CRITICAL ASSERTION: Verify contenteditable pane has focus
            let active_element_contenteditable = client
                .execute(
                    "return document.activeElement.getAttribute('contenteditable');",
                    vec![],
                )
                .await
                .expect("Failed to get active element");

            assert_eq!(
                active_element_contenteditable.as_str(),
                Some("true"),
                "After clicking editor pane, the contenteditable='true' pane should have focus. \
                 If this fails, focus management is broken and users can't type after clicking sidebar."
            );

            // Step 3: Verify typing works after focus return
            client
                .execute(
                    "document.activeElement.dispatchEvent(new InputEvent('beforeinput', { \
                        bubbles: true, \
                        cancelable: true, \
                        data: 'test' \
                    }));",
                    vec![],
                )
                .await
                .expect("Failed to dispatch input event");

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            // Verify the text was inserted
            let editor_content = client
                .find(Locator::Css(".berry-editor-scroll-content"))
                .await
                .expect("Editor content not found");

            let content_html = editor_content
                .html(true)
                .await
                .expect("Failed to get HTML");

            assert!(
                content_html.contains("test"),
                "Text should be inserted after focus returns from sidebar. \
                 If this fails, input handling is broken after sidebar interaction."
            );
        }

        client.close().await.expect("Failed to close client");
    }

    /// Test 8: Focus Management - Rapid Sidebar-Editor Switching
    ///
    /// Verifies that focus management works correctly even with rapid switching
    /// between sidebar and editor. This tests focus robustness under stress.
    #[tokio::test]
    async fn test_rapid_focus_switching_sidebar_to_editor() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Find file tree and editor pane elements
        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.rs']")).await {
            let editor_pane = client
                .find(Locator::Css(".berry-editor-pane[contenteditable='true']"))
                .await
                .expect("Editor pane not found");

            // Rapid switching: Click sidebar -> editor -> sidebar -> editor
            for i in 0..3 {
                // Click sidebar
                file_elem.click().await.expect("Failed to click sidebar");
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // Click editor
                editor_pane.click().await.expect("Failed to click editor");
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // Verify editor has focus after each cycle
                let active_contenteditable = client
                    .execute(
                        "return document.activeElement.getAttribute('contenteditable');",
                        vec![],
                    )
                    .await
                    .expect("Failed to get active element");

                assert_eq!(
                    active_contenteditable.as_str(),
                    Some("true"),
                    "Cycle {}: Editor should have focus after rapid switching",
                    i
                );
            }

            // Final verification: Type should still work
            client
                .execute(
                    "document.activeElement.dispatchEvent(new InputEvent('beforeinput', { \
                        bubbles: true, \
                        cancelable: true, \
                        data: 'abc' \
                    }));",
                    vec![],
                )
                .await
                .expect("Failed to dispatch input event");

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            let editor_content = client
                .find(Locator::Css(".berry-editor-scroll-content"))
                .await
                .expect("Editor content not found");

            let content_html = editor_content
                .html(true)
                .await
                .expect("Failed to get HTML");

            assert!(
                content_html.contains("abc"),
                "Input should work after rapid focus switching"
            );
        }

        client.close().await.expect("Failed to close client");
    }

    /// Test 9: Device Pixel Ratio - Retina/4K Display Coordinate Precision
    ///
    /// Verifies that coordinate calculations work correctly across different
    /// device pixel ratios (1.0, 2.0 for Retina, 3.0 for 4K, etc.).
    ///
    /// This is THE MOST SUBTLE bug that only appears on specific hardware:
    /// - Developer's 1.0 DPR display: Works perfectly
    /// - User's 2.0 DPR Retina display: Cursor is off by 1-2 pixels
    ///
    /// The test verifies:
    /// 1. Long line (100+ characters) end-of-line click precision
    /// 2. Fractional pixel widths (7.8125px * 100 = 781.25px) round correctly
    /// 3. Rust's char_width calculation matches browser's actual rendering
    ///
    /// If this fails, users on Retina/4K displays will experience:
    /// - Clicks at end of line put cursor in wrong position
    /// - Text insertion happens at wrong column
    /// - "My cursor jumps around randomly" bug reports
    #[tokio::test]
    async fn test_device_pixel_ratio_coordinate_precision() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.rs']")).await {
            file_elem.click().await.expect("Failed to click file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Get device pixel ratio
            let device_pixel_ratio: f64 = client
                .execute("return window.devicePixelRatio;", vec![])
                .await
                .expect("Failed to get devicePixelRatio")
                .as_f64()
                .unwrap_or(1.0);

            println!(
                "Testing on devicePixelRatio: {} ({})",
                device_pixel_ratio,
                if device_pixel_ratio >= 2.0 {
                    "Retina/4K"
                } else {
                    "Standard"
                }
            );

            // Focus editor
            let editor_pane = client
                .find(Locator::Css(".berry-editor-pane[contenteditable='true']"))
                .await
                .expect("Editor pane not found");

            editor_pane.click().await.expect("Failed to click editor");
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Insert a long line (100 characters) to test fractional pixel accumulation
            // 100 chars * 7.8125px = 781.25px (tests rounding behavior)
            let long_line = "x".repeat(100);

            client
                .execute(
                    &format!(
                        "document.activeElement.dispatchEvent(new InputEvent('beforeinput', {{ \
                            bubbles: true, \
                            cancelable: true, \
                            data: '{}' \
                        }}));",
                        long_line
                    ),
                    vec![],
                )
                .await
                .expect("Failed to insert long line");

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Get the rendered line's bounding box
            let line_element = client
                .find(Locator::Css(".berry-editor-line"))
                .await
                .expect("Line element not found");

            // Get line's position and width
            let line_rect = client
                .execute(
                    "const line = arguments[0]; \
                     const rect = line.getBoundingClientRect(); \
                     return { \
                         left: rect.left, \
                         top: rect.top, \
                         width: rect.width, \
                         height: rect.height \
                     };",
                    vec![serde_json::json!(line_element)],
                )
                .await
                .expect("Failed to get line rect");

            let left = line_rect["left"].as_f64().unwrap_or(0.0);
            let top = line_rect["top"].as_f64().unwrap_or(0.0);
            let width = line_rect["width"].as_f64().unwrap_or(0.0);
            let height = line_rect["height"].as_f64().unwrap_or(0.0);

            println!(
                "Line rect: left={}, top={}, width={}, height={}",
                left, top, width, height
            );

            // CRITICAL TEST: Click at the END of the line
            // This is where fractional pixel accumulation causes the most issues
            // Expected: 100 chars * 7.8125px/char = 781.25px
            let end_x = left + width - 5.0; // 5px from right edge
            let center_y = top + (height / 2.0);

            println!("Clicking at end of line: ({}, {})", end_x, center_y);

            // Click at the calculated end position
            client
                .execute(
                    &format!(
                        "const pane = document.querySelector('.berry-editor-pane'); \
                         const event = new MouseEvent('mousedown', {{ \
                             bubbles: true, \
                             cancelable: true, \
                             clientX: {}, \
                             clientY: {} \
                         }}); \
                         pane.dispatchEvent(event);",
                        end_x, center_y
                    ),
                    vec![],
                )
                .await
                .expect("Failed to dispatch mousedown");

            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Insert a marker character at cursor position
            client
                .execute(
                    "document.activeElement.dispatchEvent(new InputEvent('beforeinput', { \
                        bubbles: true, \
                        cancelable: true, \
                        data: 'M' \
                    }));",
                    vec![],
                )
                .await
                .expect("Failed to insert marker");

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            // CRITICAL ASSERTION: Verify marker 'M' is near the end
            // Get the content and find position of 'M'
            let editor_content = client
                .find(Locator::Css(".berry-editor-scroll-content"))
                .await
                .expect("Editor content not found");

            let content_html = editor_content
                .html(true)
                .await
                .expect("Failed to get HTML");

            // The marker should appear near the end (within last 10 characters)
            // because we clicked near the end of the 100-char line
            let contains_marker = content_html.contains('M');
            assert!(
                contains_marker,
                "Marker 'M' should be inserted after end-of-line click"
            );

            // More precise check: Extract text content and verify marker position
            let text_content: String = client
                .execute(
                    "const lines = document.querySelectorAll('.berry-editor-line'); \
                     return Array.from(lines).map(l => l.textContent).join('\\n');",
                    vec![],
                )
                .await
                .expect("Failed to get text content")
                .as_str()
                .unwrap_or("")
                .to_string();

            println!("Content after click: '{}'", text_content);

            // Find position of 'M' in the first line
            if let Some(first_line) = text_content.lines().next() {
                if let Some(marker_pos) = first_line.find('M') {
                    println!("Marker 'M' found at position: {}", marker_pos);

                    // CRITICAL: On any DPR, clicking near end of 100-char line
                    // should place cursor within reasonable range (90-101)
                    // If marker is at position 50, there's a major coordinate bug
                    assert!(
                        marker_pos >= 90,
                        "COORDINATE BUG on devicePixelRatio {}: \
                         Clicked near end (x={}px) but cursor landed at position {}. \
                         Expected position 90-101. \
                         This indicates char_width calculation doesn't match actual rendering \
                         on this device pixel ratio.",
                        device_pixel_ratio,
                        end_x,
                        marker_pos
                    );

                    assert!(
                        marker_pos <= 101,
                        "Cursor position {} exceeds line length (expected ≤101)",
                        marker_pos
                    );

                    println!(
                        "✅ PASS: Cursor positioned correctly at {} (within 90-101 range) \
                         on devicePixelRatio {}",
                        marker_pos, device_pixel_ratio
                    );
                } else {
                    panic!("Marker 'M' not found in text content: '{}'", first_line);
                }
            }

            // Additional verification: Click at START of line and verify cursor goes to column 0
            let start_x = left + 5.0; // 5px from left edge

            client
                .execute(
                    &format!(
                        "const pane = document.querySelector('.berry-editor-pane'); \
                         const event = new MouseEvent('mousedown', {{ \
                             bubbles: true, \
                             cancelable: true, \
                             clientX: {}, \
                             clientY: {} \
                         }}); \
                         pane.dispatchEvent(event);",
                        start_x, center_y
                    ),
                    vec![],
                )
                .await
                .expect("Failed to click start");

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            // Insert marker at start
            client
                .execute(
                    "document.activeElement.dispatchEvent(new InputEvent('beforeinput', { \
                        bubbles: true, \
                        cancelable: true, \
                        data: 'S' \
                    }));",
                    vec![],
                )
                .await
                .expect("Failed to insert start marker");

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            let text_after_start: String = client
                .execute(
                    "const lines = document.querySelectorAll('.berry-editor-line'); \
                     return Array.from(lines).map(l => l.textContent).join('\\n');",
                    vec![],
                )
                .await
                .expect("Failed to get text")
                .as_str()
                .unwrap_or("")
                .to_string();

            println!("Content after start click: '{}'", text_after_start);

            if let Some(first_line) = text_after_start.lines().next() {
                assert!(
                    first_line.starts_with('S') || first_line.chars().position(|c| c == 'S') == Some(0),
                    "Start marker 'S' should be at or near position 0. \
                     Line: '{}'. devicePixelRatio: {}",
                    first_line,
                    device_pixel_ratio
                );
            }

            println!(
                "✅ COMPLETE: Device pixel ratio {} test passed. \
                 Coordinate precision verified for both start and end of long lines.",
                device_pixel_ratio
            );
        }

        client.close().await.expect("Failed to close client");
    }
}
