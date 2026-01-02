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
pub struct VirtualOfficeSpace {
    pub id: String,
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub tile_size: i32,
    pub background_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualOfficeUser {
    pub id: String,
    pub space_id: String,
    pub user_id: String,
    pub username: String,
    pub x: i32,
    pub y: i32,
    pub direction: String,
    pub avatar: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualOfficeObject {
    pub id: String,
    pub space_id: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub object_type: String,
    pub properties: Option<String>,
    pub walkable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionUpdate {
    pub user_id: String,
    pub username: String,
    pub x: i32,
    pub y: i32,
    pub direction: String,
    pub avatar: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProximityEvent {
    pub event_type: String, // "enter" or "leave"
    pub user_id: String,
    pub username: String,
    pub nearby_users: Vec<String>,
}

// ============================================
// State Management
// ============================================

#[derive(Clone)]
pub struct VirtualOfficeState {
    pub db: SqlitePool,
    pub position_tx: Arc<RwLock<broadcast::Sender<PositionUpdate>>>,
    pub proximity_tx: Arc<RwLock<broadcast::Sender<ProximityEvent>>>,
    pub active_users: Arc<RwLock<HashMap<String, VirtualOfficeUser>>>,
    pub proximity_threshold: i32, // Distance in tiles for proximity detection
}

impl VirtualOfficeState {
    pub fn new(db: SqlitePool) -> Self {
        let (pos_tx, _) = broadcast::channel(1000);
        let (prox_tx, _) = broadcast::channel(1000);
        Self {
            db,
            position_tx: Arc::new(RwLock::new(pos_tx)),
            proximity_tx: Arc::new(RwLock::new(prox_tx)),
            active_users: Arc::new(RwLock::new(HashMap::new())),
            proximity_threshold: 3, // 3 tiles proximity
        }
    }

    pub async fn broadcast_position(&self, update: PositionUpdate) {
        let tx = self.position_tx.read().await;
        let _ = tx.send(update);
    }

    pub async fn broadcast_proximity(&self, event: ProximityEvent) {
        let tx = self.proximity_tx.read().await;
        let _ = tx.send(event);
    }

    pub async fn check_proximity(&self, user_id: &str, x: i32, y: i32) -> Vec<String> {
        let users = self.active_users.read().await;
        let mut nearby = Vec::new();

        for (uid, user) in users.iter() {
            if uid != user_id {
                let distance = ((user.x - x).pow(2) + (user.y - y).pow(2)) as f32;
                let distance = distance.sqrt();
                if distance <= self.proximity_threshold as f32 {
                    nearby.push(uid.clone());
                }
            }
        }

        nearby
    }
}

// ============================================
// API Endpoints
// ============================================

/// Get virtual office space
pub async fn get_space(
    State(state): State<VirtualOfficeState>,
    Path(space_id): Path<String>,
) -> Result<Json<VirtualOfficeSpace>, StatusCode> {
    let space = sqlx::query_as::<_, (String, String, i32, i32, i32, String)>(
        "SELECT id, name, width, height, tile_size, background_color FROM virtual_office_spaces WHERE id = ?"
    )
    .bind(&space_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch space: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(VirtualOfficeSpace {
        id: space.0,
        name: space.1,
        width: space.2,
        height: space.3,
        tile_size: space.4,
        background_color: space.5,
    }))
}

/// Get all virtual office spaces
pub async fn list_spaces(
    State(state): State<VirtualOfficeState>,
) -> Result<Json<Vec<VirtualOfficeSpace>>, StatusCode> {
    let spaces = sqlx::query_as::<_, (String, String, i32, i32, i32, String)>(
        "SELECT id, name, width, height, tile_size, background_color FROM virtual_office_spaces"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch spaces: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let result = spaces
        .into_iter()
        .map(|(id, name, width, height, tile_size, background_color)| VirtualOfficeSpace {
            id,
            name,
            width,
            height,
            tile_size,
            background_color,
        })
        .collect();

    Ok(Json(result))
}

/// Get objects in a space
pub async fn get_objects(
    State(state): State<VirtualOfficeState>,
    Path(space_id): Path<String>,
) -> Result<Json<Vec<VirtualOfficeObject>>, StatusCode> {
    let objects = sqlx::query_as::<_, (String, String, i32, i32, i32, i32, String, Option<String>, i32)>(
        "SELECT id, space_id, x, y, width, height, object_type, properties, walkable FROM virtual_office_objects WHERE space_id = ?"
    )
    .bind(&space_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch objects: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let result = objects
        .into_iter()
        .map(|(id, space_id, x, y, width, height, object_type, properties, walkable)| VirtualOfficeObject {
            id,
            space_id,
            x,
            y,
            width,
            height,
            object_type,
            properties,
            walkable: walkable != 0,
        })
        .collect();

    Ok(Json(result))
}

/// Get active users in a space
pub async fn get_active_users(
    State(state): State<VirtualOfficeState>,
    Path(space_id): Path<String>,
) -> Result<Json<Vec<VirtualOfficeUser>>, StatusCode> {
    let users = state.active_users.read().await;
    let result: Vec<VirtualOfficeUser> = users
        .values()
        .filter(|u| u.space_id == space_id)
        .cloned()
        .collect();

    Ok(Json(result))
}

// ============================================
// WebSocket Handler
// ============================================

pub async fn virtual_office_handler(
    ws: WebSocketUpgrade,
    State(state): State<VirtualOfficeState>,
) -> Response {
    ws.on_upgrade(|socket| handle_virtual_office(socket, state))
}

async fn handle_virtual_office(socket: WebSocket, state: VirtualOfficeState) {
    let (mut sender, mut receiver) = socket.split();

    let user_id = Uuid::new_v4().to_string();
    let username = "admin".to_string(); // TODO: Get from session
    let space_id = "default".to_string(); // TODO: Get from request

    // Get or create user in database
    let existing_user = sqlx::query_as::<_, (String, String, String, String, i32, i32, String, String, String)>(
        "SELECT id, space_id, user_id, username, x, y, direction, avatar, status FROM virtual_office_users WHERE user_id = ?"
    )
    .bind(&user_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let mut current_user = if let Some(user) = existing_user {
        VirtualOfficeUser {
            id: user.0,
            space_id: user.1,
            user_id: user.2,
            username: user.3,
            x: user.4,
            y: user.5,
            direction: user.6,
            avatar: user.7,
            status: user.8,
        }
    } else {
        // Create new user
        let id = Uuid::new_v4().to_string();
        let new_user = VirtualOfficeUser {
            id: id.clone(),
            space_id: space_id.clone(),
            user_id: user_id.clone(),
            username: username.clone(),
            x: 5,
            y: 5,
            direction: "down".to_string(),
            avatar: "👤".to_string(),
            status: "online".to_string(),
        };

        sqlx::query(
            "INSERT INTO virtual_office_users (id, space_id, user_id, username, x, y, direction, avatar, status, last_update) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(&space_id)
        .bind(&user_id)
        .bind(&username)
        .bind(5)
        .bind(5)
        .bind("down")
        .bind("👤")
        .bind("online")
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&state.db)
        .await
        .ok();

        new_user
    };

    // Add to active users
    {
        let mut users = state.active_users.write().await;
        users.insert(user_id.clone(), current_user.clone());
    }

    // Subscribe to position updates
    let mut pos_rx = {
        let tx = state.position_tx.read().await;
        tx.subscribe()
    };

    // Subscribe to proximity events
    let mut prox_rx = {
        let tx = state.proximity_tx.read().await;
        tx.subscribe()
    };

    // Forward broadcasts to WebSocket
    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(pos_update) = pos_rx.recv() => {
                    let msg = serde_json::json!({
                        "type": "position",
                        "data": pos_update
                    });
                    if let Ok(json) = serde_json::to_string(&msg) {
                        if sender.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                }
                Ok(prox_event) = prox_rx.recv() => {
                    let msg = serde_json::json!({
                        "type": "proximity",
                        "data": prox_event
                    });
                    if let Ok(json) = serde_json::to_string(&msg) {
                        if sender.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    // Handle incoming messages
    let state_clone = state.clone();
    let user_id_clone = user_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if let Ok(update) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(msg_type) = update.get("type").and_then(|v| v.as_str()) {
                        match msg_type {
                            "move" => {
                                if let (Some(x), Some(y), Some(dir)) = (
                                    update.get("x").and_then(|v| v.as_i64()),
                                    update.get("y").and_then(|v| v.as_i64()),
                                    update.get("direction").and_then(|v| v.as_str()),
                                ) {
                                    let x = x as i32;
                                    let y = y as i32;

                                    // Update user position
                                    current_user.x = x;
                                    current_user.y = y;
                                    current_user.direction = dir.to_string();

                                    // Update in active users
                                    {
                                        let mut users = state_clone.active_users.write().await;
                                        users.insert(user_id_clone.clone(), current_user.clone());
                                    }

                                    // Update database
                                    sqlx::query(
                                        "UPDATE virtual_office_users SET x = ?, y = ?, direction = ?, last_update = ? WHERE user_id = ?"
                                    )
                                    .bind(x)
                                    .bind(y)
                                    .bind(dir)
                                    .bind(chrono::Utc::now().to_rfc3339())
                                    .bind(&user_id_clone)
                                    .execute(&state_clone.db)
                                    .await
                                    .ok();

                                    // Broadcast position update
                                    state_clone.broadcast_position(PositionUpdate {
                                        user_id: user_id_clone.clone(),
                                        username: current_user.username.clone(),
                                        x,
                                        y,
                                        direction: dir.to_string(),
                                        avatar: current_user.avatar.clone(),
                                    }).await;

                                    // Check proximity
                                    let nearby = state_clone.check_proximity(&user_id_clone, x, y).await;
                                    if !nearby.is_empty() {
                                        state_clone.broadcast_proximity(ProximityEvent {
                                            event_type: "nearby".to_string(),
                                            user_id: user_id_clone.clone(),
                                            username: current_user.username.clone(),
                                            nearby_users: nearby,
                                        }).await;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    // Clean up - remove from active users
    {
        let mut users = state.active_users.write().await;
        users.remove(&user_id);
    }
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

        // Create tables
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS virtual_office_spaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                width INTEGER NOT NULL DEFAULT 50,
                height INTEGER NOT NULL DEFAULT 50,
                tile_size INTEGER NOT NULL DEFAULT 32,
                background_color TEXT DEFAULT '#1a1a1a',
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"
        )
        .execute(&pool)
        .await
        .expect("Failed to create spaces table");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS virtual_office_users (
                id TEXT PRIMARY KEY,
                space_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                username TEXT NOT NULL,
                x INTEGER NOT NULL DEFAULT 5,
                y INTEGER NOT NULL DEFAULT 5,
                direction TEXT NOT NULL DEFAULT 'down',
                avatar TEXT DEFAULT '👤',
                status TEXT DEFAULT 'online',
                last_update DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (space_id) REFERENCES virtual_office_spaces(id) ON DELETE CASCADE
            )"
        )
        .execute(&pool)
        .await
        .expect("Failed to create users table");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS virtual_office_objects (
                id TEXT PRIMARY KEY,
                space_id TEXT NOT NULL,
                x INTEGER NOT NULL,
                y INTEGER NOT NULL,
                width INTEGER NOT NULL DEFAULT 1,
                height INTEGER NOT NULL DEFAULT 1,
                object_type TEXT NOT NULL,
                properties TEXT,
                walkable INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (space_id) REFERENCES virtual_office_spaces(id) ON DELETE CASCADE
            )"
        )
        .execute(&pool)
        .await
        .expect("Failed to create objects table");

        pool
    }

    #[tokio::test]
    async fn test_virtual_office_state_creation() {
        let pool = setup_test_db().await;
        let state = VirtualOfficeState::new(pool.clone());

        assert_eq!(state.proximity_threshold, 3);
        assert!(state.active_users.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_create_space() {
        let pool = setup_test_db().await;
        let space_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO virtual_office_spaces (id, name, width, height, tile_size, background_color)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&space_id)
        .bind("Test Office")
        .bind(50)
        .bind(50)
        .bind(32)
        .bind("#1a1a1a")
        .execute(&pool)
        .await
        .expect("Failed to insert space");

        let space: (String, String, i32, i32) = sqlx::query_as(
            "SELECT id, name, width, height FROM virtual_office_spaces WHERE id = ?"
        )
        .bind(&space_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch space");

        assert_eq!(space.0, space_id);
        assert_eq!(space.1, "Test Office");
        assert_eq!(space.2, 50);
        assert_eq!(space.3, 50);
    }

    #[tokio::test]
    async fn test_create_user() {
        let pool = setup_test_db().await;
        let space_id = Uuid::new_v4().to_string();
        let user_id = Uuid::new_v4().to_string();

        // Create space first
        sqlx::query(
            "INSERT INTO virtual_office_spaces (id, name, width, height, tile_size, background_color)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&space_id)
        .bind("Test Office")
        .bind(50)
        .bind(50)
        .bind(32)
        .bind("#1a1a1a")
        .execute(&pool)
        .await
        .expect("Failed to insert space");

        // Create user
        sqlx::query(
            "INSERT INTO virtual_office_users (id, space_id, user_id, username, x, y, direction, avatar, status)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&user_id)
        .bind(&space_id)
        .bind("test_user_id")
        .bind("TestUser")
        .bind(10)
        .bind(10)
        .bind("down")
        .bind("👤")
        .bind("online")
        .execute(&pool)
        .await
        .expect("Failed to insert user");

        let user: (String, String, i32, i32) = sqlx::query_as(
            "SELECT id, username, x, y FROM virtual_office_users WHERE id = ?"
        )
        .bind(&user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch user");

        assert_eq!(user.0, user_id);
        assert_eq!(user.1, "TestUser");
        assert_eq!(user.2, 10);
        assert_eq!(user.3, 10);
    }

    #[tokio::test]
    async fn test_proximity_detection() {
        let pool = setup_test_db().await;
        let state = VirtualOfficeState::new(pool.clone());

        // Add two users close to each other
        let user1_id = "user1".to_string();
        let user2_id = "user2".to_string();
        let user3_id = "user3".to_string();

        {
            let mut users = state.active_users.write().await;
            users.insert(user1_id.clone(), VirtualOfficeUser {
                id: user1_id.clone(),
                space_id: "space1".to_string(),
                user_id: user1_id.clone(),
                username: "User1".to_string(),
                x: 10,
                y: 10,
                direction: "down".to_string(),
                avatar: "👤".to_string(),
                status: "online".to_string(),
            });

            users.insert(user2_id.clone(), VirtualOfficeUser {
                id: user2_id.clone(),
                space_id: "space1".to_string(),
                user_id: user2_id.clone(),
                username: "User2".to_string(),
                x: 12, // Within 3 tiles
                y: 10,
                direction: "down".to_string(),
                avatar: "👤".to_string(),
                status: "online".to_string(),
            });

            users.insert(user3_id.clone(), VirtualOfficeUser {
                id: user3_id.clone(),
                space_id: "space1".to_string(),
                user_id: user3_id.clone(),
                username: "User3".to_string(),
                x: 20, // Far away
                y: 20,
                direction: "down".to_string(),
                avatar: "👤".to_string(),
                status: "online".to_string(),
            });
        }

        let nearby = state.check_proximity(&user1_id, 10, 10).await;
        assert_eq!(nearby.len(), 1);
        assert_eq!(nearby[0], user2_id);
    }

    #[tokio::test]
    async fn test_create_objects() {
        let pool = setup_test_db().await;
        let space_id = Uuid::new_v4().to_string();
        let object_id = Uuid::new_v4().to_string();

        // Create space first
        sqlx::query(
            "INSERT INTO virtual_office_spaces (id, name, width, height, tile_size, background_color)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&space_id)
        .bind("Test Office")
        .bind(50)
        .bind(50)
        .bind(32)
        .bind("#1a1a1a")
        .execute(&pool)
        .await
        .expect("Failed to insert space");

        // Create object (wall)
        sqlx::query(
            "INSERT INTO virtual_office_objects (id, space_id, x, y, width, height, object_type, properties, walkable)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&object_id)
        .bind(&space_id)
        .bind(5)
        .bind(5)
        .bind(8)
        .bind(1)
        .bind("wall")
        .bind("{\"color\":\"#555\"}")
        .bind(0)
        .execute(&pool)
        .await
        .expect("Failed to insert object");

        let object: (String, i32, i32, i32, i32, String) = sqlx::query_as(
            "SELECT id, x, y, width, height, object_type FROM virtual_office_objects WHERE id = ?"
        )
        .bind(&object_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch object");

        assert_eq!(object.0, object_id);
        assert_eq!(object.1, 5);
        assert_eq!(object.2, 5);
        assert_eq!(object.3, 8);
        assert_eq!(object.4, 1);
        assert_eq!(object.5, "wall");
    }

    #[tokio::test]
    async fn test_position_update_message() {
        let update = PositionUpdate {
            user_id: "user1".to_string(),
            username: "TestUser".to_string(),
            x: 15,
            y: 20,
            direction: "right".to_string(),
            avatar: "👤".to_string(),
        };

        let json = serde_json::to_string(&update).expect("Failed to serialize");
        let parsed: PositionUpdate = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(parsed.user_id, "user1");
        assert_eq!(parsed.x, 15);
        assert_eq!(parsed.y, 20);
        assert_eq!(parsed.direction, "right");
    }

    #[tokio::test]
    async fn test_proximity_event_message() {
        let event = ProximityEvent {
            event_type: "nearby".to_string(),
            user_id: "user1".to_string(),
            username: "TestUser".to_string(),
            nearby_users: vec!["user2".to_string(), "user3".to_string()],
        };

        let json = serde_json::to_string(&event).expect("Failed to serialize");
        let parsed: ProximityEvent = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(parsed.event_type, "nearby");
        assert_eq!(parsed.nearby_users.len(), 2);
        assert!(parsed.nearby_users.contains(&"user2".to_string()));
    }
}
