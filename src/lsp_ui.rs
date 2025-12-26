//! LSP UI Integration
//!
//! Provides UI components for LSP features like completion, diagnostics, and hover.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use crate::common::async_bridge::TauriBridge;
use crate::canvas_renderer::Position;

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
            completion_cache: RwSignal::new(Vec::new()),
            diagnostics: RwSignal::new(Vec::new()),
            hover_info: RwSignal::new(None),
        }
    }

    /// Set the current file path
    pub fn set_file_path(&self, path: String) {
        self.file_path.set(path);
    }

    /// Request completions at a specific position
    pub async fn request_completions(&self, position: Position) -> anyhow::Result<Vec<CompletionItem>> {
        #[derive(Serialize)]
        struct CompletionRequest {
            file_path: String,
            line: u32,
            character: u32,
        }

        let request = CompletionRequest {
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
        #[derive(Serialize)]
        struct HoverRequest {
            file_path: String,
            line: u32,
            character: u32,
        }

        let request = HoverRequest {
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
        #[derive(Serialize)]
        struct DiagnosticsRequest {
            file_path: String,
        }

        let request = DiagnosticsRequest {
            file_path: self.file_path.get_untracked(),
        };

        let diagnostics: Vec<Diagnostic> = TauriBridge::invoke("lsp_get_diagnostics", request).await?;

        // Update cache
        self.diagnostics.set(diagnostics.clone());

        Ok(diagnostics)
    }

    /// Go to definition at a specific position
    pub async fn goto_definition(&self, position: Position) -> anyhow::Result<Position> {
        #[derive(Serialize)]
        struct DefinitionRequest {
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
            file_path: self.file_path.get_untracked(),
            line: position.line as u32,
            character: position.column as u32,
        };

        let location: Location = TauriBridge::invoke("lsp_goto_definition", request).await?;

        Ok(Position::new(location.line as usize, location.character as usize))
    }

    /// Find all references at a specific position
    pub async fn find_references(&self, position: Position) -> anyhow::Result<Vec<Position>> {
        #[derive(Serialize)]
        struct ReferencesRequest {
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
