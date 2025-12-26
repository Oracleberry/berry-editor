//! LSP Client for WASM
//! Bindings to Tauri LSP commands

pub mod bindings;
pub mod features;

pub use bindings::*;
pub use features::*;

use serde::{Deserialize, Serialize};

/// Re-export types from main LSP module
pub use crate::lsp::{Position, Range, CompletionItem, Diagnostic, Location};

/// LSP client wrapper
pub struct LspClientWasm {
    language: String,
    initialized: bool,
}

impl LspClientWasm {
    pub fn new(language: &str) -> Self {
        Self {
            language: language.to_string(),
            initialized: false,
        }
    }

    /// Initialize LSP for this language
    pub async fn initialize(&mut self, root_uri: &str) -> Result<(), String> {
        lsp_initialize(self.language.clone(), root_uri.to_string()).await?;
        self.initialized = true;
        Ok(())
    }

    /// Get completions
    pub async fn completions(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<CompletionItem>, String> {
        if !self.initialized {
            return Err("LSP not initialized".to_string());
        }

        lsp_get_completions(
            self.language.clone(),
            file_path.to_string(),
            line,
            character,
        )
        .await
    }

    /// Get hover information
    pub async fn hover(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<HoverInfo>, String> {
        if !self.initialized {
            return Err("LSP not initialized".to_string());
        }

        lsp_get_hover(
            self.language.clone(),
            file_path.to_string(),
            line,
            character,
        )
        .await
    }

    /// Go to definition
    pub async fn goto_definition(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<Location>, String> {
        if !self.initialized {
            return Err("LSP not initialized".to_string());
        }

        lsp_goto_definition(
            self.language.clone(),
            file_path.to_string(),
            line,
            character,
        )
        .await
    }

    /// Shutdown LSP
    pub async fn shutdown(&mut self) -> Result<(), String> {
        if !self.initialized {
            return Ok(());
        }

        lsp_shutdown(self.language.clone()).await?;
        self.initialized = false;
        Ok(())
    }
}

/// Hover information (simplified from LSP Hover)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverInfo {
    pub contents: String,
    pub range: Option<Range>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_client_creation() {
        let client = LspClientWasm::new("rust");
        assert_eq!(client.language, "rust");
        assert!(!client.initialized);
    }
}
