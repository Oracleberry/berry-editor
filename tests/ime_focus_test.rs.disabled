use fantoccini::{ClientBuilder, Locator};

const APP_URL: &str = "http://localhost:8081";

#[tokio::test]
async fn test_clicking_canvas_focuses_ime_input() {
    let client = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await
        .expect("Failed to connect to WebDriver");

    client.goto(APP_URL).await.expect("Failed to navigate");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Open a file first - click on any file in file tree
    let _file_click = client
        .execute(
            "const fileDiv = document.querySelector('.file-tree-item'); \
             if (fileDiv) { \
                 fileDiv.click(); \
                 return true; \
             } \
             return false",
            vec![]
        )
        .await
        .expect("Failed to click file");

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Get active element before clicking
    let active_before = client
        .execute("return document.activeElement?.tagName", vec![])
        .await
        .expect("Failed to get active element");

    println!("Active element before click: {:?}", active_before);

    // Click on canvas area (center, well past the gutter)
    // First, get canvas position and size
    let canvas_rect = client
        .execute(
            "const canvas = document.querySelector('canvas'); \
             const rect = canvas.getBoundingClientRect(); \
             return { x: rect.left, y: rect.top, width: rect.width, height: rect.height }",
            vec![]
        )
        .await
        .expect("Failed to get canvas rect");

    println!("Canvas rect: {:?}", canvas_rect);

    // Click at center of canvas (well past gutter which is ~70px)
    client
        .execute(
            "const canvas = document.querySelector('canvas'); \
             const rect = canvas.getBoundingClientRect(); \
             const centerX = rect.left + rect.width / 2; \
             const centerY = rect.top + rect.height / 2; \
             const event = new MouseEvent('mousedown', { \
                 view: window, \
                 bubbles: true, \
                 cancelable: true, \
                 clientX: centerX, \
                 clientY: centerY \
             }); \
             canvas.dispatchEvent(event); \
             return { x: centerX, y: centerY }",
            vec![]
        )
        .await
        .expect("Failed to click canvas");

    println!("Clicked canvas at center");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Get console logs to see if mousedown handler was called
    let logs = client
        .execute(
            "const entries = performance.getEntriesByType('mark'); \
             return entries.map(e => e.name)",
            vec![]
        )
        .await
        .expect("Failed to get logs");

    println!("Performance marks: {:?}", logs);

    // Get active element after clicking
    let active_after = client
        .execute("return document.activeElement?.tagName", vec![])
        .await
        .expect("Failed to get active element");

    println!("Active element after click: {:?}", active_after);

    // Check if IME input element is in the DOM
    let ime_in_dom = client
        .execute("return document.querySelector('input[type=\"text\"]') !== null", vec![])
        .await
        .expect("Failed to check IME in DOM");

    println!("IME input in DOM: {:?}", ime_in_dom);

    // Check if IME input is focused
    let active_is_input = client
        .execute(
            "const active = document.activeElement; \
             return active && active.tagName === 'INPUT' && active.type === 'text'",
            vec![]
        )
        .await
        .expect("Failed to check active element");

    println!("Active element is IME input: {:?}", active_is_input);
    assert_eq!(
        active_is_input.as_bool(),
        Some(true),
        "IME input should be focused after clicking canvas"
    );

    client.close().await.expect("Failed to close browser");
}

#[tokio::test]
async fn test_typing_into_ime_input_triggers_keydown() {
    let client = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await
        .expect("Failed to connect to WebDriver");

    client.goto(APP_URL).await.expect("Failed to navigate");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Click on canvas to focus
    let canvas = client
        .find(Locator::Css("canvas"))
        .await
        .expect("Canvas not found");

    canvas.click().await.expect("Failed to click canvas");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Find IME input and type into it
    let ime_input = client
        .find(Locator::Css("input[type='text']"))
        .await
        .expect("IME input not found");

    // Type a character
    ime_input.send_keys("a").await.expect("Failed to type");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Check if input has value (it shouldn't, because keydown handler clears it)
    let input_value = client
        .execute("return document.querySelector('input[type=\"text\"]').value", vec![])
        .await
        .expect("Failed to get input value");

    println!("Input value after typing: {:?}", input_value);

    client.close().await.expect("Failed to close browser");
}
