//! Virtual Scroll Editor Component
//! High-performance editor that can handle 100k+ line files
//! ✅ IntelliJ Pro: Snapshots, Segment Rendering
//!
//! ## Architecture: "Real Model + Virtual View" (IntelliJ/VS Code式)
//!
//! ### 3-Layer Design:
//!
//! 1. **[本物の入力層] Real Input Layer** (ContentEditable Div)
//!    - すべてのキーボード入力をキャプチャ（日本語IME含む）
//!    - データフロー: contenteditable div → tabs.update() → buffer.insert() → version++
//!
//! 2. **[本物のデータ層] Real Model Layer** (TextBuffer with Ropey)
//!    - 実際のテキストデータを保持（不変なRope構造）
//!    - version番号で更新を追跡
//!    - バッファ更新 → version++ → リアクティブに描画をトリガー
//!
//! 3. **[バーチャル表示層] Virtual View Layer** (VirtualScroll + Rendering)
//!    - 本物のバッファから可視範囲の行だけを取得して描画
//!    - データフロー: tabs.with() → buffer.version読取 → visible lines → HTML
//!    - 100万行でも見えている部分だけレンダリング（O(visible_lines)）
//!
//! 4. **[バーチャルカーソル] Virtual Cursor Layer** (Independent Overlay)
//!    - 絶対位置で描画される独立カーソル
//!    - 本物のバッファから行データを読み取り、正確な座標を計算
//!
//! ### データフロー（明確な一方向）:
//! ```text
//! User Input -> ContentEditable Div (on:beforeinput/on:compositionend)
//!      |
//!      v
//! tabs.update() [Reactive Signal]
//!      |
//!      v
//! TextBuffer.insert() -> version++ [Real Model]
//!      |
//!      v
//! tabs.with() detects change [Reactive]
//!      |
//!      v
//! Re-render visible lines only [Virtual View]
//! ```

use crate::buffer::TextBuffer;
use crate::completion_widget::CompletionWidget;
use crate::lsp::{LspClient, Position as LspPosition};
use crate::lsp_ui::CompletionItem;
use crate::syntax::SyntaxHighlighter;
use crate::tauri_bindings::{self, HighlightResult};
use crate::virtual_scroll::VirtualScroll;
use leptos::ev::Event;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use unicode_width::UnicodeWidthChar;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

// ✅ Coordinate system constants - synchronized with CSS and MEASURED from actual browser rendering
// ✅ IntelliJ 1:2 Model: 全角文字は半角の正確に2倍の幅
const LINE_HEIGHT: f64 = 20.0; // pixels
const CHAR_WIDTH_ASCII: f64 = 7.8125; // JetBrains Mono 13px (half-width) - CSS measured: 7.8125px
const CHAR_WIDTH_WIDE: f64 = 15.625; // JetBrains Mono 13px (full-width) - 2x ASCII = 15.625px
const GUTTER_WIDTH: f64 = 55.0; // Line number gutter width
const TEXT_PADDING: f64 = 15.0; // Left padding for text content

/// ✅ Calculate horizontal position for cursor based on character count
/// This is the single source of truth for x-position calculation
fn calculate_x_position(line_str: &str, char_col: usize) -> f64 {
    line_str
        .chars()
        .take(char_col)
        .map(|ch| {
            if ch as u32 > 255 {
                CHAR_WIDTH_WIDE
            } else {
                CHAR_WIDTH_ASCII
            }
        })
        .sum::<f64>()
}

/// ✅ Calculate column position from x coordinate
/// Inverse of calculate_x_position - finds the character position from pixel offset
fn get_col_from_x(line_str: &str, x_in_text: f64) -> usize {
    let mut current_x = 0.0;
    let mut col = 0;
    for (i, ch) in line_str.chars().enumerate() {
        if ch == '\n' {
            break;
        }
        let w = if ch as u32 > 255 {
            CHAR_WIDTH_WIDE
        } else {
            CHAR_WIDTH_ASCII
        };
        if x_in_text < current_x + (w / 2.0) {
            break;
        }
        current_x += w;
        col = i + 1;
    }
    col
}

/// ✅ IntelliJ Pro: Extract word at position for Go to Definition
/// Returns the identifier/word at the given character position in the line
fn extract_word_at_position(line: &str, col: usize) -> String {
    let chars: Vec<char> = line.chars().collect();

    if col >= chars.len() {
        return String::new();
    }

    // Check if the character at col is part of an identifier
    if !is_identifier_char(chars[col]) {
        return String::new();
    }

    // Find start of word (go backwards)
    let mut start = col;
    while start > 0 && is_identifier_char(chars[start - 1]) {
        start -= 1;
    }

    // Find end of word (go forwards)
    let mut end = col;
    while end < chars.len() && is_identifier_char(chars[end]) {
        end += 1;
    }

    chars[start..end].iter().collect()
}

/// Helper: Check if character is part of an identifier (alphanumeric or underscore)
fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Represents a single edit operation for undo/redo
#[derive(Clone, Debug)]
enum EditOperation {
    Insert {
        position: usize,
        text: String,
        cursor_before: (usize, usize), // (line, col)
        cursor_after: (usize, usize),
    },
    Delete {
        position: usize,
        text: String,
        cursor_before: (usize, usize),
        cursor_after: (usize, usize),
    },
}

/// Manages undo/redo history
#[derive(Clone, Debug)]
struct UndoHistory {
    undo_stack: Vec<EditOperation>,
    redo_stack: Vec<EditOperation>,
    max_history: usize,
}

impl UndoHistory {
    fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            // ✅ MEMORY FIX: Reduced from 1000 to 100 to save memory
            // Each operation stores text, which can be large
            max_history: 100,
        }
    }

    fn push(&mut self, operation: EditOperation) {
        self.undo_stack.push(operation);
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
        // Clear redo stack when new edit is made
        self.redo_stack.clear();
    }

    fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    fn undo(&mut self) -> Option<EditOperation> {
        if let Some(op) = self.undo_stack.pop() {
            self.redo_stack.push(op.clone());
            Some(op)
        } else {
            None
        }
    }

    fn redo(&mut self) -> Option<EditOperation> {
        if let Some(op) = self.redo_stack.pop() {
            self.undo_stack.push(op.clone());
            Some(op)
        } else {
            None
        }
    }
}

#[derive(Clone)]
struct EditorTab {
    path: String,
    buffer: TextBuffer,
    highlighter: SyntaxHighlighter,
    scroll: VirtualScroll,
    is_modified: bool,
    // ✅ MEMORY FIX: Removed original_content - it duplicated the entire file in memory!
    // The buffer (Rope) is the single source of truth. To check if modified,
    // we can compare buffer hash or use the is_modified flag.
    // Selection state (char indices)
    selection_start: Option<usize>,
    selection_end: Option<usize>,
    // Undo/Redo history
    undo_history: UndoHistory,
    // ✅ Parallel syntax highlighting cache (line_number -> HTML)
    highlight_cache: HashMap<usize, String>,
    // Cursor position
    cursor_line: usize,
    cursor_col: usize,
}

#[component]
pub fn VirtualEditorPanel(selected_file: RwSignal<Option<(String, String)>>) -> impl IntoView {
    let tabs = RwSignal::new(Vec::<EditorTab>::new());
    let active_tab_index = RwSignal::new(0usize);
    let scroll_top = RwSignal::new(0.0);
    let container_ref = NodeRef::<leptos::html::Div>::new();
    let editor_pane_ref = NodeRef::<leptos::html::Div>::new();  // ✅ ContentEditable div reference

    // ✅ IntelliJ Pattern: Independent cursor signals (UI truth source)
    // These are separate from tabs to avoid reactive loops
    let cursor_line = RwSignal::new(0usize);
    let cursor_col = RwSignal::new(0usize);

    // ✅ CRITICAL: Browser-measured character width (synchronized with actual rendering)
    // Start with CSS constant, then update with actual browser measurement
    let actual_char_width = RwSignal::new(CHAR_WIDTH_ASCII);

    // ✅ Selection state (moved to top level to avoid reset on re-render)
    let selection_start = RwSignal::new(None::<usize>);
    let selection_end = RwSignal::new(None::<usize>);

    // ✅ CRITICAL: IME composition state to prevent double input
    // When true, ignore input events (they'll be handled on compositionend)
    let is_composing = RwSignal::new(false);
    // Composition preview text (what user is currently typing in IME)
    let composition_text = RwSignal::new(String::new());
    // Composition start position (line, col) - fixed during composition
    let composition_start_pos = RwSignal::new((0usize, 0usize));

    // ✅ IntelliJ Pro: Auto-completion state
    let show_completion = RwSignal::new(false);
    let completion_items = RwSignal::new(Vec::<CompletionItem>::new());
    let completion_position = RwSignal::new((0usize, 0usize)); // (line, col)

    // LSP client for completions (Rust analyzer) - wrapped in RwSignal to allow multiple accesses
    let lsp_client = RwSignal::new(LspClient::new("rust"));

    // ✅ CRITICAL FIX: Initialize with an empty tab so input works immediately
    Effect::new_isomorphic(move |_| {
        if tabs.with_untracked(|t| t.is_empty()) {
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::prelude::*;
                #[wasm_bindgen]
                extern "C" {
                    #[wasm_bindgen(js_namespace = console)]
                    fn log(s: &str);
                }
                log("🚀 Initializing empty tab for editor startup");
            }

            let empty_buffer = TextBuffer::from_str(
                "// Welcome to BerryEditor\n// Start typing or open a file...\n",
            );
            let mut highlighter = SyntaxHighlighter::new();
            let _ = highlighter.set_language("rust");
            let line_count = empty_buffer.len_lines();
            let scroll = VirtualScroll::new(line_count, 800.0, LINE_HEIGHT);

            let initial_tab = EditorTab {
                path: String::from("Untitled"),
                buffer: empty_buffer,
                highlighter,
                scroll,
                is_modified: false,
                selection_start: None,
                selection_end: None,
                undo_history: UndoHistory::new(),
                highlight_cache: HashMap::new(),
                cursor_line: 0,
                cursor_col: 0,
            };

            tabs.update(|t| t.push(initial_tab));
            active_tab_index.set(0);
        }
    });

    // ✅ CRITICAL: Auto-focus editor pane on mount so keyboard input works immediately
    // Use Effect (Leptos reactive system) to set focus when editor pane is mounted
    Effect::new_isomorphic(move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::prelude::*;
            use wasm_bindgen::JsCast;
            #[wasm_bindgen]
            extern "C" {
                #[wasm_bindgen(js_namespace = console)]
                fn log(s: &str);
            }

            if let Some(el) = editor_pane_ref.get() {
                log("⌨️ Auto-focusing editor pane on mount");

                // Use setTimeout to ensure DOM is fully ready
                let el_clone = el.clone();
                let _ = web_sys::window().and_then(|win| {
                    let closure = Closure::wrap(Box::new(move || {
                        let _ = el_clone.focus();
                        log("✅ Editor pane focused via Effect!");
                    }) as Box<dyn FnMut()>);
                    win.set_timeout_with_callback_and_timeout_and_arguments_0(
                        closure.as_ref().unchecked_ref(),
                        100,
                    )
                    .ok()?;
                    closure.forget();
                    Some(())
                });
            }
        }
    });

    // ✅ CRITICAL: Measure actual character width from browser rendering
    // This ensures Rust calculations match the physical pixel rendering exactly
    Effect::new_isomorphic(move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::prelude::*;
            use wasm_bindgen::JsCast;

            #[wasm_bindgen]
            extern "C" {
                #[wasm_bindgen(js_namespace = console)]
                fn log(s: &str);
            }

            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    // Create temporary element to measure actual character width
                    if let Ok(el) = document.create_element("div") {
                        if let Ok(html_el) = el.dyn_into::<web_sys::HtmlElement>() {
                            let style = html_el.style();
                            let _ = style.set_property("font-family", "'JetBrains Mono', monospace");
                            let _ = style.set_property("font-size", "13px");
                            let _ = style.set_property("position", "absolute");
                            let _ = style.set_property("visibility", "hidden");
                            let _ = style.set_property("white-space", "pre");
                            let _ = style.set_property("font-variant-ligatures", "none");
                            let _ = style.set_property("font-kerning", "none");
                            let _ = style.set_property("letter-spacing", "0px");

                            // Measure 10 ASCII characters to get accurate average width
                            html_el.set_text_content(Some("WWWWWWWWWW"));

                            if let Some(body) = document.body() {
                                let _ = body.append_child(&html_el);

                                // Get actual rendered width and calculate per-character width
                                let width = html_el.get_bounding_client_rect().width() / 10.0;

                                // Clean up
                                let _ = body.remove_child(&html_el);

                                // Update the signal with measured value
                                actual_char_width.set(width);

                                log(&format!("📏 Measured actual char width: {}px (CSS constant: {}px)", width, CHAR_WIDTH_ASCII));
                            }
                        }
                    }
                }
            }
        }
    });

    // ✅ FIX: Direct file selection handling - use .get() to clone and establish dependency
    Effect::new_isomorphic(move |_| {
        // Use .get() to clone the data and establish reactive dependency
        if let Some((path, content)) = selected_file.get() {
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::prelude::*;
                #[wasm_bindgen]
                extern "C" {
                    #[wasm_bindgen(js_namespace = console)]
                    fn log(s: &str);
                }
                log(&format!(
                    "🔥 Effect: Opening file: {}, length: {}",
                    path,
                    content.len()
                ));
            }

            // ✅ Use .get() to establish dependency on tabs (not .get_untracked())
            let current_tabs = tabs.get();
            let existing_tab_index = current_tabs.iter().position(|tab| tab.path == path);

            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::prelude::*;
                #[wasm_bindgen]
                extern "C" {
                    #[wasm_bindgen(js_namespace = console)]
                    fn log(s: &str);
                }
                log(&format!(
                    "Tab check: existing={:?}, current_tabs_len={}",
                    existing_tab_index,
                    current_tabs.len()
                ));
            }

            if let Some(idx) = existing_tab_index {
                #[cfg(target_arch = "wasm32")]
                {
                    use wasm_bindgen::prelude::*;
                    #[wasm_bindgen]
                    extern "C" {
                        #[wasm_bindgen(js_namespace = console)]
                        fn log(s: &str);
                    }
                    log(&format!("Switching to existing tab at index {}", idx));
                }
                // Switch to existing tab
                active_tab_index.set(idx);
                // Force re-render
                scroll_top.set(0.0);
            } else {
                #[cfg(target_arch = "wasm32")]
                {
                    use wasm_bindgen::prelude::*;
                    #[wasm_bindgen]
                    extern "C" {
                        #[wasm_bindgen(js_namespace = console)]
                        fn log(s: &str);
                    }
                    log("Creating new tab...");
                }
                // Create new tab
                let buffer = TextBuffer::from_str(&content);
                let mut highlighter = SyntaxHighlighter::new();

                // Auto-detect language from file extension (case-insensitive)
                let path_lower = path.to_lowercase();
                if path_lower.ends_with(".rs") {
                    let _ = highlighter.set_language("rust");
                } else if path_lower.ends_with(".js") || path_lower.ends_with(".ts") {
                    let _ = highlighter.set_language("javascript");
                } else if path_lower.ends_with(".py") {
                    let _ = highlighter.set_language("python");
                } else if path_lower.ends_with(".toml") {
                    let _ = highlighter.set_language("toml");
                } else if path_lower.ends_with(".md") || path_lower.ends_with(".markdown") {
                    let _ = highlighter.set_language("markdown");
                }

                // Create virtual scroll for this file
                let line_count = buffer.len_lines();
                let viewport_height = 800.0; // Will be updated by resize observer
                let scroll = VirtualScroll::new(line_count, viewport_height, LINE_HEIGHT);

                let tab = EditorTab {
                    path: path.clone(),
                    buffer: buffer.clone(),
                    highlighter,
                    scroll,
                    is_modified: false,
                    // ✅ MEMORY FIX: No original_content clone!
                    selection_start: None,
                    selection_end: None,
                    undo_history: UndoHistory::new(),
                    highlight_cache: HashMap::new(),
                    cursor_line: 0,
                    cursor_col: 0,
                };

                // ✅ 修正: update 内でインデックスまで確定させる
                let mut new_index = 0;
                tabs.update(|t| {
                    t.push(tab);
                    new_index = t.len().saturating_sub(1);
                });

                #[cfg(target_arch = "wasm32")]
                {
                    use wasm_bindgen::prelude::*;
                    #[wasm_bindgen]
                    extern "C" {
                        #[wasm_bindgen(js_namespace = console)]
                        fn log(s: &str);
                    }
                    log(&format!(
                        "✅ Tab added! new_index: {}, total tabs: {}",
                        new_index,
                        new_index + 1
                    ));
                }

                // 即座にセットすることで、描画エンジンに「新しいタブがある」と教え込む
                active_tab_index.set(new_index);

                // ✅ CRITICAL FIX: Reset scroll position for new tab
                // Without this, scroll_top signal retains value from previous tab
                scroll_top.set(0.0);

                // ✅ Parallel syntax highlighting: Send all lines to Tauri backend
                let path_for_highlight = path.clone();
                let line_count = buffer.len_lines();
                spawn_local(async move {
                    // ✅ CRITICAL FIX: Delay highlighting to give UI thread time to process input
                    // This prevents blocking input events when opening large files
                    #[cfg(target_arch = "wasm32")]
                    {
                        gloo_timers::future::TimeoutFuture::new(100).await;
                    }

                    // Prepare lines for parallel highlighting
                    let lines_to_highlight: Vec<(usize, String)> = (0..line_count)
                        .filter_map(|i| buffer.line(i).map(|line| (i, line.to_string())))
                        .collect();

                    // Call Tauri parallel highlighter
                    if let Ok(results) = tauri_bindings::highlight_file_parallel(
                        &path_for_highlight,
                        lines_to_highlight,
                    )
                    .await
                    {
                        // ✅ CRITICAL FIX: Use update_untracked to prevent reactive re-render loop
                        // This allows cache updates without triggering expensive full re-renders
                        tabs.update_untracked(|t| {
                            if let Some(tab) = t.get_mut(new_index) {
                                for result in results {
                                    let html = highlight_result_to_html(&result);
                                    tab.highlight_cache.insert(result.line_number, html);
                                }
                            }
                        });
                    }
                });

                // ✅ Force re-render by toggling scroll (ensures dependency fires)
                scroll_top.set(-1.0);
                scroll_top.set(0.0);
            }
        }
    });

    // ✅ Bulletproof: Tab switch restoration with ghost Effect guard
    // Restore cursor position when tab switches
    Effect::new(move |_| {
        // ✅ 最初の確認
        if active_tab_index.is_disposed() || tabs.is_disposed() {
            return;
        }

        // Get current tab index (reactive trigger)
        let idx = active_tab_index.get();

        // ✅ with_untracked でディスポーズチェック済み
        let data = tabs.with_untracked(|t| t.get(idx).map(|tab| (tab.cursor_line, tab.cursor_col)));

        if let Some((l, c)) = data {
            // ✅ 書き込み直前にもう一度確認
            if !cursor_line.is_disposed() && !cursor_col.is_disposed() {
                cursor_line.set(l);
                cursor_col.set(c);
            }
        }
    });

    // ✅ Bulletproof: Background save with ghost Effect guard
    // Saves cursor position to tab storage when cursor moves
    Effect::new(move |_| {
        // ✅ 最初の確認
        if cursor_line.is_disposed()
            || cursor_col.is_disposed()
            || active_tab_index.is_disposed()
            || tabs.is_disposed()
        {
            return;
        }

        // Track cursor changes (reactive triggers)
        let l = cursor_line.get();
        let c = cursor_col.get();
        let idx = active_tab_index.get_untracked();

        // ✅ Ghost guard: skip if tab doesn't exist
        tabs.update_untracked(|t| {
            if let Some(tab) = t.get_mut(idx) {
                tab.cursor_line = l;
                tab.cursor_col = c;
            }
            // Silently ignore if tab is disposed
        });
    });

    // ✅ Safe active tab accessor (prevents Disposed panic)
    let get_active_tab = move || -> Option<EditorTab> {
        // Use untracked access to prevent reactive loops
        let idx = active_tab_index.get_untracked();
        tabs.with_untracked(|t| t.get(idx).cloned())
    };

    // ✅ IntelliJ Pro: Handle completion selection
    let on_completion_select = move |item: CompletionItem| {
        // Get text to insert (prefer insert_text, fallback to label)
        let insert_text = item.insert_text.unwrap_or(item.label);

        // Get current cursor position
        let insert_idx = tabs.with_untracked(|t| {
            t.get(active_tab_index.get_untracked())
                .map(|tab| {
                    tab.buffer.line_to_char(cursor_line.get_untracked())
                        + cursor_col.get_untracked()
                })
                .unwrap_or(0)
        });

        // Insert completion text
        tabs.update_untracked(|t| {
            if let Some(tab) = t.get_mut(active_tab_index.get_untracked()) {
                tab.buffer.insert(insert_idx, &insert_text);
                tab.is_modified = true;
            }
        });

        // Update cursor position
        let new_idx = insert_idx + insert_text.chars().count();
        let (new_line, new_col) = tabs.with_untracked(|t| {
            if let Some(tab) = t.get(active_tab_index.get()) {
                let line = tab.buffer.char_to_line(new_idx);
                let line_start = tab.buffer.line_to_char(line);
                (line, new_idx - line_start)
            } else {
                (0, 0)
            }
        });

        cursor_line.set(new_line);
        cursor_col.set(new_col);

        // Hide completion widget
        show_completion.set(false);
        completion_items.set(Vec::new());
    };

    // Handle scroll event
    let on_scroll = move |ev: Event| {
        if let Some(target) = ev.target() {
            if let Ok(element) = target.dyn_into::<HtmlElement>() {
                let new_scroll_top = element.scroll_top() as f64;
                scroll_top.set(new_scroll_top);

                // Update virtual scroll in active tab
                tabs.update_untracked(|t| {
                    if let Some(tab) = t.get_mut(active_tab_index.get_untracked()) {
                        tab.scroll.set_scroll_top(new_scroll_top);
                    }
                });
                // No need to trigger re-render for scroll - visual update is handled by scroll_top signal
            }
        }
    };

    // Close tab function
    let close_tab = move |idx: usize| {
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

    // ✅ Bulletproof keyboard handler: prevents ghost handler from accessing disposed signals
    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        // ✅ CRITICAL: Complete guard - abort if any signal is disposed (zombie handler)
        // This prevents panics from "ReadUntracked on disposed signal"
        if tabs.is_disposed() || active_tab_index.is_disposed() || cursor_line.is_disposed() || cursor_col.is_disposed() {
            return;
        }

        // ✅ Additional safety check: verify active tab exists
        let tab_exists = tabs.try_with_untracked(|t| {
            let idx = active_tab_index.get_untracked();
            t.get(idx).is_some()
        }).unwrap_or(false);

        if !tab_exists {
            return;
        }

        let key = ev.key();

        // ✅ Handle Cmd+S / Ctrl+S for Save
        if (ev.meta_key() || ev.ctrl_key()) && key == "s" {
            ev.prevent_default();
            ev.stop_propagation();

            let idx = active_tab_index.get_untracked();
            let save_data = tabs.with_untracked(|t| {
                t.get(idx).map(|tab| (tab.path.clone(), tab.buffer.to_string()))
            });

            if let Some((path, content)) = save_data {
                // Skip saving "Untitled" tabs
                if path != "Untitled" {
                    spawn_local(async move {
                        match tauri_bindings::write_file(&path, &content).await {
                            Ok(_) => {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    use wasm_bindgen::prelude::*;
                                    #[wasm_bindgen]
                                    extern "C" {
                                        #[wasm_bindgen(js_namespace = console)]
                                        fn log(s: &str);
                                    }
                                    log(&format!("✅ File saved: {}", path));
                                }

                                // Mark as not modified
                                tabs.update_untracked(|t| {
                                    if let Some(tab) = t.get_mut(idx) {
                                        tab.is_modified = false;
                                    }
                                });
                            }
                            Err(e) => {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    use wasm_bindgen::prelude::*;
                                    #[wasm_bindgen]
                                    extern "C" {
                                        #[wasm_bindgen(js_namespace = console)]
                                        fn log(s: &str);
                                    }
                                    log(&format!("❌ Failed to save file: {}", e));
                                }
                            }
                        }
                    });
                }
            }
            return;
        }

        // ✅ Handle Cmd+Z (macOS) / Ctrl+Z (Windows/Linux) for Undo
        // ✅ Handle Cmd+Shift+Z (macOS) / Ctrl+Y (Windows/Linux) for Redo
        // Check both meta_key (Cmd on Mac) and ctrl_key (Ctrl on Windows/Linux)
        let is_undo = (ev.meta_key() || ev.ctrl_key()) && key == "z" && !ev.shift_key();

        if is_undo {
            // Undo
            ev.prevent_default();
            ev.stop_propagation();

            let idx = active_tab_index.get_untracked();
            tabs.update(|t| {
                if let Some(tab) = t.get_mut(idx) {
                    if let Some(op) = tab.undo_history.undo() {
                        match op {
                            EditOperation::Insert {
                                position,
                                text,
                                cursor_before,
                                ..
                            } => {
                                // Undo insert: remove the inserted text
                                tab.buffer.remove(position, position + text.len());
                                cursor_line.set(cursor_before.0);
                                cursor_col.set(cursor_before.1);
                                tab.highlight_cache.remove(&cursor_before.0);
                            }
                            EditOperation::Delete {
                                position,
                                text,
                                cursor_before,
                                ..
                            } => {
                                // Undo delete: re-insert the deleted text
                                tab.buffer.insert(position, &text);
                                cursor_line.set(cursor_before.0);
                                cursor_col.set(cursor_before.1);
                                tab.highlight_cache.remove(&cursor_before.0);
                            }
                        }
                        tab.is_modified = true;
                    }
                }
            });
            return;
        }

        let is_redo =
            (ev.meta_key() || ev.ctrl_key()) && ((key == "z" && ev.shift_key()) || key == "y");

        if is_redo {
            // Redo
            ev.prevent_default();
            ev.stop_propagation();

            let idx = active_tab_index.get_untracked();
            tabs.update(|t| {
                if let Some(tab) = t.get_mut(idx) {
                    if let Some(op) = tab.undo_history.redo() {
                        match op {
                            EditOperation::Insert {
                                position,
                                text,
                                cursor_after,
                                ..
                            } => {
                                // Redo insert: re-insert the text
                                tab.buffer.insert(position, &text);
                                cursor_line.set(cursor_after.0);
                                cursor_col.set(cursor_after.1);
                                tab.highlight_cache.remove(&cursor_after.0);
                            }
                            EditOperation::Delete {
                                position,
                                text,
                                cursor_after,
                                ..
                            } => {
                                // Redo delete: remove the text again
                                tab.buffer.remove(position, position + text.len());
                                cursor_line.set(cursor_after.0);
                                cursor_col.set(cursor_after.1);
                                tab.highlight_cache.remove(&cursor_after.0);
                            }
                        }
                        tab.is_modified = true;
                    }
                }
            });
            return;
        }

        // Prevent default for editor keys only
        match key.as_str() {
            "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight" | "Backspace" | "Enter" => {
                ev.prevent_default();
                ev.stop_propagation();
            }
            _ => return, // ✅ Early return for non-editor keys
        }

        // ✅ Ghost guard: verify component is still alive
        let idx = active_tab_index.get_untracked();
        let tab_exists = tabs.with_untracked(|t| t.get(idx).is_some());
        if !tab_exists {
            return; // Component disposed, skip safely
        }

        // ✅ Arrow keys: Update cursor position only (independent cursor layer will re-render)
        match key.as_str() {
            "ArrowUp" => {
                cursor_line.update(|l| *l = l.saturating_sub(1));
            }
            "ArrowDown" => {
                let max_lines = tabs
                    .with_untracked(|t| t.get(idx).map(|tab| tab.buffer.len_lines()).unwrap_or(1));
                cursor_line.update(|l| *l = (*l + 1).min(max_lines.saturating_sub(1)));
            }
            "ArrowLeft" => {
                let (line, col) = (cursor_line.get_untracked(), cursor_col.get_untracked());
                if col > 0 {
                    cursor_col.update(|c| *c = col - 1);
                } else if line > 0 {
                    cursor_line.update(|l| *l = line - 1);
                    let prev_line_len = tabs.with_untracked(|t| {
                        t.get(idx)
                            .and_then(|tab| tab.buffer.line(line - 1))
                            .map(|s| s.trim_end_matches('\n').len())
                            .unwrap_or(0)
                    });
                    cursor_col.update(|c| *c = prev_line_len);
                }
            }
            "ArrowRight" => {
                let (line, col) = (cursor_line.get_untracked(), cursor_col.get_untracked());
                let (line_len, total_lines) = tabs.with_untracked(|t| {
                    if let Some(tab) = t.get(idx) {
                        let ll = tab
                            .buffer
                            .line(line)
                            .map(|s| s.trim_end_matches('\n').len())
                            .unwrap_or(0);
                        (ll, tab.buffer.len_lines())
                    } else {
                        (0, 1)
                    }
                });

                if col < line_len {
                    cursor_col.update(|c| *c = col + 1);
                } else if line + 1 < total_lines {
                    cursor_line.update(|l| *l = line + 1);
                    cursor_col.update(|c| *c = 0);
                }
            }
            "Backspace" => {
                let (line, col) = (cursor_line.get_untracked(), cursor_col.get_untracked());
                // ✅ Use update() to trigger re-render so deletion is visible
                tabs.update(|t| {
                    if let Some(tab) = t.get_mut(idx) {
                        if col > 0 {
                            // Delete character before cursor
                            let pos = tab.buffer.line_to_char(line) + col;
                            // ✅ Record operation for undo
                            let deleted_text = tab.buffer.slice(pos - 1, pos).unwrap_or_default();
                            tab.undo_history.push(EditOperation::Delete {
                                position: pos - 1,
                                text: deleted_text,
                                cursor_before: (line, col),
                                cursor_after: (line, col - 1),
                            });

                            tab.buffer.remove(pos - 1, pos);
                            tab.is_modified = true;
                            tab.highlight_cache.remove(&line);
                        } else if line > 0 {
                            // Delete newline, merge with previous line
                            let prev_line_len = tab
                                .buffer
                                .line(line - 1)
                                .map(|s| s.trim_end_matches('\n').len())
                                .unwrap_or(0);
                            let pos = tab.buffer.line_to_char(line);
                            if pos > 0 {
                                // ✅ Record operation for undo
                                let deleted_text =
                                    tab.buffer.slice(pos - 1, pos).unwrap_or_default();
                                tab.undo_history.push(EditOperation::Delete {
                                    position: pos - 1,
                                    text: deleted_text,
                                    cursor_before: (line, col),
                                    cursor_after: (line - 1, prev_line_len),
                                });

                                tab.buffer.remove(pos - 1, pos);
                                tab.is_modified = true;
                                tab.highlight_cache.remove(&(line - 1));
                                tab.highlight_cache.remove(&line);
                                // Update cursor after deleting
                                cursor_line.set(line - 1);
                                cursor_col.set(prev_line_len);
                            }
                        }
                    }
                });
                // Update cursor position for normal backspace
                if col > 0 {
                    cursor_col.set(col - 1);
                }
            }
            "Enter" => {
                // ✅ CRITICAL: Complete IME guard - prevents double newline on Japanese input confirmation

                // 1. If IME composition is in progress, ignore this Enter (let browser handle it)
                if is_composing.get_untracked() {
                    return;
                }

                // 2. Check browser's native composition state (most reliable)
                // This catches "confirmation Enter" that hasn't updated our flag yet
                if ev.is_composing() {
                    return;
                }

                // 3. Check keyCode == 229 (standard IME processing indicator)
                // Even if key == "Enter", keyCode 229 means "IME is handling this"
                if ev.key_code() == 229 {
                    return;
                }

                // ✅ Only "real" Enter reaches here - prevent default and stop propagation
                ev.prevent_default();
                ev.stop_propagation();

                let (line, col) = (cursor_line.get_untracked(), cursor_col.get_untracked());

                // ✅ Use try_update for safe signal access
                let _ = tabs.try_update(|t| {
                    if let Some(tab) = t.get_mut(idx) {
                        let pos = tab.buffer.line_to_char(line) + col;
                        // ✅ Record operation for undo
                        tab.undo_history.push(EditOperation::Insert {
                            position: pos,
                            text: "\n".to_string(),
                            cursor_before: (line, col),
                            cursor_after: (line + 1, 0),
                        });

                        tab.buffer.insert(pos, "\n");
                        tab.is_modified = true;
                        // Clear cache for current and next line
                        tab.highlight_cache.remove(&line);
                        tab.highlight_cache.remove(&(line + 1));
                    }
                });

                // Update cursor position
                cursor_line.set(line + 1);
                cursor_col.set(0);
            }
            _ => {}
        }
    };

    // ❌ on_input_handler removed - using on:beforeinput on ContentEditable div

    view! {
        <div
            class="berry-editor-main"
            node_ref=container_ref
            tabindex="0"
            on:mousedown=move |ev| {
                ev.prevent_default();
                // ✅ CRITICAL: Force focus on editor pane with every click
                if let Some(el) = editor_pane_ref.get() {
                    let _ = el.focus();
                }
            }
            style="outline: none; position: relative; height: 100%; width: 100%; display: flex; flex-direction: column; cursor: text;"
        >
            // ❌ textarea removed - using ContentEditable on berry-editor-pane instead

            // Tab Bar
            <div class="berry-editor-tab-bar">
                {move || {
                    // ✅ Ensure reactive dependency on tabs
                    let current_tabs = tabs.get();
                    // ✅ Read render_trigger to force re-render when tabs change
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

            // Editor Pane with Virtual Scrolling (✅ ContentEditable)
            <div
                class="berry-editor-pane"
                node_ref=editor_pane_ref
                contenteditable="true"
                spellcheck="false"
                tabindex="0"
                on:scroll=on_scroll
                on:keydown=handle_keydown
                // ✅ CRITICAL: IME Composition Events - Lock Rust updates during Japanese input
                on:compositionstart=move |_ev| {
                    // ✅ Lock: Browser handles IME display during composition
                    is_composing.set(true);
                }
                on:compositionend=move |ev: web_sys::CompositionEvent| {
                    // ✅ Disposal guard
                    if tabs.is_disposed() || active_tab_index.is_disposed() || cursor_line.is_disposed() || cursor_col.is_disposed() {
                        return;
                    }

                    // ✅ Unlock: Composition finished, commit to Rust buffer
                    is_composing.set(false);

                    // Get confirmed text from IME
                    if let Some(data) = ev.data() {
                        let idx = active_tab_index.get_untracked();
                        let line = cursor_line.get_untracked();
                        let col = cursor_col.get_untracked();
                        let char_count = data.chars().count();

                        // ✅ Update buffer with confirmed IME text
                        let _ = tabs.try_update(|t| {
                            if let Some(tab) = t.get_mut(idx) {
                                let pos = tab.buffer.line_to_char(line) + col;

                                tab.undo_history.push(EditOperation::Insert {
                                    position: pos,
                                    text: data.clone(),
                                    cursor_before: (line, col),
                                    cursor_after: (line, col + char_count),
                                });

                                tab.buffer.insert(pos, &data);
                                tab.is_modified = true;
                                tab.highlight_cache.remove(&line);
                            }
                        });

                        // Update cursor position
                        cursor_col.update(|c| *c += char_count);
                    }

                    // ✅ CRITICAL: Clean up browser's leftover DOM after IME confirmation
                    // This prevents double rendering (browser's text + Rust's text)
                    if let Some(el) = editor_pane_ref.get() {
                        el.set_text_content(None);  // Clear internal DOM, let Rust re-render
                    }
                }
                on:beforeinput=move |ev: web_sys::InputEvent| {
                    // ✅ CRITICAL: Disposal guard
                    if tabs.is_disposed() || active_tab_index.is_disposed() || cursor_line.is_disposed() || cursor_col.is_disposed() {
                        return;
                    }

                    // ✅ CRITICAL: Ignore input during IME composition (prevents double insertion)
                    if is_composing.get_untracked() {
                        return;
                    }

                    // ✅ CRITICAL: Prevent browser's default DOM manipulation
                    ev.prevent_default();

                    // Get input data (for non-IME input like English)
                    if let Some(data) = ev.data() {
                        let idx = active_tab_index.get_untracked();
                        let line = cursor_line.get_untracked();
                        let col = cursor_col.get_untracked();
                        let char_count = data.chars().count();

                        // ✅ Update buffer with input data
                        let _ = tabs.try_update(|t| {
                            if let Some(tab) = t.get_mut(idx) {
                                let pos = tab.buffer.line_to_char(line) + col;

                                tab.undo_history.push(EditOperation::Insert {
                                    position: pos,
                                    text: data.clone(),
                                    cursor_before: (line, col),
                                    cursor_after: (line, col + char_count),
                                });

                                tab.buffer.insert(pos, &data);
                                tab.is_modified = true;
                                tab.highlight_cache.remove(&line);
                            }
                        });

                        // Update cursor position
                        cursor_col.update(|c| *c += char_count);
                    }
                }
                on:mousedown=move |ev| {
                    // ❌ DO NOT call ev.prevent_default() - it kills browser's native text selection!
                    // ✅ Only handle focus, let browser handle selection
                    if let Some(target) = ev.target() {
                        if let Ok(element) = target.dyn_into::<web_sys::HtmlElement>() {
                            if element.class_list().contains("berry-editor-pane") {
                                // Background click - focus editor pane
                                if let Some(el) = editor_pane_ref.get() {
                                    let _ = el.focus();
                                }
                            }
                        }
                    }
                }
                style="position: relative; overflow: auto; height: 100%; background: #1E1E1E; display: flex; caret-color: transparent; outline: none; user-select: text;"
            >
                {move || {
                    // ✅ Disposed check first
                    if tabs.is_disposed() || active_tab_index.is_disposed() {
                        return view! { <div></div> }.into_any();
                    }

                    let idx = active_tab_index.get();

                    #[cfg(target_arch = "wasm32")]
                    {
                        use wasm_bindgen::prelude::*;
                        #[wasm_bindgen]
                        extern "C" {
                            #[wasm_bindgen(js_namespace = console)]
                            fn log(s: &str);
                        }
                        let count = tabs.with(|t| t.len());
                        log(&format!("RENDER: active_tab_index={}, tabs.len()={}", idx, count));
                    }

                    // ✅ CRITICAL: Read tabs.with() to create reactive dependency
                    // This detects:
                    //   1. New tabs added (tabs.len() changes)
                    //   2. Buffer updates (buffer.version() changes)
                    //   3. Tab switches (active_tab_index changes)
                    let tab_data = tabs.with(|t| {
                        t.get(idx).map(|tab| {
                            (
                                tab.buffer.len_lines(),
                                tab.buffer.version(),  // ✅ Read version to detect buffer changes
                                tab.scroll.clone(),
                                tab.buffer.clone(),  // ✅ Clone buffer for rendering (Ropey clone is O(1))
                            )
                        })
                    });

                    let Some((line_count, buffer_version, scroll_state, buffer)) = tab_data else {
                        // Debug: Show why no tab is available
                        let tabs_count = tabs.with(|t| t.len());
                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::prelude::*;
                            #[wasm_bindgen]
                            extern "C" {
                                #[wasm_bindgen(js_namespace = console)]
                                fn log(s: &str);
                            }
                            log(&format!("RENDER ERROR: No tab at index {}. Total tabs: {}", idx, tabs_count));
                        }
                        return view! {
                            <div class="empty-screen" style="padding: 20px; color: white;">
                                {format!("No tab at index {}. Total tabs: {}", idx, tabs_count)}
                            </div>
                        }.into_any();
                    };

                    // Extract values from tab_data
                    let line_count_val = line_count;
                    let buffer_clone = buffer.clone();

                    #[cfg(target_arch = "wasm32")]
                    {
                        use wasm_bindgen::prelude::*;
                        #[wasm_bindgen]
                        extern "C" {
                            #[wasm_bindgen(js_namespace = console)]
                            fn log(s: &str);
                        }
                        log(&format!("RENDER SUCCESS: line_count={}, buffer_version={}", line_count_val, buffer_version));
                    }

                    // ✅ Calculate total height
                    let total_height = line_count_val.max(1) as f64 * LINE_HEIGHT;

                    // ✅ NEW STRUCTURE: Simple, reactive layout
                    return view! {
                        <div class="berry-editor-scroll-content" style=format!("height: {}px; width: 100%; position: relative; display: flex;", total_height)>

                            // [Layer 1] Line Number Gutter (Sticky, z-index: 20)
                            <div class="berry-editor-gutter" style=format!("width: {}px; background: #313335; border-right: 1px solid #323232; position: sticky; left: 0; z-index: 20; height: 100%;", GUTTER_WIDTH)>
                                {move || {
                                    let current_scroll = scroll_top.get();
                                    let start_line = (current_scroll / LINE_HEIGHT).floor() as usize;
                                    let end_line = (start_line + 50).min(line_count_val);

                                    view! {
                                        <div style=format!("position: absolute; top: {}px; width: 100%;", start_line as f64 * LINE_HEIGHT)>
                                            {(start_line..end_line).map(|n| {
                                                view! {
                                                    <div style=format!("height: {}px; color: #606366; font-size: 13px; text-align: right; padding-right: 8px; font-family: 'JetBrains Mono', monospace; line-height: {}px;", LINE_HEIGHT, LINE_HEIGHT)>
                                                        {n + 1}
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }
                                }}
                            </div>

                            // [Layer 2] Text Display Area (receives click events)
                            <div
                                class="berry-editor-lines-container"
                                style="flex: 1; position: relative; height: 100%; cursor: text;"
                                on:mousedown=move |ev: web_sys::MouseEvent| {
                                    // ✅ Calculate click position - MUST add scroll_top for correct line calculation
                                    if let Some(target) = ev.current_target() {
                                        let element: web_sys::HtmlElement = target.dyn_into().unwrap();
                                        let rect = element.get_bounding_client_rect();
                                        let s_top = scroll_top.get_untracked();

                                        let rel_x = ev.client_x() as f64 - rect.left();
                                        let rel_y = ev.client_y() as f64 - rect.top() + s_top; // Add scroll position!

                                        let line = (rel_y / LINE_HEIGHT).floor() as usize;
                                        let x_in_text = (rel_x - TEXT_PADDING).max(0.0);

                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            use wasm_bindgen::prelude::*;
                                            #[wasm_bindgen]
                                            extern "C" {
                                                #[wasm_bindgen(js_namespace = console)]
                                                fn log(s: &str);
                                            }
                                            log(&format!("🖱️ Click: client=({}, {}), rect=({}, {}), rel=({}, {}), line={}, x_in_text={}",
                                                ev.client_x(), ev.client_y(), rect.left(), rect.top(), rel_x, rel_y, line, x_in_text));
                                        }

                                        tabs.with_untracked(|t| {
                                            if let Some(tab) = t.get(active_tab_index.get_untracked()) {
                                                let clamped_line = line.min(tab.buffer.len_lines().saturating_sub(1));
                                                let line_str = tab.buffer.line(clamped_line).unwrap_or_default();

                                                // ✅ Find character position
                                                let mut current_x = 0.0;
                                                let mut col = 0;
                                                for (i, ch) in line_str.chars().enumerate() {
                                                    if ch == '\n' { break; }
                                                    let w = if ch as u32 > 255 { CHAR_WIDTH_WIDE } else { CHAR_WIDTH_ASCII };
                                                    if x_in_text < current_x + (w / 2.0) { break; }
                                                    current_x += w;
                                                    col = i + 1;
                                                }

                                                #[cfg(target_arch = "wasm32")]
                                                {
                                                    use wasm_bindgen::prelude::*;
                                                    #[wasm_bindgen]
                                                    extern "C" {
                                                        #[wasm_bindgen(js_namespace = console)]
                                                        fn log(s: &str);
                                                    }
                                                    log(&format!("📍 Cursor: line={}, col={}, line_text={:?}", clamped_line, col, line_str.chars().take(50).collect::<String>()));
                                                }

                                                cursor_line.set(clamped_line);
                                                cursor_col.set(col);

                                                // Initialize selection range
                                                let char_idx = tab.buffer.line_to_char(clamped_line) + col;
                                                selection_start.set(Some(char_idx));
                                                selection_end.set(Some(char_idx));
                                            }
                                        });

                                        // ✅ Focus editor pane after setting cursor position
                                        if let Some(el) = editor_pane_ref.get() {
                                            let _ = el.focus();
                                        }
                                    }
                                }
                            >
                                // ✅ Virtual Cursor Layer (z-index: 30)
                                <div style=move || {
                                    if cursor_line.is_disposed() || cursor_col.is_disposed() {
                                        return "display: none;".to_string();
                                    }

                                    let l = cursor_line.get();
                                    let c = cursor_col.get();
                                    let current_idx = active_tab_index.get();

                                    let x_offset = tabs.with(|t| {
                                        t.get(current_idx).and_then(|tab| {
                                            tab.buffer.line(l).map(|s| calculate_x_position(&s, c))
                                        })
                                    }).unwrap_or(0.0);

                                    format!(
                                        "position: absolute; left: {}px; top: {}px; width: 2px; height: 18px; background: #aeafad; z-index: 30; pointer-events: none; animation: blink 1s step-end infinite;",
                                        TEXT_PADDING + x_offset,
                                        l as f64 * LINE_HEIGHT
                                    )
                                }></div>

                                // ✅ IME Composition Preview Layer (z-index: 31)
                                // Shows preview text while typing with IME (Japanese/Chinese/Korean)
                                {move || {
                                    let comp_text = composition_text.get();

                                    // ✅ Use saved composition start position (not current cursor position)
                                    let (l, c) = composition_start_pos.get();
                                    let current_idx = active_tab_index.get();

                                    let x_offset = tabs.with(|t| {
                                        t.get(current_idx).and_then(|tab| {
                                            tab.buffer.line(l).map(|s| calculate_x_position(&s, c))
                                        })
                                    }).unwrap_or(0.0);

                                    let style = if !comp_text.is_empty() {
                                        format!(
                                            "position: absolute; left: {}px; top: {}px; color: #aeafad; background: #1e1e1e; text-decoration: underline; pointer-events: none; z-index: 31; font-family: 'JetBrains Mono', monospace; font-size: 13px; line-height: {}px; white-space: pre; padding: 0 2px;",
                                            TEXT_PADDING + x_offset,
                                            l as f64 * LINE_HEIGHT,
                                            LINE_HEIGHT
                                        )
                                    } else {
                                        "display: none;".to_string()
                                    };

                                    view! {
                                        <span style=style>
                                            {comp_text}
                                        </span>
                                    }
                                }}

                                // ✅ E2E Test API: Hidden debug element for automated tests
                                <div
                                    id="berry-test-api"
                                    data-testid="buffer-state"
                                    style="display: none;"
                                    data-buffer-content=move || {
                                        tabs.with(|t| {
                                            t.get(active_tab_index.get()).map(|tab| {
                                                tab.buffer.to_string()
                                            }).unwrap_or_default()
                                        })
                                    }
                                    data-cursor-line=move || cursor_line.get().to_string()
                                    data-cursor-col=move || cursor_col.get().to_string()
                                    data-line-count=move || {
                                        tabs.with(|t| {
                                            t.get(active_tab_index.get()).map(|tab| {
                                                tab.buffer.len_lines()
                                            }).unwrap_or(0).to_string()
                                        })
                                    }
                                ></div>

                                // ✅ Visible Lines Viewport (z-index: 10)
                                {move || {
                                    let current_scroll = scroll_top.get();
                                    let start_line = (current_scroll / LINE_HEIGHT).floor() as usize;
                                    let end_line = (start_line + 50).min(line_count_val);

                                    // ✅ Force reactivity: get tabs version to trigger re-render on cache updates
                                    let _ = tabs.get();

                                    view! {
                                        <div class="berry-editor-viewport" style=format!("position: absolute; top: {}px; left: 0; width: 100%; z-index: 10; user-select: text; cursor: text;", start_line as f64 * LINE_HEIGHT)>
                                            {(start_line..end_line).filter_map(|line_idx| {
                                                tabs.with(|t| {
                                                    if let Some(tab) = t.get(idx) {
                                                        tab.buffer.line(line_idx).map(|line_text| {
                                                            let text = line_text.to_string();
                                                            let cached_html = tab.highlight_cache.get(&line_idx).cloned();

                                                            let final_html = match cached_html {
                                                                Some(html) if !html.is_empty() => html,
                                                                _ => syntax_highlight_line(&text),
                                                            };

                                                            view! {
                                                                <div
                                                                    class="berry-editor-line"
                                                                    style=format!("height: {}px; line-height: {}px; padding-left: {}px; white-space: pre; font-family: 'JetBrains Mono', monospace; font-size: 13px; user-select: text;", LINE_HEIGHT, LINE_HEIGHT, TEXT_PADDING)
                                                                    inner_html=final_html
                                                                ></div>
                                                            }
                                                        })
                                                    } else {
                                                        None
                                                    }
                                                })
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }
                                }}
                            </div>
                        </div>
                    }.into_any();
                }}
            </div>

            // Status Bar
            <div class="berry-editor-status-bar">
                {move || {
                    let idx = active_tab_index.get();
                    if let Some(tab) = tabs.with(|t| t.get(idx).cloned()) {
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

/// Convert parallel highlighting result to HTML
fn highlight_result_to_html(result: &HighlightResult) -> String {
    let mut html = String::new();
    for token in &result.tokens {
        html.push_str(&format!(
            "<span style=\"color: {}\">{}</span>",
            token.color,
            html_escape(&token.text)
        ));
    }
    html
}

/// IntelliJ Darcula syntax highlighting for Rust code (fallback for non-Tauri mode)
fn syntax_highlight_line(line: &str) -> String {
    let keywords = [
        "fn", "let", "mut", "const", "static", "impl", "trait", "struct", "enum", "mod", "pub",
        "use", "crate", "self", "super", "async", "await", "move", "if", "else", "match", "loop",
        "while", "for", "in", "return", "break", "continue", "as", "ref", "where", "unsafe",
        "extern", "type", "dyn",
    ];

    let types = [
        "String", "str", "usize", "isize", "f64", "f32", "i32", "u32", "i64", "u64", "bool", "Vec",
        "Option", "Result", "Some", "None", "Ok", "Err", "Box", "Rc", "Arc", "RefCell", "RwSignal",
    ];

    let mut result = String::new();
    let mut chars = line.chars().peekable();
    let mut current_word = String::new();
    let mut in_string = false;
    let mut in_comment = false;
    let mut in_attribute = false;
    let mut string_char = ' ';

    while let Some(ch) = chars.next() {
        // Handle comments
        if !in_string && !in_attribute && ch == '/' && chars.peek() == Some(&'/') {
            in_comment = true;
            flush_word(&mut result, &mut current_word, &keywords, &types);
            result.push_str("<span style=\"color:#629755;font-style:italic\">"); // IntelliJ Darcula comment color
            result.push_str(&escape_html_char(ch));
            continue;
        }

        if in_comment {
            result.push_str(&escape_html_char(ch));
            continue;
        }

        // ✅ IntelliJ Pattern: Handle attributes #[...]
        if !in_string && ch == '#' && chars.peek() == Some(&'[') {
            in_attribute = true;
            flush_word(&mut result, &mut current_word, &keywords, &types);
            result.push_str("<span style=\"color:#bbb529\">"); // Darcula attribute color
            result.push_str(&escape_html_char(ch));
            continue;
        }

        if in_attribute {
            result.push_str(&escape_html_char(ch));
            if ch == ']' {
                result.push_str("</span>");
                in_attribute = false;
            }
            continue;
        }

        // Handle strings
        if (ch == '"' || ch == '\'') && !in_string {
            in_string = true;
            string_char = ch;
            flush_word(&mut result, &mut current_word, &keywords, &types);
            result.push_str("<span style=\"color:#6a8759\">"); // Darcula string color
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
            result.push_str(&escape_html_char(ch));
        }
    }

    flush_word(&mut result, &mut current_word, &keywords, &types);

    if in_comment {
        result.push_str("</span>");
    }
    if in_attribute {
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
        // ✅ IntelliJ Pattern: Check for SCREAMING_SNAKE_CASE constants
        let is_constant = current_word.len() > 1
            && current_word
                .chars()
                .all(|c| c.is_uppercase() || c.is_numeric() || c == '_')
            && current_word.chars().any(|c| c.is_uppercase());

        if keywords.contains(&current_word.as_str()) {
            // Darcula keyword color (orange) with bold
            result.push_str(&format!(
                "<span style=\"color:#cc7832;font-weight:bold\">{}</span>",
                html_escape(current_word)
            ));
        } else if types.contains(&current_word.as_str()) {
            // IntelliJ Darcula type color (light gray)
            result.push_str(&format!(
                "<span style=\"color:#A9B7C6\">{}</span>",
                html_escape(current_word)
            ));
        } else if is_constant {
            // ✅ IntelliJ Pattern: Darcula constant color (purple) for SCREAMING_SNAKE_CASE
            result.push_str(&format!(
                "<span style=\"color:#9876aa\">{}</span>",
                html_escape(current_word)
            ));
        } else if current_word
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            // User-defined types (IntelliJ Darcula light gray)
            result.push_str(&format!(
                "<span style=\"color:#A9B7C6\">{}</span>",
                html_escape(current_word)
            ));
        } else {
            // Default text color
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
    // ✅ No space→nbsp conversion - CSS white-space: pre handles spacing
}
