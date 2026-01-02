//! Remote development API endpoints

use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::berrycode::remote::{
    RemoteFileNode, RemotePath, SshAuth, SshConfig, SshConnection, SshConnectionManager,
};

use crate::berrycode::web::infrastructure::error::{WebError, WebResult};
use crate::berrycode::web::infrastructure::database::{Database, RemoteConnectionInfo};

/// Remote API state
#[derive(Clone)]
pub struct RemoteApiState {
    pub connection_manager: Arc<SshConnectionManager>,
    pub database: Database,
}

/// Connection request
#[derive(Debug, Deserialize, Serialize)]
pub struct ConnectRequest {
    pub id: String,
    pub host: String,
    pub port: Option<u16>,
    pub username: String,
    pub auth_type: String, // "password", "key", "agent"
    pub password: Option<String>,
    pub private_key_path: Option<String>,
    pub passphrase: Option<String>,
    pub session_id: String,
}

/// Connection response
#[derive(Debug, Serialize)]
pub struct ConnectResponse {
    pub success: bool,
    pub message: String,
    pub connection_id: String,
}

/// Connection info
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub status: String,
}

/// File tree request
#[derive(Debug, Deserialize)]
pub struct FileTreeRequest {
    pub connection_id: String,
    pub path: Option<String>,
    pub max_depth: Option<usize>,
}

/// Remote file content
#[derive(Debug, Serialize, Deserialize)]
pub struct RemoteFileContent {
    pub path: String,
    pub content: String,
    pub language: Option<String>,
}

/// Remote file save request
#[derive(Debug, Deserialize)]
pub struct RemoteFileSaveRequest {
    pub connection_id: String,
    pub path: String,
    pub content: String,
}

/// Execute command request
#[derive(Debug, Deserialize)]
pub struct ExecCommandRequest {
    pub connection_id: String,
    pub command: String,
}

/// Execute command response
#[derive(Debug, Serialize)]
pub struct ExecCommandResponse {
    pub output: String,
    pub exit_code: i32,
}

/// Connect to remote host
pub async fn connect(
    State(state): State<RemoteApiState>,
    Json(payload): Json<ConnectRequest>,
) -> WebResult<Json<ConnectResponse>> {
    tracing::info!(
        "Connecting to remote host: {}@{}:{}",
        payload.username,
        payload.host,
        payload.port.unwrap_or(22)
    );

    // Build SSH config
    let auth = match payload.auth_type.as_str() {
        "password" => {
            let password = payload
                .password
                .ok_or_else(|| WebError::BadRequest("Password required".to_string()))?;
            SshAuth::Password(password)
        }
        "key" => {
            let private_key_path = payload
                .private_key_path
                .ok_or_else(|| WebError::BadRequest("Private key path required".to_string()))?;
            SshAuth::PublicKey {
                private_key_path,
                passphrase: payload.passphrase,
            }
        }
        "agent" => SshAuth::Agent,
        _ => {
            return Err(WebError::BadRequest(format!(
                "Invalid auth type: {}",
                payload.auth_type
            )))
        }
    };

    let config = SshConfig {
        host: payload.host.clone(),
        port: payload.port.unwrap_or(22),
        username: payload.username.clone(),
        auth,
    };

    // Connect
    match state
        .connection_manager
        .add_connection(payload.id.clone(), config.clone())
        .await
    {
        Ok(_) => {
            // Save connection to database
            if let Err(e) = state
                .database
                .save_remote_connection(
                    &payload.session_id,
                    &payload.id,
                    &payload.host,
                    config.port,
                    &payload.username,
                )
                .await
            {
                tracing::error!("Failed to save remote connection: {}", e);
            }

            Ok(Json(ConnectResponse {
                success: true,
                message: "Connected successfully".to_string(),
                connection_id: payload.id,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to connect: {}", e);
            Err(WebError::Internal(format!(
                "Connection failed: {}",
                e
            )))
        }
    }
}

/// Disconnect from remote host
pub async fn disconnect(
    Path(connection_id): Path<String>,
    State(state): State<RemoteApiState>,
) -> WebResult<StatusCode> {
    tracing::info!("Disconnecting from remote host: {}", connection_id);

    state
        .connection_manager
        .remove_connection(&connection_id)
        .await
        .map_err(|e| WebError::Internal(format!("Failed to disconnect: {}", e)))?;

    // Remove from database
    if let Err(e) = state.database.delete_remote_connection(&connection_id).await {
        tracing::error!("Failed to delete remote connection: {}", e);
    }

    Ok(StatusCode::OK)
}

/// List all connections
pub async fn list_connections(
    State(state): State<RemoteApiState>,
    Query(session_query): Query<HashMap<String, String>>,
) -> WebResult<Json<Vec<RemoteConnectionInfo>>> {
    let session_id = session_query
        .get("session_id")
        .ok_or_else(|| WebError::BadRequest("session_id required".to_string()))?;

    // Get connections from database
    let connections = state
        .database
        .get_remote_connections(session_id)
        .await
        .map_err(|e| {
            WebError::Internal(format!("Failed to list connections: {}", e))
        })?;

    Ok(Json(connections))
}

/// Get remote file tree
pub async fn get_file_tree(
    Query(query): Query<FileTreeRequest>,
    State(state): State<RemoteApiState>,
) -> WebResult<Json<RemoteFileNode>> {
    tracing::debug!(
        "Getting remote file tree for connection: {}",
        query.connection_id
    );

    let connection = state
        .connection_manager
        .get_connection(&query.connection_id)
        .await
        .map_err(|e| WebError::NotFound(format!("Connection not found: {}", e)))?;

    let path = query.path.as_deref().unwrap_or("/");
    let max_depth = query.max_depth.unwrap_or(3);

    // Get connection root directory (home directory)
    let root = {
        let mut conn = connection.lock().await;
        let home = conn.exec("echo $HOME").map_err(|e| {
            WebError::Internal(format!("Failed to get home directory: {}", e))
        })?;
        std::path::PathBuf::from(home.trim())
    };

    let fs = crate::berrycode::remote::RemoteFileSystem::new(connection, root);

    let tree = fs.build_file_tree(path, max_depth).await.map_err(|e| {
        WebError::Internal(format!("Failed to build file tree: {}", e))
    })?;

    Ok(Json(tree))
}

/// Read remote file
pub async fn read_file(
    Path((connection_id, file_path)): Path<(String, String)>,
    State(state): State<RemoteApiState>,
) -> WebResult<Json<RemoteFileContent>> {
    tracing::debug!("Reading remote file: {} on {}", file_path, connection_id);

    let connection = state
        .connection_manager
        .get_connection(&connection_id)
        .await
        .map_err(|e| WebError::NotFound(format!("Connection not found: {}", e)))?;

    let contents = {
        let mut conn = connection.lock().await;
        conn.read_file(&file_path)
            .map_err(|e| WebError::Internal(format!("Failed to read file: {}", e)))?
    };

    let content = String::from_utf8(contents)
        .map_err(|e| WebError::Internal(format!("Invalid UTF-8: {}", e)))?;

    let language = detect_language(&file_path);

    Ok(Json(RemoteFileContent {
        path: file_path,
        content,
        language,
    }))
}

/// Write remote file
pub async fn write_file(
    State(state): State<RemoteApiState>,
    Json(payload): Json<RemoteFileSaveRequest>,
) -> WebResult<StatusCode> {
    tracing::debug!(
        "Writing remote file: {} on {}",
        payload.path,
        payload.connection_id
    );

    let connection = state
        .connection_manager
        .get_connection(&payload.connection_id)
        .await
        .map_err(|e| WebError::NotFound(format!("Connection not found: {}", e)))?;

    {
        let mut conn = connection.lock().await;
        conn.write_file(&payload.path, payload.content.as_bytes())
            .map_err(|e| WebError::Internal(format!("Failed to write file: {}", e)))?;
    }

    Ok(StatusCode::OK)
}

/// Execute command on remote host
pub async fn execute_command(
    State(state): State<RemoteApiState>,
    Json(payload): Json<ExecCommandRequest>,
) -> WebResult<Json<ExecCommandResponse>> {
    tracing::debug!(
        "Executing command on {}: {}",
        payload.connection_id,
        payload.command
    );

    let connection = state
        .connection_manager
        .get_connection(&payload.connection_id)
        .await
        .map_err(|e| WebError::NotFound(format!("Connection not found: {}", e)))?;

    let output = {
        let mut conn = connection.lock().await;
        conn.exec(&payload.command).map_err(|e| {
            WebError::Internal(format!("Failed to execute command: {}", e))
        })?
    };

    Ok(Json(ExecCommandResponse {
        output,
        exit_code: 0,
    }))
}

/// Test remote connection
pub async fn test_connection(
    Path(connection_id): Path<String>,
    State(state): State<RemoteApiState>,
) -> WebResult<Json<serde_json::Value>> {
    tracing::debug!("Testing connection: {}", connection_id);

    let is_alive = state
        .connection_manager
        .test_connection(&connection_id)
        .await
        .unwrap_or(false);

    Ok(Json(serde_json::json!({
        "alive": is_alive,
        "connection_id": connection_id
    })))
}

/// WebSocket handler for remote terminal
pub async fn remote_terminal_ws(
    ws: WebSocketUpgrade,
    Path(connection_id): Path<String>,
    State(state): State<RemoteApiState>,
) -> Response {
    ws.on_upgrade(move |socket| async move {
        if let Err(e) = handle_remote_terminal(socket, connection_id, state).await {
            tracing::error!("Remote terminal error: {}", e);
        }
    })
}

async fn handle_remote_terminal(
    socket: axum::extract::ws::WebSocket,
    connection_id: String,
    state: RemoteApiState,
) -> anyhow::Result<()> {
    tracing::info!("Starting remote terminal session for {}", connection_id);

    let connection = state.connection_manager.get_connection(&connection_id).await?;

    // Open shell channel
    let channel = {
        let mut conn = connection.lock().await;
        conn.open_shell()?
    };

    // Wrap channel in Arc<Mutex> for shared access
    let channel = Arc::new(Mutex::new(channel));
    let channel_clone = channel.clone();

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Spawn task to read from SSH channel and send to WebSocket
    let read_task = tokio::spawn(async move {
        let mut buffer = [0u8; 4096];
        loop {
            let read_result = {
                let mut ch = channel_clone.lock().await;
                ch.read(&mut buffer)
            };

            match read_result {
                Ok(n) if n > 0 => {
                    let data = &buffer[..n];
                    if ws_sender
                        .send(axum::extract::ws::Message::Binary(data.to_vec()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(_) => break, // EOF
                Err(e) => {
                    tracing::error!("SSH read error: {}", e);
                    break;
                }
            }
        }
    });

    // Read from WebSocket and write to SSH channel
    while let Some(msg) = ws_receiver.next().await {
        let write_result = match msg {
            Ok(axum::extract::ws::Message::Binary(data)) => {
                let mut ch = channel.lock().await;
                ch.write_all(&data)
            }
            Ok(axum::extract::ws::Message::Text(text)) => {
                let mut ch = channel.lock().await;
                ch.write_all(text.as_bytes())
            }
            Ok(axum::extract::ws::Message::Close(_)) => break,
            _ => continue,
        };

        if write_result.is_err() {
            break;
        }
    }

    read_task.abort();
    Ok(())
}

/// Detect programming language from file extension
fn detect_language(file_path: &str) -> Option<String> {
    let ext = file_path.rsplit('.').next()?;

    let language = match ext.to_lowercase().as_str() {
        "rs" => "rust",
        "py" => "python",
        "js" => "javascript",
        "ts" => "typescript",
        "jsx" => "javascript",
        "tsx" => "typescript",
        "html" => "html",
        "css" => "css",
        "json" => "json",
        "md" => "markdown",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "sh" => "bash",
        "c" => "c",
        "cpp" | "cc" | "cxx" => "cpp",
        "go" => "go",
        "java" => "java",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "kt" => "kotlin",
        _ => return None,
    };

    Some(language.to_string())
}

/// Port forwarding request
#[derive(Debug, Deserialize)]
pub struct PortForwardRequest {
    pub connection_id: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

/// Port forwarding response
#[derive(Debug, Serialize)]
pub struct PortForwardResponse {
    pub success: bool,
    pub message: String,
    pub local_port: u16,
}

/// Start port forwarding
pub async fn start_port_forward(
    State(_state): State<RemoteApiState>,
    Json(_payload): Json<PortForwardRequest>,
) -> WebResult<Json<PortForwardResponse>> {
    // TODO: Implement port forwarding
    // This would require maintaining a separate task that forwards traffic
    // between a local port and a remote port through the SSH tunnel

    Err(WebError::Internal(
        "Port forwarding not yet implemented".to_string(),
    ))
}
