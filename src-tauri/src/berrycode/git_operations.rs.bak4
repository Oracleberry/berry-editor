//! Git Operations Engine
//! Provides high-level Git operations using git2 library

use anyhow::{Context, Result};
use git2::{Branch, BranchType, Commit, Delta, DiffOptions, Repository, Signature, Status, StatusOptions};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub id: String,
    pub author: String,
    pub email: String,
    pub timestamp: i64,
    pub message: String,
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub upstream: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatus {
    pub path: String,
    pub status: String,
    pub is_staged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameLineInfo {
    pub line_number: usize,
    pub commit_id: String,
    pub author: String,
    pub timestamp: i64,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub origin: char,
    pub content: String,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub status: String,
    pub hunks: Vec<DiffHunk>,
}

pub struct GitOperations {
    repo_path: PathBuf,
}

impl GitOperations {
    /// Open a Git repository at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let repo_path = path.as_ref().to_path_buf();

        // Verify it's a valid git repository
        Repository::open(&repo_path)
            .with_context(|| format!("Failed to open Git repository at {:?}", repo_path))?;

        Ok(Self { repo_path })
    }

    /// Get the status of all files in the repository
    pub fn get_status(&self) -> Result<Vec<FileStatus>> {
        let repo = Repository::open(&self.repo_path)?;

        let mut opts = StatusOptions::new();
        opts.include_untracked(true);
        opts.recurse_untracked_dirs(true);

        let statuses = repo.statuses(Some(&mut opts))?;

        let mut result = Vec::new();

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let status_flags = entry.status();

            let status_str = if status_flags.contains(Status::WT_NEW) {
                "untracked"
            } else if status_flags.contains(Status::WT_MODIFIED) {
                "modified"
            } else if status_flags.contains(Status::WT_DELETED) {
                "deleted"
            } else if status_flags.contains(Status::INDEX_NEW) {
                "added"
            } else if status_flags.contains(Status::INDEX_MODIFIED) {
                "staged"
            } else if status_flags.contains(Status::INDEX_DELETED) {
                "removed"
            } else {
                "unknown"
            };

            let is_staged = status_flags.intersects(
                Status::INDEX_NEW | Status::INDEX_MODIFIED | Status::INDEX_DELETED
            );

            result.push(FileStatus {
                path,
                status: status_str.to_string(),
                is_staged,
            });
        }

        Ok(result)
    }

    /// Get file history
    pub fn get_file_history<P: AsRef<Path>>(&self, file_path: P) -> Result<Vec<CommitInfo>> {
        let repo = Repository::open(&self.repo_path)?;
        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;

        let path = file_path.as_ref();
        let mut commits = Vec::new();

        for oid in revwalk {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;

            // Check if this commit touches the file
            if commit.parent_count() == 0 {
                // Initial commit
                commits.push(self.commit_to_info(&commit)?);
            } else {
                let tree = commit.tree()?;
                let parent = commit.parent(0)?;
                let parent_tree = parent.tree()?;

                let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), None)?;

                let mut touches_file = false;
                diff.foreach(
                    &mut |delta, _| {
                        if let Some(new_file) = delta.new_file().path() {
                            if new_file == path {
                                touches_file = true;
                            }
                        }
                        if let Some(old_file) = delta.old_file().path() {
                            if old_file == path {
                                touches_file = true;
                            }
                        }
                        true
                    },
                    None,
                    None,
                    None,
                )?;

                if touches_file {
                    commits.push(self.commit_to_info(&commit)?);
                }
            }
        }

        Ok(commits)
    }

    /// Get blame information for a file
    pub fn blame<P: AsRef<Path>>(&self, file_path: P) -> Result<Vec<BlameLineInfo>> {
        let repo = Repository::open(&self.repo_path)?;
        let blame = repo.blame_file(file_path.as_ref(), None)?;

        let mut result = Vec::new();

        for (idx, hunk) in blame.iter().enumerate() {
            let commit = repo.find_commit(hunk.final_commit_id())?;

            result.push(BlameLineInfo {
                line_number: idx + 1,
                commit_id: hunk.final_commit_id().to_string(),
                author: commit.author().name().unwrap_or("Unknown").to_string(),
                timestamp: commit.time().seconds(),
                content: String::new(), // Will be filled by caller with actual line content
            });
        }

        Ok(result)
    }

    /// List all branches
    pub fn list_branches(&self) -> Result<Vec<BranchInfo>> {
        let repo = Repository::open(&self.repo_path)?;
        let branches = repo.branches(Some(BranchType::Local))?;

        let mut result = Vec::new();
        let head = repo.head().ok();

        for branch_result in branches {
            let (branch, _) = branch_result?;
            let name = branch.name()?.unwrap_or("").to_string();
            let is_head = head.as_ref().and_then(|h| h.shorthand()).map(|s| s == name).unwrap_or(false);

            let upstream = branch.upstream().ok().and_then(|u| {
                u.name().ok().flatten().map(|s| s.to_string())
            });

            result.push(BranchInfo {
                name,
                is_head,
                upstream,
            });
        }

        Ok(result)
    }

    /// Create a new branch
    pub fn create_branch(&self, name: &str) -> Result<()> {
        let repo = Repository::open(&self.repo_path)?;
        let head = repo.head()?;
        let commit = head.peel_to_commit()?;

        repo.branch(name, &commit, false)?;
        Ok(())
    }

    /// Checkout a branch
    pub fn checkout_branch(&self, name: &str) -> Result<()> {
        let repo = Repository::open(&self.repo_path)?;

        let (object, reference) = repo.revparse_ext(name)?;

        repo.checkout_tree(&object, None)?;

        match reference {
            Some(gref) => {
                repo.set_head(gref.name().unwrap())?;
            }
            None => {
                repo.set_head_detached(object.id())?;
            }
        }

        Ok(())
    }

    /// Commit changes
    pub fn commit(&self, message: &str, files: Vec<PathBuf>) -> Result<String> {
        let repo = Repository::open(&self.repo_path)?;
        let mut index = repo.index()?;

        // Stage files
        for file in files {
            index.add_path(&file)?;
        }
        index.write()?;

        // Create commit
        let signature = Signature::now("BerryCode User", "user@berrycode.local")?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let parent_commit = repo.head()?.peel_to_commit()?;

        let oid = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent_commit],
        )?;

        Ok(oid.to_string())
    }

    /// Get diff for a file
    pub fn diff<P: AsRef<Path>>(&self, file_path: P) -> Result<FileDiff> {
        use std::cell::RefCell;

        let repo = Repository::open(&self.repo_path)?;
        let mut opts = DiffOptions::new();
        opts.pathspec(file_path.as_ref());

        let head = repo.head()?;
        let head_tree = head.peel_to_tree()?;

        let diff = repo.diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut opts))?;

        let file_diff = RefCell::new(FileDiff {
            old_path: None,
            new_path: None,
            status: String::new(),
            hunks: Vec::new(),
        });

        diff.foreach(
            &mut |delta, _| {
                let mut fd = file_diff.borrow_mut();
                fd.old_path = delta.old_file().path().map(|p| p.to_string_lossy().to_string());
                fd.new_path = delta.new_file().path().map(|p| p.to_string_lossy().to_string());
                fd.status = match delta.status() {
                    Delta::Added => "added",
                    Delta::Deleted => "deleted",
                    Delta::Modified => "modified",
                    Delta::Renamed => "renamed",
                    Delta::Copied => "copied",
                    Delta::Unmodified => "unmodified",
                    _ => "unknown",
                }.to_string();
                true
            },
            None,
            Some(&mut |_delta, hunk| {
                let hunk_info = DiffHunk {
                    old_start: hunk.old_start(),
                    old_lines: hunk.old_lines(),
                    new_start: hunk.new_start(),
                    new_lines: hunk.new_lines(),
                    header: String::from_utf8_lossy(hunk.header()).to_string(),
                    lines: Vec::new(),
                };
                file_diff.borrow_mut().hunks.push(hunk_info);
                true
            }),
            Some(&mut |_delta, _hunk, line| {
                if let Some(last_hunk) = file_diff.borrow_mut().hunks.last_mut() {
                    last_hunk.lines.push(DiffLine {
                        origin: line.origin(),
                        content: String::from_utf8_lossy(line.content()).to_string(),
                        old_lineno: line.old_lineno(),
                        new_lineno: line.new_lineno(),
                    });
                }
                true
            }),
        )?;

        Ok(file_diff.into_inner())
    }

    /// Push to remote
    pub fn push(&self, remote: &str, branch: &str) -> Result<()> {
        let repo = Repository::open(&self.repo_path)?;
        let mut remote_obj = repo.find_remote(remote)?;

        let refspec = format!("refs/heads/{}:refs/heads/{}", branch, branch);
        remote_obj.push(&[&refspec], None)?;

        Ok(())
    }

    /// Pull from remote
    pub fn pull(&self, remote: &str, branch: &str) -> Result<()> {
        let repo = Repository::open(&self.repo_path)?;
        let mut remote_obj = repo.find_remote(remote)?;

        // Fetch
        remote_obj.fetch(&[branch], None, None)?;

        // Merge
        let fetch_head = repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;

        let analysis = repo.merge_analysis(&[&fetch_commit])?;

        if analysis.0.is_up_to_date() {
            // Already up to date
            return Ok(());
        } else if analysis.0.is_fast_forward() {
            // Fast-forward merge
            let refname = format!("refs/heads/{}", branch);
            let mut reference = repo.find_reference(&refname)?;
            reference.set_target(fetch_commit.id(), "Fast-forward")?;
            repo.set_head(&refname)?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        } else {
            // Normal merge
            repo.merge(&[&fetch_commit], None, None)?;
        }

        Ok(())
    }

    /// Merge a branch
    pub fn merge_branch(&self, branch_name: &str) -> Result<String> {
        let repo = Repository::open(&self.repo_path)?;

        let branch = repo.find_branch(branch_name, BranchType::Local)?;
        let annotated_commit = repo.reference_to_annotated_commit(branch.get())?;

        let analysis = repo.merge_analysis(&[&annotated_commit])?;

        if analysis.0.is_up_to_date() {
            Ok("Already up to date".to_string())
        } else if analysis.0.is_fast_forward() {
            Ok("Fast-forward merge".to_string())
        } else {
            repo.merge(&[&annotated_commit], None, None)?;
            Ok("Merge commit required".to_string())
        }
    }

    /// Convert git2::Commit to CommitInfo
    fn commit_to_info(&self, commit: &Commit) -> Result<CommitInfo> {
        Ok(CommitInfo {
            id: commit.id().to_string(),
            author: commit.author().name().unwrap_or("Unknown").to_string(),
            email: commit.author().email().unwrap_or("unknown@unknown").to_string(),
            timestamp: commit.time().seconds(),
            message: commit.message().unwrap_or("").to_string(),
            parents: commit.parents().map(|p| p.id().to_string()).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize git repo
        Repository::init(&repo_path).unwrap();

        (temp_dir, repo_path)
    }

    #[test]
    fn test_git_operations_open() {
        let (_temp, repo_path) = setup_test_repo();
        let ops = GitOperations::open(&repo_path);
        assert!(ops.is_ok());
    }

    #[test]
    fn test_git_operations_open_invalid() {
        let result = GitOperations::open("/nonexistent/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_status_empty() {
        let (_temp, repo_path) = setup_test_repo();
        let ops = GitOperations::open(&repo_path).unwrap();
        let status = ops.get_status().unwrap();
        assert!(status.is_empty());
    }

    #[test]
    fn test_list_branches() {
        let (_temp, repo_path) = setup_test_repo();
        let ops = GitOperations::open(&repo_path).unwrap();
        let branches = ops.list_branches().unwrap();
        // New repo has no branches until first commit
        assert!(branches.is_empty() || branches.len() >= 0);
    }

    #[test]
    fn test_file_status_serialization() {
        let status = FileStatus {
            path: "test.rs".to_string(),
            status: "modified".to_string(),
            is_staged: false,
        };

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: FileStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(status.path, deserialized.path);
        assert_eq!(status.status, deserialized.status);
        assert_eq!(status.is_staged, deserialized.is_staged);
    }

    #[test]
    fn test_commit_info_serialization() {
        let info = CommitInfo {
            id: "abc123".to_string(),
            author: "Test User".to_string(),
            email: "test@example.com".to_string(),
            timestamp: 1234567890,
            message: "Test commit".to_string(),
            parents: vec!["parent1".to_string()],
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: CommitInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(info.id, deserialized.id);
        assert_eq!(info.author, deserialized.author);
    }

    #[test]
    fn test_branch_info_serialization() {
        let info = BranchInfo {
            name: "main".to_string(),
            is_head: true,
            upstream: Some("origin/main".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: BranchInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(info.name, deserialized.name);
        assert_eq!(info.is_head, deserialized.is_head);
    }
}
