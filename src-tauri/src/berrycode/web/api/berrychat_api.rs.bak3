use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use futures::stream::StreamExt;
use futures::SinkExt;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

// ============================================
// Data Structures
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub topic: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BerryChatMessage {
    pub id: String,
    pub channel_id: String,
    pub user: String,
    pub content: String,
    pub timestamp: String,
    pub parent_message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageWithMetadata {
    pub id: String,
    pub channel_id: String,
    pub user: String,
    pub content: String,
    pub timestamp: String,
    pub reactions: Vec<Reaction>,
    pub thread_count: i64,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Reaction {
    pub emoji: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub topic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    pub channel_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddReactionRequest {
    pub emoji: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateThreadReplyRequest {
    pub parent_message_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDMRequest {
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub search_type: Option<String>, // "messages" or "files"
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BerryChatUser {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub status: String,
    pub status_message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePresenceRequest {
    pub status: String, // "online", "away", "busy", "offline"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketEvent {
    pub r#type: String,
    pub channel: Option<String>,
    pub message: Option<MessageWithMetadata>,
    pub message_id: Option<String>,
    pub reaction: Option<String>,
    pub user: Option<String>,
    pub user_id: Option<String>,
    pub status: Option<String>,
}

// ============================================
// State Management
// ============================================

#[derive(Clone)]
pub struct BerryChatApiState {
    pub db: SqlitePool,
    pub broadcast_tx: Arc<RwLock<broadcast::Sender<WebSocketEvent>>>,
}

impl BerryChatApiState {
    pub fn new(db: SqlitePool) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            db,
            broadcast_tx: Arc::new(RwLock::new(tx)),
        }
    }

    pub async fn broadcast(&self, event: WebSocketEvent) {
        let tx = self.broadcast_tx.read().await;
        let _ = tx.send(event);
    }
}

// ============================================
// API Endpoints
// ============================================

/// Get all channels
pub async fn list_channels(
    State(state): State<BerryChatApiState>,
) -> Result<Json<Vec<Channel>>, StatusCode> {
    let channels = sqlx::query_as::<_, Channel>(
        "SELECT id, name, topic, created_at FROM berrychat_channels ORDER BY name"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch channels: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(channels))
}

/// Create a new channel
pub async fn create_channel(
    State(state): State<BerryChatApiState>,
    Json(req): Json<CreateChannelRequest>,
) -> Result<Json<Channel>, StatusCode> {
    let id = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO berrychat_channels (id, name, topic, created_at) VALUES (?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.topic)
    .bind(&created_at)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create channel: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let channel = Channel {
        id,
        name: req.name,
        topic: req.topic,
        created_at,
    };

    Ok(Json(channel))
}

/// Get messages for a channel
pub async fn get_channel_messages(
    State(state): State<BerryChatApiState>,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<MessageWithMetadata>>, StatusCode> {
    let messages = sqlx::query_as::<_, BerryChatMessage>(
        "SELECT id, channel_id, user, content, timestamp, parent_message_id
         FROM berrychat_messages
         WHERE channel_id = ? AND parent_message_id IS NULL
         ORDER BY timestamp ASC"
    )
    .bind(&channel_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch messages: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut result = Vec::new();

    for msg in messages {
        // Get reactions for this message
        let reactions = sqlx::query_as::<_, Reaction>(
            "SELECT emoji, COUNT(*) as count
             FROM berrychat_reactions
             WHERE message_id = ?
             GROUP BY emoji"
        )
        .bind(&msg.id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        // Get thread count
        let thread_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM berrychat_messages WHERE parent_message_id = ?"
        )
        .bind(&msg.id)
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));

        result.push(MessageWithMetadata {
            id: msg.id,
            channel_id: msg.channel_id,
            user: msg.user,
            content: msg.content,
            timestamp: msg.timestamp,
            reactions,
            thread_count: thread_count.0,
            attachments: vec![],
        });
    }

    Ok(Json(result))
}

/// Send a message to a channel
pub async fn send_message(
    State(state): State<BerryChatApiState>,
    Json(req): Json<CreateMessageRequest>,
) -> Result<Json<MessageWithMetadata>, StatusCode> {
    let id = Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().to_rfc3339();
    let user = "current_user".to_string(); // TODO: Get from session

    sqlx::query(
        "INSERT INTO berrychat_messages (id, channel_id, user, content, timestamp) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&req.channel_id)
    .bind(&user)
    .bind(&req.content)
    .bind(&timestamp)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to send message: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let message = MessageWithMetadata {
        id: id.clone(),
        channel_id: req.channel_id.clone(),
        user: user.clone(),
        content: req.content,
        timestamp: timestamp.clone(),
        reactions: vec![],
        thread_count: 0,
        attachments: vec![],
    };

    // Broadcast message to WebSocket clients
    state.broadcast(WebSocketEvent {
        r#type: "message".to_string(),
        channel: Some(req.channel_id),
        message: Some(message.clone()),
        message_id: None,
        reaction: None,
        user: None,
        user_id: None,
        status: None,
    }).await;

    Ok(Json(message))
}

/// Add a reaction to a message
pub async fn add_reaction(
    State(state): State<BerryChatApiState>,
    Path(message_id): Path<String>,
    Json(req): Json<AddReactionRequest>,
) -> Result<Json<String>, StatusCode> {
    let user = "current_user".to_string(); // TODO: Get from session
    let reaction_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO berrychat_reactions (id, message_id, user, emoji) VALUES (?, ?, ?, ?)"
    )
    .bind(&reaction_id)
    .bind(&message_id)
    .bind(&user)
    .bind(&req.emoji)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to add reaction: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Broadcast reaction to WebSocket clients
    state.broadcast(WebSocketEvent {
        r#type: "reaction".to_string(),
        channel: None,
        message: None,
        message_id: Some(message_id),
        reaction: Some(req.emoji),
        user: Some(user),
        user_id: None,
        status: None,
    }).await;

    Ok(Json("OK".to_string()))
}

/// Get thread messages
pub async fn get_thread(
    State(state): State<BerryChatApiState>,
    Path(message_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let messages = sqlx::query_as::<_, BerryChatMessage>(
        "SELECT id, channel_id, user, content, timestamp, parent_message_id
         FROM berrychat_messages
         WHERE id = ? OR parent_message_id = ?
         ORDER BY timestamp ASC"
    )
    .bind(&message_id)
    .bind(&message_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch thread: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut result = Vec::new();

    for msg in messages {
        let reactions = sqlx::query_as::<_, Reaction>(
            "SELECT emoji, COUNT(*) as count
             FROM berrychat_reactions
             WHERE message_id = ?
             GROUP BY emoji"
        )
        .bind(&msg.id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        result.push(MessageWithMetadata {
            id: msg.id,
            channel_id: msg.channel_id,
            user: msg.user,
            content: msg.content,
            timestamp: msg.timestamp,
            reactions,
            thread_count: 0,
            attachments: vec![],
        });
    }

    Ok(Json(serde_json::json!({
        "messages": result
    })))
}

// ============================================
// WebSocket Handler
// ============================================

pub async fn berrychat_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<BerryChatApiState>,
) -> Response {
    ws.on_upgrade(|socket| handle_berrychat_socket(socket, state))
}

async fn handle_berrychat_socket(socket: WebSocket, state: BerryChatApiState) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast channel
    let mut rx = {
        let tx = state.broadcast_tx.read().await;
        tx.subscribe()
    };

    // Spawn task to forward broadcast messages to WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&event) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming WebSocket messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                tracing::info!("Received WebSocket message: {}", text);
                // Handle client messages if needed
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}

// ============================================
// Additional Endpoints
// ============================================

/// Get all BerryChat users
pub async fn list_users(
    State(state): State<BerryChatApiState>,
) -> Result<Json<Vec<BerryChatUser>>, StatusCode> {
    let users = sqlx::query_as::<_, BerryChatUser>(
        "SELECT id, username, display_name, email, avatar_url, status, status_message, created_at FROM berrychat_users ORDER BY username"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch users: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(users))
}

/// Update user presence status
pub async fn update_presence(
    State(state): State<BerryChatApiState>,
    Json(req): Json<UpdatePresenceRequest>,
) -> Result<Json<String>, StatusCode> {
    let user = "admin"; // TODO: Get from session

    sqlx::query("UPDATE berrychat_users SET status = ? WHERE username = ?")
        .bind(&req.status)
        .bind(user)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update presence: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Broadcast presence update
    state.broadcast(WebSocketEvent {
        r#type: "user_presence".to_string(),
        channel: None,
        message: None,
        message_id: None,
        reaction: None,
        user: Some(user.to_string()),
        user_id: None,
        status: Some(req.status),
    }).await;

    Ok(Json("OK".to_string()))
}

/// Send thread reply
pub async fn send_thread_reply(
    State(state): State<BerryChatApiState>,
    Json(req): Json<CreateThreadReplyRequest>,
) -> Result<Json<MessageWithMetadata>, StatusCode> {
    let id = Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().to_rfc3339();
    let user = "admin".to_string(); // TODO: Get from session

    // Get the parent message to determine channel
    let parent: BerryChatMessage = sqlx::query_as::<_, BerryChatMessage>(
        "SELECT id, channel_id, user, content, timestamp, parent_message_id FROM berrychat_messages WHERE id = ?"
    )
    .bind(&req.parent_message_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch parent message: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    sqlx::query(
        "INSERT INTO berrychat_messages (id, channel_id, user, content, timestamp, parent_message_id) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&parent.channel_id)
    .bind(&user)
    .bind(&req.content)
    .bind(&timestamp)
    .bind(&req.parent_message_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to send thread reply: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let message = MessageWithMetadata {
        id: id.clone(),
        channel_id: parent.channel_id.clone(),
        user: user.clone(),
        content: req.content,
        timestamp: timestamp.clone(),
        reactions: vec![],
        thread_count: 0,
        attachments: vec![],
    };

    // Broadcast thread reply
    state.broadcast(WebSocketEvent {
        r#type: "thread_reply".to_string(),
        channel: Some(parent.channel_id),
        message: Some(message.clone()),
        message_id: Some(req.parent_message_id),
        reaction: None,
        user: None,
        user_id: None,
        status: None,
    }).await;

    Ok(Json(message))
}

/// Search messages
pub async fn search_messages(
    State(state): State<BerryChatApiState>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<Vec<MessageWithMetadata>>, StatusCode> {
    let search_pattern = format!("%{}%", req.query);

    let messages = sqlx::query_as::<_, BerryChatMessage>(
        "SELECT id, channel_id, user, content, timestamp, parent_message_id
         FROM berrychat_messages
         WHERE content LIKE ? AND parent_message_id IS NULL
         ORDER BY timestamp DESC
         LIMIT 50"
    )
    .bind(&search_pattern)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to search messages: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut result = Vec::new();

    for msg in messages {
        let reactions = sqlx::query_as::<_, Reaction>(
            "SELECT emoji, COUNT(*) as count
             FROM berrychat_reactions
             WHERE message_id = ?
             GROUP BY emoji"
        )
        .bind(&msg.id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        let thread_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM berrychat_messages WHERE parent_message_id = ?"
        )
        .bind(&msg.id)
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));

        result.push(MessageWithMetadata {
            id: msg.id,
            channel_id: msg.channel_id,
            user: msg.user,
            content: msg.content,
            timestamp: msg.timestamp,
            reactions,
            thread_count: thread_count.0,
            attachments: vec![],
        });
    }

    Ok(Json(result))
}
