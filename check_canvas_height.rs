//! Quick test to check actual canvas height in browser
use fantoccini::{ClientBuilder, Locator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::native()
        .connect("http://localhost:4444")
        .await?;

    client.goto("http://localhost:8081").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    println!("=== Initial state ===");
    let initial_container = client
        .execute(
            "const container = document.querySelector('.berry-editor-main'); \
             if (container) { \
                 const rect = container.getBoundingClientRect(); \
                 return { width: rect.width, height: rect.height }; \
             } \
             return null;",
            vec![],
        )
        .await?;
    println!("Container: {:?}", initial_container);

    let initial_canvas = client
        .execute(
            "const canvas = document.querySelector('canvas'); \
             return canvas ? { width: canvas.width, height: canvas.height } : null;",
            vec![],
        )
        .await?;
    println!("Canvas: {:?}", initial_canvas);

    // Click a file
    println!("\n=== Clicking a .rs file ===");
    if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.rs']")).await {
        file_elem.click().await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let after_container = client
            .execute(
                "const container = document.querySelector('.berry-editor-main'); \
                 if (container) { \
                     const rect = container.getBoundingClientRect(); \
                     return { width: rect.width, height: rect.height }; \
                 } \
                 return null;",
                vec![],
            )
            .await?;
        println!("Container after file load: {:?}", after_container);

        let after_canvas = client
            .execute(
                "const canvas = document.querySelector('canvas'); \
                 return canvas ? { width: canvas.width, height: canvas.height } : null;",
                vec![],
            )
            .await?;
        println!("Canvas after file load: {:?}", after_canvas);

        // Check parent elements
        let parent_info = client
            .execute(
                "const container = document.querySelector('.berry-editor-main'); \
                 if (container && container.parentElement) { \
                     const rect = container.parentElement.getBoundingClientRect(); \
                     return { \
                         className: container.parentElement.className, \
                         width: rect.width, \
                         height: rect.height \
                     }; \
                 } \
                 return null;",
                vec![],
            )
            .await?;
        println!("Parent element: {:?}", parent_info);
    }

    client.close().await?;
    Ok(())
}
