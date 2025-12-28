# レンダリング不具合の根本原因分析

## 症状

- ファイルを開いても画面に何も表示されない
- コンソールログには成功メッセージ（`RENDER SUCCESS: line_count=413`）
- タブは作成されている（`Tab added! new_index: 0, total tabs: 1`）
- `berry_invoke` は成功している
- デバッグビューは正しく表示される

## 根本原因

### 1. リアクティブ依存関係の断絶

**問題のあったコード**:
```rust
view! {
    <div class="berry-editor-main">
        // ... existing code ...

        // ❌ 問題: これらの変数は一度だけ計算され、更新されない
        let total_lines = line_count.max(1);
        let total_height = total_lines as f64 * line_height;
        let (start_line, end_line) = scroll_calculator.visible_range();

        // ❌ この時点で total_height が 0 または初期値で固定される
        <div style=format!("height: {}px;", total_height)>
            // スクロールコンテナの高さが 0 → 何も表示されない
        </div>
    </div>
}
```

**問題点**:
- `view!` マクロの外側で計算された変数は「静的な値」として扱われる
- ファイル読み込み時（`tabs.update()`）に line_count が変わっても、total_height は再計算されない
- 初期状態（line_count=0）の値が固定され、スクロールコンテナの高さが 0 になる

### 2. スクロール範囲計算の問題

**問題のあったコード**:
```rust
// ❌ scroll_calculator は一度だけ計算される
let mut scroll_calculator = tab_scroll.clone();
scroll_calculator.set_scroll_top(current_scroll);
let (start_line, end_line) = scroll_calculator.visible_range();

// ❌ start_line, end_line は固定値になる
{(start_line..end_line).map(|line_idx| {
    // 行のレンダリング
})}
```

**問題点**:
- スクロール位置（`scroll_top`）が変わっても、`start_line` と `end_line` は再計算されない
- ユーザーがスクロールしても表示される行が変わらない

### 3. 行番号エリアの欠如

**問題**:
- CSS には `.berry-editor-line-numbers` の定義があった
- しかし、実際の DOM 要素を生成するコードが存在しなかった
- テキストを `left: 55px` から始めると、空白が生じた

## 修正内容

### 1. リアクティブなクロージャの使用

**修正後のコード**:
```rust
view! {
    <div class="berry-editor-scroll-content" style=format!("height: {}px; ...", total_height)>

        // ✅ 行番号ガター: リアクティブなクロージャで囲む
        <div class="berry-editor-gutter">
            {move || {
                // ✅ scroll_top.get() で依存関係を確立
                let current_scroll = scroll_top.get();
                let start_line = (current_scroll / 20.0).floor() as usize;
                let end_line = (start_line + 50).min(line_count_val);

                // ✅ スクロールの度に再計算される
                view! {
                    <div style=format!("top: {}px;", start_line * 20.0)>
                        {(start_line..end_line).map(|n| {
                            view! { <div>{n + 1}</div> }
                        }).collect::<Vec<_>>()}
                    </div>
                }
            }}
        </div>

        // ✅ テキスト表示エリア: 同様にリアクティブに
        <div class="berry-editor-lines-container">
            {move || {
                let current_scroll = scroll_top.get();
                let start_line = (current_scroll / 20.0).floor() as usize;
                let end_line = (start_line + 50).min(line_count_val);

                // ✅ tabs.with() で依存関係を確立
                tabs.with(|t| {
                    let tab = &t[idx];
                    // ✅ ファイルが読み込まれる度に再レンダリング
                    view! {
                        {(start_line..end_line).map(|line_idx| {
                            // 行のレンダリング
                        }).collect::<Vec<_>>()}
                    }
                })
            }}
        </div>
    </div>
}
```

### 2. 主要な変更点

| 項目 | 修正前 | 修正後 |
|------|--------|--------|
| total_height | 一度だけ計算（固定値） | tabs.with() の外で計算、ファイル読み込み時に更新 |
| start_line, end_line | 一度だけ計算（固定値） | move \|\| クロージャ内で scroll_top.get() から計算 |
| 行番号ガター | 存在しない | sticky で実装、スクロールと同期 |
| リアクティブ性 | 断絶している | scroll_top.get() と tabs.with() で確立 |

## Leptos 0.7 のリアクティブシステムの注意点

### 依存関係の確立方法

1. **`.get()` を使う**: シグナルの変更を追跡する
   ```rust
   move || {
       let value = signal.get();  // ✅ 依存関係が確立される
       // value が変わると、このクロージャが再実行される
   }
   ```

2. **`.with()` を使う**: データへのアクセス時に依存関係を確立
   ```rust
   move || {
       tabs.with(|t| {  // ✅ tabs が更新されると再実行
           // t を使った処理
       })
   }
   ```

3. **`.get_untracked()` や `.with_untracked()`**: 依存関係を確立しない
   ```rust
   move || {
       let value = signal.get_untracked();  // ❌ 依存関係なし
       // signal が変わっても再実行されない
   }
   ```

### クロージャのスコープ

```rust
// ❌ 間違い: クロージャの外で計算
let total_height = line_count * 20.0;
view! {
    <div style=format!("height: {}px", total_height)>
        // line_count が変わっても total_height は変わらない
    </div>
}

// ✅ 正しい: クロージャの中で計算
view! {
    <div style=move || format!("height: {}px", line_count.get() * 20.0)>
        // line_count が変わると再計算される
    </div>
}
```

## まとめ

この問題は **Leptos のリアクティブシステムの理解不足** に起因していました：

1. **静的な値の固定化**: `view!` の外で計算された値は更新されない
2. **依存関係の欠如**: `.get()` や `.with()` を使わないと変更が追跡されない
3. **クロージャのスコープ**: リアクティブにしたい部分は `move ||` で囲む必要がある

修正後は、すべての動的な部分がリアクティブなクロージャ内で計算されるため、
ファイル読み込みやスクロールに正しく反応するようになりました。
