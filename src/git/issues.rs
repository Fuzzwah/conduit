//! GitHub issue fetching via the gh CLI

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

/// A GitHub issue with its number and title
#[derive(Debug, Clone)]
pub struct GithubIssue {
    pub number: u32,
    pub title: String,
}

#[derive(Deserialize)]
struct GhIssue {
    number: u32,
    title: String,
}

/// Fetch open GitHub issues for the repository at `repo_path`.
///
/// Returns an empty list (not an error) when gh is unavailable, not authenticated,
/// the repo is not on GitHub, or there are simply no open issues.
pub fn fetch_open_issues(repo_path: &Path) -> Vec<GithubIssue> {
    if !is_github_repo(repo_path) {
        return Vec::new();
    }

    let output = Command::new("gh")
        .args([
            "issue",
            "list",
            "--json",
            "number,title",
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
        .map(|i| GithubIssue {
            number: i.number,
            title: i.title,
        })
        .collect()
}

fn is_github_repo(path: &Path) -> bool {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(path)
        .output();

    let Ok(output) = output else {
        return false;
    };

    if !output.status.success() {
        return false;
    }

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_lowercase()
        .contains("github.com")
}
