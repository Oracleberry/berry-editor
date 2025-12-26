//! LSP Feature Helpers
//! Higher-level wrappers for LSP features

use super::LspClientWasm;
use crate::lsp::{CompletionItem, Position};

/// Trigger completion based on current text and cursor
pub fn should_trigger_completion(text: &str, cursor_char: char) -> bool {
    // Trigger on dot, double colon, or after typing identifier chars
    matches!(cursor_char, '.' | ':') || text.len() >= 2
}

/// Filter completions based on partial input
pub fn filter_completions(items: Vec<CompletionItem>, partial: &str) -> Vec<CompletionItem> {
    if partial.is_empty() {
        return items;
    }

    let partial_lower = partial.to_lowercase();

    items
        .into_iter()
        .filter(|item| {
            item.label.to_lowercase().contains(&partial_lower)
                || item
                    .insert_text
                    .as_ref()
                    .map(|t| t.to_lowercase().contains(&partial_lower))
                    .unwrap_or(false)
        })
        .collect()
}

/// Sort completions by relevance
pub fn sort_completions_by_relevance(
    mut items: Vec<CompletionItem>,
    partial: &str,
) -> Vec<CompletionItem> {
    let partial_lower = partial.to_lowercase();

    items.sort_by(|a, b| {
        // Prioritize exact matches
        let a_exact = a.label.to_lowercase() == partial_lower;
        let b_exact = b.label.to_lowercase() == partial_lower;

        if a_exact && !b_exact {
            return std::cmp::Ordering::Less;
        }
        if !a_exact && b_exact {
            return std::cmp::Ordering::Greater;
        }

        // Then prefix matches
        let a_prefix = a.label.to_lowercase().starts_with(&partial_lower);
        let b_prefix = b.label.to_lowercase().starts_with(&partial_lower);

        if a_prefix && !b_prefix {
            return std::cmp::Ordering::Less;
        }
        if !a_prefix && b_prefix {
            return std::cmp::Ordering::Greater;
        }

        // Finally alphabetical
        a.label.cmp(&b.label)
    });

    items
}

/// Get word at position for hover
pub fn get_word_at_position(text: &str, position: Position) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let line = lines.get(position.line as usize)?;
    let chars: Vec<char> = line.chars().collect();

    if (position.character as usize) >= chars.len() {
        return None;
    }

    // Find word boundaries
    let mut start = position.character as usize;
    let mut end = position.character as usize;

    // Move back to start of word
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }

    // Move forward to end of word
    while end < chars.len() && is_word_char(chars[end]) {
        end += 1;
    }

    if start == end {
        return None;
    }

    Some(chars[start..end].iter().collect())
}

/// Check if character is part of word
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_trigger_completion() {
        assert!(should_trigger_completion("foo", '.'));
        assert!(should_trigger_completion("foo", ':'));
        assert!(should_trigger_completion("ab", 'c'));
        assert!(!should_trigger_completion("a", 'b'));
    }

    #[test]
    fn test_filter_completions() {
        let items = vec![
            CompletionItem {
                label: "println".to_string(),
                kind: Some(3),
                detail: None,
                documentation: None,
                insert_text: None,
            },
            CompletionItem {
                label: "print".to_string(),
                kind: Some(3),
                detail: None,
                documentation: None,
                insert_text: None,
            },
            CompletionItem {
                label: "format".to_string(),
                kind: Some(3),
                detail: None,
                documentation: None,
                insert_text: None,
            },
        ];

        let filtered = filter_completions(items, "print");
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|i| i.label == "println"));
        assert!(filtered.iter().any(|i| i.label == "print"));
    }

    #[test]
    fn test_sort_completions_by_relevance() {
        let items = vec![
            CompletionItem {
                label: "println".to_string(),
                kind: Some(3),
                detail: None,
                documentation: None,
                insert_text: None,
            },
            CompletionItem {
                label: "print".to_string(),
                kind: Some(3),
                detail: None,
                documentation: None,
                insert_text: None,
            },
            CompletionItem {
                label: "format".to_string(),
                kind: Some(3),
                detail: None,
                documentation: None,
                insert_text: None,
            },
        ];

        let sorted = sort_completions_by_relevance(items, "print");
        assert_eq!(sorted[0].label, "print"); // Exact match first
        assert_eq!(sorted[1].label, "println"); // Prefix match second
    }

    #[test]
    fn test_get_word_at_position() {
        let text = "fn main() { println!(\"Hello\"); }";
        let pos = Position {
            line: 0,
            character: 15,
        }; // Position in "println"

        let word = get_word_at_position(text, pos);
        assert_eq!(word, Some("println".to_string()));
    }

    #[test]
    fn test_is_word_char() {
        assert!(is_word_char('a'));
        assert!(is_word_char('Z'));
        assert!(is_word_char('0'));
        assert!(is_word_char('_'));
        assert!(!is_word_char('.'));
        assert!(!is_word_char(' '));
    }
}
