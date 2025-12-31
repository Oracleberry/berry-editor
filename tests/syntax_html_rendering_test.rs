//! Syntax Highlighting HTML Rendering Test
//!
//! Verifies that syntax highlighting HTML is correctly rendered via innerHTML,
//! not escaped as text. This test catches the regression where `inner_html`
//! was used instead of `prop:innerHTML` in Leptos 0.7.

#![cfg(not(target_arch = "wasm32"))]

use fantoccini::{Client, ClientBuilder};
use serde_json::json;
use std::time::Duration;

async fn setup_client() -> Result<Client, Box<dyn std::error::Error>> {
    let mut caps = serde_json::map::Map::new();
    let opts = json!({
        "args": ["--headless"]
    });
    caps.insert("moz:firefoxOptions".to_string(), opts);

    let client = ClientBuilder::native()
        .capabilities(caps)
        .connect("http://localhost:4444")
        .await?;

    client.goto("http://localhost:8081").await?;
    tokio::time::sleep(Duration::from_millis(3000)).await;

    Ok(client)
}

#[tokio::test]
#[ignore]
async fn test_syntax_highlighting_not_escaped() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    println!("🔍 Testing that syntax highlighting HTML is rendered, not escaped...");

    // Wait for file tree to load
    tokio::time::sleep(Duration::from_millis(2000)).await;

    // Click on build.rs file in file tree
    client
        .execute(
            r#"
        const buildRs = Array.from(document.querySelectorAll('.file-tree-item'))
            .find(el => el.textContent.includes('build.rs'));
        if (buildRs) {
            buildRs.click();
        }
        "#,
            vec![],
        )
        .await?;

    tokio::time::sleep(Duration::from_millis(2000)).await;

    // Check that HTML tags are NOT visible as text
    let has_escaped_html: bool = client
        .execute(
            r#"
        const lines = document.querySelectorAll('.berry-editor-line');
        let foundEscapedHtml = false;

        for (const line of lines) {
            const text = line.textContent || '';
            // Check if HTML tags appear as literal text
            if (text.includes('<span') || text.includes('</span>')) {
                console.error('❌ FOUND ESCAPED HTML:', text.substring(0, 100));
                foundEscapedHtml = true;
                break;
            }
        }

        return foundEscapedHtml;
        "#,
            vec![],
        )
        .await?
        .as_bool()
        .unwrap_or(false);

    if has_escaped_html {
        panic!("❌ REGRESSION: HTML tags are escaped and visible as text! The `prop:innerHTML` fix is broken.");
    }

    println!("✅ HTML tags are NOT escaped - rendering correctly");

    // Verify that actual <span> elements exist in DOM
    let span_count: i64 = client
        .execute(
            r#"
        const lines = document.querySelectorAll('.berry-editor-line');
        let totalSpans = 0;

        for (const line of lines) {
            const spans = line.querySelectorAll('span[style*="color"]');
            totalSpans += spans.length;
        }

        console.log('📊 Found', totalSpans, 'colored <span> elements');
        return totalSpans;
        "#,
            vec![],
        )
        .await?
        .as_i64()
        .unwrap_or(0);

    if span_count == 0 {
        panic!("❌ FAILED: No <span> elements with color styles found! Syntax highlighting is broken.");
    }

    println!("✅ Found {} colored <span> elements in DOM", span_count);

    // Verify that first line has actual colored spans (not plain text)
    let first_line_html: String = client
        .execute(
            r#"
        const firstLine = document.querySelector('.berry-editor-line');
        return firstLine ? firstLine.innerHTML : '';
        "#,
            vec![],
        )
        .await?
        .as_str()
        .unwrap_or("")
        .to_string();

    println!("📝 First line HTML: {}", &first_line_html[..first_line_html.len().min(200)]);

    if !first_line_html.contains("<span style=\"color:") {
        panic!("❌ FAILED: First line does not contain colored span elements!");
    }

    println!("✅ First line contains proper <span> tags with inline styles");

    // Verify text content is clean (no HTML tags)
    let first_line_text: String = client
        .execute(
            r#"
        const firstLine = document.querySelector('.berry-editor-line');
        return firstLine ? firstLine.textContent : '';
        "#,
            vec![],
        )
        .await?
        .as_str()
        .unwrap_or("")
        .to_string();

    println!("📝 First line text: {}", first_line_text);

    if first_line_text.contains("<span") || first_line_text.contains("</span>") {
        panic!("❌ REGRESSION DETECTED: HTML tags are visible as text! Check `prop:innerHTML` usage.");
    }

    println!("✅ Text content is clean - no HTML tags visible");

    println!("\n✅ SUCCESS: Syntax highlighting HTML is correctly rendered via innerHTML!");
    println!("   - No escaped HTML tags in text content");
    println!("   - Actual <span> elements exist in DOM");
    println!("   - Colors are applied via inline styles");

    client.close().await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_prop_innerhtml_on_file_open() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    println!("🔍 Testing innerHTML rendering immediately after opening file...");

    // Wait for app to load
    tokio::time::sleep(Duration::from_millis(2000)).await;

    // Open Cargo.toml (should have syntax highlighting)
    client
        .execute(
            r#"
        const cargoToml = Array.from(document.querySelectorAll('.file-tree-item'))
            .find(el => el.textContent.includes('Cargo.toml'));
        if (cargoToml) {
            cargoToml.click();
        }
        "#,
            vec![],
        )
        .await?;

    // Check immediately (within 500ms) - regression often appears on initial render
    tokio::time::sleep(Duration::from_millis(500)).await;

    let has_escaped_html_on_load: bool = client
        .execute(
            r#"
        const lines = document.querySelectorAll('.berry-editor-line');
        for (const line of lines) {
            const text = line.textContent || '';
            if (text.includes('<span') || text.includes('&lt;span')) {
                console.error('❌ ESCAPED HTML ON LOAD:', text.substring(0, 100));
                return true;
            }
        }
        return false;
        "#,
            vec![],
        )
        .await?
        .as_bool()
        .unwrap_or(false);

    if has_escaped_html_on_load {
        panic!("❌ CRITICAL REGRESSION: HTML is escaped on initial file load!");
    }

    println!("✅ SUCCESS: No escaped HTML on initial file load");

    client.close().await?;
    Ok(())
}
