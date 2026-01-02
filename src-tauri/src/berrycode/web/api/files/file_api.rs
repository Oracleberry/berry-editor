//! File browser and editor API

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::berrycode::web::infrastructure::error::{
    file_not_found, invalid_path, path_traversal, session_not_found, FileError, WebError,
    WebResult,
};
use crate::berrycode::web::infrastructure::session_db::SessionDbStore;

/// File API state
#[derive(Clone)]
pub struct FileApiState {
    pub session_store: SessionDbStore,
}

/// File tree node
#[derive(Debug, Serialize, Deserialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileNode>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_status: Option<String>,  // "modified", "staged", "untracked", "conflict"
}

/// File content response
#[derive(Debug, Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub language: Option<String>,
}

/// File save request
#[derive(Debug, Serialize, Deserialize)]
pub struct FileSaveRequest {
    pub content: String,
}

/// List files query params
#[derive(Debug, Deserialize)]
pub struct ListFilesQuery {
    pub session_id: String,
    pub path: Option<String>,
}

/// Get file tree
pub async fn get_file_tree(
    Query(query): Query<ListFilesQuery>,
    State(state): State<FileApiState>,
) -> WebResult<Json<FileNode>> {
    tracing::debug!(session_id = %query.session_id, "Getting file tree");

    // Get session
    let session = state
        .session_store
        .get_session(&query.session_id)
        .await
        .ok_or_else(|| session_not_found(&query.session_id))?;

    let root_path = &session.project_root;
    let scan_path = if let Some(ref p) = query.path {
        root_path.join(p)
    } else {
        root_path.clone()
    };

    // Security: ensure path is within project root
    if !scan_path.starts_with(root_path) {
        tracing::warn!(
            scan_path = %scan_path.display(),
            root_path = %root_path.display(),
            "Path traversal attempt detected"
        );
        return Err(path_traversal(scan_path.to_string_lossy()));
    }

    // Get git status for the project
    let git_status_map = get_git_status_map(root_path);

    let tree = build_file_tree(&scan_path, root_path, &git_status_map)?;

    tracing::debug!(session_id = %query.session_id, "File tree built successfully");
    Ok(Json(tree))
}

/// Get git status map for all files in the repository
fn get_git_status_map(root: &PathBuf) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;
    use std::process::Command;

    let mut status_map = HashMap::new();

    // Run git status --porcelain
    let output = Command::new("git")
        .args(&["status", "--porcelain"])
        .current_dir(root)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.len() < 3 {
                    continue;
                }

                let status_code = &line[0..2];
                let file_path = &line[3..];

                // Determine status based on git status --porcelain format
                // XY format: X = index status, Y = working tree status
                let status = match status_code {
                    "??" => "untracked",
                    "A " | "M " | "D " => "staged",  // Staged changes
                    " M" | " D" => "modified",  // Modified in working tree
                    "MM" | "AM" | "DM" => "modified",  // Both staged and modified
                    "UU" | "AA" | "DD" => "conflict",  // Conflict
                    _ => {
                        let first_char = status_code.chars().next().unwrap();
                        let second_char = status_code.chars().nth(1).unwrap();

                        if first_char != ' ' && second_char == ' ' {
                            "staged"
                        } else if second_char != ' ' {
                            "modified"
                        } else {
                            "staged"
                        }
                    }
                };

                status_map.insert(file_path.to_string(), status.to_string());
            }
        }
    }

    status_map
}

/// Build file tree recursively
fn build_file_tree(
    path: &PathBuf,
    root: &PathBuf,
    git_status_map: &std::collections::HashMap<String, String>,
) -> WebResult<FileNode> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let relative_path = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    let is_dir = path.is_dir();

    let children = if is_dir {
        let mut child_nodes = Vec::new();

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();

                // Skip hidden files and common ignored directories
                if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                    // Skip .git directory but allow important config files
                    let is_hidden_but_important = name.starts_with('.') && (
                        name == ".gitignore"
                        || name == ".env"
                        || name == ".env.example"
                        || name.starts_with(".berrycode")
                        || name == ".dockerignore"
                        || name == ".editorconfig"
                        || name == ".prettierrc"
                        || name == ".eslintrc"
                        || name == ".cargo"
                    );

                    let should_skip = (name.starts_with('.') && !is_hidden_but_important)
                        || name == "node_modules"
                        || name == "target"
                        || name == "dist"
                        || name == "build"
                        || name == "data";

                    if should_skip {
                        continue;
                    }
                }

                // Skip binary and database files (but allow images)
                if !entry_path.is_dir() {
                    if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                        let skip_extensions = [
                            "db", "sqlite", "sqlite3", "lock",
                            "exe", "dll", "so", "dylib",
                            "zip", "tar", "gz", "bz2", "7z",
                            "pdf", "doc", "docx",
                        ];
                        if skip_extensions.contains(&ext) {
                            continue;
                        }
                    }
                }

                if let Ok(node) = build_file_tree(&entry_path, root, git_status_map) {
                    child_nodes.push(node);
                }
            }
        }

        // Sort: directories first, then files
        child_nodes.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        Some(child_nodes)
    } else {
        None
    };

    // Get git status for this file
    let git_status = if !is_dir {
        git_status_map.get(&relative_path).cloned()
    } else {
        None
    };

    Ok(FileNode {
        name,
        path: relative_path,
        is_dir,
        children,
        git_status,
    })
}

/// Read file content
pub async fn read_file(
    Path(file_path): Path<String>,
    Query(query): Query<ListFilesQuery>,
    State(state): State<FileApiState>,
) -> WebResult<Json<FileContent>> {
    tracing::debug!(session_id = %query.session_id, file_path = %file_path, "Reading file");

    // Get session
    let session = state
        .session_store
        .get_session(&query.session_id)
        .await
        .ok_or_else(|| session_not_found(&query.session_id))?;

    let full_path = session.project_root.join(&file_path);

    // Security: ensure path is within project root
    if !full_path.starts_with(&session.project_root) {
        tracing::warn!(
            file_path = %file_path,
            full_path = %full_path.display(),
            "Path traversal attempt detected"
        );
        return Err(path_traversal(&file_path));
    }

    if !full_path.exists() {
        return Err(file_not_found(&file_path));
    }

    let content = fs::read_to_string(&full_path).map_err(|e| {
        tracing::error!(error = %e, file_path = %file_path, "Failed to read file");
        WebError::File(FileError::ReadError(format!("Failed to read file: {}", e)))
    })?;

    let language = detect_language(&file_path);

    tracing::debug!(file_path = %file_path, size = %content.len(), "File read successfully");
    Ok(Json(FileContent {
        path: file_path,
        content,
        language,
    }))
}

/// Write file content
pub async fn write_file(
    Path(file_path): Path<String>,
    Query(query): Query<ListFilesQuery>,
    State(state): State<FileApiState>,
    Json(payload): Json<FileSaveRequest>,
) -> WebResult<StatusCode> {
    tracing::debug!(session_id = %query.session_id, file_path = %file_path, "Writing file");

    // Get session
    let session = state
        .session_store
        .get_session(&query.session_id)
        .await
        .ok_or_else(|| session_not_found(&query.session_id))?;

    let full_path = session.project_root.join(&file_path);

    // Security: ensure path is within project root
    if !full_path.starts_with(&session.project_root) {
        tracing::warn!(
            file_path = %file_path,
            full_path = %full_path.display(),
            "Path traversal attempt detected"
        );
        return Err(path_traversal(&file_path));
    }

    // Create parent directories if needed
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            tracing::error!(error = %e, parent = %parent.display(), "Failed to create parent directory");
            WebError::File(FileError::DirectoryError(format!(
                "Failed to create parent directory: {}",
                e
            )))
        })?;
    }

    fs::write(&full_path, &payload.content).map_err(|e| {
        tracing::error!(error = %e, file_path = %file_path, "Failed to write file");
        WebError::File(FileError::WriteError(format!("Failed to write file: {}", e)))
    })?;

    tracing::info!(file_path = %file_path, size = %payload.content.len(), "File written successfully");
    Ok(StatusCode::OK)
}

/// Detect programming language from file extension
fn detect_language(file_path: &str) -> Option<String> {
    let ext = file_path.rsplit('.').next()?;

    let language = match ext.to_lowercase().as_str() {
        "rs" => "rust",
        "py" => "python",
        "js" => "javascript",
        "ts" => "typescript",
        "jsx" => "javascript",
        "tsx" => "typescript",
        "html" => "html",
        "css" => "css",
        "json" => "json",
        "md" => "markdown",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "sh" => "bash",
        "c" => "c",
        "cpp" | "cc" | "cxx" => "cpp",
        "go" => "go",
        "java" => "java",
        _ => return None,
    };

    Some(language.to_string())
}

/// Markdown preview response
#[derive(Debug, Serialize, Deserialize)]
pub struct MarkdownPreview {
    pub html: String,
}

/// Render Markdown to HTML
pub async fn render_markdown(
    Query(query): Query<ListFilesQuery>,
    State(state): State<FileApiState>,
) -> WebResult<Json<MarkdownPreview>> {
    let session = state
        .session_store
        .get_session(&query.session_id)
        .await
        .ok_or_else(|| session_not_found(&query.session_id))?;

    let project_root = PathBuf::from(&session.project_root);
    let file_path = project_root.join(query.path.as_deref().unwrap_or(""));

    // Security: prevent path traversal
    if !file_path.starts_with(&project_root) {
        return Err(path_traversal(file_path.to_string_lossy().to_string()));
    }

    if !file_path.exists() {
        return Err(file_not_found(file_path.to_string_lossy().to_string()));
    }

    let content = fs::read_to_string(&file_path).map_err(|e| {
        invalid_path(format!("Failed to read file: {}", e))
    })?;

    // Use comrak to render markdown to HTML
    use comrak::{markdown_to_html, ComrakOptions};
    let mut options = ComrakOptions::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.render.unsafe_ = false; // Disable raw HTML for security

    let html = markdown_to_html(&content, &options);

    Ok(Json(MarkdownPreview { html }))
}

/// Delete file request
#[derive(Debug, Deserialize)]
pub struct DeleteFileRequest {
    pub session_id: String,
    pub file_path: String,
}

/// Delete file response
#[derive(Debug, Serialize)]
pub struct DeleteFileResponse {
    pub success: bool,
    pub message: String,
}

/// Delete a file (move to trash for safety)
pub async fn delete_file(
    State(state): State<FileApiState>,
    Json(request): Json<DeleteFileRequest>,
) -> WebResult<Json<DeleteFileResponse>> {
    tracing::debug!(
        session_id = %request.session_id,
        file_path = %request.file_path,
        "Deleting file"
    );

    // Get session
    let session = state
        .session_store
        .get_session(&request.session_id)
        .await
        .ok_or_else(|| session_not_found(&request.session_id))?;

    let project_root = PathBuf::from(&session.project_root);
    let file_path = project_root.join(&request.file_path);

    // Security check: prevent deletion outside project root
    if !file_path.starts_with(&project_root) {
        tracing::warn!(
            file_path = %file_path.display(),
            project_root = %project_root.display(),
            "Path traversal attempt detected"
        );
        return Err(path_traversal(file_path.to_string_lossy().to_string()));
    }

    if !file_path.exists() {
        return Err(file_not_found(file_path.to_string_lossy().to_string()));
    }

    // Move to trash instead of permanent deletion
    match trash::delete(&file_path) {
        Ok(_) => {
            tracing::info!(file_path = %file_path.display(), "File moved to trash");
            Ok(Json(DeleteFileResponse {
                success: true,
                message: format!("Moved {} to trash", file_path.display()),
            }))
        }
        Err(e) => {
            tracing::error!(
                file_path = %file_path.display(),
                error = %e,
                "Failed to delete file"
            );
            Err(WebError::Internal(format!("Failed to delete file: {}", e)))
        }
    }
}

/// Apply edits request (for LSP rename across multiple files)
#[derive(Debug, Deserialize)]
pub struct ApplyEditsRequest {
    pub file_path: String,
    pub edits: Vec<TextEdit>,
}

#[derive(Debug, Deserialize)]
pub struct TextEdit {
    pub range: TextRange,
    pub new_text: String,
}

#[derive(Debug, Deserialize)]
pub struct TextRange {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Deserialize)]
pub struct Position {
    pub line: usize,
    pub character: usize,
}

/// Apply edits response
#[derive(Debug, Serialize)]
pub struct ApplyEditsResponse {
    pub success: bool,
}

/// Apply text edits to a file
pub async fn apply_edits(
    Json(request): Json<ApplyEditsRequest>,
) -> WebResult<Json<ApplyEditsResponse>> {
    let file_path = PathBuf::from(&request.file_path);

    if !file_path.exists() {
        return Err(file_not_found(file_path.to_string_lossy().to_string()));
    }

    // Read file content
    let content = fs::read_to_string(&file_path).map_err(|e| {
        invalid_path(format!("Failed to read file: {}", e))
    })?;

    // Split into lines
    let mut lines: Vec<String> = content.lines().map(String::from).collect();

    // Apply edits in reverse order to maintain line/character positions
    let mut sorted_edits = request.edits;
    sorted_edits.sort_by(|a, b| {
        b.range.start.line.cmp(&a.range.start.line)
            .then_with(|| b.range.start.character.cmp(&a.range.start.character))
    });

    for edit in sorted_edits {
        let start_line = edit.range.start.line;
        let start_char = edit.range.start.character;
        let end_line = edit.range.end.line;
        let end_char = edit.range.end.character;

        if start_line >= lines.len() {
            continue;
        }

        if start_line == end_line {
            // Single line edit
            if let Some(line) = lines.get_mut(start_line) {
                let before = line.chars().take(start_char).collect::<String>();
                let after = line.chars().skip(end_char).collect::<String>();
                *line = format!("{}{}{}", before, edit.new_text, after);
            }
        } else {
            // Multi-line edit
            if let Some(start_line_text) = lines.get(start_line) {
                let before = start_line_text.chars().take(start_char).collect::<String>();

                if let Some(end_line_text) = lines.get(end_line) {
                    let after = end_line_text.chars().skip(end_char).collect::<String>();

                    // Replace the range with new text
                    let new_line = format!("{}{}{}", before, edit.new_text, after);

                    // Remove lines in the range
                    lines.drain(start_line..=end_line);

                    // Insert new line
                    lines.insert(start_line, new_line);
                }
            }
        }
    }

    // Write back to file
    let new_content = lines.join("\n");
    fs::write(&file_path, new_content).map_err(|e| {
        invalid_path(format!("Failed to write file: {}", e))
    })?;

    Ok(Json(ApplyEditsResponse { success: true }))
}
