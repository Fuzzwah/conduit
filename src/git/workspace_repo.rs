//! Workspace repository management (worktrees or full checkouts).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::git::worktree::{BranchStatus, BranchStatusOptions, WorktreeError};
use crate::git::{WorkspaceMode, WorktreeManager};

/// Manager that can create/remove either worktrees or full checkouts.
#[derive(Debug, Clone)]
pub struct WorkspaceRepoManager {
    worktree: WorktreeManager,
}

impl WorkspaceRepoManager {
    /// Create a new manager without a managed directory.
    pub fn new() -> Self {
        Self {
            worktree: WorktreeManager::new(),
        }
    }

    /// Create a manager with a managed directory for workspaces.
    pub fn with_managed_dir(dir: PathBuf) -> Self {
        Self {
            worktree: WorktreeManager::with_managed_dir(dir),
        }
    }

    /// Get the managed workspace path for a repo + name.
    pub fn workspace_path(&self, repo_path: &Path, name: &str) -> PathBuf {
        self.worktree.workspace_path(repo_path, name)
    }

    /// Create a workspace (worktree or checkout).
    ///
    /// `progress` is called with each stage message and each line of git output.
    /// Pass `|_| {}` when progress reporting is not needed.
    ///
    /// The caller is responsible for ensuring the base repo's remote-tracking
    /// refs are fresh (e.g. via `crate::git::sync_remote`); this function does
    /// not call `git fetch`.
    pub fn create_workspace(
        &self,
        mode: WorkspaceMode,
        repo_path: &Path,
        branch: &str,
        name: &str,
        progress: impl Fn(&str),
    ) -> Result<PathBuf, WorktreeError> {
        // When the target branch doesn't exist yet (new workspace), branch from
        // origin/<default> rather than the current local HEAD, so the workspace
        // always starts from the latest fetched remote state even if local main
        // is behind origin.
        let branch_is_new = !self.worktree.ref_exists(repo_path, branch).unwrap_or(true);

        progress("Creating worktree...");
        if branch_is_new {
            let default_branch = self
                .worktree
                .get_main_branch(repo_path)
                .unwrap_or_else(|_| "main".to_string());
            let origin_ref = format!("origin/{}", default_branch);
            let base = if self
                .worktree
                .ref_exists(repo_path, &origin_ref)
                .unwrap_or(false)
            {
                origin_ref
            } else {
                default_branch
            };
            match mode {
                WorkspaceMode::Worktree => self
                    .worktree
                    .create_worktree_from_branch(repo_path, &base, branch, name),
                WorkspaceMode::Checkout => {
                    self.create_checkout_from_branch(repo_path, &base, branch, name)
                }
            }
        } else {
            match mode {
                WorkspaceMode::Worktree => self.worktree.create_worktree(repo_path, branch, name),
                WorkspaceMode::Checkout => self.create_checkout(repo_path, branch, name),
            }
        }
    }

    /// Create a workspace from a base branch into a new branch.
    ///
    /// `progress` is called with each stage message and each line of git output.
    /// Pass `|_| {}` when progress reporting is not needed.
    ///
    /// The caller is responsible for ensuring the base repo's remote-tracking
    /// refs are fresh (e.g. via `crate::git::sync_remote`); this function does
    /// not call `git fetch`.
    pub fn create_workspace_from_branch(
        &self,
        mode: WorkspaceMode,
        repo_path: &Path,
        base_branch: &str,
        new_branch: &str,
        name: &str,
        progress: impl Fn(&str),
    ) -> Result<PathBuf, WorktreeError> {
        // Prefer origin/<base_branch> so the new workspace starts from the latest
        // remote-tracking ref rather than a potentially stale local branch.
        let origin_ref = format!("origin/{}", base_branch);
        let start_point = if self
            .worktree
            .ref_exists(repo_path, &origin_ref)
            .unwrap_or(false)
        {
            origin_ref
        } else {
            base_branch.to_string()
        };
        progress("Creating worktree...");
        match mode {
            WorkspaceMode::Worktree => {
                self.worktree
                    .create_worktree_from_branch(repo_path, &start_point, new_branch, name)
            }
            WorkspaceMode::Checkout => {
                self.create_checkout_from_branch(repo_path, &start_point, new_branch, name)
            }
        }
    }

    /// Remove a workspace (worktree or checkout).
    pub fn remove_workspace(
        &self,
        mode: WorkspaceMode,
        repo_path: &Path,
        workspace_path: &Path,
    ) -> Result<(), WorktreeError> {
        match mode {
            WorkspaceMode::Worktree => self.worktree.remove_worktree(repo_path, workspace_path),
            WorkspaceMode::Checkout => self.remove_checkout(workspace_path),
        }
    }

    /// Prune stale worktree metadata (no-op for checkouts).
    pub fn prune_workspaces(
        &self,
        mode: WorkspaceMode,
        repo_path: &Path,
    ) -> Result<(), WorktreeError> {
        match mode {
            WorkspaceMode::Worktree => self.worktree.prune_worktrees(repo_path),
            WorkspaceMode::Checkout => Ok(()),
        }
    }

    /// Get current branch for a workspace path.
    pub fn get_current_branch(&self, workspace_path: &Path) -> Result<String, WorktreeError> {
        self.worktree.get_current_branch(workspace_path)
    }

    /// Get branch status for a workspace path.
    pub fn get_branch_status(&self, workspace_path: &Path) -> Result<BranchStatus, WorktreeError> {
        self.worktree.get_branch_status(workspace_path)
    }

    /// Get branch status for a workspace path, optionally preferring GH CLI for merge detection.
    pub fn get_branch_status_with_gh_option(
        &self,
        workspace_path: &Path,
        use_gh_cli_merge_status: bool,
    ) -> Result<BranchStatus, WorktreeError> {
        self.worktree.get_branch_status_with_options(
            workspace_path,
            BranchStatusOptions {
                use_gh_cli_merge_status,
            },
        )
    }

    /// Get a branch SHA for the repo/workspace depending on mode.
    pub fn get_branch_sha(
        &self,
        mode: WorkspaceMode,
        repo_path: &Path,
        workspace_path: &Path,
        branch: &str,
    ) -> Result<String, WorktreeError> {
        match mode {
            WorkspaceMode::Worktree => self.worktree.get_branch_sha(repo_path, branch),
            WorkspaceMode::Checkout => self.worktree.get_branch_sha(workspace_path, branch),
        }
    }

    /// Delete a local branch for either a worktree repo or a checkout workspace.
    pub fn delete_branch(
        &self,
        mode: WorkspaceMode,
        repo_path: &Path,
        workspace_path: &Path,
        branch: &str,
    ) -> Result<(), WorktreeError> {
        match mode {
            WorkspaceMode::Worktree => self.worktree.delete_branch(repo_path, branch),
            WorkspaceMode::Checkout => self.worktree.delete_branch(workspace_path, branch),
        }
    }

    /// Delete a remote branch (always uses the base repo path).
    pub fn delete_remote_branch(
        &self,
        repo_path: &Path,
        branch: &str,
    ) -> Result<(), WorktreeError> {
        self.worktree.delete_remote_branch(repo_path, branch)
    }

    /// Check if a remote branch exists on origin for the base repo path.
    pub fn remote_branch_exists(
        &self,
        repo_path: &Path,
        branch: &str,
    ) -> Result<bool, WorktreeError> {
        if !self.is_git_repo(repo_path) {
            return Err(WorktreeError::NotAGitRepo(repo_path.to_path_buf()));
        }

        let output = Command::new("git")
            .args(["ls-remote", "--exit-code", "--heads", "origin", branch])
            .current_dir(repo_path)
            .output()?;

        if output.status.success() {
            return Ok(true);
        }

        if output.status.code() == Some(2) {
            return Ok(false);
        }

        Err(WorktreeError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }

    /// Rename a local branch (in workspace path).
    pub fn rename_branch(
        &self,
        workspace_path: &Path,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), WorktreeError> {
        self.worktree
            .rename_branch(workspace_path, old_name, new_name)
    }

    /// Check if a path is a git repository.
    pub fn is_git_repo(&self, path: &Path) -> bool {
        self.worktree.is_git_repo(path)
    }

    fn create_checkout(
        &self,
        repo_path: &Path,
        branch: &str,
        name: &str,
    ) -> Result<PathBuf, WorktreeError> {
        if !self.is_git_repo(repo_path) {
            return Err(WorktreeError::NotAGitRepo(repo_path.to_path_buf()));
        }

        let workspace_path = self.workspace_path(repo_path, name);
        if workspace_path.exists() {
            return Err(WorktreeError::AlreadyExists(workspace_path));
        }

        if let Some(parent) = workspace_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Use the parent directory of the repo for cloning (works even for root-level repos).
        let base_dir = repo_path.parent().unwrap_or(Path::new("."));
        let output = Command::new("git")
            .args(["clone", "--no-hardlinks", "--"])
            .arg(repo_path)
            .arg(&workspace_path)
            .current_dir(base_dir)
            .output()?;

        if !output.status.success() {
            self.cleanup_failed_checkout(&workspace_path, "Failed to clone checkout workspace");
            return Err(WorktreeError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        self.sync_origin_from_base(repo_path, &workspace_path);

        let output = Command::new("git")
            .args(["checkout", branch])
            .current_dir(&workspace_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("did not match any file")
                || stderr.contains("did not match any")
                || stderr.contains("pathspec")
            {
                let output = Command::new("git")
                    .args(["checkout", "-b", branch])
                    .current_dir(&workspace_path)
                    .output()?;

                if !output.status.success() {
                    self.cleanup_failed_checkout(
                        &workspace_path,
                        "Failed to create branch in checkout workspace",
                    );
                    return Err(WorktreeError::CommandFailed(
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ));
                }
            } else {
                self.cleanup_failed_checkout(&workspace_path, "Failed to checkout branch");
                return Err(WorktreeError::CommandFailed(stderr.to_string()));
            }
        }

        Ok(workspace_path)
    }

    fn create_checkout_from_branch(
        &self,
        repo_path: &Path,
        base_branch: &str,
        new_branch: &str,
        name: &str,
    ) -> Result<PathBuf, WorktreeError> {
        if !self.is_git_repo(repo_path) {
            return Err(WorktreeError::NotAGitRepo(repo_path.to_path_buf()));
        }

        let workspace_path = self.workspace_path(repo_path, name);
        if workspace_path.exists() {
            return Err(WorktreeError::AlreadyExists(workspace_path));
        }

        if let Some(parent) = workspace_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Use the parent directory of the repo for cloning (works even for root-level repos).
        let base_dir = repo_path.parent().unwrap_or(Path::new("."));
        let output = Command::new("git")
            .args(["clone", "--no-hardlinks", "--"])
            .arg(repo_path)
            .arg(&workspace_path)
            .current_dir(base_dir)
            .output()?;

        if !output.status.success() {
            self.cleanup_failed_checkout(&workspace_path, "Failed to clone checkout workspace");
            return Err(WorktreeError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        self.sync_origin_from_base(repo_path, &workspace_path);

        let base_ref = base_branch.to_string();
        let output = Command::new("git")
            .args(["checkout", "-b", new_branch, &base_ref])
            .current_dir(&workspace_path)
            .output()?;

        if !output.status.success() {
            let initial_stderr = String::from_utf8_lossy(&output.stderr);

            let is_branch_exists =
                initial_stderr.contains("branch") && initial_stderr.contains("already exists");
            if is_branch_exists {
                let output = Command::new("git")
                    .args(["checkout", new_branch])
                    .current_dir(&workspace_path)
                    .output()?;
                if !output.status.success() {
                    self.cleanup_failed_checkout(
                        &workspace_path,
                        "Failed to checkout existing branch in checkout workspace",
                    );
                    return Err(WorktreeError::CommandFailed(
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ));
                }
                return Ok(workspace_path);
            }

            let is_invalid_ref = initial_stderr.contains("not a valid")
                || initial_stderr.contains("invalid reference")
                || initial_stderr.contains("pathspec");
            if is_invalid_ref {
                let origin_ref = format!("origin/{}", base_branch);
                let output = Command::new("git")
                    .args(["checkout", "-b", new_branch, &origin_ref])
                    .current_dir(&workspace_path)
                    .output()?;
                if output.status.success() {
                    return Ok(workspace_path);
                }
            }

            self.cleanup_failed_checkout(
                &workspace_path,
                "Failed to create branch from base in checkout workspace",
            );
            return Err(WorktreeError::CommandFailed(initial_stderr.to_string()));
        }

        Ok(workspace_path)
    }

    fn remove_checkout(&self, workspace_path: &Path) -> Result<(), WorktreeError> {
        if !workspace_path.exists() {
            return Err(WorktreeError::NotFound(workspace_path.to_path_buf()));
        }

        std::fs::remove_dir_all(workspace_path)?;
        Ok(())
    }

    fn cleanup_failed_checkout(&self, workspace_path: &Path, context: &str) {
        if let Err(err) = std::fs::remove_dir_all(workspace_path) {
            tracing::warn!(
                error = %err,
                path = %workspace_path.display(),
                "{context}"
            );
        }
    }

    fn sync_origin_from_base(&self, base_repo: &Path, workspace_path: &Path) {
        let origin_url = match Command::new("git")
            .args(["config", "--get", "remote.origin.url"])
            .current_dir(base_repo)
            .output()
        {
            Ok(output) if output.status.success() => {
                let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if url.is_empty() {
                    None
                } else {
                    Some(url)
                }
            }
            Ok(output) => {
                tracing::debug!(
                    error = %String::from_utf8_lossy(&output.stderr),
                    "Failed to read origin URL from base repo"
                );
                None
            }
            Err(e) => {
                tracing::debug!(error = %e, "Failed to run git config for origin URL");
                None
            }
        };

        let Some(origin_url) = origin_url else {
            return;
        };

        match Command::new("git")
            .args(["remote", "set-url", "origin", &origin_url])
            .current_dir(workspace_path)
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    tracing::warn!(
                        error = %String::from_utf8_lossy(&output.stderr),
                        path = %workspace_path.display(),
                        "Failed to update origin URL for checkout workspace"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    path = %workspace_path.display(),
                    "Failed to run git remote set-url for checkout workspace"
                );
            }
        }
    }
}

impl Default for WorkspaceRepoManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn init_git_repo(path: &Path) -> std::io::Result<()> {
        fn run_git_checked(path: &Path, args: &[&str]) -> std::io::Result<()> {
            let output = Command::new("git").args(args).current_dir(path).output()?;
            if output.status.success() {
                return Ok(());
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("git {:?} failed: {}{}", args, stdout, stderr),
            ))
        }

        run_git_checked(path, &["init"])?;
        run_git_checked(path, &["config", "user.email", "test@test.com"])?;
        run_git_checked(path, &["config", "user.name", "Test"])?;

        std::fs::write(path.join("README.md"), "# Test")?;
        run_git_checked(path, &["add", "."])?;
        run_git_checked(path, &["commit", "-m", "Initial commit"])?;

        Ok(())
    }

    fn run_git(path: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(path)
            .output()
            .expect("failed to run git");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_create_and_remove_checkout() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path().join("repo");
        std::fs::create_dir(&repo_path).unwrap();
        init_git_repo(&repo_path).unwrap();

        let manager = WorkspaceRepoManager::new();
        let wt_path = manager
            .create_workspace(
                WorkspaceMode::Checkout,
                &repo_path,
                "feature",
                "feature",
                |_| {},
            )
            .unwrap();

        assert!(wt_path.exists());
        manager
            .remove_workspace(WorkspaceMode::Checkout, &repo_path, &wt_path)
            .unwrap();
        assert!(!wt_path.exists());
    }

    #[test]
    fn test_remote_branch_exists() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path().join("repo");
        let remote_path = dir.path().join("remote.git");
        std::fs::create_dir(&repo_path).unwrap();
        init_git_repo(&repo_path).unwrap();

        run_git(
            dir.path(),
            &["init", "--bare", remote_path.to_str().unwrap()],
        );
        run_git(
            &repo_path,
            &["remote", "add", "origin", remote_path.to_str().unwrap()],
        );
        run_git(&repo_path, &["checkout", "-b", "feature/test"]);
        run_git(&repo_path, &["push", "-u", "origin", "feature/test"]);

        let manager = WorkspaceRepoManager::new();
        assert!(manager
            .remote_branch_exists(&repo_path, "feature/test")
            .unwrap());
        assert!(!manager
            .remote_branch_exists(&repo_path, "missing-branch")
            .unwrap());
    }

    fn get_head_sha(path: &Path) -> String {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(path)
            .output()
            .expect("failed to run git rev-parse");
        assert!(output.status.success(), "git rev-parse HEAD failed");
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Verify that a new workspace starts from `origin/main` even when local `main`
    /// is behind origin. The caller is responsible for fetching first; this test
    /// runs `git fetch` explicitly before `create_workspace` to mirror that contract.
    #[test]
    fn test_create_workspace_uses_origin_main_when_local_main_is_stale() {
        let dir = tempdir().unwrap();

        // Set up "remote": a non-bare repo acting as origin.
        let remote_path = dir.path().join("remote");
        std::fs::create_dir(&remote_path).unwrap();
        run_git(&remote_path, &["init", "-b", "main"]);
        run_git(&remote_path, &["config", "user.email", "test@test.com"]);
        run_git(&remote_path, &["config", "user.name", "Test"]);
        run_git(
            &remote_path,
            &["config", "receive.denyCurrentBranch", "ignore"],
        );
        std::fs::write(remote_path.join("README.md"), "# Remote").unwrap();
        run_git(&remote_path, &["add", "."]);
        run_git(&remote_path, &["commit", "-m", "initial"]);

        // Clone remote to get the local repo (local main = initial commit).
        let local_path = dir.path().join("local");
        run_git(
            dir.path(),
            &["clone", remote_path.to_str().unwrap(), "local"],
        );
        run_git(&local_path, &["config", "user.email", "test@test.com"]);
        run_git(&local_path, &["config", "user.name", "Test"]);

        // Add a second commit to the remote; local main stays at the first commit.
        std::fs::write(remote_path.join("extra.txt"), "new content").unwrap();
        run_git(&remote_path, &["add", "."]);
        run_git(&remote_path, &["commit", "-m", "second commit"]);
        let remote_head = get_head_sha(&remote_path);

        // Sanity: local main is still at the old commit before create_workspace runs.
        let local_main_sha = get_head_sha(&local_path);
        assert_ne!(
            local_main_sha, remote_head,
            "test setup: local main should be behind origin before workspace creation"
        );

        // The caller is now responsible for fetching before create_workspace runs.
        run_git(&local_path, &["fetch", "origin", "--quiet"]);

        // create_workspace must branch from origin/main (remote_head).
        let manager = WorkspaceRepoManager::with_managed_dir(dir.path().join("workspaces"));
        let wt_path = manager
            .create_workspace(
                WorkspaceMode::Worktree,
                &local_path,
                "fuz/new-feature",
                "new-feature",
                |_| {},
            )
            .expect("create_workspace failed");

        let workspace_head = get_head_sha(&wt_path);
        assert_eq!(
            workspace_head, remote_head,
            "workspace should start from origin/main ({}), not stale local main ({})",
            remote_head, local_main_sha
        );
    }
}
