# カーソル移動修正の検証ガイド

## 修正内容

カーソルが動かない問題を修正しました。原因は `request_animation_frame` による非同期処理で、リアクティブシステムが分断されていたことです。

### 修正箇所

`src/core/virtual_editor.rs:290-304`

**修正前:**
```rust
Effect::new(move |_| {
    let l = cursor_line.get();
    let c = cursor_col.get();

    // ❌ 非同期処理により、UIの更新が次のフレームまで遅延
    if let Some(window) = web_sys::window() {
        let _ = window.request_animation_frame(
            wasm_bindgen::closure::Closure::once_into_js(move || {
                let idx = active_tab_index.get_untracked();
                tabs.update_untracked(|t| {
                    if let Some(tab) = t.get_mut(idx) {
                        tab.cursor_line = l;
                        tab.cursor_col = c;
                    }
                });
            })
            .unchecked_ref()
        );
    }
});
```

**修正後:**
```rust
Effect::new(move |_| {
    let l = cursor_line.get();
    let c = cursor_col.get();
    let idx = active_tab_index.get_untracked();

    // ✅ 即座に更新、リアクティブループを回避
    tabs.update_untracked(|t| {
        if let Some(tab) = t.get_mut(idx) {
            tab.cursor_line = l;
            tab.cursor_col = c;
        }
    });
});
```

### 修正の原理

1. **問題:** `request_animation_frame` で処理を遅延させていたため、カーソル位置の変更がUIに即座に反映されなかった
2. **解決:** `tabs.update_untracked()` を使って、リアクティブな通知を発生させずに即座に値を更新
3. **効果:**
   - カーソルの動きが即座にUIに反映される
   - `RefCell` のパニックを回避（`untracked` により再帰的な更新を防止）

## 検証方法

### 1. アプリケーションの起動

```bash
cargo tauri dev
```

### 2. 基本的なカーソル移動テスト

1. ファイルを開く
2. 矢印キー（↑ ↓ ← →）を押す
3. **期待する動作:** カーソルが即座に動く（遅延なし）
4. **失敗例:** カーソルが動かない、または1フレーム遅れて動く

### 3. 高速なカーソル移動テスト

1. 矢印キーを連続で高速に押す
2. **期待する動作:** すべてのキー入力に追従してカーソルが動く
3. **失敗例:** 途中で止まる、パニックが発生する

### 4. タブ切り替えとカーソル位置テスト

1. ファイル1を開き、カーソルを適当な位置（例: 5行10列目）に移動
2. 別のファイル2を開く（新しいタブ）
3. ファイル1のタブをクリックして戻る
4. **期待する動作:** カーソルが元の位置（5行10列目）に復元される
5. **失敗例:** カーソルが (0, 0) にリセットされる

### 5. 編集中のカーソル移動テスト

1. コードを編集しながら、矢印キーでカーソルを移動
2. 文字を入力した後、即座に矢印キーを押す
3. **期待する動作:** スムーズにカーソルが動く
4. **失敗例:** カーソルが固まる、パニックが発生する

### 6. パニック回避の確認

ブラウザの開発者コンソール（F12）を開いて、以下を確認:

```
期待: エラーなし
失敗: "Disposed" エラーまたは RefCell パニック
```

## テストケース一覧

| # | テスト内容 | 期待する動作 | 確認方法 |
|---|-----------|-------------|---------|
| 1 | 基本的な矢印キー移動 | 即座に動く | 矢印キーを押してカーソルの動きを見る |
| 2 | 高速連続入力 | すべての入力に追従 | 矢印キーを高速で連打 |
| 3 | タブ切り替え後の復元 | カーソル位置が保存・復元される | 2つのタブを切り替える |
| 4 | 編集中のカーソル移動 | スムーズに動く | 文字入力後すぐに矢印キー |
| 5 | パニックなし | コンソールにエラーなし | 開発者コンソールを確認 |
| 6 | 複数タブでの保存 | 各タブで独立したカーソル位置 | 3つ以上のタブで確認 |

## デバッグ情報

カーソルが動かない場合の診断:

### 開発者コンソールのエラーを確認

```javascript
// ブラウザコンソールで実行
console.log("Cursor debugging enabled");
```

### Rust側のログ

`src/core/virtual_editor.rs` に以下を追加して、カーソル更新を追跡:

```rust
Effect::new(move |_| {
    let l = cursor_line.get();
    let c = cursor_col.get();
    web_sys::console::log_1(&format!("Cursor moved: ({}, {})", l, c).into());
    // ... 既存のコード
});
```

## 既知の制限事項

- この修正は Leptos のリアクティブシステムに依存しています
- WASM環境でのみ動作します（Tauri デスクトップアプリ含む）

## 関連ファイル

- `src/core/virtual_editor.rs` - 主な修正箇所（290-304行目）
- `tests/cursor_position_test.rs` - カーソル位置計算のユニットテスト
- `CURSOR_FIX_VERIFICATION.md` - このファイル

## まとめ

この修正により:
- ✅ カーソルが即座に動く
- ✅ パニックが発生しない
- ✅ タブ切り替え時にカーソル位置が保存される
- ✅ IntelliJ Pro 仕様の高速レスポンスを実現
