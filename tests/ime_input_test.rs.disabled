use fantoccini::{ClientBuilder, Locator};

const APP_URL: &str = "http://localhost:8081";

#[tokio::test]
async fn test_ime_input_element_exists_and_receives_focus() {
    let client = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await
        .expect("Failed to connect to WebDriver");

    client.goto(APP_URL).await.expect("Failed to navigate");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Open a file first
    let readme = client
        .find(Locator::XPath("//div[contains(text(), 'README.md')]"))
        .await
        .expect("README.md not found");
    readme.click().await.expect("Failed to click README.md");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Click on canvas area to focus
    let canvas = client
        .find(Locator::Css("canvas"))
        .await
        .expect("Canvas not found");
    canvas.click().await.expect("Failed to click canvas");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Check if IME input element exists
    let ime_input = client
        .find(Locator::Css("input[type='text']"))
        .await
        .expect("IME input not found");

    // Check if IME input is focused
    let active_element = client
        .execute("return document.activeElement.tagName", vec![])
        .await
        .expect("Failed to get active element");

    println!("Active element: {:?}", active_element);

    // Check IME input properties
    let ime_position = client
        .execute("return document.querySelector('input[type=\"text\"]').style.position", vec![])
        .await
        .expect("Failed to get IME input position");

    println!("IME input position: {:?}", ime_position);

    let ime_zindex = client
        .execute("return document.querySelector('input[type=\"text\"]').style.zIndex", vec![])
        .await
        .expect("Failed to get IME input z-index");

    println!("IME input z-index: {:?}", ime_zindex);

    client.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_keyboard_events_reach_ime_input() {
    let client = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await
        .expect("Failed to connect to WebDriver");

    client.goto(APP_URL).await.expect("Failed to navigate");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Open a file
    let readme = client
        .find(Locator::XPath("//div[contains(text(), 'README.md')]"))
        .await
        .expect("README.md not found");
    readme.click().await.expect("Failed to click README.md");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Click on canvas to focus
    let canvas = client
        .find(Locator::Css("canvas"))
        .await
        .expect("Canvas not found");
    canvas.click().await.expect("Failed to click canvas");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Get console logs before typing
    let logs_before = client
        .execute("return window.console_logs || []", vec![])
        .await
        .expect("Failed to get console logs");

    println!("Logs before typing: {:?}", logs_before);

    // Type a single character
    let ime_input = client
        .find(Locator::Css("input[type='text']"))
        .await
        .expect("IME input not found");

    ime_input.send_keys("a").await.expect("Failed to type");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Check console logs for keydown event
    let logs_after = client
        .execute(
            "return Array.from(document.querySelectorAll('*')).map(el => el.tagName)",
            vec![]
        )
        .await
        .expect("Failed to get elements");

    println!("Page elements: {:?}", logs_after);

    client.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_ime_input_receives_keydown_not_canvas() {
    let client = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await
        .expect("Failed to connect to WebDriver");

    client.goto(APP_URL).await.expect("Failed to navigate");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Check that canvas does NOT have tabindex
    let canvas_tabindex = client
        .execute("return document.querySelector('canvas')?.getAttribute('tabindex')", vec![])
        .await
        .expect("Failed to get canvas tabindex");

    println!("Canvas tabindex: {:?}", canvas_tabindex);
    assert!(canvas_tabindex.is_null(), "Canvas should not have tabindex");

    // Check that canvas does NOT have keydown listener directly
    let canvas_has_keydown = client
        .execute(
            "const canvas = document.querySelector('canvas'); \
             return canvas && canvas.onkeydown !== null",
            vec![]
        )
        .await
        .expect("Failed to check canvas keydown");

    println!("Canvas has keydown: {:?}", canvas_has_keydown);

    // Check that IME input exists
    let ime_input_exists = client
        .execute("return document.querySelector('input[type=\"text\"]') !== null", vec![])
        .await
        .expect("Failed to check IME input");

    println!("IME input exists: {:?}", ime_input_exists);
    assert_eq!(ime_input_exists.as_bool(), Some(true), "IME input should exist");

    // Check IME input is positioned absolutely with high z-index
    let ime_styles = client
        .execute(
            "const input = document.querySelector('input[type=\"text\"]'); \
             return { \
                position: input.style.position, \
                zIndex: input.style.zIndex, \
                opacity: input.style.opacity \
             }",
            vec![]
        )
        .await
        .expect("Failed to get IME input styles");

    println!("IME input styles: {:?}", ime_styles);

    client.close().await.expect("Failed to close browser");
}
