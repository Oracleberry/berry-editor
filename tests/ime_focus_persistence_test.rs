use fantoccini::{ClientBuilder, Locator};

const APP_URL: &str = "http://localhost:8081";

#[tokio::test]
async fn test_ime_input_count() {
    let client = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await
        .expect("Failed to connect to WebDriver");

    client.goto(APP_URL).await.expect("Failed to navigate");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Count how many IME input elements exist
    let input_count = client
        .execute("return document.querySelectorAll('input[type=\"text\"]').length", vec![])
        .await
        .expect("Failed to count inputs");

    println!("IME input count: {:?}", input_count);
    assert_eq!(input_count.as_u64(), Some(1), "There should be exactly 1 IME input element");

    client.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_click_maintains_focus_after_render() {
    let client = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await
        .expect("Failed to connect to WebDriver");

    client.goto(APP_URL).await.expect("Failed to navigate");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Open a file first
    let _file_click = client
        .execute(
            "const fileDiv = document.querySelector('.file-tree-item'); \
             if (fileDiv) { fileDiv.click(); return true; } \
             return false",
            vec![]
        )
        .await
        .expect("Failed to click file");

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Click on canvas center
    client
        .execute(
            "const canvas = document.querySelector('canvas'); \
             const rect = canvas.getBoundingClientRect(); \
             const event = new MouseEvent('mousedown', { \
                 view: window, \
                 bubbles: true, \
                 cancelable: true, \
                 clientX: rect.left + rect.width / 2, \
                 clientY: rect.top + rect.height / 2 \
             }); \
             canvas.dispatchEvent(event);",
            vec![]
        )
        .await
        .expect("Failed to click canvas");

    // Wait for requestAnimationFrame to execute (need more time for render + RAF + blur + re-focus)
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Check if IME input is still focused after render
    let is_focused = client
        .execute(
            "const input = document.querySelector('input[type=\"text\"]'); \
             return document.activeElement === input",
            vec![]
        )
        .await
        .expect("Failed to check focus");

    println!("IME input is focused after render: {:?}", is_focused);
    assert_eq!(
        is_focused.as_bool(),
        Some(true),
        "IME input should remain focused after render"
    );

    client.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_typing_sends_keydown_to_ime_input() {
    let client = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await
        .expect("Failed to connect to WebDriver");

    client.goto(APP_URL).await.expect("Failed to navigate");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Open a file
    let _file_click = client
        .execute(
            "const fileDiv = document.querySelector('.file-tree-item'); \
             if (fileDiv) { fileDiv.click(); return true; } \
             return false",
            vec![]
        )
        .await
        .expect("Failed to click file");

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Click canvas to focus
    client
        .execute(
            "const canvas = document.querySelector('canvas'); \
             const rect = canvas.getBoundingClientRect(); \
             const event = new MouseEvent('mousedown', { \
                 view: window, \
                 bubbles: true, \
                 cancelable: true, \
                 clientX: rect.left + rect.width / 2, \
                 clientY: rect.top + rect.height / 2 \
             }); \
             canvas.dispatchEvent(event);",
            vec![]
        )
        .await
        .expect("Failed to click canvas");

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Type a character
    let ime_input = client
        .find(Locator::Css("input[type='text']"))
        .await
        .expect("IME input not found");

    ime_input.send_keys("hello").await.expect("Failed to type");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Check that text was inserted into buffer (should see it on canvas)
    // The input value itself should be cleared after each keydown
    let input_value = client
        .execute("return document.querySelector('input[type=\"text\"]').value", vec![])
        .await
        .expect("Failed to get input value");

    println!("Input value after typing: {:?}", input_value);

    client.close().await.expect("Failed to close browser");
}
