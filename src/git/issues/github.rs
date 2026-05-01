//! GitHub issue provider via the `gh` CLI.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::config::IssuesConfig;

use super::{IssueProvider, RemoteIssue};

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
