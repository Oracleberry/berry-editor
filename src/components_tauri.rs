//! UI Components for BerryEditor - Tauri Version
//! Uses native file system access

use crate::core::virtual_editor::VirtualEditorPanel;
use crate::file_tree_tauri::FileTreePanelTauri;
use crate::search_panel::SearchPanel;
use crate::database_panel::DatabasePanel;
use crate::workflow_panel::WorkflowPanel;
use crate::terminal_panel::TerminalPanel;
use crate::tauri_bindings;
use leptos::prelude::*;

/// Active panel in the sidebar
#[derive(Clone, Copy, PartialEq)]
enum ActivePanel {
    Explorer,
    Search,
    Chat,
    Database,
    Workflow,
    Terminal,
    VirtualOffice,
}

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

    // Active panel state (Explorer or Search)
    let active_panel = RwSignal::new(ActivePanel::Explorer);

    // Search panel state
    let search_is_open = RwSignal::new(true); // Always open when Search is active

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
                // Activity Bar (leftmost vertical icon bar)
                <div class="activity-bar" style="
                    width: 54px;
                    background: #333333;
                    display: flex;
                    flex-direction: column;
                    align-items: center;
                    padding: 10px 0;
                    gap: 20px;
                    border-right: 1px solid #1e1e1e;
                ">
                    // Files/Explorer icon
                    <div
                        on:click=move |_| active_panel.set(ActivePanel::Explorer)
                        style=move || format!(
                            "cursor: pointer; font-size: 25px !important; color: {}; transition: color 0.2s; line-height: 25px;",
                            if active_panel.get() == ActivePanel::Explorer { "#FFFFFF" } else { "#858585" }
                        )
                        title="Explorer"
                    >
                        <i class="codicon codicon-files" style="font-size: 25px !important;"></i>
                    </div>

                    // Search icon
                    <div
                        on:click=move |_| active_panel.set(ActivePanel::Search)
                        style=move || format!(
                            "cursor: pointer; font-size: 25px !important; color: {}; transition: color 0.2s; line-height: 25px;",
                            if active_panel.get() == ActivePanel::Search { "#FFFFFF" } else { "#858585" }
                        )
                        title="Search"
                    >
                        <i class="codicon codicon-search" style="font-size: 25px !important;"></i>
                    </div>

                    // Chat (BerryChat) icon
                    <div
                        on:click=move |_| active_panel.set(ActivePanel::Chat)
                        style=move || format!(
                            "cursor: pointer; font-size: 25px !important; color: {}; transition: color 0.2s; line-height: 25px;",
                            if active_panel.get() == ActivePanel::Chat { "#FFFFFF" } else { "#858585" }
                        )
                        title="BerryChat"
                    >
                        <i class="codicon codicon-comment-discussion" style="font-size: 25px !important;"></i>
                    </div>

                    // Database Tools icon
                    <div
                        on:click=move |_| active_panel.set(ActivePanel::Database)
                        style=move || format!(
                            "cursor: pointer; font-size: 25px !important; color: {}; transition: color 0.2s; line-height: 25px;",
                            if active_panel.get() == ActivePanel::Database { "#FFFFFF" } else { "#858585" }
                        )
                        title="Database Tools"
                    >
                        <i class="codicon codicon-database" style="font-size: 25px !important;"></i>
                    </div>

                    // Workflow icon
                    <div
                        on:click=move |_| active_panel.set(ActivePanel::Workflow)
                        style=move || format!(
                            "cursor: pointer; font-size: 25px !important; color: {}; transition: color 0.2s; line-height: 25px;",
                            if active_panel.get() == ActivePanel::Workflow { "#FFFFFF" } else { "#858585" }
                        )
                        title="Workflow Automation"
                    >
                        <i class="codicon codicon-symbol-event" style="font-size: 25px !important;"></i>
                    </div>

                    // Terminal icon
                    <div
                        on:click=move |_| active_panel.set(ActivePanel::Terminal)
                        style=move || format!(
                            "cursor: pointer; font-size: 25px !important; color: {}; transition: color 0.2s; line-height: 25px;",
                            if active_panel.get() == ActivePanel::Terminal { "#FFFFFF" } else { "#858585" }
                        )
                        title="Integrated Terminal"
                    >
                        <i class="codicon codicon-terminal" style="font-size: 25px !important;"></i>
                    </div>

                    // Virtual Office icon
                    <div
                        on:click=move |_| active_panel.set(ActivePanel::VirtualOffice)
                        style=move || format!(
                            "cursor: pointer; font-size: 25px !important; color: {}; transition: color 0.2s; line-height: 25px;",
                            if active_panel.get() == ActivePanel::VirtualOffice { "#FFFFFF" } else { "#858585" }
                        )
                        title="Virtual Office"
                    >
                        <i class="codicon codicon-organization" style="font-size: 25px !important;"></i>
                    </div>
                </div>

                // Sidebar - switches between all panels
                {move || {
                    let path = root_path.get();
                    match active_panel.get() {
                        ActivePanel::Explorer => {
                            if !path.is_empty() {
                                view! {
                                    <FileTreePanelTauri on_file_select=selected_file root_path=path.clone() />
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
                        },
                        ActivePanel::Search => {
                            if !path.is_empty() {
                                let path_clone = path.clone();
                                view! {
                                    <SearchPanel
                                        is_open=search_is_open
                                        root_path=path_clone
                                        on_result_click=move |file_path: String, line: usize| {
                                            leptos::logging::log!("Search result clicked: {} at line {}", file_path, line);
                                            // TODO: Open file and jump to line
                                        }
                                    />
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
                        },
                        ActivePanel::Chat => {
                            view! {
                                <div class="berry-editor-sidebar" style="background: #252526;">
                                    <div class="berry-editor-sidebar-header" style="
                                        padding: 8px 12px;
                                        background: #2D2D30;
                                        border-bottom: 1px solid #1e1e1e;
                                        font-size: 12px;
                                        font-weight: 600;
                                        color: #cccccc;
                                    ">
                                        "BERRYCHAT"
                                    </div>
                                    <div style="padding: 20px; color: #858585; font-size: 12px;">
                                        <div style="margin-bottom: 10px;">
                                            <i class="codicon codicon-info" style="margin-right: 8px;"></i>
                                            "Team chat integration coming soon"
                                        </div>
                                        <div style="margin-top: 20px; font-size: 11px; line-height: 1.6;">
                                            "Features:"
                                            <ul style="margin-top: 8px; padding-left: 20px;">
                                                <li>"Channel management"</li>
                                                <li>"Direct messaging"</li>
                                                <li>"Real-time notifications"</li>
                                                <li>"File sharing"</li>
                                                <li>"Code snippets"</li>
                                            </ul>
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        },
                        ActivePanel::Database => {
                            view! {
                                <DatabasePanel is_active=Signal::derive(move || active_panel.get() == ActivePanel::Database) />
                            }.into_any()
                        },
                        ActivePanel::Workflow => {
                            view! {
                                <WorkflowPanel is_active=Signal::derive(move || active_panel.get() == ActivePanel::Workflow) />
                            }.into_any()
                        },
                        ActivePanel::Terminal => {
                            // Terminal is shown in main area, hide sidebar
                            view! {
                                <div class="berry-editor-sidebar" style="display: none;"></div>
                            }.into_any()
                        },
                        ActivePanel::VirtualOffice => {
                            view! {
                                <div class="berry-editor-sidebar" style="background: #252526;">
                                    <div class="berry-editor-sidebar-header" style="
                                        padding: 8px 12px;
                                        background: #2D2D30;
                                        border-bottom: 1px solid #1e1e1e;
                                        font-size: 12px;
                                        font-weight: 600;
                                        color: #cccccc;
                                    ">
                                        "VIRTUAL OFFICE"
                                    </div>
                                    <div style="padding: 20px; color: #858585; font-size: 12px;">
                                        <div style="margin-bottom: 10px;">
                                            <i class="codicon codicon-info" style="margin-right: 8px;"></i>
                                            "Virtual office collaboration coming soon"
                                        </div>
                                        <div style="margin-top: 20px; font-size: 11px; line-height: 1.6;">
                                            "Features:"
                                            <ul style="margin-top: 8px; padding-left: 20px;">
                                                <li>"Team presence awareness"</li>
                                                <li>"Real-time collaboration"</li>
                                                <li>"Screen sharing"</li>
                                                <li>"Code review sessions"</li>
                                                <li>"Pair programming"</li>
                                            </ul>
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        }
                    }
                }}

                // Main Editor Area with Virtual Scrolling
                {move || {
                    let path = root_path.get();
                    if active_panel.get() == ActivePanel::Terminal && !path.is_empty() {
                        view! {
                            <div style="flex: 1; display: flex; flex-direction: column; height: 100%;">
                                <TerminalPanel project_path=Signal::derive(move || root_path.get()) />
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <VirtualEditorPanel selected_file=selected_file />
                        }.into_any()
                    }
                }}
            </div>

            // Status Bar at bottom
            <StatusBar />
        </div>
    }
}
