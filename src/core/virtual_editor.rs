//! Virtual Scroll Editor Component
//! High-performance editor that can handle 100k+ line files

use leptos::prelude::*;
use leptos::ev::Event;
use crate::buffer::TextBuffer;
use crate::syntax::SyntaxHighlighter;
use crate::virtual_scroll::VirtualScroll;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

const LINE_HEIGHT: f64 = 20.0; // pixels

fn event_target_value(ev: &web_sys::Event) -> String {
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
        .map(|textarea| textarea.value())
        .unwrap_or_default()
}

#[derive(Clone)]
struct EditorTab {
    path: String,
    buffer: TextBuffer,
    highlighter: SyntaxHighlighter,
    scroll: VirtualScroll,
    is_modified: bool,
    original_content: String,
    is_editing: bool,  // New: track if in edit mode
}

#[component]
pub fn VirtualEditorPanel(
    selected_file: RwSignal<Option<(String, String)>>,
) -> impl IntoView {
    let tabs = RwSignal::new(Vec::<EditorTab>::new());
    let active_tab_index = RwSignal::new(0usize);
    let scroll_top = RwSignal::new(0.0);

    web_sys::console::log_1(&"[VirtualEditorPanel] Component initialized".into());

    // Effect to watch for file selection changes
    Effect::new_isomorphic(move |_| {
        let file_data = selected_file.get();
        web_sys::console::log_1(&format!("[VirtualEditorPanel EFFECT] Checking selected_file: {:?}", file_data.as_ref().map(|(p, c)| (p.as_str(), c.len()))).into());

        if let Some((path, content)) = file_data {
            web_sys::console::log_1(&format!("[VirtualEditorPanel EFFECT] File selected: {} ({} bytes)", path, content.len()).into());

            // Check if tab already exists
            let current_tabs = tabs.get_untracked();
            let existing_tab_index = current_tabs.iter().position(|tab| tab.path == path);

            if let Some(idx) = existing_tab_index {
                // Switch to existing tab
                web_sys::console::log_1(&format!("[VirtualEditorPanel EFFECT] Switching to existing tab {}", idx).into());
                active_tab_index.set(idx);
            } else {
                // Create new tab
                web_sys::console::log_1(&"[VirtualEditorPanel EFFECT] Creating new tab".into());
                let buffer = TextBuffer::from_str(&content);
                let mut highlighter = SyntaxHighlighter::new();

                // Auto-detect language from file extension
                if path.ends_with(".rs") {
                    let _ = highlighter.set_language("rust");
                } else if path.ends_with(".js") || path.ends_with(".ts") {
                    let _ = highlighter.set_language("javascript");
                } else if path.ends_with(".py") {
                    let _ = highlighter.set_language("python");
                }

                // Create virtual scroll for this file
                let line_count = buffer.len_lines();
                let viewport_height = 800.0; // Will be updated by resize observer
                let scroll = VirtualScroll::new(line_count, viewport_height, LINE_HEIGHT);

                let tab = EditorTab {
                    path: path.clone(),
                    buffer,
                    highlighter,
                    scroll,
                    is_modified: false,
                    original_content: content.clone(),
                    is_editing: false,  // Start in view mode
                };

                tabs.update(|t| t.push(tab));
                let new_index = tabs.get_untracked().len() - 1;
                web_sys::console::log_1(&format!("[VirtualEditorPanel EFFECT] Tab created at index {}", new_index).into());
                active_tab_index.set(new_index);
                scroll_top.set(0.0);
            }
        }
    });

    // Get active tab
    let active_tab = move || {
        let idx = active_tab_index.get();
        let current_tabs = tabs.get();
        current_tabs.get(idx).cloned()
    };

    // Handle scroll event
    let on_scroll = move |ev: Event| {
        if let Some(target) = ev.target() {
            if let Ok(element) = target.dyn_into::<HtmlElement>() {
                let new_scroll_top = element.scroll_top() as f64;
                scroll_top.set(new_scroll_top);

                // Update virtual scroll in active tab
                tabs.update(|t| {
                    if let Some(tab) = t.get_mut(active_tab_index.get_untracked()) {
                        tab.scroll.set_scroll_top(new_scroll_top);
                    }
                });
            }
        }
    };

    // Close tab function
    let close_tab = move |idx: usize| {
        web_sys::console::log_1(&format!("[VirtualEditorPanel] Closing tab {}", idx).into());

        tabs.update(|t| {
            if idx < t.len() {
                t.remove(idx);

                // Adjust active tab index if needed
                let current_active = active_tab_index.get_untracked();
                if t.is_empty() {
                    active_tab_index.set(0);
                } else if current_active >= t.len() {
                    active_tab_index.set(t.len() - 1);
                } else if idx <= current_active && current_active > 0 {
                    active_tab_index.set(current_active - 1);
                }
            }
        });
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
                        let is_modified = tab.is_modified;
                        let tab_class = if is_active {
                            "berry-editor-tab active"
                        } else {
                            "berry-editor-tab"
                        };

                        view! {
                            <div class=tab_class>
                                <span
                                    class="berry-editor-tab-label"
                                    on:click=move |_| {
                                        active_tab_index.set(idx);
                                        scroll_top.set(0.0);
                                    }
                                >
                                    {if is_modified { "● " } else { "" }}
                                    {filename}
                                </span>
                                <span
                                    class="berry-editor-tab-close"
                                    on:click=move |e| {
                                        e.stop_propagation();
                                        close_tab(idx);
                                    }
                                >
                                    "×"
                                </span>
                            </div>
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>

            // Editor Pane with Virtual Scrolling
            <div
                class="berry-editor-pane"
                on:scroll=on_scroll
                style="overflow-y: auto; height: 100%;"
            >
                {move || {
                    let tab_opt = active_tab();

                    if let Some(tab) = tab_opt {
                        let content = tab.buffer.to_string();
                        let is_editing = tab.is_editing;

                        if is_editing {
                            // EDIT MODE: Show textarea with line numbers
                            let textarea_ref = NodeRef::<leptos::html::Textarea>::new();
                            let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                            let line_count = lines.len().max(1);

                            // Auto-focus when entering edit mode
                            Effect::new(move |_| {
                                if let Some(textarea) = textarea_ref.get() {
                                    let _ = textarea.focus();
                                }
                            });

                            view! {
                                <div style="
                                    display: flex;
                                    width: 100%;
                                    height: 100%;
                                    background: #2b2b2b;
                                ">
                                    // Line numbers
                                    <div style="
                                        min-width: 50px;
                                        text-align: right;
                                        padding: 10px 12px 10px 0;
                                        background: #313335;
                                        color: #606366;
                                        font-size: 13px;
                                        line-height: 20px;
                                        user-select: none;
                                        border-right: 1px solid #323232;
                                        font-family: Menlo, Monaco, 'Courier New', monospace;
                                    ">
                                        {(1..=line_count).map(|n| {
                                            view! {
                                                <div>{n}</div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>

                                    <textarea
                                        node_ref=textarea_ref
                                        style="
                                            flex: 1;
                                            background: #2b2b2b;
                                            color: #a9b7c6;
                                            font-family: Menlo, Monaco, 'Courier New', monospace;
                                            font-size: 13px;
                                            line-height: 20px;
                                            padding: 10px;
                                            border: none;
                                            outline: none;
                                            resize: none;
                                            tab-size: 4;
                                        "
                                    on:input=move |ev| {
                                        let new_content = event_target_value(&ev);
                                        tabs.update(|t| {
                                            if let Some(tab) = t.get_mut(active_tab_index.get_untracked()) {
                                                tab.buffer = TextBuffer::from_str(&new_content);
                                                tab.is_modified = true;
                                            }
                                        });
                                    }
                                    on:keydown=move |ev| {
                                        // Save on Ctrl+S / Cmd+S
                                        if (ev.ctrl_key() || ev.meta_key()) && ev.key() == "s" {
                                            ev.prevent_default();
                                            let current_content = tabs.with_untracked(|t| {
                                                t.get(active_tab_index.get_untracked())
                                                    .map(|tab| (tab.path.clone(), tab.buffer.to_string()))
                                            });

                                            if let Some((path, content)) = current_content {
                                                web_sys::console::log_1(&format!("[Editor] Saving file: {}", path).into());

                                                use crate::tauri_bindings;
                                                use leptos::task::spawn_local;

                                                spawn_local(async move {
                                                    match tauri_bindings::write_file(&path, &content).await {
                                                        Ok(_) => {
                                                            web_sys::console::log_1(&"[Editor] File saved successfully".into());
                                                            // Mark as unmodified and exit edit mode
                                                            tabs.update(|t| {
                                                                if let Some(tab) = t.get_mut(active_tab_index.get_untracked()) {
                                                                    tab.is_modified = false;
                                                                    tab.is_editing = false;
                                                                }
                                                            });
                                                        }
                                                        Err(e) => {
                                                            web_sys::console::log_1(&format!("[Editor] Error saving file: {}", e).into());
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                        // Escape key to exit edit mode
                                        else if ev.key() == "Escape" {
                                            tabs.update(|t| {
                                                if let Some(tab) = t.get_mut(active_tab_index.get_untracked()) {
                                                    tab.is_editing = false;
                                                }
                                            });
                                        }
                                    }
                                >{content}</textarea>
                                </div>
                            }.into_any()
                        } else {
                            // VIEW MODE: Show syntax highlighted code
                            let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

                            view! {
                                <div
                                    style="
                                        width: 100%;
                                        height: 100%;
                                        overflow: auto;
                                        padding: 10px;
                                        font-family: Menlo, Monaco, 'Courier New', monospace;
                                        font-size: 13px;
                                        line-height: 20px;
                                        background: #2b2b2b;
                                        cursor: text;
                                        pointer-events: auto !important;
                                        position: relative;
                                        z-index: 1;
                                    "
                                    on:click=move |ev| {
                                        // Enter edit mode on SINGLE click for now (debugging)
                                        web_sys::console::log_1(&"[VirtualEditor] Click detected!".into());
                                        tabs.update(|t| {
                                            if let Some(tab) = t.get_mut(active_tab_index.get_untracked()) {
                                                tab.is_editing = true;
                                                web_sys::console::log_1(&"[VirtualEditor] Entering edit mode".into());
                                            }
                                        });
                                    }
                                >
                                    {lines.iter().enumerate().map(|(idx, line)| {
                                        let highlighted_html = syntax_highlight_line(line);

                                        view! {
                                            <div style="display: flex; pointer-events: auto;">
                                                <span style="
                                                    color: #606366;
                                                    margin-right: 10px;
                                                    user-select: none;
                                                    -webkit-user-select: none;
                                                    -moz-user-select: none;
                                                    -ms-user-select: none;
                                                    min-width: 40px;
                                                    text-align: right;
                                                    font-size: 12px;
                                                    pointer-events: none;
                                                ">
                                                    {idx + 1}
                                                </span>
                                                <code style="
                                                    margin: 0;
                                                    padding: 0;
                                                    font-family: inherit;
                                                    font-size: inherit;
                                                    line-height: inherit;
                                                    flex: 1;
                                                    white-space: pre;
                                                    display: block;
                                                    user-select: text;
                                                    -webkit-user-select: text;
                                                    pointer-events: auto;
                                                    cursor: text;
                                                " inner_html=highlighted_html></code>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        }
                    } else {
                        view! {
                            <div style="display:flex;align-items:center;justify-content:center;height:100%;color:#606366;background:#2b2b2b;">
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
                        format!("{} | UTF-8 | {} lines", lang, tab.buffer.len_lines())
                    } else {
                        "Ready".to_string()
                    }
                }}
            </div>
        </div>
    }
}

/// Basic syntax highlighting for Rust code
fn syntax_highlight_line(line: &str) -> String {
    let keywords = [
        "fn", "let", "mut", "const", "static", "impl", "trait", "struct", "enum",
        "mod", "pub", "use", "crate", "self", "super", "async", "await", "move",
        "if", "else", "match", "loop", "while", "for", "in", "return", "break",
        "continue", "as", "ref", "where", "unsafe", "extern", "type", "dyn",
    ];

    let types = ["String", "str", "usize", "isize", "f64", "f32", "i32", "u32",
                 "i64", "u64", "bool", "Vec", "Option", "Result", "Some", "None",
                 "Ok", "Err", "Box", "Rc", "Arc", "RefCell", "RwSignal"];

    let mut result = String::new();
    let mut chars = line.chars().peekable();
    let mut current_word = String::new();
    let mut in_string = false;
    let mut in_comment = false;
    let mut string_char = ' ';

    while let Some(ch) = chars.next() {
        // Handle comments
        if !in_string && ch == '/' && chars.peek() == Some(&'/') {
            in_comment = true;
            flush_word(&mut result, &mut current_word, &keywords, &types);
            result.push_str("<span style=\"color:#6A9955\">");
            result.push_str(&escape_html_char(ch));
            continue;
        }

        if in_comment {
            result.push_str(&escape_html_char(ch));
            continue;
        }

        // Handle strings
        if (ch == '"' || ch == '\'') && !in_string {
            in_string = true;
            string_char = ch;
            flush_word(&mut result, &mut current_word, &keywords, &types);
            result.push_str("<span style=\"color:#CE9178\">");
            result.push(ch);
            continue;
        }

        if in_string {
            if ch == string_char {
                result.push(ch);
                result.push_str("</span>");
                in_string = false;
            } else {
                result.push_str(&escape_html_char(ch));
            }
            continue;
        }

        // Handle word boundaries
        if ch.is_alphanumeric() || ch == '_' {
            current_word.push(ch);
        } else {
            flush_word(&mut result, &mut current_word, &keywords, &types);
            // Preserve spaces explicitly as &nbsp;
            if ch == ' ' {
                result.push_str("&nbsp;");
            } else {
                result.push_str(&escape_html_char(ch));
            }
        }
    }

    flush_word(&mut result, &mut current_word, &keywords, &types);

    if in_comment {
        result.push_str("</span>");
    }

    result
}

fn escape_html_char(ch: char) -> String {
    match ch {
        '<' => "&lt;".to_string(),
        '>' => "&gt;".to_string(),
        '&' => "&amp;".to_string(),
        '"' => "&quot;".to_string(),
        '\'' => "&#39;".to_string(),
        _ => ch.to_string(),
    }
}

fn flush_word(result: &mut String, current_word: &mut String, keywords: &[&str], types: &[&str]) {
    if !current_word.is_empty() {
        if keywords.contains(&current_word.as_str()) {
            result.push_str(&format!("<span style=\"color:#569CD6\">{}</span>", html_escape(current_word)));
        } else if types.contains(&current_word.as_str()) {
            result.push_str(&format!("<span style=\"color:#4EC9B0\">{}</span>", html_escape(current_word)));
        } else if current_word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            result.push_str(&format!("<span style=\"color:#4EC9B0\">{}</span>", html_escape(current_word)));
        } else {
            result.push_str(&html_escape(current_word));
        }
        current_word.clear();
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
        .replace(' ', "&nbsp;")
}
