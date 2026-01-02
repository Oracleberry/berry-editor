use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::Response,
    Json,
};
use futures::stream::StreamExt;
use futures::SinkExt;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

// ============================================
// Data Structures
// ============================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSession {
    pub id: String,
    pub channel_id: String,
    pub initiator: String,
    pub call_type: String, // "audio", "video", "screen"
    pub participants: Vec<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartCallRequest {
    pub channel_id: String,
    pub call_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinCallRequest {
    pub call_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalingMessage {
    pub r#type: String, // "offer", "answer", "ice-candidate", "join", "leave"
    pub call_id: String,
    pub from_user: String,
    pub to_user: Option<String>,
    pub sdp: Option<String>,
    pub candidate: Option<serde_json::Value>,
}

// ============================================
// State Management
// ============================================

#[derive(Clone)]
pub struct WebRTCState {
    pub db: SqlitePool,
    pub signaling_tx: Arc<RwLock<broadcast::Sender<SignalingMessage>>>,
    pub active_calls: Arc<RwLock<HashMap<String, CallSession>>>,
    pub peer_connections: Arc<RwLock<HashMap<String, Vec<String>>>>, // call_id -> user_ids
}

impl WebRTCState {
    pub fn new(db: SqlitePool) -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self {
            db,
            signaling_tx: Arc::new(RwLock::new(tx)),
            active_calls: Arc::new(RwLock::new(HashMap::new())),
            peer_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn broadcast_signaling(&self, message: SignalingMessage) {
        let tx = self.signaling_tx.read().await;
        let _ = tx.send(message);
    }
}

// ============================================
// API Endpoints
// ============================================

/// Start a new call
pub async fn start_call(
    State(state): State<WebRTCState>,
    Json(req): Json<StartCallRequest>,
) -> Result<Json<CallSession>, StatusCode> {
    let call_id = Uuid::new_v4().to_string();
    let initiator = "admin".to_string(); // TODO: Get from session
    let started_at = chrono::Utc::now().to_rfc3339();

    let call_session = CallSession {
        id: call_id.clone(),
        channel_id: req.channel_id.clone(),
        initiator: initiator.clone(),
        call_type: req.call_type.clone(),
        participants: vec![initiator.clone()],
        started_at: started_at.clone(),
        ended_at: None,
    };

    // Store in database
    sqlx::query(
        "INSERT INTO webrtc_calls (id, channel_id, initiator, call_type, started_at) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&call_id)
    .bind(&req.channel_id)
    .bind(&initiator)
    .bind(&req.call_type)
    .bind(&started_at)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create call session: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Add to active calls
    {
        let mut active_calls = state.active_calls.write().await;
        active_calls.insert(call_id.clone(), call_session.clone());
    }

    {
        let mut connections = state.peer_connections.write().await;
        connections.insert(call_id.clone(), vec![initiator]);
    }

    Ok(Json(call_session))
}

/// Join an existing call
pub async fn join_call(
    State(state): State<WebRTCState>,
    Json(req): Json<JoinCallRequest>,
) -> Result<Json<CallSession>, StatusCode> {
    let user_id = "admin".to_string(); // TODO: Get from session

    // Get call session
    let mut call_session = {
        let active_calls = state.active_calls.read().await;
        active_calls.get(&req.call_id).cloned()
            .ok_or(StatusCode::NOT_FOUND)?
    };

    // Add user to participants
    if !call_session.participants.contains(&user_id) {
        call_session.participants.push(user_id.clone());

        // Update in memory
        {
            let mut active_calls = state.active_calls.write().await;
            active_calls.insert(req.call_id.clone(), call_session.clone());
        }

        {
            let mut connections = state.peer_connections.write().await;
            if let Some(participants) = connections.get_mut(&req.call_id) {
                participants.push(user_id.clone());
            }
        }

        // Update database
        sqlx::query(
            "INSERT INTO webrtc_participants (id, call_id, user_id, joined_at) VALUES (?, ?, ?, ?)"
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&req.call_id)
        .bind(&user_id)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add participant: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    Ok(Json(call_session))
}

/// End a call
pub async fn end_call(
    State(state): State<WebRTCState>,
    Path(call_id): Path<String>,
) -> Result<Json<String>, StatusCode> {
    let ended_at = chrono::Utc::now().to_rfc3339();

    // Update database
    sqlx::query("UPDATE webrtc_calls SET ended_at = ? WHERE id = ?")
        .bind(&ended_at)
        .bind(&call_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to end call: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Remove from active calls
    {
        let mut active_calls = state.active_calls.write().await;
        active_calls.remove(&call_id);
    }

    {
        let mut connections = state.peer_connections.write().await;
        connections.remove(&call_id);
    }

    Ok(Json("OK".to_string()))
}

/// Leave a call
pub async fn leave_call(
    State(state): State<WebRTCState>,
    Path(call_id): Path<String>,
) -> Result<Json<String>, StatusCode> {
    let user_id = "admin".to_string(); // TODO: Get from session

    // Remove from participants
    {
        let mut active_calls = state.active_calls.write().await;
        if let Some(call) = active_calls.get_mut(&call_id) {
            call.participants.retain(|u| u != &user_id);
        }
    }

    {
        let mut connections = state.peer_connections.write().await;
        if let Some(participants) = connections.get_mut(&call_id) {
            participants.retain(|u| u != &user_id);
        }
    }

    // Broadcast leave message
    state.broadcast_signaling(SignalingMessage {
        r#type: "leave".to_string(),
        call_id: call_id.clone(),
        from_user: user_id,
        to_user: None,
        sdp: None,
        candidate: None,
    }).await;

    Ok(Json("OK".to_string()))
}

/// Get active calls for a channel
pub async fn get_active_calls(
    State(state): State<WebRTCState>,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<CallSession>>, StatusCode> {
    let active_calls = state.active_calls.read().await;
    let calls: Vec<CallSession> = active_calls
        .values()
        .filter(|c| c.channel_id == channel_id)
        .cloned()
        .collect();

    Ok(Json(calls))
}

// ============================================
// WebSocket Signaling Handler
// ============================================

pub async fn webrtc_signaling_handler(
    ws: WebSocketUpgrade,
    State(state): State<WebRTCState>,
) -> Response {
    ws.on_upgrade(|socket| handle_webrtc_signaling(socket, state))
}

async fn handle_webrtc_signaling(socket: WebSocket, state: WebRTCState) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to signaling broadcast
    let mut rx = {
        let tx = state.signaling_tx.read().await;
        tx.subscribe()
    };

    // Forward broadcast messages to WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Ok(message) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&message) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming WebSocket messages
    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                tracing::info!("Received WebRTC signaling: {}", text);

                // Parse and broadcast the signaling message
                if let Ok(signaling_msg) = serde_json::from_str::<SignalingMessage>(&text) {
                    state_clone.broadcast_signaling(signaling_msg).await;
                }
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
