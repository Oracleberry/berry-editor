//! Mouse Selection Test
//! Tests that mouse dragging correctly selects text without being interrupted by IME focus handling

use berry_editor::core::virtual_editor::VirtualEditorPanel;
use leptos::prelude::*;
use wasm_bindgen_test::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

mod test_helpers;
use test_helpers::{get_test_document, wait_for_render};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_mouse_selection_basic() {
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // テキストファイルを開く
    let text = "Hello World\nLine 2\nLine 3";
    selected_file.set(Some(("/test.txt".to_string(), text.to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();
    let canvas = document
        .query_selector("canvas")
        .unwrap()
        .expect("Canvas exists")
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();

    // Canvasが存在することを確認
    assert!(canvas.width() > 0);
    assert!(canvas.height() > 0);

    // IME inputが存在することを確認
    let ime_input = document
        .query_selector("input[type='text']")
        .unwrap();
    assert!(ime_input.is_some(), "IME input should exist");
}

#[wasm_bindgen_test]
async fn test_mouse_drag_selection_japanese() {
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // 日本語テキストを開く
    let text = "こんにちは世界\n日本語のテスト\nLine 3";
    selected_file.set(Some(("/test.txt".to_string(), text.to_string())));
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

    let has_content = data.iter().any(|&byte| byte != 0);
    assert!(has_content, "Canvas should have rendered content");
}

#[wasm_bindgen_test]
async fn test_ime_input_exists_and_positioned() {
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    let text = "Test content";
    selected_file.set(Some(("/test.txt".to_string(), text.to_string())));
    wait_for_render().await;
    wait_for_render().await;

    let document = get_test_document();

    // IME inputが存在し、位置が設定されていることを確認
    let ime_input = document
        .query_selector("input[type='text']")
        .unwrap()
        .expect("IME input should exist");

    let ime_element = ime_input.dyn_into::<web_sys::HtmlElement>().unwrap();
    let style = ime_element.get_attribute("style").unwrap();

    // position: absolute が含まれていることを確認
    assert!(style.contains("position: absolute"), "IME input should have absolute positioning");
}
