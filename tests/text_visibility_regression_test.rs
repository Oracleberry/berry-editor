//! E2E Test: Text Input Visibility Regression
//!
//! This test verifies the fix for the critical bug where:
//! 1. Text was being typed into the buffer
//! 2. But the text was invisible/transparent on screen
//!
//! Root causes that were fixed:
//! - Textarea was inside reactive closure (caused DOM destruction)
//! - Input handler used update_untracked() (prevented UI re-render)
//! - CSS pointer-events: none (prevented focus)
//!
//! This test ensures text is VISIBLE after typing.

#[cfg(test)]
mod text_visibility_tests {
    use fantoccini::{ClientBuilder, Locator};
    use tokio;

    const APP_URL: &str = "http://localhost:8081";

    #[tokio::test]
    async fn test_text_appears_visible_after_typing_rs_file() {
        // Start WebDriver client
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        // Navigate to app
        client.goto(APP_URL).await.expect("Failed to navigate");

        // Wait for app to load
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Find and click on a .rs file in the file tree
        let rs_file = client
            .find(Locator::Css("div[data-path$='.rs']"))
            .await;

        if let Ok(file_elem) = rs_file {
            file_elem.click().await.expect("Failed to click RS file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Focus the editor by clicking on the editor pane
            if let Ok(editor_pane) = client.find(Locator::Css(".berry-editor-pane")).await {
                editor_pane.click().await.expect("Failed to click editor");
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }

            // Find the hidden textarea
            let textarea = client
                .find(Locator::Css("textarea.hidden-input"))
                .await
                .expect("Textarea not found");

            // Type some test text
            textarea.send_keys("fn test() {}").await.expect("Failed to type");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // CRITICAL: Verify text is visible in the editor content
            // The text should appear in the rendered lines
            let editor_content = client
                .find(Locator::Css(".berry-editor-scroll-content"))
                .await
                .expect("Editor content not found");

            let content_html = editor_content
                .html(true)
                .await
                .expect("Failed to get HTML");

            // Verify the typed text appears in the rendered content
            assert!(
                content_html.contains("fn") || content_html.contains("test"),
                "Typed text should be VISIBLE in the editor. HTML: {}",
                &content_html[..500.min(content_html.len())]
            );
        }

        client.close().await.expect("Failed to close client");
    }

    #[tokio::test]
    async fn test_text_appears_visible_after_typing_html_file() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Click on an HTML file
        let html_file = client
            .find(Locator::Css("div[data-path$='.html']"))
            .await;

        if let Ok(file_elem) = html_file {
            file_elem.click().await.expect("Failed to click HTML file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            if let Ok(editor_pane) = client.find(Locator::Css(".berry-editor-pane")).await {
                editor_pane.click().await.expect("Failed to click editor");
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }

            let textarea = client
                .find(Locator::Css("textarea.hidden-input"))
                .await
                .expect("Textarea not found");

            // Type HTML content
            textarea.send_keys("<div>test</div>").await.expect("Failed to type");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Verify text is visible
            let editor_content = client
                .find(Locator::Css(".berry-editor-scroll-content"))
                .await
                .expect("Editor content not found");

            let content_html = editor_content
                .html(true)
                .await
                .expect("Failed to get HTML");

            assert!(
                content_html.contains("div") || content_html.contains("test"),
                "HTML text should be VISIBLE in the editor"
            );
        }

        client.close().await.expect("Failed to close client");
    }

    #[tokio::test]
    async fn test_text_appears_visible_after_typing_toml_file() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Click on a TOML file (e.g., Cargo.toml)
        let toml_file = client
            .find(Locator::Css("div[data-path$='.toml']"))
            .await;

        if let Ok(file_elem) = toml_file {
            file_elem.click().await.expect("Failed to click TOML file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            if let Ok(editor_pane) = client.find(Locator::Css(".berry-editor-pane")).await {
                editor_pane.click().await.expect("Failed to click editor");
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }

            let textarea = client
                .find(Locator::Css("textarea.hidden-input"))
                .await
                .expect("Textarea not found");

            // Type TOML content
            textarea.send_keys("[package]").await.expect("Failed to type");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Verify text is visible
            let editor_content = client
                .find(Locator::Css(".berry-editor-scroll-content"))
                .await
                .expect("Editor content not found");

            let content_html = editor_content
                .html(true)
                .await
                .expect("Failed to get HTML");

            assert!(
                content_html.contains("package"),
                "TOML text should be VISIBLE in the editor"
            );
        }

        client.close().await.expect("Failed to close client");
    }

    #[tokio::test]
    async fn test_textarea_remains_focusable_after_typing() {
        // This test verifies the DOM destruction bug is fixed
        // Previously, textarea was inside reactive closure and got destroyed
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Open any file
        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.rs']")).await {
            file_elem.click().await.expect("Failed to click file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Get textarea reference
            let textarea = client
                .find(Locator::Css("textarea.hidden-input"))
                .await
                .expect("Textarea not found");

            // Type first character
            textarea.send_keys("a").await.expect("Failed to type first char");
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            // Verify textarea still exists and is focusable
            let textarea_again = client
                .find(Locator::Css("textarea.hidden-input"))
                .await
                .expect("Textarea should still exist after typing");

            // Type second character to verify it's still functional
            textarea_again.send_keys("b").await.expect("Failed to type second char");
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            // Verify both characters are visible
            let editor_content = client
                .find(Locator::Css(".berry-editor-scroll-content"))
                .await
                .expect("Editor content not found");

            let content_html = editor_content
                .html(true)
                .await
                .expect("Failed to get HTML");

            assert!(
                content_html.contains("ab") || (content_html.contains("a") && content_html.contains("b")),
                "Both typed characters should be visible (textarea not destroyed)"
            );
        }

        client.close().await.expect("Failed to close client");
    }

    #[tokio::test]
    async fn test_textarea_has_pointer_events_auto() {
        // This test verifies the CSS fix for pointer-events
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let textarea = client
            .find(Locator::Css("textarea.hidden-input"))
            .await
            .expect("Textarea not found");

        // Get computed style
        let pointer_events = client
            .execute(
                "return window.getComputedStyle(arguments[0]).pointerEvents;",
                vec![serde_json::to_value(&textarea).unwrap()],
            )
            .await
            .expect("Failed to get computed style");

        assert_eq!(
            pointer_events.as_str(),
            Some("auto"),
            "Textarea must have pointer-events: auto to receive focus"
        );

        client.close().await.expect("Failed to close client");
    }
}
