//! Remote file system operations

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::ssh_client::{RemoteFileInfo, SshConnection};

/// Remote file system proxy
pub struct RemoteFileSystem {
    connection: Arc<Mutex<SshConnection>>,
    root: PathBuf,
}

impl RemoteFileSystem {
    /// Create a new remote file system
    pub fn new(connection: Arc<Mutex<SshConnection>>, root: PathBuf) -> Self {
        Self { connection, root }
    }

    /// Read a file
    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let full_path = self.resolve_path(path)?;
        let mut conn = self.connection.lock().await;
        conn.read_file(&full_path)
    }

    /// Write a file
    pub async fn write_file(&self, path: &str, contents: &[u8]) -> Result<()> {
        let full_path = self.resolve_path(path)?;
        let mut conn = self.connection.lock().await;
        conn.write_file(&full_path, contents)
    }

    /// List directory
    pub async fn read_dir(&self, path: &str) -> Result<Vec<RemoteFileInfo>> {
        let full_path = self.resolve_path(path)?;
        let mut conn = self.connection.lock().await;
        conn.read_dir(&full_path)
    }

    /// Create directory
    pub async fn create_dir(&self, path: &str) -> Result<()> {
        let full_path = self.resolve_path(path)?;
        let mut conn = self.connection.lock().await;
        conn.create_dir_all(&PathBuf::from(full_path))
    }

    /// Delete file
    pub async fn delete_file(&self, path: &str) -> Result<()> {
        let full_path = self.resolve_path(path)?;
        let mut conn = self.connection.lock().await;
        conn.delete_file(&full_path)
    }

    /// Delete directory
    pub async fn delete_dir(&self, path: &str) -> Result<()> {
        let full_path = self.resolve_path(path)?;
        let mut conn = self.connection.lock().await;
        conn.delete_dir(&full_path)
    }

    /// Check if path exists
    pub async fn exists(&self, path: &str) -> Result<bool> {
        let full_path = self.resolve_path(path)?;
        let mut conn = self.connection.lock().await;
        conn.exists(&full_path)
    }

    /// Get file metadata
    pub async fn stat(&self, path: &str) -> Result<RemoteFileInfo> {
        let full_path = self.resolve_path(path)?;
        let mut conn = self.connection.lock().await;
        conn.stat(&full_path)
    }

    /// Execute command
    pub async fn exec(&self, command: &str) -> Result<String> {
        let mut conn = self.connection.lock().await;
        conn.exec(command)
    }

    /// Build file tree recursively
    pub async fn build_file_tree(&self, path: &str, max_depth: usize) -> Result<RemoteFileNode> {
        self.build_file_tree_recursive(path, 0, max_depth).await
    }

    fn build_file_tree_recursive<'a>(
        &'a self,
        path: &'a str,
        depth: usize,
        max_depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<RemoteFileNode>> + Send + 'a>>
    {
        Box::pin(async move {
            let info = self.stat(path).await?;

            let children = if info.is_dir && depth < max_depth {
                let entries = self.read_dir(path).await?;
                let mut child_nodes = Vec::new();

                for entry in entries {
                    // Skip hidden files and common ignored directories
                    if entry.name.starts_with('.')
                        || entry.name == "node_modules"
                        || entry.name == "target"
                        || entry.name == "dist"
                        || entry.name == "build"
                    {
                        continue;
                    }

                    match self
                        .build_file_tree_recursive(&entry.path, depth + 1, max_depth)
                        .await
                    {
                        Ok(node) => child_nodes.push(node),
                        Err(e) => {
                            tracing::warn!("Failed to build tree for {}: {}", entry.path, e);
                        }
                    }
                }

                // Sort: directories first, then files
                child_nodes.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                });

                Some(child_nodes)
            } else {
                None
            };

            Ok(RemoteFileNode {
                name: info.name,
                path: info.path,
                is_dir: info.is_dir,
                size: info.size,
                children,
            })
        })
    }

    /// Resolve relative path to absolute path
    fn resolve_path(&self, path: &str) -> Result<String> {
        let path = Path::new(path);

        // Security: prevent path traversal
        if path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(anyhow!("Path traversal not allowed"));
        }

        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };

        Ok(full_path.to_string_lossy().to_string())
    }

    /// Get root directory
    pub fn get_root(&self) -> &Path {
        &self.root
    }
}

/// Remote file tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub children: Option<Vec<RemoteFileNode>>,
}

/// Remote path parser
pub struct RemotePath;

impl RemotePath {
    /// Parse a remote path (e.g., "ssh://user@host:port/path/to/file")
    pub fn parse(path: &str) -> Result<RemotePathInfo> {
        if !path.starts_with("ssh://") {
            return Err(anyhow!("Invalid remote path: must start with ssh://"));
        }

        let path = &path[6..]; // Remove "ssh://"

        // Split into user@host:port and path
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid remote path format"));
        }

        let host_part = parts[0];
        let file_path = format!("/{}", parts[1]);

        // Parse user@host:port
        let (user, host_port) = if let Some(idx) = host_part.find('@') {
            (host_part[..idx].to_string(), &host_part[idx + 1..])
        } else {
            return Err(anyhow!("Username not specified in remote path"));
        };

        // Parse host:port
        let (host, port) = if let Some(idx) = host_port.find(':') {
            let port_str = &host_port[idx + 1..];
            let port = port_str
                .parse::<u16>()
                .map_err(|_| anyhow!("Invalid port number"))?;
            (host_port[..idx].to_string(), port)
        } else {
            (host_port.to_string(), 22)
        };

        Ok(RemotePathInfo {
            username: user,
            host,
            port,
            path: file_path,
        })
    }

    /// Build a remote path string
    pub fn build(user: &str, host: &str, port: u16, path: &str) -> String {
        if port == 22 {
            format!("ssh://{}@{}{}", user, host, path)
        } else {
            format!("ssh://{}@{}:{}{}", user, host, port, path)
        }
    }

    /// Check if a path is a remote path
    pub fn is_remote(path: &str) -> bool {
        path.starts_with("ssh://")
    }

    /// Get connection ID from remote path
    pub fn get_connection_id(path: &str) -> Result<String> {
        let info = Self::parse(path)?;
        Ok(format!("{}@{}:{}", info.username, info.host, info.port))
    }
}

/// Parsed remote path information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePathInfo {
    pub username: String,
    pub host: String,
    pub port: u16,
    pub path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_remote_path() {
        let path = "ssh://user@example.com:2222/home/user/project/file.txt";
        let info = RemotePath::parse(path).unwrap();

        assert_eq!(info.username, "user");
        assert_eq!(info.host, "example.com");
        assert_eq!(info.port, 2222);
        assert_eq!(info.path, "/home/user/project/file.txt");
    }

    #[test]
    fn test_parse_remote_path_default_port() {
        let path = "ssh://user@example.com/home/user/file.txt";
        let info = RemotePath::parse(path).unwrap();

        assert_eq!(info.username, "user");
        assert_eq!(info.host, "example.com");
        assert_eq!(info.port, 22);
        assert_eq!(info.path, "/home/user/file.txt");
    }

    #[test]
    fn test_build_remote_path() {
        let path = RemotePath::build("user", "example.com", 22, "/home/user/file.txt");
        assert_eq!(path, "ssh://user@example.com/home/user/file.txt");

        let path = RemotePath::build("user", "example.com", 2222, "/home/user/file.txt");
        assert_eq!(path, "ssh://user@example.com:2222/home/user/file.txt");
    }

    #[test]
    fn test_is_remote() {
        assert!(RemotePath::is_remote("ssh://user@host/path"));
        assert!(!RemotePath::is_remote("/local/path"));
        assert!(!RemotePath::is_remote("http://example.com"));
    }
}
