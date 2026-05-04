//! Git status utilities for tracking diff statistics

use std::path::Path;
use std::process::Command;

/// Git diff statistics (additions, deletions, files changed)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitDiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub files_changed: usize,
}

impl GitDiffStats {
    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        self.additions > 0 || self.deletions > 0
    }

    /// Get git diff stats for the current working directory
    /// Uses `git diff --shortstat` to get uncommitted changes
    pub fn from_working_dir(working_dir: &Path) -> Self {
        // Get stats for staged and unstaged changes
        let output = Command::new("git")
            .args(["--no-optional-locks", "diff", "--shortstat", "HEAD"])
            .current_dir(working_dir)
            .output();

        let stats = match output {
            Ok(o) if o.status.success() => {
                let output_str = String::from_utf8_lossy(&o.stdout);
                Self::parse_shortstat(&output_str)
            }
            _ => GitDiffStats::default(),
        };

        // Fallback: if HEAD comparison fails (e.g., no commits yet), try unstaged-only diff
        if stats == GitDiffStats::default() {
            let unstaged = Command::new("git")
                .args(["--no-optional-locks", "diff", "--shortstat"])
                .current_dir(working_dir)
                .output();

            if let Ok(o) = unstaged {
                if o.status.success() {
                    let output_str = String::from_utf8_lossy(&o.stdout);
                    return Self::parse_shortstat(&output_str);
                }
            }
        }

        stats
    }

    /// Parse from `git diff --shortstat` output
    /// Format: "1 file changed, 44 insertions(+), 10 deletions(-)"
    fn parse_shortstat(output: &str) -> Self {
        let mut stats = GitDiffStats::default();
        let output = output.trim();

        if output.is_empty() {
            return stats;
        }

        for part in output.split(',') {
            let part = part.trim();
            if part.contains("insertion") {
                stats.additions = part
                    .split_whitespace()
                    .next()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0);
            } else if part.contains("deletion") {
                stats.deletions = part
                    .split_whitespace()
                    .next()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0);
            } else if part.contains("file") {
                stats.files_changed = part
                    .split_whitespace()
                    .next()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0);
            }
        }

        stats
    }
}

/// A single dirty file entry from `git status --porcelain`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DirtyFile {
    /// XY status codes (e.g. " M", "??", "A ")
    pub status: String,
    pub path: String,
}

/// List dirty files using `git status --porcelain`.
pub fn git_diff_files(working_dir: &Path) -> Vec<DirtyFile> {
    let output = Command::new("git")
        .args(["--no-optional-locks", "status", "--porcelain"])
        .current_dir(working_dir)
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            if line.len() < 3 {
                return None;
            }
            let status = line[..2].to_string();
            let path = line[3..].trim_matches('"').to_string();
            Some(DirtyFile { status, path })
        })
        .collect()
}

/// Get the number of commits HEAD is ahead/behind its origin main branch.
///
/// Returns `(commits_ahead, commits_behind)`. Both are 0 on any failure (e.g., no
/// remote, detached HEAD). This compares against the locally-cached remote ref so
/// it does not perform a network fetch.
pub fn get_ahead_behind(working_dir: &Path) -> (usize, usize) {
    // Detect the default branch name (e.g. "main" or "master") from the origin remote.
    let main_branch = Command::new("git")
        .args(["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])
        .current_dir(working_dir)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                // Strip "origin/" prefix → e.g. "origin/main" → "main"
                s.strip_prefix("origin/").map(|b| b.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "main".to_string());

    let output = Command::new("git")
        .args([
            "rev-list",
            "--left-right",
            "--count",
            &format!("HEAD...origin/{}", main_branch),
        ])
        .current_dir(working_dir)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let parts: Vec<&str> = stdout.split_whitespace().collect();
            if parts.len() == 2 {
                let ahead = parts[0].parse().unwrap_or(0);
                let behind = parts[1].parse().unwrap_or(0);
                return (ahead, behind);
            }
        }
        _ => {}
    }

    (0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shortstat_full() {
        let output = " 3 files changed, 44 insertions(+), 10 deletions(-)";
        let stats = GitDiffStats::parse_shortstat(output);
        assert_eq!(stats.files_changed, 3);
        assert_eq!(stats.additions, 44);
        assert_eq!(stats.deletions, 10);
    }

    #[test]
    fn test_parse_shortstat_insertions_only() {
        let output = " 1 file changed, 25 insertions(+)";
        let stats = GitDiffStats::parse_shortstat(output);
        assert_eq!(stats.files_changed, 1);
        assert_eq!(stats.additions, 25);
        assert_eq!(stats.deletions, 0);
    }

    #[test]
    fn test_parse_shortstat_deletions_only() {
        let output = " 2 files changed, 15 deletions(-)";
        let stats = GitDiffStats::parse_shortstat(output);
        assert_eq!(stats.files_changed, 2);
        assert_eq!(stats.additions, 0);
        assert_eq!(stats.deletions, 15);
    }

    #[test]
    fn test_parse_shortstat_empty() {
        let output = "";
        let stats = GitDiffStats::parse_shortstat(output);
        assert_eq!(stats.files_changed, 0);
        assert_eq!(stats.additions, 0);
        assert_eq!(stats.deletions, 0);
    }

    #[test]
    fn test_has_changes() {
        let empty = GitDiffStats::default();
        assert!(!empty.has_changes());

        let with_additions = GitDiffStats {
            additions: 10,
            deletions: 0,
            files_changed: 1,
        };
        assert!(with_additions.has_changes());

        let with_deletions = GitDiffStats {
            additions: 0,
            deletions: 5,
            files_changed: 1,
        };
        assert!(with_deletions.has_changes());
    }

    #[test]
    fn git_diff_files_returns_dirty_entries() {
        use std::process::Command as Cmd;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let path = dir.path();
        Cmd::new("git")
            .args(["init", "-q"])
            .current_dir(path)
            .output()
            .unwrap();
        Cmd::new("git")
            .args(["config", "user.email", "t@t"])
            .current_dir(path)
            .output()
            .unwrap();
        Cmd::new("git")
            .args(["config", "user.name", "T"])
            .current_dir(path)
            .output()
            .unwrap();
        // Initial commit
        std::fs::write(path.join("a.txt"), "init").unwrap();
        Cmd::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        Cmd::new("git")
            .args(["commit", "-q", "-m", "init"])
            .current_dir(path)
            .output()
            .unwrap();

        // Modify a tracked file and add an untracked file
        std::fs::write(path.join("a.txt"), "changed").unwrap();
        std::fs::write(path.join("b.txt"), "new").unwrap();

        let files = git_diff_files(path);
        assert!(!files.is_empty(), "should report dirty files");
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"a.txt"), "a.txt should appear as modified");
        assert!(paths.contains(&"b.txt"), "b.txt should appear as untracked");
    }

    #[test]
    fn git_diff_files_empty_on_clean_repo() {
        use std::process::Command as Cmd;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let path = dir.path();
        Cmd::new("git")
            .args(["init", "-q"])
            .current_dir(path)
            .output()
            .unwrap();
        Cmd::new("git")
            .args(["config", "user.email", "t@t"])
            .current_dir(path)
            .output()
            .unwrap();
        Cmd::new("git")
            .args(["config", "user.name", "T"])
            .current_dir(path)
            .output()
            .unwrap();
        std::fs::write(path.join("a.txt"), "init").unwrap();
        Cmd::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        Cmd::new("git")
            .args(["commit", "-q", "-m", "init"])
            .current_dir(path)
            .output()
            .unwrap();

        let files = git_diff_files(path);
        assert!(files.is_empty(), "clean repo should have no dirty files");
    }
}
