//! API Keys management API with encrypted storage

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::berrycode::web::infrastructure::crypto::{decrypt_api_key, encrypt_api_key};
use crate::berrycode::web::infrastructure::database::Database;

/// API Keys API state
#[derive(Clone)]
pub struct ApiKeysApiState {
    pub db: Database,
}

/// API keys response (with masked keys for display)
#[derive(Debug, Serialize)]
pub struct ApiKeysResponse {
    pub keys: HashMap<String, ApiKeyInfo>,
}

/// API key information (for display)
#[derive(Debug, Serialize)]
pub struct ApiKeyInfo {
    pub provider: String,
    pub masked_key: String,
    pub is_set: bool,
}

/// Save API keys request
#[derive(Debug, Deserialize)]
pub struct SaveApiKeysRequest {
    pub keys: HashMap<String, String>, // provider -> plaintext_key
}

/// Mask API key for display (show first 8 chars, hide the rest)
fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "****".to_string();
    }

    let prefix_len = 8.min(key.len());
    let prefix = &key[..prefix_len];
    let masked_len = (key.len() - prefix_len).min(20);

    format!("{}{}...", prefix, "*".repeat(masked_len))
}

/// GET /api/api-keys/:session_id - Get all API keys for a session
pub async fn get_api_keys(
    Path(session_id): Path<String>,
    State(state): State<ApiKeysApiState>,
) -> Result<Json<ApiKeysResponse>, StatusCode> {
    // Get encrypted keys from database
    let encrypted_keys = state
        .db
        .get_all_api_keys(&session_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get API keys: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Decrypt and mask keys for response
    let mut keys = HashMap::new();

    for (provider, encrypted_key) in encrypted_keys {
        match decrypt_api_key(&encrypted_key) {
            Ok(plaintext) => {
                keys.insert(
                    provider.clone(),
                    ApiKeyInfo {
                        provider: provider.clone(),
                        masked_key: mask_api_key(&plaintext),
                        is_set: true,
                    },
                );
            }
            Err(e) => {
                tracing::error!("Failed to decrypt API key for {}: {}", provider, e);
                // Still show that a key exists, but mark it as invalid
                keys.insert(
                    provider.clone(),
                    ApiKeyInfo {
                        provider: provider.clone(),
                        masked_key: "[decryption failed]".to_string(),
                        is_set: false,
                    },
                );
            }
        }
    }

    Ok(Json(ApiKeysResponse { keys }))
}

/// POST /api/api-keys/:session_id - Save API keys for a session
pub async fn save_api_keys(
    Path(session_id): Path<String>,
    State(state): State<ApiKeysApiState>,
    Json(request): Json<SaveApiKeysRequest>,
) -> Result<Json<ApiKeysResponse>, StatusCode> {
    // Validate and encrypt keys
    for (provider, plaintext_key) in &request.keys {
        // Skip empty keys (allow deletion via empty string)
        if plaintext_key.is_empty() {
            state
                .db
                .delete_api_key(&session_id, provider)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to delete API key for {}: {}", provider, e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            continue;
        }

        // Basic validation
        if plaintext_key.len() < 8 {
            tracing::warn!("API key for {} is too short", provider);
            return Err(StatusCode::BAD_REQUEST);
        }

        // Encrypt the key
        let encrypted_key = encrypt_api_key(plaintext_key).map_err(|e| {
            tracing::error!("Failed to encrypt API key for {}: {}", provider, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        // Save to database
        state
            .db
            .save_api_key(&session_id, provider, &encrypted_key)
            .await
            .map_err(|e| {
                tracing::error!("Failed to save API key for {}: {}", provider, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    // Return updated keys (masked)
    get_api_keys(Path(session_id), State(state)).await
}

/// DELETE /api/api-keys/:session_id/:provider - Delete specific API key
pub async fn delete_api_key(
    Path((session_id, provider)): Path<(String, String)>,
    State(state): State<ApiKeysApiState>,
) -> Result<StatusCode, StatusCode> {
    state
        .db
        .delete_api_key(&session_id, &provider)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete API key for {}: {}", provider, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/api-keys/:session_id - Delete all API keys for a session
pub async fn delete_all_api_keys(
    Path(session_id): Path<String>,
    State(state): State<ApiKeysApiState>,
) -> Result<StatusCode, StatusCode> {
    state
        .db
        .delete_all_api_keys(&session_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete all API keys: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get decrypted API key for use (not exposed via HTTP)
pub async fn get_decrypted_api_key(
    db: &Database,
    session_id: &str,
    provider: &str,
) -> anyhow::Result<Option<String>> {
    if let Some(encrypted_key) = db.get_api_key(session_id, provider).await? {
        let plaintext = decrypt_api_key(&encrypted_key)?;
        Ok(Some(plaintext))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key() {
        assert_eq!(mask_api_key("sk-proj-12345678901234567890"), "sk-proj-********************...");
        assert_eq!(mask_api_key("short"), "****");
        assert_eq!(mask_api_key("sk-12345"), "****");
        assert_eq!(mask_api_key("sk-ant-api03-1234567890"), "sk-ant-a********************...");
    }

    #[tokio::test]
    async fn test_save_and_get_api_keys() {
        use crate::berrycode::web::infrastructure::database::Database;

        let db = Database::new("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        // Create a test session
        db.create_session("test-session-1", &std::path::PathBuf::from("/test/project"))
            .await
            .expect("Failed to create test session");

        let state = ApiKeysApiState { db: db.clone() };

        // Save API keys
        let mut keys_to_save = HashMap::new();
        keys_to_save.insert("openai".to_string(), "sk-proj-test1234567890".to_string());
        keys_to_save.insert("anthropic".to_string(), "sk-ant-test1234567890".to_string());

        let request = SaveApiKeysRequest { keys: keys_to_save };

        let result = save_api_keys(
            Path("test-session-1".to_string()),
            State(state.clone()),
            Json(request),
        )
        .await;

        assert!(result.is_ok());

        // Get API keys
        let result = get_api_keys(
            Path("test-session-1".to_string()),
            State(state),
        )
        .await;

        assert!(result.is_ok());
        let Json(response) = result.unwrap();

        assert_eq!(response.keys.len(), 2);
        assert!(response.keys.contains_key("openai"));
        assert!(response.keys.contains_key("anthropic"));

        // Keys should be masked
        let openai_key = &response.keys["openai"];
        assert!(openai_key.masked_key.contains("sk-proj-"));
        assert!(openai_key.masked_key.contains("*"));
        assert!(openai_key.is_set);
    }

    #[tokio::test]
    async fn test_delete_api_key() {
        use crate::berrycode::web::infrastructure::database::Database;

        let db = Database::new("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        db.create_session("test-session-1", &std::path::PathBuf::from("/test/project"))
            .await
            .expect("Failed to create test session");

        let state = ApiKeysApiState { db: db.clone() };

        // Save a key
        let mut keys = HashMap::new();
        keys.insert("openai".to_string(), "sk-test-12345678".to_string());

        let request = SaveApiKeysRequest { keys };
        save_api_keys(
            Path("test-session-1".to_string()),
            State(state.clone()),
            Json(request),
        )
        .await
        .expect("Failed to save key");

        // Delete it
        let result = delete_api_key(
            Path(("test-session-1".to_string(), "openai".to_string())),
            State(state.clone()),
        )
        .await;

        assert!(result.is_ok());

        // Verify deletion
        let result = get_api_keys(
            Path("test-session-1".to_string()),
            State(state),
        )
        .await
        .expect("Failed to get keys");

        assert_eq!(result.0.keys.len(), 0);
    }

    #[tokio::test]
    async fn test_get_decrypted_api_key_function() {
        use crate::berrycode::web::infrastructure::database::Database;

        let db = Database::new("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        db.create_session("test-session-1", &std::path::PathBuf::from("/test/project"))
            .await
            .expect("Failed to create test session");

        // Save encrypted key
        let plaintext = "sk-test-1234567890";
        let encrypted = encrypt_api_key(plaintext).expect("Encryption failed");

        db.save_api_key("test-session-1", "openai", &encrypted)
            .await
            .expect("Failed to save key");

        // Get decrypted key
        let result = get_decrypted_api_key(&db, "test-session-1", "openai")
            .await
            .expect("Failed to get key");

        assert_eq!(result, Some(plaintext.to_string()));

        // Try non-existent key
        let result = get_decrypted_api_key(&db, "test-session-1", "nonexistent")
            .await
            .expect("Failed to get key");

        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_save_empty_key_deletes() {
        use crate::berrycode::web::infrastructure::database::Database;

        let db = Database::new("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        db.create_session("test-session-1", &std::path::PathBuf::from("/test/project"))
            .await
            .expect("Failed to create test session");

        let state = ApiKeysApiState { db: db.clone() };

        // Save a key
        let mut keys = HashMap::new();
        keys.insert("openai".to_string(), "sk-test-12345678".to_string());
        let request = SaveApiKeysRequest { keys };
        save_api_keys(
            Path("test-session-1".to_string()),
            State(state.clone()),
            Json(request),
        )
        .await
        .expect("Failed to save key");

        // Verify key exists
        let result = get_api_keys(Path("test-session-1".to_string()), State(state.clone()))
            .await
            .unwrap();
        assert_eq!(result.0.keys.len(), 1);

        // Save empty key (should delete)
        let mut empty_keys = HashMap::new();
        empty_keys.insert("openai".to_string(), "".to_string());
        let request = SaveApiKeysRequest { keys: empty_keys };
        save_api_keys(
            Path("test-session-1".to_string()),
            State(state.clone()),
            Json(request),
        )
        .await
        .expect("Failed to save empty key");

        // Verify key is deleted
        let result = get_api_keys(Path("test-session-1".to_string()), State(state))
            .await
            .unwrap();
        assert_eq!(result.0.keys.len(), 0);
    }

    #[tokio::test]
    async fn test_save_invalid_key_format() {
        use crate::berrycode::web::infrastructure::database::Database;

        let db = Database::new("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        db.create_session("test-session-1", &std::path::PathBuf::from("/test/project"))
            .await
            .expect("Failed to create test session");

        let state = ApiKeysApiState { db: db.clone() };

        // Try to save key that's too short
        let mut keys = HashMap::new();
        keys.insert("openai".to_string(), "short".to_string());
        let request = SaveApiKeysRequest { keys };

        let result = save_api_keys(
            Path("test-session-1".to_string()),
            State(state),
            Json(request),
        )
        .await;

        // Should fail validation
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_delete_all_api_keys() {
        use crate::berrycode::web::infrastructure::database::Database;

        let db = Database::new("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        db.create_session("test-session-1", &std::path::PathBuf::from("/test/project"))
            .await
            .expect("Failed to create test session");

        let state = ApiKeysApiState { db: db.clone() };

        // Save multiple keys
        let mut keys = HashMap::new();
        keys.insert("openai".to_string(), "sk-test-12345678".to_string());
        keys.insert("anthropic".to_string(), "sk-ant-12345678".to_string());
        let request = SaveApiKeysRequest { keys };
        save_api_keys(
            Path("test-session-1".to_string()),
            State(state.clone()),
            Json(request),
        )
        .await
        .expect("Failed to save keys");

        // Delete all
        let result = delete_all_api_keys(
            Path("test-session-1".to_string()),
            State(state.clone()),
        )
        .await;

        assert!(result.is_ok());

        // Verify all deleted
        let result = get_api_keys(Path("test-session-1".to_string()), State(state))
            .await
            .unwrap();
        assert_eq!(result.0.keys.len(), 0);
    }

    #[tokio::test]
    async fn test_get_keys_for_nonexistent_session() {
        use crate::berrycode::web::infrastructure::database::Database;

        let db = Database::new("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        let state = ApiKeysApiState { db: db.clone() };

        // Get keys for non-existent session
        let result = get_api_keys(
            Path("nonexistent-session".to_string()),
            State(state),
        )
        .await;

        // Should succeed with empty keys
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0.keys.len(), 0);
    }

    #[tokio::test]
    async fn test_mask_different_key_formats() {
        let test_cases = vec![
            ("sk-proj-1234567890123456", "sk-proj-****************..."),
            ("sk-ant-api03-1234567890", "sk-ant-a********************..."),
            ("AIzaSyDxxxxxxxxxxxxxxxxxxxxx", "AIzaSyDx********************..."),
            ("xai-1234567890", "xai-1234********************..."),
            ("a", "****"),
            ("ab", "****"),
            ("abc", "****"),
            ("abcd", "****"),
            ("abcde", "****"),
            ("abcdef", "****"),
            ("abcdefg", "****"),
            ("abcdefgh", "abcdefgh"),
            ("abcdefghi", "abcdefgh*..."),
        ];

        for (input, expected_prefix) in test_cases {
            let masked = mask_api_key(input);
            assert!(
                masked.starts_with(&expected_prefix[..expected_prefix.len().min(masked.len())]),
                "Failed for input '{}': got '{}', expected to start with '{}'",
                input,
                masked,
                expected_prefix
            );
        }
    }

    #[tokio::test]
    async fn test_update_existing_keys() {
        use crate::berrycode::web::infrastructure::database::Database;

        let db = Database::new("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        db.create_session("test-session-1", &std::path::PathBuf::from("/test/project"))
            .await
            .expect("Failed to create test session");

        let state = ApiKeysApiState { db: db.clone() };

        // Save initial keys
        let mut keys1 = HashMap::new();
        keys1.insert("openai".to_string(), "sk-proj-old-key-12345678".to_string());
        save_api_keys(
            Path("test-session-1".to_string()),
            State(state.clone()),
            Json(SaveApiKeysRequest { keys: keys1 }),
        )
        .await
        .expect("Failed to save initial keys");

        // Update with new keys
        let mut keys2 = HashMap::new();
        keys2.insert("openai".to_string(), "sk-proj-new-key-12345678".to_string());
        save_api_keys(
            Path("test-session-1".to_string()),
            State(state.clone()),
            Json(SaveApiKeysRequest { keys: keys2 }),
        )
        .await
        .expect("Failed to update keys");

        // Verify updated key
        let decrypted = get_decrypted_api_key(&db, "test-session-1", "openai")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(decrypted, "sk-proj-new-key-12345678");
    }

    #[tokio::test]
    async fn test_multiple_providers_simultaneously() {
        use crate::berrycode::web::infrastructure::database::Database;

        let db = Database::new("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        db.create_session("test-session-1", &std::path::PathBuf::from("/test/project"))
            .await
            .expect("Failed to create test session");

        let state = ApiKeysApiState { db: db.clone() };

        // Save all 4 providers
        let mut keys = HashMap::new();
        keys.insert("openai".to_string(), "sk-proj-openai123456".to_string());
        keys.insert("anthropic".to_string(), "sk-ant-api03-anthropic12".to_string());
        keys.insert("xai".to_string(), "xai-xai123456789".to_string());
        keys.insert("google".to_string(), "AIzaSyGoogle1234567".to_string());

        save_api_keys(
            Path("test-session-1".to_string()),
            State(state.clone()),
            Json(SaveApiKeysRequest { keys: keys.clone() }),
        )
        .await
        .expect("Failed to save keys");

        // Verify all providers
        for (provider, expected_key) in keys {
            let decrypted = get_decrypted_api_key(&db, "test-session-1", &provider)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(decrypted, expected_key);
        }

        // Verify GET endpoint returns all 4
        let result = get_api_keys(Path("test-session-1".to_string()), State(state))
            .await
            .unwrap();
        assert_eq!(result.0.keys.len(), 4);
    }
}
