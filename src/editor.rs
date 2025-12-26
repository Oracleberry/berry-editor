//! Editor Panel Component
//! 100% Rust - No JavaScript!

use leptos::prelude::*;
use crate::buffer::TextBuffer;
use crate::syntax::SyntaxHighlighter;

#[derive(Clone)]
struct EditorTab {
    path: String,
    buffer: TextBuffer,
    highlighter: SyntaxHighlighter,
}

#[component]
pub fn EditorPanel(
    selected_file: RwSignal<Option<(String, String)>>,
) -> impl IntoView {
    let tabs = RwSignal::new(Vec::<EditorTab>::new());
    let active_tab_index = RwSignal::new(0usize);

    web_sys::console::log_1(&"[EditorPanel] Component initialized".into());

    // Use Effect to watch for file selection changes
    Effect::new_isomorphic(move |_| {
        let file_data = selected_file.get();
        web_sys::console::log_1(&format!("[EditorPanel EFFECT] Checking selected_file: {:?}", file_data.as_ref().map(|(p, c)| (p.as_str(), c.len()))).into());

        if let Some((path, content)) = file_data {
            web_sys::console::log_1(&format!("[EditorPanel EFFECT] File selected: {} ({} bytes)", path, content.len()).into());

            // Check if tab already exists
            let current_tabs = tabs.get_untracked();
            let existing_tab_index = current_tabs.iter().position(|tab| tab.path == path);

            if let Some(idx) = existing_tab_index {
                // Switch to existing tab
                web_sys::console::log_1(&format!("[EditorPanel EFFECT] Switching to existing tab {}", idx).into());
                active_tab_index.set(idx);
            } else {
                // Create new tab
                web_sys::console::log_1(&"[EditorPanel EFFECT] Creating new tab".into());
                let buffer = TextBuffer::from_str(&content);
                let highlighter = SyntaxHighlighter::new();
                let tab = EditorTab {
                    path: path.clone(),
                    buffer,
                    highlighter,
                };

                tabs.update(|t| t.push(tab));
                let new_index = tabs.get_untracked().len() - 1;
                web_sys::console::log_1(&format!("[EditorPanel EFFECT] Tab created at index {}", new_index).into());
                active_tab_index.set(new_index);
            }
        }
    });

    // Get active tab
    let active_tab = move || {
        let idx = active_tab_index.get();
        let current_tabs = tabs.get();
        current_tabs.get(idx).cloned()
    };

    view! {
        <div class="berry-editor-main">
            // Tab Bar
            <div class="berry-editor-tab-bar">
                {move || {
                    web_sys::console::log_1(&"[EditorPanel] Rendering tab bar".into());
                    let current_tabs = tabs.get();
                    let current_index = active_tab_index.get();
                    web_sys::console::log_1(&format!("[EditorPanel] Tabs: {}, Active: {}", current_tabs.len(), current_index).into());

                    current_tabs.iter().enumerate().map(|(idx, tab)| {
                        let is_active = idx == current_index;
                        let filename = tab.path.split('/').last().unwrap_or(&tab.path).to_string();
                        let tab_class = if is_active {
                            "berry-editor-tab active"
                        } else {
                            "berry-editor-tab"
                        };

                        view! {
                            <div class=tab_class on:click=move |_| {
                                active_tab_index.set(idx);
                            }>
                                <span>{filename}</span>
                            </div>
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>

            // Editor Pane
            <div class="berry-editor-pane">
                {move || {
                    web_sys::console::log_1(&"[EditorPanel] Rendering editor pane".into());
                    let tab_opt = active_tab();

                    if let Some(tab) = tab_opt {
                        web_sys::console::log_1(&format!("[EditorPanel] Displaying file: {}", tab.path).into());
                        let content = tab.buffer.to_string();
                        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                        let line_count = lines.len();
                        web_sys::console::log_1(&format!("[EditorPanel] Lines: {}", line_count).into());

                        view! {
                            <div class="berry-editor-content">
                                <div class="berry-editor-line-numbers">
                                    {(1..=line_count).map(|line_num| {
                                        view! {
                                            <div class="berry-editor-line-number">{line_num}</div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                                <div class="berry-editor-lines">
                                    {lines.into_iter().map(|line| {
                                        view! {
                                            <div class="berry-editor-line">
                                                <span>{line}</span>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        web_sys::console::log_1(&"[EditorPanel] No active tab".into());
                        view! {
                            <div style="display:flex;align-items:center;justify-content:center;height:100%;color:#858585;">
                                "Click a file to start editing..."
                            </div>
                        }.into_any()
                    }
                }}
            </div>

            // Status Bar
            <div class="berry-editor-status-bar">
                {move || {
                    if let Some(tab) = active_tab() {
                        let lang = tab.highlighter.get_language().unwrap_or("text");
                        format!("{} | UTF-8", lang)
                    } else {
                        "Ready".to_string()
                    }
                }}
            </div>
        </div>
    }
}
