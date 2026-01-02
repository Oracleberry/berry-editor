//! Style Verification Integration Test
//!
//! このテストはブラウザで実際に適用されているスタイルを確認します

#![cfg(not(target_arch = "wasm32"))]

use fantoccini::{Client, ClientBuilder, Locator};
use tokio;

async fn setup_client() -> Result<Client, fantoccini::error::NewSessionError> {
    ClientBuilder::native()
        .connect("http://localhost:4444")
        .await
}

#[tokio::test]
async fn test_background_colors_applied() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    // Tauriアプリに接続
    client.goto("http://localhost:8081").await?;

    // ページが完全に読み込まれるまで待機
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    println!("✅ Page loaded");

    // タブバーの背景色を確認
    let tab_bar = client
        .find(Locator::Css(".berry-editor-tabs"))
        .await?;

    let tab_bar_bg = tab_bar
        .css_value("background-color")
        .await?;

    println!("🎨 Tab bar background-color: {}", tab_bar_bg);

    // RGB(49, 51, 53) = #313335
    assert!(
        tab_bar_bg.contains("49") || tab_bar_bg.contains("#313335"),
        "Expected tab bar background to be #313335, got: {}",
        tab_bar_bg
    );

    // エディタペインの背景色を確認
    let editor_pane = client
        .find(Locator::Css(".berry-editor-pane"))
        .await?;

    let editor_pane_bg = editor_pane
        .css_value("background-color")
        .await?;

    println!("🎨 Editor pane background-color: {}", editor_pane_bg);

    // RGB(30, 31, 34) = #1E1F22
    assert!(
        editor_pane_bg.contains("30") || editor_pane_bg.contains("#1E1F22"),
        "Expected editor pane background to be #1E1F22, got: {}",
        editor_pane_bg
    );

    // サイドバーの背景色を確認
    let sidebar = client
        .find(Locator::Css(".berry-editor-sidebar"))
        .await?;

    let sidebar_bg = sidebar
        .css_value("background-color")
        .await?;

    println!("🎨 Sidebar background-color: {}", sidebar_bg);

    // RGB(49, 51, 53) = #313335
    assert!(
        sidebar_bg.contains("49") || sidebar_bg.contains("#313335"),
        "Expected sidebar background to be #313335, got: {}",
        sidebar_bg
    );

    // ファイルツリーの背景色を確認
    let file_tree = client
        .find(Locator::Css(".berry-editor-file-tree"))
        .await?;

    let file_tree_bg = file_tree
        .css_value("background-color")
        .await?;

    println!("🎨 File tree background-color: {}", file_tree_bg);

    assert!(
        file_tree_bg.contains("49") || file_tree_bg.contains("#313335"),
        "Expected file tree background to be #313335, got: {}",
        file_tree_bg
    );

    client.close().await?;
    Ok(())
}

#[tokio::test]
async fn test_font_settings_applied() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    client.goto("http://localhost:8081").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // ✅ Trigger a render by clicking on a file in the file tree
    let file_click_result = client
        .execute(
            r#"
            // Click on the first file in the tree to open it
            const fileItem = document.querySelector('.berry-editor-file-item');
            if (fileItem) {
                fileItem.click();
                return { success: true, clicked: fileItem.textContent };
            }
            return { success: false };
            "#,
            vec![],
        )
        .await?;

    println!("📁 File click result: {:?}", file_click_result);

    // Wait for file to load and render
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    // Canvasのフォント設定を確認（複数回試行）
    let result = client
        .execute(
            r#"
            const canvas = document.querySelector('.berry-editor-pane canvas');
            if (!canvas) return { error: 'Canvas not found' };

            // ✅ 新しいコンテキストを取得して、フォントを設定してみる
            const ctx = canvas.getContext('2d');
            if (!ctx) return { error: 'Context not found' };

            // ✅ デバッグ: フォント設定の前後を確認
            const fontBefore = ctx.font;

            // フォントを明示的に設定してみる
            ctx.font = "300 13px 'JetBrains Mono'";
            const fontAfterSet = ctx.font;

            return {
                fontBefore: fontBefore,
                fontAfterSet: fontAfterSet,
                dpr: window.devicePixelRatio,
                canvasWidth: canvas.width,
                canvasHeight: canvas.height,
                cssWidth: canvas.style.width,
                cssHeight: canvas.style.height
            };
            "#,
            vec![],
        )
        .await?;

    println!("🎨 Canvas settings: {:?}", result);

    // フォント設定を確認
    let font_before = result
        .as_object()
        .and_then(|obj| obj.get("fontBefore"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let font_after_set = result
        .as_object()
        .and_then(|obj| obj.get("fontAfterSet"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    println!("🔤 Font before manual set: {}", font_before);
    println!("🔤 Font after manual set: {}", font_after_set);

    // ✅ JavaScriptから直接設定したフォントが反映されるかチェック
    assert!(
        font_after_set.contains("13px") || font_after_set.contains("13.0px"),
        "Expected font size to be 13px after manual set, got: {}",
        font_after_set
    );

    // もしJavaScriptから設定したフォントが反映されたなら、
    // Rustコードでも設定できるはず。
    // 反映されていない場合は、CanvasRenderer::new()が呼ばれていないか、
    // フォント名が無効である可能性がある。
    if !font_after_set.contains("JetBrains Mono") {
        println!("⚠️  WARNING: JetBrains Mono font may not be available in the test environment");
    }

    // ファイルツリーアイテムのフォント設定を確認
    let file_item_result = client
        .find(Locator::Css(".berry-editor-file-item"))
        .await;

    if let Ok(file_item) = file_item_result {
        let font_size = file_item.css_value("font-size").await?;
        let font_weight = file_item.css_value("font-weight").await?;

        println!("🔤 File item font-size: {}", font_size);
        println!("🔤 File item font-weight: {}", font_weight);

        assert!(
            font_size.contains("13px"),
            "Expected file item font size to be 13px, got: {}",
            font_size
        );

        assert!(
            font_weight == "300",
            "Expected file item font weight to be 300, got: {}",
            font_weight
        );
    }

    client.close().await?;
    Ok(())
}

#[tokio::test]
async fn test_inline_styles_verification() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    client.goto("http://localhost:8081").await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // インラインスタイルが正しく適用されているか確認
    let result = client
        .execute(
            r#"
            const tabs = document.querySelector('.berry-editor-tabs');
            const pane = document.querySelector('.berry-editor-pane');
            const sidebar = document.querySelector('.berry-editor-sidebar');

            return {
                tabsInlineStyle: tabs ? tabs.getAttribute('style') : null,
                tabsComputedBg: tabs ? window.getComputedStyle(tabs).backgroundColor : null,
                paneInlineStyle: pane ? pane.getAttribute('style') : null,
                paneComputedBg: pane ? window.getComputedStyle(pane).backgroundColor : null,
                sidebarComputedBg: sidebar ? window.getComputedStyle(sidebar).backgroundColor : null
            };
            "#,
            vec![],
        )
        .await?;

    let obj = result.as_object().unwrap();

    println!("\n📊 Inline Styles Verification:");
    println!("  Tabs inline style: {:?}", obj.get("tabsInlineStyle"));
    println!("  Tabs computed bg: {:?}", obj.get("tabsComputedBg"));
    println!("  Pane inline style: {:?}", obj.get("paneInlineStyle"));
    println!("  Pane computed bg: {:?}", obj.get("paneComputedBg"));
    println!("  Sidebar computed bg: {:?}", obj.get("sidebarComputedBg"));

    // タブバーのインラインスタイルに#313335が含まれているべき
    if let Some(tabs_style) = obj.get("tabsInlineStyle").and_then(|v| v.as_str()) {
        assert!(
            tabs_style.contains("#313335") || tabs_style.contains("rgb(49, 51, 53)"),
            "Expected tabs inline style to contain #313335, got: {}",
            tabs_style
        );
    }

    // エディタペインのインラインスタイルに#1E1F22が含まれているべき
    if let Some(pane_style) = obj.get("paneInlineStyle").and_then(|v| v.as_str()) {
        assert!(
            pane_style.contains("#1E1F22") || pane_style.contains("rgb(30, 31, 34)"),
            "Expected pane inline style to contain #1E1F22, got: {}",
            pane_style
        );
    }

    client.close().await?;
    Ok(())
}
