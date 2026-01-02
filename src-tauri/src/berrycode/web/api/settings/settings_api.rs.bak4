//! Settings API
//!
//! Provides REST endpoints for loading and saving user settings.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    pub theme: String,
    pub font_size: u32,
    pub tab_size: u32,
    pub word_wrap: bool,
    pub minimap_enabled: bool,
    pub auto_save: bool,
    pub format_on_save: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            theme: "vs-dark".to_string(),
            font_size: 14,
            tab_size: 4,
            word_wrap: true,
            minimap_enabled: true,
            auto_save: true,
            format_on_save: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISettings {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub enable_inline_completions: bool,
}

impl Default for AISettings {
    fn default() -> Self {
        Self {
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: 2000,
            enable_inline_completions: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    pub editor: EditorSettings,
    pub ai: AISettings,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            editor: EditorSettings::default(),
            ai: AISettings::default(),
        }
    }
}

#[derive(Clone)]
pub struct SettingsApiState {
    settings_store: Arc<Mutex<HashMap<String, UserSettings>>>,
}

impl SettingsApiState {
    pub fn new() -> Self {
        Self {
            settings_store: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

/// Load settings for a session
pub async fn load_settings(
    State(state): State<SettingsApiState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let settings_store = state.settings_store.lock().await;

    let settings = settings_store
        .get(&session_id)
        .cloned()
        .unwrap_or_default();

    Json(settings)
}

/// Save settings for a session
pub async fn save_settings(
    State(state): State<SettingsApiState>,
    Path(session_id): Path<String>,
    Json(settings): Json<UserSettings>,
) -> impl IntoResponse {
    let mut settings_store = state.settings_store.lock().await;
    settings_store.insert(session_id.clone(), settings.clone());

    tracing::info!("Settings saved for session: {}", session_id);

    (StatusCode::OK, Json(settings))
}

#[derive(Debug, Serialize)]
struct SettingsResponse {
    success: bool,
    message: String,
}
