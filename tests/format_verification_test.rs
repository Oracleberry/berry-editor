//! Format Verification E2E Test
//!
//! Verifies that Rust code formatting (indentation, spacing) is correctly displayed
//! in the desktop app after cargo fmt

use fantoccini::{Client, ClientBuilder};
use serde_json::json;
use std::time::Duration;

async fn setup_client() -> Result<Client, Box<dyn std::error::Error>> {
    let mut caps = serde_json::map::Map::new();
    let opts = json!({
        "args": []
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
async fn test_rust_code_formatting() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    println!("📐 Testing Rust code formatting and indentation...");

    // Well-formatted Rust code with proper indentation
    let test_code = r#"pub struct EditorTab {
    pub path: String,
    pub buffer: Rope,
    pub cursor_line: usize,
    pub cursor_col: usize,
}

impl EditorTab {
    pub fn new(path: String, content: String) -> Self {
        Self {
            path,
            buffer: Rope::from_str(&content),
            cursor_line: 0,
            cursor_col: 0,
        }
    }

    pub fn line_count(&self) -> usize {
        self.buffer.len_lines()
    }
}"#;

    // Insert test code
    client
        .execute(
            &format!(
                r#"
        const textarea = document.querySelector('textarea');
        if (textarea) {{
            textarea.value = `{}`;
            textarea.dispatchEvent(new Event('input', {{ bubbles: true }}));
        }}
        "#,
                test_code.replace('\n', "\\n").replace('`', "\\`")
            ),
            vec![],
        )
        .await?;

    tokio::time::sleep(Duration::from_millis(2000)).await;

    // Get rendered lines with their indentation
    let lines_data: serde_json::Value = client
        .execute(
            r#"
        const lines = document.querySelectorAll('.berry-editor-line');
        const linesData = [];

        lines.forEach((line, index) => {
            const text = line.textContent || '';

            // Count leading spaces by checking non-breaking spaces
            let leadingSpaces = 0;
            let i = 0;
            while (i < text.length && (text[i] === ' ' || text.charCodeAt(i) === 160)) {
                leadingSpaces++;
                i++;
            }

            const trimmedText = text.trim();

            linesData.push({
                index: index,
                text: trimmedText,
                leadingSpaces: leadingSpaces,
                fullText: text
            });
        });

        return linesData;
        "#,
            vec![],
        )
        .await?;

    println!("\n📊 Rendered Lines:");
    if let Some(lines_array) = lines_data.as_array() {
        for line_data in lines_array {
            let index = line_data["index"].as_u64().unwrap_or(0);
            let text = line_data["text"].as_str().unwrap_or("");
            let leading_spaces = line_data["leadingSpaces"].as_u64().unwrap_or(0);

            if !text.is_empty() {
                println!("Line {}: [{}] '{}'", index + 1, leading_spaces, text);
            }
        }
    }

    // Verify specific indentation patterns
    let mut errors = Vec::new();

    // Line 1: "pub struct EditorTab {" - no indentation
    if let Some(line1) = lines_data.get(0) {
        let spaces = line1["leadingSpaces"].as_u64().unwrap_or(999);
        let text = line1["text"].as_str().unwrap_or("");
        if text.starts_with("pub struct") && spaces != 0 {
            errors.push(format!("Line 1: Expected 0 spaces, got {}", spaces));
        }
    }

    // Line 2: "pub path: String," - should have 4 spaces
    if let Some(line2) = lines_data.get(1) {
        let spaces = line2["leadingSpaces"].as_u64().unwrap_or(0);
        let text = line2["text"].as_str().unwrap_or("");
        if text.contains("pub path") && spaces != 4 {
            errors.push(format!("Line 2 (struct field): Expected 4 spaces, got {}", spaces));
        }
    }

    // Line 7: "impl EditorTab {" - no indentation
    if let Some(line7) = lines_data.get(6) {
        let spaces = line7["leadingSpaces"].as_u64().unwrap_or(999);
        let text = line7["text"].as_str().unwrap_or("");
        if text.starts_with("impl EditorTab") && spaces != 0 {
            errors.push(format!("Line 7: Expected 0 spaces, got {}", spaces));
        }
    }

    // Line 8: "pub fn new..." - should have 4 spaces
    if let Some(line8) = lines_data.get(7) {
        let spaces = line8["leadingSpaces"].as_u64().unwrap_or(0);
        let text = line8["text"].as_str().unwrap_or("");
        if text.contains("pub fn new") && spaces != 4 {
            errors.push(format!("Line 8 (method): Expected 4 spaces, got {}", spaces));
        }
    }

    // Line 9: "Self {" - should have 8 spaces
    if let Some(line9) = lines_data.get(8) {
        let spaces = line9["leadingSpaces"].as_u64().unwrap_or(0);
        let text = line9["text"].as_str().unwrap_or("");
        if text.starts_with("Self") && spaces != 8 {
            errors.push(format!("Line 9 (method body): Expected 8 spaces, got {}", spaces));
        }
    }

    // Line 10: "path," - should have 12 spaces
    if let Some(line10) = lines_data.get(9) {
        let spaces = line10["leadingSpaces"].as_u64().unwrap_or(0);
        let text = line10["text"].as_str().unwrap_or("");
        if text.starts_with("path,") && spaces != 12 {
            errors.push(format!("Line 10 (struct literal field): Expected 12 spaces, got {}", spaces));
        }
    }

    println!("\n🔍 Indentation Verification:");
    if errors.is_empty() {
        println!("✅ All indentation levels are correct!");
        println!("   - Top-level declarations: 0 spaces");
        println!("   - Struct fields: 4 spaces");
        println!("   - Method declarations: 4 spaces");
        println!("   - Method body: 8 spaces");
        println!("   - Struct literal fields: 12 spaces");
    } else {
        println!("❌ Indentation errors found:");
        for error in &errors {
            println!("   - {}", error);
        }
    }

    // Verify that code contains expected keywords with proper structure
    let buffer_content: String = client
        .execute(
            r#"
        const testApi = document.querySelector('[data-testid="buffer-state"]');
        return testApi ? testApi.getAttribute('data-buffer-content') || '' : '';
        "#,
            vec![],
        )
        .await?
        .as_str()
        .unwrap_or("")
        .to_string();

    println!("\n📝 Structure Verification:");
    let has_struct = buffer_content.contains("pub struct EditorTab");
    let has_impl = buffer_content.contains("impl EditorTab");
    let has_method = buffer_content.contains("pub fn new");
    let has_self = buffer_content.contains("Self {");

    println!("   struct declaration: {}", if has_struct { "✅" } else { "❌" });
    println!("   impl block: {}", if has_impl { "✅" } else { "❌" });
    println!("   method declaration: {}", if has_method { "✅" } else { "❌" });
    println!("   Self constructor: {}", if has_self { "✅" } else { "❌" });

    println!("\n⏳ Keeping browser open for 10 seconds for visual inspection...");
    tokio::time::sleep(Duration::from_millis(10000)).await;

    // Assert all checks passed
    assert!(errors.is_empty(), "Indentation errors: {:?}", errors);
    assert!(has_struct, "Missing struct declaration");
    assert!(has_impl, "Missing impl block");
    assert!(has_method, "Missing method declaration");
    assert!(has_self, "Missing Self constructor");

    println!("\n✅ SUCCESS: All formatting and indentation is correct!");

    client.close().await?;
    Ok(())
}
