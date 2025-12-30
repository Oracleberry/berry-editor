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
}
