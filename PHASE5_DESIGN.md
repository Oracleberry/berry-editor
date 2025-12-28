# Phase 5: UX Polishing - v1.0への磨き上げ

## 概要
Phase 1-4で基本機能は揃いました。Phase 5では「ユーザー体験（UX）」をIntelliJ/VS Codeレベルに引き上げます。

## 目標
- 製品レベルのエディタ体験
- IntelliJ並みの操作性
- プロトタイプから「実用可能なIDE」への昇格

## 実装項目（優先順位順）

### A. コマンドパレット (Search Everywhere) 🎯 最優先
**IntelliJの Shift 2回押し / VS CodeのCmd+P相当**

**機能**:
- ファイル名検索
- アクション検索 (Git commit, 設定, etc.)
- シンボル検索 (関数、クラス、変数)
- 最近開いたファイル
- キーボードショートカット表示

**UI**:
```
┌────────────────────────────────────┐
│ > search query                     │
├────────────────────────────────────┤
│ 📄 src/main.rs                     │
│ 📄 src/editor.rs                   │
│ ⚙️  Settings                       │
│ 🔧 Git: Commit                     │
│ 🔍 Function: handle_keydown        │
└────────────────────────────────────┘
```

**実装**:
- `src/command_palette.rs` - メインコンポーネント
- `Cmd/Ctrl+Shift+P`: コマンドパレットトグル
- `Cmd/Ctrl+P`: ファイル検索モード
- Fuzzy matching (fuzzywuzzy-rs)
- アイコン・カテゴリ表示

**優先度**: ⭐⭐⭐⭐⭐ (最も使用頻度が高く、UX向上効果大)

---

### B. Splitter UI (リサイズ可能パネル) 🎯 高優先
**IntelliJ風のドラッグ可能なサイドバー・ボトムパネル**

**機能**:
- サイドバー幅調整
- ボトムパネル高さ調整
- パネル折りたたみ
- サイズ記憶 (localStorage)

**実装**:
```rust
#[component]
pub fn ResizableSplitter(
    orientation: Orientation,  // Horizontal / Vertical
    initial_size: f64,
    min_size: f64,
    max_size: f64,
) -> impl IntoView
```

**UI**:
```
┌──────┬────────────┬─────────┐
│ Side │            │  Debug  │
│ Bar  │   Editor   │  Panel  │
│ [↔]  │            │  [↔]    │
└──────┴────────────┴─────────┘
       │   Terminal │
       │    [↕]     │
       └────────────┘
```

**実装**:
- `src/common/splitter.rs` - リサイズコンポーネント
- マウスドラッグハンドリング
- CSS Grid/Flexbox との統合

**優先度**: ⭐⭐⭐⭐⭐ (他機能の前提となる基本UI)

---

### C. ターミナル統合 🎯 高優先
**xterm-js + Tauri pty でIDE内シェル**

**機能**:
- IDE内ターミナル
- 複数ターミナルタブ
- カレントディレクトリ連動
- カラーテーマ対応

**アーキテクチャ**:
```
WASM (xterm.js)
  ↕ WebSocket/Tauri invoke
Tauri (pty backend)
  ↕ PTY
Local Shell (bash/zsh/PowerShell)
```

**実装**:
- `src/terminal.rs` - xterm.js Rustラッパー
- `src-tauri/src/pty.rs` - PTYバックエンド
- タブ管理
- キーバインド (Ctrl+\`: ターミナルトグル)

**依存関係**:
```toml
[dependencies]
# WASM side
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["HtmlElement", "WebSocket"] }

# Tauri side
portable-pty = "0.8"
```

**優先度**: ⭐⭐⭐⭐ (開発ワークフローに必須)

---

### D. インクリメンタル・シンタックスハイライト 🎯 中優先
**tree-sitterフル活用で、入力瞬時に構文エラー表示**

**機能**:
- リアルタイム構文解析 (LSPを待たない)
- 構文エラーの赤波線
- セマンティックハイライト
- 括弧マッチング強調

**実装**:
```rust
// Incremental parsing
pub struct IncrementalParser {
    parser: tree_sitter::Parser,
    tree: Option<tree_sitter::Tree>,
}

impl IncrementalParser {
    pub fn parse_incremental(&mut self,
        old_text: &str,
        new_text: &str,
        edit: &InputEdit
    ) -> Tree {
        // tree-sitterの差分パース
    }
}
```

**tree-sitter活用**:
- `tree-sitter-rust`
- `tree-sitter-typescript`
- `tree-sitter-python`
- 差分パース (InputEdit)

**優先度**: ⭐⭐⭐ (リアルタイム性向上、視覚的フィードバック)

---

### E. Debugger完成 🎯 中優先
**debugger/ モジュールの完成**

**機能**:
- ブレークポイント設定
- ステップ実行 (Step Over, Step In, Step Out)
- 変数ウォッチ
- コールスタック表示
- デバッグコンソール

**実装状況**:
- ✅ UI基本構造 (debug_toolbar, variables_panel, call_stack_panel)
- ❌ DAP (Debug Adapter Protocol) 統合
- ❌ ブレークポイント永続化

**必要作業**:
- `src-tauri/src/dap/` - DAPクライアント実装
- ブレークポイントUI統合
- デバッグセッション管理

**優先度**: ⭐⭐⭐ (IDE完成度向上)

---

### F. 追加のUX改善

#### F1. キーボードショートカット可視化
- ショートカット一覧表示
- カスタマイズUI
- コンフリクト検出

#### F2. 設定UI
- 設定パネル
- テーマ選択
- フォントサイズ調整
- キーマップ選択

#### F3. ファイルツリー強化
- ドラッグ&ドロップ
- 右クリックメニュー強化
- ファイル作成ダイアログ
- Gitステータス表示

#### F4. エディタタブ強化
- タブドラッグ並び替え
- タブ分割 (Split Editor)
- タブグループ
- 未保存マーク

#### F5. ステータスバー強化
- カーソル位置表示 (行:列)
- 文字エンコーディング表示
- EOL表示 (LF/CRLF)
- ファイルサイズ表示

#### F6. ミニマップ完成
- `src/minimap.rs` の完成
- スクロール連動
- 構文ハイライト表示

## 実装順序（推奨）

### Sprint 1: 基本UI改善 (1-2週間)
1. ✅ **Splitter UI** - 他の前提
2. ✅ **コマンドパレット** - UX向上効果大

### Sprint 2: 開発ワークフロー (1-2週間)
3. ✅ **ターミナル統合** - cargo build等が実行可能に
4. ✅ **キーボードショートカット可視化**

### Sprint 3: エディタ体験向上 (1-2週間)
5. ✅ **インクリメンタル・シンタックスハイライト**
6. ✅ **エディタタブ強化**
7. ✅ **ミニマップ完成**

### Sprint 4: 開発者機能完成 (1-2週間)
8. ✅ **Debugger完成**
9. ✅ **設定UI**

### Sprint 5: 最終磨き上げ (1週間)
10. ✅ **ファイルツリー強化**
11. ✅ **ステータスバー強化**
12. ✅ パフォーマンス最適化
13. ✅ ドキュメント整備

## v1.0 リリース基準

### 必須機能
- ✅ ファイルツリー (Phase 1)
- ✅ エディタ基本機能 (Phase 1)
- ✅ タブ管理 (Phase 2)
- ✅ 検索・置換 (Phase 2)
- ✅ LSP統合 (Phase 3)
- ✅ Git統合 (Phase 4)
- ⬜ コマンドパレット (Phase 5)
- ⬜ ターミナル統合 (Phase 5)
- ⬜ Splitter UI (Phase 5)

### UX品質
- ⬜ キーボードショートカット完備
- ⬜ レスポンシブ (<50ms インタラクション)
- ⬜ エラーハンドリング完備
- ⬜ ローディング状態表示

### ドキュメント
- ⬜ README.md
- ⬜ ユーザーガイド
- ⬜ キーボードショートカット一覧
- ⬜ 設定リファレンス

### パフォーマンス
- ⬜ 10,000行ファイル: スムーズスクロール
- ⬜ 10,000ファイルプロジェクト: <2秒起動
- ⬜ LSP応答: <100ms
- ⬜ Git操作: <500ms

## 技術的検討事項

### tree-sitter統合
- **現状**: 基本的な使い方のみ
- **改善**: 差分パース、エラー検出、セマンティックハイライト

### WebAssembly最適化
- **現状**: デバッグビルド
- **改善**: --release, wasm-opt, コード分割

### Tauri最適化
- **現状**: 基本設定
- **改善**: バンドルサイズ削減、起動時間短縮

## まとめ

Phase 5では、「機能」から「体験」へのシフトを行います。

**Before (Phase 1-4)**: 「動く」エディタ
**After (Phase 5)**: 「使いたくなる」IDE

これにより、BerryEditorはRust製エディタ界の有力な選択肢（Zed, Lapce等に並ぶ存在）となります。
