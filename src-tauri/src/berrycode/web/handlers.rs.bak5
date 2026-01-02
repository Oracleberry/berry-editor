//! HTTP handlers for web interface

use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tera::{Context, Tera};
use lsp_types::{
    CallHierarchyItem,
    TypeHierarchyItem,
    Position,
};

#[derive(Deserialize)]
pub struct LspCallHierarchyPrepareRequest {
    pub file_path: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Deserialize)]
pub struct LspHierarchyCallRequest {
    pub item: CallHierarchyItem,
}

#[derive(Deserialize)]
pub struct LspTypeHierarchyPrepareRequest {
    pub file_path: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Deserialize)]
pub struct LspTypeHierarchyRequest {
    pub item: TypeHierarchyItem,
}

use super::infrastructure::error::{session_not_found, WebError, WebResult};
use super::infrastructure::session_db::{Session, SessionDbStore};

/// Handler state
#[derive(Clone)]
pub struct HandlerState {
    pub session_store: SessionDbStore,
    pub tera: Tera,
    #[cfg(debug_assertions)]
    pub template_path: String,
}

#[cfg(debug_assertions)]
impl HandlerState {
    /// Get Tera instance with hot reload in debug mode
    pub fn get_tera(&self) -> Result<Tera, tera::Error> {
        Tera::new(&self.template_path)
    }
}

#[cfg(not(debug_assertions))]
impl HandlerState {
    /// Get cached Tera instance in release mode
    pub fn get_tera(&self) -> Result<Tera, tera::Error> {
        Ok(self.tera.clone())
    }
}

/// Session creation request
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub project_root: Option<String>,
}

/// Session response
#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub project_root: String,
}

/// Landing page
pub async fn landing_page(State(state): State<HandlerState>) -> WebResult<Html<String>> {
    tracing::debug!("Rendering landing page");

    let context = Context::new();
    let tera = state.get_tera()?;
    let html = tera.render("landing.html", &context)?;
    Ok(Html(html))
}

/// Dashboard page - Project selection
pub async fn dashboard_page(State(state): State<HandlerState>) -> WebResult<Html<String>> {
    tracing::debug!("Rendering dashboard page");

    let mut context = Context::new();
    context.insert("version", env!("CARGO_PKG_VERSION"));
    let tera = state.get_tera()?;
    let html = tera.render("dashboard.html", &context)?;
    Ok(Html(html))
}

/// LSP test page
pub async fn lsp_test_page(State(state): State<HandlerState>) -> WebResult<Html<String>> {
    tracing::debug!("Rendering LSP test page");

    let context = Context::new();
    let tera = state.get_tera()?;
    let html = tera.render("lsp_test.html", &context)?;
    Ok(Html(html))
}

/// Index page
pub async fn index(State(state): State<HandlerState>) -> WebResult<Html<String>> {
    tracing::debug!("Rendering index page");

    let mut context = Context::new();
    context.insert("version", env!("CARGO_PKG_VERSION"));

    // Get recent projects
    let recent_projects = state.session_store.get_recent_projects(10).await?;
    context.insert("recent_projects", &recent_projects);

    let tera = state.get_tera()?;
    let html = tera.render("index.html", &context)?;
    tracing::debug!("Index page rendered successfully");
    Ok(Html(html))
}

/// Test page for debugging JavaScript
pub async fn test_page(State(state): State<HandlerState>) -> WebResult<Html<String>> {
    tracing::debug!("Rendering test page");

    let context = Context::new();
    let tera = state.get_tera()?;
    let html = tera.render("test.html", &context)?;
    tracing::debug!("Test page rendered successfully");
    Ok(Html(html))
}

/// Workflows page
pub async fn workflows(State(state): State<HandlerState>) -> WebResult<Html<String>> {
    tracing::debug!("Rendering workflows page");

    let mut context = Context::new();
    context.insert("version", env!("CARGO_PKG_VERSION"));

    let tera = state.get_tera()?;
    let html = tera.render("workflows.html", &context)?;
    tracing::debug!("Workflows page rendered successfully");
    Ok(Html(html))
}

/// Workflow history page
pub async fn workflow_history(State(state): State<HandlerState>) -> WebResult<Html<String>> {
    tracing::debug!("Rendering workflow history page");

    let mut context = Context::new();
    context.insert("version", env!("CARGO_PKG_VERSION"));

    let tera = state.get_tera()?;
    let html = tera.render("workflow_history.html", &context)?;
    tracing::debug!("Workflow history page rendered successfully");
    Ok(Html(html))
}

/// Flow editor page
pub async fn flow_editor(State(state): State<HandlerState>) -> WebResult<Html<String>> {
    tracing::debug!("Rendering flow editor page");

    let mut context = Context::new();
    context.insert("version", env!("CARGO_PKG_VERSION"));

    let tera = state.get_tera()?;
    let html = tera.render("flow_editor.html", &context)?;
    tracing::debug!("Flow editor page rendered successfully");
    Ok(Html(html))
}

/// Create new session
pub async fn create_session(
    State(state): State<HandlerState>,
    Json(payload): Json<CreateSessionRequest>,
) -> WebResult<Json<SessionResponse>> {
    tracing::info!("Creating new session");

    let project_root = if let Some(root) = payload.project_root {
        PathBuf::from(root)
    } else {
        std::env::current_dir().map_err(|e| {
            tracing::error!(error = %e, "Failed to get current directory");
            WebError::Internal(format!("Failed to get current directory: {}", e))
        })?
    };

    // Validate project root exists
    if !project_root.exists() {
        tracing::warn!(path = %project_root.display(), "Project root does not exist");
        return Err(WebError::BadRequest(format!(
            "Project root does not exist: {}",
            project_root.display()
        )));
    }

    // Validate it's a directory
    if !project_root.is_dir() {
        tracing::warn!(path = %project_root.display(), "Project root is not a directory");
        return Err(WebError::BadRequest(format!(
            "Project root is not a directory: {}",
            project_root.display()
        )));
    }

    let session_id = state.session_store.create_session(project_root.clone()).await?;

    tracing::info!(session_id = %session_id, path = %project_root.display(), "Session created successfully");

    Ok(Json(SessionResponse {
        session_id,
        project_root: project_root.to_string_lossy().to_string(),
    }))
}

/// Get session info
pub async fn get_session(
    AxumPath(session_id): AxumPath<String>,
    State(state): State<HandlerState>,
) -> WebResult<Json<Session>> {
    tracing::debug!(session_id = %session_id, "Getting session info");

    let session = state
        .session_store
        .get_session(&session_id)
        .await
        .ok_or_else(|| session_not_found(&session_id))?;

    tracing::debug!(session_id = %session_id, "Session info retrieved successfully");
    Ok(Json(session))
}

/// Chat page
pub async fn chat_page(
    AxumPath(session_id): AxumPath<String>,
    State(state): State<HandlerState>,
) -> WebResult<Html<String>> {
    tracing::debug!(session_id = %session_id, "Rendering chat page");

    let session = state
        .session_store
        .get_session(&session_id)
        .await
        .ok_or_else(|| session_not_found(&session_id))?;

    let mut context = Context::new();
    context.insert("session_id", &session_id);
    context.insert("project_root", &session.project_root.to_string_lossy().to_string());
    context.insert("version", env!("CARGO_PKG_VERSION"));
    context.insert("mode", "chat");

    let tera = state.get_tera()?;
    let html = tera.render("editor.html", &context)?;
    tracing::debug!(session_id = %session_id, "Chat page rendered successfully (using editor)");
    Ok(Html(html))
}

/// Devin Mode page
pub async fn devin_page(
    AxumPath(session_id): AxumPath<String>,
    State(state): State<HandlerState>,
) -> Result<Html<String>, Response> {
    // Verify session exists
    let session = state
        .session_store
        .get_session(&session_id)
        .await
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, "Session not found").into_response()
        })?;

    let mut context = Context::new();
    context.insert("session_id", &session_id);
    context.insert("project_root", &session.project_root.to_string_lossy().to_string());
    context.insert("version", env!("CARGO_PKG_VERSION"));
    context.insert("mode", "devin");

    state
        .tera
        .render("chat_modern.html", &context)
        .map(Html)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template error: {}", e),
            )
                .into_response()
        })
}

/// Enable sharing for session
pub async fn enable_sharing(
    AxumPath(session_id): AxumPath<String>,
    State(state): State<HandlerState>,
) -> Result<Json<String>, StatusCode> {
    let share_url = state
        .session_store
        .enable_sharing(&session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(share_url))
}

/// Get current working directory
pub async fn get_current_dir() -> Result<Json<String>, StatusCode> {
    let cwd = std::env::current_dir()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string_lossy()
        .to_string();
    Ok(Json(cwd))
}

/// Get recent projects
pub async fn get_recent_projects(
    State(state): State<HandlerState>,
) -> Result<Json<Vec<super::infrastructure::database::RecentProject>>, StatusCode> {
    let projects = state
        .session_store
        .get_recent_projects(10)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(projects))
}

/// Serve favicon (redirect to SVG)
pub async fn favicon() -> Redirect {
    Redirect::permanent("/static/favicon.svg")
}

/// Shared session page
pub async fn shared_session(
    AxumPath(session_id): AxumPath<String>,
    State(state): State<HandlerState>,
) -> Result<Html<String>, Response> {
    // Verify session exists and is shared
    let session = state
        .session_store
        .get_session(&session_id)
        .await
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, "Session not found").into_response()
        })?;

    if !session.shared {
        return Err((StatusCode::FORBIDDEN, "Session not shared").into_response());
    }

    // Render read-only view
    let mut context = Context::new();
    context.insert("session_id", &session_id);
    context.insert("project_root", &session.project_root.to_string_lossy().to_string());
    context.insert("read_only", &true);
    context.insert("version", env!("CARGO_PKG_VERSION"));

    state
        .tera
        .render("chat_modern.html", &context)
        .map(Html)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template error: {}", e),
            )
                .into_response()
        })
}

/// Login page
pub async fn login_page(State(state): State<HandlerState>) -> WebResult<Html<String>> {
    tracing::debug!("Rendering login page");

    let context = Context::new();
    let tera = state.get_tera()?;
    let html = tera.render("login.html", &context)?;
    tracing::debug!("Login page rendered successfully");
    Ok(Html(html))
}

/// Register page
pub async fn register_page(State(state): State<HandlerState>) -> WebResult<Html<String>> {
    tracing::debug!("Rendering register page");

    let context = Context::new();
    let tera = state.get_tera()?;
    let html = tera.render("register.html", &context)?;
    tracing::debug!("Register page rendered successfully");
    Ok(Html(html))
}

/// Health check endpoint for K8s liveness probe
pub async fn health_check() -> StatusCode {
    // Simple health check - returns 200 if server is running
    tracing::debug!("Health check requested");
    StatusCode::OK
}

/// Readiness check endpoint for K8s readiness probe
pub async fn readiness_check(State(state): State<HandlerState>) -> StatusCode {
    // Check if database is accessible
    match state.session_store.health_check().await {
        Ok(_) => {
            tracing::debug!("Readiness check passed");
            StatusCode::OK
        }
        Err(e) => {
            tracing::error!("Readiness check failed: {}", e);
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

/// BerryEditor page (Pure Rust Code Editor)
pub async fn berry_editor_page(State(state): State<HandlerState>) -> WebResult<Html<String>> {
    tracing::debug!("Rendering BerryEditor page");

    // Load the actual editor HTML from templates
    let context = Context::new();
    let tera = state.get_tera()?;
    let html = tera.render("editor.html", &context)?;
    Ok(Html(html))
}

/// BerryEditor page BACKUP (Pure Rust Code Editor)
pub async fn _berry_editor_page_backup(State(state): State<HandlerState>) -> WebResult<Html<String>> {
    tracing::debug!("Rendering BerryEditor page");

    // For now, return a simple HTML that loads the WASM editor
    // In production, this would load from gui-editor/dist/index.html
    let html = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>BerryEditor - Pure Rust Code Editor</title>
    <style>
        body {
            margin: 0;
            padding: 20px;
            font-family: 'Consolas', 'Monaco', monospace;
            background: #1e1e1e;
            color: #d4d4d4;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
        }
        .container {
            text-align: center;
            max-width: 600px;
        }
        h1 {
            color: #007acc;
            margin-bottom: 20px;
        }
        .info {
            background: #252525;
            padding: 30px;
            border-radius: 8px;
            border: 1px solid #3e3e3e;
            margin-top: 20px;
        }
        code {
            background: #2d2d2d;
            padding: 4px 8px;
            border-radius: 4px;
            font-size: 14px;
        }
        .step {
            margin: 15px 0;
            text-align: left;
        }
        a {
            color: #007acc;
            text-decoration: none;
        }
        a:hover {
            text-decoration: underline;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 BerryEditor</h1>
        <p>Pure Rust Code Editor - 100% WASM, Zero JavaScript!</p>

        <div class="info">
            <h2>Build Instructions</h2>
            <div class="step">
                <p><strong>1. Install Trunk:</strong></p>
                <code>cargo install trunk</code>
            </div>
            <div class="step">
                <p><strong>2. Add WASM target:</strong></p>
                <code>rustup target add wasm32-unknown-unknown</code>
            </div>
            <div class="step">
                <p><strong>3. Build the editor:</strong></p>
                <code>cd gui-editor && trunk build --release</code>
            </div>
            <div class="step">
                <p><strong>4. Or run dev server:</strong></p>
                <code>./scripts/dev-gui-editor.sh</code>
            </div>
            <p style="margin-top: 20px;">
                <a href="https://github.com/lapce/lapce" target="_blank">Inspired by Lapce</a> |
                <a href="https://github.com/zed-industries/zed" target="_blank">Zed</a> |
                <a href="https://github.com/leptos-rs/leptos" target="_blank">Leptos</a>
            </p>
        </div>
    </div>
</body>
</html>
"#;

    Ok(Html(html.to_string()))
}
