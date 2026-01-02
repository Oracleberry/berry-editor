//! AI-powered code completion (Copilot-style)
//! Uses OpenAI/Anthropic APIs for intelligent code suggestions

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::berrycode::web::api::settings::api_keys_api::get_decrypted_api_key;
use crate::berrycode::web::infrastructure::database::Database;

#[derive(Clone)]
pub struct AiCompletionState {
    openai_api_key: Option<String>,
    anthropic_api_key: Option<String>,
    db: Database,
}

impl AiCompletionState {
    pub fn new(db: Database) -> Self {
        Self {
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            db,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AiCompletionRequest {
    pub session_id: String,
    pub path: String,
    pub prefix: String,  // Code before cursor
    pub suffix: String,  // Code after cursor
    pub language: String,
}

#[derive(Debug, Serialize)]
pub struct AiCompletionResponse {
    pub success: bool,
    pub completions: Vec<String>,
    pub error: Option<String>,
}

/// AI code completion endpoint
pub async fn ai_complete(
    State(state): State<Arc<AiCompletionState>>,
    Json(request): Json<AiCompletionRequest>,
) -> impl IntoResponse {
    tracing::info!(
        session_id = %request.session_id,
        language = %request.language,
        "AI completion request"
    );

    // Get API keys from database first, then fall back to environment variables
    let openai_key = get_decrypted_api_key(&state.db, &request.session_id, "openai")
        .await
        .ok()
        .flatten()
        .or_else(|| state.openai_api_key.clone());

    let anthropic_key = get_decrypted_api_key(&state.db, &request.session_id, "anthropic")
        .await
        .ok()
        .flatten()
        .or_else(|| state.anthropic_api_key.clone());

    // Check if API keys are available
    if openai_key.is_none() && anthropic_key.is_none() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(AiCompletionResponse {
                success: false,
                completions: Vec::new(),
                error: Some("No AI API keys configured. Set API keys in Settings > API Keys or via environment variables.".to_string()),
            }),
        );
    }

    // Build prompt for AI
    let prompt = format!(
        r#"Complete the following {} code. Only return the completion, no explanations.

File: {}

Code before cursor:
```{}
{}
```

Code after cursor:
```{}
{}
```

Provide a single-line or multi-line completion that fits naturally between the prefix and suffix:"#,
        request.language,
        request.path,
        request.language,
        request.prefix,
        request.language,
        request.suffix
    );

    // Try OpenAI first, then Anthropic
    let completion = if let Some(api_key) = &openai_key {
        complete_with_openai(&prompt, api_key).await
    } else if let Some(api_key) = &anthropic_key {
        complete_with_anthropic(&prompt, api_key).await
    } else {
        Err("No API key available".to_string())
    };

    match completion {
        Ok(text) => (
            StatusCode::OK,
            Json(AiCompletionResponse {
                success: true,
                completions: vec![text],
                error: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AiCompletionResponse {
                success: false,
                completions: Vec::new(),
                error: Some(e),
            }),
        ),
    }
}

async fn complete_with_openai(prompt: &str, api_key: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "gpt-4o-mini",
            "messages": [
                {
                    "role": "system",
                    "content": "You are a code completion assistant. Provide concise, accurate code completions."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.3,
            "max_tokens": 500
        }))
        .send()
        .await
        .map_err(|e| format!("OpenAI API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("OpenAI API error {}: {}", status, error_text));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;

    let completion = data["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("No completion in response")?
        .to_string();

    // Clean up the completion (remove markdown code blocks if present)
    let cleaned = completion
        .trim()
        .trim_start_matches("```")
        .trim_start_matches("rust")
        .trim_start_matches("python")
        .trim_start_matches("javascript")
        .trim_start_matches("typescript")
        .trim_end_matches("```")
        .trim()
        .to_string();

    Ok(cleaned)
}

async fn complete_with_anthropic(prompt: &str, api_key: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-3-5-haiku-20241022",
            "max_tokens": 500,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.3
        }))
        .send()
        .await
        .map_err(|e| format!("Anthropic API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Anthropic API error {}: {}", status, error_text));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Anthropic response: {}", e))?;

    let completion = data["content"][0]["text"]
        .as_str()
        .ok_or("No completion in response")?
        .to_string();

    // Clean up the completion
    let cleaned = completion
        .trim()
        .trim_start_matches("```")
        .trim_start_matches("rust")
        .trim_start_matches("python")
        .trim_start_matches("javascript")
        .trim_start_matches("typescript")
        .trim_end_matches("```")
        .trim()
        .to_string();

    Ok(cleaned)
}
