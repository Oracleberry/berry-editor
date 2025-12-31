//! RustRover (IntelliJ Darcula) Official Color Scheme
//!
//! This module defines the complete theme system for BerryEditor,
//! designed to perfectly match RustRover's Darcula theme.

/// Editor theme definition with all colors
#[derive(Debug, Clone, Copy)]
pub struct EditorTheme {
    // Backgrounds (The "Blacks")
    pub bg_main: &'static str,         // #2B2B2B (Window/Tabs)
    pub bg_editor: &'static str,       // #1E1E1E (Code Area)
    pub bg_sidebar: &'static str,      // #3C3F41 (Project Tree)
    pub bg_status_bar: &'static str,   // #2D2D30 (Bottom bar)

    // UI Elements
    pub border: &'static str,          // #323232 (Separator)
    pub scrollbar: &'static str,       // #4E5254 (Thumb color)

    // Text (The "Whites")
    pub text_default: &'static str,    // #A9B7C6 (Standard text)
    pub text_dim: &'static str,        // #858585 (Comments/Disabled)
    pub text_header: &'static str,     // #BBBBBB (Sidebar headers)

    // Selection & Focus
    pub cursor: &'static str,          // #BBBBBB (Caret)
    pub selection: &'static str,       // #214283 (Deep blue)
    pub line_numbers: &'static str,    // #606366 (Gutter text)
    pub caret_row: &'static str,       // #323232 (Active line highlight)

    // Syntax Highlighting (Darcula)
    pub syntax_keyword: &'static str,    // fn, pub, struct, let, mut
    pub syntax_function: &'static str,   // function names
    pub syntax_type: &'static str,       // String, usize, custom types
    pub syntax_string: &'static str,     // string literals
    pub syntax_number: &'static str,     // numeric literals
    pub syntax_comment: &'static str,    // comments
    pub syntax_attribute: &'static str,  // #[derive]
    pub syntax_macro: &'static str,      // macros!
    pub syntax_variable: &'static str,   // variable names
    pub syntax_constant: &'static str,   // CONSTANTS
    pub syntax_default: &'static str,    // default text color
}

/// RustRover Darcula theme - official colors
pub const RUSTROVER_DARCULA: EditorTheme = EditorTheme {
    // Backgrounds (The "Blacks")
    bg_main: "#2B2B2B",         // Window/Tabs
    bg_editor: "#1E1E1E",       // Code Area (darker for contrast)
    bg_sidebar: "#3C3F41",      // Project Tree
    bg_status_bar: "#2D2D30",   // Bottom bar

    // UI Elements
    border: "#323232",          // Separator
    scrollbar: "#4E5254",       // Thumb color

    // Text (The "Whites")
    text_default: "#A9B7C6",    // Standard text
    text_dim: "#858585",        // Comments/Disabled
    text_header: "#BBBBBB",     // Sidebar headers

    // Selection & Focus
    cursor: "#BBBBBB",          // Caret
    selection: "#214283",       // Deep blue
    line_numbers: "#606366",    // Gutter text
    caret_row: "#323232",       // Active line highlight

    // Syntax Highlighting (Darcula - accurate colors from RustRover)
    syntax_keyword: "#CC7832",      // Orange: fn, pub, struct, let, mut, if, else, match
    syntax_function: "#FFC66D",     // Yellow: function names
    syntax_type: "#A9B7C6",         // Light gray: String, usize, custom types
    syntax_string: "#6A8759",       // Green: string literals
    syntax_number: "#6897BB",       // Blue: numeric literals
    syntax_comment: "#629755",      // Green-gray: comments (Darcula standard)
    syntax_attribute: "#BBB529",    // Yellow-green: #[derive]
    syntax_macro: "#A9B7C6",        // Light gray: println!, vec!
    syntax_variable: "#A9B7C6",     // Light gray: local variables
    syntax_constant: "#9876AA",     // Purple: CONSTANTS
    syntax_default: "#A9B7C6",      // Light gray: default text
};

impl EditorTheme {
    /// Get the currently active theme
    pub fn current() -> &'static EditorTheme {
        &RUSTROVER_DARCULA
    }
}
