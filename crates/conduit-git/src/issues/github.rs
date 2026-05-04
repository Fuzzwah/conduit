//! GitHub issue provider via the `gh` CLI.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use super::{IssueProvider, IssuesConfig, RemoteIssue};

pub struct GitHubProvider;

#[derive(Deserialize)]
struct GhLabel {
    name: String,
}

#[derive(Deserialize)]
struct GhAssignee {
    login: String,
}

#[derive(Deserialize)]
struct GhIssue {
    number: u32,
    title: String,
    #[serde(default)]
    labels: Vec<GhLabel>,
    #[serde(default)]
    assignees: Vec<GhAssignee>,
}

impl IssueProvider for GitHubProvider {
    fn name(&self) -> &'static str {
        "github"
    }

    fn supports(&self, remote_url: &str, _config: &IssuesConfig) -> bool {
        remote_url.to_lowercase().contains("github.com")
    }

    fn fetch_open_issues(&self, repo_path: &Path) -> Vec<RemoteIssue> {
        let output = Command::new("gh")
            .args([
                "issue",
                "list",
                "--json",
                "number,title,labels,assignees",
                "--state",
                "open",
                "--limit",
                "50",
            ])
            .current_dir(repo_path)
            .output();

        let Ok(output) = output else {
            return Vec::new();
        };

        if !output.status.success() {
            return Vec::new();
        }

        let json = String::from_utf8_lossy(&output.stdout);
        let parsed: Vec<GhIssue> = serde_json::from_str(&json).unwrap_or_default();
        parsed
            .into_iter()
            .map(|i| RemoteIssue {
                number: i.number,
                title: i.title,
                labels: i.labels.into_iter().map(|l| l.name).collect(),
                assignee_logins: i.assignees.into_iter().map(|a| a.login).collect(),
            })
            .collect()
    }

    fn current_user(&self, repo_path: &Path) -> Option<String> {
        let output = Command::new("gh")
            .args(["api", "user", "--jq", ".login"])
            .current_dir(repo_path)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let login = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if login.is_empty() {
            None
        } else {
            Some(login)
        }
    }
}

/// A GitHub issue view returned by `gh issue view`.
#[derive(Debug, Clone)]
pub struct IssueView {
    pub number: i32,
    pub title: String,
    pub state: String,
    pub url: String,
}

/// Fetch a single GitHub issue by number using `gh issue view`.
pub fn view_issue(repo_path: &Path, number: i32) -> Option<IssueView> {
    #[derive(Deserialize)]
    struct GhIssueView {
        number: i32,
        title: String,
        state: String,
        url: String,
    }

    let output = Command::new("gh")
        .args([
            "issue",
            "view",
            &number.to_string(),
            "--json",
            "number,title,state,url",
        ])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json = String::from_utf8_lossy(&output.stdout);
    let issue: GhIssueView = serde_json::from_str(&json).ok()?;
    Some(IssueView {
        number: issue.number,
        title: issue.title,
        state: issue.state,
        url: issue.url,
    })
}

/// Infer a GitHub issue number from a branch name by matching the first `#<digits>` token.
pub fn infer_active_issue(branch_name: &str) -> Option<i32> {
    let bytes = branch_name.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'#' {
            let start = i + 1;
            let end = bytes[start..]
                .iter()
                .position(|&b| !b.is_ascii_digit())
                .map(|n| start + n)
                .unwrap_or(bytes.len());
            if end > start {
                if let Ok(n) = branch_name[start..end].parse::<i32>() {
                    return Some(n);
                }
            }
        }
        i += 1;
    }
    None
}

/// Close a GitHub issue by number using `gh issue close`.
pub fn close_issue(repo_path: &Path, number: i32) -> std::io::Result<()> {
    let output = Command::new("gh")
        .args(["issue", "close", &number.to_string()])
        .current_dir(repo_path)
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(std::io::Error::other(format!(
            "gh issue close failed: {}",
            stderr
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_issue_from_branch_with_hash() {
        assert_eq!(infer_active_issue("fuz/feat-#123"), Some(123));
    }

    #[test]
    fn infer_issue_picks_first_number() {
        assert_eq!(infer_active_issue("fuz/#42-and-#99"), Some(42));
    }

    #[test]
    fn infer_issue_returns_none_when_no_token() {
        assert_eq!(infer_active_issue("fuz/feature-without-issue"), None);
    }

    #[test]
    fn infer_issue_returns_none_for_empty_string() {
        assert_eq!(infer_active_issue(""), None);
    }

    #[test]
    fn infer_issue_handles_hash_at_end_no_digits() {
        assert_eq!(infer_active_issue("branch#"), None);
    }

    // --- close_issue stub tests ---

    static GH_STUB_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[cfg(unix)]
    fn with_fake_gh<F: FnOnce() -> R, R>(script_body: &str, f: F) -> R {
        use std::os::unix::fs::PermissionsExt;
        use tempfile::tempdir;

        let _guard = GH_STUB_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let bin_dir = tempdir().unwrap();
        let gh_path = bin_dir.path().join("gh");
        std::fs::write(&gh_path, format!("#!/bin/sh\n{}\n", script_body)).unwrap();
        let mut perms = std::fs::metadata(&gh_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&gh_path, perms).unwrap();

        let old_path = std::env::var("PATH").unwrap_or_default();
        unsafe {
            std::env::set_var("PATH", format!("{}:{}", bin_dir.path().display(), old_path));
        }
        let result = f();
        unsafe {
            std::env::set_var("PATH", &old_path);
        }
        result
    }

    #[test]
    #[cfg(unix)]
    fn close_issue_succeeds_when_gh_exits_zero() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        with_fake_gh("exit 0", || {
            close_issue(dir.path(), 42).unwrap();
        });
    }

    #[test]
    #[cfg(unix)]
    fn close_issue_errors_when_gh_fails() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        with_fake_gh("echo 'not found' >&2; exit 1", || {
            let result = close_issue(dir.path(), 99);
            assert!(result.is_err());
        });
    }

    #[test]
    #[cfg(unix)]
    fn view_issue_parses_open_issue() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let script = r#"printf '{"number":42,"title":"Fix bug","state":"OPEN","url":"https://github.com/x/y/issues/42"}\n'"#;
        with_fake_gh(script, || {
            let view = view_issue(dir.path(), 42).unwrap();
            assert_eq!(view.number, 42);
            assert_eq!(view.state, "OPEN");
            assert_eq!(view.title, "Fix bug");
        });
    }

    #[test]
    #[cfg(unix)]
    fn view_issue_returns_none_on_gh_failure() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        with_fake_gh("exit 1", || {
            assert!(view_issue(dir.path(), 99).is_none());
        });
    }
}
