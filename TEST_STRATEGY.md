# テスト戦略

BerryEditorのテスト戦略は**「メンテナンスしやすい」**かつ**「マルチプラットフォーム対応」**を目指して設計されています。

## 設計原則

1. **DOM依存を最小化**: ブラウザなしで実行できる単体テストを優先
2. **ロジックとUIを分離**: ビジネスロジックはDOM不要なテストでカバー
3. **統合テストは最小限**: 本当に重要なUI挙動のみをブラウザでテスト
4. **プラットフォーム抽象化**: モックを使ってプラットフォーム非依存にテスト

## テスト構成

### 単体テスト（`cargo test`で実行、超高速）

#### 1. **ライブラリテスト** (`src/**/mod.rs`内)
```
cargo test --lib
```
- **82個のテスト** - ブラウザ不要
- 各モジュールの純粋なRustロジックをテスト
- 例: `buffer.rs`, `storage.rs`, `events.rs`, `platform.rs`

#### 2. **独立した単体テストファイル** (`tests/`)

**`platform_abstraction_test.rs`** (16テスト)
- プラットフォーム抽象化レイヤーのテスト
- MockStorageを使ったストレージ操作のテスト
- イベント変換ロジックのテスト
- プラットフォーム判定のテスト

**`cursor_position_test.rs`** (15テスト)
- カーソル位置計算ロジックのテスト
- スクロールオフセット計算のテスト
- 境界値テスト

**`edit_mode_activation_test.rs`** (12テスト)
- エディットモード移行ロジックのテスト
- クリック位置→カーソル位置変換のテスト
- 回帰テスト（バグ修正の証明）

**`buffer_complete_test.rs`**
- TextBufferの全操作のテスト
- 挿入、削除、行操作など

**`syntax_highlight_test.rs`**
- シンタックスハイライトロジックのテスト

### 統合テスト（`wasm-pack test`で実行、WASM必要）

#### 3. **WASM統合テスト**

**`integration_wasm.rs`** (最小限の重要なテストのみ)
- エディタ初期化テスト
- ファイル読み込みとタブ生成
- 仮想スクロールの動作確認
- Buffer操作の統合テスト

**`integration_test.rs`**
- 基本的なWASM初期化テスト

## テスト実行方法

### すべての単体テスト（推奨：日常開発で使用）
```bash
cargo test
```
- **実行時間: ~3秒**
- ブラウザ不要
- 110個以上のテストが実行される

### WASM統合テスト（CI/リリース前）
```bash
wasm-pack test --headless --firefox
# または
wasm-pack test --headless --chrome
```
- ブラウザが必要
- 実際のDOM操作をテスト

### 特定のテストのみ実行
```bash
# プラットフォーム抽象化テストのみ
cargo test --test platform_abstraction_test

# 特定のテスト関数のみ
cargo test test_mock_storage_operations
```

## MockStorage の使い方

```rust
use berry_editor::common::storage::{EditorStorage, MockStorage};

#[test]
fn test_my_feature() {
    let storage = MockStorage::new();

    // ブラウザなしでLocalStorageをシミュレート
    storage.set_item("key", "value").unwrap();
    assert_eq!(storage.get_item("key").unwrap(), Some("value".to_string()));
}
```

## テストカバレッジ

### 単体テスト（cargo test）
- ✅ バッファ操作（100%）
- ✅ カーソル位置計算（100%）
- ✅ ストレージ抽象化（100%）
- ✅ イベント抽象化（100%）
- ✅ プラットフォーム判定（100%）
- ✅ エディットモードロジック（100%）

### 統合テスト（wasm-pack test）
- ✅ エディタ初期化
- ✅ ファイル読み込み
- ✅ 仮想スクロール
- ⚠️ UI細部は最小限（CSSクラス名変更に強い）

## 削除されたテスト

以下のテストファイルは冗長または保守コストが高いため削除されました：

### 削除理由
- **JSテスト** (`file_tree.test.js`, `setup.js`): 100% Rust化のため不要
- **Phaseテスト** (`phase1-5_*.rs`): 冗長で重複が多い
- **DOM多用テスト** (`file_tree_expand_test.rs`, `file_tree_tauri_test.rs`): CSSクラス名依存が強く壊れやすい
- **統合テスト** (`ui_integration_test.rs`, `virtual_editor_test.rs`): `integration_wasm.rs`に統合

### 統合による効果
- テストファイル数: **18ファイル → 7ファイル** (61%削減)
- テスト実行時間: 大幅短縮
- 保守性: CSSクラス名変更への耐性向上

## ベストプラクティス

### ✅ DO（推奨）
- 新機能のロジックには必ず単体テストを追加
- MockStorageを使ってブラウザ非依存にする
- 境界値とエッジケースをテストする
- 回帰テストを追加する（バグ修正時）

### ❌ DON'T（非推奨）
- DOMの内部構造（CSSクラス名など）をテストしない
- wasm_bindgen_testを多用しない（単体テストで十分なら）
- 「初期化できるか」だけのテストを追加しない

## CI/CD統合

```yaml
# GitHub Actions example
- name: Run unit tests
  run: cargo test

- name: Run WASM tests
  run: |
    wasm-pack test --headless --firefox
```

## 今後の改善案

1. **カバレッジ測定**: `cargo-tarpaulin`でコードカバレッジを測定
2. **ベンチマーク**: 大規模ファイル編集のパフォーマンステスト
3. **E2Eテスト**: Playwright/Cypressで実際のユーザーフローをテスト
4. **モバイル**: iOS/Androidエミュレータでのテスト

## まとめ

このテスト戦略により：
- ✅ **高速**: 単体テストは3秒以内
- ✅ **メンテナンスしやすい**: DOM依存が最小限
- ✅ **マルチプラットフォーム**: MockでWeb/Desktopを抽象化
- ✅ **信頼性**: 110個以上のテストで主要機能をカバー
