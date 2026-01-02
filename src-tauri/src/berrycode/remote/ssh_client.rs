//! SSH client for remote development

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use ssh2::{Channel, Session, Sftp};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// SSH authentication method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SshAuth {
    /// Password authentication
    Password(String),
    /// Public key authentication
    PublicKey {
        private_key_path: String,
        passphrase: Option<String>,
    },
    /// SSH agent authentication
    Agent,
}

/// SSH connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: SshAuth,
}

/// SSH connection wrapper
pub struct SshConnection {
    session: Session,
    sftp: Option<Sftp>,
    config: SshConfig,
}

impl SshConnection {
    /// Create a new SSH connection
    pub fn connect(config: SshConfig) -> Result<Self> {
        tracing::info!(
            "Connecting to SSH host {}:{} as {}",
            config.host,
            config.port,
            config.username
        );

        // Connect TCP stream
        let tcp = TcpStream::connect(format!("{}:{}", config.host, config.port))
            .context("Failed to connect to SSH host")?;

        tcp.set_read_timeout(Some(Duration::from_secs(30)))?;
        tcp.set_write_timeout(Some(Duration::from_secs(30)))?;

        // Initialize SSH session
        let mut session = Session::new().context("Failed to create SSH session")?;
        session.set_tcp_stream(tcp);
        session.handshake().context("SSH handshake failed")?;

        // Authenticate
        match &config.auth {
            SshAuth::Password(password) => {
                session
                    .userauth_password(&config.username, password)
                    .context("SSH password authentication failed")?;
            }
            SshAuth::PublicKey {
                private_key_path,
                passphrase,
            } => {
                let key_path = Path::new(private_key_path);
                session
                    .userauth_pubkey_file(
                        &config.username,
                        None,
                        key_path,
                        passphrase.as_deref(),
                    )
                    .context("SSH public key authentication failed")?;
            }
            SshAuth::Agent => {
                let mut agent = session.agent().context("Failed to connect to SSH agent")?;
                agent.connect().context("Failed to connect to SSH agent")?;
                agent
                    .list_identities()
                    .context("Failed to list SSH agent identities")?;
                let identities = agent.identities().context("Failed to get identities")?;

                let mut authenticated = false;
                for identity in identities {
                    if agent.userauth(&config.username, &identity).is_ok() {
                        authenticated = true;
                        break;
                    }
                }

                if !authenticated {
                    return Err(anyhow!("SSH agent authentication failed"));
                }
            }
        }

        if !session.authenticated() {
            return Err(anyhow!("SSH authentication failed"));
        }

        tracing::info!("SSH connection established successfully");

        Ok(Self {
            session,
            sftp: None,
            config,
        })
    }

    /// Get or create SFTP channel
    fn get_sftp(&mut self) -> Result<&Sftp> {
        if self.sftp.is_none() {
            let sftp = self.session.sftp().context("Failed to create SFTP channel")?;
            self.sftp = Some(sftp);
        }
        Ok(self.sftp.as_ref().unwrap())
    }

    /// Execute a command on the remote host
    pub fn exec(&mut self, command: &str) -> Result<String> {
        tracing::debug!("Executing remote command: {}", command);

        let mut channel = self
            .session
            .channel_session()
            .context("Failed to open SSH channel")?;

        channel.exec(command).context("Failed to execute command")?;

        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .context("Failed to read command output")?;

        channel.wait_close().context("Failed to close channel")?;

        let exit_status = channel.exit_status()?;
        if exit_status != 0 {
            let mut stderr = String::new();
            channel.stderr().read_to_string(&mut stderr).ok();
            tracing::warn!(
                "Command exited with status {}: {}",
                exit_status,
                stderr.trim()
            );
        }

        Ok(output)
    }

    /// Read a file from the remote host
    pub fn read_file(&mut self, path: &str) -> Result<Vec<u8>> {
        tracing::debug!("Reading remote file: {}", path);

        let sftp = self.get_sftp()?;
        let remote_path = Path::new(path);

        let mut file = sftp
            .open(remote_path)
            .context(format!("Failed to open remote file: {}", path))?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .context("Failed to read remote file")?;

        tracing::debug!("Read {} bytes from {}", contents.len(), path);
        Ok(contents)
    }

    /// Write a file to the remote host
    pub fn write_file(&mut self, path: &str, contents: &[u8]) -> Result<()> {
        tracing::debug!("Writing remote file: {} ({} bytes)", path, contents.len());

        let remote_path = Path::new(path);

        // Create parent directories if needed
        if let Some(parent) = remote_path.parent() {
            self.create_dir_all(parent)?;
        }

        // Get sftp after creating directories to avoid borrowing conflicts
        let sftp = self.get_sftp()?;
        let mut file = sftp
            .create(remote_path)
            .context(format!("Failed to create remote file: {}", path))?;

        file.write_all(contents)
            .context("Failed to write remote file")?;

        tracing::info!("Wrote {} bytes to {}", contents.len(), path);
        Ok(())
    }

    /// List directory contents
    pub fn read_dir(&mut self, path: &str) -> Result<Vec<RemoteFileInfo>> {
        tracing::debug!("Listing remote directory: {}", path);

        let sftp = self.get_sftp()?;
        let remote_path = Path::new(path);

        let entries = sftp
            .readdir(remote_path)
            .context(format!("Failed to read remote directory: {}", path))?;

        let mut files = Vec::new();
        for (path, stat) in entries {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            files.push(RemoteFileInfo {
                name,
                path: path.to_string_lossy().to_string(),
                is_dir: stat.is_dir(),
                size: stat.size.unwrap_or(0),
            });
        }

        Ok(files)
    }

    /// Create a directory
    pub fn create_dir(&mut self, path: &Path) -> Result<()> {
        let sftp = self.get_sftp()?;
        sftp.mkdir(path, 0o755)
            .context(format!("Failed to create directory: {}", path.display()))?;
        Ok(())
    }

    /// Create directories recursively
    pub fn create_dir_all(&mut self, path: &Path) -> Result<()> {
        // Check if directory exists
        let sftp = self.get_sftp()?;
        if sftp.stat(path).is_ok() {
            return Ok(());
        }

        // Create parent directories first
        if let Some(parent) = path.parent() {
            self.create_dir_all(parent)?;
        }

        // Create this directory
        self.create_dir(path)?;
        Ok(())
    }

    /// Check if a path exists
    pub fn exists(&mut self, path: &str) -> Result<bool> {
        let sftp = self.get_sftp()?;
        let remote_path = Path::new(path);
        Ok(sftp.stat(remote_path).is_ok())
    }

    /// Delete a file
    pub fn delete_file(&mut self, path: &str) -> Result<()> {
        let sftp = self.get_sftp()?;
        let remote_path = Path::new(path);
        sftp.unlink(remote_path)
            .context(format!("Failed to delete remote file: {}", path))?;
        Ok(())
    }

    /// Delete a directory
    pub fn delete_dir(&mut self, path: &str) -> Result<()> {
        let sftp = self.get_sftp()?;
        let remote_path = Path::new(path);
        sftp.rmdir(remote_path)
            .context(format!("Failed to delete remote directory: {}", path))?;
        Ok(())
    }

    /// Get file metadata
    pub fn stat(&mut self, path: &str) -> Result<RemoteFileInfo> {
        let sftp = self.get_sftp()?;
        let remote_path = Path::new(path);

        let stat = sftp
            .stat(remote_path)
            .context(format!("Failed to stat remote file: {}", path))?;

        Ok(RemoteFileInfo {
            name: remote_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string(),
            path: path.to_string(),
            is_dir: stat.is_dir(),
            size: stat.size.unwrap_or(0),
        })
    }

    /// Open an interactive shell channel
    pub fn open_shell(&mut self) -> Result<Channel> {
        let mut channel = self
            .session
            .channel_session()
            .context("Failed to open SSH channel")?;

        channel.request_pty("xterm", None, None)?;
        channel.shell().context("Failed to open shell")?;

        Ok(channel)
    }

    /// Get connection info
    pub fn get_config(&self) -> &SshConfig {
        &self.config
    }
}

/// Remote file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFileInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

/// SSH connection manager
pub struct SshConnectionManager {
    connections: Arc<Mutex<HashMap<String, Arc<Mutex<SshConnection>>>>>,
}

impl SshConnectionManager {
    /// Create a new SSH connection manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add or update a connection
    pub async fn add_connection(&self, id: String, config: SshConfig) -> Result<()> {
        let connection = SshConnection::connect(config)?;
        let mut connections = self.connections.lock().await;
        connections.insert(id, Arc::new(Mutex::new(connection)));
        Ok(())
    }

    /// Get a connection by ID
    pub async fn get_connection(&self, id: &str) -> Result<Arc<Mutex<SshConnection>>> {
        let connections = self.connections.lock().await;
        connections
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("Connection not found: {}", id))
    }

    /// Remove a connection
    pub async fn remove_connection(&self, id: &str) -> Result<()> {
        let mut connections = self.connections.lock().await;
        connections
            .remove(id)
            .ok_or_else(|| anyhow!("Connection not found: {}", id))?;
        Ok(())
    }

    /// List all active connections
    pub async fn list_connections(&self) -> Vec<String> {
        let connections = self.connections.lock().await;
        connections.keys().cloned().collect()
    }

    /// Test a connection
    pub async fn test_connection(&self, id: &str) -> Result<bool> {
        let connection = self.get_connection(id).await?;
        let mut conn = connection.lock().await;

        // Try to execute a simple command
        match conn.exec("echo test") {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

impl Default for SshConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_config_serialization() {
        let config = SshConfig {
            host: "example.com".to_string(),
            port: 22,
            username: "user".to_string(),
            auth: SshAuth::Password("password".to_string()),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: SshConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.host, deserialized.host);
        assert_eq!(config.port, deserialized.port);
        assert_eq!(config.username, deserialized.username);
    }
}
