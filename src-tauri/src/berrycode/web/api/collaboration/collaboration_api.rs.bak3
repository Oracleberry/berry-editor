//! Collaboration API endpoints
//!
//! Provides REST and WebSocket endpoints for Live Share functionality

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    response::{IntoResponse, Json, Response},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

use crate::berrycode::collaboration::{
    session_manager::{CollaborationSession, CursorPosition, User, UserRole, SessionManager},
    ot_engine::{Operation, OperationType, OTEngine},
};

/// Collaboration API state
#[derive(Clone)]
pub struct CollaborationApiState {
    pub session_manager: Arc<SessionManager>,
    pub ot_engines: Arc<RwLock<HashMap<String, OTEngine>>>,
    pub broadcasters: Arc<RwLock<HashMap<String, broadcast::Sender<CollaborationMessage>>>>,
}

impl CollaborationApiState {
    pub fn new() -> Self {
        Self {
            session_manager: Arc::new(SessionManager::new()),
            ot_engines: Arc::new(RwLock::new(HashMap::new())),
            broadcasters: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Request to create a collaboration session
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub name: String,
    pub project_root: String,
    pub user_name: String,
}

/// Request to join a collaboration session
#[derive(Debug, Deserialize)]
pub struct JoinSessionRequest {
    pub invite_code: String,
    pub user_name: String,
}

/// Response for session operations
#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub session: CollaborationSession,
}

/// Request to change user role
#[derive(Debug, Deserialize)]
pub struct ChangeRoleRequest {
    pub target_user_id: String,
    pub new_role: UserRole,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CollaborationMessage {
    /// User joined the session
    UserJoined {
        user: User,
    },
    /// User left the session
    UserLeft {
        user_id: String,
    },
    /// Cursor position update
    CursorUpdate {
        user_id: String,
        cursor: CursorPosition,
    },
    /// Text operation
    TextOperation {
        operation: Operation,
    },
    /// User role changed
    RoleChanged {
        user_id: String,
        new_role: UserRole,
    },
    /// Error message
    Error {
        message: String,
    },
    /// Session state sync (initial connection)
    SessionSync {
        session: CollaborationSession,
        file_versions: HashMap<String, u64>,
    },
}

/// Create a new collaboration session
pub async fn create_collaboration_session(
    State(state): State<CollaborationApiState>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<SessionResponse>, (StatusCode, String)> {
    let user_id = uuid::Uuid::new_v4().to_string();
    let user = User {
        id: user_id,
        name: req.user_name,
        role: UserRole::Host,
        color: generate_user_color(),
        cursor_position: None,
        joined_at: chrono::Utc::now(),
    };

    let session = state
        .session_manager
        .create_session(req.name, user, req.project_root)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // Create OT engine for this session
    let mut ot_engines = state.ot_engines.write().await;
    ot_engines.insert(session.id.clone(), OTEngine::new());

    // Create broadcast channel for this session
    let (tx, _) = broadcast::channel(100);
    let mut broadcasters = state.broadcasters.write().await;
    broadcasters.insert(session.id.clone(), tx);

    Ok(Json(SessionResponse { session }))
}

/// Join a collaboration session
pub async fn join_collaboration_session(
    State(state): State<CollaborationApiState>,
    Json(req): Json<JoinSessionRequest>,
) -> Result<Json<SessionResponse>, (StatusCode, String)> {
    let user_id = uuid::Uuid::new_v4().to_string();
    let user = User {
        id: user_id,
        name: req.user_name,
        role: UserRole::Editor,
        color: generate_user_color(),
        cursor_position: None,
        joined_at: chrono::Utc::now(),
    };

    let session = state
        .session_manager
        .join_session(&req.invite_code, user.clone())
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // Broadcast user joined
    if let Some(tx) = state.broadcasters.read().await.get(&session.id) {
        let _ = tx.send(CollaborationMessage::UserJoined { user });
    }

    Ok(Json(SessionResponse { session }))
}

/// Get collaboration session
pub async fn get_collaboration_session(
    State(state): State<CollaborationApiState>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionResponse>, (StatusCode, String)> {
    let session = state
        .session_manager
        .get_session(&session_id)
        .await
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    Ok(Json(SessionResponse { session }))
}

/// Leave a collaboration session
pub async fn leave_collaboration_session(
    State(state): State<CollaborationApiState>,
    Path(session_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode, (StatusCode, String)> {
    let user_id = params
        .get("user_id")
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing user_id".to_string()))?;

    state
        .session_manager
        .leave_session(&session_id, user_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // Broadcast user left
    if let Some(tx) = state.broadcasters.read().await.get(&session_id) {
        let _ = tx.send(CollaborationMessage::UserLeft {
            user_id: user_id.clone(),
        });
    }

    Ok(StatusCode::OK)
}

/// Change user role
pub async fn change_collaboration_role(
    State(state): State<CollaborationApiState>,
    Path(session_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    Json(req): Json<ChangeRoleRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let requester_id = params
        .get("user_id")
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing user_id".to_string()))?;

    state
        .session_manager
        .change_user_role(&session_id, requester_id, &req.target_user_id, req.new_role.clone())
        .await
        .map_err(|e| (StatusCode::FORBIDDEN, e))?;

    // Broadcast role change
    if let Some(tx) = state.broadcasters.read().await.get(&session_id) {
        let _ = tx.send(CollaborationMessage::RoleChanged {
            user_id: req.target_user_id,
            new_role: req.new_role,
        });
    }

    Ok(StatusCode::OK)
}

/// WebSocket handler for collaboration
pub async fn collaboration_ws_handler(
    ws: WebSocketUpgrade,
    Path(session_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<CollaborationApiState>,
) -> Response {
    let user_id = match params.get("user_id") {
        Some(id) => id.clone(),
        None => return (StatusCode::BAD_REQUEST, "Missing user_id").into_response(),
    };

    ws.on_upgrade(move |socket| handle_collaboration_socket(socket, session_id, user_id, state))
}

/// Handle WebSocket connection for collaboration
async fn handle_collaboration_socket(
    mut socket: WebSocket,
    session_id: String,
    user_id: String,
    state: CollaborationApiState,
) {
    tracing::info!("Collaboration WebSocket connected: session={}, user={}", session_id, user_id);

    // Get session
    let session = match state.session_manager.get_session(&session_id).await {
        Some(s) => s,
        None => {
            let _ = socket
                .send(Message::Text(
                    serde_json::to_string(&CollaborationMessage::Error {
                        message: "Session not found".to_string(),
                    })
                    .unwrap(),
                ))
                .await;
            return;
        }
    };

    // Get file versions from OT engine
    let file_versions = {
        let ot_engines = state.ot_engines.read().await;
        if let Some(engine) = ot_engines.get(&session_id) {
            // For now, we'll send empty versions - in a full implementation,
            // we'd track all open files
            HashMap::new()
        } else {
            HashMap::new()
        }
    };

    // Send initial session sync
    let sync_msg = CollaborationMessage::SessionSync {
        session: session.clone(),
        file_versions,
    };

    if let Ok(json) = serde_json::to_string(&sync_msg) {
        let _ = socket.send(Message::Text(json)).await;
    }

    // Subscribe to broadcasts
    let mut rx = {
        let broadcasters = state.broadcasters.read().await;
        match broadcasters.get(&session_id) {
            Some(tx) => tx.subscribe(),
            None => {
                tracing::error!("No broadcaster for session {}", session_id);
                return;
            }
        }
    };

    // Handle incoming and outgoing messages
    loop {
        tokio::select! {
            // Incoming WebSocket message
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = handle_collaboration_message(
                            text,
                            &session_id,
                            &user_id,
                            &state,
                        )
                        .await
                        {
                            tracing::error!("Error handling message: {}", e);
                            let error_msg = CollaborationMessage::Error {
                                message: e.to_string(),
                            };
                            if let Ok(json) = serde_json::to_string(&error_msg) {
                                let _ = socket.send(Message::Text(json)).await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!("Collaboration WebSocket closed: session={}, user={}", session_id, user_id);
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // Broadcast message
            msg = rx.recv() => {
                match msg {
                    Ok(collab_msg) => {
                        // Don't send back messages from this user
                        let skip = match &collab_msg {
                            CollaborationMessage::CursorUpdate { user_id: uid, .. } => uid == &user_id,
                            CollaborationMessage::TextOperation { operation } => operation.user_id == user_id,
                            _ => false,
                        };

                        if !skip {
                            if let Ok(json) = serde_json::to_string(&collab_msg) {
                                if socket.send(Message::Text(json)).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        tracing::warn!("Client lagged behind in receiving messages");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("Broadcast channel closed");
                        break;
                    }
                }
            }
        }
    }

    tracing::info!("Collaboration WebSocket disconnected: session={}, user={}", session_id, user_id);
}

/// Handle incoming collaboration message
async fn handle_collaboration_message(
    text: String,
    session_id: &str,
    user_id: &str,
    state: &CollaborationApiState,
) -> Result<(), String> {
    let msg: CollaborationMessage = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse message: {}", e))?;

    match msg {
        CollaborationMessage::CursorUpdate { cursor, .. } => {
            state
                .session_manager
                .update_cursor(session_id, user_id, cursor.clone())
                .await?;

            // Broadcast cursor update
            if let Some(tx) = state.broadcasters.read().await.get(session_id) {
                let _ = tx.send(CollaborationMessage::CursorUpdate {
                    user_id: user_id.to_string(),
                    cursor,
                });
            }
        }

        CollaborationMessage::TextOperation { operation } => {
            // Apply operation through OT engine
            let mut ot_engines = state.ot_engines.write().await;
            if let Some(engine) = ot_engines.get_mut(session_id) {
                // In a full implementation, we'd read the current file content
                // For now, we just process the operation
                let _version = engine.get_version(&operation.file_path);

                // Broadcast the operation to all other users
                if let Some(tx) = state.broadcasters.read().await.get(session_id) {
                    let _ = tx.send(CollaborationMessage::TextOperation { operation });
                }
            }
        }

        _ => {
            return Err("Invalid message type".to_string());
        }
    }

    Ok(())
}

/// Generate a random user color
fn generate_user_color() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // Generate pleasant pastel colors
    let colors = [
        "#FF6B6B", "#4ECDC4", "#45B7D1", "#FFA07A", "#98D8C8",
        "#F7DC6F", "#BB8FCE", "#85C1E2", "#F8B88B", "#A9DFBF",
    ];

    colors[rng.gen_range(0..colors.len())].to_string()
}
