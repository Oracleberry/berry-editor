//! UI Components for BerryEditor - Tauri Version
//! Uses native file system access

use crate::core::virtual_editor::VirtualEditorPanel;
use crate::file_tree_tauri::FileTreePanelTauri;
use crate::tauri_bindings;
use leptos::prelude::*;

/// Status Bar component with branding
#[component]
pub fn StatusBar() -> impl IntoView {
    view! {
        <div class="berry-editor-status-bar" style="
            display: flex;
            justify-content: space-between;
            align-items: center;
            height: 22px;
            background-color: #2D2D30;
            color: #CCCCCC;
            font-size: 12px;
            padding: 0 12px;
            border-top: 1px solid #1E1E1E;
        ">
            <div class="berry-editor-status-left" style="display: flex; gap: 12px; align-items: center;">
                <span style="font-weight: bold; color: #E0E0E0;">"BerryEditor"</span>
                <span style="color: #858585;">"100% Rust"</span>
            </div>
            <div class="berry-editor-status-right" style="color: #858585;">
                <span>"WASM"</span>
            </div>
        </div>
    }
}

#[component]
pub fn EditorAppTauri() -> impl IntoView {
    // File selection state (shared between FileTree and Editor)
    let selected_file = RwSignal::new(Option::<(String, String)>::None); // (path, content)

    // Get current directory dynamically from Tauri
    // ✅ Start with empty path - will be populated by Effect
    // In test environment, get_current_dir() will return "." due to is_tauri_context() check
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
                    leptos::logging::warn!("Failed to get current directory: {}", _e);
                    root_path.set("/Users/kyosukeishizu/oracleberry/berrcode/gui-editor".to_string());
                }
            }
        });
    });

    view! {
        <div class="berry-editor-container">
            <div class="berry-editor-main-area" style="display: flex; flex: 1; overflow: hidden;">
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

            // Status Bar at bottom
            <StatusBar />
        </div>
    }
}
