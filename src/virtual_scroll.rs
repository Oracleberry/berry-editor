//! Virtual Scrolling Implementation
//!
//! Renders only visible lines for optimal performance with large files.

/// Virtual scrolling manager for efficient rendering of large files
#[derive(Debug, Clone)]
pub struct VirtualScroll {
    /// Total number of lines in the document
    total_lines: usize,
    /// Height of the viewport in pixels
    viewport_height: f64,
    /// Height of each line in pixels
    line_height: f64,
    /// Current scroll position (top of viewport)
    scroll_top: f64,
    /// Cached visible range (start_line, end_line)
    visible_range: (usize, usize),
    /// Overscan lines (render extra lines above/below for smooth scrolling)
    overscan: usize,
}

impl VirtualScroll {
    /// Create a new virtual scroll manager
    pub fn new(total_lines: usize, viewport_height: f64, line_height: f64) -> Self {
        let mut vs = Self {
            total_lines,
            viewport_height,
            line_height,
            scroll_top: 0.0,
            visible_range: (0, 0),
            overscan: 5,
        };
        vs.calculate_visible_range();
        vs
    }

    /// Update the scroll position and recalculate visible range
    pub fn set_scroll_top(&mut self, scroll_top: f64) {
        self.scroll_top = scroll_top.max(0.0);
        self.calculate_visible_range();
    }

    /// Update the viewport height
    pub fn set_viewport_height(&mut self, viewport_height: f64) {
        self.viewport_height = viewport_height;
        self.calculate_visible_range();
    }

    /// Update the total number of lines
    pub fn set_total_lines(&mut self, total_lines: usize) {
        self.total_lines = total_lines;
        self.calculate_visible_range();
    }

    /// Calculate the visible range of lines
    fn calculate_visible_range(&mut self) {
        if self.total_lines == 0 {
            self.visible_range = (0, 0);
            return;
        }

        // Calculate first visible line
        let first_visible = (self.scroll_top / self.line_height).floor() as usize;

        // Calculate last visible line
        let visible_lines = (self.viewport_height / self.line_height).ceil() as usize;
        let last_visible = (first_visible + visible_lines).min(self.total_lines);

        // Apply overscan
        let start = first_visible.saturating_sub(self.overscan);
        let end = (last_visible + self.overscan).min(self.total_lines);

        self.visible_range = (start, end);
    }

    /// Get the current visible range (inclusive start, exclusive end)
    pub fn visible_range(&self) -> (usize, usize) {
        self.visible_range
    }

    /// Get the Y offset for a specific line number
    pub fn get_line_offset(&self, line: usize) -> f64 {
        line as f64 * self.line_height
    }

    /// Get the total scrollable height
    pub fn total_height(&self) -> f64 {
        self.total_lines as f64 * self.line_height
    }

    /// Check if a line is currently visible
    pub fn is_line_visible(&self, line: usize) -> bool {
        line >= self.visible_range.0 && line < self.visible_range.1
    }

    /// Get the line at a specific Y coordinate
    pub fn line_at_y(&self, y: f64) -> usize {
        let line = ((self.scroll_top + y) / self.line_height).floor() as usize;
        line.min(self.total_lines.saturating_sub(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_scroll_creation() {
        let vs = VirtualScroll::new(1000, 600.0, 20.0);
        assert_eq!(vs.total_lines, 1000);
        assert_eq!(vs.viewport_height, 600.0);
        assert_eq!(vs.line_height, 20.0);
    }

    #[test]
    fn test_visible_range_calculation() {
        let vs = VirtualScroll::new(1000, 600.0, 20.0);
        let (start, end) = vs.visible_range();

        // With overscan of 5, should render from line 0 to ~40
        // (600px / 20px = 30 visible lines, +5 overscan on each side)
        assert_eq!(start, 0);
        assert!(end > 30 && end <= 40);
    }

    #[test]
    fn test_scroll_position_update() {
        let mut vs = VirtualScroll::new(1000, 600.0, 20.0);

        // Scroll down by 200px (10 lines)
        vs.set_scroll_top(200.0);
        let (start, end) = vs.visible_range();

        // Should now start around line 5 (10 - 5 overscan)
        assert!(start >= 5 && start <= 10);
    }

    #[test]
    fn test_line_offset() {
        let vs = VirtualScroll::new(1000, 600.0, 20.0);

        assert_eq!(vs.get_line_offset(0), 0.0);
        assert_eq!(vs.get_line_offset(10), 200.0);
        assert_eq!(vs.get_line_offset(50), 1000.0);
    }

    #[test]
    fn test_total_height() {
        let vs = VirtualScroll::new(1000, 600.0, 20.0);
        assert_eq!(vs.total_height(), 20000.0);
    }

    #[test]
    fn test_is_line_visible() {
        let vs = VirtualScroll::new(1000, 600.0, 20.0);

        assert!(vs.is_line_visible(0));
        assert!(vs.is_line_visible(30));
        assert!(!vs.is_line_visible(500));
    }

    #[test]
    fn test_line_at_y() {
        let vs = VirtualScroll::new(1000, 600.0, 20.0);

        assert_eq!(vs.line_at_y(0.0), 0);
        assert_eq!(vs.line_at_y(20.0), 1);
        assert_eq!(vs.line_at_y(100.0), 5);
    }

    #[test]
    fn test_empty_document() {
        let vs = VirtualScroll::new(0, 600.0, 20.0);
        assert_eq!(vs.visible_range(), (0, 0));
        assert_eq!(vs.total_height(), 0.0);
    }

    #[test]
    fn test_viewport_resize() {
        let mut vs = VirtualScroll::new(1000, 600.0, 20.0);
        let (_, end1) = vs.visible_range();

        vs.set_viewport_height(1200.0);
        let (_, end2) = vs.visible_range();

        // Larger viewport should show more lines
        assert!(end2 > end1);
    }

    #[test]
    fn test_negative_scroll_clamping() {
        let mut vs = VirtualScroll::new(1000, 600.0, 20.0);
        vs.set_scroll_top(-100.0);

        // Should clamp to 0
        assert_eq!(vs.scroll_top, 0.0);
        assert_eq!(vs.visible_range().0, 0);
    }
}
