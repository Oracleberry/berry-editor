//! Jupyter Notebook API endpoints

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::berrycode::jupyter::{Notebook, KernelManager, ExecutionResult, KernelStatus};
use crate::berrycode::web::infrastructure::error::{WebError, WebResult};
use crate::berrycode::web::infrastructure::session_db::SessionDbStore;

/// Jupyter API state
#[derive(Clone)]
pub struct JupyterApiState {
    pub session_store: SessionDbStore,
    pub kernel_manager: Arc<Mutex<KernelManager>>,
}

/// Notebook request/response structures
#[derive(Debug, Serialize, Deserialize)]
pub struct NotebookQuery {
    pub session_id: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotebookResponse {
    pub notebook: Notebook,
    pub kernel_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateNotebookRequest {
    pub session_id: String,
    pub path: String,
    pub kernel_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteCellRequest {
    pub session_id: String,
    pub notebook_path: String,
    pub cell_index: usize,
    pub kernel_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteCellResponse {
    pub result: ExecutionResult,
    pub kernel_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCellRequest {
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KernelRequest {
    pub session_id: String,
    pub kernel_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KernelResponse {
    pub kernel_id: String,
    pub status: KernelStatus,
}

impl JupyterApiState {
    pub fn new(session_store: SessionDbStore) -> Self {
        Self {
            session_store,
            kernel_manager: Arc::new(Mutex::new(KernelManager::new())),
        }
    }
}

/// Create Jupyter API router
pub fn jupyter_api_router() -> Router<JupyterApiState> {
    Router::new()
        .route("/api/jupyter/notebook", get(get_notebook))
        .route("/api/jupyter/notebook", post(create_notebook))
        .route("/api/jupyter/notebook", put(save_notebook))
        .route("/api/jupyter/cell/execute", post(execute_cell))
        .route("/api/jupyter/cell/:cell_index", put(update_cell))
        .route("/api/jupyter/cell/:cell_index", delete(delete_cell))
        .route("/api/jupyter/cell/:cell_index/outputs", delete(clear_cell_outputs))
        .route("/api/jupyter/kernel", post(start_kernel))
        .route("/api/jupyter/kernel/:kernel_id", delete(stop_kernel))
        .route("/api/jupyter/kernel/:kernel_id/restart", post(restart_kernel))
        .route("/api/jupyter/kernel/:kernel_id/interrupt", post(interrupt_kernel))
        .route("/api/jupyter/kernel/:kernel_id/status", get(get_kernel_status))
        .route("/api/jupyter/kernels", get(list_kernels))
}

/// Get a notebook
async fn get_notebook(
    Query(query): Query<NotebookQuery>,
    State(state): State<JupyterApiState>,
) -> WebResult<Json<NotebookResponse>> {
    tracing::debug!(session_id = %query.session_id, path = %query.path, "Getting notebook");

    // Get session
    let session = state
        .session_store
        .get_session(&query.session_id)
        .await
        .ok_or_else(|| WebError::NotFound(format!("Session not found: {}", query.session_id)))?;

    // Build full path
    let full_path = session.project_root.join(&query.path);

    // Security check
    if !full_path.starts_with(&session.project_root) {
        return Err(WebError::PermissionDenied(
            "Path traversal attempt detected".to_string(),
        ));
    }

    // Load notebook
    let notebook = Notebook::from_file(&full_path)
        .map_err(|e| WebError::Internal(format!("Failed to load notebook: {}", e)))?;

    Ok(Json(NotebookResponse {
        notebook,
        kernel_id: None,
    }))
}

/// Create a new notebook
async fn create_notebook(
    State(state): State<JupyterApiState>,
    Json(req): Json<CreateNotebookRequest>,
) -> WebResult<Json<NotebookResponse>> {
    tracing::info!(session_id = %req.session_id, path = %req.path, "Creating notebook");

    // Get session
    let session = state
        .session_store
        .get_session(&req.session_id)
        .await
        .ok_or_else(|| WebError::NotFound(format!("Session not found: {}", req.session_id)))?;

    // Build full path
    let full_path = session.project_root.join(&req.path);

    // Security check
    if !full_path.starts_with(&session.project_root) {
        return Err(WebError::PermissionDenied(
            "Path traversal attempt detected".to_string(),
        ));
    }

    // Create notebook
    let notebook = Notebook::new(&req.kernel_name);

    // Save notebook
    notebook
        .save(&full_path)
        .map_err(|e| WebError::Internal(format!("Failed to save notebook: {}", e)))?;

    Ok(Json(NotebookResponse {
        notebook,
        kernel_id: None,
    }))
}

/// Save a notebook
async fn save_notebook(
    State(state): State<JupyterApiState>,
    Json(payload): Json<serde_json::Value>,
) -> WebResult<Json<serde_json::Value>> {
    let session_id = payload["session_id"]
        .as_str()
        .ok_or_else(|| WebError::BadRequest("Missing session_id".to_string()))?;

    let path = payload["path"]
        .as_str()
        .ok_or_else(|| WebError::BadRequest("Missing path".to_string()))?;

    let notebook: Notebook = serde_json::from_value(payload["notebook"].clone())
        .map_err(|e| WebError::BadRequest(format!("Invalid notebook data: {}", e)))?;

    tracing::debug!(session_id = %session_id, path = %path, "Saving notebook");

    // Get session
    let session = state
        .session_store
        .get_session(session_id)
        .await
        .ok_or_else(|| WebError::NotFound(format!("Session not found: {}", session_id)))?;

    // Build full path
    let full_path = session.project_root.join(path);

    // Security check
    if !full_path.starts_with(&session.project_root) {
        return Err(WebError::PermissionDenied(
            "Path traversal attempt detected".to_string(),
        ));
    }

    // Save notebook
    notebook
        .save(&full_path)
        .map_err(|e| WebError::Internal(format!("Failed to save notebook: {}", e)))?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// Execute a cell
async fn execute_cell(
    State(state): State<JupyterApiState>,
    Json(req): Json<ExecuteCellRequest>,
) -> WebResult<Json<ExecuteCellResponse>> {
    tracing::debug!(
        session_id = %req.session_id,
        cell_index = req.cell_index,
        "Executing cell"
    );

    // Get session
    let session = state
        .session_store
        .get_session(&req.session_id)
        .await
        .ok_or_else(|| WebError::NotFound(format!("Session not found: {}", req.session_id)))?;

    // Build full path
    let full_path = session.project_root.join(&req.notebook_path);

    // Load notebook
    let notebook = Notebook::from_file(&full_path)
        .map_err(|e| WebError::Internal(format!("Failed to load notebook: {}", e)))?;

    // Get cell
    let cell = notebook
        .get_cell(req.cell_index)
        .ok_or_else(|| WebError::NotFound("Cell not found".to_string()))?;

    // Get code
    let code = cell.get_source();

    // Get or create kernel
    let mut kernel_manager = state.kernel_manager.lock().await;

    let kernel_id = if let Some(kid) = req.kernel_id {
        kid
    } else {
        // Start a new kernel
        let kernel_name = notebook
            .metadata
            .kernelspec
            .as_ref()
            .map(|k| k.name.as_str())
            .unwrap_or("python3");

        kernel_manager
            .start_kernel(kernel_name)
            .await
            .map_err(|e| WebError::Internal(format!("Failed to start kernel: {}", e)))?
    };

    // Execute code
    let result = kernel_manager
        .execute(&kernel_id, &code, false)
        .await
        .map_err(|e| WebError::Internal(format!("Failed to execute code: {}", e)))?;

    Ok(Json(ExecuteCellResponse { result, kernel_id }))
}

/// Update cell source
async fn update_cell(
    Path((session_id, notebook_path, cell_index)): Path<(String, String, usize)>,
    State(state): State<JupyterApiState>,
    Json(req): Json<UpdateCellRequest>,
) -> WebResult<Json<serde_json::Value>> {
    tracing::debug!(
        session_id = %session_id,
        cell_index = cell_index,
        "Updating cell"
    );

    // Get session
    let session = state
        .session_store
        .get_session(&session_id)
        .await
        .ok_or_else(|| WebError::NotFound(format!("Session not found: {}", session_id)))?;

    // Build full path
    let full_path = session.project_root.join(&notebook_path);

    // Load notebook
    let mut notebook = Notebook::from_file(&full_path)
        .map_err(|e| WebError::Internal(format!("Failed to load notebook: {}", e)))?;

    // Update cell
    notebook
        .update_cell_source(cell_index, req.source)
        .map_err(|e| WebError::Internal(format!("Failed to update cell: {}", e)))?;

    // Save notebook
    notebook
        .save(&full_path)
        .map_err(|e| WebError::Internal(format!("Failed to save notebook: {}", e)))?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// Delete a cell
async fn delete_cell(
    Path((session_id, notebook_path, cell_index)): Path<(String, String, usize)>,
    State(state): State<JupyterApiState>,
) -> WebResult<Json<serde_json::Value>> {
    tracing::debug!(
        session_id = %session_id,
        cell_index = cell_index,
        "Deleting cell"
    );

    // Get session
    let session = state
        .session_store
        .get_session(&session_id)
        .await
        .ok_or_else(|| WebError::NotFound(format!("Session not found: {}", session_id)))?;

    // Build full path
    let full_path = session.project_root.join(&notebook_path);

    // Load notebook
    let mut notebook = Notebook::from_file(&full_path)
        .map_err(|e| WebError::Internal(format!("Failed to load notebook: {}", e)))?;

    // Delete cell
    if cell_index < notebook.cells.len() {
        notebook.cells.remove(cell_index);
    } else {
        return Err(WebError::NotFound("Cell not found".to_string()));
    }

    // Save notebook
    notebook
        .save(&full_path)
        .map_err(|e| WebError::Internal(format!("Failed to save notebook: {}", e)))?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// Clear cell outputs
async fn clear_cell_outputs(
    Path((session_id, notebook_path, cell_index)): Path<(String, String, usize)>,
    State(state): State<JupyterApiState>,
) -> WebResult<Json<serde_json::Value>> {
    tracing::debug!(
        session_id = %session_id,
        cell_index = cell_index,
        "Clearing cell outputs"
    );

    // Get session
    let session = state
        .session_store
        .get_session(&session_id)
        .await
        .ok_or_else(|| WebError::NotFound(format!("Session not found: {}", session_id)))?;

    // Build full path
    let full_path = session.project_root.join(&notebook_path);

    // Load notebook
    let mut notebook = Notebook::from_file(&full_path)
        .map_err(|e| WebError::Internal(format!("Failed to load notebook: {}", e)))?;

    // Clear outputs
    notebook
        .clear_cell_outputs(cell_index)
        .map_err(|e| WebError::Internal(format!("Failed to clear outputs: {}", e)))?;

    // Save notebook
    notebook
        .save(&full_path)
        .map_err(|e| WebError::Internal(format!("Failed to save notebook: {}", e)))?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// Start a kernel
async fn start_kernel(
    State(state): State<JupyterApiState>,
    Json(req): Json<KernelRequest>,
) -> WebResult<Json<KernelResponse>> {
    tracing::info!(kernel_name = %req.kernel_name, "Starting kernel");

    let mut kernel_manager = state.kernel_manager.lock().await;

    let kernel_id = kernel_manager
        .start_kernel(&req.kernel_name)
        .await
        .map_err(|e| WebError::Internal(format!("Failed to start kernel: {}", e)))?;

    let status = kernel_manager
        .get_status(&kernel_id)
        .await
        .map_err(|e| WebError::Internal(format!("Failed to get kernel status: {}", e)))?;

    Ok(Json(KernelResponse { kernel_id, status }))
}

/// Stop a kernel
async fn stop_kernel(
    Path(kernel_id): Path<String>,
    State(state): State<JupyterApiState>,
) -> WebResult<Json<serde_json::Value>> {
    tracing::info!(kernel_id = %kernel_id, "Stopping kernel");

    let kernel_manager = state.kernel_manager.lock().await;

    kernel_manager
        .stop_kernel(&kernel_id)
        .await
        .map_err(|e| WebError::Internal(format!("Failed to stop kernel: {}", e)))?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// Restart a kernel
async fn restart_kernel(
    Path(kernel_id): Path<String>,
    State(state): State<JupyterApiState>,
) -> WebResult<Json<KernelResponse>> {
    tracing::info!(kernel_id = %kernel_id, "Restarting kernel");

    let kernel_manager = state.kernel_manager.lock().await;

    kernel_manager
        .restart_kernel(&kernel_id)
        .await
        .map_err(|e| WebError::Internal(format!("Failed to restart kernel: {}", e)))?;

    let status = kernel_manager
        .get_status(&kernel_id)
        .await
        .map_err(|e| WebError::Internal(format!("Failed to get kernel status: {}", e)))?;

    Ok(Json(KernelResponse {
        kernel_id: kernel_id.to_string(),
        status,
    }))
}

/// Interrupt a kernel
async fn interrupt_kernel(
    Path(kernel_id): Path<String>,
    State(state): State<JupyterApiState>,
) -> WebResult<Json<serde_json::Value>> {
    tracing::info!(kernel_id = %kernel_id, "Interrupting kernel");

    let kernel_manager = state.kernel_manager.lock().await;

    kernel_manager
        .interrupt_kernel(&kernel_id)
        .await
        .map_err(|e| WebError::Internal(format!("Failed to interrupt kernel: {}", e)))?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// Get kernel status
async fn get_kernel_status(
    Path(kernel_id): Path<String>,
    State(state): State<JupyterApiState>,
) -> WebResult<Json<KernelResponse>> {
    let kernel_manager = state.kernel_manager.lock().await;

    let status = kernel_manager
        .get_status(&kernel_id)
        .await
        .map_err(|e| WebError::Internal(format!("Failed to get kernel status: {}", e)))?;

    Ok(Json(KernelResponse {
        kernel_id: kernel_id.to_string(),
        status,
    }))
}

/// List all kernels
async fn list_kernels(
    State(state): State<JupyterApiState>,
) -> WebResult<Json<serde_json::Value>> {
    let kernel_manager = state.kernel_manager.lock().await;

    let kernels = kernel_manager.list_kernels().await;

    Ok(Json(serde_json::json!({ "kernels": kernels })))
}
