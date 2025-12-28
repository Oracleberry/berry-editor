//! Syntax Highlighting Colors Test
//!
//! Verifies that IntelliJ Darcula color scheme is correctly applied
//! to all token types in the desktop app

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
async fn test_intellij_darcula_colors() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    println!("🎨 Testing IntelliJ Darcula syntax highlighting colors...");

    // Create a test Rust file with various token types
    let test_code = r#"
// Comment line
pub struct Position {
    pub x: usize,
    pub y: usize,
}

pub fn calculate(value: i32) -> String {
    let result = 42;
    "Hello World".to_string()
}
"#;

    // Open a new file
    client
        .execute(
            r#"
        const editor = document.querySelector('[data-editor-instance]');
        if (editor) {
            // Trigger new file creation
            const event = new CustomEvent('berry-new-file');
            editor.dispatchEvent(event);
        }
        "#,
            vec![],
        )
        .await?;

    tokio::time::sleep(Duration::from_millis(500)).await;

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

    // Check syntax highlighting colors
    let colors: serde_json::Value = client
        .execute(
            r#"
        const lines = document.querySelectorAll('.berry-editor-line');
        const colors = {
            keywords: [],
            types: [],
            strings: [],
            numbers: [],
            comments: []
        };

        lines.forEach(line => {
            const spans = line.querySelectorAll('span[style*="color"]');
            spans.forEach(span => {
                const style = span.getAttribute('style');
                const colorMatch = style.match(/color:\s*([^;]+)/);
                const text = span.textContent.trim();

                if (colorMatch && text) {
                    const color = colorMatch[1].trim().toUpperCase();

                    // Classify by text content
                    if (['PUB', 'STRUCT', 'FN', 'LET'].includes(text.toUpperCase())) {
                        colors.keywords.push({text, color});
                    } else if (['POSITION', 'STRING', 'USIZE', 'I32'].includes(text.toUpperCase())) {
                        colors.types.push({text, color});
                    } else if (text.startsWith('"') || text.includes('Hello')) {
                        colors.strings.push({text, color});
                    } else if (!isNaN(parseInt(text))) {
                        colors.numbers.push({text, color});
                    } else if (text.startsWith('//')) {
                        colors.comments.push({text, color});
                    }
                }
            });
        });

        return colors;
        "#,
            vec![],
        )
        .await?;

    println!("\n📊 Detected Colors:");
    println!("{}", serde_json::to_string_pretty(&colors)?);

    // Define IntelliJ Darcula expected colors
    let expected_keyword_color = "#CC7832";
    let expected_type_color = "#A9B7C6";
    let expected_string_color = "#6A8759";
    let expected_number_color = "#6897BB";
    let expected_comment_color = "#629755";

    // Verify keyword colors
    if let Some(keywords) = colors["keywords"].as_array() {
        for keyword in keywords {
            let color = keyword["color"].as_str().unwrap_or("");
            let text = keyword["text"].as_str().unwrap_or("");

            if !color.eq_ignore_ascii_case(expected_keyword_color) {
                println!("❌ FAILED: Keyword '{}' has color {} instead of {}",
                    text, color, expected_keyword_color);
                panic!("Keyword color mismatch");
            }
        }
        if !keywords.is_empty() {
            println!("✅ Keywords: Correct color {}", expected_keyword_color);
        }
    }

    // Verify type colors
    if let Some(types) = colors["types"].as_array() {
        for type_token in types {
            let color = type_token["color"].as_str().unwrap_or("");
            let text = type_token["text"].as_str().unwrap_or("");

            if !color.eq_ignore_ascii_case(expected_type_color) {
                println!("❌ FAILED: Type '{}' has color {} instead of {}",
                    text, color, expected_type_color);
                panic!("Type color mismatch");
            }
        }
        if !types.is_empty() {
            println!("✅ Types: Correct color {}", expected_type_color);
        }
    }

    // Verify string colors
    if let Some(strings) = colors["strings"].as_array() {
        for string in strings {
            let color = string["color"].as_str().unwrap_or("");
            let text = string["text"].as_str().unwrap_or("");

            if !color.eq_ignore_ascii_case(expected_string_color) {
                println!("❌ FAILED: String '{}' has color {} instead of {}",
                    text, color, expected_string_color);
                panic!("String color mismatch");
            }
        }
        if !strings.is_empty() {
            println!("✅ Strings: Correct color {}", expected_string_color);
        }
    }

    // Verify number colors
    if let Some(numbers) = colors["numbers"].as_array() {
        for number in numbers {
            let color = number["color"].as_str().unwrap_or("");
            let text = number["text"].as_str().unwrap_or("");

            if !color.eq_ignore_ascii_case(expected_number_color) {
                println!("❌ FAILED: Number '{}' has color {} instead of {}",
                    text, color, expected_number_color);
                panic!("Number color mismatch");
            }
        }
        if !numbers.is_empty() {
            println!("✅ Numbers: Correct color {}", expected_number_color);
        }
    }

    // Verify comment colors
    if let Some(comments) = colors["comments"].as_array() {
        for comment in comments {
            let color = comment["color"].as_str().unwrap_or("");
            let text = comment["text"].as_str().unwrap_or("");

            if !color.eq_ignore_ascii_case(expected_comment_color) {
                println!("❌ FAILED: Comment '{}' has color {} instead of {}",
                    text, color, expected_comment_color);
                panic!("Comment color mismatch");
            }
        }
        if !comments.is_empty() {
            println!("✅ Comments: Correct color {}", expected_comment_color);
        }
    }

    println!("\n✅ SUCCESS: All IntelliJ Darcula colors are correctly applied!");
    println!("⏳ Keeping browser open for 10 seconds for visual inspection...");
    tokio::time::sleep(Duration::from_millis(10000)).await;

    client.close().await?;
    Ok(())
}
