use fantoccini::{ClientBuilder, Locator};

const APP_URL: &str = "http://localhost:8081";

#[tokio::test]
async fn test_ime_input_can_be_focused_manually() {
    let client = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await
        .expect("Failed to connect to WebDriver");

    client.goto(APP_URL).await.expect("Failed to navigate");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Try to manually focus the IME input via JavaScript
    let focus_result = client
        .execute(
            "const input = document.querySelector('input[type=\"text\"]'); \
             if (input) { \
                 input.focus(); \
                 return { \
                     exists: true, \
                     isFocused: document.activeElement === input, \
                     tagName: document.activeElement.tagName \
                 }; \
             } else { \
                 return { exists: false }; \
             }",
            vec![]
        )
        .await
        .expect("Failed to focus IME input");

    println!("Manual focus result: {:?}", focus_result);

    client.close().await.expect("Failed to close browser");
}
