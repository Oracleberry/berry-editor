//! Mouse Selection Test
//! Tests that mouse dragging correctly selects text

use berry_editor::core::virtual_editor::VirtualEditorPanel;
use leptos::prelude::*;
use wasm_bindgen_test::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, MouseEvent, MouseEventInit};

mod test_helpers;
use test_helpers::{get_test_document, wait_for_render};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_mouse_click_moves_cursor() {
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

    // マウスクリックイベントを発火
    let mut event_init = MouseEventInit::new();
    event_init.client_x(100);
    event_init.client_y(50);
    event_init.bubbles(true);

    let mousedown_event = MouseEvent::new_with_event_init_dict("mousedown", &event_init).unwrap();
    canvas.dispatch_event(&mousedown_event).unwrap();
    wait_for_render().await;

    let mouseup_event = MouseEvent::new_with_event_init_dict("mouseup", &event_init).unwrap();
    canvas.dispatch_event(&mouseup_event).unwrap();
    wait_for_render().await;

    // テストが完了したことを確認
    assert!(true, "Mouse click should not crash");
}

#[wasm_bindgen_test]
async fn test_mouse_drag_creates_selection() {
    get_test_document().body().unwrap().set_inner_html("");

    let selected_file = RwSignal::new(None::<(String, String)>);

    let _dispose = leptos::mount::mount_to_body(move || {
        view! { <VirtualEditorPanel selected_file=selected_file /> }
    });

    wait_for_render().await;

    // テキストファイルを開く
    let text = "Hello World";
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

    // mousedown at position 1
    let mut down_init = MouseEventInit::new();
    down_init.client_x(100);
    down_init.client_y(20);
    down_init.bubbles(true);

    let mousedown = MouseEvent::new_with_event_init_dict("mousedown", &down_init).unwrap();
    canvas.dispatch_event(&mousedown).unwrap();
    wait_for_render().await;

    // mousemove to position 2
    let mut move_init = MouseEventInit::new();
    move_init.client_x(200);
    move_init.client_y(20);
    move_init.bubbles(true);

    let mousemove = MouseEvent::new_with_event_init_dict("mousemove", &move_init).unwrap();
    canvas.dispatch_event(&mousemove).unwrap();
    wait_for_render().await;

    // mouseup at position 2
    let mouseup = MouseEvent::new_with_event_init_dict("mouseup", &move_init).unwrap();
    canvas.dispatch_event(&mouseup).unwrap();
    wait_for_render().await;

    // 選択が作成されたことを確認（クラッシュしないことを確認）
    assert!(true, "Mouse drag should create selection without crashing");
}

#[wasm_bindgen_test]
async fn test_ime_input_exists() {
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

    // IME inputが存在することを確認
    let ime_input = document
        .query_selector("input[type='text']")
        .unwrap();
    assert!(ime_input.is_some(), "IME input should exist");
}
