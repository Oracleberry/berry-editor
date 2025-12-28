//! UI Components for BerryEditor - Tauri Version
//! Uses native file system access

use leptos::prelude::*;
use crate::file_tree_tauri::FileTreePanelTauri;
use crate::core::virtual_editor::VirtualEditorPanel;

#[component]
pub fn EditorAppTauri() -> impl IntoView {

    // File selection state (shared between FileTree and Editor)
    let selected_file = RwSignal::new(Option::<(String, String)>::None); // (path, content)

    // In WASM, we need to use a default path or get it from Tauri
    // For now, use the current project directory
    let root_path = "/Users/kyosukeishizu/oracleberry/berrcode/gui-editor".to_string();

    view! {
        <div class="berry-editor-container">
            // Left Sidebar - File Tree
            <FileTreePanelTauri on_file_select=selected_file root_path=root_path />

            // Main Editor Area with Virtual Scrolling
            <VirtualEditorPanel selected_file=selected_file />
        </div>
    }
}
