//! Canvas-based Virtual Editor
//!
//! Phase 1: Basic Canvas rendering without input
//! - Display canvas element
//! - Render text from Rope buffer
//! - Draw cursor

use crate::buffer::TextBuffer;
use crate::core::canvas_renderer::{CanvasRenderer, LINE_HEIGHT};
use crate::syntax::SyntaxHighlighter;
use leptos::html::Canvas;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

// Undo/Redo用の状態スナップショット
#[derive(Clone)]
struct EditorSnapshot {
    buffer: TextBuffer,
    cursor_line: usize,
    cursor_col: usize,
}

// エディタタブ（簡略版）
// Note: Ropeのcloneは O(1) なので、Rcは不要
#[derive(Clone)]
struct EditorTab {
    file_path: String,
    buffer: TextBuffer,
    cursor_line: usize,
    cursor_col: usize,
    scroll_top: f64,
    // テキスト選択範囲
    selection_start: Option<(usize, usize)>, // (line, col)
    selection_end: Option<(usize, usize)>,   // (line, col)
    // Undo/Redo履歴
    undo_stack: Vec<EditorSnapshot>,
    redo_stack: Vec<EditorSnapshot>,
    // シンタックスハイライト
    syntax_highlighter: SyntaxHighlighter,
}

impl EditorTab {
    fn new(file_path: String, content: String) -> Self {
        // ファイル拡張子から言語を推測
        let mut syntax_highlighter = SyntaxHighlighter::new();
        if file_path.ends_with(".rs") {
            let _ = syntax_highlighter.set_language("rust");
        } else if file_path.ends_with(".js") || file_path.ends_with(".jsx") {
            let _ = syntax_highlighter.set_language("javascript");
        } else if file_path.ends_with(".py") {
            let _ = syntax_highlighter.set_language("python");
        }

        Self {
            file_path,
            buffer: TextBuffer::from_str(&content),
            cursor_line: 0,
            cursor_col: 0,
            scroll_top: 0.0,
            selection_start: None,
            selection_end: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            syntax_highlighter,
        }
    }

    // 現在の状態をUndoスタックに保存
    fn save_undo_state(&mut self) {
        let snapshot = EditorSnapshot {
            buffer: self.buffer.clone(),
            cursor_line: self.cursor_line,
            cursor_col: self.cursor_col,
        };
        self.undo_stack.push(snapshot);
        // Undoスタックは最大100個まで
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        // 新しい編集が行われたらRedoスタックをクリア
        self.redo_stack.clear();
    }

    // Undo実行
    fn undo(&mut self) -> bool {
        if let Some(snapshot) = self.undo_stack.pop() {
            // 現在の状態をRedoスタックに保存
            let redo_snapshot = EditorSnapshot {
                buffer: self.buffer.clone(),
                cursor_line: self.cursor_line,
                cursor_col: self.cursor_col,
            };
            self.redo_stack.push(redo_snapshot);

            // 状態を復元
            self.buffer = snapshot.buffer;
            self.cursor_line = snapshot.cursor_line;
            self.cursor_col = snapshot.cursor_col;
            self.clear_selection();
            true
        } else {
            false
        }
    }

    // Redo実行
    fn redo(&mut self) -> bool {
        if let Some(snapshot) = self.redo_stack.pop() {
            // 現在の状態をUndoスタックに保存
            let undo_snapshot = EditorSnapshot {
                buffer: self.buffer.clone(),
                cursor_line: self.cursor_line,
                cursor_col: self.cursor_col,
            };
            self.undo_stack.push(undo_snapshot);

            // 状態を復元
            self.buffer = snapshot.buffer;
            self.cursor_line = snapshot.cursor_line;
            self.cursor_col = snapshot.cursor_col;
            self.clear_selection();
            true
        } else {
            false
        }
    }

    // 選択範囲があるかチェック
    fn has_selection(&self) -> bool {
        self.selection_start.is_some() && self.selection_end.is_some()
    }

    // 選択範囲をクリア
    fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
    }

    // 選択範囲のテキストを取得
    fn get_selected_text(&self) -> Option<String> {
        if !self.has_selection() {
            return None;
        }

        let (start_line, start_col) = self.selection_start?;
        let (end_line, end_col) = self.selection_end?;

        // 選択範囲を正規化（開始 < 終了）
        let ((sl, sc), (el, ec)) = if start_line < end_line || (start_line == end_line && start_col <= end_col) {
            ((start_line, start_col), (end_line, end_col))
        } else {
            ((end_line, end_col), (start_line, start_col))
        };

        let start_char = self.buffer.line_to_char(sl) + sc;
        let end_char = self.buffer.line_to_char(el) + ec;

        self.buffer.slice(start_char, end_char)
    }

    // 選択範囲のテキストを削除
    fn delete_selection(&mut self) {
        if !self.has_selection() {
            return;
        }

        let (start_line, start_col) = self.selection_start.unwrap();
        let (end_line, end_col) = self.selection_end.unwrap();

        // 選択範囲を正規化
        let ((sl, sc), (el, ec)) = if start_line < end_line || (start_line == end_line && start_col <= end_col) {
            ((start_line, start_col), (end_line, end_col))
        } else {
            ((end_line, end_col), (start_line, start_col))
        };

        let start_char = self.buffer.line_to_char(sl) + sc;
        let end_char = self.buffer.line_to_char(el) + ec;

        self.buffer.remove(start_char, end_char);

        // カーソルを選択開始位置に移動
        self.cursor_line = sl;
        self.cursor_col = sc;
        self.clear_selection();
    }
}

#[component]
pub fn VirtualEditorPanel(
    #[prop(into)] selected_file: Signal<Option<(String, String)>>,
) -> impl IntoView {
    let canvas_ref = NodeRef::<Canvas>::new();
    let container_ref = NodeRef::<leptos::html::Div>::new();

    // タブ管理（複数タブ対応）
    let tabs = RwSignal::new(Vec::<EditorTab>::new());
    let active_tab_index = RwSignal::new(Option::<usize>::None);

    // 再描画トリガー用
    let render_trigger = RwSignal::new(0u32);

    // IME状態管理
    let is_composing = RwSignal::new(false);
    let composing_text = RwSignal::new(String::new());

    // IME用の隠しinput要素
    let ime_input_ref = NodeRef::<leptos::html::Input>::new();

    // カーソルのピクセル位置（IME候補ウィンドウの位置制御用）
    let cursor_x = RwSignal::new(0.0);
    let cursor_y = RwSignal::new(0.0);

    // Copy/Paste用のクリップボード（簡易実装）
    let clipboard_text = RwSignal::new(String::new());

    // マウスドラッグ中かどうか
    let is_dragging = RwSignal::new(false);

    // ファイルが選択されたらタブを作成または切り替え
    Effect::new(move |_| {
        let current_file = selected_file.get();

        if let Some((path, content)) = current_file {
            tabs.update(|tabs_vec| {
                // 既存のタブを探す
                if let Some(existing_index) = tabs_vec.iter().position(|t| &t.file_path == &path) {
                    // 既存のタブをアクティブにする
                    active_tab_index.set(Some(existing_index));
                } else {
                    // 新しいタブを追加
                    tabs_vec.push(EditorTab::new(path.clone(), content.clone()));
                    active_tab_index.set(Some(tabs_vec.len() - 1));
                }
            });

            render_trigger.set(0);
        }
    });

    // 後方互換性：current_tabはMemoで計算される読み取り専用の値
    // 書き込みはヘルパー関数を使用
    let current_tab_memo = Signal::derive(move || {
        if let Some(index) = active_tab_index.get() {
            tabs.get().get(index).cloned()
        } else {
            None
        }
    });

    // current_tab.get() の代わり
    #[derive(Clone, Copy)]
    struct CurrentTab {
        tabs: RwSignal<Vec<EditorTab>>,
        active_index: RwSignal<Option<usize>>,
        memo: Signal<Option<EditorTab>>,
    }

    impl CurrentTab {
        fn get(&self) -> Option<EditorTab> {
            self.memo.get()
        }

        fn set(&self, new_tab: Option<EditorTab>) {
            if let Some(tab) = new_tab {
                if let Some(index) = self.active_index.get() {
                    let mut tabs_vec = self.tabs.get();
                    if index < tabs_vec.len() {
                        tabs_vec[index] = tab;
                        self.tabs.set(tabs_vec);
                    }
                }
            }
        }
    }

    let current_tab = CurrentTab {
        tabs,
        active_index: active_tab_index,
        memo: current_tab_memo,
    };

    // キーボードイベントハンドラー
    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        leptos::logging::log!("🎹 on_keydown called: key={}, keyCode={}, composing={}",
            ev.key(), ev.key_code(), ev.is_composing());

        // IME入力中は何もしない
        if ev.is_composing() || ev.key_code() == 229 {
            leptos::logging::log!("🇯🇵 IME composing detected, skipping");
            return;
        }

        ev.prevent_default(); // ブラウザのデフォルト動作を阻止

        let Some(mut tab) = current_tab.get() else {
            return;
        };

        let key = ev.key();
        let mut buffer_changed = false;

        // Ctrl/Cmd + Z (Undo)
        if (ev.ctrl_key() || ev.meta_key()) && key.as_str() == "z" {
            if tab.undo() {
                current_tab.set(Some(tab));
                render_trigger.update(|v| *v += 1);
                leptos::logging::log!("Undo executed");
            }
            return;
        }

        // Ctrl/Cmd + Y (Redo) または Ctrl/Cmd + Shift + Z
        if ((ev.ctrl_key() || ev.meta_key()) && key.as_str() == "y") ||
           ((ev.ctrl_key() || ev.meta_key()) && ev.shift_key() && key.as_str() == "Z") {
            if tab.redo() {
                current_tab.set(Some(tab));
                render_trigger.update(|v| *v += 1);
                leptos::logging::log!("Redo executed");
            }
            return;
        }

        // Ctrl/Cmd + S (Save)
        if (ev.ctrl_key() || ev.meta_key()) && key.as_str() == "s" {
            let file_path = tab.file_path.clone();
            let content = tab.buffer.to_string();

            // Tauri commandを使ってファイル保存
            wasm_bindgen_futures::spawn_local(async move {
                #[cfg(target_arch = "wasm32")]
                {
                    use wasm_bindgen::prelude::*;
                    #[wasm_bindgen]
                    extern "C" {
                        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
                        async fn invoke(cmd: &str, args: JsValue) -> JsValue;
                    }

                    let args = serde_wasm_bindgen::to_value(&serde_json::json!({
                        "path": file_path,
                        "contents": content,
                    })).unwrap();

                    match invoke("write_file", args).await {
                        _ => {
                            leptos::logging::log!("File saved: {}", file_path);
                        }
                    }
                }
            });

            current_tab.set(Some(tab));
            return;
        }

        // Ctrl/Cmd + A (Select All)
        if (ev.ctrl_key() || ev.meta_key()) && key.as_str() == "a" {
            tab.selection_start = Some((0, 0));
            let last_line = tab.buffer.len_lines().saturating_sub(1);
            let last_col = tab.buffer.line(last_line)
                .map(|s| s.trim_end_matches('\n').chars().count())
                .unwrap_or(0);
            tab.selection_end = Some((last_line, last_col));
            current_tab.set(Some(tab));
            render_trigger.update(|v| *v += 1);
            return;
        }

        // Ctrl/Cmd + C (Copy) - 選択範囲または行全体をコピー
        if (ev.ctrl_key() || ev.meta_key()) && key.as_str() == "c" {
            if let Some(selected_text) = tab.get_selected_text() {
                clipboard_text.set(selected_text.clone());
                leptos::logging::log!("Copied selection: {}", selected_text);
            } else if let Some(line_text) = tab.buffer.line(tab.cursor_line) {
                clipboard_text.set(line_text.to_string());
                leptos::logging::log!("Copied line: {}", line_text);
            }
            current_tab.set(Some(tab));
            return;
        }

        // Ctrl/Cmd + V (Paste) - カーソル位置または選択範囲に貼り付け
        if (ev.ctrl_key() || ev.meta_key()) && key.as_str() == "v" {
            let text_to_paste = clipboard_text.get();
            if !text_to_paste.is_empty() {
                tab.save_undo_state();

                // 選択範囲があれば先に削除
                if tab.has_selection() {
                    tab.delete_selection();
                }

                let char_idx = tab.buffer.line_to_char(tab.cursor_line) + tab.cursor_col;
                tab.buffer.insert(char_idx, &text_to_paste);

                // カーソルを貼り付けたテキストの末尾に移動
                let chars_inserted = text_to_paste.chars().count();
                tab.cursor_col += chars_inserted;
                buffer_changed = true;
                leptos::logging::log!("Pasted: {}", text_to_paste);
            }
            current_tab.set(Some(tab));
            render_trigger.update(|v| *v += 1);
            return;
        }

        // Ctrl/Cmd + X (Cut) - 選択範囲または行全体をカット
        if (ev.ctrl_key() || ev.meta_key()) && key.as_str() == "x" {
            tab.save_undo_state();

            if let Some(selected_text) = tab.get_selected_text() {
                clipboard_text.set(selected_text);
                tab.delete_selection();
                buffer_changed = true;
                leptos::logging::log!("Cut selection");
            } else if let Some(line_text) = tab.buffer.line(tab.cursor_line) {
                clipboard_text.set(line_text.to_string());
                let line_start = tab.buffer.line_to_char(tab.cursor_line);
                let line_end = line_start + line_text.len();
                tab.buffer.remove(line_start, line_end);
                tab.cursor_col = 0;
                buffer_changed = true;
                leptos::logging::log!("Cut line");
            }
            current_tab.set(Some(tab));
            render_trigger.update(|v| *v += 1);
            return;
        }

        match key.as_str() {
            // 英数字・記号の入力
            k if k.len() == 1 && !ev.ctrl_key() && !ev.meta_key() => {
                tab.save_undo_state();

                // 選択範囲があれば先に削除
                if tab.has_selection() {
                    tab.delete_selection();
                }

                let char_idx = tab.buffer.line_to_char(tab.cursor_line) + tab.cursor_col;
                tab.buffer.insert(char_idx, k);
                tab.cursor_col += 1;
                buffer_changed = true;
                leptos::logging::log!("Inserted: '{}' at line={}, col={}", k, tab.cursor_line, tab.cursor_col - 1);
            }

            // Backspace
            "Backspace" => {
                tab.save_undo_state();

                if tab.has_selection() {
                    tab.delete_selection();
                    buffer_changed = true;
                } else if tab.cursor_col > 0 {
                    // 同じ行内で削除
                    let char_idx = tab.buffer.line_to_char(tab.cursor_line) + tab.cursor_col - 1;
                    tab.buffer.remove(char_idx, char_idx + 1);
                    tab.cursor_col -= 1;
                    buffer_changed = true;
                } else if tab.cursor_line > 0 {
                    // 前の行と結合
                    let prev_line_len = tab.buffer.line(tab.cursor_line - 1)
                        .map(|s| s.trim_end_matches('\n').chars().count())
                        .unwrap_or(0);

                    let char_idx = tab.buffer.line_to_char(tab.cursor_line) - 1; // 改行文字
                    tab.buffer.remove(char_idx, char_idx + 1);
                    tab.cursor_line -= 1;
                    tab.cursor_col = prev_line_len;
                    buffer_changed = true;
                }
                leptos::logging::log!("Backspace: line={}, col={}", tab.cursor_line, tab.cursor_col);
            }

            // Delete
            "Delete" => {
                tab.save_undo_state();

                if tab.has_selection() {
                    tab.delete_selection();
                    buffer_changed = true;
                } else {
                    let line_len = tab.buffer.line(tab.cursor_line)
                        .map(|s| s.trim_end_matches('\n').chars().count())
                        .unwrap_or(0);

                    if tab.cursor_col < line_len {
                        // 同じ行内で削除
                        let char_idx = tab.buffer.line_to_char(tab.cursor_line) + tab.cursor_col;
                        tab.buffer.remove(char_idx, char_idx + 1);
                        buffer_changed = true;
                    } else if tab.cursor_line < tab.buffer.len_lines() - 1 {
                        // 次の行と結合
                        let char_idx = tab.buffer.line_to_char(tab.cursor_line) + tab.cursor_col;
                        tab.buffer.remove(char_idx, char_idx + 1);
                        buffer_changed = true;
                    }
                }
            }

            // Enter
            "Enter" => {
                tab.save_undo_state();

                // 選択範囲があれば先に削除
                if tab.has_selection() {
                    tab.delete_selection();
                }

                let char_idx = tab.buffer.line_to_char(tab.cursor_line) + tab.cursor_col;
                tab.buffer.insert(char_idx, "\n");
                tab.cursor_line += 1;
                tab.cursor_col = 0;
                buffer_changed = true;
                leptos::logging::log!("Enter: line={}, col={}", tab.cursor_line, tab.cursor_col);
            }

            // Home - 行頭に移動
            "Home" => {
                if ev.shift_key() {
                    // Shift+Home: 選択しながら行頭へ
                    if !tab.has_selection() {
                        tab.selection_start = Some((tab.cursor_line, tab.cursor_col));
                    }
                    tab.cursor_col = 0;
                    tab.selection_end = Some((tab.cursor_line, tab.cursor_col));
                } else {
                    tab.cursor_col = 0;
                    tab.clear_selection();
                }
            }

            // End - 行末に移動
            "End" => {
                let line_len = tab.buffer.line(tab.cursor_line)
                    .map(|s| s.trim_end_matches('\n').chars().count())
                    .unwrap_or(0);

                if ev.shift_key() {
                    // Shift+End: 選択しながら行末へ
                    if !tab.has_selection() {
                        tab.selection_start = Some((tab.cursor_line, tab.cursor_col));
                    }
                    tab.cursor_col = line_len;
                    tab.selection_end = Some((tab.cursor_line, tab.cursor_col));
                } else {
                    tab.cursor_col = line_len;
                    tab.clear_selection();
                }
            }

            // PageUp - 1ページ上へ
            "PageUp" => {
                let page_lines = 20; // 1ページ = 20行
                tab.cursor_line = tab.cursor_line.saturating_sub(page_lines);
                let line_len = tab.buffer.line(tab.cursor_line)
                    .map(|s| s.trim_end_matches('\n').chars().count())
                    .unwrap_or(0);
                tab.cursor_col = tab.cursor_col.min(line_len);
                if !ev.shift_key() {
                    tab.clear_selection();
                }
            }

            // PageDown - 1ページ下へ
            "PageDown" => {
                let page_lines = 20;
                tab.cursor_line = (tab.cursor_line + page_lines).min(tab.buffer.len_lines().saturating_sub(1));
                let line_len = tab.buffer.line(tab.cursor_line)
                    .map(|s| s.trim_end_matches('\n').chars().count())
                    .unwrap_or(0);
                tab.cursor_col = tab.cursor_col.min(line_len);
                if !ev.shift_key() {
                    tab.clear_selection();
                }
            }

            // 矢印キー - カーソル移動（Shiftキーで選択）
            "ArrowLeft" => {
                if ev.shift_key() {
                    // Shift+Left: 選択しながら左へ
                    if !tab.has_selection() {
                        tab.selection_start = Some((tab.cursor_line, tab.cursor_col));
                    }
                    if tab.cursor_col > 0 {
                        tab.cursor_col -= 1;
                    } else if tab.cursor_line > 0 {
                        tab.cursor_line -= 1;
                        tab.cursor_col = tab.buffer.line(tab.cursor_line)
                            .map(|s| s.trim_end_matches('\n').chars().count())
                            .unwrap_or(0);
                    }
                    tab.selection_end = Some((tab.cursor_line, tab.cursor_col));
                } else {
                    tab.clear_selection();
                    if tab.cursor_col > 0 {
                        tab.cursor_col -= 1;
                    } else if tab.cursor_line > 0 {
                        tab.cursor_line -= 1;
                        tab.cursor_col = tab.buffer.line(tab.cursor_line)
                            .map(|s| s.trim_end_matches('\n').chars().count())
                            .unwrap_or(0);
                    }
                }
            }

            "ArrowRight" => {
                let line_len = tab.buffer.line(tab.cursor_line)
                    .map(|s| s.trim_end_matches('\n').chars().count())
                    .unwrap_or(0);

                if ev.shift_key() {
                    // Shift+Right: 選択しながら右へ
                    if !tab.has_selection() {
                        tab.selection_start = Some((tab.cursor_line, tab.cursor_col));
                    }
                    if tab.cursor_col < line_len {
                        tab.cursor_col += 1;
                    } else if tab.cursor_line < tab.buffer.len_lines() - 1 {
                        tab.cursor_line += 1;
                        tab.cursor_col = 0;
                    }
                    tab.selection_end = Some((tab.cursor_line, tab.cursor_col));
                } else {
                    tab.clear_selection();
                    if tab.cursor_col < line_len {
                        tab.cursor_col += 1;
                    } else if tab.cursor_line < tab.buffer.len_lines() - 1 {
                        tab.cursor_line += 1;
                        tab.cursor_col = 0;
                    }
                }
            }

            "ArrowUp" => {
                if ev.shift_key() {
                    // Shift+Up: 選択しながら上へ
                    if !tab.has_selection() {
                        tab.selection_start = Some((tab.cursor_line, tab.cursor_col));
                    }
                    if tab.cursor_line > 0 {
                        tab.cursor_line -= 1;
                        let line_len = tab.buffer.line(tab.cursor_line)
                            .map(|s| s.trim_end_matches('\n').chars().count())
                            .unwrap_or(0);
                        tab.cursor_col = tab.cursor_col.min(line_len);
                    }
                    tab.selection_end = Some((tab.cursor_line, tab.cursor_col));
                } else {
                    tab.clear_selection();
                    if tab.cursor_line > 0 {
                        tab.cursor_line -= 1;
                        let line_len = tab.buffer.line(tab.cursor_line)
                            .map(|s| s.trim_end_matches('\n').chars().count())
                            .unwrap_or(0);
                        tab.cursor_col = tab.cursor_col.min(line_len);
                    }
                }
            }

            "ArrowDown" => {
                if ev.shift_key() {
                    // Shift+Down: 選択しながら下へ
                    if !tab.has_selection() {
                        tab.selection_start = Some((tab.cursor_line, tab.cursor_col));
                    }
                    if tab.cursor_line < tab.buffer.len_lines() - 1 {
                        tab.cursor_line += 1;
                        let line_len = tab.buffer.line(tab.cursor_line)
                            .map(|s| s.trim_end_matches('\n').chars().count())
                            .unwrap_or(0);
                        tab.cursor_col = tab.cursor_col.min(line_len);
                    }
                    tab.selection_end = Some((tab.cursor_line, tab.cursor_col));
                } else {
                    tab.clear_selection();
                    if tab.cursor_line < tab.buffer.len_lines() - 1 {
                        tab.cursor_line += 1;
                        let line_len = tab.buffer.line(tab.cursor_line)
                            .map(|s| s.trim_end_matches('\n').chars().count())
                            .unwrap_or(0);
                        tab.cursor_col = tab.cursor_col.min(line_len);
                    }
                }
            }

            _ => {
                leptos::logging::log!("Unhandled key: {}", key);
            }
        }

        // タブを更新
        current_tab.set(Some(tab));

        // バッファが変更された場合、またはカーソルが移動した場合は再描画
        render_trigger.update(|v| *v += 1);
    };

    // IMEイベントハンドラー
    let on_composition_start = move |_ev: leptos::ev::CompositionEvent| {
        is_composing.set(true);
        leptos::logging::log!("IME composition started");
    };

    let on_composition_update = move |ev: leptos::ev::CompositionEvent| {
        if let Some(data) = ev.data() {
            composing_text.set(data);
            render_trigger.update(|v| *v += 1);
            leptos::logging::log!("IME composing: {}", composing_text.get());
        }
    };

    let on_composition_end = move |ev: leptos::ev::CompositionEvent| {
        is_composing.set(false);

        // ✅ FIX: ev.data()は空になることがあるため、IME inputの値を直接取得
        let data = if let Some(input) = ime_input_ref.get() {
            let value = input.value();
            leptos::logging::log!("🔍 compositionend: ev.data()={:?}, input.value()={}", ev.data(), value);
            value
        } else {
            ev.data().unwrap_or_default()
        };

        // 確定文字をバッファに挿入
        if !data.is_empty() {
            if let Some(mut tab) = current_tab.get() {
                let old_col = tab.cursor_col;
                let char_idx = tab.buffer.line_to_char(tab.cursor_line) + tab.cursor_col;
                tab.buffer.insert(char_idx, &data);
                tab.cursor_col += data.chars().count();
                leptos::logging::log!(
                    "✅ IME committed: '{}' at pos {}, cursor: {} -> {}",
                    data,
                    char_idx,
                    old_col,
                    tab.cursor_col
                );
                current_tab.set(Some(tab));
            }
        } else {
            leptos::logging::log!("⚠️ IME committed empty string, skipping");
        }

        composing_text.set(String::new());

        // IME inputの値をクリア（次の入力に備えて）
        if let Some(input) = ime_input_ref.get() {
            input.set_value("");
            let _ = input.focus();
        }

        render_trigger.update(|v| *v += 1);
    };


    // マウスクリックでカーソル配置（ドラッグ開始）
    let on_mousedown = move |ev: leptos::ev::MouseEvent| {
        let Some(canvas) = canvas_ref.get() else {
            return;
        };

        let Some(mut tab) = current_tab.get() else {
            return;
        };

        let rect = canvas.get_bounding_client_rect();
        let x = ev.client_x() as f64 - rect.left();
        let y = ev.client_y() as f64 - rect.top();

        // カーソル位置を計算
        if let Ok(renderer) = CanvasRenderer::new((*canvas).clone().unchecked_into()) {
            // ガター幅を超えているか確認
            if x > renderer.gutter_width() {
                let text_x = x - renderer.gutter_width() - 15.0;
                let clicked_line = ((y + tab.scroll_top) / LINE_HEIGHT).floor() as usize;

                // 行範囲内に制限
                let line = clicked_line.min(tab.buffer.len_lines().saturating_sub(1));

                // 列位置を計算（簡易版：ASCII文字幅で割る）
                let col = (text_x / renderer.char_width_ascii()).round() as usize;

                // 行の長さ内に制限
                let line_len = tab.buffer.line(line)
                    .map(|s| s.trim_end_matches('\n').chars().count())
                    .unwrap_or(0);

                tab.cursor_line = line;
                tab.cursor_col = col.min(line_len);

                // ドラッグ開始
                is_dragging.set(true);
                tab.selection_start = Some((line, col.min(line_len)));
                tab.selection_end = Some((line, col.min(line_len)));

                current_tab.set(Some(tab));
                render_trigger.update(|v| *v += 1);

                leptos::logging::log!("Mouse down: line={}, col={}", line, col);

                // ✅ レンダリング完了後にフォーカスを設定（requestAnimationFrameで次のフレーム）
                if let Some(input) = ime_input_ref.get() {
                    use wasm_bindgen::JsCast;
                    let input_clone = input.clone();
                    let callback = wasm_bindgen::closure::Closure::once(move || {
                        match input_clone.focus() {
                            Ok(_) => leptos::logging::log!("✅ IME input focused (after render)"),
                            Err(e) => leptos::logging::log!("❌ IME input focus failed: {:?}", e),
                        }
                    });

                    let window = web_sys::window().unwrap();
                    let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
                    callback.forget();
                } else {
                    leptos::logging::log!("❌ IME input ref not found");
                }
            }
        }
    };

    // マウス移動（ドラッグ中）
    let on_mousemove = move |ev: leptos::ev::MouseEvent| {
        if !is_dragging.get() {
            return;
        }

        let Some(canvas) = canvas_ref.get() else {
            return;
        };

        let Some(mut tab) = current_tab.get() else {
            return;
        };

        let rect = canvas.get_bounding_client_rect();
        let x = ev.client_x() as f64 - rect.left();
        let y = ev.client_y() as f64 - rect.top();

        if let Ok(renderer) = CanvasRenderer::new((*canvas).clone().unchecked_into()) {
            if x > renderer.gutter_width() {
                let text_x = x - renderer.gutter_width() - 15.0;
                let clicked_line = ((y + tab.scroll_top) / LINE_HEIGHT).floor() as usize;
                let line = clicked_line.min(tab.buffer.len_lines().saturating_sub(1));
                let col = (text_x / renderer.char_width_ascii()).round() as usize;
                let line_len = tab.buffer.line(line)
                    .map(|s| s.trim_end_matches('\n').chars().count())
                    .unwrap_or(0);

                tab.cursor_line = line;
                tab.cursor_col = col.min(line_len);
                tab.selection_end = Some((line, col.min(line_len)));

                current_tab.set(Some(tab));
                render_trigger.update(|v| *v += 1);
            }
        }
    };

    // マウスボタンを離す（ドラッグ終了）
    let on_mouseup = move |_ev: leptos::ev::MouseEvent| {
        is_dragging.set(false);

        // 選択範囲が1文字未満なら選択解除
        if let Some(tab) = current_tab.get() {
            if let (Some(start), Some(end)) = (tab.selection_start, tab.selection_end) {
                if start == end {
                    let mut tab = tab;
                    tab.clear_selection();
                    current_tab.set(Some(tab));
                    render_trigger.update(|v| *v += 1);
                }
            }
        }
    };

    // ホイールでスクロール
    let on_wheel = move |ev: leptos::ev::WheelEvent| {
        ev.prevent_default();

        let Some(mut tab) = current_tab.get() else {
            return;
        };

        // スクロール量（1行 = LINE_HEIGHT）
        let delta = ev.delta_y();
        let scroll_lines = (delta / LINE_HEIGHT).round();

        tab.scroll_top = (tab.scroll_top + scroll_lines * LINE_HEIGHT).max(0.0);

        // 最大スクロール位置を計算
        let max_scroll = (tab.buffer.len_lines() as f64 * LINE_HEIGHT).max(0.0);
        tab.scroll_top = tab.scroll_top.min(max_scroll);

        current_tab.set(Some(tab));
        render_trigger.update(|v| *v += 1);
    };

    // Canvasのリサイズとレンダリング
    Effect::new(move |_| {
        // render_triggerに依存して、変更時に再描画
        let _ = render_trigger.get();

        let Some(canvas) = canvas_ref.get() else {
            leptos::logging::log!("❌ Canvas ref not available");
            return;
        };

        // Canvas の親要素(.berry-editor-pane)のサイズを取得
        let Some(parent) = canvas.parent_element() else {
            leptos::logging::log!("❌ Canvas parent not available");
            return;
        };

        let rect = parent.get_bounding_client_rect();
        let mut width = rect.width();
        let mut height = rect.height();

        // ✅ 高さが0の場合は、フォールバックとして親要素から取得を試みる
        if height <= 0.0 {
            leptos::logging::log!("⚠️ Parent height is 0, trying grandparent...");

            if let Some(grandparent) = parent.parent_element() {
                let gp_rect = grandparent.get_bounding_client_rect();
                leptos::logging::log!(
                    "📏 Grandparent (.berry-editor-main): {}x{}, class: {}",
                    gp_rect.width(),
                    gp_rect.height(),
                    grandparent.class_name()
                );

                if gp_rect.height() > 0.0 {
                    height = gp_rect.height();
                    leptos::logging::log!("✅ Using grandparent height: {}", height);
                }
            }

            // それでも0なら、最低限の高さを確保
            if height <= 0.0 {
                height = 500.0; // フォールバック高さ
                leptos::logging::log!("⚠️ Using fallback height: {}", height);
            }
        }

        leptos::logging::log!(
            "📏 Final canvas size: {}x{}",
            width,
            height
        );

        // サイズチェック（フォールバック後もまだ無効なら）
        if width <= 0.0 || height <= 0.0 {
            leptos::logging::log!("❌ Invalid canvas size after fallback: {}x{}", width, height);
            return;
        }

        leptos::logging::log!(
            "✅ Canvas resize: parent(.berry-editor-pane)={}x{}, setting canvas to {}x{}",
            width,
            height,
            width as u32,
            height as u32
        );

        // ✅ 物理ピクセルサイズを設定（これがないと描画されない）
        canvas.set_width(width as u32);
        canvas.set_height(height as u32);

        // レンダリング
        let tab_data = current_tab.get();
        if tab_data.is_none() {
            leptos::logging::log!("⚠️ No tab data available for rendering");
            return;
        }

        if let Some(tab) = tab_data {
            leptos::logging::log!(
                "🎨 Rendering tab: {} lines, cursor at ({}, {})",
                tab.buffer.len_lines(),
                tab.cursor_line,
                tab.cursor_col
            );
            let canvas_el: HtmlCanvasElement = (*canvas).clone().unchecked_into();

            if let Ok(renderer) = CanvasRenderer::new(canvas_el) {
                // Canvas全体をクリア
                renderer.clear(width as f64, height as f64);

                // 可視範囲の行を計算
                let start_line = (tab.scroll_top / LINE_HEIGHT).floor() as usize;
                let visible_lines = (height as f64 / LINE_HEIGHT).ceil() as usize + 1;
                let end_line = (start_line + visible_lines).min(tab.buffer.len_lines());

                // 行番号ガターを描画
                renderer.draw_gutter(start_line, end_line, height as f64);

                // 選択範囲を描画（テキストの背景として）
                if tab.has_selection() {
                    if let (Some((start_line, start_col)), Some((end_line, end_col))) =
                        (tab.selection_start, tab.selection_end) {
                        renderer.draw_selection(
                            start_line,
                            start_col,
                            end_line,
                            end_col,
                            tab.scroll_top,
                        );
                    }
                }

                // テキスト行を描画
                for line_num in start_line..end_line {
                    // Ropeから行のテキストを取得（改行を除く）
                    let line_text = tab
                        .buffer
                        .line(line_num)
                        .map(|s| s.trim_end_matches('\n').to_string())
                        .unwrap_or_default();

                    let y_offset = (line_num - start_line) as f64 * LINE_HEIGHT;
                    renderer.draw_line(
                        line_num,
                        y_offset,
                        &line_text,
                        crate::core::canvas_renderer::COLOR_FOREGROUND,
                    );
                }

                // カーソルを描画（現在行のテキストを渡す）
                // ✅ FIX: 改行を除いたテキストを渡す（改行があると文字数計算がずれる）
                let cursor_line_text = tab.buffer.line(tab.cursor_line)
                    .map(|s| s.trim_end_matches('\n').to_string())
                    .unwrap_or_default();

                // IME未確定文字列を取得
                let composing = composing_text.get();

                // IME組成中は、仮想的なテキスト（確定文字+未確定文字）を作成してカーソル位置を計算
                let (virtual_line_text, cursor_col_display) = if !composing.is_empty() {
                    // 未確定文字列がある場合、カーソル位置に挿入した仮想テキストを作る
                    let before: String = cursor_line_text.chars().take(tab.cursor_col).collect();
                    let after: String = cursor_line_text.chars().skip(tab.cursor_col).collect();
                    let virtual_text = format!("{}{}{}", before, composing, after);
                    let virtual_col = tab.cursor_col + composing.chars().count();
                    (virtual_text, virtual_col)
                } else {
                    (cursor_line_text.clone(), tab.cursor_col)
                };

                leptos::logging::log!(
                    "🎯 Drawing cursor: line={}, col={} (display_col={}), composing='{}', line_text='{}' (len={})",
                    tab.cursor_line,
                    tab.cursor_col,
                    cursor_col_display,
                    &composing,
                    &cursor_line_text,
                    cursor_line_text.chars().count()
                );

                // カーソルを描画（composing中は未確定文字列の後ろに表示）
                renderer.draw_cursor(tab.cursor_line, cursor_col_display, tab.scroll_top, &virtual_line_text);

                // IME未確定文字列を描画（あれば）
                if !composing.is_empty() {
                    // 全角文字を考慮してカーソル位置までの実際の幅を測定
                    let text_before_cursor: String = cursor_line_text
                        .chars()
                        .take(tab.cursor_col)
                        .collect();
                    let x = renderer.gutter_width() + 15.0
                        + renderer.measure_text(&text_before_cursor);
                    let y = tab.cursor_line as f64 * LINE_HEIGHT - tab.scroll_top + 15.0;

                    // 未確定文字列をカーソル位置から描画（灰色）
                    renderer.draw_text_at(x, y, &composing, "#808080");

                    leptos::logging::log!("Drew composing text '{}' at ({}, {})", composing, x, y);
                }

                // カーソル位置を計算（IME用）- 全角文字対応
                // composing中は未確定文字列の後ろに配置
                let text_before_cursor_display: String = virtual_line_text
                    .chars()
                    .take(cursor_col_display)
                    .collect();
                let cursor_pixel_x = renderer.gutter_width() + 15.0
                    + renderer.measure_text(&text_before_cursor_display);
                let cursor_pixel_y = tab.cursor_line as f64 * LINE_HEIGHT - tab.scroll_top;

                cursor_x.set(cursor_pixel_x);
                cursor_y.set(cursor_pixel_y);

                leptos::logging::log!(
                    "Rendered {} lines ({}..{}), cursor at ({}, {})",
                    end_line - start_line,
                    start_line,
                    end_line,
                    cursor_pixel_x,
                    cursor_pixel_y
                );
            }
        }
    });

    view! {
        <div
            node_ref=container_ref
            class="berry-editor-main"
            style="display: flex; flex-direction: column; flex: 1; min-width: 0; min-height: 0;"
        >
            // タブバー
            <div class="berry-editor-tabs" style="display: flex; background: #2B2B2B; border-bottom: 1px solid #323232; min-height: 35px;">
                {move || {
                    let tabs_vec = current_tab.tabs.get();
                    let active_index = current_tab.active_index.get();

                    if tabs_vec.is_empty() {
                        view! {
                            <div style="padding: 8px 16px; color: #606366; font-size: 13px;">
                                "No file open"
                            </div>
                        }.into_any()
                    } else {
                        // 全てのタブを表示
                        tabs_vec.into_iter().enumerate().map(|(index, tab)| {
                            let is_active = Some(index) == active_index;
                            let file_name = tab.file_path
                                .split('/')
                                .last()
                                .unwrap_or(&tab.file_path)
                                .to_string();

                            let tab_class = if is_active { "berry-tab active" } else { "berry-tab" };
                            let bg_color = if is_active { "#1E1E1E" } else { "#2B2B2B" };

                            // file_pathをクローンしてクロージャーで使う（indexは古くなる可能性があるため）
                            let tab_path = tab.file_path.clone();
                            let tab_path_for_close = tab_path.clone();

                            view! {
                                <div
                                    class=tab_class
                                    on:click=move |_| {
                                        // クリック時に最新のindexを検索
                                        let tabs_vec = current_tab.tabs.get();
                                        if let Some(idx) = tabs_vec.iter().position(|t| t.file_path == tab_path) {
                                            current_tab.active_index.set(Some(idx));
                                        }
                                    }
                                    style=format!("
                                        display: flex;
                                        align-items: center;
                                        padding: 8px 12px 8px 16px;
                                        background: {};
                                        border-right: 1px solid #323232;
                                        color: #A9B7C6;
                                        font-size: 13px;
                                        font-family: 'JetBrains Mono', monospace;
                                        gap: 8px;
                                        cursor: pointer;
                                    ", bg_color)
                                >
                                    <span>{file_name}</span>
                                    <button
                                        on:click=move |ev| {
                                            ev.stop_propagation();
                                            // タブを閉じる（file_pathで検索して削除）
                                            let mut tabs_vec = current_tab.tabs.get();
                                            if let Some(close_index) = tabs_vec.iter().position(|t| t.file_path == tab_path_for_close) {
                                                tabs_vec.remove(close_index);
                                                current_tab.tabs.set(tabs_vec.clone());

                                                // アクティブタブのインデックスを調整
                                                if tabs_vec.is_empty() {
                                                    // 全てのタブが閉じられた場合
                                                    current_tab.active_index.set(None);
                                                } else if Some(close_index) == current_tab.active_index.get() {
                                                    // 閉じたタブがアクティブだった場合、前のタブか次のタブをアクティブにする
                                                    let new_index = if close_index > 0 {
                                                        close_index - 1 // 前のタブ
                                                    } else {
                                                        0 // 最初のタブが閉じられた場合は新しい最初のタブ
                                                    };
                                                    // tabs_vec.len() は少なくとも 1 なので、安全に -1 できる
                                                    current_tab.active_index.set(Some(new_index.min(tabs_vec.len() - 1)));
                                                } else if let Some(active_idx) = current_tab.active_index.get() {
                                                    // 閉じたタブがアクティブタブより前にあった場合、インデックスを調整
                                                    if close_index < active_idx {
                                                        current_tab.active_index.set(Some(active_idx - 1));
                                                    }
                                                    // 閉じたタブがアクティブタブより後ろにある場合は調整不要
                                                }
                                            }
                                        }
                                        style="
                                            background: transparent;
                                            border: none;
                                            color: #606366;
                                            cursor: pointer;
                                            padding: 2px 4px;
                                            font-size: 16px;
                                            line-height: 1;
                                            display: flex;
                                            align-items: center;
                                            justify-content: center;
                                            border-radius: 2px;
                                        "
                                        onmouseover="this.style.background='#4E5157'; this.style.color='#A9B7C6';"
                                        onmouseout="this.style.background='transparent'; this.style.color='#606366';"
                                    >
                                        "×"
                                    </button>
                                </div>
                            }
                        }).collect_view().into_any()
                    }
                }}
            </div>

            <div class="berry-editor-pane" style="flex: 1; min-height: 0; display: flex; background: #2B2B2B;">
                <canvas
                    node_ref=canvas_ref
                    on:mousedown=on_mousedown
                    on:mousemove=on_mousemove
                    on:mouseup=on_mouseup
                    on:wheel=on_wheel
                    style="width: 100%; height: 100%; display: block;"
                />

                // 隠しinput要素（IME候補ウィンドウの位置制御用）
                <input
                    node_ref=ime_input_ref
                    type="text"
                    on:compositionstart=on_composition_start
                    on:compositionupdate=on_composition_update
                    on:compositionend=on_composition_end
                    on:keydown=on_keydown
                    on:focus=move |_| {
                        leptos::logging::log!("✅ IME input FOCUSED");
                    }
                    on:blur=move |ev: leptos::ev::FocusEvent| {
                        leptos::logging::log!("❌ IME input BLURRED, re-focusing...");
                        // 即座に再フォーカス（ただしIME composing中は除く）
                        if !is_composing.get() {
                            if let Some(input) = ime_input_ref.get() {
                                // Use requestAnimationFrame to avoid immediate blur loop
                                use wasm_bindgen::JsCast;
                                let input_clone = input.clone();
                                let callback = wasm_bindgen::closure::Closure::once(move || {
                                    let _ = input_clone.focus();
                                    leptos::logging::log!("🔄 Re-focused IME input after blur");
                                });
                                let window = web_sys::window().unwrap();
                                let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
                                callback.forget();
                            }
                        }
                    }
                    style=move || format!(
                        "position: absolute; \
                         left: {}px; \
                         top: {}px; \
                         width: 2px; \
                         height: {}px; \
                         opacity: 0; \
                         z-index: 999; \
                         color: transparent; \
                         background: transparent; \
                         border: none; \
                         outline: none; \
                         padding: 0; \
                         margin: 0; \
                         caret-color: transparent;",
                        cursor_x.get(),
                        cursor_y.get(),
                        LINE_HEIGHT
                    )
                />
            </div>
        </div>
    }
}
