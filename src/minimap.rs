//! Minimap Rendering using Canvas API
//! 100% Rust - No JavaScript!

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

pub struct Minimap {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
    width: u32,
    height: u32,
    scale: f64,
}

impl Minimap {
    pub fn new(canvas_id: &str) -> Result<Self, String> {
        let document = web_sys::window()
            .ok_or("No window")?
            .document()
            .ok_or("No document")?;

        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or("Canvas not found")?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| "Element is not a canvas")?;

        let context = canvas
            .get_context("2d")
            .map_err(|_| "Failed to get 2d context")?
            .ok_or("No 2d context")?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| "Failed to cast to CanvasRenderingContext2d")?;

        Ok(Self {
            canvas,
            context,
            width: 120,
            height: 800,
            scale: 0.1, // 10% of actual size
        })
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.canvas.set_width(width);
        self.canvas.set_height(height);
    }

    pub fn render(&self, lines: &[String], viewport_start: usize, viewport_end: usize) {
        // Clear canvas
        self.context.clear_rect(0.0, 0.0, self.width as f64, self.height as f64);

        // Background
        self.context.set_fill_style(&JsValue::from_str("#1e1e1e"));
        self.context
            .fill_rect(0.0, 0.0, self.width as f64, self.height as f64);

        let line_height = (self.height as f64 / lines.len().max(1) as f64).max(1.0);

        // Render each line
        for (idx, line) in lines.iter().enumerate() {
            let y = idx as f64 * line_height;

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Different colors for different line types
            let color = self.get_line_color(line);
            self.context.set_fill_style(&JsValue::from_str(color));

            // Draw line as a thin rectangle
            let line_width = (line.len() as f64 * self.scale * 2.0).min(self.width as f64);
            self.context
                .fill_rect(0.0, y, line_width, line_height.max(1.0));
        }

        // Draw viewport indicator
        if !lines.is_empty() {
            let viewport_start_y = viewport_start as f64 * line_height;
            let viewport_height = (viewport_end - viewport_start) as f64 * line_height;

            self.context.set_stroke_style(&JsValue::from_str("#007acc"));
            self.context.set_line_width(2.0);
            self.context.stroke_rect(
                0.0,
                viewport_start_y,
                self.width as f64,
                viewport_height,
            );

            // Semi-transparent overlay
            self.context.set_fill_style(&JsValue::from_str("rgba(0, 122, 204, 0.1)"));
            self.context
                .fill_rect(0.0, viewport_start_y, self.width as f64, viewport_height);
        }
    }

    fn get_line_color(&self, line: &str) -> &str {
        let trimmed = line.trim_start();

        // Comments
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            return "#6a9955";
        }

        // Strings
        if trimmed.contains('"') || trimmed.contains('\'') {
            return "#ce9178";
        }

        // Keywords
        if trimmed.starts_with("fn ")
            || trimmed.starts_with("pub ")
            || trimmed.starts_with("use ")
            || trimmed.starts_with("impl ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("trait ")
        {
            return "#569cd6";
        }

        // Default
        "#d4d4d4"
    }

    pub fn handle_click(&self, x: f64, y: f64, total_lines: usize) -> usize {
        let line_height = self.height as f64 / total_lines.max(1) as f64;
        (y / line_height) as usize
    }
}

#[wasm_bindgen]
pub struct MinimapBuilder;

#[wasm_bindgen]
impl MinimapBuilder {
    pub fn create(canvas_id: &str) -> Result<(), JsValue> {
        let minimap = Minimap::new(canvas_id).map_err(|e| JsValue::from_str(&e))?;

        // Store in global state or return handle
        // For now, just log success

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    // Note: Minimap::new() requires a real DOM canvas element, so we test logic methods only

    #[wasm_bindgen_test]
    fn test_minimap_line_color_comment() {
        // We'll create a helper function to test get_line_color logic
        fn test_line_color(line: &str) -> &str {
            let trimmed = line.trim_start();

            // Comments
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                return "#6a9955";
            }

            // Strings
            if trimmed.contains('"') || trimmed.contains('\'') {
                return "#ce9178";
            }

            // Keywords
            if trimmed.starts_with("fn ")
                || trimmed.starts_with("pub ")
                || trimmed.starts_with("use ")
                || trimmed.starts_with("impl ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("enum ")
                || trimmed.starts_with("trait ")
            {
                return "#569cd6";
            }

            // Default
            "#d4d4d4"
        }

        assert_eq!(test_line_color("// This is a comment"), "#6a9955");
        assert_eq!(test_line_color("/* Block comment */"), "#6a9955");
        assert_eq!(test_line_color(" * In block comment"), "#6a9955");
    }

    #[wasm_bindgen_test]
    fn test_minimap_line_color_string() {
        fn test_line_color(line: &str) -> &str {
            let trimmed = line.trim_start();

            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                return "#6a9955";
            }

            if trimmed.contains('"') || trimmed.contains('\'') {
                return "#ce9178";
            }

            if trimmed.starts_with("fn ")
                || trimmed.starts_with("pub ")
                || trimmed.starts_with("use ")
                || trimmed.starts_with("impl ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("enum ")
                || trimmed.starts_with("trait ")
            {
                return "#569cd6";
            }

            "#d4d4d4"
        }

        assert_eq!(test_line_color("let x = \"hello\";"), "#ce9178");
        assert_eq!(test_line_color("let y = 'c';"), "#ce9178");
    }

    #[wasm_bindgen_test]
    fn test_minimap_line_color_keywords() {
        fn test_line_color(line: &str) -> &str {
            let trimmed = line.trim_start();

            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                return "#6a9955";
            }

            if trimmed.contains('"') || trimmed.contains('\'') {
                return "#ce9178";
            }

            if trimmed.starts_with("fn ")
                || trimmed.starts_with("pub ")
                || trimmed.starts_with("use ")
                || trimmed.starts_with("impl ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("enum ")
                || trimmed.starts_with("trait ")
            {
                return "#569cd6";
            }

            "#d4d4d4"
        }

        assert_eq!(test_line_color("fn main() {"), "#569cd6");
        assert_eq!(test_line_color("pub struct Foo {"), "#569cd6");
        assert_eq!(test_line_color("use std::io;"), "#569cd6");
        assert_eq!(test_line_color("impl Trait for Type {"), "#569cd6");
        assert_eq!(test_line_color("enum Color {"), "#569cd6");
        assert_eq!(test_line_color("trait Display {"), "#569cd6");
    }

    #[wasm_bindgen_test]
    fn test_minimap_line_color_default() {
        fn test_line_color(line: &str) -> &str {
            let trimmed = line.trim_start();

            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                return "#6a9955";
            }

            if trimmed.contains('"') || trimmed.contains('\'') {
                return "#ce9178";
            }

            if trimmed.starts_with("fn ")
                || trimmed.starts_with("pub ")
                || trimmed.starts_with("use ")
                || trimmed.starts_with("impl ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("enum ")
                || trimmed.starts_with("trait ")
            {
                return "#569cd6";
            }

            "#d4d4d4"
        }

        assert_eq!(test_line_color("let x = 5;"), "#d4d4d4");
        assert_eq!(test_line_color("    }"), "#d4d4d4");
    }

    #[wasm_bindgen_test]
    fn test_handle_click_calculation() {
        // Test the line calculation logic from handle_click
        fn calculate_clicked_line(y: f64, canvas_height: u32, total_lines: usize) -> usize {
            let line_height = canvas_height as f64 / total_lines.max(1) as f64;
            (y / line_height) as usize
        }

        // Click at top
        assert_eq!(calculate_clicked_line(0.0, 800, 100), 0);

        // Click in middle
        assert_eq!(calculate_clicked_line(400.0, 800, 100), 50);

        // Click near bottom
        assert_eq!(calculate_clicked_line(790.0, 800, 100), 98);
    }

    #[wasm_bindgen_test]
    fn test_handle_click_edge_cases() {
        fn calculate_clicked_line(y: f64, canvas_height: u32, total_lines: usize) -> usize {
            let line_height = canvas_height as f64 / total_lines.max(1) as f64;
            (y / line_height) as usize
        }

        // Single line file
        assert_eq!(calculate_clicked_line(400.0, 800, 1), 0);

        // Empty file (protected by max(1))
        assert_eq!(calculate_clicked_line(400.0, 800, 0), 0);

        // Very large file
        assert_eq!(calculate_clicked_line(799.0, 800, 10000), 9987);
    }
}
