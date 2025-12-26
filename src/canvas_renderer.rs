//! Canvas-based Text Renderer
//!
//! High-performance text rendering using HTML5 Canvas API.
//! Supports syntax highlighting, cursor, and selection rendering.

use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use wasm_bindgen::JsCast;
use anyhow::{Context, Result};

/// Position in the document (line, column)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Range in the document
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
}

/// Syntax token for highlighting
#[derive(Debug, Clone)]
pub struct SyntaxToken {
    pub start: usize,
    pub end: usize,
    pub color: String,
}

/// Canvas-based text renderer
pub struct CanvasTextRenderer {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
    font_size: f64,
    line_height: f64,
    char_width: f64,
    background_color: String,
    foreground_color: String,
    cursor_color: String,
    selection_color: String,
}

impl CanvasTextRenderer {
    /// Create a new canvas renderer
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self> {
        let context = canvas
            .get_context("2d")
            .map_err(|e| anyhow::anyhow!("Failed to get 2d context: {:?}", e))?
            .ok_or_else(|| anyhow::anyhow!("Context is None"))?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| anyhow::anyhow!("Failed to cast to CanvasRenderingContext2d"))?;

        let font_size = 14.0;
        let line_height = 20.0;

        // Measure character width (assuming monospace)
        context.set_font(&format!("{}px Menlo, Monaco, 'Courier New', monospace", font_size));
        // Approximate character width for 14px monospace font
        let char_width = 8.4;

        Ok(Self {
            canvas,
            context,
            font_size,
            line_height,
            char_width,
            background_color: "#1e1e1e".to_string(),
            foreground_color: "#d4d4d4".to_string(),
            cursor_color: "#aeafad".to_string(),
            selection_color: "rgba(51, 153, 255, 0.3)".to_string(),
        })
    }

    /// Clear the canvas
    pub fn clear(&self) {
        let width = self.canvas.width() as f64;
        let height = self.canvas.height() as f64;

        self.context.set_fill_style(&self.background_color.clone().into());
        self.context.fill_rect(0.0, 0.0, width, height);
    }

    /// Render a range of lines
    pub fn render_lines(
        &self,
        start_line: usize,
        lines: &[String],
        tokens: &[Vec<SyntaxToken>],
        y_offset: f64,
    ) {
        for (i, line_text) in lines.iter().enumerate() {
            let line_num = start_line + i;
            let y = y_offset + (line_num as f64 * self.line_height);

            // Get tokens for this line if available
            let line_tokens = tokens.get(i).map(|t| t.as_slice()).unwrap_or(&[]);

            self.render_line(line_text, line_tokens, 0.0, y);
        }
    }

    /// Render a single line of text
    fn render_line(&self, text: &str, tokens: &[SyntaxToken], x: f64, y: f64) {
        let baseline_y = y + self.line_height - 4.0;

        if tokens.is_empty() {
            // No syntax highlighting, render plain text
            self.context.set_fill_style(&self.foreground_color.clone().into());
            self.context.fill_text(text, x, baseline_y).ok();
        } else {
            // Render with syntax highlighting
            let mut last_end = 0;

            for token in tokens {
                // Render un-highlighted text before this token
                if token.start > last_end {
                    let plain = &text[last_end..token.start];
                    let token_x = x + (last_end as f64 * self.char_width);
                    self.context.set_fill_style(&self.foreground_color.clone().into());
                    self.context.fill_text(plain, token_x, baseline_y).ok();
                }

                // Render highlighted token
                let token_text = &text[token.start..token.end];
                let token_x = x + (token.start as f64 * self.char_width);
                self.context.set_fill_style(&token.color.clone().into());
                self.context.fill_text(token_text, token_x, baseline_y).ok();

                last_end = token.end;
            }

            // Render any remaining text
            if last_end < text.len() {
                let remaining = &text[last_end..];
                let token_x = x + (last_end as f64 * self.char_width);
                self.context.set_fill_style(&self.foreground_color.clone().into());
                self.context.fill_text(remaining, token_x, baseline_y).ok();
            }
        }
    }

    /// Render the cursor at a specific position
    pub fn render_cursor(&self, position: Position, y_offset: f64) {
        let x = position.column as f64 * self.char_width;
        let y = y_offset + (position.line as f64 * self.line_height);

        self.context.set_fill_style(&self.cursor_color.clone().into());
        self.context.fill_rect(x, y, 2.0, self.line_height);
    }

    /// Render selection range
    pub fn render_selection(&self, range: Range, y_offset: f64) {
        if range.start.line == range.end.line {
            // Single line selection
            self.render_selection_line(
                range.start.line,
                range.start.column,
                range.end.column,
                y_offset,
            );
        } else {
            // Multi-line selection
            // First line (from start to end of line)
            self.render_selection_line(
                range.start.line,
                range.start.column,
                usize::MAX, // To end of line
                y_offset,
            );

            // Middle lines (entire lines)
            for line in (range.start.line + 1)..range.end.line {
                self.render_selection_line(line, 0, usize::MAX, y_offset);
            }

            // Last line (from start of line to end column)
            self.render_selection_line(
                range.end.line,
                0,
                range.end.column,
                y_offset,
            );
        }
    }

    /// Render selection for a single line
    fn render_selection_line(&self, line: usize, start_col: usize, end_col: usize, y_offset: f64) {
        let x = start_col as f64 * self.char_width;
        let y = y_offset + (line as f64 * self.line_height);

        let width = if end_col == usize::MAX {
            // To end of canvas
            self.canvas.width() as f64 - x
        } else {
            (end_col - start_col) as f64 * self.char_width
        };

        self.context.set_fill_style(&self.selection_color.clone().into());
        self.context.fill_rect(x, y, width, self.line_height);
    }

    /// Get line height
    pub fn line_height(&self) -> f64 {
        self.line_height
    }

    /// Get character width
    pub fn char_width(&self) -> f64 {
        self.char_width
    }

    /// Get position from pixel coordinates
    pub fn position_from_point(&self, x: f64, y: f64, y_offset: f64) -> Position {
        let line = ((y + y_offset) / self.line_height).floor() as usize;
        let column = (x / self.char_width).round() as usize;

        Position::new(line, column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_position_creation() {
        let pos = Position::new(10, 5);
        assert_eq!(pos.line, 10);
        assert_eq!(pos.column, 5);
    }

    #[wasm_bindgen_test]
    fn test_range_creation() {
        let range = Range::new(
            Position::new(1, 0),
            Position::new(3, 10),
        );
        assert_eq!(range.start.line, 1);
        assert_eq!(range.end.line, 3);
    }

    #[wasm_bindgen_test]
    fn test_syntax_token() {
        let token = SyntaxToken {
            start: 0,
            end: 5,
            color: "#ff0000".to_string(),
        };
        assert_eq!(token.start, 0);
        assert_eq!(token.end, 5);
        assert_eq!(token.color, "#ff0000");
    }
}
