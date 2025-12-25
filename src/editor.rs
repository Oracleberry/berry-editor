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
pub fn EditorPanel() -> impl IntoView {
    let (tabs, set_tabs) = signal(Vec::<EditorTab>::new());
    let (active_tab_index, set_active_tab_index) = signal(0usize);

    // Get active tab
    let active_tab = move || {
        let idx = active_tab_index.get();
        tabs.get().get(idx).cloned()
    };

    view! {
        <div class="berry-editor-main">
            // Tab Bar
            <div class="berry-editor-tab-bar">
                {move || {
                    let current_tabs = tabs.get();
                    current_tabs.iter().enumerate().map(|(idx, tab)| {
                        let is_active = idx == active_tab_index.get();
                        let filename = tab.path.split('/').last().unwrap_or(&tab.path).to_string();
                        let tab_class = if is_active {
                            "berry-editor-tab active"
                        } else {
                            "berry-editor-tab"
                        };

                        view! {
                            <div class=tab_class>
                                <span>{filename}</span>
                            </div>
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>

            // Editor Pane
            <div class="berry-editor-pane">
                {move || {
                    if let Some(tab) = active_tab() {
                        let content = tab.buffer.to_string();
                        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                        let line_count = lines.len();

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
                                    {lines.iter().map(|line| {
                                        let line_content = line.clone();
                                        view! {
                                            <div class="berry-editor-line">
                                                <span>{line_content}</span>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        }.into_any()
                    } else {
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
