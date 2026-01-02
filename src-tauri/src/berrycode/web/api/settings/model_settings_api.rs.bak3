//! Model Settings API for task-specific AI model configuration

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::berrycode::web::infrastructure::database::Database;

/// Model Settings API state
#[derive(Clone)]
pub struct ModelSettingsApiState {
    pub db: Database,
}

/// Task types for different development activities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    Design,
    Implementation,
    Review,
    Test,
    Debug,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::Design => "design",
            TaskType::Implementation => "implementation",
            TaskType::Review => "review",
            TaskType::Test => "test",
            TaskType::Debug => "debug",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "design" => Some(TaskType::Design),
            "implementation" => Some(TaskType::Implementation),
            "review" => Some(TaskType::Review),
            "test" => Some(TaskType::Test),
            "debug" => Some(TaskType::Debug),
            _ => None,
        }
    }

    pub fn all() -> Vec<TaskType> {
        vec![
            TaskType::Design,
            TaskType::Implementation,
            TaskType::Review,
            TaskType::Test,
            TaskType::Debug,
        ]
    }
}

/// Model settings response
#[derive(Debug, Serialize)]
pub struct ModelSettingsResponse {
    pub settings: HashMap<String, String>,
}

/// Save model settings request
#[derive(Debug, Deserialize)]
pub struct SaveModelSettingsRequest {
    pub settings: HashMap<String, String>,
}

/// Available AI model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableModel {
    pub name: String,
    pub display_name: String,
    pub provider: String,
    pub max_context_tokens: u32,
    pub input_cost: f64,  // $ per 1M tokens
    pub output_cost: f64, // $ per 1M tokens
    pub supports_vision: bool,

    /// Optional: Custom API endpoint (e.g., "http://localhost:11434/v1" for Ollama)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_base_url: Option<String>,

    /// Optional: Environment variable name for API key (e.g., "MY_COMPANY_API_KEY")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
}

/// Default model for each task type
pub fn default_model_for_task(task_type: &TaskType) -> &'static str {
    match task_type {
        TaskType::Design => "gpt-5.1-high",        // 総合知能 No.1 - アーキテクチャ設計最強
        TaskType::Implementation => "gpt-5.1-high", // 実装力 No.1 - LiveCodeBench 87%
        TaskType::Review => "claude-4.5-sonnet",    // 推論・知識 No.1 - 人間味のあるレビュー
        TaskType::Test => "grok-4-fast",            // コスパ最強 - 200万トークンコンテキスト
        TaskType::Debug => "gemini-2.5-flash-lite", // 速度 No.1 - 662トークン/秒
    }
}

/// Default model for task type from string
pub fn default_model_for_task_str(task_type_str: &str) -> String {
    match TaskType::from_str(task_type_str) {
        Some(task_type) => default_model_for_task(&task_type).to_string(),
        None => std::env::var("BERRYCODE_MODEL").unwrap_or_else(|_| "gpt-4o".to_string()),
    }
}

/// Get default model settings
pub fn get_default_settings() -> HashMap<String, String> {
    let mut settings = HashMap::new();
    for task_type in TaskType::all() {
        settings.insert(
            task_type.as_str().to_string(),
            default_model_for_task(&task_type).to_string(),
        );
    }
    settings
}

/// Validate model name
pub fn is_valid_model_name(model_name: &str) -> bool {
    let valid_models = get_available_models();
    valid_models.iter().any(|m| m.name == model_name)
}

/// Load custom models from environment variable
fn load_custom_models() -> Vec<AvailableModel> {
    if let Ok(custom_json) = std::env::var("BERRYCODE_CUSTOM_MODELS") {
        match serde_json::from_str::<Vec<AvailableModel>>(&custom_json) {
            Ok(models) => {
                tracing::info!("Loaded {} custom models from BERRYCODE_CUSTOM_MODELS", models.len());
                return models;
            }
            Err(e) => {
                tracing::warn!("Failed to parse BERRYCODE_CUSTOM_MODELS: {}", e);
            }
        }
    }
    Vec::new()
}

/// Get list of available AI models
pub fn get_available_models() -> Vec<AvailableModel> {
    let mut models = vec![
        // OpenAI models (Latest Generation)
        AvailableModel {
            name: "gpt-5.1-high".to_string(),
            display_name: "GPT-5.1 (high)".to_string(),
            provider: "OpenAI".to_string(),
            max_context_tokens: 200000,
            input_cost: 5.0,
            output_cost: 15.0,
            supports_vision: true,
            api_base_url: None,
            api_key_env: Some("OPENAI_API_KEY".to_string()),
        },
        AvailableModel {
            name: "gpt-4o".to_string(),
            display_name: "GPT-4o".to_string(),
            provider: "OpenAI".to_string(),
            max_context_tokens: 128000,
            input_cost: 2.50,
            output_cost: 10.0,
            supports_vision: true,
            api_base_url: None,
            api_key_env: Some("OPENAI_API_KEY".to_string()),
        },
        AvailableModel {
            name: "gpt-4o-mini".to_string(),
            display_name: "GPT-4o Mini".to_string(),
            provider: "OpenAI".to_string(),
            max_context_tokens: 128000,
            input_cost: 0.15,
            output_cost: 0.60,
            supports_vision: true,
            api_base_url: None,
            api_key_env: Some("OPENAI_API_KEY".to_string()),
        },
        AvailableModel {
            name: "gpt-4-turbo".to_string(),
            display_name: "GPT-4 Turbo".to_string(),
            provider: "OpenAI".to_string(),
            max_context_tokens: 128000,
            input_cost: 10.0,
            output_cost: 30.0,
            supports_vision: true,
            api_base_url: None,
            api_key_env: Some("OPENAI_API_KEY".to_string()),
        },
        AvailableModel {
            name: "gpt-3.5-turbo".to_string(),
            display_name: "GPT-3.5 Turbo".to_string(),
            provider: "OpenAI".to_string(),
            max_context_tokens: 16385,
            input_cost: 0.50,
            output_cost: 1.50,
            supports_vision: false,
            api_base_url: None,
            api_key_env: Some("OPENAI_API_KEY".to_string()),
        },
        // Anthropic models (Latest Generation)
        AvailableModel {
            name: "claude-4.5-sonnet".to_string(),
            display_name: "Claude 4.5 Sonnet".to_string(),
            provider: "Anthropic".to_string(),
            max_context_tokens: 200000,
            input_cost: 3.00,
            output_cost: 15.0,
            supports_vision: true,
            api_base_url: None,
            api_key_env: Some("ANTHROPIC_API_KEY".to_string()),
        },
        AvailableModel {
            name: "claude-3-5-sonnet-20241022".to_string(),
            display_name: "Claude 3.5 Sonnet".to_string(),
            provider: "Anthropic".to_string(),
            max_context_tokens: 200000,
            input_cost: 3.00,
            output_cost: 15.0,
            supports_vision: true,
            api_base_url: None,
            api_key_env: Some("ANTHROPIC_API_KEY".to_string()),
        },
        AvailableModel {
            name: "claude-3-opus-20240229".to_string(),
            display_name: "Claude 3 Opus".to_string(),
            provider: "Anthropic".to_string(),
            max_context_tokens: 200000,
            input_cost: 15.0,
            output_cost: 75.0,
            supports_vision: true,
            api_base_url: None,
            api_key_env: Some("ANTHROPIC_API_KEY".to_string()),
        },
        AvailableModel {
            name: "claude-3-haiku-20240307".to_string(),
            display_name: "Claude 3 Haiku".to_string(),
            provider: "Anthropic".to_string(),
            max_context_tokens: 200000,
            input_cost: 0.25,
            output_cost: 1.25,
            supports_vision: true,
            api_base_url: None,
            api_key_env: Some("ANTHROPIC_API_KEY".to_string()),
        },
        // xAI models
        AvailableModel {
            name: "grok-4-fast".to_string(),
            display_name: "Grok 4 Fast".to_string(),
            provider: "xAI".to_string(),
            max_context_tokens: 2000000, // 200万トークン
            input_cost: 0.50,
            output_cost: 0.50,
            supports_vision: true,
            api_base_url: None,
            api_key_env: Some("XAI_API_KEY".to_string()),
        },
        // Google models
        AvailableModel {
            name: "gemini-2.5-flash-lite".to_string(),
            display_name: "Gemini 2.5 Flash-Lite".to_string(),
            provider: "Google".to_string(),
            max_context_tokens: 1000000,
            input_cost: 0.10,
            output_cost: 0.30,
            supports_vision: true,
            api_base_url: None,
            api_key_env: Some("GOOGLE_API_KEY".to_string()),
        },
        AvailableModel {
            name: "gemini-2.0-flash".to_string(),
            display_name: "Gemini 2.0 Flash".to_string(),
            provider: "Google".to_string(),
            max_context_tokens: 1000000,
            input_cost: 0.075,
            output_cost: 0.30,
            supports_vision: true,
            api_base_url: None,
            api_key_env: Some("GOOGLE_API_KEY".to_string()),
        },
    ];

    // Add custom models from environment variable
    let custom_models = load_custom_models();
    models.extend(custom_models);

    models
}

/// GET /api/model-settings/:session_id - Get model settings for a session
pub async fn get_model_settings(
    Path(session_id): Path<String>,
    State(state): State<ModelSettingsApiState>,
) -> Result<Json<ModelSettingsResponse>, StatusCode> {
    // Get settings from database
    let mut settings = state.db
        .get_model_settings(&session_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get model settings: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Fill in defaults for missing task types
    let default_settings = get_default_settings();
    for (task_type, default_model) in default_settings {
        settings.entry(task_type).or_insert(default_model);
    }

    Ok(Json(ModelSettingsResponse { settings }))
}

/// POST /api/model-settings/:session_id - Save model settings for a session
pub async fn save_model_settings(
    Path(session_id): Path<String>,
    State(state): State<ModelSettingsApiState>,
    Json(request): Json<SaveModelSettingsRequest>,
) -> Result<Json<ModelSettingsResponse>, StatusCode> {
    // Validate all model names
    for (task_type, model_name) in &request.settings {
        if !is_valid_model_name(model_name) {
            tracing::warn!("Invalid model name '{}' for task type '{}'", model_name, task_type);
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Save to database
    state.db.save_model_settings(&session_id, &request.settings)
        .await
        .map_err(|e| {
            tracing::error!("Failed to save model settings: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Return updated settings
    let settings = state.db
        .get_model_settings(&session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ModelSettingsResponse { settings }))
}

/// GET /api/models/list - Get list of available AI models
pub async fn list_available_models() -> Result<Json<Vec<AvailableModel>>, StatusCode> {
    Ok(Json(get_available_models()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_type_conversion() {
        assert_eq!(TaskType::from_str("design"), Some(TaskType::Design));
        assert_eq!(TaskType::from_str("DESIGN"), Some(TaskType::Design));
        assert_eq!(TaskType::from_str("implementation"), Some(TaskType::Implementation));
        assert_eq!(TaskType::from_str("invalid"), None);
    }

    #[test]
    fn test_default_models() {
        assert_eq!(default_model_for_task(&TaskType::Design), "gpt-5.1-high");
        assert_eq!(default_model_for_task(&TaskType::Implementation), "gpt-5.1-high");
        assert_eq!(default_model_for_task(&TaskType::Review), "claude-4.5-sonnet");
        assert_eq!(default_model_for_task(&TaskType::Test), "grok-4-fast");
        assert_eq!(default_model_for_task(&TaskType::Debug), "gemini-2.5-flash-lite");
    }

    #[test]
    fn test_valid_model_names() {
        assert!(is_valid_model_name("gpt-4o"));
        assert!(is_valid_model_name("claude-3-5-sonnet-20241022"));
        assert!(!is_valid_model_name("invalid-model"));
    }

    #[test]
    fn test_get_default_settings() {
        let settings = get_default_settings();
        assert_eq!(settings.len(), 5);
        assert_eq!(settings.get("design"), Some(&"gpt-5.1-high".to_string()));
    }

    #[test]
    fn test_available_models_count() {
        let models = get_available_models();
        assert!(models.len() >= 12); // At least 12 models (5 OpenAI + 4 Anthropic + 1 xAI + 2 Google)
    }

    #[test]
    fn test_model_cost_information() {
        let models = get_available_models();
        let gpt4o = models.iter().find(|m| m.name == "gpt-4o").unwrap();
        assert_eq!(gpt4o.input_cost, 2.50);
        assert_eq!(gpt4o.output_cost, 10.0);
    }
}
