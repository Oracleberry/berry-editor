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


    // Use Effect to watch for file selection changes
    Effect::new_isomorphic(move |_| {
        let file_data = selected_file.get();

        if let Some((path, content)) = file_data {

            // Check if tab already exists
            let current_tabs = tabs.get_untracked();
            let existing_tab_index = current_tabs.iter().position(|tab| tab.path == path);

            if let Some(idx) = existing_tab_index {
                // Switch to existing tab
                active_tab_index.set(idx);
            } else {
                // Create new tab
                let buffer = TextBuffer::from_str(&content);
                let highlighter = SyntaxHighlighter::new();
                let tab = EditorTab {
                    path: path.clone(),
                    buffer,
                    highlighter,
                };

                tabs.update(|t| t.push(tab));
                let new_index = tabs.get_untracked().len() - 1;
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
                    let current_tabs = tabs.get();
                    let current_index = active_tab_index.get();

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
                    let tab_opt = active_tab();

                    if let Some(tab) = tab_opt {
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
