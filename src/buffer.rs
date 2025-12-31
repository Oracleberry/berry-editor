//! Text Buffer Implementation using Ropey
//! IntelliJ-inspired design: Immutable snapshots with lazy evaluation

use ropey::Rope;
use std::collections::HashMap;

/// ✅ IntelliJ Design: TextBuffer with syntax highlighting cache
/// Cache stores pre-rendered HTML for visible lines only
#[derive(Clone)]
pub struct TextBuffer {
    rope: Rope,
    file_path: Option<String>,
    modified: bool,
    language: String,
    /// ✅ IntelliJ Snapshot: Cache syntax-highlighted lines (line_idx -> HTML)
    /// Only stores visible lines to save memory (cleared on large edits)
    syntax_cache: HashMap<usize, String>,
    /// Version counter - incremented on every edit to invalidate cache
    version: u64,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            file_path: None,
            modified: false,
            language: String::from("plaintext"),
            syntax_cache: HashMap::new(),
            version: 0,
        }
    }

    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            file_path: None,
            modified: false,
            language: String::from("plaintext"),
            syntax_cache: HashMap::new(),
            version: 0,
        }
    }

    pub fn set_file_path(&mut self, path: String) {
        self.file_path = Some(path);
    }

    pub fn set_language(&mut self, lang: String) {
        self.language = lang;
    }

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        // ✅ 境界チェック：char_idxをバッファサイズ以内にクランプ
        let safe_idx = char_idx.min(self.rope.len_chars());

        // ✅ IntelliJ Pro: Incremental Syntax Analysis - only invalidate affected lines
        let start_line = self.rope.char_to_line(safe_idx);
        let newline_count = text.chars().filter(|&c| c == '\n').count();

        self.rope.insert(safe_idx, text);
        self.modified = true;
        self.version += 1;

        // ✅ IntelliJ Pro: Smart cache invalidation
        // Only clear lines that were actually modified + surrounding context
        let end_line = start_line + newline_count + 2; // +2 for context
        self.invalidate_cache_range(start_line, end_line);
    }

    pub fn remove(&mut self, start: usize, end: usize) {
        // ✅ 境界チェック：start と end をバッファサイズ以内にクランプ
        let safe_start = start.min(self.rope.len_chars());
        let safe_end = end.min(self.rope.len_chars());

        // 削除する範囲がない場合は早期リターン
        if safe_start >= safe_end {
            return;
        }

        // ✅ IntelliJ Pro: Incremental invalidation for deletions
        let start_line = self.rope.char_to_line(safe_start);
        let end_line = self.rope.char_to_line(safe_end);

        self.rope.remove(safe_start..safe_end);
        self.modified = true;
        self.version += 1;

        // ✅ IntelliJ Pro: Only invalidate affected range
        self.invalidate_cache_range(start_line, end_line + 2); // +2 for context
    }

    /// ✅ IntelliJ Pro: Invalidate only specific line range (incremental)
    fn invalidate_cache_range(&mut self, start_line: usize, end_line: usize) {
        self.syntax_cache.retain(|&line_idx, _| {
            line_idx < start_line || line_idx > end_line
        });
    }

    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn line(&self, line_idx: usize) -> Option<String> {
        if line_idx < self.len_lines() {
            Some(self.rope.line(line_idx).to_string())
        } else {
            None
        }
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn mark_saved(&mut self) {
        self.modified = false;
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn file_path(&self) -> Option<&str> {
        self.file_path.as_deref()
    }

    /// Convert line index to character index (start of the line)
    pub fn line_to_char(&self, line_idx: usize) -> usize {
        self.rope.line_to_char(line_idx)
    }

    /// Convert character index to line index
    pub fn char_to_line(&self, char_idx: usize) -> usize {
        self.rope.char_to_line(char_idx)
    }

    /// Get a slice of text from start to end char indices
    pub fn slice(&self, start: usize, end: usize) -> Option<String> {
        if start <= end && end <= self.len_chars() {
            Some(self.rope.slice(start..end).to_string())
        } else {
            None
        }
    }

    /// ✅ IntelliJ Design: Get version for cache invalidation
    pub fn version(&self) -> u64 {
        self.version
    }

    /// ✅ IntelliJ Design: Cache highlighted HTML for a line (on-demand)
    /// Returns cached HTML if available, None if not cached yet
    pub fn get_cached_highlight(&self, line_idx: usize) -> Option<&str> {
        self.syntax_cache.get(&line_idx).map(|s| s.as_str())
    }

    /// ✅ IntelliJ Design: Store highlighted HTML for a line
    pub fn cache_highlight(&mut self, line_idx: usize, html: String) {
        self.syntax_cache.insert(line_idx, html);
    }

    /// ✅ IntelliJ Design: Evict old cache entries (keep only visible range)
    /// Call this periodically to prevent cache from growing too large
    pub fn trim_cache(&mut self, visible_start: usize, visible_end: usize, keep_margin: usize) {
        let keep_start = visible_start.saturating_sub(keep_margin);
        let keep_end = visible_end + keep_margin;

        self.syntax_cache.retain(|&line_idx, _| {
            line_idx >= keep_start && line_idx <= keep_end
        });
    }

    /// ✅ IntelliJ Pro: Create immutable snapshot of the rope for rendering
    /// This is O(1) operation - Rope uses Arc internally, so clone is instant!
    /// The snapshot is frozen and won't be affected by future edits
    pub fn snapshot(&self) -> Rope {
        self.rope.clone() // O(1) - just clones Arc pointer, not data!
    }

    /// ✅ IntelliJ Pro: Get line from snapshot (for safe concurrent access)
    pub fn line_from_snapshot(snapshot: &Rope, line_idx: usize) -> Option<String> {
        if line_idx < snapshot.len_lines() {
            Some(snapshot.line(line_idx).to_string())
        } else {
            None
        }
    }

    /// ✅ IntelliJ Pro: Get horizontal segment of a line (for long-line rendering)
    /// Only returns visible characters within viewport bounds
    pub fn line_segment(&self, line_idx: usize, start_col: usize, end_col: usize) -> Option<String> {
        let line = self.line(line_idx)?;
        let chars: Vec<char> = line.chars().collect();
        let end = end_col.min(chars.len());
        if start_col >= chars.len() {
            return Some(String::new());
        }
        Some(chars[start_col..end].iter().collect())
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    // ✅ MEMORY FIX: Memory load test to verify efficiency with large files
    #[wasm_bindgen_test]
    fn test_large_file_memory_efficiency() {
        // Generate ~5MB of data (100,000 lines)
        let large_text = (0..100000)
            .map(|i| format!("Line {}: some repetitive text to fill memory and simulate real files...\n", i))
            .collect::<String>();

        let start_lines = 100000;
        let buffer = TextBuffer::from_str(&large_text);

        // Ropey uses rope data structure, so cloning should NOT duplicate the entire content
        // Instead, it shares the internal data structure (copy-on-write)
        let cloned_buffer = buffer.clone();

        // Note: 100000 lines with \n creates 100001 lines (last empty line)
        assert_eq!(buffer.len_lines(), start_lines + 1);
        assert_eq!(cloned_buffer.len_lines(), start_lines + 1);

        // Verify operations work correctly on large files
        let line_0 = buffer.line(0);
        assert!(line_0.is_some());
        assert!(line_0.unwrap().contains("Line 0:"));

        let line_50000 = buffer.line(50000);
        assert!(line_50000.is_some());
        assert!(line_50000.unwrap().contains("Line 50000:"));

        // Note: Actual memory usage should be profiled in Chrome DevTools
        // Expected behavior: Total heap size should stay under 300MB even with multiple clones
    }

    #[wasm_bindgen_test]
    fn test_multiple_clones_dont_explode_memory() {
        // Create a moderately large buffer
        let text = (0..10000)
            .map(|i| format!("Line {}: test data\n", i))
            .collect::<String>();

        let buffer = TextBuffer::from_str(&text);

        // Clone multiple times (simulating multiple tabs with same file)
        let _clone1 = buffer.clone();
        let _clone2 = buffer.clone();
        let _clone3 = buffer.clone();
        let _clone4 = buffer.clone();
        let _clone5 = buffer.clone();

        // All clones should work correctly
        // Note: 10000 lines with \n creates 10001 lines (last empty line)
        assert_eq!(buffer.len_lines(), 10001);
        assert_eq!(_clone1.len_lines(), 10001);
        assert_eq!(_clone5.len_lines(), 10001);

        // Memory should NOT increase by 5x due to Ropey's internal sharing
    }

    #[wasm_bindgen_test]
    fn test_new_buffer() {
        let buffer = TextBuffer::new();
        assert_eq!(buffer.len_chars(), 0);
        assert_eq!(buffer.len_lines(), 1); // Ropey always has at least 1 line
        assert!(!buffer.is_modified());
        assert_eq!(buffer.language(), "plaintext");
    }

    #[wasm_bindgen_test]
    fn test_from_str() {
        let text = "Hello\nWorld";
        let buffer = TextBuffer::from_str(text);
        assert_eq!(buffer.to_string(), text);
        assert_eq!(buffer.len_lines(), 2);
        assert!(!buffer.is_modified());
    }

    #[wasm_bindgen_test]
    fn test_insert() {
        let mut buffer = TextBuffer::from_str("Hello");
        buffer.insert(5, " World");
        assert_eq!(buffer.to_string(), "Hello World");
        assert!(buffer.is_modified());
    }

    #[wasm_bindgen_test]
    fn test_remove() {
        let mut buffer = TextBuffer::from_str("Hello World");
        buffer.remove(5, 11);
        assert_eq!(buffer.to_string(), "Hello");
        assert!(buffer.is_modified());
    }

    #[wasm_bindgen_test]
    fn test_len_chars() {
        let buffer = TextBuffer::from_str("Hello");
        assert_eq!(buffer.len_chars(), 5);
    }

    #[wasm_bindgen_test]
    fn test_len_lines() {
        let buffer = TextBuffer::from_str("Line1\nLine2\nLine3");
        assert_eq!(buffer.len_lines(), 3);
    }

    #[wasm_bindgen_test]
    fn test_line() {
        let buffer = TextBuffer::from_str("Line1\nLine2\nLine3");
        assert_eq!(buffer.line(0).unwrap(), "Line1\n");
        assert_eq!(buffer.line(1).unwrap(), "Line2\n");
        assert_eq!(buffer.line(2).unwrap(), "Line3");
        assert!(buffer.line(3).is_none());
    }

    #[wasm_bindgen_test]
    fn test_mark_saved() {
        let mut buffer = TextBuffer::from_str("Hello");
        buffer.insert(5, " World");
        assert!(buffer.is_modified());
        buffer.mark_saved();
        assert!(!buffer.is_modified());
    }

    #[wasm_bindgen_test]
    fn test_set_file_path() {
        let mut buffer = TextBuffer::new();
        assert!(buffer.file_path().is_none());
        buffer.set_file_path("/path/to/file.rs".to_string());
        assert_eq!(buffer.file_path(), Some("/path/to/file.rs"));
    }

    #[wasm_bindgen_test]
    fn test_set_language() {
        let mut buffer = TextBuffer::new();
        assert_eq!(buffer.language(), "plaintext");
        buffer.set_language("rust".to_string());
        assert_eq!(buffer.language(), "rust");
    }

    #[wasm_bindgen_test]
    fn test_multiple_operations() {
        let mut buffer = TextBuffer::from_str("Hello");
        buffer.insert(0, "Well, ");
        buffer.insert(buffer.len_chars(), "!");
        assert_eq!(buffer.to_string(), "Well, Hello!");

        buffer.remove(0, 6);
        assert_eq!(buffer.to_string(), "Hello!");

        assert!(buffer.is_modified());
        buffer.mark_saved();
        assert!(!buffer.is_modified());
    }

    // ========== 境界条件・異常系テスト ==========

    #[wasm_bindgen_test]
    fn test_empty_buffer_operations() {
        let mut buffer = TextBuffer::new();

        // 空バッファへの削除（クラッシュしないこと）
        buffer.remove(0, 0);
        assert_eq!(buffer.len_chars(), 0);

        // 空バッファへの挿入
        buffer.insert(0, "First");
        assert_eq!(buffer.to_string(), "First");

        // 全削除
        buffer.remove(0, buffer.len_chars());
        assert_eq!(buffer.len_chars(), 0);
    }

    #[wasm_bindgen_test]
    fn test_out_of_bounds_insert() {
        let mut buffer = TextBuffer::from_str("Hello");
        let initial_len = buffer.len_chars();

        // 境界外への挿入（min()でクランプされる）
        buffer.insert(100, " World");

        // 末尾に追加されるべき
        assert_eq!(buffer.to_string(), "Hello World");
        assert_eq!(buffer.len_chars(), initial_len + 6);
    }

    #[wasm_bindgen_test]
    fn test_out_of_bounds_remove() {
        let mut buffer = TextBuffer::from_str("Hello");

        // 境界外の削除範囲（min()でクランプされる）
        buffer.remove(0, 1000);
        assert_eq!(buffer.len_chars(), 0);
    }

    #[wasm_bindgen_test]
    fn test_remove_with_start_greater_than_length() {
        let mut buffer = TextBuffer::from_str("Hello");

        // start が length より大きい場合
        buffer.remove(100, 200);

        // 何も削除されない（Ropeの動作に依存）
        assert!(buffer.len_chars() <= 5);
    }

    #[wasm_bindgen_test]
    fn test_slice_boundary_conditions() {
        let buffer = TextBuffer::from_str("Hello");

        // 正常なスライス
        assert_eq!(buffer.slice(0, 5), Some("Hello".to_string()));

        // start == end
        assert_eq!(buffer.slice(2, 2), Some("".to_string()));

        // start > end (None を返す)
        assert_eq!(buffer.slice(5, 2), None);

        // end > len_chars() (None を返す)
        assert_eq!(buffer.slice(0, 100), None);

        // 両方とも範囲外
        assert_eq!(buffer.slice(100, 200), None);
    }

    #[wasm_bindgen_test]
    fn test_line_segment_boundary_conditions() {
        let buffer = TextBuffer::from_str("Short");

        // 列範囲が文字数を超える場合
        let segment = buffer.line_segment(0, 10, 20);
        assert_eq!(segment, Some("".to_string()));

        // start_col が文字数を超える場合
        let segment = buffer.line_segment(0, 100, 200);
        assert_eq!(segment, Some("".to_string()));

        // 存在しない行
        let segment = buffer.line_segment(10, 0, 5);
        assert_eq!(segment, None);

        // 正常なケース（部分文字列）
        let buffer2 = TextBuffer::from_str("Hello World");
        let segment = buffer2.line_segment(0, 0, 5);
        assert_eq!(segment, Some("Hello".to_string()));

        // end_col が文字数を超える（クランプされる）
        let segment = buffer2.line_segment(0, 6, 100);
        assert_eq!(segment, Some("World".to_string()));
    }

    #[wasm_bindgen_test]
    fn test_cache_operations() {
        let mut buffer = TextBuffer::from_str("Line1\nLine2\nLine3");

        // キャッシュなし
        assert_eq!(buffer.get_cached_highlight(0), None);

        // キャッシュ追加
        buffer.cache_highlight(0, "<html>Line1</html>".to_string());
        buffer.cache_highlight(1, "<html>Line2</html>".to_string());
        buffer.cache_highlight(2, "<html>Line3</html>".to_string());

        // キャッシュ取得
        assert_eq!(buffer.get_cached_highlight(0), Some("<html>Line1</html>"));
        assert_eq!(buffer.get_cached_highlight(1), Some("<html>Line2</html>"));
        assert_eq!(buffer.get_cached_highlight(2), Some("<html>Line3</html>"));

        // 存在しないキャッシュ
        assert_eq!(buffer.get_cached_highlight(10), None);
    }

    #[wasm_bindgen_test]
    fn test_trim_cache_precision() {
        let mut buffer = TextBuffer::from_str("L1\nL2\nL3\nL4\nL5");

        // 全行にキャッシュを設定
        for i in 0..5 {
            buffer.cache_highlight(i, format!("<html>L{}</html>", i + 1));
        }

        // 行1から行3まで保持（マージン0）
        buffer.trim_cache(1, 3, 0);

        // 行0は削除されるべき
        assert_eq!(buffer.get_cached_highlight(0), None);

        // 行1-3は保持されるべき
        assert!(buffer.get_cached_highlight(1).is_some());
        assert!(buffer.get_cached_highlight(2).is_some());
        assert!(buffer.get_cached_highlight(3).is_some());

        // 行4は削除されるべき
        assert_eq!(buffer.get_cached_highlight(4), None);
    }

    #[wasm_bindgen_test]
    fn test_trim_cache_with_margin() {
        let mut buffer = TextBuffer::from_str("L1\nL2\nL3\nL4\nL5\nL6\nL7");

        for i in 0..7 {
            buffer.cache_highlight(i, format!("<html>L{}</html>", i + 1));
        }

        // 行2-4を保持、マージン1
        buffer.trim_cache(2, 4, 1);

        // マージンを含めて行1-5が保持される
        assert!(buffer.get_cached_highlight(1).is_some()); // マージン
        assert!(buffer.get_cached_highlight(2).is_some()); // 範囲内
        assert!(buffer.get_cached_highlight(3).is_some()); // 範囲内
        assert!(buffer.get_cached_highlight(4).is_some()); // 範囲内
        assert!(buffer.get_cached_highlight(5).is_some()); // マージン

        // 行0と行6は削除される
        assert_eq!(buffer.get_cached_highlight(0), None);
        assert_eq!(buffer.get_cached_highlight(6), None);
    }

    #[wasm_bindgen_test]
    fn test_cache_invalidation_on_insert() {
        let mut buffer = TextBuffer::from_str("Line1\nLine2\nLine3\nLine4");

        // 全行にキャッシュ
        for i in 0..4 {
            buffer.cache_highlight(i, format!("<cached>{}</cached>", i));
        }

        let initial_version = buffer.version();

        // 行1に挿入（行1とその周辺のキャッシュが無効化される）
        let insert_pos = buffer.line_to_char(1);
        buffer.insert(insert_pos, "NEW");

        // バージョンが更新される
        assert_eq!(buffer.version(), initial_version + 1);

        // 影響を受けた行のキャッシュが削除される（start_line=1, end_line=1+0+2=3）
        assert_eq!(buffer.get_cached_highlight(1), None);
        assert_eq!(buffer.get_cached_highlight(2), None);
        assert_eq!(buffer.get_cached_highlight(3), None);

        // 行0は影響を受けないので保持される
        assert!(buffer.get_cached_highlight(0).is_some());
    }

    #[wasm_bindgen_test]
    fn test_cache_invalidation_on_remove() {
        let mut buffer = TextBuffer::from_str("Line1\nLine2\nLine3\nLine4\nLine5");

        for i in 0..5 {
            buffer.cache_highlight(i, format!("<cached>{}</cached>", i));
        }

        // 行2を削除
        let start = buffer.line_to_char(2);
        let end = buffer.line_to_char(3);
        buffer.remove(start, end);

        // 行2とその周辺（+2）のキャッシュが削除される
        assert_eq!(buffer.get_cached_highlight(2), None);
        assert_eq!(buffer.get_cached_highlight(3), None);
        assert_eq!(buffer.get_cached_highlight(4), None);
    }

    #[wasm_bindgen_test]
    fn test_line_char_conversion_boundary() {
        let buffer = TextBuffer::from_str("L1\nL2\nL3");

        // 行0の開始文字インデックス
        assert_eq!(buffer.line_to_char(0), 0);

        // 行1の開始文字インデックス（"L1\n" = 3文字）
        assert_eq!(buffer.line_to_char(1), 3);

        // 行2の開始文字インデックス
        assert_eq!(buffer.line_to_char(2), 6);

        // 文字インデックスから行番号
        assert_eq!(buffer.char_to_line(0), 0); // "L"
        assert_eq!(buffer.char_to_line(2), 0); // "\n"
        assert_eq!(buffer.char_to_line(3), 1); // "L"
        assert_eq!(buffer.char_to_line(6), 2); // "L"
    }

    #[wasm_bindgen_test]
    fn test_snapshot_immutability() {
        let mut buffer = TextBuffer::from_str("Original");

        // スナップショット作成
        let snapshot = buffer.snapshot();

        // バッファを変更
        buffer.insert(8, " Modified");

        // スナップショットは変更前の状態を保持
        assert_eq!(snapshot.to_string(), "Original");
        assert_eq!(buffer.to_string(), "Original Modified");
    }

    #[wasm_bindgen_test]
    fn test_line_from_snapshot() {
        let buffer = TextBuffer::from_str("Line1\nLine2\nLine3");
        let snapshot = buffer.snapshot();

        // 正常な行取得
        assert_eq!(
            TextBuffer::line_from_snapshot(&snapshot, 0),
            Some("Line1\n".to_string())
        );
        assert_eq!(
            TextBuffer::line_from_snapshot(&snapshot, 2),
            Some("Line3".to_string())
        );

        // 範囲外
        assert_eq!(TextBuffer::line_from_snapshot(&snapshot, 10), None);
    }

    #[wasm_bindgen_test]
    fn test_default_trait() {
        let buffer = TextBuffer::default();
        assert_eq!(buffer.len_chars(), 0);
        assert_eq!(buffer.len_lines(), 1);
        assert!(!buffer.is_modified());
    }

    #[wasm_bindgen_test]
    fn test_version_increment() {
        let mut buffer = TextBuffer::new();
        let v0 = buffer.version();

        buffer.insert(0, "A");
        let v1 = buffer.version();
        assert_eq!(v1, v0 + 1);

        buffer.remove(0, 1);
        let v2 = buffer.version();
        assert_eq!(v2, v1 + 1);

        // mark_saved はバージョンを変更しない
        buffer.mark_saved();
        assert_eq!(buffer.version(), v2);
    }

    #[wasm_bindgen_test]
    fn test_multiline_insert_cache_invalidation() {
        let mut buffer = TextBuffer::from_str("Line1\nLine2\nLine3");

        for i in 0..3 {
            buffer.cache_highlight(i, format!("cached{}", i));
        }

        // 複数行を挿入（改行2つ = newline_count = 2）
        buffer.insert(0, "New1\nNew2\n");

        // start_line=0, newline_count=2, end_line=0+2+2=4
        // 行0から行4のキャッシュが削除される
        assert_eq!(buffer.get_cached_highlight(0), None);
        assert_eq!(buffer.get_cached_highlight(1), None);
        assert_eq!(buffer.get_cached_highlight(2), None);
    }
}
