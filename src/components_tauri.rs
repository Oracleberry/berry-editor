//! UI Components for BerryEditor - Tauri Version
//! Uses native file system access

use crate::core::virtual_editor::VirtualEditorPanel;
use crate::file_tree_tauri::FileTreePanelTauri;
use crate::search_panel::SearchPanel;
use crate::database_panel::DatabasePanel;
use crate::workflow_panel::WorkflowPanel;
use crate::terminal_panel::TerminalPanel;
use crate::berrycode_panel::BerryCodePanel;
use crate::settings::EditorSettings;
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
    Settings,
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

    // Sidebar resize state
    let sidebar_width = RwSignal::new(300.0); // Default width in pixels
    let is_resizing = RwSignal::new(false);

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

    // Resize handlers
    let is_hovering_resize = RwSignal::new(false);

    let on_resize_mousedown = move |_ev: leptos::ev::MouseEvent| {
        is_resizing.set(true);
    };

    let on_mousemove = move |ev: leptos::ev::MouseEvent| {
        if is_resizing.get() {
            // Calculate new width (subtract activity bar width: 54px)
            let new_width = (ev.client_x() as f64 - 54.0)
                .max(150.0)  // Minimum width: 150px
                .min(800.0); // Maximum width: 800px
            sidebar_width.set(new_width);
        }
    };

    let on_mouseup = move |_ev: leptos::ev::MouseEvent| {
        is_resizing.set(false);
    };

    let on_resize_mouseenter = move |_ev: leptos::ev::MouseEvent| {
        is_hovering_resize.set(true);
    };

    let on_resize_mouseleave = move |_ev: leptos::ev::MouseEvent| {
        is_hovering_resize.set(false);
    };

    view! {
        <div
            class="berry-editor-container"
            on:mousemove=on_mousemove
            on:mouseup=on_mouseup
            style=move || {
                let base = "display: flex; flex-direction: column; height: 100vh; width: 100vw; overflow: hidden;";
                if is_resizing.get() {
                    format!("{} cursor: col-resize; user-select: none;", base)
                } else {
                    base.to_string()
                }
            }
        >
            <div class="berry-editor-main-area" style="display: flex; flex: 1; overflow: hidden; position: relative; min-height: 0;">
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
                    flex-shrink: 0;
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

                    // Chat (BerryCode AI) icon
                    <div
                        on:click=move |_| active_panel.set(ActivePanel::Chat)
                        style=move || format!(
                            "cursor: pointer; font-size: 25px !important; color: {}; transition: color 0.2s; line-height: 25px;",
                            if active_panel.get() == ActivePanel::Chat { "#FFFFFF" } else { "#858585" }
                        )
                        title="BerryCode AI"
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

                    // Spacer to push settings to bottom
                    <div style="flex: 1;"></div>

                    // Settings icon (bottom)
                    <div
                        on:click=move |_| active_panel.set(ActivePanel::Settings)
                        style=move || format!(
                            "cursor: pointer; font-size: 25px !important; color: {}; transition: color 0.2s; line-height: 25px;",
                            if active_panel.get() == ActivePanel::Settings { "#FFFFFF" } else { "#858585" }
                        )
                        title="Settings"
                    >
                        <i class="codicon codicon-settings-gear" style="font-size: 25px !important;"></i>
                    </div>
                </div>

                // Sidebar container with resize capability
                <div style=move || format!("width: {}px; flex-shrink: 0; overflow: hidden;", sidebar_width.get())>
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
                                <BerryCodePanel project_path=Signal::derive(move || root_path.get()) />
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
                        },
                        ActivePanel::Settings => {
                            let settings_store = StoredValue::new(EditorSettings::load());

                            let (font_size, set_font_size) = signal(settings_store.get_value().font_size);
                            let (line_height, set_line_height) = signal(settings_store.get_value().line_height);
                            let (tab_size, set_tab_size) = signal(settings_store.get_value().tab_size);
                            let (word_wrap, set_word_wrap) = signal(settings_store.get_value().word_wrap);
                            let (ai_enabled, set_ai_enabled) = signal(settings_store.get_value().ai_enabled);

                            let save_settings = move || {
                                let s = settings_store.get_value();
                                let _ = s.save();
                                leptos::logging::log!("Settings saved");
                            };

                            view! {
                                <div class="berry-editor-sidebar" style="background: #252526; height: 100%; display: flex; flex-direction: column;">
                                    <div class="berry-editor-sidebar-header" style="
                                        padding: 8px 12px;
                                        background: #2D2D30;
                                        border-bottom: 1px solid #1e1e1e;
                                        font-size: 12px;
                                        font-weight: 600;
                                        color: #cccccc;
                                    ">
                                        "SETTINGS"
                                    </div>
                                    <div style="padding: 16px; color: #BCBEC4; font-size: 12px; overflow-y: auto; flex: 1;">
                                        // Editor Settings
                                        <div style="margin-bottom: 24px;">
                                            <div style="font-weight: 600; margin-bottom: 12px; color: #FFFFFF;">
                                                "Editor"
                                            </div>
                                            <div style="display: flex; flex-direction: column; gap: 12px;">
                                                // Font Size
                                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                                    <span style="color: #BCBEC4;">"Font Size"</span>
                                                    <input
                                                        type="number"
                                                        min="8"
                                                        max="32"
                                                        prop:value=move || font_size.get()
                                                        on:input=move |ev| {
                                                            if let Ok(val) = event_target_value(&ev).parse() {
                                                                set_font_size.set(val);
                                                                settings_store.update_value(|s| s.font_size = val);
                                                                save_settings();
                                                            }
                                                        }
                                                        style="width: 60px; background: #3C3F41; border: 1px solid #555; color: #BCBEC4; padding: 4px; border-radius: 3px; font-size: 11px;"
                                                    />
                                                </div>

                                                // Font Family
                                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                                    <span style="color: #BCBEC4;">"Font Family"</span>
                                                    <select
                                                        on:change=move |ev| {
                                                            let val = event_target_value(&ev);
                                                            settings_store.update_value(|s| s.font_family = val);
                                                            save_settings();
                                                        }
                                                        style="background: #3C3F41; border: 1px solid #555; color: #BCBEC4; padding: 4px; border-radius: 3px; font-size: 11px;"
                                                    >
                                                        {
                                                            let current = settings_store.get_value().font_family;
                                                            EditorSettings::available_fonts().into_iter().map(|font| {
                                                                let is_selected = font == current.as_str();
                                                                view! {
                                                                    <option value=font selected=is_selected>{font}</option>
                                                                }
                                                            }).collect_view()
                                                        }
                                                    </select>
                                                </div>

                                                // Line Height
                                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                                    <span style="color: #BCBEC4;">"Line Height"</span>
                                                    <input
                                                        type="number"
                                                        min="14"
                                                        max="40"
                                                        prop:value=move || line_height.get()
                                                        on:input=move |ev| {
                                                            if let Ok(val) = event_target_value(&ev).parse() {
                                                                set_line_height.set(val);
                                                                settings_store.update_value(|s| s.line_height = val);
                                                                save_settings();
                                                            }
                                                        }
                                                        style="width: 60px; background: #3C3F41; border: 1px solid #555; color: #BCBEC4; padding: 4px; border-radius: 3px; font-size: 11px;"
                                                    />
                                                </div>

                                                // Tab Size
                                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                                    <span style="color: #BCBEC4;">"Tab Size"</span>
                                                    <input
                                                        type="number"
                                                        min="2"
                                                        max="8"
                                                        prop:value=move || tab_size.get()
                                                        on:input=move |ev| {
                                                            if let Ok(val) = event_target_value(&ev).parse() {
                                                                set_tab_size.set(val);
                                                                settings_store.update_value(|s| s.tab_size = val);
                                                                save_settings();
                                                            }
                                                        }
                                                        style="width: 60px; background: #3C3F41; border: 1px solid #555; color: #BCBEC4; padding: 4px; border-radius: 3px; font-size: 11px;"
                                                    />
                                                </div>

                                                // Word Wrap
                                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                                    <span style="color: #BCBEC4;">"Word Wrap"</span>
                                                    <input
                                                        type="checkbox"
                                                        prop:checked=move || word_wrap.get()
                                                        on:change=move |ev| {
                                                            let checked = event_target_checked(&ev);
                                                            set_word_wrap.set(checked);
                                                            settings_store.update_value(|s| s.word_wrap = checked);
                                                            save_settings();
                                                        }
                                                        style="cursor: pointer;"
                                                    />
                                                </div>
                                            </div>
                                        </div>

                                        // Theme Settings
                                        <div style="margin-bottom: 24px;">
                                            <div style="font-weight: 600; margin-bottom: 12px; color: #FFFFFF;">
                                                "Theme"
                                            </div>
                                            <div style="display: flex; flex-direction: column; gap: 12px;">
                                                // Color Theme
                                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                                    <span style="color: #BCBEC4;">"Color Theme"</span>
                                                    <select
                                                        on:change=move |ev| {
                                                            let val = event_target_value(&ev);
                                                            settings_store.update_value(|s| s.color_theme = val);
                                                            save_settings();
                                                        }
                                                        style="background: #3C3F41; border: 1px solid #555; color: #BCBEC4; padding: 4px; border-radius: 3px; font-size: 11px;"
                                                    >
                                                        {
                                                            let current = settings_store.get_value().color_theme;
                                                            EditorSettings::available_themes().into_iter().map(|theme| {
                                                                let is_selected = theme == current.as_str();
                                                                view! {
                                                                    <option value=theme selected=is_selected>{theme}</option>
                                                                }
                                                            }).collect_view()
                                                        }
                                                    </select>
                                                </div>
                                            </div>
                                        </div>

                                        // BerryCode AI Settings
                                        <div style="margin-bottom: 24px;">
                                            <div style="font-weight: 600; margin-bottom: 12px; color: #FFFFFF;">
                                                "BerryCode AI"
                                            </div>
                                            <div style="display: flex; flex-direction: column; gap: 12px;">
                                                // Model
                                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                                    <span style="color: #BCBEC4;">"Model"</span>
                                                    <select
                                                        on:change=move |ev| {
                                                            let val = event_target_value(&ev);
                                                            settings_store.update_value(|s| s.ai_model = val);
                                                            save_settings();
                                                        }
                                                        style="background: #3C3F41; border: 1px solid #555; color: #BCBEC4; padding: 4px; border-radius: 3px; font-size: 11px;"
                                                    >
                                                        {
                                                            let current = settings_store.get_value().ai_model;
                                                            EditorSettings::available_models().into_iter().map(|model| {
                                                                let is_selected = model == current.as_str();
                                                                view! {
                                                                    <option value=model selected=is_selected>{model}</option>
                                                                }
                                                            }).collect_view()
                                                        }
                                                    </select>
                                                </div>

                                                // Mode
                                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                                    <span style="color: #BCBEC4;">"Mode"</span>
                                                    <select
                                                        on:change=move |ev| {
                                                            let val = event_target_value(&ev);
                                                            settings_store.update_value(|s| s.ai_mode = val);
                                                            save_settings();
                                                        }
                                                        style="background: #3C3F41; border: 1px solid #555; color: #BCBEC4; padding: 4px; border-radius: 3px; font-size: 11px;"
                                                    >
                                                        {
                                                            let current = settings_store.get_value().ai_mode;
                                                            EditorSettings::available_modes().into_iter().map(|mode| {
                                                                let is_selected = mode == current.as_str();
                                                                view! {
                                                                    <option value=mode selected=is_selected>{mode}</option>
                                                                }
                                                            }).collect_view()
                                                        }
                                                    </select>
                                                </div>

                                                // AI Enabled
                                                <div style="display: flex; justify-content: space-between; align-items: center;">
                                                    <span style="color: #BCBEC4;">"Enable AI"</span>
                                                    <input
                                                        type="checkbox"
                                                        prop:checked=move || ai_enabled.get()
                                                        on:change=move |ev| {
                                                            let checked = event_target_checked(&ev);
                                                            set_ai_enabled.set(checked);
                                                            settings_store.update_value(|s| s.ai_enabled = checked);
                                                            save_settings();
                                                        }
                                                        style="cursor: pointer;"
                                                    />
                                                </div>
                                            </div>
                                        </div>

                                        // About
                                        <div style="margin-bottom: 24px;">
                                            <div style="font-weight: 600; margin-bottom: 12px; color: #FFFFFF;">
                                                "About"
                                            </div>
                                            <div style="display: flex; flex-direction: column; gap: 8px;">
                                                <div style="color: #BCBEC4;">
                                                    "BerryEditor v0.1.0"
                                                </div>
                                                <div style="color: #858585; font-size: 11px;">
                                                    "100% Rust Code Editor"
                                                </div>
                                                <div style="color: #858585; font-size: 11px;">
                                                    "Built with Leptos + Tauri + WASM"
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        }
                    }
                    }}
                </div>

                // Resize handle (IntelliJ/VS Code style)
                <div
                    on:mousedown=on_resize_mousedown
                    on:mouseenter=on_resize_mouseenter
                    on:mouseleave=on_resize_mouseleave
                    style=move || format!("
                        width: 5px;
                        cursor: col-resize;
                        background: {};
                        user-select: none;
                        flex-shrink: 0;
                        transition: background 0.15s ease;
                        position: relative;
                        z-index: 10;
                    ",
                        if is_resizing.get() {
                            "#007ACC"
                        } else if is_hovering_resize.get() {
                            "#4C4C4C"
                        } else {
                            "#1E1E1E"
                        }
                    )
                ></div>

                // Main Editor Area with Virtual Scrolling (flex to fill remaining space)
                <div style="display: flex; flex-direction: column; flex: 1; min-width: 0; min-height: 0; overflow: hidden;">
                    {move || {
                        let path = root_path.get();
                        if active_panel.get() == ActivePanel::Terminal && !path.is_empty() {
                            view! {
                                <div style="display: flex; flex-direction: column; height: 100%;">
                                    <TerminalPanel project_path=Signal::derive(move || root_path.get()) />
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <VirtualEditorPanel
                                    selected_file=selected_file
                                    is_active=Signal::derive(move || active_panel.get() != ActivePanel::Terminal)
                                />
                            }.into_any()
                        }
                    }}
                </div>
            </div>

            // Status Bar at bottom
            <StatusBar />
        </div>
    }
}
