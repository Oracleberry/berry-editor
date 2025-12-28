# Phase 3: コードインテリジェンス - 設計ドキュメント

## 概要

Phase 3では、Tree-sitterとLSP（Language Server Protocol）を統合し、本格的なコードインテリジェンス機能を実装します。これにより、VS CodeやIntelliJ IDEAと同等の開発体験を提供します。

## 目標

1. **正確な構文解析**: Tree-sitterによる高速・正確な構文木構築
2. **LSP統合**: 業界標準のLanguage Server Protocolサポート
3. **リアルタイム診断**: エラー・警告のリアルタイム表示
4. **インテリセンス**: 自動補完、ホバー情報、定義へのジャンプ

## アーキテクチャ

### 1. Tree-sitter統合

```
┌─────────────────────────────────────┐
│     Editor Frontend (WASM)          │
│  ┌──────────────────────────────┐   │
│  │   Tree-sitter WASM Binding   │   │
│  │  - tree-sitter-rust.wasm     │   │
│  │  - tree-sitter-javascript    │   │
│  │  - tree-sitter-python        │   │
│  └──────────────────────────────┘   │
│  ┌──────────────────────────────┐   │
│  │   Syntax Highlighter v2      │   │
│  │  - Token extraction          │   │
│  │  - Semantic highlighting     │   │
│  └──────────────────────────────┘   │
└─────────────────────────────────────┘
```

**主要コンポーネント**:
- `src/tree_sitter/mod.rs` - Tree-sitterメインモジュール
- `src/tree_sitter/parser.rs` - パーサー管理
- `src/tree_sitter/highlighter.rs` - 構文ハイライト
- `src/tree_sitter/query.rs` - Tree-sitterクエリ実行

**機能**:
- 複数言語のパーサーサポート
- インクリメンタル解析（編集時の差分更新）
- スコープベースのセマンティックハイライト
- 構文木のキャッシング

### 2. LSPクライアント実装

```
┌─────────────────────────────────────────────┐
│           Tauri Backend (Rust)              │
│  ┌────────────────────────────────────┐     │
│  │      LSP Client Manager            │     │
│  │  - rust-analyzer                   │     │
│  │  - typescript-language-server      │     │
│  │  - pyright                         │     │
│  └────────────────────────────────────┘     │
│  ┌────────────────────────────────────┐     │
│  │      LSP Message Router            │     │
│  │  - JSON-RPC 2.0                    │     │
│  │  - Request/Response handling       │     │
│  └────────────────────────────────────┘     │
└─────────────────────────────────────────────┘
          ↕ IPC (Tauri Commands)
┌─────────────────────────────────────────────┐
│        Frontend (WASM/Leptos)               │
│  ┌────────────────────────────────────┐     │
│  │      LSP Client (WASM side)        │     │
│  │  - Capability negotiation          │     │
│  │  - Document sync                   │     │
│  │  - Feature requests                │     │
│  └────────────────────────────────────┘     │
└─────────────────────────────────────────────┘
```

**主要コンポーネント**:
- `src-tauri/src/lsp/mod.rs` - LSPサーバー管理
- `src-tauri/src/lsp/client.rs` - LSPクライアント
- `src-tauri/src/lsp/protocol.rs` - LSPプロトコル実装
- `src/lsp_client/mod.rs` - WASM側LSPクライアント
- `src/lsp_client/features.rs` - LSP機能（completion, hover等）

**サポート機能**:
1. **textDocument/completion** - オートコンプリート
2. **textDocument/hover** - ホバー情報
3. **textDocument/definition** - 定義へジャンプ
4. **textDocument/references** - 参照検索
5. **textDocument/publishDiagnostics** - 診断情報
6. **textDocument/rename** - リネーム
7. **textDocument/formatting** - フォーマット

### 3. 診断システム

```
┌─────────────────────────────────────┐
│     Diagnostics Manager             │
│  ┌──────────────────────────────┐   │
│  │   Diagnostic Collection      │   │
│  │  - Errors                    │   │
│  │  - Warnings                  │   │
│  │  - Info/Hints                │   │
│  └──────────────────────────────┘   │
│  ┌──────────────────────────────┐   │
│  │   Gutter Markers             │   │
│  │  - Line error indicators     │   │
│  │  - Severity colors           │   │
│  └──────────────────────────────┘   │
│  ┌──────────────────────────────┐   │
│  │   Diagnostics Panel          │   │
│  │  - Grouped by severity       │   │
│  │  - Click to navigate         │   │
│  └──────────────────────────────┘   │
└─────────────────────────────────────┘
```

**主要コンポーネント**:
- `src/diagnostics/mod.rs` - 診断システムコア
- `src/diagnostics/gutter.rs` - ガターマーカー
- `src/diagnostics_panel.rs` - 診断パネルUI（既存の強化）

### 4. オートコンプリート強化

```
┌─────────────────────────────────────┐
│   Completion Widget (Enhanced)      │
│  ┌──────────────────────────────┐   │
│  │   Trigger Detection          │   │
│  │  - Dot trigger (.)           │   │
│  │  - Identifier typing         │   │
│  │  - Import suggestions        │   │
│  └──────────────────────────────┘   │
│  ┌──────────────────────────────┐   │
│  │   LSP Completion Items       │   │
│  │  - Methods                   │   │
│  │  - Properties                │   │
│  │  - Keywords                  │   │
│  │  - Snippets                  │   │
│  └──────────────────────────────┘   │
│  ┌──────────────────────────────┐   │
│  │   Fuzzy Matching & Sorting   │   │
│  │  - Relevance scoring         │   │
│  │  - Recently used items       │   │
│  └──────────────────────────────┘   │
└─────────────────────────────────────┘
```

## データ構造

### Tree-sitter関連

```rust
// src/tree_sitter/parser.rs

pub struct TreeSitterParser {
    parser: tree_sitter::Parser,
    language: Option<&'static tree_sitter::Language>,
    tree: Option<tree_sitter::Tree>,
    source: String,
}

pub struct ParsedDocument {
    tree: tree_sitter::Tree,
    language: &'static tree_sitter::Language,
    highlights: Vec<HighlightSpan>,
}

pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub highlight_type: HighlightType,
}

pub enum HighlightType {
    Keyword,
    Function,
    Variable,
    Type,
    String,
    Number,
    Comment,
    Operator,
    // ... more types
}
```

### LSP関連

```rust
// src-tauri/src/lsp/protocol.rs

pub struct LspServer {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    capabilities: ServerCapabilities,
    request_id: AtomicU64,
}

pub struct CompletionRequest {
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
    pub context: Option<CompletionContext>,
}

pub struct HoverRequest {
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
}

pub struct DiagnosticNotification {
    pub uri: String,
    pub diagnostics: Vec<Diagnostic>,
}
```

### 診断関連

```rust
// src/diagnostics/mod.rs

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub code: Option<String>,
    pub source: Option<String>,
    pub message: String,
    pub related_information: Vec<DiagnosticRelatedInformation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

pub struct DiagnosticsCollection {
    diagnostics_by_file: HashMap<String, Vec<Diagnostic>>,
    gutter_markers: HashMap<String, Vec<GutterMarker>>,
}
```

## ファイル構成

```
gui-editor/
├── src/
│   ├── tree_sitter/
│   │   ├── mod.rs              # Tree-sitterメインモジュール
│   │   ├── parser.rs           # パーサー管理
│   │   ├── highlighter.rs      # 構文ハイライト
│   │   ├── query.rs            # クエリ実行
│   │   └── languages/          # 言語別設定
│   │       ├── rust.rs
│   │       ├── javascript.rs
│   │       └── python.rs
│   ├── lsp_client/
│   │   ├── mod.rs              # LSPクライアント（WASM側）
│   │   ├── features.rs         # LSP機能実装
│   │   ├── completion.rs       # オートコンプリート
│   │   ├── hover.rs            # ホバー情報
│   │   └── diagnostics.rs      # 診断受信
│   ├── diagnostics/
│   │   ├── mod.rs              # 診断システムコア
│   │   ├── gutter.rs           # ガターマーカー
│   │   └── collection.rs       # 診断データ管理
│   └── diagnostics_panel.rs    # 診断パネルUI（強化版）
│
├── src-tauri/
│   └── src/
│       └── lsp/
│           ├── mod.rs          # LSPサーバー管理
│           ├── client.rs       # LSPクライアント
│           ├── protocol.rs     # プロトコル実装
│           ├── jsonrpc.rs      # JSON-RPC 2.0
│           └── servers/        # Language Server設定
│               ├── rust_analyzer.rs
│               ├── typescript.rs
│               └── python.rs
│
└── tests/
    ├── phase3_tree_sitter_test.rs
    ├── phase3_lsp_test.rs
    └── phase3_diagnostics_test.rs
```

## 実装計画

### ステップ1: Tree-sitter統合（基礎）

1. **Tree-sitter WASMセットアップ**
   - `tree-sitter-rust`のWASMビルド
   - WASMバインディング作成
   - パーサーの初期化

2. **基本的な構文解析**
   - ドキュメントパース
   - 構文木の取得
   - エラーノード検出

3. **シンプルなハイライト**
   - Tree-sitterクエリ定義
   - トークンタイプ抽出
   - 既存ハイライトシステムとの統合

### ステップ2: LSPクライアント実装

1. **Tauriバックエンド**
   - LSPサーバープロセス管理
   - JSON-RPC 2.0実装
   - stdin/stdoutハンドリング

2. **初期化プロトコル**
   - `initialize`リクエスト
   - サーバーケイパビリティ取得
   - `initialized`通知

3. **ドキュメント同期**
   - `textDocument/didOpen`
   - `textDocument/didChange`
   - `textDocument/didSave`
   - `textDocument/didClose`

### ステップ3: コードインテリジェンス機能

1. **オートコンプリート**
   - トリガー文字検出（`.`, `::`等）
   - `textDocument/completion`リクエスト
   - 補完アイテム表示
   - 選択時のテキスト挿入

2. **ホバー情報**
   - カーソル位置での`textDocument/hover`
   - Markdownレンダリング
   - 型情報・ドキュメント表示

3. **定義へジャンプ**
   - `textDocument/definition`リクエスト
   - ファイル間ナビゲーション
   - 該当位置へスクロール

### ステップ4: 診断システム

1. **診断受信**
   - `textDocument/publishDiagnostics`通知
   - 診断データの保存
   - ファイル別グルーピング

2. **ガター表示**
   - エラーアイコン表示
   - 重要度別の色分け
   - ホバーでメッセージ表示

3. **診断パネル**
   - エラー/警告一覧
   - 重要度別フィルタ
   - クリックでジャンプ

### ステップ5: テストとドキュメント

1. **ユニットテスト**
   - Tree-sitterパーサーテスト
   - LSPプロトコルテスト
   - 診断システムテスト

2. **統合テスト**
   - エンドツーエンドのLSPフロー
   - 複数言語サポート
   - パフォーマンステスト

3. **ドキュメント**
   - API仕様書
   - 使用例
   - トラブルシューティング

## パフォーマンス目標

| 機能 | 目標 | 備考 |
|------|------|------|
| Tree-sitter解析 | <50ms | 10,000行ファイル |
| LSP初期化 | <2秒 | rust-analyzer起動 |
| オートコンプリート | <100ms | リクエストから表示まで |
| 診断更新 | <200ms | 保存後の診断表示 |
| ホバー情報 | <50ms | カーソル移動から表示 |

## 依存関係

### Rust Crates (src-tauri)

```toml
[dependencies]
# LSP実装
tower-lsp = "0.20"
lsp-types = "0.95"
serde_json = "1.0"

# プロセス管理
tokio = { version = "1", features = ["full"] }
```

### JavaScript/WASM

```json
{
  "dependencies": {
    "tree-sitter": "^0.21.0",
    "tree-sitter-rust": "^0.21.0",
    "tree-sitter-javascript": "^0.21.0",
    "tree-sitter-python": "^0.21.0"
  }
}
```

## テスト計画

### Tree-sitter テスト

1. **パーサーテスト**
   - 各言語の基本構文
   - エラー回復
   - インクリメンタルパース

2. **ハイライトテスト**
   - トークン抽出
   - 色分け正確性
   - エッジケース

### LSP テスト

1. **プロトコルテスト**
   - 初期化シーケンス
   - リクエスト/レスポンス
   - 通知ハンドリング

2. **機能テスト**
   - オートコンプリート精度
   - ホバー情報の正確性
   - 定義ジャンプ

### 診断テスト

1. **データ管理テスト**
   - 診断の追加/削除
   - ファイル別グルーピング
   - 重要度フィルタ

2. **UI テスト**
   - ガター表示
   - パネル表示
   - クリックナビゲーション

## リスクと対策

| リスク | 影響 | 対策 |
|--------|------|------|
| Tree-sitter WASMサイズ | バンドルサイズ増大 | 遅延ロード、言語別分割 |
| LSPサーバー起動遅延 | 初期UX低下 | プログレス表示、バックグラウンド初期化 |
| メモリ使用量 | 大規模ファイルで問題 | 構文木キャッシング、LRU戦略 |
| LSP互換性 | サーバー間の差異 | 共通インターフェース、フォールバック |

## 次フェーズへの展開

Phase 3完了後の発展:
1. **Phase 4**: デバッグ機能（DAP統合）
2. **Phase 5**: Git統合UI
3. **Phase 6**: リファクタリング機能
4. **Phase 7**: プロジェクト管理

## まとめ

Phase 3では、Tree-sitterとLSPを統合することで、BerryEditorを本格的なIDEに進化させます。正確な構文解析、リアルタイム診断、インテリジェントな補完により、開発者体験を大幅に向上させます。
