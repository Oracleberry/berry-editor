//! Platform-agnostic storage abstraction
//!
//! Provides a unified interface for persistent storage across different platforms:
//! - Web: LocalStorage
//! - Desktop/Mobile: File-based storage via Tauri

use anyhow::{Result, anyhow};

/// Trait for platform-agnostic key-value storage
pub trait EditorStorage {
    /// Get a value by key
    fn get_item(&self, key: &str) -> Result<Option<String>>;

    /// Set a value by key
    fn set_item(&self, key: &str, value: &str) -> Result<()>;

    /// Remove a value by key
    fn remove_item(&self, key: &str) -> Result<()>;

    /// Clear all storage
    fn clear(&self) -> Result<()>;
}

// ========================================
// Web Implementation (LocalStorage)
// ========================================

#[cfg(target_arch = "wasm32")]
pub struct WebStorage;

#[cfg(target_arch = "wasm32")]
impl WebStorage {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_arch = "wasm32")]
impl EditorStorage for WebStorage {
    fn get_item(&self, key: &str) -> Result<Option<String>> {
        let window = web_sys::window()
            .ok_or_else(|| anyhow!("No window object available"))?;

        let storage = window.local_storage()
            .map_err(|e| anyhow!("Failed to access localStorage: {:?}", e))?
            .ok_or_else(|| anyhow!("localStorage is not available"))?;

        storage.get_item(key)
            .map_err(|e| anyhow!("Failed to get item from localStorage: {:?}", e))
    }

    fn set_item(&self, key: &str, value: &str) -> Result<()> {
        let window = web_sys::window()
            .ok_or_else(|| anyhow!("No window object available"))?;

        let storage = window.local_storage()
            .map_err(|e| anyhow!("Failed to access localStorage: {:?}", e))?
            .ok_or_else(|| anyhow!("localStorage is not available"))?;

        storage.set_item(key, value)
            .map_err(|e| anyhow!("Failed to set item in localStorage: {:?}", e))
    }

    fn remove_item(&self, key: &str) -> Result<()> {
        let window = web_sys::window()
            .ok_or_else(|| anyhow!("No window object available"))?;

        let storage = window.local_storage()
            .map_err(|e| anyhow!("Failed to access localStorage: {:?}", e))?
            .ok_or_else(|| anyhow!("localStorage is not available"))?;

        storage.remove_item(key)
            .map_err(|e| anyhow!("Failed to remove item from localStorage: {:?}", e))
    }

    fn clear(&self) -> Result<()> {
        let window = web_sys::window()
            .ok_or_else(|| anyhow!("No window object available"))?;

        let storage = window.local_storage()
            .map_err(|e| anyhow!("Failed to access localStorage: {:?}", e))?
            .ok_or_else(|| anyhow!("localStorage is not available"))?;

        storage.clear()
            .map_err(|e| anyhow!("Failed to clear localStorage: {:?}", e))
    }
}

// ========================================
// Native Implementation (File-based via Tauri)
// ========================================

#[cfg(not(target_arch = "wasm32"))]
pub struct NativeStorage {
    storage_path: std::path::PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl NativeStorage {
    pub fn new() -> Result<Self> {
        // Get app data directory
        let storage_path = Self::get_storage_dir()?;

        // Create directory if it doesn't exist
        if !storage_path.exists() {
            std::fs::create_dir_all(&storage_path)
                .map_err(|e| anyhow!("Failed to create storage directory: {}", e))?;
        }

        Ok(Self { storage_path })
    }

    fn get_storage_dir() -> Result<std::path::PathBuf> {
        // Try to get Tauri app data directory
        // Fallback to platform-specific directories
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME")
                .map_err(|_| anyhow!("HOME environment variable not set"))?;
            Ok(std::path::PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("BerryEditor"))
        }

        #[cfg(target_os = "linux")]
        {
            let home = std::env::var("HOME")
                .map_err(|_| anyhow!("HOME environment variable not set"))?;
            Ok(std::path::PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("berry-editor"))
        }

        #[cfg(target_os = "windows")]
        {
            let appdata = std::env::var("APPDATA")
                .map_err(|_| anyhow!("APPDATA environment variable not set"))?;
            Ok(std::path::PathBuf::from(appdata).join("BerryEditor"))
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            Ok(std::path::PathBuf::from("./storage"))
        }
    }

    fn get_file_path(&self, key: &str) -> std::path::PathBuf {
        // Sanitize key to use as filename
        let safe_key = key.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        self.storage_path.join(format!("{}.json", safe_key))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl EditorStorage for NativeStorage {
    fn get_item(&self, key: &str) -> Result<Option<String>> {
        let file_path = self.get_file_path(key);

        if !file_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| anyhow!("Failed to read storage file: {}", e))?;

        Ok(Some(content))
    }

    fn set_item(&self, key: &str, value: &str) -> Result<()> {
        let file_path = self.get_file_path(key);

        std::fs::write(&file_path, value)
            .map_err(|e| anyhow!("Failed to write storage file: {}", e))
    }

    fn remove_item(&self, key: &str) -> Result<()> {
        let file_path = self.get_file_path(key);

        if file_path.exists() {
            std::fs::remove_file(&file_path)
                .map_err(|e| anyhow!("Failed to remove storage file: {}", e))?;
        }

        Ok(())
    }

    fn clear(&self) -> Result<()> {
        // Remove all files in storage directory
        if self.storage_path.exists() {
            std::fs::remove_dir_all(&self.storage_path)
                .map_err(|e| anyhow!("Failed to clear storage: {}", e))?;
            std::fs::create_dir_all(&self.storage_path)
                .map_err(|e| anyhow!("Failed to recreate storage directory: {}", e))?;
        }

        Ok(())
    }
}

// ========================================
// Mock Implementation (for testing)
// ========================================

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock storage for testing (no browser or filesystem required)
#[derive(Clone)]
pub struct MockStorage {
    data: Arc<Mutex<HashMap<String, String>>>,
}

impl MockStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn clear_all(&self) {
        self.data.lock().unwrap().clear();
    }
}

impl EditorStorage for MockStorage {
    fn get_item(&self, key: &str) -> Result<Option<String>> {
        Ok(self.data.lock().unwrap().get(key).cloned())
    }

    fn set_item(&self, key: &str, value: &str) -> Result<()> {
        self.data.lock().unwrap().insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn remove_item(&self, key: &str) -> Result<()> {
        self.data.lock().unwrap().remove(key);
        Ok(())
    }

    fn clear(&self) -> Result<()> {
        self.data.lock().unwrap().clear();
        Ok(())
    }
}

// ========================================
// Factory function
// ========================================

/// Create a storage instance for the current platform
pub fn create_storage() -> Result<Box<dyn EditorStorage>> {
    #[cfg(target_arch = "wasm32")]
    {
        Ok(Box::new(WebStorage::new()))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(Box::new(NativeStorage::new()?))
    }
}

/// Create a mock storage for testing
pub fn create_mock_storage() -> Box<dyn EditorStorage> {
    Box::new(MockStorage::new())
}

// ========================================
// Tests
// ========================================

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_web_storage_creation() {
        let _storage = WebStorage::new();
        // Just verify it can be created
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_native_storage_creation() {
        let storage = NativeStorage::new();
        assert!(storage.is_ok());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_native_storage_operations() {
        let storage = NativeStorage::new().unwrap();

        // Test set and get
        let key = "test_key";
        let value = "test_value";

        storage.set_item(key, value).unwrap();
        let retrieved = storage.get_item(key).unwrap();
        assert_eq!(retrieved, Some(value.to_string()));

        // Test remove
        storage.remove_item(key).unwrap();
        let retrieved = storage.get_item(key).unwrap();
        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_create_storage() {
        let storage = create_storage();
        assert!(storage.is_ok());
    }

    #[test]
    fn test_mock_storage_operations() {
        let storage = MockStorage::new();

        // Test set and get
        storage.set_item("key1", "value1").unwrap();
        assert_eq!(storage.get_item("key1").unwrap(), Some("value1".to_string()));

        // Test overwrite
        storage.set_item("key1", "value2").unwrap();
        assert_eq!(storage.get_item("key1").unwrap(), Some("value2".to_string()));

        // Test remove
        storage.remove_item("key1").unwrap();
        assert_eq!(storage.get_item("key1").unwrap(), None);

        // Test multiple keys
        storage.set_item("a", "1").unwrap();
        storage.set_item("b", "2").unwrap();
        storage.set_item("c", "3").unwrap();

        assert_eq!(storage.get_item("a").unwrap(), Some("1".to_string()));
        assert_eq!(storage.get_item("b").unwrap(), Some("2".to_string()));
        assert_eq!(storage.get_item("c").unwrap(), Some("3".to_string()));

        // Test clear
        storage.clear().unwrap();
        assert_eq!(storage.get_item("a").unwrap(), None);
        assert_eq!(storage.get_item("b").unwrap(), None);
        assert_eq!(storage.get_item("c").unwrap(), None);
    }

    #[test]
    fn test_mock_storage_clone() {
        let storage1 = MockStorage::new();
        storage1.set_item("shared", "data").unwrap();

        let storage2 = storage1.clone();
        assert_eq!(storage2.get_item("shared").unwrap(), Some("data".to_string()));

        // Verify they share the same underlying data
        storage2.set_item("shared", "updated").unwrap();
        assert_eq!(storage1.get_item("shared").unwrap(), Some("updated".to_string()));
    }

    #[test]
    fn test_create_mock_storage() {
        let storage = create_mock_storage();
        storage.set_item("test", "value").unwrap();
        assert_eq!(storage.get_item("test").unwrap(), Some("value".to_string()));
    }
}
