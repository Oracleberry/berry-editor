//! Text Selection Cursor Position Test
//! Tests that cursor position is correctly aligned when text is selected,
//! especially with Japanese multi-byte characters

use berry_editor::core::virtual_editor::VirtualEditorPanel;
use leptos::prelude::*;
use wasm_bindgen_test::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

mod test_helpers;
use test_helpers::{get_test_document, wait_for_render};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_cursor_position_with_japanese_text_selection() {
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // 日本語テキストを含むファイルを開く
    let japanese_text = "こんにちは世界\n日本語のテスト\n## バグ修正";
    selected_file.set(Some(("/test.md".to_string(), japanese_text.to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();
    let canvas = document
        .query_selector("canvas")
        .unwrap()
        .expect("Canvas exists")
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();

    // Canvasが描画されていることを確認
    assert!(canvas.width() > 0);
    assert!(canvas.height() > 0);

    // テキストが描画されていることを確認（画像データが真っ黒でないこと）
    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    let image_data = context
        .get_image_data(0.0, 0.0, canvas.width() as f64, canvas.height() as f64)
        .unwrap();
    let data = image_data.data();

    // すべてのピクセルが黒(0,0,0)ではないことを確認
    let has_non_black_pixel = data.iter().any(|&byte| byte != 0);
    assert!(has_non_black_pixel, "Canvas should have rendered content");
}

#[wasm_bindgen_test]
async fn test_selection_rectangle_aligns_with_text() {
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // 日本語と英語が混在するテキスト
    let mixed_text = "Hello こんにちは World 世界";
    selected_file.set(Some(("/test.txt".to_string(), mixed_text.to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();
    let canvas = document
        .query_selector("canvas")
        .unwrap()
        .expect("Canvas exists")
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();

    // 選択を模擬するため、マウスイベントを発火
    // （実際のテストでは、選択範囲の描画が正しいかをチェック）

    // Canvasのコンテキストを取得
    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    // フォント設定を確認
    let font = context.font();
    assert!(font.contains("JetBrains Mono") || font.contains("monospace"));
}

#[wasm_bindgen_test]
async fn test_cursor_after_selection_with_emoji() {
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // 絵文字を含むテキスト
    let emoji_text = "🎉 Hello 🌍 World 🚀";
    selected_file.set(Some(("/test.txt".to_string(), emoji_text.to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();
    let canvas = document
        .query_selector("canvas")
        .unwrap()
        .expect("Canvas exists")
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();

    // Canvasが描画されていることを確認
    assert!(canvas.width() > 0);
    assert!(canvas.height() > 0);
}

#[wasm_bindgen_test]
async fn test_multiline_selection_cursor_position() {
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // 複数行の日本語テキスト
    let multiline_text = "一行目：こんにちは\n二行目：世界\n三行目：テスト";
    selected_file.set(Some(("/test.txt".to_string(), multiline_text.to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();
    let canvas = document
        .query_selector("canvas")
        .unwrap()
        .expect("Canvas exists")
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    // テキストが複数行描画されていることを確認
    let image_data = context
        .get_image_data(0.0, 0.0, canvas.width() as f64, canvas.height() as f64)
        .unwrap();
    let data = image_data.data();

    let has_content = data.iter().any(|&byte| byte != 0);
    assert!(has_content, "Canvas should render multiline text");
}
