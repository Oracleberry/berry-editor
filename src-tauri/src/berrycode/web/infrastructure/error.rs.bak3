//! Custom error types for BerryCode Web

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::fmt;

/// Main error type for BerryCode Web
#[derive(Debug)]
pub enum WebError {
    /// Database-related errors
    Database(DatabaseError),
    /// Authentication errors
    Auth(AuthError),
    /// Session errors
    Session(SessionError),
    /// File operation errors
    File(FileError),
    /// WebSocket errors
    WebSocket(String),
    /// Configuration errors
    Config(String),
    /// Template rendering errors
    Template(String),
    /// Internal server errors
    Internal(String),
    /// Bad request errors
    BadRequest(String),
    /// Not found errors
    NotFound(String),
    /// Permission denied errors
    PermissionDenied(String),
    /// Git operation errors
    Git(GitError),
    /// Terminal operation errors
    Terminal(TerminalError),
    /// Validation errors
    Validation(ValidationError),
}

/// Database-specific errors
#[derive(Debug)]
pub enum DatabaseError {
    /// Connection error
    Connection(String),
    /// Query execution error
    Query(String),
    /// Transaction error
    Transaction(String),
    /// Record not found
    NotFound(String),
    /// Constraint violation
    Constraint(String),
    /// Migration error
    Migration(String),
    /// Generic database error
    Other(anyhow::Error),
}

/// Authentication-specific errors
#[derive(Debug)]
pub enum AuthError {
    /// Invalid credentials
    InvalidCredentials,
    /// Token expired
    TokenExpired,
    /// Token invalid
    InvalidToken,
    /// User not found
    UserNotFound,
    /// User already exists
    UserAlreadyExists,
    /// Insufficient permissions
    InsufficientPermissions,
    /// Password hashing error
    HashingError(String),
    /// Session creation failed
    SessionCreationFailed,
}

/// Session-specific errors
#[derive(Debug)]
pub enum SessionError {
    /// Session not found
    NotFound(String),
    /// Session expired
    Expired(String),
    /// Invalid session data
    Invalid(String),
    /// Session creation failed
    CreationFailed(String),
}

/// File operation-specific errors
#[derive(Debug)]
pub enum FileError {
    /// File not found
    NotFound(String),
    /// Permission denied
    PermissionDenied(String),
    /// Invalid path
    InvalidPath(String),
    /// Path traversal attempt
    PathTraversal(String),
    /// Read error
    ReadError(String),
    /// Write error
    WriteError(String),
    /// Directory operation error
    DirectoryError(String),
    /// IO error
    IoError(std::io::Error),
}

/// Git operation-specific errors
#[derive(Debug)]
pub enum GitError {
    /// Repository not found
    RepoNotFound,
    /// Invalid repository
    InvalidRepo(String),
    /// Commit failed
    CommitFailed(String),
    /// Branch operation failed
    BranchError(String),
    /// Merge conflict
    MergeConflict(String),
    /// Status error
    StatusError(String),
    /// Diff error
    DiffError(String),
}

/// Terminal operation-specific errors
#[derive(Debug)]
pub enum TerminalError {
    /// Command execution failed
    ExecutionFailed(String),
    /// Invalid command
    InvalidCommand(String),
    /// Command timeout
    Timeout,
    /// Process not found
    ProcessNotFound(String),
    /// Permission denied
    PermissionDenied(String),
}

/// Validation-specific errors
#[derive(Debug)]
pub enum ValidationError {
    /// Missing required field
    MissingField(String),
    /// Invalid field format
    InvalidFormat { field: String, reason: String },
    /// Value out of range
    OutOfRange { field: String, min: i64, max: i64, actual: i64 },
    /// Invalid length
    InvalidLength { field: String, min: usize, max: usize, actual: usize },
    /// Custom validation error
    Custom(String),
}

impl fmt::Display for WebError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebError::Database(e) => write!(f, "Database error: {}", e),
            WebError::Auth(e) => write!(f, "Authentication error: {}", e),
            WebError::Session(e) => write!(f, "Session error: {}", e),
            WebError::File(e) => write!(f, "File error: {}", e),
            WebError::WebSocket(msg) => write!(f, "WebSocket error: {}", msg),
            WebError::Config(msg) => write!(f, "Configuration error: {}", msg),
            WebError::Template(msg) => write!(f, "Template error: {}", msg),
            WebError::Internal(msg) => write!(f, "Internal error: {}", msg),
            WebError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            WebError::NotFound(msg) => write!(f, "Not found: {}", msg),
            WebError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            WebError::Git(e) => write!(f, "Git error: {}", e),
            WebError::Terminal(e) => write!(f, "Terminal error: {}", e),
            WebError::Validation(e) => write!(f, "Validation error: {}", e),
        }
    }
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseError::Connection(msg) => write!(f, "Database connection failed: {}", msg),
            DatabaseError::Query(msg) => write!(f, "Query execution failed: {}", msg),
            DatabaseError::Transaction(msg) => write!(f, "Transaction failed: {}", msg),
            DatabaseError::NotFound(msg) => write!(f, "Record not found: {}", msg),
            DatabaseError::Constraint(msg) => write!(f, "Constraint violation: {}", msg),
            DatabaseError::Migration(msg) => write!(f, "Migration failed: {}", msg),
            DatabaseError::Other(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::InvalidCredentials => write!(f, "Invalid username or password"),
            AuthError::TokenExpired => write!(f, "Authentication token has expired"),
            AuthError::InvalidToken => write!(f, "Invalid authentication token"),
            AuthError::UserNotFound => write!(f, "User not found"),
            AuthError::UserAlreadyExists => write!(f, "User already exists"),
            AuthError::InsufficientPermissions => write!(f, "Insufficient permissions"),
            AuthError::HashingError(msg) => write!(f, "Password hashing error: {}", msg),
            AuthError::SessionCreationFailed => write!(f, "Failed to create session"),
        }
    }
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::NotFound(id) => write!(f, "Session not found: {}", id),
            SessionError::Expired(id) => write!(f, "Session expired: {}", id),
            SessionError::Invalid(msg) => write!(f, "Invalid session: {}", msg),
            SessionError::CreationFailed(msg) => write!(f, "Session creation failed: {}", msg),
        }
    }
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::NotFound(path) => write!(f, "File not found: {}", path),
            FileError::PermissionDenied(path) => write!(f, "Permission denied: {}", path),
            FileError::InvalidPath(path) => write!(f, "Invalid path: {}", path),
            FileError::PathTraversal(path) => write!(f, "Path traversal attempt detected: {}", path),
            FileError::ReadError(msg) => write!(f, "Failed to read file: {}", msg),
            FileError::WriteError(msg) => write!(f, "Failed to write file: {}", msg),
            FileError::DirectoryError(msg) => write!(f, "Directory operation failed: {}", msg),
            FileError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::RepoNotFound => write!(f, "Git repository not found"),
            GitError::InvalidRepo(msg) => write!(f, "Invalid repository: {}", msg),
            GitError::CommitFailed(msg) => write!(f, "Commit failed: {}", msg),
            GitError::BranchError(msg) => write!(f, "Branch operation failed: {}", msg),
            GitError::MergeConflict(msg) => write!(f, "Merge conflict: {}", msg),
            GitError::StatusError(msg) => write!(f, "Failed to get status: {}", msg),
            GitError::DiffError(msg) => write!(f, "Failed to get diff: {}", msg),
        }
    }
}

impl fmt::Display for TerminalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TerminalError::ExecutionFailed(msg) => write!(f, "Command execution failed: {}", msg),
            TerminalError::InvalidCommand(cmd) => write!(f, "Invalid command: {}", cmd),
            TerminalError::Timeout => write!(f, "Command execution timed out"),
            TerminalError::ProcessNotFound(id) => write!(f, "Process not found: {}", id),
            TerminalError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::MissingField(field) => write!(f, "Missing required field: {}", field),
            ValidationError::InvalidFormat { field, reason } => {
                write!(f, "Invalid format for field '{}': {}", field, reason)
            }
            ValidationError::OutOfRange { field, min, max, actual } => {
                write!(f, "Field '{}' value {} is out of range [{}, {}]", field, actual, min, max)
            }
            ValidationError::InvalidLength { field, min, max, actual } => {
                write!(f, "Field '{}' length {} is invalid (expected {} to {})", field, actual, min, max)
            }
            ValidationError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for WebError {}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        // Log the error with appropriate level
        match &self {
            WebError::Database(_) | WebError::Internal(_) => {
                tracing::error!(error = %self, "Server error occurred");
            }
            WebError::Auth(_) | WebError::Session(_) => {
                tracing::warn!(error = %self, "Authentication/Session error");
            }
            WebError::Validation(_) | WebError::BadRequest(_) => {
                tracing::debug!(error = %self, "Validation error");
            }
            _ => {
                tracing::info!(error = %self, "Error occurred");
            }
        }

        let (status, error_message, error_type) = match &self {
            WebError::Database(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                match e {
                    DatabaseError::NotFound(_) => self.to_string(),
                    _ => "Database error occurred".to_string(),
                },
                "Database",
            ),
            WebError::Auth(e) => (
                match e {
                    AuthError::InvalidCredentials => StatusCode::UNAUTHORIZED,
                    AuthError::TokenExpired | AuthError::InvalidToken => StatusCode::UNAUTHORIZED,
                    AuthError::InsufficientPermissions => StatusCode::FORBIDDEN,
                    AuthError::UserAlreadyExists => StatusCode::CONFLICT,
                    _ => StatusCode::UNAUTHORIZED,
                },
                self.to_string(),
                "Auth",
            ),
            WebError::Session(e) => (
                match e {
                    SessionError::NotFound(_) => StatusCode::NOT_FOUND,
                    SessionError::Expired(_) => StatusCode::UNAUTHORIZED,
                    _ => StatusCode::BAD_REQUEST,
                },
                self.to_string(),
                "Session",
            ),
            WebError::File(e) => (
                match e {
                    FileError::NotFound(_) => StatusCode::NOT_FOUND,
                    FileError::PermissionDenied(_) | FileError::PathTraversal(_) => StatusCode::FORBIDDEN,
                    FileError::InvalidPath(_) => StatusCode::BAD_REQUEST,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                },
                self.to_string(),
                "File",
            ),
            WebError::Git(e) => (
                match e {
                    GitError::RepoNotFound => StatusCode::NOT_FOUND,
                    GitError::MergeConflict(_) => StatusCode::CONFLICT,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                },
                self.to_string(),
                "Git",
            ),
            WebError::Terminal(e) => (
                match e {
                    TerminalError::InvalidCommand(_) => StatusCode::BAD_REQUEST,
                    TerminalError::PermissionDenied(_) => StatusCode::FORBIDDEN,
                    TerminalError::Timeout => StatusCode::REQUEST_TIMEOUT,
                    TerminalError::ProcessNotFound(_) => StatusCode::NOT_FOUND,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                },
                self.to_string(),
                "Terminal",
            ),
            WebError::Validation(_) => (StatusCode::BAD_REQUEST, self.to_string(), "Validation"),
            WebError::WebSocket(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone(), "WebSocket"),
            WebError::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone(), "Config"),
            WebError::Template(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone(), "Template"),
            WebError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone(), "Internal"),
            WebError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone(), "BadRequest"),
            WebError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone(), "NotFound"),
            WebError::PermissionDenied(msg) => (StatusCode::FORBIDDEN, msg.clone(), "PermissionDenied"),
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16(),
            "type": error_type,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }));

        (status, body).into_response()
    }
}

impl From<anyhow::Error> for WebError {
    fn from(err: anyhow::Error) -> Self {
        WebError::Database(DatabaseError::Other(err))
    }
}

impl From<std::io::Error> for WebError {
    fn from(err: std::io::Error) -> Self {
        WebError::File(FileError::IoError(err))
    }
}

impl From<tera::Error> for WebError {
    fn from(err: tera::Error) -> Self {
        WebError::Template(format!("Template error: {}", err))
    }
}

impl From<axum::Error> for WebError {
    fn from(err: axum::Error) -> Self {
        WebError::Internal(format!("Axum error: {}", err))
    }
}

// WebSocket error conversion - requires tokio_tungstenite in Cargo.toml
// #[cfg(feature = "web")]
// impl From<tokio_tungstenite::tungstenite::Error> for WebError {
//     fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
//         WebError::WebSocket(format!("WebSocket error: {}", err))
//     }
// }

#[cfg(feature = "web")]
impl From<sqlx::Error> for WebError {
    fn from(err: sqlx::Error) -> Self {
        use sqlx::Error;
        match err {
            Error::RowNotFound => WebError::Database(DatabaseError::NotFound("Row not found".to_string())),
            Error::Database(db_err) => {
                if let Some(constraint) = db_err.constraint() {
                    WebError::Database(DatabaseError::Constraint(constraint.to_string()))
                } else {
                    WebError::Database(DatabaseError::Query(db_err.to_string()))
                }
            }
            Error::PoolTimedOut | Error::PoolClosed => {
                WebError::Database(DatabaseError::Connection("Connection pool error".to_string()))
            }
            _ => WebError::Database(DatabaseError::Other(anyhow::Error::new(err))),
        }
    }
}

impl From<DatabaseError> for WebError {
    fn from(err: DatabaseError) -> Self {
        WebError::Database(err)
    }
}

impl From<AuthError> for WebError {
    fn from(err: AuthError) -> Self {
        WebError::Auth(err)
    }
}

impl From<SessionError> for WebError {
    fn from(err: SessionError) -> Self {
        WebError::Session(err)
    }
}

impl From<FileError> for WebError {
    fn from(err: FileError) -> Self {
        WebError::File(err)
    }
}

impl From<GitError> for WebError {
    fn from(err: GitError) -> Self {
        WebError::Git(err)
    }
}

impl From<TerminalError> for WebError {
    fn from(err: TerminalError) -> Self {
        WebError::Terminal(err)
    }
}

impl From<ValidationError> for WebError {
    fn from(err: ValidationError) -> Self {
        WebError::Validation(err)
    }
}

/// Result type alias for WebError
pub type WebResult<T> = Result<T, WebError>;

// ============== General Error Helpers ==============

/// Helper function to create a bad request error
pub fn bad_request(msg: impl Into<String>) -> WebError {
    WebError::BadRequest(msg.into())
}

/// Helper function to create a not found error
pub fn not_found(msg: impl Into<String>) -> WebError {
    WebError::NotFound(msg.into())
}

/// Helper function to create an internal error
pub fn internal_error(msg: impl Into<String>) -> WebError {
    WebError::Internal(msg.into())
}

/// Helper function to create a permission denied error
pub fn permission_denied(msg: impl Into<String>) -> WebError {
    WebError::PermissionDenied(msg.into())
}

// ============== Auth Error Helpers ==============

/// Helper for invalid credentials error
pub fn invalid_credentials() -> WebError {
    WebError::Auth(AuthError::InvalidCredentials)
}

/// Helper for token expired error
pub fn token_expired() -> WebError {
    WebError::Auth(AuthError::TokenExpired)
}

/// Helper for invalid token error
pub fn invalid_token() -> WebError {
    WebError::Auth(AuthError::InvalidToken)
}

/// Helper for user not found error
pub fn user_not_found() -> WebError {
    WebError::Auth(AuthError::UserNotFound)
}

/// Helper for user already exists error
pub fn user_already_exists() -> WebError {
    WebError::Auth(AuthError::UserAlreadyExists)
}

/// Helper for insufficient permissions error
pub fn insufficient_permissions() -> WebError {
    WebError::Auth(AuthError::InsufficientPermissions)
}

// ============== Session Error Helpers ==============

/// Helper for session not found error
pub fn session_not_found(session_id: impl Into<String>) -> WebError {
    WebError::Session(SessionError::NotFound(session_id.into()))
}

/// Helper for session expired error
pub fn session_expired(session_id: impl Into<String>) -> WebError {
    WebError::Session(SessionError::Expired(session_id.into()))
}

// ============== File Error Helpers ==============

/// Helper for file not found error
pub fn file_not_found(path: impl Into<String>) -> WebError {
    WebError::File(FileError::NotFound(path.into()))
}

/// Helper for file permission denied error
pub fn file_permission_denied(path: impl Into<String>) -> WebError {
    WebError::File(FileError::PermissionDenied(path.into()))
}

/// Helper for path traversal error
pub fn path_traversal(path: impl Into<String>) -> WebError {
    WebError::File(FileError::PathTraversal(path.into()))
}

/// Helper for invalid path error
pub fn invalid_path(path: impl Into<String>) -> WebError {
    WebError::File(FileError::InvalidPath(path.into()))
}

// ============== Git Error Helpers ==============

/// Helper for git repo not found error
pub fn git_repo_not_found() -> WebError {
    WebError::Git(GitError::RepoNotFound)
}

/// Helper for git commit failed error
pub fn git_commit_failed(msg: impl Into<String>) -> WebError {
    WebError::Git(GitError::CommitFailed(msg.into()))
}

// ============== Terminal Error Helpers ==============

/// Helper for command execution failed error
pub fn command_execution_failed(msg: impl Into<String>) -> WebError {
    WebError::Terminal(TerminalError::ExecutionFailed(msg.into()))
}

/// Helper for invalid command error
pub fn invalid_command(cmd: impl Into<String>) -> WebError {
    WebError::Terminal(TerminalError::InvalidCommand(cmd.into()))
}

/// Helper for command timeout error
pub fn command_timeout() -> WebError {
    WebError::Terminal(TerminalError::Timeout)
}

// ============== Validation Error Helpers ==============

/// Helper for missing field error
pub fn missing_field(field: impl Into<String>) -> WebError {
    WebError::Validation(ValidationError::MissingField(field.into()))
}

/// Helper for invalid format error
pub fn invalid_format(field: impl Into<String>, reason: impl Into<String>) -> WebError {
    WebError::Validation(ValidationError::InvalidFormat {
        field: field.into(),
        reason: reason.into(),
    })
}

/// Helper for value out of range error
pub fn out_of_range(field: impl Into<String>, min: i64, max: i64, actual: i64) -> WebError {
    WebError::Validation(ValidationError::OutOfRange {
        field: field.into(),
        min,
        max,
        actual,
    })
}

/// Helper for invalid length error
pub fn invalid_length(field: impl Into<String>, min: usize, max: usize, actual: usize) -> WebError {
    WebError::Validation(ValidationError::InvalidLength {
        field: field.into(),
        min,
        max,
        actual,
    })
}