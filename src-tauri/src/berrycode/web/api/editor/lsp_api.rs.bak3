//! LSP (Language Server Protocol) API endpoints

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use lsp_types::{
    TextEdit, WorkspaceEdit, SymbolInformation,
    SemanticTokens, SemanticTokensResult, Range,
    InlayHint, CodeLens, Position, InlayHintLabel,
    CallHierarchyItem, CallHierarchyIncomingCall, CallHierarchyOutgoingCall,
    TypeHierarchyItem,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::berrycode::lsp_client::LspClient;

/// LSP API state
#[derive(Clone)]
pub struct LspApiState {
    /// LSP clients by session ID
    pub clients: Arc<Mutex<HashMap<String, LspClient>>>,
}

impl LspApiState {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get or create LSP client for a project
    pub async fn get_client(&self, session_id: &str, project_root: PathBuf) -> LspClient {
        let mut clients = self.clients.lock().await;

        if let Some(client) = clients.get(session_id) {
            // TODO: Clone is not ideal, but LspClient needs to be cloneable or use Arc
            // For now, create new client
            let client = LspClient::new(project_root.clone());
            clients.insert(session_id.to_string(), client.clone());
            client
        } else {
            let client = LspClient::new(project_root);
            clients.insert(session_id.to_string(), client.clone());
            client
        }
    }
}

impl Default for LspApiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Semantic tokens request
#[derive(Debug, Deserialize)]
pub struct SemanticTokensRequest {
    pub session_id: String,
    pub path: String,
}

/// Semantic tokens response
#[derive(Debug, Serialize)]
pub struct SemanticTokensResponse {
    pub success: bool,
    pub tokens: Option<SemanticTokens>,
    pub result_id: Option<String>,
}

/// Range formatting request
#[derive(Debug, Deserialize)]
pub struct RangeFormattingRequest {
    pub session_id: String,
    pub path: String,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

/// Range formatting response
#[derive(Debug, Serialize)]
pub struct RangeFormattingResponse {
    pub success: bool,
    pub edits: Vec<TextEditJson>,
    pub error: Option<String>,
}

/// Go to definition request
#[derive(Debug, Deserialize)]
pub struct GotoDefinitionRequest {
    pub session_id: String,
    pub path: String,
    pub line: u32,
    pub column: u32,
}

/// Find references request
#[derive(Debug, Deserialize)]
pub struct FindReferencesRequest {
    pub session_id: String,
    pub path: String,
    pub line: u32,
    pub column: u32,
}

/// Location response
#[derive(Debug, Serialize)]
pub struct LocationResponse {
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

/// Go to definition response
#[derive(Debug, Serialize)]
pub struct GotoDefinitionResponse {
    pub success: bool,
    pub location: Option<LocationResponse>,
    pub error: Option<String>,
}

/// Find references response
#[derive(Debug, Serialize)]
pub struct FindReferencesResponse {
    pub success: bool,
    pub locations: Vec<LocationResponse>,
    pub error: Option<String>,
}

/// Rename request
#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    pub session_id: String,
    pub path: String,
    pub line: u32,
    pub column: u32,
    pub old_name: String,
    pub new_name: String,
}

/// Rename response - workspace edit
#[derive(Debug, Serialize)]
pub struct RenameResponse {
    pub success: bool,
    pub edits: Vec<FileEdit>,
    pub error: Option<String>,
}

/// File edit
#[derive(Debug, Serialize)]
pub struct FileEdit {
    pub file_path: String,
    pub edits: Vec<TextEditJson>,
}

/// Text edit (JSON-serializable version of LSP TextEdit)
#[derive(Debug, Serialize)]
pub struct TextEditJson {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub new_text: String,
}

/// Handle rename request
pub async fn lsp_rename(
    State(state): State<LspApiState>,
    Json(request): Json<RenameRequest>,
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        path = %request.path,
        old_name = %request.old_name,
        new_name = %request.new_name,
        "LSP rename request"
    );

    // Get project root from current directory
    let project_root = std::env::current_dir().unwrap_or_default();

    // Convert path to absolute if it's relative
    let file_path = PathBuf::from(&request.path);
    let absolute_path = if file_path.is_absolute() {
        file_path
    } else {
        project_root.join(&file_path)
    };

    let client = state.get_client(&request.session_id, project_root).await;

    // Convert 1-based (editor) to 0-based (LSP)
    let line = request.line.saturating_sub(1);
    let column = request.column.saturating_sub(1);

    match client.rename(&absolute_path, line, column, &request.new_name) {
        Ok(Some(workspace_edit)) => {
            // Convert WorkspaceEdit to our response format
            let file_edits = convert_workspace_edit(workspace_edit);

            tracing::info!(
                session_id = %request.session_id,
                files_changed = file_edits.len(),
                "LSP rename successful"
            );

            (
                StatusCode::OK,
                Json(RenameResponse {
                    success: true,
                    edits: file_edits,
                    error: None,
                }),
            )
        }
        Ok(None) => {
            tracing::warn!(
                session_id = %request.session_id,
                "LSP rename returned no edits"
            );

            (
                StatusCode::OK,
                Json(RenameResponse {
                    success: false,
                    edits: Vec::new(),
                    error: Some("Symbol not found or cannot be renamed".to_string()),
                }),
            )
        }
        Err(e) => {
            tracing::error!(
                session_id = %request.session_id,
                error = %e,
                "LSP rename failed"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RenameResponse {
                    success: false,
                    edits: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

/// Handle goto definition request
pub async fn lsp_goto_definition(
    State(state): State<LspApiState>,
    Json(request): Json<GotoDefinitionRequest>,
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        path = %request.path,
        line = request.line,
        column = request.column,
        "LSP goto definition request"
    );

    // Get project root from current directory
    let project_root = std::env::current_dir().unwrap_or_default();

    // Convert path to absolute if it's relative
    let file_path = PathBuf::from(&request.path);
    let absolute_path = if file_path.is_absolute() {
        file_path
    } else {
        project_root.join(&file_path)
    };

    let client = state.get_client(&request.session_id, project_root).await;

    // Convert 1-based (editor) to 0-based (LSP)
    let line = request.line.saturating_sub(1);
    let column = request.column.saturating_sub(1);

    match client.goto_definition(&absolute_path, line, column) {
        Ok(Some(location)) => {
            let file_path = location.uri.to_file_path().ok();
            if let Some(path) = file_path {
                let response = LocationResponse {
                    file_path: path.to_string_lossy().to_string(),
                    line: location.range.start.line + 1, // Convert to 1-based
                    column: location.range.start.character + 1,
                    end_line: location.range.end.line + 1,
                    end_column: location.range.end.character + 1,
                };

                tracing::info!(
                    session_id = %request.session_id,
                    target_file = %response.file_path,
                    target_line = response.line,
                    "LSP goto definition successful"
                );

                return (
                    StatusCode::OK,
                    Json(GotoDefinitionResponse {
                        success: true,
                        location: Some(response),
                        error: None,
                    }),
                );
            }
        }
        Ok(None) => {
            tracing::warn!(
                session_id = %request.session_id,
                "LSP goto definition returned no location"
            );
        }
        Err(e) => {
            tracing::error!(
                session_id = %request.session_id,
                error = %e,
                "LSP goto definition failed"
            );

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GotoDefinitionResponse {
                    success: false,
                    location: None,
                    error: Some(e.to_string()),
                }),
            );
        }
    }

    (
        StatusCode::OK,
        Json(GotoDefinitionResponse {
            success: false,
            location: None,
            error: Some("Definition not found".to_string()),
        }),
    )
}

/// Handle semantic tokens request
pub async fn lsp_semantic_tokens(
    State(state): State<LspApiState>,
    Json(request): Json<SemanticTokensRequest>,
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        path = %request.path,
        "LSP semantic tokens request"
    );

    // Get project root from current directory
    let project_root = std::env::current_dir().unwrap_or_default();

    // Convert path to absolute if it's relative
    let file_path = PathBuf::from(&request.path);
    let absolute_path = if file_path.is_absolute() {
        file_path
    } else {
        project_root.join(&file_path)
    };

    let client = state.get_client(&request.session_id, project_root).await;

    match client.semantic_tokens(&absolute_path) {
        Ok(tokens) => {
            tracing::debug!(
                session_id = %request.session_id,
                "LSP semantic tokens retrieved"
            );

            (
                StatusCode::OK,
                Json(SemanticTokensResponse {
                    success: tokens.is_some(),
                    tokens,
                    result_id: Some(uuid::Uuid::new_v4().to_string()),
                }),
            )
        }
        Err(e) => {
            tracing::error!(
                session_id = %request.session_id,
                error = %e,
                "LSP semantic tokens failed"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SemanticTokensResponse {
                    success: false,
                    tokens: None,
                    result_id: None,
                }),
            )
        }
    }
}

/// Handle range formatting request
pub async fn lsp_range_formatting(
    State(state): State<LspApiState>,
    Json(request): Json<RangeFormattingRequest>,
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        path = %request.path,
        "LSP range formatting request"
    );

    // Get project root from current directory
    let project_root = std::env::current_dir().unwrap_or_default();

    // Convert path to absolute if it's relative
    let file_path = PathBuf::from(&request.path);
    let absolute_path = if file_path.is_absolute() {
        file_path
    } else {
        project_root.join(&file_path)
    };

    let client = state.get_client(&request.session_id, project_root).await;

    let range = Range {
        start: lsp_types::Position {
            line: request.start_line,
            character: request.start_column,
        },
        end: lsp_types::Position {
            line: request.end_line,
            character: request.end_column,
        },
    };

    match client.document_range_formatting(&absolute_path, range) {
        Ok(edits) => {
            let text_edits: Vec<TextEditJson> = edits
                .into_iter()
                .map(|edit: TextEdit| TextEditJson {
                    start_line: edit.range.start.line,
                    start_column: edit.range.start.character,
                    end_line: edit.range.end.line,
                    end_column: edit.range.end.character,
                    new_text: edit.new_text,
                })
                .collect();

            tracing::info!(
                session_id = %request.session_id,
                edits_count = text_edits.len(),
                "LSP range formatting successful"
            );

            (
                StatusCode::OK,
                Json(RangeFormattingResponse {
                    success: true,
                    edits: text_edits,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!(
                session_id = %request.session_id,
                error = %e,
                "LSP range formatting failed"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RangeFormattingResponse {
                    success: false,
                    edits: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

/// Handle find references request
pub async fn lsp_find_references(
    State(state): State<LspApiState>,
    Json(request): Json<FindReferencesRequest>,
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        path = %request.path,
        line = request.line,
        column = request.column,
        "LSP find references request"
    );

    // Get project root from current directory
    let project_root = std::env::current_dir().unwrap_or_default();

    // Convert path to absolute if it's relative
    let file_path = PathBuf::from(&request.path);
    let absolute_path = if file_path.is_absolute() {
        file_path
    } else {
        project_root.join(&file_path)
    };

    let client = state.get_client(&request.session_id, project_root).await;

    // Convert 1-based (editor) to 0-based (LSP)
    let line = request.line.saturating_sub(1);
    let column = request.column.saturating_sub(1);

    match client.find_references(&absolute_path, line, column) {
        Ok(locations) => {
            let responses: Vec<LocationResponse> = locations
                .into_iter()
                .filter_map(|location| {
                    let file_path = location.uri.to_file_path().ok()?;
                    Some(LocationResponse {
                        file_path: file_path.to_string_lossy().to_string(),
                        line: location.range.start.line + 1, // Convert to 1-based
                        column: location.range.start.character + 1,
                        end_line: location.range.end.line + 1,
                        end_column: location.range.end.character + 1,
                    })
                })
                .collect();

            tracing::info!(
                session_id = %request.session_id,
                references_found = responses.len(),
                "LSP find references successful"
            );

            (
                StatusCode::OK,
                Json(FindReferencesResponse {
                    success: true,
                    locations: responses,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!(
                session_id = %request.session_id,
                error = %e,
                "LSP find references failed"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(FindReferencesResponse {
                    success: false,
                    locations: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

/// Convert LSP WorkspaceEdit to our response format
fn convert_workspace_edit(workspace_edit: WorkspaceEdit) -> Vec<FileEdit> {
    let mut file_edits = Vec::new();

    if let Some(changes) = workspace_edit.changes {
        for (uri, edits) in changes {
            let file_path = uri.to_file_path().ok();
            if let Some(path) = file_path {
                let converted_edits: Vec<TextEditJson> = edits
                    .into_iter()
                    .map(|edit| TextEditJson {
                        start_line: edit.range.start.line,
                        start_column: edit.range.start.character,
                        end_line: edit.range.end.line,
                        end_column: edit.range.end.character,
                        new_text: edit.new_text,
                    })
                    .collect();

                file_edits.push(FileEdit {
                    file_path: path.to_string_lossy().to_string(),
                    edits: converted_edits,
                });
            }
        }
    }

    // TODO: Handle workspace_edit.document_changes if needed

    file_edits
}

/// Hover request
#[derive(Debug, Deserialize)]
pub struct HoverRequest {
    pub session_id: String,
    pub path: String,
    pub line: u32,
    pub column: u32,
}

/// Hover response
#[derive(Debug, Serialize)]
pub struct HoverResponse {
    pub success: bool,
    pub content: Option<String>,
    pub error: Option<String>,
}

/// Handle hover request
pub async fn lsp_hover(
    State(state): State<LspApiState>,
    Json(request): Json<HoverRequest>,
) -> impl IntoResponse {
    tracing::debug!(
        session_id = %request.session_id,
        path = %request.path,
        line = request.line,
        column = request.column,
        "LSP hover request"
    );

    // Get project root from current directory
    let project_root = std::env::current_dir().unwrap_or_default();

    // Convert path to absolute if it's relative
    let file_path = PathBuf::from(&request.path);
    let absolute_path = if file_path.is_absolute() {
        file_path
    } else {
        project_root.join(&file_path)
    };

    let client = state.get_client(&request.session_id, project_root).await;

    // Convert 1-based (editor) to 0-based (LSP)
    let line = request.line.saturating_sub(1);
    let column = request.column.saturating_sub(1);

    match client.hover(&absolute_path, line, column) {
        Ok(Some(content)) => {
            tracing::debug!(
                session_id = %request.session_id,
                "LSP hover successful"
            );

            (
                StatusCode::OK,
                Json(HoverResponse {
                    success: true,
                    content: Some(content),
                    error: None,
                }),
            )
        }
        Ok(None) => {
            tracing::debug!(
                session_id = %request.session_id,
                "LSP hover returned no content"
            );

            (
                StatusCode::OK,
                Json(HoverResponse {
                    success: true,
                    content: None,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!(
                session_id = %request.session_id,
                error = %e,
                "LSP hover failed"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HoverResponse {
                    success: false,
                    content: None,
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

/// Diagnostics request
#[derive(Debug, Deserialize)]
pub struct DiagnosticsRequest {
    pub session_id: String,
    pub path: String,
}

/// Diagnostic severity
#[derive(Debug, Serialize, Clone)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// Diagnostic message
#[derive(Debug, Serialize, Clone)]
pub struct DiagnosticMessage {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

/// Diagnostics response
#[derive(Debug, Serialize)]
pub struct DiagnosticsResponse {
    pub success: bool,
    pub diagnostics: Vec<DiagnosticMessage>,
    pub error: Option<String>,
}

/// Handle diagnostics request
pub async fn lsp_diagnostics(
    State(state): State<LspApiState>,
    query: Option<axum::extract::Query<DiagnosticsRequest>>,
    json: Option<Json<DiagnosticsRequest>>,
) -> impl IntoResponse {
    // Accept either query params or JSON body
    let request = match (query, json) {
        (Some(axum::extract::Query(req)), _) => req,
        (_, Some(Json(req))) => req,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(DiagnosticsResponse {
                    success: false,
                    diagnostics: Vec::new(),
                    error: Some("Missing session_id and path parameters".to_string()),
                }),
            );
        }
    };

    tracing::debug!(
        session_id = %request.session_id,
        path = %request.path,
        "LSP diagnostics request"
    );

    // Get project root from current directory
    let project_root = std::env::current_dir().unwrap_or_default();

    // Convert path to absolute if it's relative
    let file_path = PathBuf::from(&request.path);
    let absolute_path = if file_path.is_absolute() {
        file_path
    } else {
        project_root.join(&file_path)
    };

    let client = state.get_client(&request.session_id, project_root).await;

    match client.get_diagnostics(&absolute_path) {
        Ok(diagnostics) => {
            let diagnostic_messages: Vec<DiagnosticMessage> = diagnostics
                .into_iter()
                .map(|diag| {
                    let severity = match diag.severity {
                        Some(lsp_types::DiagnosticSeverity::ERROR) => DiagnosticSeverity::Error,
                        Some(lsp_types::DiagnosticSeverity::WARNING) => DiagnosticSeverity::Warning,
                        Some(lsp_types::DiagnosticSeverity::INFORMATION) => DiagnosticSeverity::Information,
                        Some(lsp_types::DiagnosticSeverity::HINT) => DiagnosticSeverity::Hint,
                        _ => DiagnosticSeverity::Error,
                    };

                    DiagnosticMessage {
                        severity,
                        message: diag.message,
                        start_line: diag.range.start.line,
                        start_column: diag.range.start.character,
                        end_line: diag.range.end.line,
                        end_column: diag.range.end.character,
                    }
                })
                .collect();

            tracing::debug!(
                session_id = %request.session_id,
                diagnostics_count = diagnostic_messages.len(),
                "LSP diagnostics retrieved"
            );

            (
                StatusCode::OK,
                Json(DiagnosticsResponse {
                    success: true,
                    diagnostics: diagnostic_messages,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!(
                session_id = %request.session_id,
                error = %e,
                "LSP diagnostics failed"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DiagnosticsResponse {
                    success: false,
                    diagnostics: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

/// Code actions request
#[derive(Debug, Deserialize)]
pub struct CodeActionsRequest {
    pub session_id: String,
    pub path: String,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

/// Code action response
#[derive(Debug, Serialize)]
pub struct CodeActionsResponse {
    pub success: bool,
    pub actions: Vec<CodeActionJson>,
    pub error: Option<String>,
}

/// Code action (JSON-serializable version)
#[derive(Debug, Serialize)]
pub struct CodeActionJson {
    pub title: String,
    pub kind: Option<String>,
    pub edit: Option<serde_json::Value>,
}

/// Handle code actions request
pub async fn lsp_code_actions(
    State(state): State<LspApiState>,
    Json(request): Json<CodeActionsRequest>,
) -> impl IntoResponse {
    use lsp_types::CodeActionOrCommand;

    // Get project root from current directory
    let project_root = std::env::current_dir().unwrap_or_default();

    // Convert path to absolute if it's relative
    let file_path = PathBuf::from(&request.path);
    let absolute_path = if file_path.is_absolute() {
        file_path
    } else {
        project_root.join(&file_path)
    };

    let client = state.get_client(&request.session_id, project_root).await;
    let diagnostics = client.get_diagnostics(&absolute_path).unwrap_or_default();

    match client.code_actions(
        &absolute_path,
        request.start_line.saturating_sub(1),
        request.start_column.saturating_sub(1),
        request.end_line.saturating_sub(1),
        request.end_column.saturating_sub(1),
        diagnostics
    ) {
        Ok(actions) => {
            let action_jsons: Vec<CodeActionJson> = actions
                .into_iter()
                .filter_map(|action| match action {
                    CodeActionOrCommand::CodeAction(ca) => Some(CodeActionJson {
                        title: ca.title,
                        kind: ca.kind.map(|k| format!("{:?}", k)),
                        edit: ca.edit.and_then(|e| serde_json::to_value(e).ok()),
                    }),
                    CodeActionOrCommand::Command(_) => None,
                })
                .collect();

            (StatusCode::OK, Json(CodeActionsResponse {
                success: true,
                actions: action_jsons,
                error: None,
            }))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CodeActionsResponse {
                success: false,
                actions: Vec::new(),
                error: Some(e.to_string()),
            }),
        ),
    }
}

/// Completion request
#[derive(Debug, Deserialize)]
pub struct CompletionRequest {
    pub session_id: String,
    pub path: String,
    pub line: u32,
    pub column: u32,
}

/// Completion response
#[derive(Debug, Serialize)]
pub struct CompletionResponse {
    pub success: bool,
    pub items: Vec<CompletionItemJson>,
    pub error: Option<String>,
}

/// Completion item
#[derive(Debug, Serialize)]
pub struct CompletionItemJson {
    pub label: String,
    pub kind: Option<u32>,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
}

/// Handle completion request
pub async fn lsp_completion(
    State(state): State<LspApiState>,
    Json(request): Json<CompletionRequest>,
) -> impl IntoResponse {
    use lsp_types::{Documentation, MarkupContent};

    // Get project root from current directory
    let project_root = std::env::current_dir().unwrap_or_default();

    // Convert path to absolute if it's relative
    let file_path = PathBuf::from(&request.path);
    let absolute_path = if file_path.is_absolute() {
        file_path
    } else {
        project_root.join(&file_path)
    };

    let client = state.get_client(&request.session_id, project_root).await;

    match client.completion(
        &absolute_path,
        request.line.saturating_sub(1),
        request.column.saturating_sub(1)
    ) {
        Ok(items) => {
            let item_jsons: Vec<CompletionItemJson> = items
                .into_iter()
                .map(|item| {
                    let documentation = match item.documentation {
                        Some(Documentation::String(s)) => Some(s),
                        Some(Documentation::MarkupContent(MarkupContent { value, .. })) => Some(value),
                        None => None,
                    };

                    CompletionItemJson {
                        label: item.label,
                        kind: item.kind.map(|k| match k {
                            lsp_types::CompletionItemKind::TEXT => 1,
                            lsp_types::CompletionItemKind::METHOD => 2,
                            lsp_types::CompletionItemKind::FUNCTION => 3,
                            lsp_types::CompletionItemKind::CONSTRUCTOR => 4,
                            lsp_types::CompletionItemKind::FIELD => 5,
                            lsp_types::CompletionItemKind::VARIABLE => 6,
                            lsp_types::CompletionItemKind::CLASS => 7,
                            lsp_types::CompletionItemKind::INTERFACE => 8,
                            lsp_types::CompletionItemKind::MODULE => 9,
                            lsp_types::CompletionItemKind::PROPERTY => 10,
                            lsp_types::CompletionItemKind::UNIT => 11,
                            lsp_types::CompletionItemKind::VALUE => 12,
                            lsp_types::CompletionItemKind::ENUM => 13,
                            lsp_types::CompletionItemKind::KEYWORD => 14,
                            lsp_types::CompletionItemKind::SNIPPET => 15,
                            lsp_types::CompletionItemKind::COLOR => 16,
                            lsp_types::CompletionItemKind::FILE => 17,
                            lsp_types::CompletionItemKind::REFERENCE => 18,
                            lsp_types::CompletionItemKind::FOLDER => 19,
                            lsp_types::CompletionItemKind::ENUM_MEMBER => 20,
                            lsp_types::CompletionItemKind::CONSTANT => 21,
                            lsp_types::CompletionItemKind::STRUCT => 22,
                            lsp_types::CompletionItemKind::EVENT => 23,
                            lsp_types::CompletionItemKind::OPERATOR => 24,
                            lsp_types::CompletionItemKind::TYPE_PARAMETER => 25,
                            _ => 1,
                        }),
                        detail: item.detail,
                        documentation,
                        insert_text: item.insert_text,
                    }
                })
                .collect();

            (StatusCode::OK, Json(CompletionResponse {
                success: true,
                items: item_jsons,
                error: None,
            }))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CompletionResponse {
                success: false,
                items: Vec::new(),
                error: Some(e.to_string()),
            }),
        ),
    }
}

/// Signature help request
#[derive(Debug, Deserialize)]
pub struct SignatureHelpRequest {
    pub session_id: String,
    pub path: String,
    pub line: u32,
    pub column: u32,
}

/// Signature help response
#[derive(Debug, Serialize)]
pub struct SignatureHelpResponse {
    pub success: bool,
    pub signatures: Vec<SignatureInformationJson>,
    pub active_signature: Option<u32>,
    pub active_parameter: Option<u32>,
    pub error: Option<String>,
}

/// Signature information
#[derive(Debug, Serialize)]
pub struct SignatureInformationJson {
    pub label: String,
    pub documentation: Option<String>,
    pub parameters: Vec<ParameterInformationJson>,
}

/// Parameter information
#[derive(Debug, Serialize)]
pub struct ParameterInformationJson {
    pub label: String,
    pub documentation: Option<String>,
}

/// Handle signature help request
pub async fn lsp_signature_help(
    State(state): State<LspApiState>,
    Json(request): Json<SignatureHelpRequest>,
) -> impl IntoResponse {
    use lsp_types::{Documentation, MarkupContent, ParameterLabel};

    // Get project root from current directory
    let project_root = std::env::current_dir().unwrap_or_default();

    // Convert path to absolute if it's relative
    let file_path = PathBuf::from(&request.path);
    let absolute_path = if file_path.is_absolute() {
        file_path
    } else {
        project_root.join(&file_path)
    };

    let client = state.get_client(&request.session_id, project_root).await;

    match client.signature_help(
        &absolute_path,
        request.line.saturating_sub(1),
        request.column.saturating_sub(1)
    ) {
        Ok(Some(sig_help)) => {
            let signatures: Vec<SignatureInformationJson> = sig_help
                .signatures
                .into_iter()
                .map(|sig| {
                    let documentation = match sig.documentation {
                        Some(Documentation::String(s)) => Some(s),
                        Some(Documentation::MarkupContent(MarkupContent { value, .. })) => Some(value),
                        None => None,
                    };

                    let parameters = sig
                        .parameters
                        .unwrap_or_default()
                        .into_iter()
                        .map(|param| {
                            let label = match param.label {
                                ParameterLabel::Simple(s) => s,
                                ParameterLabel::LabelOffsets([start, end]) => {
                                    sig.label[start as usize..end as usize].to_string()
                                }
                            };

                            let doc = match param.documentation {
                                Some(Documentation::String(s)) => Some(s),
                                Some(Documentation::MarkupContent(MarkupContent { value, .. })) => Some(value),
                                None => None,
                            };

                            ParameterInformationJson {
                                label,
                                documentation: doc,
                            }
                        })
                        .collect();

                    SignatureInformationJson {
                        label: sig.label,
                        documentation,
                        parameters,
                    }
                })
                .collect();

            (StatusCode::OK, Json(SignatureHelpResponse {
                success: true,
                signatures,
                active_signature: sig_help.active_signature,
                active_parameter: sig_help.active_parameter,
                error: None,
            }))
        }
        Ok(None) => (StatusCode::OK, Json(SignatureHelpResponse {
            success: true,
            signatures: Vec::new(),
            active_signature: None,
            active_parameter: None,
            error: None,
        })),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SignatureHelpResponse {
                success: false,
                signatures: Vec::new(),
                active_signature: None,
                active_parameter: None,
                error: Some(e.to_string()),
            }),
        ),
    }
}

// Document Symbols types
#[derive(Debug, Deserialize)]
pub struct DocumentSymbolsRequest {
    pub session_id: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
struct DocumentSymbolJson {
    name: String,
    detail: Option<String>,
    kind: u32,
    range: RangeJson,
    selection_range: RangeJson,
    children: Option<Vec<DocumentSymbolJson>>,
}

#[derive(Debug, Serialize, Clone)]
pub struct RangeJson {
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
}

#[derive(Debug, Deserialize)]
pub struct CallHierarchyPrepareRequest {
    pub session_id: String,
    pub path: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Serialize)]
pub struct CallHierarchyItemJson {
    pub name: String,
    pub detail: Option<String>,
    pub kind: u32,
    pub uri: String,
    pub range: RangeJson,
    pub selection_range: RangeJson,
}

#[derive(Debug, Serialize)]
pub struct CallHierarchyItemResponse {
    pub success: bool,
    pub items: Vec<CallHierarchyItemJson>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CallHierarchyCallsRequest {
    pub session_id: String,
    pub item: CallHierarchyItemJsonRequest,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CallHierarchyItemJsonRequest {
    pub name: String,
    pub detail: Option<String>,
    pub kind: u32,
    pub uri: String,
    pub range: RangeJsonRequest,
    pub selection_range: RangeJsonRequest,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RangeJsonRequest {
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
}

// Helper function to convert CallHierarchyItemJsonRequest to CallHierarchyItem
impl CallHierarchyItemJsonRequest {
    fn to_lsp_item(&self) -> Option<CallHierarchyItem> {
        let uri = lsp_types::Url::parse(&self.uri).ok()?;
        let kind = match self.kind {
            3 => lsp_types::SymbolKind::FUNCTION,
            6 => lsp_types::SymbolKind::METHOD,
            _ => lsp_types::SymbolKind::MODULE,
        };

        Some(CallHierarchyItem {
            name: self.name.clone(),
            kind,
            tags: None,
            detail: self.detail.clone(),
            uri,
            range: Range {
                start: Position {
                    line: self.range.start_line,
                    character: self.range.start_character,
                },
                end: Position {
                    line: self.range.end_line,
                    character: self.range.end_character,
                },
            },
            selection_range: Range {
                start: Position {
                    line: self.selection_range.start_line,
                    character: self.selection_range.start_character,
                },
                end: Position {
                    line: self.selection_range.end_line,
                    character: self.selection_range.end_character,
                },
            },
            data: None,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct TypeHierarchyPrepareRequest {
    pub session_id: String,
    pub path: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Serialize)]
pub struct TypeHierarchyItemJson {
    pub name: String,
    pub detail: Option<String>,
    pub kind: u32,
    pub uri: String,
    pub range: RangeJson,
    pub selection_range: RangeJson,
}

#[derive(Debug, Serialize)]
pub struct TypeHierarchyItemResponse {
    pub success: bool,
    pub items: Vec<TypeHierarchyItemJson>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TypeHierarchyItemsRequest {
    pub session_id: String,
    pub item: TypeHierarchyItemJsonRequest,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TypeHierarchyItemJsonRequest {
    pub name: String,
    pub detail: Option<String>,
    pub kind: u32,
    pub uri: String,
    pub range: RangeJsonRequest,
    pub selection_range: RangeJsonRequest,
}

// Helper function to convert TypeHierarchyItemJsonRequest to TypeHierarchyItem
impl TypeHierarchyItemJsonRequest {
    fn to_lsp_item(&self) -> Option<TypeHierarchyItem> {
        let uri = lsp_types::Url::parse(&self.uri).ok()?;
        let kind = match self.kind {
            5 => lsp_types::SymbolKind::CLASS,
            10 => lsp_types::SymbolKind::INTERFACE,
            11 => lsp_types::SymbolKind::STRUCT,
            _ => lsp_types::SymbolKind::MODULE,
        };

        Some(TypeHierarchyItem {
            name: self.name.clone(),
            kind,
            tags: None,
            detail: self.detail.clone(),
            uri,
            range: Range {
                start: Position {
                    line: self.range.start_line,
                    character: self.range.start_character,
                },
                end: Position {
                    line: self.range.end_line,
                    character: self.range.end_character,
                },
            },
            selection_range: Range {
                start: Position {
                    line: self.selection_range.start_line,
                    character: self.selection_range.start_character,
                },
                end: Position {
                    line: self.selection_range.end_line,
                    character: self.selection_range.end_character,
                },
            },
            data: None,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct DocumentSymbolsResponse {
    pub success: bool,
    pub symbols: Vec<DocumentSymbolJson>,
    pub error: Option<String>,
}

/// LSP document symbols endpoint
pub async fn lsp_document_symbols(
    State(state): State<LspApiState>,
    Json(request): Json<DocumentSymbolsRequest>,
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        path = %request.path,
        "LSP document symbols request"
    );

    // Get project root from current directory
    let project_root = std::env::current_dir().unwrap_or_default();

    // Convert path to absolute if it's relative
    let file_path = PathBuf::from(&request.path);
    let absolute_path = if file_path.is_absolute() {
        file_path
    } else {
        project_root.join(&file_path)
    };

    let client = state.get_client(&request.session_id, project_root).await;

    match client.document_symbols(&absolute_path) {
        Ok(symbols) => {
            fn convert_symbol(sym: lsp_types::DocumentSymbol) -> DocumentSymbolJson {
                // Convert SymbolKind enum to u32
                let kind_u32 = match sym.kind {
                    lsp_types::SymbolKind::FILE => 1,
                    lsp_types::SymbolKind::MODULE => 2,
                    lsp_types::SymbolKind::NAMESPACE => 3,
                    lsp_types::SymbolKind::PACKAGE => 4,
                    lsp_types::SymbolKind::CLASS => 5,
                    lsp_types::SymbolKind::METHOD => 6,
                    lsp_types::SymbolKind::PROPERTY => 7,
                    lsp_types::SymbolKind::FIELD => 8,
                    lsp_types::SymbolKind::CONSTRUCTOR => 9,
                    lsp_types::SymbolKind::ENUM => 10,
                    lsp_types::SymbolKind::INTERFACE => 11,
                    lsp_types::SymbolKind::FUNCTION => 12,
                    lsp_types::SymbolKind::VARIABLE => 13,
                    lsp_types::SymbolKind::CONSTANT => 14,
                    lsp_types::SymbolKind::STRING => 15,
                    lsp_types::SymbolKind::NUMBER => 16,
                    lsp_types::SymbolKind::BOOLEAN => 17,
                    lsp_types::SymbolKind::ARRAY => 18,
                    lsp_types::SymbolKind::OBJECT => 19,
                    lsp_types::SymbolKind::KEY => 20,
                    lsp_types::SymbolKind::NULL => 21,
                    lsp_types::SymbolKind::ENUM_MEMBER => 22,
                    lsp_types::SymbolKind::STRUCT => 23,
                    lsp_types::SymbolKind::EVENT => 24,
                    lsp_types::SymbolKind::OPERATOR => 25,
                    lsp_types::SymbolKind::TYPE_PARAMETER => 26,
                    _ => 1, // Default to FILE
                };

                DocumentSymbolJson {
                    name: sym.name,
                    detail: sym.detail,
                    kind: kind_u32,
                    range: RangeJson {
                        start_line: sym.range.start.line,
                        start_character: sym.range.start.character,
                        end_line: sym.range.end.line,
                        end_character: sym.range.end.character,
                    },
                    selection_range: RangeJson {
                        start_line: sym.selection_range.start.line,
                        start_character: sym.selection_range.start.character,
                        end_line: sym.selection_range.end.line,
                        end_character: sym.selection_range.end.character,
                    },
                    children: sym.children.map(|children| {
                        children.into_iter().map(convert_symbol).collect()
                    }),
                }
            }

            let symbol_jsons: Vec<DocumentSymbolJson> =
                symbols.into_iter().map(convert_symbol).collect();

            (
                StatusCode::OK,
                Json(DocumentSymbolsResponse {
                    success: true,
                    symbols: symbol_jsons,
                    error: None,
                }),
            )
        }
        Err(e) => {
            let error_msg = e.to_string();

            // If it's an unsupported file extension, return OK with empty symbols
            // (not a server error, just not supported)
            if error_msg.contains("Unsupported file extension") {
                tracing::debug!(
                    session_id = %request.session_id,
                    error = %e,
                    "LSP document symbols not supported for this file type"
                );
                (
                    StatusCode::OK,
                    Json(DocumentSymbolsResponse {
                        success: true,
                        symbols: Vec::new(),
                        error: None,
                    }),
                )
            } else {
                tracing::error!(
                    session_id = %request.session_id,
                    error = %e,
                    "LSP document symbols failed"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(DocumentSymbolsResponse {
                        success: false,
                        symbols: Vec::new(),
                        error: Some(error_msg),
                    }),
                )
            }
        }
    }
}

/// Inlay Hints request
#[derive(Debug, Deserialize)]
pub struct InlayHintsRequest {
    pub session_id: String,
    pub path: String,
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
}

/// Inlay Hints response
#[derive(Debug, Serialize)]
pub struct InlayHintsResponse {
    pub success: bool,
    pub hints: Vec<InlayHintJson>,
    pub error: Option<String>,
}

/// Inlay Hint JSON (serializable)
#[derive(Debug, Serialize)]
pub struct InlayHintJson {
    pub position: InlayHintPositionJson,
    pub label: String,
    pub kind: Option<String>,
    pub tooltip: Option<String>,
}

/// Inlay Hint Position JSON
#[derive(Debug, Serialize)]
pub struct InlayHintPositionJson {
    pub line: u32,
    pub character: u32,
}

/// Code Lens request
#[derive(Debug, Deserialize)]
pub struct CodeLensRequest {
    pub session_id: String,
    pub path: String,
}

/// Code Lens response
#[derive(Debug, Serialize)]
pub struct CodeLensResponse {
    pub success: bool,
    pub lenses: Vec<CodeLensJson>,
    pub error: Option<String>,
}

/// Code Lens JSON (serializable)
#[derive(Debug, Serialize)]
pub struct CodeLensJson {
    pub range: RangeJson,
    pub command: Option<CodeLensCommandJson>,
}

/// Code Lens Command JSON
#[derive(Debug, Serialize)]
pub struct CodeLensCommandJson {
    pub title: String,
    pub command: String,
    pub arguments: Option<Vec<serde_json::Value>>,
}

/// Handle Inlay Hints request
pub async fn lsp_inlay_hints(
    State(state): State<LspApiState>,
    Json(request): Json<InlayHintsRequest>,
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        path = %request.path,
        "LSP inlay hints request"
    );

    // Get project root from current directory
    let project_root = std::env::current_dir().unwrap_or_default();

    // Convert path to absolute if it's relative
    let file_path = PathBuf::from(&request.path);
    let absolute_path = if file_path.is_absolute() {
        file_path
    } else {
        project_root.join(&file_path)
    };

    let client = state.get_client(&request.session_id, project_root).await;

    // Prepare the Range for inlay hints
    let range = Range {
        start: Position {
            line: request.start_line,
            character: request.start_character,
        },
        end: Position {
            line: request.end_line,
            character: request.end_character,
        },
    };

    match client.inlay_hints(&absolute_path, range) {
        Ok(hints) => {
            let hint_jsons: Vec<InlayHintJson> = hints
                .into_iter()
                .map(|hint| InlayHintJson {
                    position: InlayHintPositionJson {
                        line: hint.position.line,
                        character: hint.position.character,
                    },
                    label: match &hint.label {
                        InlayHintLabel::String(s) => s.clone(),
                        InlayHintLabel::LabelParts(parts) => parts.iter()
                            .map(|p| p.value.clone())
                            .collect(),
                    },
                    kind: hint.kind.map(|k| match k {
                        lsp_types::InlayHintKind::TYPE => "type".to_string(),
                        lsp_types::InlayHintKind::PARAMETER => "parameter".to_string(),
                        _ => "other".to_string(),
                    }),
                    tooltip: hint.tooltip.map(|t| match t {
                        lsp_types::InlayHintTooltip::String(s) => s,
                        lsp_types::InlayHintTooltip::MarkupContent(mc) => mc.value,
                    }),
                })
                .collect();

            tracing::info!(
                session_id = %request.session_id,
                hints_count = hint_jsons.len(),
                "LSP inlay hints retrieved"
            );

            (
                StatusCode::OK,
                Json(InlayHintsResponse {
                    success: true,
                    hints: hint_jsons,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!(
                session_id = %request.session_id,
                error = %e,
                "LSP inlay hints failed"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(InlayHintsResponse {
                    success: false,
                    hints: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

/// Handle Code Lens request
pub async fn lsp_code_lens(
    State(state): State<LspApiState>,
    Json(request): Json<CodeLensRequest>,
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        path = %request.path,
        "LSP code lens request"
    );

    // Get project root from current directory
    let project_root = std::env::current_dir().unwrap_or_default();

    // Convert path to absolute if it's relative
    let file_path = PathBuf::from(&request.path);
    let absolute_path = if file_path.is_absolute() {
        file_path
    } else {
        project_root.join(&file_path)
    };

    let client = state.get_client(&request.session_id, project_root).await;

    match client.code_lens(&absolute_path) {
        Ok(lenses) => {
            let lens_jsons: Vec<CodeLensJson> = lenses
                .into_iter()
                .map(|lens| CodeLensJson {
                    range: RangeJson {
                        start_line: lens.range.start.line,
                        start_character: lens.range.start.character,
                        end_line: lens.range.end.line,
                        end_character: lens.range.end.character,
                    },
                    command: lens.command.map(|cmd| CodeLensCommandJson {
                        title: cmd.title,
                        command: cmd.command,
                        arguments: cmd.arguments,
                    }),
                })
                .collect();

            tracing::info!(
                session_id = %request.session_id,
                lenses_count = lens_jsons.len(),
                "LSP code lens retrieved"
            );

            (
                StatusCode::OK,
                Json(CodeLensResponse {
                    success: true,
                    lenses: lens_jsons,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!(
                session_id = %request.session_id,
                error = %e,
                "LSP code lens failed"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CodeLensResponse {
                    success: false,
                    lenses: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

/// Document formatting request
#[derive(Debug, Deserialize)]
pub struct DocumentFormattingRequest {
    pub session_id: String,
    pub path: String,
}

/// Document formatting response
#[derive(Debug, Serialize)]
pub struct DocumentFormattingResponse {
    pub success: bool,
    pub edits: Vec<TextEditJson>,
    pub error: Option<String>,
}

/// Workspace symbols request
#[derive(Debug, Deserialize)]
pub struct WorkspaceSymbolsRequest {
    pub session_id: String,
    pub query: String,
}

/// Workspace Symbols response
#[derive(Debug, Serialize)]
pub struct WorkspaceSymbolsResponse {
    pub success: bool,
    pub symbols: Vec<SymbolInformation>,
    pub error: Option<String>,
}

/// Handle document formatting request
pub async fn lsp_document_formatting(
    State(state): State<LspApiState>,
    Json(request): Json<DocumentFormattingRequest>,
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        path = %request.path,
        "LSP document formatting request"
    );

    // Get project root from current directory
    let project_root = std::env::current_dir().unwrap_or_default();

    // Convert path to absolute if it's relative
    let file_path = PathBuf::from(&request.path);
    let absolute_path = if file_path.is_absolute() {
        file_path
    } else {
        project_root.join(&file_path)
    };

    let client = state.get_client(&request.session_id, project_root).await;

    match client.document_formatting(&absolute_path) {
        Ok(edits) => {
            let text_edits: Vec<TextEditJson> = edits
                .into_iter()
                .map(|edit: TextEdit| TextEditJson {
                    start_line: edit.range.start.line,
                    start_column: edit.range.start.character,
                    end_line: edit.range.end.line,
                    end_column: edit.range.end.character,
                    new_text: edit.new_text,
                })
                .collect();

            tracing::info!(
                session_id = %request.session_id,
                edits_count = text_edits.len(),
                "LSP document formatting successful"
            );

            (
                StatusCode::OK,
                Json(DocumentFormattingResponse {
                    success: true,
                    edits: text_edits,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!(
                session_id = %request.session_id,
                error = %e,
                "LSP document formatting failed"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DocumentFormattingResponse {
                    success: false,
                    edits: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

/// Handle workspace symbols request
pub async fn lsp_workspace_symbols(
    State(state): State<LspApiState>,
    Json(request): Json<WorkspaceSymbolsRequest>,
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        query = %request.query,
        "LSP workspace symbols request"
    );

    // Get project roots for all opened projects
    let clients = state.clients.lock().await;
    let mut all_symbols = Vec::new();

    for (_, client) in clients.iter() {
        // For now, search in the project root. This is a placeholder implementation.
        // You would typically want to search across multiple files in the project.
        match client.workspace_symbols(&request.query) {
            Ok(mut symbols) => {
                all_symbols.append(&mut symbols);
            }
            Err(e) => {
                tracing::warn!(
                    session_id = %request.session_id,
                    error = %e,
                    "LSP workspace symbols partial failure"
                );
            }
        }
    }

    tracing::info!(
        session_id = %request.session_id,
        symbols_count = all_symbols.len(),
        "LSP workspace symbols search completed"
    );

    (
        StatusCode::OK,
        Json(WorkspaceSymbolsResponse {
            success: true,
            symbols: all_symbols,
            error: None,
        }),
    )
}

/// Handle Call Hierarchy Prepare request
pub async fn lsp_prepare_call_hierarchy(
    State(state): State<LspApiState>,
    Json(request): Json<CallHierarchyPrepareRequest>
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        path = %request.path,
        line = request.line,
        column = request.column,
        "LSP prepare call hierarchy request"
    );

    // Get project root from current directory
    let project_root = std::env::current_dir().unwrap_or_default();

    // Convert path to absolute if it's relative
    let file_path = PathBuf::from(&request.path);
    let absolute_path = if file_path.is_absolute() {
        file_path
    } else {
        project_root.join(&file_path)
    };

    let client = state.get_client(&request.session_id, project_root).await;

    // Convert 1-based (editor) to 0-based (LSP)
    let line = request.line.saturating_sub(1);
    let column = request.column.saturating_sub(1);

    match client.prepare_call_hierarchy(&absolute_path, line, column) {
        Ok(items) => {
            let serialized_items: Vec<CallHierarchyItemJson> = items
                .into_iter()
                .map(|item: CallHierarchyItem| CallHierarchyItemJson {
                    name: item.name,
                    detail: item.detail,
                    kind: match item.kind {
                        lsp_types::SymbolKind::FUNCTION => 3,
                        lsp_types::SymbolKind::METHOD => 6,
                        _ => 12,
                    },
                    uri: item.uri.to_string(),
                    range: RangeJson {
                        start_line: item.range.start.line,
                        start_character: item.range.start.character,
                        end_line: item.range.end.line,
                        end_character: item.range.end.character,
                    },
                    selection_range: RangeJson {
                        start_line: item.selection_range.start.line,
                        start_character: item.selection_range.start.character,
                        end_line: item.selection_range.end.line,
                        end_character: item.selection_range.end.character,
                    },
                })
                .collect();

            tracing::info!(
                session_id = %request.session_id,
                items_count = serialized_items.len(),
                "LSP call hierarchy prepare successful"
            );

            (
                StatusCode::OK,
                Json(CallHierarchyItemResponse {
                    success: true,
                    items: serialized_items,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!(
                session_id = %request.session_id,
                error = %e,
                "LSP call hierarchy prepare failed"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CallHierarchyItemResponse {
                    success: false,
                    items: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

/// Handle Call Hierarchy Incoming Calls request
pub async fn lsp_call_hierarchy_incoming_calls(
    State(state): State<LspApiState>,
    Json(request): Json<CallHierarchyCallsRequest>
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        item_name = %request.item.name,
        "LSP call hierarchy incoming calls request"
    );

    let project_root = lsp_types::Url::parse(&request.item.uri)
        .ok()
        .and_then(|url| url.to_file_path().ok())
        .and_then(|p| p.parent().map(|p: &std::path::Path| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let client = state.get_client(&request.session_id, project_root).await;

    let lsp_item = match request.item.to_lsp_item() {
        Some(item) => item,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(CallHierarchyItemResponse {
                    success: false,
                    items: vec![],
                    error: Some("Invalid item format".to_string()),
                }),
            );
        }
    };

    match client.call_hierarchy_incoming_calls(lsp_item) {
        Ok(incoming_calls) => {
            let serialized_items: Vec<CallHierarchyItemJson> = incoming_calls
                .into_iter()
                .map(|call: CallHierarchyIncomingCall| CallHierarchyItemJson {
                    name: call.from.name,
                    detail: call.from.detail,
                    kind: match call.from.kind {
                        lsp_types::SymbolKind::FUNCTION => 3,
                        lsp_types::SymbolKind::METHOD => 6,
                        _ => 12,
                    },
                    uri: call.from.uri.to_string(),
                    range: RangeJson {
                        start_line: call.from.range.start.line,
                        start_character: call.from.range.start.character,
                        end_line: call.from.range.end.line,
                        end_character: call.from.range.end.character,
                    },
                    selection_range: RangeJson {
                        start_line: call.from.selection_range.start.line,
                        start_character: call.from.selection_range.start.character,
                        end_line: call.from.selection_range.end.line,
                        end_character: call.from.selection_range.end.character,
                    },
                })
                .collect();

            tracing::info!(
                session_id = %request.session_id,
                items_count = serialized_items.len(),
                "LSP call hierarchy incoming calls successful"
            );

            (
                StatusCode::OK,
                Json(CallHierarchyItemResponse {
                    success: true,
                    items: serialized_items,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!(
                session_id = %request.session_id,
                error = %e,
                "LSP call hierarchy incoming calls failed"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CallHierarchyItemResponse {
                    success: false,
                    items: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

/// Handle Call Hierarchy Outgoing Calls request
pub async fn lsp_call_hierarchy_outgoing_calls(
    State(state): State<LspApiState>,
    Json(request): Json<CallHierarchyCallsRequest>
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        item_name = %request.item.name,
        "LSP call hierarchy outgoing calls request"
    );

    let project_root = lsp_types::Url::parse(&request.item.uri)
        .ok()
        .and_then(|url| url.to_file_path().ok())
        .and_then(|p| p.parent().map(|p: &std::path::Path| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let client = state.get_client(&request.session_id, project_root).await;

    let lsp_item = match request.item.to_lsp_item() {
        Some(item) => item,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(CallHierarchyItemResponse {
                    success: false,
                    items: vec![],
                    error: Some("Invalid item format".to_string()),
                }),
            );
        }
    };

    match client.call_hierarchy_outgoing_calls(lsp_item) {
        Ok(outgoing_calls) => {
            let serialized_items: Vec<CallHierarchyItemJson> = outgoing_calls
                .into_iter()
                .map(|call: CallHierarchyOutgoingCall| CallHierarchyItemJson {
                    name: call.to.name,
                    detail: call.to.detail,
                    kind: match call.to.kind {
                        lsp_types::SymbolKind::FUNCTION => 3,
                        lsp_types::SymbolKind::METHOD => 6,
                        _ => 12,
                    },
                    uri: call.to.uri.to_string(),
                    range: RangeJson {
                        start_line: call.to.range.start.line,
                        start_character: call.to.range.start.character,
                        end_line: call.to.range.end.line,
                        end_character: call.to.range.end.character,
                    },
                    selection_range: RangeJson {
                        start_line: call.to.selection_range.start.line,
                        start_character: call.to.selection_range.start.character,
                        end_line: call.to.selection_range.end.line,
                        end_character: call.to.selection_range.end.character,
                    },
                })
                .collect();

            tracing::info!(
                session_id = %request.session_id,
                items_count = serialized_items.len(),
                "LSP call hierarchy outgoing calls successful"
            );

            (
                StatusCode::OK,
                Json(CallHierarchyItemResponse {
                    success: true,
                    items: serialized_items,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!(
                session_id = %request.session_id,
                error = %e,
                "LSP call hierarchy outgoing calls failed"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CallHierarchyItemResponse {
                    success: false,
                    items: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

/// Handle Type Hierarchy Prepare request
pub async fn lsp_prepare_type_hierarchy(
    State(state): State<LspApiState>,
    Json(request): Json<TypeHierarchyPrepareRequest>
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        path = %request.path,
        line = request.line,
        column = request.column,
        "LSP prepare type hierarchy request"
    );

    // Get project root from current directory
    let project_root = std::env::current_dir().unwrap_or_default();

    // Convert path to absolute if it's relative
    let file_path = PathBuf::from(&request.path);
    let absolute_path = if file_path.is_absolute() {
        file_path
    } else {
        project_root.join(&file_path)
    };

    let client = state.get_client(&request.session_id, project_root).await;

    // Convert 1-based (editor) to 0-based (LSP)
    let line = request.line.saturating_sub(1);
    let column = request.column.saturating_sub(1);

    match client.prepare_type_hierarchy(&absolute_path, line, column) {
        Ok(items) => {
            let serialized_items: Vec<TypeHierarchyItemJson> = items
                .into_iter()
                .map(|item: TypeHierarchyItem| TypeHierarchyItemJson {
                    name: item.name,
                    detail: item.detail,
                    kind: match item.kind {
                        lsp_types::SymbolKind::CLASS => 5,
                        lsp_types::SymbolKind::INTERFACE => 11,
                        lsp_types::SymbolKind::STRUCT => 23,
                        _ => 12,
                    },
                    uri: item.uri.to_string(),
                    range: RangeJson {
                        start_line: item.range.start.line,
                        start_character: item.range.start.character,
                        end_line: item.range.end.line,
                        end_character: item.range.end.character,
                    },
                    selection_range: RangeJson {
                        start_line: item.selection_range.start.line,
                        start_character: item.selection_range.start.character,
                        end_line: item.selection_range.end.line,
                        end_character: item.selection_range.end.character,
                    },
                })
                .collect();

            tracing::info!(
                session_id = %request.session_id,
                items_count = serialized_items.len(),
                "LSP type hierarchy prepare successful"
            );

            (
                StatusCode::OK,
                Json(TypeHierarchyItemResponse {
                    success: true,
                    items: serialized_items,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!(
                session_id = %request.session_id,
                error = %e,
                "LSP type hierarchy prepare failed"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TypeHierarchyItemResponse {
                    success: false,
                    items: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

/// Handle Type Hierarchy Supertypes request
pub async fn lsp_type_hierarchy_supertypes(
    State(state): State<LspApiState>,
    Json(request): Json<TypeHierarchyItemsRequest>
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        item_name = %request.item.name,
        "LSP type hierarchy supertypes request"
    );

    let project_root = lsp_types::Url::parse(&request.item.uri)
        .ok()
        .and_then(|url| url.to_file_path().ok())
        .and_then(|p| p.parent().map(|p: &std::path::Path| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let client = state.get_client(&request.session_id, project_root).await;

    let lsp_item = match request.item.to_lsp_item() {
        Some(item) => item,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(TypeHierarchyItemResponse {
                    success: false,
                    items: vec![],
                    error: Some("Invalid item format".to_string()),
                }),
            );
        }
    };

    match client.type_hierarchy_supertypes(lsp_item) {
        Ok(supertypes) => {
            let serialized_items: Vec<TypeHierarchyItemJson> = supertypes
                .into_iter()
                .map(|item: TypeHierarchyItem| TypeHierarchyItemJson {
                    name: item.name,
                    detail: item.detail,
                    kind: match item.kind {
                        lsp_types::SymbolKind::CLASS => 5,
                        lsp_types::SymbolKind::INTERFACE => 11,
                        lsp_types::SymbolKind::STRUCT => 23,
                        _ => 12,
                    },
                    uri: item.uri.to_string(),
                    range: RangeJson {
                        start_line: item.range.start.line,
                        start_character: item.range.start.character,
                        end_line: item.range.end.line,
                        end_character: item.range.end.character,
                    },
                    selection_range: RangeJson {
                        start_line: item.selection_range.start.line,
                        start_character: item.selection_range.start.character,
                        end_line: item.selection_range.end.line,
                        end_character: item.selection_range.end.character,
                    },
                })
                .collect();

            tracing::info!(
                session_id = %request.session_id,
                items_count = serialized_items.len(),
                "LSP type hierarchy supertypes successful"
            );

            (
                StatusCode::OK,
                Json(TypeHierarchyItemResponse {
                    success: true,
                    items: serialized_items,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!(
                session_id = %request.session_id,
                error = %e,
                "LSP type hierarchy supertypes failed"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TypeHierarchyItemResponse {
                    success: false,
                    items: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

/// Handle Type Hierarchy Subtypes request
pub async fn lsp_type_hierarchy_subtypes(
    State(state): State<LspApiState>,
    Json(request): Json<TypeHierarchyItemsRequest>
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        item_name = %request.item.name,
        "LSP type hierarchy subtypes request"
    );

    let project_root = lsp_types::Url::parse(&request.item.uri)
        .ok()
        .and_then(|url| url.to_file_path().ok())
        .and_then(|p| p.parent().map(|p: &std::path::Path| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let client = state.get_client(&request.session_id, project_root).await;

    let lsp_item = match request.item.to_lsp_item() {
        Some(item) => item,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(TypeHierarchyItemResponse {
                    success: false,
                    items: vec![],
                    error: Some("Invalid item format".to_string()),
                }),
            );
        }
    };

    match client.type_hierarchy_subtypes(lsp_item) {
        Ok(subtypes) => {
            let serialized_items: Vec<TypeHierarchyItemJson> = subtypes
                .into_iter()
                .map(|item: TypeHierarchyItem| TypeHierarchyItemJson {
                    name: item.name,
                    detail: item.detail,
                    kind: match item.kind {
                        lsp_types::SymbolKind::CLASS => 5,
                        lsp_types::SymbolKind::INTERFACE => 11,
                        lsp_types::SymbolKind::STRUCT => 23,
                        _ => 12,
                    },
                    uri: item.uri.to_string(),
                    range: RangeJson {
                        start_line: item.range.start.line,
                        start_character: item.range.start.character,
                        end_line: item.range.end.line,
                        end_character: item.range.end.character,
                    },
                    selection_range: RangeJson {
                        start_line: item.selection_range.start.line,
                        start_character: item.selection_range.start.character,
                        end_line: item.selection_range.end.line,
                        end_character: item.selection_range.end.character,
                    },
                })
                .collect();

            tracing::info!(
                session_id = %request.session_id,
                items_count = serialized_items.len(),
                "LSP type hierarchy subtypes successful"
            );

            (
                StatusCode::OK,
                Json(TypeHierarchyItemResponse {
                    success: true,
                    items: serialized_items,
                    error: None,
                }),
            )
        }
        Err(e) => {
            tracing::error!(
                session_id = %request.session_id,
                error = %e,
                "LSP type hierarchy subtypes failed"
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TypeHierarchyItemResponse {
                    success: false,
                    items: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}
