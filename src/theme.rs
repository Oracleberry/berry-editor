//! 💡 RustRover (Actual Screenshot) Pixel-Perfect Color Scheme
//!
//! This module defines the complete theme system for BerryEditor,
//! with colors extracted from actual RustRover screenshots for pixel-perfect accuracy.

/// Editor theme definition with all colors
#[derive(Debug, Clone, Copy)]
pub struct EditorTheme {
    // Backgrounds (The "Blacks")
    pub bg_main: &'static str,         // #2B2D30 (Window/Tabs/Non-active)
    pub bg_editor: &'static str,       // #1E1F22 (Code Area)
    pub bg_sidebar: &'static str,      // #2B2D30 (Project Tree)
    pub bg_tab_active: &'static str,   // #1E1F22 (Active tab)
    pub bg_status_bar: &'static str,   // #2B2D30 (Bottom bar)

    // UI Elements
    pub border: &'static str,          // #393B40 (Separator/Indent guides)
    pub scrollbar: &'static str,       // #4E5254 (Thumb color)

    // Text (The "Whites")
    pub text_default: &'static str,    // #BCBEC4 (Standard identifiers)
    pub text_dim: &'static str,        // #7A7E85 (Comments/Disabled)
    pub text_header: &'static str,     // #BBBBBB (Sidebar headers)

    // Selection & Focus
    pub cursor: &'static str,          // #BBBBBB (Caret)
    pub selection: &'static str,       // #214283 (Deep blue)
    pub line_numbers: &'static str,    // #4B5059 (Gutter text)
    pub caret_row: &'static str,       // #26282E (Active line highlight)

    // Syntax Highlighting (Pixel-Analyzed from Screenshot)
    pub syntax_keyword: &'static str,      // pub, struct, impl, fn
    pub syntax_function: &'static str,     // function names
    pub syntax_type: &'static str,         // TextBuffer, String, Option
    pub syntax_identifier: &'static str,   // rope, file_path (variables/fields)
    pub syntax_string: &'static str,       // "plaintext" (string literals)
    pub syntax_number: &'static str,       // 0, 42 (numeric literals)
    pub syntax_comment: &'static str,      // // comments
    pub syntax_doc_comment: &'static str,  // /// doc comments
    pub syntax_attribute: &'static str,    // #[derive(Clone)]
    pub syntax_macro: &'static str,        // println!, vec!
    pub syntax_constant: &'static str,     // CONSTANTS
}

/// RustRover theme - pixel-perfect colors from actual screenshot
pub const RUSTROVER_DARCULA: EditorTheme = EditorTheme {
    // Backgrounds (The "Blacks") - Pixel-analyzed
    bg_main: "#2B2D30",         // Window/Tabs/Non-active
    bg_editor: "#1E1F22",       // Code Area (slightly lighter than before)
    bg_sidebar: "#2B2D30",      // Project Tree (left sidebar)
    bg_tab_active: "#1E1F22",   // Active tab background
    bg_status_bar: "#2B2D30",   // Bottom status bar

    // UI Elements - Pixel-analyzed
    border: "#393B40",          // Separator/Indent guides (lighter than before)
    scrollbar: "#4E5254",       // Thumb color

    // Text (The "Whites") - Pixel-analyzed
    text_default: "#BCBEC4",    // Standard identifiers (whiter)
    text_dim: "#7A7E85",        // Comments/Disabled (darker)
    text_header: "#BBBBBB",     // Sidebar headers

    // Selection & Focus - Pixel-analyzed
    cursor: "#BBBBBB",          // Caret
    selection: "#214283",       // Deep blue (selection)
    line_numbers: "#4B5059",    // Gutter text (darker)
    caret_row: "#26282E",       // Active line highlight (darker)

    // Syntax Highlighting - Pixel-analyzed from screenshot
    syntax_keyword: "#CF8E6D",      // Orange: pub, struct, impl, fn
    syntax_function: "#FFC66D",     // Yellow: function names
    syntax_type: "#C77DBB",         // Purple-pink: TextBuffer, String, Option
    syntax_identifier: "#BCBEC4",   // White-ish: rope, file_path (variables/fields)
    syntax_string: "#6AAB73",       // Green: "plaintext" string literals
    syntax_number: "#2AACB8",       // Cyan-blue: 0, 42 numeric literals
    syntax_comment: "#7A7E85",      // Gray: // comments
    syntax_doc_comment: "#5F826B",  // Dark green: /// doc comments
    syntax_attribute: "#B3AE60",    // Yellow: #[derive(Clone)]
    syntax_macro: "#BCBEC4",        // White-ish: println!, vec!
    syntax_constant: "#9876AA",     // Purple: CONSTANTS
};

impl EditorTheme {
    /// Get the currently active theme
    pub fn current() -> &'static EditorTheme {
        &RUSTROVER_DARCULA
    }
}
