//! LSP Client Implementation
//! Manages LSP server process and communication

use super::protocol::*;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// LSP Client
pub struct LspClient {
    language: String,
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    request_id: Arc<AtomicU64>,
    capabilities: ServerCapabilities,
}

impl LspClient {
    /// Create new LSP client for a language
    pub fn new(language: &str, root_uri: &str) -> Result<Self, String> {
        let mut client = Self {
            language: language.to_string(),
            process: None,
            stdin: None,
            stdout: None,
            request_id: Arc::new(AtomicU64::new(1)),
            capabilities: ServerCapabilities::default(),
        };

        // Start language server process
        client.start_server()?;

        // Initialize
        client.initialize(root_uri)?;

        Ok(client)
    }

    /// Start language server process
    fn start_server(&mut self) -> Result<(), String> {
        let (command, args) = Self::get_server_command(&self.language)?;

        let mut process = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null()) // Suppress server stderr for now
            .spawn()
            .map_err(|e| format!("Failed to start LSP server: {}", e))?;

        let stdin = process.stdin.take().ok_or("Failed to get stdin")?;

        let stdout = process.stdout.take().ok_or("Failed to get stdout")?;

        self.process = Some(process);
        self.stdin = Some(stdin);
        self.stdout = Some(BufReader::new(stdout));

        Ok(())
    }

    /// Get server command for language
    fn get_server_command(language: &str) -> Result<(String, Vec<String>), String> {
        match language {
            "rust" => Ok(("rust-analyzer".to_string(), vec![])),
            "typescript" | "javascript" => Ok((
                "typescript-language-server".to_string(),
                vec!["--stdio".to_string()],
            )),
            "python" => Ok((
                "pyright-langserver".to_string(),
                vec!["--stdio".to_string()],
            )),
            _ => Err(format!("Unsupported language: {}", language)),
        }
    }

    /// Initialize LSP server
    fn initialize(&mut self, root_uri: &str) -> Result<(), String> {
        let params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true
                        }
                    },
                    "hover": {
                        "contentFormat": ["markdown", "plaintext"]
                    }
                }
            }
        });

        let id = self.next_request_id();
        let request = LspRequest::new(id, "initialize", Some(params));

        let response = self.send_request(request)?;

        // Parse server capabilities
        if let Some(result) = response.result {
            if let Some(caps) = result.get("capabilities") {
                self.capabilities = serde_json::from_value(caps.clone()).unwrap_or_default();
            }
        }

        // Send initialized notification
        let notification = LspNotification::new("initialized", Some(serde_json::json!({})));
        self.send_notification(notification)?;

        Ok(())
    }

    /// Send request and wait for response
    fn send_request(&mut self, request: LspRequest) -> Result<LspResponse, String> {
        let message = serde_json::to_string(&request)
            .map_err(|e| format!("Failed to serialize request: {}", e))?;

        self.write_message(&message)?;

        // Read response (simplified - in production, use async/await)
        self.read_response(request.id)
    }

    /// Send notification (no response expected)
    fn send_notification(&mut self, notification: LspNotification) -> Result<(), String> {
        let message = serde_json::to_string(&notification)
            .map_err(|e| format!("Failed to serialize notification: {}", e))?;

        self.write_message(&message)
    }

    /// Write message to stdin
    fn write_message(&mut self, message: &str) -> Result<(), String> {
        let stdin = self.stdin.as_mut().ok_or("No stdin available")?;

        let content_length = message.len();
        let header = format!("Content-Length: {}\r\n\r\n", content_length);

        stdin
            .write_all(header.as_bytes())
            .map_err(|e| format!("Failed to write header: {}", e))?;

        stdin
            .write_all(message.as_bytes())
            .map_err(|e| format!("Failed to write message: {}", e))?;

        stdin
            .flush()
            .map_err(|e| format!("Failed to flush: {}", e))?;

        Ok(())
    }

    /// Read response from stdout
    fn read_response(&mut self, expected_id: u64) -> Result<LspResponse, String> {
        let stdout = self.stdout.as_mut().ok_or("No stdout available")?;

        loop {
            // Read headers
            let mut headers = Vec::new();
            let mut line = String::new();

            loop {
                line.clear();
                stdout
                    .read_line(&mut line)
                    .map_err(|e| format!("Failed to read line: {}", e))?;

                if line == "\r\n" || line == "\n" {
                    break;
                }

                headers.push(line.clone());
            }

            // Parse Content-Length
            let content_length: usize = headers
                .iter()
                .find(|h| h.starts_with("Content-Length:"))
                .and_then(|h| h.split(':').nth(1))
                .and_then(|s| s.trim().parse().ok())
                .ok_or("No Content-Length header")?;

            // Read content
            let mut buffer = vec![0u8; content_length];
            std::io::Read::read_exact(stdout, &mut buffer)
                .map_err(|e| format!("Failed to read content: {}", e))?;

            let content = String::from_utf8(buffer).map_err(|e| format!("Invalid UTF-8: {}", e))?;

            // Parse message
            let message: LspMessage = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse message: {}", e))?;

            match message {
                LspMessage::Response(response) => {
                    if response.id == expected_id {
                        return Ok(response);
                    }
                    // Wrong response ID, continue reading
                }
                LspMessage::Notification(_notification) => {
                    // Handle notification (e.g., diagnostics) - for now, ignore
                    continue;
                }
                LspMessage::Request(_request) => {
                    // Server sent a request - for now, ignore
                    continue;
                }
            }
        }
    }

    /// Get next request ID
    fn next_request_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Get completions at position
    pub fn get_completions(
        &mut self,
        file_uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<CompletionItem>, String> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": file_uri
            },
            "position": {
                "line": line,
                "character": character
            }
        });

        let id = self.next_request_id();
        let request = LspRequest::new(id, "textDocument/completion", Some(params));

        let response = self.send_request(request)?;

        if let Some(result) = response.result {
            // Result can be CompletionList or Vec<CompletionItem>
            if let Some(items) = result.get("items") {
                // CompletionList
                return serde_json::from_value(items.clone())
                    .map_err(|e| format!("Failed to parse completion items: {}", e));
            } else {
                // Direct Vec<CompletionItem>
                return serde_json::from_value(result)
                    .map_err(|e| format!("Failed to parse completion items: {}", e));
            }
        }

        Ok(Vec::new())
    }

    /// Get hover information at position
    pub fn get_hover(
        &mut self,
        file_uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Option<Hover>, String> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": file_uri
            },
            "position": {
                "line": line,
                "character": character
            }
        });

        let id = self.next_request_id();
        let request = LspRequest::new(id, "textDocument/hover", Some(params));

        let response = self.send_request(request)?;

        if let Some(result) = response.result {
            if result.is_null() {
                return Ok(None);
            }

            let hover: Hover = serde_json::from_value(result)
                .map_err(|e| format!("Failed to parse hover: {}", e))?;

            return Ok(Some(hover));
        }

        Ok(None)
    }

    /// Shutdown LSP server
    pub fn shutdown(&mut self) -> Result<(), String> {
        // Send shutdown request
        let id = self.next_request_id();
        let request = LspRequest::new(id, "shutdown", None);
        let _ = self.send_request(request); // Ignore errors

        // Send exit notification
        let notification = LspNotification::new("exit", None);
        let _ = self.send_notification(notification); // Ignore errors

        // Kill process
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }

        Ok(())
    }

    /// Get server capabilities
    pub fn capabilities(&self) -> &ServerCapabilities {
        &self.capabilities
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_server_command_rust() {
        let (cmd, args) = LspClient::get_server_command("rust").unwrap();
        assert_eq!(cmd, "rust-analyzer");
        assert_eq!(args.len(), 0);
    }

    #[test]
    fn test_get_server_command_typescript() {
        let (cmd, args) = LspClient::get_server_command("typescript").unwrap();
        assert_eq!(cmd, "typescript-language-server");
        assert!(args.contains(&"--stdio".to_string()));
    }

    #[test]
    fn test_get_server_command_unsupported() {
        let result = LspClient::get_server_command("unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_next_request_id() {
        let client = LspClient {
            language: "rust".to_string(),
            process: None,
            stdin: None,
            stdout: None,
            request_id: Arc::new(AtomicU64::new(1)),
            capabilities: ServerCapabilities::default(),
        };

        assert_eq!(client.next_request_id(), 1);
        assert_eq!(client.next_request_id(), 2);
        assert_eq!(client.next_request_id(), 3);
    }
}
