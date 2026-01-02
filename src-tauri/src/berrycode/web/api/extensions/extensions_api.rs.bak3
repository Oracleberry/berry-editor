//! Extensions API for BerryCode
//!
//! Provides extension management functionality:
//! - Extension discovery and loading
//! - Extension installation/uninstallation
//! - Extension API surface for plugins
//! - Sandboxed execution environment

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tokio::sync::RwLock;
use std::sync::Arc;

use crate::berrycode::web::infrastructure::error::{WebError, WebResult};
use crate::berrycode::web::infrastructure::session_db::SessionDbStore;

/// Extension manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub main: String,
    pub icon: Option<String>,
    pub categories: Option<Vec<String>>,
    #[serde(rename = "activationEvents")]
    pub activation_events: Option<Vec<String>>,
    pub contributes: Option<ExtensionContributes>,
    pub permissions: Option<Vec<String>>,
    pub dependencies: Option<HashMap<String, String>>,
}

/// Extension contributions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionContributes {
    pub commands: Option<Vec<ExtensionCommand>>,
    pub menus: Option<ExtensionMenus>,
    pub languages: Option<Vec<ExtensionLanguage>>,
}

/// Extension command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionCommand {
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
    pub keybinding: Option<String>,
}

/// Extension menus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMenus {
    pub toolbar: Option<Vec<ExtensionMenuItem>>,
    pub sidebar: Option<Vec<ExtensionSidebarItem>>,
}

/// Extension menu item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMenuItem {
    #[serde(rename = "commandId")]
    pub command_id: String,
    pub group: Option<String>,
    pub order: Option<i32>,
}

/// Extension sidebar item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionSidebarItem {
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
}

/// Extension language support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionLanguage {
    pub id: String,
    pub extensions: Vec<String>,
    pub aliases: Option<Vec<String>>,
}

/// Installed extension info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledExtension {
    pub manifest: ExtensionManifest,
    pub path: String,
    pub enabled: bool,
    pub installed_at: i64,
}

/// Extension state
#[derive(Clone)]
pub struct ExtensionsApiState {
    pub session_store: SessionDbStore,
    pub extensions: Arc<RwLock<HashMap<String, InstalledExtension>>>,
    pub extensions_dir: PathBuf,
}

impl ExtensionsApiState {
    /// Create new extensions API state
    pub fn new(session_store: SessionDbStore, extensions_dir: PathBuf) -> Self {
        Self {
            session_store,
            extensions: Arc::new(RwLock::new(HashMap::new())),
            extensions_dir,
        }
    }

    /// Load extensions from disk
    pub async fn load_extensions(&self) -> anyhow::Result<()> {
        tracing::info!("Loading extensions from {:?}", self.extensions_dir);

        // Create extensions directory if it doesn't exist
        fs::create_dir_all(&self.extensions_dir)?;

        // Load all manifests first (without holding the lock)
        let mut loaded_extensions = Vec::new();

        // Scan extensions directory
        if let Ok(entries) = fs::read_dir(&self.extensions_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let manifest_path = entry.path().join("manifest.json");
                    if manifest_path.exists() {
                        match Self::load_extension_manifest_static(&manifest_path) {
                            Ok(manifest) => {
                                let extension = InstalledExtension {
                                    manifest: manifest.clone(),
                                    path: entry.path().to_string_lossy().to_string(),
                                    enabled: true,
                                    installed_at: chrono::Utc::now().timestamp(),
                                };
                                loaded_extensions.push(extension);
                                tracing::info!("Loaded extension: {} v{}", manifest.name, manifest.version);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to load extension from {:?}: {}", manifest_path, e);
                            }
                        }
                    }
                }
            }
        }

        // Now acquire lock and insert all extensions
        let mut extensions = self.extensions.write().await;
        extensions.clear();
        for extension in loaded_extensions {
            extensions.insert(extension.manifest.id.clone(), extension);
        }

        tracing::info!("Loaded {} extensions", extensions.len());
        Ok(())
    }

    /// Load extension manifest from file (static version)
    fn load_extension_manifest_static(path: &std::path::Path) -> anyhow::Result<ExtensionManifest> {
        let content = fs::read_to_string(path)?;
        let manifest: ExtensionManifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    /// Validate extension manifest
    fn validate_manifest(&self, manifest: &ExtensionManifest) -> Result<(), String> {
        // Check required fields
        if manifest.id.is_empty() {
            return Err("Extension ID is required".to_string());
        }
        if manifest.name.is_empty() {
            return Err("Extension name is required".to_string());
        }
        if manifest.version.is_empty() {
            return Err("Extension version is required".to_string());
        }
        if manifest.main.is_empty() {
            return Err("Extension main file is required".to_string());
        }

        // Validate ID format (kebab-case)
        if !manifest.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err("Extension ID must be kebab-case (lowercase letters, numbers, and hyphens)".to_string());
        }

        // Validate version format (semver)
        let version_parts: Vec<&str> = manifest.version.split('.').collect();
        if version_parts.len() != 3 || !version_parts.iter().all(|p| p.parse::<u32>().is_ok()) {
            return Err("Extension version must follow semver format (x.y.z)".to_string());
        }

        Ok(())
    }
}

/// List all installed extensions
pub async fn list_extensions(
    State(state): State<ExtensionsApiState>,
) -> WebResult<Json<Vec<InstalledExtension>>> {
    tracing::debug!("Listing installed extensions");

    let extensions = state.extensions.read().await;
    let extension_list: Vec<InstalledExtension> = extensions.values().cloned().collect();

    tracing::debug!("Found {} installed extensions", extension_list.len());
    Ok(Json(extension_list))
}

/// Get extension details
pub async fn get_extension(
    Path(extension_id): Path<String>,
    State(state): State<ExtensionsApiState>,
) -> WebResult<Json<InstalledExtension>> {
    tracing::debug!(extension_id = %extension_id, "Getting extension details");

    let extensions = state.extensions.read().await;
    let extension = extensions
        .get(&extension_id)
        .ok_or_else(|| WebError::NotFound(format!("Extension not found: {}", extension_id)))?;

    Ok(Json(extension.clone()))
}

/// Get extension source code
pub async fn get_extension_source(
    Path(extension_id): Path<String>,
    State(state): State<ExtensionsApiState>,
) -> WebResult<Json<ExtensionSourceResponse>> {
    tracing::debug!(extension_id = %extension_id, "Getting extension source code");

    let extensions = state.extensions.read().await;
    let extension = extensions
        .get(&extension_id)
        .ok_or_else(|| WebError::NotFound(format!("Extension not found: {}", extension_id)))?;

    // Read main file
    let main_path = PathBuf::from(&extension.path).join(&extension.manifest.main);
    let source = fs::read_to_string(&main_path)
        .map_err(|e| WebError::Internal(format!("Failed to read extension source: {}", e)))?;

    Ok(Json(ExtensionSourceResponse {
        manifest: extension.manifest.clone(),
        source,
    }))
}

#[derive(Debug, Serialize)]
pub struct ExtensionSourceResponse {
    pub manifest: ExtensionManifest,
    pub source: String,
}

/// Install extension request
#[derive(Debug, Deserialize)]
pub struct InstallExtensionRequest {
    pub manifest: ExtensionManifest,
    pub source: String,
}

/// Install extension
pub async fn install_extension(
    State(state): State<ExtensionsApiState>,
    Json(request): Json<InstallExtensionRequest>,
) -> WebResult<Json<InstalledExtension>> {
    tracing::info!(extension_id = %request.manifest.id, "Installing extension");

    // Validate manifest
    state
        .validate_manifest(&request.manifest)
        .map_err(|e| WebError::BadRequest(e))?;

    // Check if extension already exists
    let extensions = state.extensions.read().await;
    if extensions.contains_key(&request.manifest.id) {
        return Err(WebError::BadRequest(format!(
            "Extension already installed: {}",
            request.manifest.id
        )));
    }
    drop(extensions);

    // Create extension directory
    let extension_dir = state.extensions_dir.join(&request.manifest.id);
    fs::create_dir_all(&extension_dir)
        .map_err(|e| WebError::Internal(format!("Failed to create extension directory: {}", e)))?;

    // Write manifest
    let manifest_path = extension_dir.join("manifest.json");
    let manifest_content = serde_json::to_string_pretty(&request.manifest)
        .map_err(|e| WebError::Internal(format!("Failed to serialize manifest: {}", e)))?;
    fs::write(&manifest_path, manifest_content)
        .map_err(|e| WebError::Internal(format!("Failed to write manifest: {}", e)))?;

    // Write main file
    let main_path = extension_dir.join(&request.manifest.main);
    fs::write(&main_path, request.source)
        .map_err(|e| WebError::Internal(format!("Failed to write extension source: {}", e)))?;

    // Add to installed extensions
    let installed_extension = InstalledExtension {
        manifest: request.manifest.clone(),
        path: extension_dir.to_string_lossy().to_string(),
        enabled: true,
        installed_at: chrono::Utc::now().timestamp(),
    };

    let mut extensions = state.extensions.write().await;
    extensions.insert(request.manifest.id.clone(), installed_extension.clone());

    tracing::info!(
        extension_id = %request.manifest.id,
        "Extension installed successfully"
    );

    Ok(Json(installed_extension))
}

/// Uninstall extension
pub async fn uninstall_extension(
    Path(extension_id): Path<String>,
    State(state): State<ExtensionsApiState>,
) -> WebResult<StatusCode> {
    tracing::info!(extension_id = %extension_id, "Uninstalling extension");

    // Remove from installed extensions
    let mut extensions = state.extensions.write().await;
    let extension = extensions
        .remove(&extension_id)
        .ok_or_else(|| WebError::NotFound(format!("Extension not found: {}", extension_id)))?;

    // Delete extension directory
    let extension_path = PathBuf::from(&extension.path);
    if extension_path.exists() {
        fs::remove_dir_all(&extension_path)
            .map_err(|e| WebError::Internal(format!("Failed to delete extension directory: {}", e)))?;
    }

    tracing::info!(extension_id = %extension_id, "Extension uninstalled successfully");
    Ok(StatusCode::NO_CONTENT)
}

/// Enable/disable extension request
#[derive(Debug, Deserialize)]
pub struct ToggleExtensionRequest {
    pub enabled: bool,
}

/// Enable or disable extension
pub async fn toggle_extension(
    Path(extension_id): Path<String>,
    State(state): State<ExtensionsApiState>,
    Json(request): Json<ToggleExtensionRequest>,
) -> WebResult<Json<InstalledExtension>> {
    tracing::info!(
        extension_id = %extension_id,
        enabled = %request.enabled,
        "Toggling extension"
    );

    let mut extensions = state.extensions.write().await;
    let extension = extensions
        .get_mut(&extension_id)
        .ok_or_else(|| WebError::NotFound(format!("Extension not found: {}", extension_id)))?;

    extension.enabled = request.enabled;

    tracing::info!(
        extension_id = %extension_id,
        enabled = %request.enabled,
        "Extension toggled successfully"
    );

    Ok(Json(extension.clone()))
}

/// Extension API call request (from extension runtime)
#[derive(Debug, Deserialize)]
pub struct ExtensionApiRequest {
    pub extension_id: String,
    pub method: String,
    pub params: serde_json::Value,
}

/// Extension API call response
#[derive(Debug, Serialize)]
pub struct ExtensionApiResponse {
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Handle extension API calls
pub async fn extension_api_call(
    Path(session_id): Path<String>,
    State(state): State<ExtensionsApiState>,
    Json(request): Json<ExtensionApiRequest>,
) -> WebResult<Json<ExtensionApiResponse>> {
    tracing::debug!(
        session_id = %session_id,
        extension_id = %request.extension_id,
        method = %request.method,
        "Handling extension API call"
    );

    // Verify extension is installed and enabled
    let extensions = state.extensions.read().await;
    let extension = extensions
        .get(&request.extension_id)
        .ok_or_else(|| WebError::NotFound(format!("Extension not found: {}", request.extension_id)))?;

    if !extension.enabled {
        return Ok(Json(ExtensionApiResponse {
            success: false,
            result: None,
            error: Some("Extension is disabled".to_string()),
        }));
    }

    // Check permissions
    let required_permission = match request.method.as_str() {
        "fs.readFile" | "fs.readDirectory" => Some("fileSystem:read"),
        "fs.writeFile" | "fs.createDirectory" => Some("fileSystem:write"),
        "terminal.execute" => Some("terminal:execute"),
        "git.status" | "git.diff" => Some("git:read"),
        "git.commit" | "git.push" => Some("git:write"),
        "lsp.definition" | "lsp.references" => Some("lsp:read"),
        "lsp.rename" => Some("lsp:write"),
        _ => None,
    };

    if let Some(perm) = required_permission {
        let has_permission = extension
            .manifest
            .permissions
            .as_ref()
            .map(|perms| perms.iter().any(|p| p == perm))
            .unwrap_or(false);

        if !has_permission {
            return Ok(Json(ExtensionApiResponse {
                success: false,
                result: None,
                error: Some(format!("Missing permission: {}", perm)),
            }));
        }
    }

    drop(extensions);

    // Delegate to appropriate API
    let result = match request.method.as_str() {
        "storage.get" => handle_storage_get(&request.extension_id, &request.params).await,
        "storage.set" => handle_storage_set(&request.extension_id, &request.params).await,
        _ => Err("Unknown API method".to_string()),
    };

    match result {
        Ok(value) => Ok(Json(ExtensionApiResponse {
            success: true,
            result: Some(value),
            error: None,
        })),
        Err(error) => Ok(Json(ExtensionApiResponse {
            success: false,
            result: None,
            error: Some(error),
        })),
    }
}

/// Handle storage.get API call
async fn handle_storage_get(
    extension_id: &str,
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let key = params
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'key' parameter".to_string())?;

    // In a real implementation, this would read from a database or file
    // For now, return null (not found)
    tracing::debug!(extension_id = %extension_id, key = %key, "Storage get");
    Ok(serde_json::Value::Null)
}

/// Handle storage.set API call
async fn handle_storage_set(
    extension_id: &str,
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let key = params
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'key' parameter".to_string())?;

    let value = params
        .get("value")
        .ok_or_else(|| "Missing 'value' parameter".to_string())?;

    // In a real implementation, this would write to a database or file
    tracing::debug!(extension_id = %extension_id, key = %key, "Storage set");
    Ok(serde_json::json!({"success": true}))
}
