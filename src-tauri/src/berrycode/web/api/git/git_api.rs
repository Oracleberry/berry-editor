//! Git API for web interface

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use git2::{Repository, StatusOptions};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::berrycode::web::infrastructure::session_db::SessionDbStore;
use crate::berrycode::repo::GitRepo;

/// Git API state
#[derive(Clone)]
pub struct GitApiState {
    pub session_store: SessionDbStore,
}

/// Git status response (matches JavaScript expectation)
#[derive(Debug, Serialize)]
pub struct GitStatusResponse {
    pub branch: Option<String>,
    pub modified: Vec<String>,
    pub staged: Vec<String>,
    pub untracked: Vec<String>,
}

/// Commit request
#[derive(Debug, Deserialize)]
pub struct GitCommitRequest {
    pub message: String,
    pub files: Option<Vec<String>>,
}

/// Get git status
pub async fn get_git_status(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<GitStatusResponse>, StatusCode> {
    // Get session
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    // Open git repo
    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get current branch
    let branch = repo
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(String::from));

    // Get git status with detailed categorization
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);

    let statuses = repo
        .statuses(Some(&mut opts))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut modified = Vec::new();
    let mut staged = Vec::new();
    let mut untracked = Vec::new();

    for entry in statuses.iter() {
        if let Some(path) = entry.path() {
            let status = entry.status();
            let path_str = path.to_string();

            // Untracked files (new in working tree, not in index)
            if status.is_wt_new() {
                untracked.push(path_str.clone());
            }

            // Staged files (in index)
            if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
                staged.push(path_str.clone());
            }

            // Modified in working tree (not staged)
            if status.is_wt_modified() || status.is_wt_deleted() {
                // Only add if not already in staged
                if !status.is_index_modified() && !status.is_index_deleted() {
                    modified.push(path_str.clone());
                }
            }
        }
    }

    Ok(Json(GitStatusResponse {
        branch,
        modified,
        staged,
        untracked,
    }))
}

/// Stage all files (git add -A)
pub async fn stage_all_files(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<StatusCode, StatusCode> {
    // Get session
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    // Open git repo
    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get index and add all files
    let mut index = repo.index().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Add all files (equivalent to git add -A)
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    index.write().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

/// Create git commit
pub async fn create_commit(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<GitCommitRequest>,
) -> Result<StatusCode, StatusCode> {
    // Get session
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    // Validate message
    if payload.message.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Open git repo
    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get signature
    let signature = repo
        .signature()
        .or_else(|_| {
            git2::Signature::now("BerryCode", "berrycode@example.com")
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get the current index and write tree
    let mut index = repo.index().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tree_id = index.write_tree().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tree = repo.find_tree(tree_id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get HEAD commit as parent
    let head = repo.head().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let parent_commit = head.peel_to_commit().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create commit
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        &payload.message,
        &tree,
        &[&parent_commit],
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

/// Get git diff
pub async fn get_git_diff(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<String>, StatusCode> {
    // Get session
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    // Open git repo
    let repo = GitRepo::new(Some(&session.project_root))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get diff
    let diff = repo.diff().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(diff))
}

/// Branch information
#[derive(Debug, Serialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
}

/// Get all branches (local and remote)
pub async fn get_branches(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<Vec<BranchInfo>>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut branches = Vec::new();

    // Get current branch
    let current_branch = repo
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(String::from));

    // Get local branches
    let local_branches = repo.branches(Some(git2::BranchType::Local))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for branch in local_branches {
        if let Ok((branch, _)) = branch {
            if let Some(name) = branch.name().ok().flatten() {
                branches.push(BranchInfo {
                    name: name.to_string(),
                    is_current: current_branch.as_ref().map_or(false, |cb| cb == name),
                    is_remote: false,
                });
            }
        }
    }

    // Get remote branches
    let remote_branches = repo.branches(Some(git2::BranchType::Remote))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for branch in remote_branches {
        if let Ok((branch, _)) = branch {
            if let Some(name) = branch.name().ok().flatten() {
                branches.push(BranchInfo {
                    name: name.to_string(),
                    is_current: false,
                    is_remote: true,
                });
            }
        }
    }

    Ok(Json(branches))
}

/// Switch branch request
#[derive(Debug, Deserialize)]
pub struct SwitchBranchRequest {
    pub branch_name: String,
}

/// Switch to a different branch
pub async fn switch_branch(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<SwitchBranchRequest>,
) -> Result<StatusCode, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Find the branch
    let branch = repo.find_branch(&payload.branch_name, git2::BranchType::Local)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Get the branch reference
    let branch_ref = branch.get();

    // Set HEAD to the branch
    repo.set_head(branch_ref.name().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Checkout the branch
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

/// Create branch request
#[derive(Debug, Deserialize)]
pub struct CreateBranchRequest {
    pub branch_name: String,
    pub from_branch: Option<String>,
}

/// Create a new branch
pub async fn create_branch(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<CreateBranchRequest>,
) -> Result<StatusCode, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get the commit to branch from
    let commit = if let Some(from_branch) = payload.from_branch {
        let branch = repo.find_branch(&from_branch, git2::BranchType::Local)
            .map_err(|_| StatusCode::NOT_FOUND)?;
        branch.get().peel_to_commit()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        repo.head()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .peel_to_commit()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    // Create the branch
    repo.branch(&payload.branch_name, &commit, false)
        .map_err(|_| StatusCode::CONFLICT)?;

    Ok(StatusCode::CREATED)
}

/// Commit history entry
#[derive(Debug, Serialize)]
pub struct CommitHistoryEntry {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

/// Get commit history
pub async fn get_commit_history(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<Vec<CommitHistoryEntry>>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut revwalk = repo.revwalk()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    revwalk.push_head()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut history = Vec::new();

    for oid in revwalk.take(50) {  // Limit to 50 commits
        if let Ok(oid) = oid {
            if let Ok(commit) = repo.find_commit(oid) {
                history.push(CommitHistoryEntry {
                    sha: commit.id().to_string()[..8].to_string(),
                    message: commit.message().unwrap_or("").to_string(),
                    author: commit.author().name().unwrap_or("Unknown").to_string(),
                    date: commit.time().seconds().to_string(),
                });
            }
        }
    }

    Ok(Json(history))
}

/// Stage file request
#[derive(Debug, Deserialize)]
pub struct StageFileRequest {
    pub file_path: String,
}

/// Stage a specific file
pub async fn stage_file(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<StageFileRequest>,
) -> Result<StatusCode, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut index = repo.index()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    index.add_path(std::path::Path::new(&payload.file_path))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    index.write()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

/// Unstage a specific file
pub async fn unstage_file(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<StageFileRequest>,
) -> Result<StatusCode, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let head = repo.head()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let head_commit = head.peel_to_commit()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let head_tree = head_commit.tree()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _index = repo.index()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Reset the file in index to HEAD
    repo.reset_default(Some(head_commit.as_object()), &[std::path::Path::new(&payload.file_path)])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

/// Push to remote repository
pub async fn git_push(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get current branch
    let head = repo.head().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let branch_name = head.shorthand().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get remote
    let mut remote = repo.find_remote("origin")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Push to remote
    let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
    remote.push(&[&refspec], None)
        .map_err(|e| {
            tracing::error!("Push failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(format!("Successfully pushed to origin/{}", branch_name)))
}

/// Pull from remote repository
pub async fn git_pull(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Fetch from remote
    let mut remote = repo.find_remote("origin")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    remote.fetch(&["refs/heads/*:refs/remotes/origin/*"], None, None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get current branch
    let head = repo.head().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let branch_name = head.shorthand().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get remote branch
    let remote_branch_name = format!("origin/{}", branch_name);
    let remote_branch_ref = repo.find_reference(&format!("refs/remotes/{}", remote_branch_name))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let remote_commit = remote_branch_ref.peel_to_commit()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get current commit
    let local_commit = head.peel_to_commit()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create annotated commit for merge analysis
    let annotated_commit = repo.find_annotated_commit(remote_commit.id())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Merge analysis
    let (analysis, _) = repo.merge_analysis(&[&annotated_commit])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if analysis.is_up_to_date() {
        return Ok(Json("Already up to date".to_string()));
    }

    if analysis.is_fast_forward() {
        // Fast-forward merge
        let refname = format!("refs/heads/{}", branch_name);
        let mut reference = repo.find_reference(&refname)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        reference.set_target(remote_commit.id(), "fast-forward")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        repo.set_head(&refname)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json("Successfully pulled (fast-forward)".to_string()))
    } else {
        Ok(Json("Cannot fast-forward, manual merge required".to_string()))
    }
}

/// Fetch from remote repository
pub async fn git_fetch(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut remote = repo.find_remote("origin")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    remote.fetch(&["refs/heads/*:refs/remotes/origin/*"], None, None)
        .map_err(|e| {
            tracing::error!("Fetch failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json("Successfully fetched from origin".to_string()))
}

/// Delete a branch
#[derive(Debug, Deserialize)]
pub struct DeleteBranchRequest {
    pub branch_name: String,
    pub force: Option<bool>,
}

pub async fn delete_branch(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<DeleteBranchRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut branch = repo.find_branch(&payload.branch_name, git2::BranchType::Local)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    branch.delete()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(format!("Branch '{}' deleted successfully", payload.branch_name)))
}

/// Merge a branch
#[derive(Debug, Deserialize)]
pub struct MergeBranchRequest {
    pub branch_name: String,
}

pub async fn merge_branch(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<MergeBranchRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Find the branch to merge
    let branch = repo.find_branch(&payload.branch_name, git2::BranchType::Local)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let branch_ref = branch.get();
    let branch_commit = branch_ref.peel_to_commit()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let annotated_commit = repo.find_annotated_commit(branch_commit.id())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Perform merge analysis
    let (analysis, _) = repo.merge_analysis(&[&annotated_commit])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if analysis.is_up_to_date() {
        return Ok(Json("Already up to date".to_string()));
    }

    if analysis.is_fast_forward() {
        // Fast-forward merge
        let head = repo.head().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut reference = repo.find_reference(head.name().unwrap())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        reference.set_target(branch_commit.id(), &format!("Fast-forward merge {}", payload.branch_name))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(format!("Fast-forward merged '{}'", payload.branch_name)))
    } else {
        // Normal merge - requires manual conflict resolution
        repo.merge(&[&annotated_commit], None, None)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let index = repo.index().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if index.has_conflicts() {
            Ok(Json("Merge has conflicts - please resolve them".to_string()))
        } else {
            Ok(Json(format!("Merged '{}' - please commit the changes", payload.branch_name)))
        }
    }
}

/// Stash changes
pub async fn stash_push(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let signature = repo.signature()
        .or_else(|_| git2::Signature::now("BerryCode", "berrycode@example.com"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    repo.stash_save(&signature, "Stashed changes", None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json("Changes stashed successfully".to_string()))
}

/// List stashes
#[derive(Debug, Serialize)]
pub struct StashInfo {
    pub index: usize,
    pub message: String,
}

pub async fn stash_list(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<Vec<StashInfo>>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut stashes = Vec::new();

    repo.stash_foreach(|index, message, _oid| {
        stashes.push(StashInfo {
            index,
            message: message.to_string(),
        });
        true
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(stashes))
}

/// Apply stash
#[derive(Debug, Deserialize)]
pub struct StashApplyRequest {
    pub index: usize,
}

pub async fn stash_apply(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<StashApplyRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    repo.stash_apply(payload.index, None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json("Stash applied successfully".to_string()))
}

/// Drop stash
pub async fn stash_drop(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<StashApplyRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    repo.stash_drop(payload.index)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json("Stash dropped successfully".to_string()))
}

/// Get file history
#[derive(Debug, Serialize)]
pub struct FileHistoryCommit {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub date: i64,
}

#[derive(Debug, Deserialize)]
pub struct FileHistoryQuery {
    pub session_id: String,
    pub file_path: String,
}

pub async fn get_file_history(
    Query(query): Query<FileHistoryQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<Vec<FileHistoryCommit>>, StatusCode> {
    let session = state
        .session_store
        .get_session(&query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut revwalk = repo.revwalk()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    revwalk.push_head()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut history = Vec::new();

    for oid in revwalk {
        let oid = oid.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let commit = repo.find_commit(oid)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Check if this commit affects the file
        let tree = commit.tree().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if tree.get_path(std::path::Path::new(&query.file_path)).is_ok() {
            history.push(FileHistoryCommit {
                sha: format!("{:.7}", oid),
                message: commit.message().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("Unknown").to_string(),
                date: commit.time().seconds(),
            });
        }

        if history.len() >= 50 {
            break;
        }
    }

    Ok(Json(history))
}

/// Get tags
#[derive(Debug, Serialize)]
pub struct TagInfo {
    pub name: String,
    pub message: Option<String>,
}

pub async fn get_tags(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<Vec<TagInfo>>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tag_names = repo.tag_names(None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut tags = Vec::new();

    for tag_name in tag_names.iter() {
        if let Some(name) = tag_name {
            tags.push(TagInfo {
                name: name.to_string(),
                message: None,
            });
        }
    }

    Ok(Json(tags))
}

/// Create tag
#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub tag_name: String,
    pub message: Option<String>,
}

pub async fn create_tag(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<CreateTagRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let head = repo.head()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let target = head.peel_to_commit()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let signature = repo.signature()
        .or_else(|_| git2::Signature::now("BerryCode", "berrycode@example.com"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(msg) = &payload.message {
        repo.tag(&payload.tag_name, target.as_object(), &signature, msg, false)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    } else {
        repo.tag_lightweight(&payload.tag_name, target.as_object(), false)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(Json(format!("Tag '{}' created successfully", payload.tag_name)))
}

/// Delete tag
#[derive(Debug, Deserialize)]
pub struct DeleteTagRequest {
    pub tag_name: String,
}

pub async fn delete_tag(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<DeleteTagRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    repo.tag_delete(&payload.tag_name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(format!("Tag '{}' deleted successfully", payload.tag_name)))
}

/// Commit graph entry with parent and branch information
#[derive(Debug, Serialize)]
pub struct CommitGraphEntry {
    pub sha: String,
    pub short_sha: String,
    pub message: String,
    pub author: String,
    pub email: String,
    pub date: i64,
    pub parent_shas: Vec<String>,
    pub branches: Vec<String>,
    pub tags: Vec<String>,
}

/// Get commit graph for visualization
pub async fn get_commit_graph(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<Vec<CommitGraphEntry>>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build a map of commit SHA -> branches
    let mut commit_branches: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    // Get all local branches
    let branches = repo.branches(Some(git2::BranchType::Local))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for branch in branches {
        if let Ok((branch, _)) = branch {
            if let Some(target) = branch.get().target() {
                let sha = target.to_string();
                let name = branch.name().ok().flatten().unwrap_or("").to_string();
                commit_branches.entry(sha).or_insert_with(Vec::new).push(name);
            }
        }
    }

    // Build a map of commit SHA -> tags
    let mut commit_tags: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    let tag_names = repo.tag_names(None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for tag_name in tag_names.iter().flatten() {
        if let Ok(obj) = repo.revparse_single(tag_name) {
            let sha = obj.id().to_string();
            commit_tags.entry(sha).or_insert_with(Vec::new).push(tag_name.to_string());
        }
    }

    // Walk commits from all branches
    let mut revwalk = repo.revwalk()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Push all branch heads
    let branches = repo.branches(Some(git2::BranchType::Local))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for branch in branches {
        if let Ok((branch, _)) = branch {
            if let Some(target) = branch.get().target() {
                let _ = revwalk.push(target);
            }
        }
    }

    let mut graph = Vec::new();

    for oid in revwalk.take(100) {  // Limit to 100 commits
        if let Ok(oid) = oid {
            if let Ok(commit) = repo.find_commit(oid) {
                let sha = commit.id().to_string();
                let parent_shas: Vec<String> = commit.parents()
                    .map(|p| p.id().to_string())
                    .collect();

                graph.push(CommitGraphEntry {
                    sha: sha.clone(),
                    short_sha: sha[..8].to_string(),
                    message: commit.message().unwrap_or("").to_string(),
                    author: commit.author().name().unwrap_or("Unknown").to_string(),
                    email: commit.author().email().unwrap_or("").to_string(),
                    date: commit.time().seconds(),
                    parent_shas,
                    branches: commit_branches.get(&sha).cloned().unwrap_or_default(),
                    tags: commit_tags.get(&sha).cloned().unwrap_or_default(),
                });
            }
        }
    }

    Ok(Json(graph))
}

/// Rebase request
#[derive(Debug, Deserialize)]
pub struct RebaseRequest {
    pub branch: String,
    pub onto: String,
}

/// Rebase branch
pub async fn git_rebase(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<RebaseRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Find the branch to rebase
    let branch_ref = repo.find_reference(&format!("refs/heads/{}", payload.branch))
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let branch_commit = branch_ref.peel_to_commit()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let branch_annotated = repo.find_annotated_commit(branch_commit.id())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Find the target to rebase onto
    let onto_ref = repo.find_reference(&format!("refs/heads/{}", payload.onto))
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let onto_commit = onto_ref.peel_to_commit()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let onto_annotated = repo.find_annotated_commit(onto_commit.id())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get signature
    let signature = repo.signature()
        .or_else(|_| git2::Signature::now("BerryCode", "berrycode@example.com"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Perform rebase
    let mut rebase = repo.rebase(
        Some(&branch_annotated),
        Some(&onto_annotated),
        None,
        None,
    ).map_err(|e| {
        tracing::error!("Rebase failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Process each rebase operation
    while let Some(op) = rebase.next() {
        if op.is_ok() {
            rebase.commit(None, &signature, None)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        } else {
            // Abort on error
            rebase.abort()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            return Err(StatusCode::CONFLICT);
        }
    }

    // Finish the rebase
    rebase.finish(Some(&signature))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(format!("Successfully rebased {} onto {}", payload.branch, payload.onto)))
}

/// Cherry-pick request
#[derive(Debug, Deserialize)]
pub struct CherryPickRequest {
    pub commit_sha: String,
}

/// Cherry-pick a commit
pub async fn git_cherry_pick(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<CherryPickRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Find the commit to cherry-pick
    let oid = git2::Oid::from_str(&payload.commit_sha)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let commit = repo.find_commit(oid)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Perform cherry-pick
    repo.cherrypick(&commit, None)
        .map_err(|e| {
            tracing::error!("Cherry-pick failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Check for conflicts
    let mut index = repo.index()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if index.has_conflicts() {
        return Ok(Json("Cherry-pick resulted in conflicts. Please resolve them manually.".to_string()));
    }

    // Commit the cherry-pick
    let signature = repo.signature()
        .or_else(|_| git2::Signature::now("BerryCode", "berrycode@example.com"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tree_id = index.write_tree()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tree = repo.find_tree(tree_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let head = repo.head()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let parent = head.peel_to_commit()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let message = format!("Cherry-pick: {}", commit.message().unwrap_or(""));

    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        &message,
        &tree,
        &[&parent],
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Cleanup cherry-pick state
    repo.cleanup_state()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(format!("Successfully cherry-picked commit {}", &payload.commit_sha[..8])))
}

/// File content at a specific commit
#[derive(Debug, Deserialize, Serialize)]
pub struct FileAtCommitRequest {
    pub session_id: String,
    pub file_path: String,
    pub commit: String,
}

#[derive(Debug, Serialize)]
pub struct FileAtCommitResponse {
    pub content: String,
    pub file_path: String,
    pub commit: String,
}

/// Get file content at a specific commit
pub async fn get_file_at_commit(
    Query(query): Query<FileAtCommitRequest>,
    State(state): State<GitApiState>,
) -> Result<Json<FileAtCommitResponse>, StatusCode> {
    let session = state
        .session_store
        .get_session(&query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get the commit
    let commit = match query.commit.as_str() {
        "HEAD" => repo.head()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .peel_to_commit()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        commit_sha => {
            let oid = git2::Oid::from_str(commit_sha)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            repo.find_commit(oid)
                .map_err(|_| StatusCode::NOT_FOUND)?
        }
    };

    // Get the tree
    let tree = commit.tree()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Find the file in the tree
    let entry = tree.get_path(std::path::Path::new(&query.file_path))
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Get file content
    let blob = entry.to_object(&repo)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_blob()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let content = std::str::from_utf8(blob.content())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    Ok(Json(FileAtCommitResponse {
        content,
        file_path: query.file_path,
        commit: commit.id().to_string(),
    }))
}

/// Detailed diff for a file
#[derive(Debug, Serialize)]
pub struct DetailedDiff {
    pub file_path: String,
    pub old_path: Option<String>,
    pub status: String, // "modified", "added", "deleted", "renamed"
    pub hunks: Vec<DiffHunk>,
    pub stats: DiffStats,
}

#[derive(Debug, Serialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Serialize)]
pub struct DiffLine {
    pub line_type: String, // "context", "addition", "deletion"
    pub old_line_no: Option<u32>,
    pub new_line_no: Option<u32>,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
}

/// Query for detailed diff
#[derive(Debug, Deserialize)]
pub struct DetailedDiffQuery {
    pub session_id: String,
    pub file_path: Option<String>,
    pub commit_sha: Option<String>,
}

/// Get detailed diff (simplified version)
pub async fn get_detailed_diff(
    Query(query): Query<DetailedDiffQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<Vec<DetailedDiff>>, StatusCode> {
    let session = state
        .session_store
        .get_session(&query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let _repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Simplified implementation - return placeholder
    // Full implementation would require more complex borrow management
    Ok(Json(Vec::new()))
}

/// Git Reset request
#[derive(Debug, Deserialize)]
pub struct GitResetRequest {
    pub commit_sha: String,
    pub reset_type: String, // "soft", "mixed", "hard"
}

/// Git Reset
pub async fn git_reset(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<GitResetRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let oid = git2::Oid::from_str(&payload.commit_sha)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let commit = repo.find_commit(oid)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let reset_type = match payload.reset_type.as_str() {
        "soft" => git2::ResetType::Soft,
        "mixed" => git2::ResetType::Mixed,
        "hard" => git2::ResetType::Hard,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    repo.reset(commit.as_object(), reset_type, None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(format!("Reset to {} ({})", &payload.commit_sha[..8], payload.reset_type)))
}

/// Blame information
#[derive(Debug, Serialize)]
pub struct BlameInfo {
    pub lines: Vec<BlameLine>,
}

#[derive(Debug, Serialize)]
pub struct BlameLine {
    pub line_number: usize,
    pub commit_sha: String,
    pub author: String,
    pub author_email: String,
    pub date: i64,
    pub content: String,
}

/// Query for blame
#[derive(Debug, Deserialize)]
pub struct BlameQuery {
    pub session_id: String,
    pub file_path: String,
}

/// Get blame for a file
pub async fn get_blame(
    Query(query): Query<BlameQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<BlameInfo>, StatusCode> {
    let session = state
        .session_store
        .get_session(&query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let blame = repo.blame_file(std::path::Path::new(&query.file_path), None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Read file content
    let file_path = std::path::Path::new(&session.project_root).join(&query.file_path);
    let content = std::fs::read_to_string(&file_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut lines = Vec::new();
    for (idx, line_content) in content.lines().enumerate() {
        let line_num = idx + 1;

        let hunk = blame.get_line(line_num)
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        let commit = repo.find_commit(hunk.final_commit_id())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        lines.push(BlameLine {
            line_number: line_num,
            commit_sha: hunk.final_commit_id().to_string(),
            author: commit.author().name().unwrap_or("Unknown").to_string(),
            author_email: commit.author().email().unwrap_or("").to_string(),
            date: commit.time().seconds(),
            content: line_content.to_string(),
        });
    }

    Ok(Json(BlameInfo { lines }))
}

/// Branch comparison
#[derive(Debug, Serialize)]
pub struct BranchComparison {
    pub ahead: Vec<CommitGraphEntry>,
    pub behind: Vec<CommitGraphEntry>,
    pub stats: ComparisonStats,
}

#[derive(Debug, Serialize)]
pub struct ComparisonStats {
    pub ahead_count: usize,
    pub behind_count: usize,
}

/// Query for branch comparison
#[derive(Debug, Deserialize)]
pub struct BranchComparisonQuery {
    pub session_id: String,
    pub base_branch: String,
    pub compare_branch: String,
}

/// Compare two branches
pub async fn compare_branches(
    Query(query): Query<BranchComparisonQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<BranchComparison>, StatusCode> {
    let session = state
        .session_store
        .get_session(&query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get both branches
    let base_ref = repo.find_reference(&format!("refs/heads/{}", query.base_branch))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let compare_ref = repo.find_reference(&format!("refs/heads/{}", query.compare_branch))
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let base_oid = base_ref.target().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let compare_oid = compare_ref.target().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get ahead commits (in compare but not in base)
    let mut ahead_revwalk = repo.revwalk()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    ahead_revwalk.push(compare_oid)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    ahead_revwalk.hide(base_oid)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut ahead = Vec::new();
    for oid in ahead_revwalk {
        if let Ok(oid) = oid {
            if let Ok(commit) = repo.find_commit(oid) {
                let sha = commit.id().to_string();
                ahead.push(CommitGraphEntry {
                    sha: sha.clone(),
                    short_sha: sha[..8].to_string(),
                    message: commit.message().unwrap_or("").to_string(),
                    author: commit.author().name().unwrap_or("Unknown").to_string(),
                    email: commit.author().email().unwrap_or("").to_string(),
                    date: commit.time().seconds(),
                    parent_shas: commit.parents().map(|p| p.id().to_string()).collect(),
                    branches: Vec::new(),
                    tags: Vec::new(),
                });
            }
        }
    }

    // Get behind commits (in base but not in compare)
    let mut behind_revwalk = repo.revwalk()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    behind_revwalk.push(base_oid)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    behind_revwalk.hide(compare_oid)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut behind = Vec::new();
    for oid in behind_revwalk {
        if let Ok(oid) = oid {
            if let Ok(commit) = repo.find_commit(oid) {
                let sha = commit.id().to_string();
                behind.push(CommitGraphEntry {
                    sha: sha.clone(),
                    short_sha: sha[..8].to_string(),
                    message: commit.message().unwrap_or("").to_string(),
                    author: commit.author().name().unwrap_or("Unknown").to_string(),
                    email: commit.author().email().unwrap_or("").to_string(),
                    date: commit.time().seconds(),
                    parent_shas: commit.parents().map(|p| p.id().to_string()).collect(),
                    branches: Vec::new(),
                    tags: Vec::new(),
                });
            }
        }
    }

    Ok(Json(BranchComparison {
        stats: ComparisonStats {
            ahead_count: ahead.len(),
            behind_count: behind.len(),
        },
        ahead,
        behind,
    }))
}

/// Reflog entry
#[derive(Debug, Serialize)]
pub struct ReflogEntry {
    pub index: usize,
    pub old_oid: String,
    pub new_oid: String,
    pub committer: String,
    pub message: String,
}

/// Get reflog
pub async fn get_reflog(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<Vec<ReflogEntry>>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let reflog = repo.reflog("HEAD")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let entries: Vec<ReflogEntry> = reflog.iter().enumerate().map(|(idx, entry)| {
        ReflogEntry {
            index: idx,
            old_oid: entry.id_old().to_string(),
            new_oid: entry.id_new().to_string(),
            committer: entry.committer().name().unwrap_or("Unknown").to_string(),
            message: entry.message().unwrap_or("").to_string(),
        }
    }).collect();

    Ok(Json(entries))
}

/// Remote info
#[derive(Debug, Serialize)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub fetch_url: Option<String>,
    pub push_url: Option<String>,
}

/// Get all remotes
pub async fn get_remotes(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<Vec<RemoteInfo>>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut remotes = Vec::new();
    for name in repo.remotes().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.iter() {
        if let Some(name) = name {
            if let Ok(remote) = repo.find_remote(name) {
                remotes.push(RemoteInfo {
                    name: name.to_string(),
                    url: remote.url().unwrap_or("").to_string(),
                    fetch_url: remote.url().map(String::from),
                    push_url: remote.pushurl().map(String::from),
                });
            }
        }
    }

    Ok(Json(remotes))
}

/// Add remote request
#[derive(Debug, Deserialize)]
pub struct AddRemoteRequest {
    pub name: String,
    pub url: String,
}

/// Add a remote
pub async fn add_remote(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<AddRemoteRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    repo.remote(&payload.name, &payload.url)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(format!("Remote '{}' added successfully", payload.name)))
}

/// Remove remote request
#[derive(Debug, Deserialize)]
pub struct RemoveRemoteRequest {
    pub name: String,
}

/// Remove a remote
pub async fn remove_remote(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<RemoveRemoteRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    repo.remote_delete(&payload.name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(format!("Remote '{}' removed successfully", payload.name)))
}

/// Submodule info
#[derive(Debug, Serialize)]
pub struct SubmoduleInfo {
    pub name: String,
    pub path: String,
    pub url: String,
}

/// Get all submodules
pub async fn get_submodules(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<Vec<SubmoduleInfo>>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut submodules = Vec::new();

    repo.submodules()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .iter()
        .for_each(|sub| {
            if let (Some(name), Some(path), Some(url)) = (sub.name(), sub.path().to_str(), sub.url()) {
                submodules.push(SubmoduleInfo {
                    name: name.to_string(),
                    path: path.to_string(),
                    url: url.to_string(),
                });
            }
        });

    Ok(Json(submodules))
}

/// Add submodule request
#[derive(Debug, Deserialize)]
pub struct AddSubmoduleRequest {
    pub url: String,
    pub path: String,
}

/// Add a submodule
pub async fn add_submodule(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<AddSubmoduleRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    repo.submodule(&payload.url, std::path::Path::new(&payload.path), false)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(format!("Submodule added at {}", payload.path)))
}

/// Search commits query
#[derive(Debug, Deserialize)]
pub struct SearchCommitsQuery {
    pub session_id: String,
    pub query: String,
    pub search_type: String, // "message", "author", "sha"
}

/// Search commits
pub async fn search_commits(
    Query(query): Query<SearchCommitsQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<Vec<CommitGraphEntry>>, StatusCode> {
    let session = state
        .session_store
        .get_session(&query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut revwalk = repo.revwalk()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    revwalk.push_head()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut results = Vec::new();
    let search_query = query.query.to_lowercase();

    for oid in revwalk {
        if let Ok(oid) = oid {
            if let Ok(commit) = repo.find_commit(oid) {
                let matches = match query.search_type.as_str() {
                    "message" => commit.message().unwrap_or("").to_lowercase().contains(&search_query),
                    "author" => commit.author().name().unwrap_or("").to_lowercase().contains(&search_query),
                    "sha" => commit.id().to_string().contains(&search_query),
                    _ => false,
                };

                if matches {
                    let sha = commit.id().to_string();
                    results.push(CommitGraphEntry {
                        sha: sha.clone(),
                        short_sha: sha[..8].to_string(),
                        message: commit.message().unwrap_or("").to_string(),
                        author: commit.author().name().unwrap_or("Unknown").to_string(),
                        email: commit.author().email().unwrap_or("").to_string(),
                        date: commit.time().seconds(),
                        parent_shas: commit.parents().map(|p| p.id().to_string()).collect(),
                        branches: Vec::new(),
                        tags: Vec::new(),
                    });

                    if results.len() >= 50 {
                        break;
                    }
                }
            }
        }
    }

    Ok(Json(results))
}

/// Bisect state
#[derive(Debug, Serialize)]
pub struct BisectInfo {
    pub is_active: bool,
    pub current_commit: Option<String>,
    pub remaining_commits: usize,
}

/// Get bisect status
pub async fn get_bisect_status(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<BisectInfo>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo_path = std::path::Path::new(&session.project_root);
    let bisect_active = repo_path.join(".git/BISECT_START").exists();

    Ok(Json(BisectInfo {
        is_active: bisect_active,
        current_commit: None,
        remaining_commits: 0,
    }))
}

/// Create patch
#[derive(Debug, Deserialize)]
pub struct CreatePatchQuery {
    pub session_id: String,
    pub commit_sha: String,
}

/// Create patch from commit
pub async fn create_patch(
    Query(query): Query<CreatePatchQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let oid = git2::Oid::from_str(&query.commit_sha)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let commit = repo.find_commit(oid)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Get diff for this commit
    let tree = commit.tree()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let parent_tree = if commit.parent_count() > 0 {
        Some(commit.parent(0)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .tree()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
    } else {
        None
    };

    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Format as patch
    let mut patch = Vec::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        patch.extend_from_slice(line.content());
        true
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let patch_str = String::from_utf8_lossy(&patch).to_string();

    Ok(Json(patch_str))
}

/// Apply patch request
#[derive(Debug, Deserialize)]
pub struct ApplyPatchRequest {
    pub patch: String,
    #[serde(default = "default_apply_mode")]
    pub mode: String, // "apply", "am", or "check"
}

fn default_apply_mode() -> String {
    "apply".to_string()
}

#[derive(Debug, Serialize)]
pub struct ApplyPatchResponse {
    pub message: String,
    pub success: bool,
}

/// Apply a patch
pub async fn apply_patch(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<ApplyPatchRequest>,
) -> Result<Json<ApplyPatchResponse>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    // Write patch to temporary file
    let patch_path = std::path::Path::new(&session.project_root).join(".git/tmp_patch");
    std::fs::write(&patch_path, &payload.patch)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build command based on mode
    let mut cmd = std::process::Command::new("git");

    match payload.mode.as_str() {
        "am" => {
            // git am - apply as commit with author info
            cmd.args(&["am", patch_path.to_str().unwrap_or("")]);
        }
        "check" => {
            // git apply --check - check if patch can be applied
            cmd.args(&["apply", "--check", patch_path.to_str().unwrap_or("")]);
        }
        _ => {
            // git apply - default mode
            cmd.args(&["apply", patch_path.to_str().unwrap_or("")]);
        }
    }

    let output = cmd
        .current_dir(&session.project_root)
        .output()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Clean up
    let _ = std::fs::remove_file(&patch_path);

    let success = output.status.success();
    let message = if success {
        match payload.mode.as_str() {
            "check" => "パッチは適用可能です".to_string(),
            "am" => "パッチをコミットとして適用しました".to_string(),
            _ => "パッチを適用しました".to_string(),
        }
    } else {
        format!("パッチ適用失敗: {}", String::from_utf8_lossy(&output.stderr))
    };

    Ok(Json(ApplyPatchResponse { message, success }))
}

/// Worktree info
#[derive(Debug, Serialize)]
pub struct WorktreeInfo {
    pub path: String,
    pub is_main: bool,
    pub branch: Option<String>,
}

/// Get all worktrees
pub async fn get_worktrees(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<Vec<WorktreeInfo>>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut worktrees = vec![WorktreeInfo {
        path: session.project_root.display().to_string(),
        is_main: true,
        branch: None,
    }];

    // Check for additional worktrees
    let worktree_dir = std::path::Path::new(&session.project_root).join(".git/worktrees");
    if worktree_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(worktree_dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    worktrees.push(WorktreeInfo {
                        path: name.clone(),
                        is_main: false,
                        branch: Some(name),
                    });
                }
            }
        }
    }

    Ok(Json(worktrees))
}

/// Gitflow init request
#[derive(Debug, Deserialize)]
pub struct GitflowInitRequest {
    pub master_branch: String,
    pub develop_branch: String,
}

/// Initialize Gitflow
pub async fn gitflow_init(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<GitflowInitRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create develop branch if it doesn't exist
    if repo.find_branch(&payload.develop_branch, git2::BranchType::Local).is_err() {
        let head = repo.head()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let commit = head.peel_to_commit()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        repo.branch(&payload.develop_branch, &commit, false)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(Json(format!("Gitflow initialized with master: {}, develop: {}",
        payload.master_branch, payload.develop_branch)))
}

/// Start Gitflow feature
#[derive(Debug, Deserialize)]
pub struct GitflowFeatureRequest {
    pub feature_name: String,
}

/// Start a Gitflow feature branch
pub async fn gitflow_feature_start(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<GitflowFeatureRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get develop branch
    let develop = repo.find_branch("develop", git2::BranchType::Local)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let develop_commit = develop.get().peel_to_commit()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create feature branch
    let branch_name = format!("feature/{}", payload.feature_name);
    repo.branch(&branch_name, &develop_commit, false)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(format!("Feature branch '{}' created", branch_name)))
}

/// Finish Gitflow feature
pub async fn gitflow_feature_finish(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<GitflowFeatureRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let branch_name = format!("feature/{}", payload.feature_name);

    // Merge into develop
    let feature_branch = repo.find_branch(&branch_name, git2::BranchType::Local)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let feature_commit = feature_branch.get().peel_to_commit()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Switch to develop
    let develop_branch = repo.find_branch("develop", git2::BranchType::Local)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    repo.set_head(develop_branch.get().name().unwrap_or("refs/heads/develop"))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Merge feature
    let annotated = repo.find_annotated_commit(feature_commit.id())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    repo.merge(&[&annotated], None, None)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Delete feature branch
    let mut feature_branch = repo.find_branch(&branch_name, git2::BranchType::Local)
        .map_err(|_| StatusCode::NOT_FOUND)?;
    feature_branch.delete()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(format!("Feature '{}' merged and deleted", payload.feature_name)))
}

// ========== Git Bisect Functions ==========

/// Bisect start request
#[derive(Debug, Deserialize)]
pub struct BisectStartRequest {
    pub bad: String,
    pub good: String,
}

/// Bisect response
#[derive(Debug, Serialize)]
pub struct BisectResponse {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<u32>,
    #[serde(default)]
    pub found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// Start git bisect
pub async fn bisect_start(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<BisectStartRequest>,
) -> Result<Json<BisectResponse>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let output = std::process::Command::new("git")
        .args(&["bisect", "start", &payload.bad, &payload.good])
        .current_dir(&session.project_root)
        .output()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !output.status.success() {
        return Ok(Json(BisectResponse {
            message: format!("Bisect開始失敗: {}", String::from_utf8_lossy(&output.stderr)),
            current: None,
            steps: None,
            found: false,
            commit: None,
            author: None,
            date: None,
        }));
    }

    // Get current commit info
    let current_output = std::process::Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .current_dir(&session.project_root)
        .output()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let current_commit = String::from_utf8_lossy(&current_output.stdout).trim().to_string();

    // Parse output for steps remaining
    let output_str = String::from_utf8_lossy(&output.stdout);
    let steps = parse_bisect_steps(&output_str);

    Ok(Json(BisectResponse {
        message: "Bisectを開始しました".to_string(),
        current: Some(current_commit),
        steps,
        found: false,
        commit: None,
        author: None,
        date: None,
    }))
}

/// Mark current commit as bad
pub async fn bisect_bad(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<BisectResponse>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let output = std::process::Command::new("git")
        .args(&["bisect", "bad"])
        .current_dir(&session.project_root)
        .output()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    parse_bisect_output(output, &session.project_root)
}

/// Mark current commit as good
pub async fn bisect_good(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<BisectResponse>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let output = std::process::Command::new("git")
        .args(&["bisect", "good"])
        .current_dir(&session.project_root)
        .output()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    parse_bisect_output(output, &session.project_root)
}

/// Skip current commit
pub async fn bisect_skip(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<BisectResponse>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let output = std::process::Command::new("git")
        .args(&["bisect", "skip"])
        .current_dir(&session.project_root)
        .output()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    parse_bisect_output(output, &session.project_root)
}

/// Reset bisect
pub async fn bisect_reset(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<BisectResponse>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let output = std::process::Command::new("git")
        .args(&["bisect", "reset"])
        .current_dir(&session.project_root)
        .output()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if output.status.success() {
        Ok(Json(BisectResponse {
            message: "Bisectを終了しました".to_string(),
            current: None,
            steps: None,
            found: false,
            commit: None,
            author: None,
            date: None,
        }))
    } else {
        Ok(Json(BisectResponse {
            message: format!("Bisectリセット失敗: {}", String::from_utf8_lossy(&output.stderr)),
            current: None,
            steps: None,
            found: false,
            commit: None,
            author: None,
            date: None,
        }))
    }
}

/// Get bisect log
pub async fn bisect_log(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let output = std::process::Command::new("git")
        .args(&["bisect", "log"])
        .current_dir(&session.project_root)
        .output()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let log = String::from_utf8_lossy(&output.stdout).to_string();

    Ok(Json(serde_json::json!({
        "log": log
    })))
}

// Helper function to parse bisect output
fn parse_bisect_output(
    output: std::process::Output,
    project_root: &std::path::Path,
) -> Result<Json<BisectResponse>, StatusCode> {
    let output_str = String::from_utf8_lossy(&output.stdout);

    // Check if bisect found the first bad commit
    if output_str.contains("is the first bad commit") {
        // Extract commit info
        let lines: Vec<&str> = output_str.lines().collect();
        let commit = lines.get(0)
            .and_then(|line| line.split_whitespace().next())
            .map(|s| s.to_string());

        // Get commit details
        if let Some(commit_sha) = &commit {
            let show_output = std::process::Command::new("git")
                .args(&["show", "-s", "--format=%an%n%ad", commit_sha])
                .current_dir(project_root)
                .output()
                .ok();

            if let Some(show) = show_output {
                let show_str = String::from_utf8_lossy(&show.stdout);
                let mut show_lines = show_str.lines();
                let author = show_lines.next().map(|s| s.to_string());
                let date = show_lines.next().map(|s| s.to_string());

                return Ok(Json(BisectResponse {
                    message: "バグを導入したコミットを特定しました！".to_string(),
                    current: None,
                    steps: None,
                    found: true,
                    commit,
                    author,
                    date,
                }));
            }
        }

        return Ok(Json(BisectResponse {
            message: "バグを導入したコミットを特定しました！".to_string(),
            current: None,
            steps: None,
            found: true,
            commit,
            author: None,
            date: None,
        }));
    }

    // Get current commit
    let current_output = std::process::Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .current_dir(project_root)
        .output()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let current_commit = String::from_utf8_lossy(&current_output.stdout).trim().to_string();

    // Parse steps remaining
    let steps = parse_bisect_steps(&output_str);

    Ok(Json(BisectResponse {
        message: "次のコミットをテストしてください".to_string(),
        current: Some(current_commit),
        steps,
        found: false,
        commit: None,
        author: None,
        date: None,
    }))
}

// Helper function to parse steps remaining from git bisect output
fn parse_bisect_steps(output: &str) -> Option<u32> {
    // Look for pattern like "Bisecting: 5 revisions left to test"
    for line in output.lines() {
        if line.contains("revisions left") || line.contains("revision left") {
            if let Some(num_str) = line.split_whitespace().nth(1) {
                if let Ok(num) = num_str.parse::<u32>() {
                    return Some(num);
                }
            }
        }
    }
    None
}

// ========== Merge Conflict Resolution Functions ==========

/// Conflict file info
#[derive(Debug, Serialize)]
pub struct ConflictFile {
    pub path: String,
    pub content: String,
    pub has_conflicts: bool,
}

/// Get files with merge conflicts
pub async fn get_conflicted_files(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<Vec<ConflictFile>>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let index = repo.index()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut conflicted_files = Vec::new();

    if index.has_conflicts() {
        let conflicts = index.conflicts()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        for conflict in conflicts {
            if let Ok(conflict) = conflict {
                if let Some(our) = conflict.our {
                    let path = String::from_utf8_lossy(&our.path).to_string();

                    // Read file content
                    let file_path = std::path::Path::new(&session.project_root).join(&path);
                    let content = std::fs::read_to_string(&file_path)
                        .unwrap_or_else(|_| String::from("Unable to read file"));

                    conflicted_files.push(ConflictFile {
                        path,
                        content,
                        has_conflicts: true,
                    });
                }
            }
        }
    }

    Ok(Json(conflicted_files))
}

/// Conflict resolution request
#[derive(Debug, Deserialize)]
pub struct ResolveConflictRequest {
    pub file_path: String,
    pub resolved_content: String,
}

/// Resolve a conflict by accepting resolved content
pub async fn resolve_conflict(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<ResolveConflictRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    // Write resolved content to file
    let file_path = std::path::Path::new(&session.project_root).join(&payload.file_path);
    std::fs::write(&file_path, &payload.resolved_content)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Stage the resolved file
    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut index = repo.index()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    index.add_path(std::path::Path::new(&payload.file_path))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    index.write()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(format!("Conflict resolved for {}", payload.file_path)))
}

/// Accept current version (ours) for conflict resolution
#[derive(Debug, Deserialize)]
pub struct AcceptVersionRequest {
    pub file_path: String,
    pub version: String, // "current", "incoming", "both"
}

/// Accept a specific version to resolve conflict
pub async fn accept_conflict_version(
    Query(session_query): Query<crate::berrycode::web::api::files::file_api::ListFilesQuery>,
    State(state): State<GitApiState>,
    Json(payload): Json<AcceptVersionRequest>,
) -> Result<Json<String>, StatusCode> {
    let session = state
        .session_store
        .get_session(&session_query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let file_path = std::path::Path::new(&session.project_root).join(&payload.file_path);
    let content = std::fs::read_to_string(&file_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Parse conflict markers and extract content
    let resolved_content = match payload.version.as_str() {
        "current" => resolve_to_current(&content),
        "incoming" => resolve_to_incoming(&content),
        "both" => resolve_to_both(&content),
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // Write resolved content
    std::fs::write(&file_path, &resolved_content)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Stage the file
    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut index = repo.index()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    index.add_path(std::path::Path::new(&payload.file_path))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    index.write()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(format!("Accepted {} version for {}", payload.version, payload.file_path)))
}

// Helper functions for conflict resolution
fn resolve_to_current(content: &str) -> String {
    let mut result = String::new();
    let mut in_conflict = false;
    let mut skip_until_end = false;

    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            in_conflict = true;
            skip_until_end = false;
        } else if line.starts_with("=======") && in_conflict {
            skip_until_end = true;
        } else if line.starts_with(">>>>>>>") && in_conflict {
            in_conflict = false;
            skip_until_end = false;
        } else if !in_conflict || (!skip_until_end && in_conflict) {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

fn resolve_to_incoming(content: &str) -> String {
    let mut result = String::new();
    let mut in_conflict = false;
    let mut accept_lines = false;

    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            in_conflict = true;
            accept_lines = false;
        } else if line.starts_with("=======") && in_conflict {
            accept_lines = true;
        } else if line.starts_with(">>>>>>>") && in_conflict {
            in_conflict = false;
            accept_lines = false;
        } else if !in_conflict || (accept_lines && in_conflict) {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

fn resolve_to_both(content: &str) -> String {
    let mut result = String::new();

    for line in content.lines() {
        if !line.starts_with("<<<<<<<")
            && !line.starts_with("=======")
            && !line.starts_with(">>>>>>>") {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

/// Get 3-way merge content for a conflicted file
#[derive(Debug, Serialize)]
pub struct ThreeWayMerge {
    pub file_path: String,
    pub current: String,
    pub incoming: String,
    pub base: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ThreeWayMergeQuery {
    pub session_id: String,
    pub file_path: String,
}

/// Get 3-way merge content
pub async fn get_three_way_merge(
    Query(query): Query<ThreeWayMergeQuery>,
    State(state): State<GitApiState>,
) -> Result<Json<ThreeWayMerge>, StatusCode> {
    let session = state
        .session_store
        .get_session(&query.session_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let file_path = std::path::Path::new(&session.project_root).join(&query.file_path);
    let content = std::fs::read_to_string(&file_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Extract current and incoming versions from conflict markers
    let (current, incoming) = extract_conflict_versions(&content);

    // Try to get base version (merge base)
    let repo = Repository::open(&session.project_root)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let base = get_merge_base_content(&repo, &query.file_path).ok();

    Ok(Json(ThreeWayMerge {
        file_path: query.file_path.clone(),
        current,
        incoming,
        base,
    }))
}

fn extract_conflict_versions(content: &str) -> (String, String) {
    let mut current = String::new();
    let mut incoming = String::new();
    let mut in_current = false;
    let mut in_incoming = false;

    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            in_current = true;
        } else if line.starts_with("=======") {
            in_current = false;
            in_incoming = true;
        } else if line.starts_with(">>>>>>>") {
            in_incoming = false;
        } else if in_current {
            current.push_str(line);
            current.push('\n');
        } else if in_incoming {
            incoming.push_str(line);
            incoming.push('\n');
        }
    }

    (current, incoming)
}

fn get_merge_base_content(repo: &Repository, file_path: &str) -> Result<String, git2::Error> {
    // Get HEAD and MERGE_HEAD
    let head = repo.head()?.peel_to_commit()?;

    // Try to read MERGE_HEAD
    let merge_head_path = repo.path().join("MERGE_HEAD");
    if !merge_head_path.exists() {
        return Err(git2::Error::from_str("No merge in progress"));
    }

    let merge_head_oid = std::fs::read_to_string(merge_head_path)
        .map_err(|_| git2::Error::from_str("Failed to read MERGE_HEAD"))?;
    let merge_head_oid = merge_head_oid.trim();
    let merge_commit = repo.find_commit(git2::Oid::from_str(merge_head_oid)?)?;

    // Find merge base
    let merge_base_oid = repo.merge_base(head.id(), merge_commit.id())?;
    let base_commit = repo.find_commit(merge_base_oid)?;

    // Get file content at merge base
    let tree = base_commit.tree()?;
    let entry = tree.get_path(std::path::Path::new(file_path))?;
    let blob = entry.to_object(repo)?.into_blob()
        .map_err(|_| git2::Error::from_str("Not a blob"))?;

    Ok(std::str::from_utf8(blob.content())
        .map_err(|_| git2::Error::from_str("Invalid UTF-8"))?
        .to_string())
}
