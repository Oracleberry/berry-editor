//! Cryptographic utilities for API key encryption

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

/// Encryption key length (32 bytes for AES-256)
const KEY_LEN: usize = 32;

/// Nonce length (12 bytes for AES-GCM)
const NONCE_LEN: usize = 12;

/// Get or generate master encryption key from environment variable
fn get_master_key() -> Result<[u8; KEY_LEN]> {
    if let Ok(key_hex) = std::env::var("BERRYCODE_MASTER_KEY") {
        // Decode from hex string
        let key_bytes = hex::decode(&key_hex)
            .map_err(|e| anyhow!("Invalid BERRYCODE_MASTER_KEY hex format: {}", e))?;

        if key_bytes.len() != KEY_LEN {
            return Err(anyhow!(
                "BERRYCODE_MASTER_KEY must be {} bytes (got {})",
                KEY_LEN,
                key_bytes.len()
            ));
        }

        let mut key = [0u8; KEY_LEN];
        key.copy_from_slice(&key_bytes);
        Ok(key)
    } else {
        // Generate a deterministic key from hostname + username as fallback
        // This is NOT secure for production but works for local development
        tracing::warn!(
            "BERRYCODE_MASTER_KEY not set. Using deterministic key based on system info. \
             For production, set BERRYCODE_MASTER_KEY to a secure random key."
        );

        let hostname = hostname::get()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "default".to_string());

        let seed = format!("berrycode-{}-{}", hostname, username);

        // Use SHA-256 to derive a key
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(seed.as_bytes());
        let result = hasher.finalize();

        let mut key = [0u8; KEY_LEN];
        key.copy_from_slice(&result);
        Ok(key)
    }
}

/// Encrypt API key using AES-256-GCM
pub fn encrypt_api_key(plaintext: &str) -> Result<String> {
    let key = get_master_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;

    // Generate random nonce
    let mut nonce_bytes = [0u8; NONCE_LEN];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    // Combine nonce + ciphertext and encode as base64
    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(&combined))
}

/// Decrypt API key using AES-256-GCM
pub fn decrypt_api_key(encrypted: &str) -> Result<String> {
    let key = get_master_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;

    // Decode from base64
    let combined = BASE64
        .decode(encrypted)
        .map_err(|e| anyhow!("Invalid base64: {}", e))?;

    if combined.len() < NONCE_LEN {
        return Err(anyhow!("Encrypted data too short"));
    }

    // Split nonce and ciphertext
    let (nonce_bytes, ciphertext) = combined.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Decrypt
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| anyhow!("Invalid UTF-8: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let original = "sk-proj-test-api-key-12345";

        let encrypted = encrypt_api_key(original).expect("Encryption failed");
        assert_ne!(encrypted, original);

        let decrypted = decrypt_api_key(&encrypted).expect("Decryption failed");
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_different_encryptions() {
        let original = "sk-ant-test-key";

        let encrypted1 = encrypt_api_key(original).expect("Encryption 1 failed");
        let encrypted2 = encrypt_api_key(original).expect("Encryption 2 failed");

        // Different nonces should produce different ciphertexts
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same plaintext
        assert_eq!(decrypt_api_key(&encrypted1).unwrap(), original);
        assert_eq!(decrypt_api_key(&encrypted2).unwrap(), original);
    }

    #[test]
    fn test_invalid_base64() {
        let result = decrypt_api_key("invalid-base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_corrupted_data() {
        let original = "test-key";
        let mut encrypted = encrypt_api_key(original).expect("Encryption failed");

        // Corrupt the encrypted data
        encrypted.push_str("corrupted");

        let result = decrypt_api_key(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_string() {
        let original = "";
        let encrypted = encrypt_api_key(original).expect("Encryption failed");
        let decrypted = decrypt_api_key(&encrypted).expect("Decryption failed");
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_long_api_key() {
        let original = "sk-proj-".to_string() + &"a".repeat(200);
        let encrypted = encrypt_api_key(&original).expect("Encryption failed");
        let decrypted = decrypt_api_key(&encrypted).expect("Decryption failed");
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_special_characters() {
        let original = "sk-test!@#$%^&*()_+-=[]{}|;':\",./<>?";
        let encrypted = encrypt_api_key(original).expect("Encryption failed");
        let decrypted = decrypt_api_key(&encrypted).expect("Decryption failed");
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_unicode_characters() {
        let original = "sk-test-日本語-emoji🔒🔑-中文";
        let encrypted = encrypt_api_key(original).expect("Encryption failed");
        let decrypted = decrypt_api_key(&encrypted).expect("Decryption failed");
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_whitespace_and_newlines() {
        let original = "sk-test\nwith\nnewlines\tand\ttabs  and  spaces";
        let encrypted = encrypt_api_key(original).expect("Encryption failed");
        let decrypted = decrypt_api_key(&encrypted).expect("Decryption failed");
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_short_encrypted_data() {
        // Encrypted data shorter than nonce length
        let short_data = base64::engine::general_purpose::STANDARD.encode("short");
        let result = decrypt_api_key(&short_data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn test_multiple_encryptions_same_key() {
        let original = "sk-test-consistency";

        // Encrypt multiple times
        let mut encrypted_values = vec![];
        for _ in 0..5 {
            encrypted_values.push(encrypt_api_key(original).unwrap());
        }

        // All encrypted values should be different (different nonces)
        for i in 0..encrypted_values.len() {
            for j in i + 1..encrypted_values.len() {
                assert_ne!(encrypted_values[i], encrypted_values[j]);
            }
        }

        // But all should decrypt to the same value
        for encrypted in encrypted_values {
            assert_eq!(decrypt_api_key(&encrypted).unwrap(), original);
        }
    }

    #[test]
    fn test_all_provider_key_formats() {
        let test_keys = vec![
            ("openai", "sk-proj-1234567890abcdefghijklmnopqrstuvwxyz"),
            ("anthropic", "sk-ant-api03-1234567890abcdefghijklmnopqrstuvwxyz"),
            ("xai", "xai-1234567890abcdefghijklmnopqrstuvwxyz"),
            ("google", "AIzaSyD1234567890abcdefghijklmnopqrstuvwxyz"),
        ];

        for (provider, key) in test_keys {
            let encrypted = encrypt_api_key(key).expect(&format!("Encryption failed for {}", provider));
            let decrypted = decrypt_api_key(&encrypted).expect(&format!("Decryption failed for {}", provider));
            assert_eq!(decrypted, key, "Mismatch for provider: {}", provider);
        }
    }
}
