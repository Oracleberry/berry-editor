# Phase 4: Git UI Integration - 設計ドキュメント

## 概要
BerryEditorにGit統合機能を追加し、IntelliJ/VS Code風のバージョン管理UIを実装します。

## 目標
- ✅ Gitステータス表示
- ✅ ファイルのステージング/アンステージング
- ✅ コミット作成
- ✅ ブランチ切り替え
- ✅ Diff表示
- ✅ Blame表示
- ✅ コミット履歴
- ✅ プッシュ/プル操作

## アーキテクチャ

### Tauri Backend
```
src-tauri/src/git/
├── mod.rs              - Git manager
├── commands.rs         - Tauri commands
├── operations.rs       - Git operations (git2-rs)
└── types.rs            - Git types
```

### WASM Frontend
```
src/git_ui/
├── mod.rs                  - Module exports
├── source_control_panel.rs - Main Git panel (✅ already implemented)
├── diff_view.rs            - Diff viewer (✅ already implemented)
├── blame_view.rs           - Blame annotations (✅ already implemented)
├── commit_history.rs       - Commit log view
└── branch_manager.rs       - Branch management
```

## 実装機能

### 1. Source Control Panel
**Status**: ✅ Already implemented

**機能**:
- ファイル変更一覧表示
- ステージング/アンステージング
- コミットメッセージ入力
- ブランチ選択・切り替え
- リフレッシュボタン

### 2. Diff View
**Status**: ✅ Already implemented

**機能**:
- 行ごとの差分表示
- 追加/削除/変更のハイライト
- サイドバイサイド/インライン表示切り替え

### 3. Blame View
**Status**: ✅ Already implemented

**機能**:
- 行ごとのコミット情報表示
- コミッター情報
- コミット日時
- コミットメッセージ

### 4. Commit History (新規)
**機能**:
- コミット一覧表示
- コミットグラフ
- コミット詳細表示
- ファイル変更リスト

### 5. Branch Manager (新規)
**機能**:
- ブランチ一覧
- 新規ブランチ作成
- ブランチ削除
- マージ操作

### 6. Remote Operations (新規)
**機能**:
- Push
- Pull
- Fetch
- リモートブランチ表示

## Tauri Git Commands

### Status Commands
```rust
git_status() -> Vec<FileStatus>
git_list_branches() -> Vec<BranchInfo>
git_current_branch() -> String
```

### Staging Commands
```rust
git_stage_file(path: String) -> Result<()>
git_unstage_file(path: String) -> Result<()>
git_stage_all() -> Result<()>
git_unstage_all() -> Result<()>
```

### Commit Commands
```rust
git_commit(message: String) -> Result<String>
git_amend(message: String) -> Result<String>
```

### Branch Commands
```rust
git_checkout_branch(name: String) -> Result<()>
git_create_branch(name: String) -> Result<()>
git_delete_branch(name: String) -> Result<()>
git_merge_branch(name: String) -> Result<()>
```

### History Commands
```rust
git_log(limit: u32) -> Vec<CommitInfo>
git_show_commit(hash: String) -> CommitDetail
```

### Diff Commands
```rust
git_diff_file(path: String) -> DiffResult
git_diff_staged() -> Vec<FileDiff>
git_diff_commits(hash1: String, hash2: String) -> Vec<FileDiff>
```

### Blame Commands
```rust
git_blame(path: String) -> Vec<BlameLine>
```

### Remote Commands
```rust
git_push(remote: String, branch: String) -> Result<()>
git_pull(remote: String, branch: String) -> Result<()>
git_fetch(remote: String) -> Result<()>
git_list_remotes() -> Vec<RemoteInfo>
```

## データ型

```rust
#[derive(Serialize, Deserialize)]
pub struct FileStatus {
    pub path: String,
    pub status: String,      // "M", "A", "D", "U", etc.
    pub is_staged: bool,
}

#[derive(Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
}

#[derive(Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
    pub parents: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CommitDetail {
    pub info: CommitInfo,
    pub files: Vec<FileDiff>,
    pub stats: DiffStats,
}

#[derive(Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Serialize, Deserialize)]
pub struct DiffLine {
    pub line_type: String,  // "add", "delete", "context"
    pub content: String,
    pub old_line_no: Option<u32>,
    pub new_line_no: Option<u32>,
}

#[derive(Serialize, Deserialize)]
pub struct BlameLine {
    pub line_no: u32,
    pub commit_hash: String,
    pub author: String,
    pub timestamp: i64,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub fetch_url: Option<String>,
    pub push_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct DiffStats {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}
```

## UI Layout

```
┌─────────────────────────────────────────┐
│  BerryEditor                            │
├────────┬──────────────────┬─────────────┤
│ File   │  Editor Pane     │  Git Panel  │
│ Tree   │                  │             │
│        │                  │ Branch: main│
│        │                  │ ⟳ Refresh   │
│        │                  │             │
│        │                  │ CHANGES     │
│        │                  │ M file.rs + │
│        │                  │ A new.rs  + │
│        │                  │             │
│        │                  │ STAGED      │
│        │                  │ M lib.rs  - │
│        │                  │             │
│        │                  │ Commit msg  │
│        │                  │ [Commit]    │
└────────┴──────────────────┴─────────────┘
```

## パフォーマンス目標
- Git status取得: <100ms
- Diff生成: <200ms
- Commit作成: <500ms
- ファイル数10,000以下のリポジトリで快適動作

## 依存関係
```toml
[dependencies]
git2 = "0.19"         # libgit2 bindings
chrono = "0.4"        # 日時処理
```

## テスト戦略
- ユニットテスト: Git操作関数
- 統合テスト: Tauriコマンド
- UIテスト: コンポーネント動作

## セキュリティ
- ⚠️ プライベートリポジトリの認証情報保護
- ⚠️ SSH鍵管理
- ⚠️ Git hooks実行の制限

## 制限事項
- 大規模リポジトリ (10万ファイル以上) では動作が重くなる可能性
- 複雑なマージコンフリクトは手動解決が必要
- サブモジュールは初期バージョンでは未対応
