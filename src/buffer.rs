//! Text Buffer Implementation using Ropey

use ropey::Rope;

#[derive(Clone)]
pub struct TextBuffer {
    rope: Rope,
    file_path: Option<String>,
    modified: bool,
    language: String,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            file_path: None,
            modified: false,
            language: String::from("plaintext"),
        }
    }

    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            file_path: None,
            modified: false,
            language: String::from("plaintext"),
        }
    }

    pub fn set_file_path(&mut self, path: String) {
        self.file_path = Some(path);
    }

    pub fn set_language(&mut self, lang: String) {
        self.language = lang;
    }

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        self.rope.insert(char_idx, text);
        self.modified = true;
    }

    pub fn remove(&mut self, start: usize, end: usize) {
        self.rope.remove(start..end);
        self.modified = true;
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
