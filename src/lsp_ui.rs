//! LSP UI Integration
//!
//! Provides UI components for LSP features like completion, diagnostics, and hover.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use crate::common::async_bridge::TauriBridge;
use crate::types::Position;

/// LSP Completion Item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: Option<u32>,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
}

/// LSP Diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: DiagnosticRange,
    pub severity: u32,
    pub message: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticRange {
    pub start: DiagnosticPosition,
    pub end: DiagnosticPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticPosition {
    pub line: u32,
    pub character: u32,
}

/// LSP Hover Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverInfo {
    pub contents: String,
    pub range: Option<DiagnosticRange>,
}

/// LSP Integration Manager
#[derive(Clone, Copy)]
pub struct LspIntegration {
    /// Current file path
    file_path: RwSignal<String>,
    /// Current language
    language: RwSignal<String>,
    /// LSP initialized flag
    initialized: RwSignal<bool>,
    /// Completion items cache
    completion_cache: RwSignal<Vec<CompletionItem>>,
    /// Diagnostics cache
    diagnostics: RwSignal<Vec<Diagnostic>>,
    /// Hover information cache
    hover_info: RwSignal<Option<HoverInfo>>,
}

impl LspIntegration {
    /// Create a new LSP integration
    pub fn new() -> Self {
        Self {
            file_path: RwSignal::new(String::new()),
            language: RwSignal::new(String::from("rust")),
            initialized: RwSignal::new(false),
            completion_cache: RwSignal::new(Vec::new()),
            diagnostics: RwSignal::new(Vec::new()),
            hover_info: RwSignal::new(None),
        }
    }

    /// Detect language from file extension
    fn detect_language(file_path: &str) -> String {
        if file_path.ends_with(".rs") {
            "rust".to_string()
        } else if file_path.ends_with(".ts") || file_path.ends_with(".tsx") {
            "typescript".to_string()
        } else if file_path.ends_with(".js") || file_path.ends_with(".jsx") {
            "javascript".to_string()
        } else if file_path.ends_with(".py") {
            "python".to_string()
        } else {
            "rust".to_string() // Default to Rust
        }
    }

    /// Initialize LSP server for the current file
    pub async fn initialize(&self, file_path: String, root_uri: String) -> anyhow::Result<()> {
        let language = Self::detect_language(&file_path);

        #[derive(Serialize)]
        struct InitRequest {
            language: String,
            root_uri: String,
        }

        let request = InitRequest {
            language: language.clone(),
            root_uri,
        };

        let _result: bool = TauriBridge::invoke("lsp_initialize", request).await?;

        self.language.set(language);
        self.file_path.set(file_path);
        self.initialized.set(true);

        Ok(())
    }

    /// Set the current file path
    pub fn set_file_path(&self, path: String) {
        self.file_path.set(path.clone());
        let language = Self::detect_language(&path);
        self.language.set(language);
    }

    /// Request completions at a specific position
    pub async fn request_completions(&self, position: Position) -> anyhow::Result<Vec<CompletionItem>> {
        if !self.initialized.get_untracked() {
            return Ok(Vec::new()); // Not initialized, return empty
        }

        #[derive(Serialize)]
        struct CompletionRequest {
            language: String,
            file_path: String,
            line: u32,
            character: u32,
        }

        let request = CompletionRequest {
            language: self.language.get_untracked(),
            file_path: self.file_path.get_untracked(),
            line: position.line as u32,
            character: position.column as u32,
        };

        let items: Vec<CompletionItem> = TauriBridge::invoke("lsp_get_completions", request).await?;

        // Update cache
        self.completion_cache.set(items.clone());

        Ok(items)
    }

    /// Request hover information at a specific position
    pub async fn request_hover(&self, position: Position) -> anyhow::Result<Option<HoverInfo>> {
        if !self.initialized.get_untracked() {
            return Ok(None); // Not initialized, return empty
        }

        #[derive(Serialize)]
        struct HoverRequest {
            language: String,
            file_path: String,
            line: u32,
            character: u32,
        }

        let request = HoverRequest {
            language: self.language.get_untracked(),
            file_path: self.file_path.get_untracked(),
            line: position.line as u32,
            character: position.column as u32,
        };

        let info: Option<HoverInfo> = TauriBridge::invoke("lsp_hover", request).await?;

        // Update cache
        self.hover_info.set(info.clone());

        Ok(info)
    }

    /// Request diagnostics for the current file
    pub async fn request_diagnostics(&self) -> anyhow::Result<Vec<Diagnostic>> {
        if !self.initialized.get_untracked() {
            return Ok(Vec::new()); // Not initialized, return empty
        }

        #[derive(Serialize)]
        struct DiagnosticsRequest {
            language: String,
            file_path: String,
        }

        let request = DiagnosticsRequest {
            language: self.language.get_untracked(),
            file_path: self.file_path.get_untracked(),
        };

        let diagnostics: Vec<Diagnostic> = TauriBridge::invoke("lsp_get_diagnostics", request).await?;

        // Update cache
        self.diagnostics.set(diagnostics.clone());

        Ok(diagnostics)
    }

    /// Go to definition at a specific position
    pub async fn goto_definition(&self, position: Position) -> anyhow::Result<Position> {
        if !self.initialized.get_untracked() {
            return Err(anyhow::anyhow!("LSP not initialized"));
        }

        #[derive(Serialize)]
        struct DefinitionRequest {
            language: String,
            file_path: String,
            line: u32,
            character: u32,
        }

        #[derive(Deserialize)]
        struct Location {
            line: u32,
            character: u32,
        }

        let request = DefinitionRequest {
            language: self.language.get_untracked(),
            file_path: self.file_path.get_untracked(),
            line: position.line as u32,
            character: position.column as u32,
        };

        let location: Location = TauriBridge::invoke("lsp_goto_definition", request).await?;

        Ok(Position::new(location.line as usize, location.character as usize))
    }

    /// Find all references at a specific position
    pub async fn find_references(&self, position: Position) -> anyhow::Result<Vec<Position>> {
        if !self.initialized.get_untracked() {
            return Ok(Vec::new()); // Not initialized, return empty
        }

        #[derive(Serialize)]
        struct ReferencesRequest {
            language: String,
            file_path: String,
            line: u32,
            character: u32,
        }

        #[derive(Deserialize)]
        struct Location {
            line: u32,
            character: u32,
        }

        let request = ReferencesRequest {
            language: self.language.get_untracked(),
            file_path: self.file_path.get_untracked(),
            line: position.line as u32,
            character: position.column as u32,
        };

        let locations: Vec<Location> = TauriBridge::invoke("lsp_find_references", request).await?;

        Ok(locations.into_iter().map(|loc| {
            Position::new(loc.line as usize, loc.character as usize)
        }).collect())
    }

    /// Get cached completions
    pub fn completions(&self) -> RwSignal<Vec<CompletionItem>> {
        self.completion_cache
    }

    /// Get cached diagnostics
    pub fn diagnostics(&self) -> RwSignal<Vec<Diagnostic>> {
        self.diagnostics
    }

    /// Get cached hover info
    pub fn hover_info(&self) -> RwSignal<Option<HoverInfo>> {
        self.hover_info
    }
}

impl Default for LspIntegration {
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
    fn test_lsp_integration_creation() {
        let lsp = LspIntegration::new();
        assert_eq!(lsp.file_path.get_untracked(), "");
        assert_eq!(lsp.completion_cache.get_untracked().len(), 0);
    }

    #[wasm_bindgen_test]
    fn test_set_file_path() {
        let lsp = LspIntegration::new();
        lsp.set_file_path("/path/to/file.rs".to_string());
        assert_eq!(lsp.file_path.get_untracked(), "/path/to/file.rs");
    }
}
