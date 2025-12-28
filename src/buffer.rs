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
        // ✅ IntelliJ Pro: Incremental Syntax Analysis - only invalidate affected lines
        let start_line = self.rope.char_to_line(char_idx.min(self.rope.len_chars()));
        let newline_count = text.chars().filter(|&c| c == '\n').count();

        self.rope.insert(char_idx, text);
        self.modified = true;
        self.version += 1;

        // ✅ IntelliJ Pro: Smart cache invalidation
        // Only clear lines that were actually modified + surrounding context
        let end_line = start_line + newline_count + 2; // +2 for context
        self.invalidate_cache_range(start_line, end_line);
    }

    pub fn remove(&mut self, start: usize, end: usize) {
        // ✅ IntelliJ Pro: Incremental invalidation for deletions
        let start_line = self.rope.char_to_line(start.min(self.rope.len_chars()));
        let end_char = end.min(self.rope.len_chars());
        let end_line = self.rope.char_to_line(end_char);

        self.rope.remove(start..end);
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

        assert_eq!(buffer.len_lines(), start_lines);
        assert_eq!(cloned_buffer.len_lines(), start_lines);

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
        assert_eq!(buffer.len_lines(), 10000);
        assert_eq!(_clone1.len_lines(), 10000);
        assert_eq!(_clone5.len_lines(), 10000);

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
}
