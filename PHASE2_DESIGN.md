# Phase 2: IDEの骨格 - 設計ドキュメント

## 目標
> **VS CodeレベルのIDE基本機能を実装する**

Phase 1で高性能なエディター基盤が完成したので、Phase 2ではIDEとして必須の機能を追加します。

---

## Phase 2で実装する機能

### 1. 高度なタブ管理 📑

#### 1.1 タブを閉じる機能
- **要件**:
  - 各タブに×ボタンを追加
  - クリックでタブを閉じる
  - 未保存の変更がある場合は確認ダイアログ
  - 最後のタブを閉じた場合は空の状態に戻る

- **実装**:
  - `EditorTab`に`is_modified: bool`フィールド追加
  - `close_tab(index: usize)`関数の実装
  - 確認ダイアログコンポーネント

#### 1.2 未保存ファイルの表示
- **要件**:
  - 変更されたファイルのタブに●マークを表示
  - ファイル名の色を変える（オプション）

- **実装**:
  - テキスト変更の検出ロジック
  - タブのビジュアル更新

#### 1.3 タブのコンテキストメニュー
- **要件**:
  - 右クリックでメニュー表示
  - 「Close」「Close Others」「Close All」
  - 「Copy Path」「Reveal in File Tree」

- **実装**:
  - `ContextMenu`コンポーネント
  - メニューアクション処理

#### 1.4 キーボードショートカット
- **要件**:
  - `Cmd+W` / `Ctrl+W`: タブを閉じる
  - `Cmd+Tab` / `Ctrl+Tab`: タブ切り替え
  - `Cmd+Shift+T` / `Ctrl+Shift+T`: 閉じたタブを復元

---

### 2. ファイルツリーの完全化 🌳

#### 2.1 コンテキストメニュー
- **要件**:
  - 右クリックでメニュー表示
  - ファイル: 「Rename」「Delete」「Copy Path」
  - フォルダ: 「New File」「New Folder」「Rename」「Delete」

- **実装**:
  - `FileTreeContextMenu`コンポーネント
  - メニュー項目の条件付き表示

#### 2.2 新規ファイル/フォルダ作成
- **要件**:
  - コンテキストメニューから「New File」「New Folder」
  - インライン入力フィールドでファイル名入力
  - Enterで確定、Escでキャンセル
  - Tauriの`create_file`コマンド使用

- **実装**:
  - `CreateFileDialog`コンポーネント（インライン）
  - Tauri `create_file`の呼び出し
  - ファイルツリーのリフレッシュ

#### 2.3 削除機能
- **要件**:
  - 確認ダイアログ表示
  - ファイル/フォルダを削除
  - Tauriの`delete_file`コマンド使用

- **実装**:
  - 確認ダイアログコンポーネント
  - Tauri `delete_file`の呼び出し
  - ファイルツリーのリフレッシュ

#### 2.4 リネーム機能
- **要件**:
  - インライン編集でファイル名変更
  - Enterで確定、Escでキャンセル
  - Tauriの`rename_file`コマンド使用

- **実装**:
  - インライン編集モード
  - Tauri `rename_file`の呼び出し
  - ファイルツリーのリフレッシュ

---

### 3. プロジェクト全体検索 🔍

#### 3.1 検索パネル
- **要件**:
  - サイドバーに検索パネルを追加
  - 検索入力フィールド
  - オプション: 大文字小文字区別、正規表現、ファイルパターン

- **実装**:
  - `SearchPanel`コンポーネント
  - 検索オプションのUI

#### 3.2 ripgrep統合（バックエンド）
- **要件**:
  - Tauriコマンドで`ripgrep`を実行
  - 検索結果をJSON形式で返す
  - パフォーマンス: 10万行のプロジェクトで< 1秒

- **実装**:
  - `src-tauri/Cargo.toml`に`grep`クレート追加
  - `search_in_files`コマンドの実装
  - 検索結果の構造体定義

```rust
#[derive(Serialize, Deserialize)]
pub struct SearchResult {
    pub path: String,
    pub line_number: usize,
    pub column: usize,
    pub line_text: String,
    pub match_start: usize,
    pub match_end: usize,
}
```

#### 3.3 検索結果の表示
- **要件**:
  - 検索結果をファイル別にグループ化
  - クリックでファイルを開き、該当行にジャンプ
  - マッチした文字列をハイライト

- **実装**:
  - `SearchResultsList`コンポーネント
  - 結果アイテムのクリックハンドラ
  - エディターへのジャンプ機能

#### 3.4 検索のキーボードショートカット
- **要件**:
  - `Cmd+Shift+F` / `Ctrl+Shift+F`: 検索パネルを開く
  - `Escape`: 検索パネルを閉じる

---

## アーキテクチャの変更

### 1. 状態管理の拡張

**新しいシグナル**:
```rust
// タブ管理
let tabs = RwSignal::new(Vec::<EditorTab>::new());
let active_tab_index = RwSignal::new(0usize);
let closed_tabs = RwSignal::new(Vec::<EditorTab>::new()); // 復元用

// 検索
let search_query = RwSignal::new(String::new());
let search_results = RwSignal::new(Vec::<SearchResult>::new());
let is_search_panel_open = RwSignal::new(false);

// ファイルツリー
let context_menu_position = RwSignal::new(None::<(i32, i32)>);
let selected_tree_item = RwSignal::new(None::<String>);
```

### 2. コンポーネント構造

```
EditorAppTauri
├── Sidebar
│   ├── FileTreePanelTauri (既存)
│   └── SearchPanel (NEW)
├── MainArea
│   ├── TabBar (拡張)
│   │   ├── Tab (×ボタン追加)
│   │   └── TabContextMenu (NEW)
│   └── VirtualEditorPanel (既存)
└── ContextMenus
    ├── FileTreeContextMenu (NEW)
    └── ConfirmDialog (NEW)
```

### 3. Tauriコマンドの追加

**新規コマンド**:
1. `search_in_files(query, options)` - ripgrep検索
2. (既存のファイル操作コマンドを活用)

---

## 実装順序

### ステップ1: タブ管理機能（1日目）
1. ✅ タブに×ボタンを追加
2. ✅ `close_tab`関数の実装
3. ✅ 未保存マーク（●）の表示
4. ✅ タブのコンテキストメニュー
5. ✅ キーボードショートカット

### ステップ2: ファイルツリー拡張（2日目）
1. ✅ コンテキストメニューの表示
2. ✅ 新規ファイル/フォルダ作成ダイアログ
3. ✅ 削除機能（確認ダイアログ付き）
4. ✅ リネーム機能（インライン編集）

### ステップ3: 検索機能（3日目）
1. ✅ 検索パネルUI
2. ✅ ripgrep統合（バックエンド）
3. ✅ 検索結果の表示
4. ✅ 結果からファイルへジャンプ

### ステップ4: テストとドキュメント（4日目）
1. ✅ 全機能のテストコード作成
2. ✅ ドキュメント更新
3. ✅ Phase 2完了レポート作成

---

## UI/UXデザイン

### タブバー

```
┌─────────────────────────────────────────────────┐
│ main.rs ● × │ lib.rs × │ test.rs × │ + │      │
└─────────────────────────────────────────────────┘
  ↑ 未保存  ↑閉じる    ↑通常     ↑新規タブ
```

### ファイルツリー コンテキストメニュー

```
📁 src/           ← 右クリック
  📄 main.rs      ┌─────────────────┐
  📄 lib.rs       │ New File        │
                  │ New Folder      │
                  │ ───────────────  │
                  │ Rename          │
                  │ Delete          │
                  │ ───────────────  │
                  │ Copy Path       │
                  └─────────────────┘
```

### 検索パネル

```
┌─ SEARCH ────────────────────────────────────┐
│ 🔍 [________________________] [Search]      │
│                                              │
│ ☐ Match Case  ☐ Regex  ☐ Whole Word        │
│ Files to include: *.rs, *.toml              │
│                                              │
│ 📄 src/main.rs (3 results)                  │
│   12: fn main() {                            │
│   45: // main function                       │
│   67: println!("main");                      │
│                                              │
│ 📄 src/lib.rs (1 result)                    │
│   23: pub fn main_lib() {                    │
└──────────────────────────────────────────────┘
```

---

## パフォーマンス目標

| 操作 | 目標時間 |
|------|---------|
| タブを閉じる | < 50ms |
| ファイル作成 | < 100ms |
| ファイル削除 | < 100ms |
| ファイルリネーム | < 100ms |
| 検索（10万行） | < 1秒 |
| 検索結果表示 | < 50ms |

---

## セキュリティ考慮事項

1. **ファイル操作の確認**
   - 削除前に確認ダイアログ
   - 上書き前に確認

2. **パス検証**
   - ディレクトリトラバーサル防止
   - 絶対パスの検証

3. **検索のサンドボックス化**
   - プロジェクトディレクトリ内のみ検索
   - システムディレクトリへのアクセス制限

---

## テスト戦略

### ユニットテスト
- タブ管理: 15+ tests
- ファイル操作: 20+ tests
- 検索機能: 15+ tests

### 統合テスト
- タブとエディターの連携
- ファイルツリーとTauriコマンドの連携
- 検索とエディターのジャンプ機能

### E2Eテスト
- ユーザーフローの検証
- キーボードショートカット

**目標**: Phase 2でも100%カバレッジ達成

---

## Phase 2完了の定義

- ✅ タブ管理機能が完全に動作
- ✅ ファイルツリーで全操作が可能
- ✅ 検索機能が高速で正確
- ✅ 100+テストが全てパス
- ✅ ドキュメント完備
- ✅ パフォーマンス目標達成

---

**Phase 2開始日**: 2025-12-26
**Phase 2完了予定**: 2025-12-30
