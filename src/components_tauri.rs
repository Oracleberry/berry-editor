//! UI Components for BerryEditor - Tauri Version
//! Uses native file system access

use crate::core::virtual_editor::VirtualEditorPanel;
use crate::file_tree_tauri::FileTreePanelTauri;
use crate::tauri_bindings;
use leptos::prelude::*;

#[component]
pub fn EditorAppTauri() -> impl IntoView {
    // File selection state (shared between FileTree and Editor)
    let selected_file = RwSignal::new(Option::<(String, String)>::None); // (path, content)

    // Get current directory dynamically from Tauri
    let root_path = RwSignal::new(String::new());

    // Load current directory on mount
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            match tauri_bindings::get_current_dir().await {
                Ok(path) => {
                    root_path.set(path);
                }
                Err(_e) => {
                    // Fallback to default path
                    #[cfg(target_arch = "wasm32")]
                    {
                        use wasm_bindgen::prelude::*;
                        web_sys::console::warn_1(&JsValue::from_str(&format!("Failed to get current directory: {}", _e)));
                    }
                    root_path.set("/Users/kyosukeishizu/oracleberry/berrcode/gui-editor".to_string());
                }
            }
        });
    });

    view! {
        <div class="berry-editor-container">
            // Left Sidebar - File Tree (only render when root_path is loaded)
            {move || {
                let path = root_path.get();
                if !path.is_empty() {
                    view! {
                        <FileTreePanelTauri on_file_select=selected_file root_path=path />
                    }.into_any()
                } else {
                    view! {
                        <div class="berry-editor-sidebar">
                            <div style="padding: 16px; color: #858585;">
                                "Loading..."
                            </div>
                        </div>
                    }.into_any()
                }
            }}

            // Main Editor Area with Virtual Scrolling
            <VirtualEditorPanel selected_file=selected_file />
        </div>
    }
}
