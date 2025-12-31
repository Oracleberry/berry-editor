# BerryCode GUI Editor - 100% Canvas Architecture

**Last Updated**: 2025-12-31
**Architecture**: Pure Canvas Rendering with Rust Event Handling

## 🎯 Core Philosophy: Zero Browser Input Dependency

This editor is built on a **100% Canvas + 100% Rust** architecture. We **completely eliminate** browser native input mechanisms (`contenteditable`, `textarea`, `input` elements for text editing) and implement everything from scratch.

### Why Canvas?

1. **Pixel-Perfect Control**: We control every pixel, every character position, every rendering detail
2. **No Browser Bugs**: Eliminated all ContentEditable quirks, inconsistencies, and platform-specific behaviors
3. **Predictable**: What you code in Rust is exactly what renders on screen
4. **High Performance**: Direct Canvas API calls, no DOM manipulation overhead

## 🏗️ Architecture Overview

```
┌─────────────────────────────────────────────┐
│  User Interaction (Keyboard/Mouse/IME)     │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│  Rust Event Handlers (virtual_editor.rs)   │
│  • on:keydown → Text buffer manipulation   │
│  • on:mousedown/move/up → Selection        │
│  • on:wheel → Scrolling                    │
│  • on:composition* → IME handling          │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│  TextBuffer (Rope data structure)          │
│  • Efficient text storage & manipulation   │
│  • O(log n) insertion/deletion             │
│  • O(1) cloning for Undo/Redo              │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│  Canvas Renderer (canvas_renderer.rs)      │
│  • draw_gutter() → Line numbers            │
│  • draw_line() → Text rendering            │
│  • draw_cursor() → Cursor visualization    │
│  • draw_selection() → Selection highlight  │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│  HTML5 Canvas Element                      │
│  • <canvas> (visible, receives events)     │
│  • <input> (hidden, IME position anchor)   │
└─────────────────────────────────────────────┘
```

## 📁 File Structure

### Core Components

```
src/
├── core/
│   ├── canvas_renderer.rs    ✅ ONLY file allowed to use web-sys Canvas API
│   ├── virtual_editor.rs     ✅ Main editor logic (events, state, rendering)
│   ├── bridge.rs             ✅ Tauri bindings
│   └── mod.rs                ✅ Module exports
├── buffer.rs                 ✅ Rope-based text buffer
├── syntax.rs                 ✅ Syntax highlighting
└── lib.rs                    ✅ Library entry point
```

### Strict Rules

1. **canvas_renderer.rs**: ONLY file allowed to use `web_sys::CanvasRenderingContext2d`
2. **virtual_editor.rs**: Event handling, state management, no direct Canvas API calls
3. **NO contenteditable**: This concept is banned from the entire codebase
4. **NO textarea/input for editing**: Only a hidden `<input>` for IME positioning

## 🎨 Canvas Rendering Details

### Text Rendering

```rust
pub fn draw_line(&self, line_num: usize, y_offset: f64, text: &str, color: &str) {
    let x = self.gutter_width + 15.0;
    let y = y_offset + 15.0;
    self.context.set_fill_style(&color.into());
    let _ = self.context.fill_text(text, x, y);
}
```

### Cursor Rendering

```rust
pub fn draw_cursor(&self, line: usize, col: usize, scroll_top: f64) {
    let x = self.gutter_width + 15.0 + (col as f64 * self.char_width_ascii);
    let y = line as f64 * self.line_height - scroll_top;

    self.context.set_stroke_style(&COLOR_CURSOR.into());
    self.context.set_line_width(2.0);
    self.context.begin_path();
    self.context.move_to(x, y);
    self.context.line_to(x, y + self.line_height);
    self.context.stroke();
}
```

### Selection Rendering

```rust
pub fn draw_selection(&self, start_line: usize, start_col: usize,
                       end_line: usize, end_col: usize, scroll_top: f64) {
    self.context.set_fill_style(&COLOR_SELECTION.into());
    // Draw blue rectangle behind selected text
    // ...
}
```

## ⌨️ Event Handling

All events are handled in pure Rust in `virtual_editor.rs`:

### Keyboard Events

```rust
let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
    // IME check
    if ev.is_composing() || ev.key_code() == 229 { return; }

    ev.prevent_default(); // Block ALL browser defaults

    let key = ev.key();

    match key.as_str() {
        // Single character input
        k if k.len() == 1 && !ev.ctrl_key() && !ev.meta_key() => {
            tab.save_undo_state();
            let char_idx = tab.buffer.line_to_char(line) + col;
            tab.buffer.insert(char_idx, k);
            // ...
        }

        // Ctrl+Z (Undo)
        "z" if ev.ctrl_key() || ev.meta_key() => {
            tab.undo();
        }

        // Arrow keys with Shift for selection
        "ArrowLeft" if ev.shift_key() => {
            if !tab.has_selection() {
                tab.selection_start = Some((line, col));
            }
            // Move cursor...
            tab.selection_end = Some((new_line, new_col));
        }

        _ => {}
    }

    render_trigger.update(|v| *v += 1); // Trigger redraw
};
```

### Mouse Events

```rust
let on_mousedown = move |ev: leptos::ev::MouseEvent| {
    let rect = canvas.get_bounding_client_rect();
    let x = ev.client_x() as f64 - rect.left();
    let y = ev.client_y() as f64 - rect.top();

    // Calculate cursor position from pixel coordinates
    let line = ((y + scroll_top) / LINE_HEIGHT) as usize;
    let col = ((x - gutter_width - 15.0) / char_width) as usize;

    // Start text selection
    is_dragging.set(true);
    tab.selection_start = Some((line, col));
};

let on_mousemove = move |ev: leptos::ev::MouseEvent| {
    if is_dragging.get() {
        // Update selection_end...
    }
};

let on_mouseup = move |_ev: leptos::ev::MouseEvent| {
    is_dragging.set(false);
};
```

### IME Composition Events

```rust
let on_composition_start = move |_ev: leptos::ev::CompositionEvent| {
    is_composing.set(true);
};

let on_composition_update = move |ev: leptos::ev::CompositionEvent| {
    if let Some(data) = ev.data() {
        composing_text.set(data);
        render_trigger.update(|v| *v += 1); // Show gray uncommitted text
    }
};

let on_composition_end = move |ev: leptos::ev::CompositionEvent| {
    is_composing.set(false);
    if let Some(data) = ev.data() {
        // Insert confirmed text
        tab.buffer.insert(char_idx, &data);
    }
    composing_text.set(String::new());
};
```

## 🇯🇵 IME Support Strategy

Since Canvas cannot display native IME candidate windows, we use a **hidden `<input>` element**:

```rust
view! {
    <canvas
        on:keydown=on_keydown
        on:mousedown=on_mousedown
        on:mousemove=on_mousemove
        on:mouseup=on_mouseup
        on:wheel=on_wheel
    />

    // Hidden input for IME positioning
    <input
        type="text"
        on:compositionstart=on_composition_start
        on:compositionupdate=on_composition_update
        on:compositionend=on_composition_end
        on:keydown=on_keydown
        style=move || format!(
            "position: absolute; \
             left: {}px; \
             top: {}px; \
             opacity: 0; \
             pointer-events: none; \
             z-index: 1;",
            cursor_x.get(),
            cursor_y.get()
        )
    />
}
```

**How it works:**
1. User clicks canvas → Hidden input gets focus
2. User types Japanese → `compositionupdate` fires
3. We render uncommitted text in gray on canvas
4. User confirms → `compositionend` fires, we insert into buffer
5. Hidden input follows cursor position → IME window appears at cursor

## 🔄 State Management

### EditorTab Structure

```rust
struct EditorTab {
    file_path: String,
    buffer: TextBuffer,              // Rope-based text storage
    cursor_line: usize,
    cursor_col: usize,
    scroll_top: f64,
    selection_start: Option<(usize, usize)>,
    selection_end: Option<(usize, usize)>,
    undo_stack: Vec<EditorSnapshot>,  // Max 100 snapshots
    redo_stack: Vec<EditorSnapshot>,
    syntax_highlighter: SyntaxHighlighter,
}
```

### Undo/Redo

```rust
fn save_undo_state(&mut self) {
    let snapshot = EditorSnapshot {
        buffer: self.buffer.clone(), // O(1) thanks to Rope
        cursor_line: self.cursor_line,
        cursor_col: self.cursor_col,
    };
    self.undo_stack.push(snapshot);
    if self.undo_stack.len() > 100 {
        self.undo_stack.remove(0);
    }
    self.redo_stack.clear();
}

fn undo(&mut self) -> bool {
    if let Some(snapshot) = self.undo_stack.pop() {
        let redo = EditorSnapshot { /* current state */ };
        self.redo_stack.push(redo);

        self.buffer = snapshot.buffer;
        self.cursor_line = snapshot.cursor_line;
        self.cursor_col = snapshot.cursor_col;
        true
    } else {
        false
    }
}
```

## ⚡ Performance Optimizations

1. **Rope Data Structure**: O(log n) edits, O(1) cloning for undo
2. **Viewport Culling**: Only render visible lines
3. **Incremental Rendering**: Only redraw when `render_trigger` changes
4. **Canvas Reuse**: Single canvas element, no DOM manipulation

## 🎯 Implemented Features

### ✅ Core Editing
- [x] Text input (ASCII + Unicode)
- [x] Japanese/Chinese IME support
- [x] Backspace/Delete with line merging
- [x] Enter key (newline insertion)
- [x] Arrow keys navigation
- [x] Mouse click positioning
- [x] Mouse drag selection

### ✅ Advanced Editing
- [x] Undo/Redo (Ctrl+Z / Ctrl+Y)
- [x] Copy/Paste/Cut (Ctrl+C/V/X)
- [x] Select All (Ctrl+A)
- [x] Delete key
- [x] Selection operations (delete/replace on type)

### ✅ Navigation
- [x] Home/End (line start/end)
- [x] PageUp/PageDown (20-line scroll)
- [x] Mouse wheel scrolling
- [x] Shift+Arrow keys (selection while moving)
- [x] Shift+Home/End (select to line start/end)

### ✅ File Operations
- [x] File save (Ctrl+S via Tauri)
- [x] File loading
- [x] Multiple file tabs (foundation ready)

### ✅ Visual Features
- [x] Line numbers gutter
- [x] Cursor rendering
- [x] Selection highlighting
- [x] Scroll support
- [x] IntelliJ Darcula color scheme

### 🚧 Future Enhancements
- [ ] Syntax highlighting colors on canvas
- [ ] Multi-cursor editing
- [ ] Find & Replace
- [ ] Code folding
- [ ] Minimap

## 🚨 Banned Patterns

### ❌ NEVER DO THIS:

```rust
// ❌ Using contenteditable
<div contenteditable="true"></div>

// ❌ Using textarea/input for text editing
<textarea></textarea>

// ❌ Direct web-sys usage outside canvas_renderer.rs
use web_sys::CanvasRenderingContext2d; // Only in canvas_renderer.rs!

// ❌ DOM manipulation for text
element.set_inner_html(&text);

// ❌ Browser native selection API
window.get_selection();
```

### ✅ ALWAYS DO THIS:

```rust
// ✅ All text goes through TextBuffer
tab.buffer.insert(char_idx, text);

// ✅ All rendering through CanvasRenderer
renderer.draw_line(line_num, y_offset, &text, color);

// ✅ All events in Rust
on:keydown=on_keydown

// ✅ Selection in Rust state
tab.selection_start = Some((line, col));
```

## 🧪 Testing

### Unit Tests (79 tests)
```bash
cargo test --lib
```

Tests: Buffer operations, syntax parsing, file tree, etc.

### WASM Tests (132 tests)
```bash
wasm-pack test --headless --firefox
```

Tests: Canvas rendering, event handling, IME, etc.

### E2E Tests (planned)
```bash
cargo test --test e2e_cursor_positioning_test
```

Tests: Real browser behavior, keyboard input, mouse interaction.

## 📊 **Performance Benchmarks**

- Text insertion: < 1ms (Rope O(log n))
- Undo/Redo: < 1ms (Rope clone is O(1))
- Rendering 1000 lines: < 16ms (viewport culling)
- Selection update: < 1ms

## 🎓 Learning Resources

- **Rope Data Structure**: https://xi-editor.io/docs/rope_science_00.html
- **Canvas API**: https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API
- **Leptos Framework**: https://leptos.dev/
- **Text Editor Architecture**: https://code.visualstudio.com/blogs/2018/03/23/text-buffer-reimplementation

## 🏆 Success Metrics

A change is successful when:
- ✅ All 210 tests pass (79 unit + 132 WASM)
- ✅ Real keyboard typing works in Tauri app
- ✅ Japanese IME input works correctly
- ✅ Mouse operations work (click, drag, scroll)
- ✅ No contenteditable references anywhere
- ✅ All rendering through Canvas API only

## 📝 Migration History

**2025-12-31**: Complete migration from ContentEditable to Canvas
- Removed all `contenteditable` dependencies
- Implemented 100% Rust event handling
- Created `canvas_renderer.rs` as single rendering authority
- Achieved feature parity with ContentEditable version
- 210/211 tests passing

---

## ワークフロー・鉄の掟 (Test-Driven Cleanup)
コーディング完了を定義する3つのステップ：

1.  **影響範囲の検証 (E2E First)**:
    - 実装・修正が完了した後は、必ず **デスクトップアプリのE2Eテスト** (`./run_e2e_tests.sh`) を実行し、既存機能にデグレード（先祖返り）がないことを証明すること。
2.  **新機能の保証 (Add E2E)**:
    - 新しく追加した機能や修正したバグに対して、それを検証する E2E テストが存在しない場合は、**必ず新規に E2E テストを追加**すること。
3.  **負の遺産の掃除 (Delete Obsolete)**:
    - 設計変更（例: DOMからCanvasへの移行）によって**不要になったテストコードは、即座に、かつ完全に削除**すること。動かないテストを残すことは罪である。

## テストの優先順位
- **Canvas導通**: 文字入力、IME（日本語入力）、カーソル移動が Canvas 上で期待通り動作するか。
- **データ整合性**: `Rope` バッファと Canvas 描画座標が 1px の狂いもなく同期しているか。
- **リサイズ耐性**: ウィンドウリサイズ後に描画が崩れず、入力が継続できるか。

---

**Remember**: This is a **100% Canvas + 100% Rust** editor. There is NO contenteditable, NO textarea, NO browser native text input. We control everything from keyboard events to pixel rendering.
