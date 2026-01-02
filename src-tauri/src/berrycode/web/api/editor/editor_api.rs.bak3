//! Editor integration API via WebSocket
//!
//! This module provides a WebSocket API for editor integrations (VSCode, Cursor, Zed, Neovim, etc.)

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Editor API state
#[derive(Clone)]
pub struct EditorApiState {
    pub sessions: Arc<Mutex<Vec<EditorSession>>>,
}

/// Editor session
#[derive(Clone, Debug)]
pub struct EditorSession {
    pub editor_name: String,
    pub editor_version: String,
    pub workspace_path: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

/// Editor API message types
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EditorMessage {
    /// Editor connection info
    #[serde(rename = "connect")]
    Connect {
        editor_name: String,
        editor_version: String,
        workspace_path: String,
    },

    /// Get file content
    #[serde(rename = "get_file")]
    GetFile {
        file_path: String,
    },

    /// Update file content
    #[serde(rename = "update_file")]
    UpdateFile {
        file_path: String,
        content: String,
    },

    /// Get current cursor position
    #[serde(rename = "get_cursor")]
    GetCursor,

    /// Set cursor position
    #[serde(rename = "set_cursor")]
    SetCursor {
        file_path: String,
        line: u32,
        column: u32,
    },

    /// Execute command
    #[serde(rename = "execute")]
    Execute {
        command: String,
        args: Option<serde_json::Value>,
    },

    /// Response from server
    #[serde(rename = "response")]
    Response {
        success: bool,
        data: Option<serde_json::Value>,
        error: Option<String>,
    },

    /// Notification from server
    #[serde(rename = "notification")]
    Notification {
        message: String,
        level: String, // "info", "warning", "error"
    },
}

impl EditorApiState {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for EditorApiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Editor API router
pub fn editor_api_router() -> Router<EditorApiState> {
    Router::new()
        .route("/ws/editor", get(editor_ws_handler))
}

/// WebSocket handler for editor connections
async fn editor_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<EditorApiState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_editor_socket(socket, state))
}

/// Handle editor WebSocket connection
async fn handle_editor_socket(mut socket: WebSocket, state: EditorApiState) {
    tracing::info!("Editor connected");

    // Send welcome message
    let welcome = EditorMessage::Notification {
        message: "Connected to BerryCode".to_string(),
        level: "info".to_string(),
    };

    if let Ok(json) = serde_json::to_string(&welcome) {
        let _ = socket.send(Message::Text(json)).await;
    }

    let mut session: Option<EditorSession> = None;

    // Handle incoming messages
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    tracing::debug!("Received editor message: {}", text);

                    if let Ok(editor_msg) = serde_json::from_str::<EditorMessage>(&text) {
                        let response = handle_editor_message(editor_msg, &mut session, &state).await;

                        if let Ok(json) = serde_json::to_string(&response) {
                            let _ = socket.send(Message::Text(json)).await;
                        }
                    }
                }
                Message::Close(_) => {
                    tracing::info!("Editor disconnected");
                    break;
                }
                _ => {}
            }
        }
    }

    // Remove session on disconnect
    if let Some(sess) = session {
        let mut sessions = state.sessions.lock().await;
        sessions.retain(|s| s.editor_name != sess.editor_name || s.workspace_path != sess.workspace_path);
    }
}

/// Handle individual editor message
async fn handle_editor_message(
    msg: EditorMessage,
    session: &mut Option<EditorSession>,
    state: &EditorApiState,
) -> EditorMessage {
    match msg {
        EditorMessage::Connect { editor_name, editor_version, workspace_path } => {
            let new_session = EditorSession {
                editor_name: editor_name.clone(),
                editor_version: editor_version.clone(),
                workspace_path: workspace_path.clone(),
                connected_at: chrono::Utc::now(),
            };

            // Add to sessions list
            let mut sessions = state.sessions.lock().await;
            sessions.push(new_session.clone());
            *session = Some(new_session);

            EditorMessage::Response {
                success: true,
                data: Some(serde_json::json!({
                    "message": format!("Connected: {} {}", editor_name, editor_version)
                })),
                error: None,
            }
        }

        EditorMessage::GetFile { file_path } => {
            // Read file from workspace
            if let Some(sess) = session {
                let full_path = std::path::Path::new(&sess.workspace_path).join(&file_path);

                match std::fs::read_to_string(&full_path) {
                    Ok(content) => EditorMessage::Response {
                        success: true,
                        data: Some(serde_json::json!({
                            "file_path": file_path,
                            "content": content
                        })),
                        error: None,
                    },
                    Err(e) => EditorMessage::Response {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to read file: {}", e)),
                    },
                }
            } else {
                EditorMessage::Response {
                    success: false,
                    data: None,
                    error: Some("Not connected. Send Connect message first.".to_string()),
                }
            }
        }

        EditorMessage::UpdateFile { file_path, content } => {
            if let Some(sess) = session {
                let full_path = std::path::Path::new(&sess.workspace_path).join(&file_path);

                // Create parent directories if needed
                if let Some(parent) = full_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                match std::fs::write(&full_path, content) {
                    Ok(_) => EditorMessage::Response {
                        success: true,
                        data: Some(serde_json::json!({
                            "file_path": file_path,
                            "message": "File updated successfully"
                        })),
                        error: None,
                    },
                    Err(e) => EditorMessage::Response {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to write file: {}", e)),
                    },
                }
            } else {
                EditorMessage::Response {
                    success: false,
                    data: None,
                    error: Some("Not connected. Send Connect message first.".to_string()),
                }
            }
        }

        EditorMessage::Execute { command, args } => {
            if let Some(sess) = session {
                // Execute command in workspace
                let output = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&command)
                    .current_dir(&sess.workspace_path)
                    .output();

                match output {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&out.stderr).to_string();

                        EditorMessage::Response {
                            success: out.status.success(),
                            data: Some(serde_json::json!({
                                "command": command,
                                "stdout": stdout,
                                "stderr": stderr,
                                "exit_code": out.status.code()
                            })),
                            error: if !out.status.success() {
                                Some(stderr)
                            } else {
                                None
                            },
                        }
                    }
                    Err(e) => EditorMessage::Response {
                        success: false,
                        data: None,
                        error: Some(format!("Failed to execute command: {}", e)),
                    },
                }
            } else {
                EditorMessage::Response {
                    success: false,
                    data: None,
                    error: Some("Not connected. Send Connect message first.".to_string()),
                }
            }
        }

        _ => EditorMessage::Response {
            success: false,
            data: None,
            error: Some("Unsupported message type".to_string()),
        },
    }
}
