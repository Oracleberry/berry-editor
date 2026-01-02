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
pub struct SlackMessage {
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
pub struct SlackUser {
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
pub struct SlackApiState {
    pub db: SqlitePool,
    pub broadcast_tx: Arc<RwLock<broadcast::Sender<WebSocketEvent>>>,
}

impl SlackApiState {
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
    State(state): State<SlackApiState>,
) -> Result<Json<Vec<Channel>>, StatusCode> {
    let channels = sqlx::query_as::<_, Channel>(
        "SELECT id, name, topic, created_at FROM slack_channels ORDER BY name"
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
    State(state): State<SlackApiState>,
    Json(req): Json<CreateChannelRequest>,
) -> Result<Json<Channel>, StatusCode> {
    let id = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO slack_channels (id, name, topic, created_at) VALUES (?, ?, ?, ?)"
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
    State(state): State<SlackApiState>,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<MessageWithMetadata>>, StatusCode> {
    let messages = sqlx::query_as::<_, SlackMessage>(
        "SELECT id, channel_id, user, content, timestamp, parent_message_id
         FROM slack_messages
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
             FROM slack_reactions
             WHERE message_id = ?
             GROUP BY emoji"
        )
        .bind(&msg.id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        // Get thread count
        let thread_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM slack_messages WHERE parent_message_id = ?"
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
    State(state): State<SlackApiState>,
    Json(req): Json<CreateMessageRequest>,
) -> Result<Json<MessageWithMetadata>, StatusCode> {
    let id = Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().to_rfc3339();
    let user = "current_user".to_string(); // TODO: Get from session

    sqlx::query(
        "INSERT INTO slack_messages (id, channel_id, user, content, timestamp) VALUES (?, ?, ?, ?, ?)"
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
    State(state): State<SlackApiState>,
    Path(message_id): Path<String>,
    Json(req): Json<AddReactionRequest>,
) -> Result<Json<String>, StatusCode> {
    let user = "current_user".to_string(); // TODO: Get from session
    let reaction_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO slack_reactions (id, message_id, user, emoji) VALUES (?, ?, ?, ?)"
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
    State(state): State<SlackApiState>,
    Path(message_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let messages = sqlx::query_as::<_, SlackMessage>(
        "SELECT id, channel_id, user, content, timestamp, parent_message_id
         FROM slack_messages
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
             FROM slack_reactions
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

pub async fn slack_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<SlackApiState>,
) -> Response {
    ws.on_upgrade(|socket| handle_slack_socket(socket, state))
}

async fn handle_slack_socket(socket: WebSocket, state: SlackApiState) {
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

/// Get all Slack users
pub async fn list_users(
    State(state): State<SlackApiState>,
) -> Result<Json<Vec<SlackUser>>, StatusCode> {
    let users = sqlx::query_as::<_, SlackUser>(
        "SELECT id, username, display_name, email, avatar_url, status, status_message, created_at FROM slack_users ORDER BY username"
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
    State(state): State<SlackApiState>,
    Json(req): Json<UpdatePresenceRequest>,
) -> Result<Json<String>, StatusCode> {
    let user = "admin"; // TODO: Get from session

    sqlx::query("UPDATE slack_users SET status = ? WHERE username = ?")
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

/// Get channel members
pub async fn get_channel_members(
    State(state): State<SlackApiState>,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<SlackUser>>, StatusCode> {
    let members = sqlx::query_as::<_, SlackUser>(
        "SELECT u.id, u.username, u.display_name, u.email, u.avatar_url, u.status, u.status_message, u.created_at
         FROM slack_users u
         INNER JOIN slack_channel_members cm ON u.id = cm.user_id
         WHERE cm.channel_id = ?
         ORDER BY u.username"
    )
    .bind(&channel_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch channel members: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(members))
}

/// Add user to channel
pub async fn add_channel_member(
    State(state): State<SlackApiState>,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> Result<Json<String>, StatusCode> {
    let member_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO slack_channel_members (id, channel_id, user_id, joined_at, role) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&member_id)
    .bind(&channel_id)
    .bind(&user_id)
    .bind(chrono::Utc::now().to_rfc3339())
    .bind("member")
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to add channel member: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json("OK".to_string()))
}

/// Remove user from channel
pub async fn remove_channel_member(
    State(state): State<SlackApiState>,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> Result<Json<String>, StatusCode> {
    sqlx::query("DELETE FROM slack_channel_members WHERE channel_id = ? AND user_id = ?")
        .bind(&channel_id)
        .bind(&user_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to remove channel member: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json("OK".to_string()))
}

/// Send thread reply
pub async fn send_thread_reply(
    State(state): State<SlackApiState>,
    Json(req): Json<CreateThreadReplyRequest>,
) -> Result<Json<MessageWithMetadata>, StatusCode> {
    let id = Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().to_rfc3339();
    let user = "admin".to_string(); // TODO: Get from session

    // Get the parent message to determine channel
    let parent: SlackMessage = sqlx::query_as::<_, SlackMessage>(
        "SELECT id, channel_id, user, content, timestamp, parent_message_id FROM slack_messages WHERE id = ?"
    )
    .bind(&req.parent_message_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch parent message: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    sqlx::query(
        "INSERT INTO slack_messages (id, channel_id, user, content, timestamp, parent_message_id) VALUES (?, ?, ?, ?, ?, ?)"
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

/// Create or get direct message channel
pub async fn create_or_get_dm(
    State(state): State<SlackApiState>,
    Json(req): Json<CreateDMRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user1_id = "admin_user_id".to_string(); // TODO: Get from session
    let (user_a, user_b) = if user1_id < req.user_id {
        (&user1_id, &req.user_id)
    } else {
        (&req.user_id, &user1_id)
    };

    // Check if DM already exists
    let existing_dm: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM slack_direct_messages WHERE user1_id = ? AND user2_id = ?"
    )
    .bind(user_a)
    .bind(user_b)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to check DM: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let dm_id = if let Some((id,)) = existing_dm {
        id
    } else {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO slack_direct_messages (id, user1_id, user2_id, created_at) VALUES (?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(user_a)
        .bind(user_b)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create DM: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        id
    };

    Ok(Json(serde_json::json!({
        "dm_id": dm_id,
        "user_id": req.user_id
    })))
}

/// Search messages
pub async fn search_messages(
    State(state): State<SlackApiState>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<Vec<MessageWithMetadata>>, StatusCode> {
    let search_pattern = format!("%{}%", req.query);

    let messages = sqlx::query_as::<_, SlackMessage>(
        "SELECT id, channel_id, user, content, timestamp, parent_message_id
         FROM slack_messages
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
             FROM slack_reactions
             WHERE message_id = ?
             GROUP BY emoji"
        )
        .bind(&msg.id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        let thread_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM slack_messages WHERE parent_message_id = ?"
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

/// Upload file attachment
pub async fn upload_file(
    State(state): State<SlackApiState>,
    Path(message_id): Path<String>,
    Json(file_data): Json<serde_json::Value>,
) -> Result<Json<String>, StatusCode> {
    // In a real implementation, this would handle multipart file upload
    // For now, we'll just store file metadata

    let file_id = Uuid::new_v4().to_string();
    let filename = file_data.get("filename")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let file_path = format!("uploads/{}", file_id);
    let file_size = file_data.get("size")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    sqlx::query(
        "INSERT INTO slack_file_attachments (id, message_id, filename, file_path, file_size, mime_type, uploaded_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&file_id)
    .bind(&message_id)
    .bind(filename)
    .bind(&file_path)
    .bind(file_size)
    .bind(file_data.get("mime_type").and_then(|v| v.as_str()))
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to upload file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(file_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        // Create channels table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS slack_channels (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                topic TEXT,
                description TEXT,
                is_private INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                created_by TEXT
            )"
        )
        .execute(&pool)
        .await
        .expect("Failed to create channels table");

        // Create messages table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS slack_messages (
                id TEXT PRIMARY KEY,
                channel_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                username TEXT NOT NULL,
                content TEXT NOT NULL,
                thread_id TEXT,
                timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                edited_at DATETIME,
                FOREIGN KEY (channel_id) REFERENCES slack_channels(id) ON DELETE CASCADE
            )"
        )
        .execute(&pool)
        .await
        .expect("Failed to create messages table");

        // Create reactions table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS slack_reactions (
                id TEXT PRIMARY KEY,
                message_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                emoji TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (message_id) REFERENCES slack_messages(id) ON DELETE CASCADE
            )"
        )
        .execute(&pool)
        .await
        .expect("Failed to create reactions table");

        pool
    }

    #[tokio::test]
    async fn test_slack_channel_queries() {
        let pool = setup_test_db().await;
        let channel_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO slack_channels (id, name, topic, is_private) VALUES (?, ?, ?, ?)"
        )
        .bind(&channel_id)
        .bind("test-channel")
        .bind("Test Topic")
        .bind(0)
        .execute(&pool)
        .await
        .expect("Failed to insert channel");

        let channels: Vec<(String,)> = sqlx::query_as("SELECT id FROM slack_channels")
            .fetch_all(&pool)
            .await
            .expect("Failed to fetch channels");

        assert!(!channels.is_empty());
    }

    #[tokio::test]
    async fn test_create_channel() {
        let pool = setup_test_db().await;
        let channel_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO slack_channels (id, name, topic, is_private) VALUES (?, ?, ?, ?)"
        )
        .bind(&channel_id)
        .bind("test-channel")
        .bind("Test Topic")
        .bind(0)
        .execute(&pool)
        .await
        .expect("Failed to insert channel");

        let channel: (String, String) = sqlx::query_as(
            "SELECT id, name FROM slack_channels WHERE id = ?"
        )
        .bind(&channel_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch channel");

        assert_eq!(channel.0, channel_id);
        assert_eq!(channel.1, "test-channel");
    }

    #[tokio::test]
    async fn test_send_message() {
        let pool = setup_test_db().await;
        let channel_id = Uuid::new_v4().to_string();
        let message_id = Uuid::new_v4().to_string();

        // Create channel first
        sqlx::query(
            "INSERT INTO slack_channels (id, name, topic, is_private) VALUES (?, ?, ?, ?)"
        )
        .bind(&channel_id)
        .bind("general")
        .bind("General Discussion")
        .bind(0)
        .execute(&pool)
        .await
        .expect("Failed to insert channel");

        // Send message
        sqlx::query(
            "INSERT INTO slack_messages (id, channel_id, user_id, username, content) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&message_id)
        .bind(&channel_id)
        .bind("user1")
        .bind("TestUser")
        .bind("Hello, Slack!")
        .execute(&pool)
        .await
        .expect("Failed to insert message");

        let message: (String, String) = sqlx::query_as(
            "SELECT id, content FROM slack_messages WHERE id = ?"
        )
        .bind(&message_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch message");

        assert_eq!(message.0, message_id);
        assert_eq!(message.1, "Hello, Slack!");
    }

    #[tokio::test]
    async fn test_add_reaction() {
        let pool = setup_test_db().await;
        let channel_id = Uuid::new_v4().to_string();
        let message_id = Uuid::new_v4().to_string();
        let reaction_id = Uuid::new_v4().to_string();

        // Create channel and message
        sqlx::query("INSERT INTO slack_channels (id, name, topic, is_private) VALUES (?, ?, ?, ?)")
            .bind(&channel_id)
            .bind("general")
            .bind("General")
            .bind(0)
            .execute(&pool)
            .await
            .expect("Failed to insert channel");

        sqlx::query("INSERT INTO slack_messages (id, channel_id, user_id, username, content) VALUES (?, ?, ?, ?, ?)")
            .bind(&message_id)
            .bind(&channel_id)
            .bind("user1")
            .bind("TestUser")
            .bind("Test message")
            .execute(&pool)
            .await
            .expect("Failed to insert message");

        // Add reaction
        sqlx::query("INSERT INTO slack_reactions (id, message_id, user_id, emoji) VALUES (?, ?, ?, ?)")
            .bind(&reaction_id)
            .bind(&message_id)
            .bind("user2")
            .bind("👍")
            .execute(&pool)
            .await
            .expect("Failed to insert reaction");

        let reaction: (String, String) = sqlx::query_as(
            "SELECT message_id, emoji FROM slack_reactions WHERE id = ?"
        )
        .bind(&reaction_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch reaction");

        assert_eq!(reaction.0, message_id);
        assert_eq!(reaction.1, "👍");
    }

    #[tokio::test]
    async fn test_thread_message() {
        let pool = setup_test_db().await;
        let channel_id = Uuid::new_v4().to_string();
        let parent_id = Uuid::new_v4().to_string();
        let thread_id = Uuid::new_v4().to_string();

        // Create channel
        sqlx::query("INSERT INTO slack_channels (id, name, topic, is_private) VALUES (?, ?, ?, ?)")
            .bind(&channel_id)
            .bind("general")
            .bind("General")
            .bind(0)
            .execute(&pool)
            .await
            .expect("Failed to insert channel");

        // Create parent message
        sqlx::query("INSERT INTO slack_messages (id, channel_id, user_id, username, content) VALUES (?, ?, ?, ?, ?)")
            .bind(&parent_id)
            .bind(&channel_id)
            .bind("user1")
            .bind("User1")
            .bind("Parent message")
            .execute(&pool)
            .await
            .expect("Failed to insert parent message");

        // Create thread reply
        sqlx::query("INSERT INTO slack_messages (id, channel_id, user_id, username, content, thread_id) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&thread_id)
            .bind(&channel_id)
            .bind("user2")
            .bind("User2")
            .bind("Thread reply")
            .bind(&parent_id)
            .execute(&pool)
            .await
            .expect("Failed to insert thread message");

        let thread: (String, String) = sqlx::query_as(
            "SELECT thread_id, content FROM slack_messages WHERE id = ?"
        )
        .bind(&thread_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch thread message");

        assert_eq!(thread.0, parent_id);
        assert_eq!(thread.1, "Thread reply");
    }

    #[tokio::test]
    async fn test_slack_message_serialization() {
        let message = SlackMessage {
            id: "msg1".to_string(),
            channel_id: "channel1".to_string(),
            user: "TestUser".to_string(),
            content: "Test message".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            parent_message_id: None,
        };

        let json = serde_json::to_string(&message).expect("Failed to serialize");
        let parsed: SlackMessage = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(parsed.id, "msg1");
        assert_eq!(parsed.channel_id, "channel1");
        assert_eq!(parsed.content, "Test message");
    }

    #[tokio::test]
    async fn test_update_presence_request_serialization() {
        let presence = UpdatePresenceRequest {
            status: "online".to_string(),
        };

        let json = serde_json::to_string(&presence).expect("Failed to serialize");
        let parsed: UpdatePresenceRequest = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(parsed.status, "online");
    }
}
