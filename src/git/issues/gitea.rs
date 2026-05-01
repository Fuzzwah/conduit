//! Gitea issue provider via the v1 REST API.
//!
//! Detection is allowlist-based: a repo is considered "Gitea" only if its
//! origin host appears in `IssuesConfig::gitea_hosts`. The host is derived
//! from the URL exactly the same way for HTTPS and SSH origins.
//!
//! Authentication uses `GITEA_TOKEN`. Without a token (or with an invalid one)
//! the provider returns an empty list — same silent contract as the GitHub
//! provider.

use std::env;
use std::path::Path;

use serde::Deserialize;

use crate::config::IssuesConfig;

use super::_rest::{extract_host, extract_owner_repo, get_json};
use super::{IssueProvider, RemoteIssue};

pub struct GiteaProvider;

#[derive(Deserialize)]
struct ApiLabel {
    name: String,
}

#[derive(Deserialize)]
struct ApiUser {
    login: String,
}

#[derive(Deserialize)]
struct ApiIssue {
    number: u32,
    title: String,
    #[serde(default)]
    labels: Vec<ApiLabel>,
    #[serde(default)]
    assignees: Vec<ApiUser>,
    /// Gitea returns the merged issue/PR feed by default; this lets us filter
    /// pull requests out client-side as a defence in depth, even though we also
    /// pass `type=issues` in the query string.
    #[serde(default)]
    pull_request: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct ApiSelfUser {
    login: String,
}

const ENV_TOKEN: &str = "GITEA_TOKEN";

fn host_in_allowlist(host: &str, hosts: &[String]) -> bool {
    hosts.iter().any(|h| h.eq_ignore_ascii_case(host))
}

fn read_origin_url(repo_path: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(repo_path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if url.is_empty() {
        None
    } else {
        Some(url)
    }
}

fn api_base(host: &str) -> String {
    format!("https://{}/api/v1", host)
}

impl IssueProvider for GiteaProvider {
    fn name(&self) -> &'static str {
        "gitea"
    }

    fn supports(&self, remote_url: &str, config: &IssuesConfig) -> bool {
        let Some(host) = extract_host(remote_url) else {
            return false;
        };
        // GitHub always wins; the registry already orders GitHub first, but be
        // explicit so that a misconfigured `gitea_hosts = ["github.com"]` can't
        // route GitHub repos through the wrong provider.
        if host == "github.com" {
            return false;
        }
        host_in_allowlist(&host, &config.gitea_hosts)
    }

    fn fetch_open_issues(&self, repo_path: &Path) -> Vec<RemoteIssue> {
        let Ok(token) = env::var(ENV_TOKEN) else {
            return Vec::new();
        };
        let Some(origin) = read_origin_url(repo_path) else {
            return Vec::new();
        };
        let Some(host) = extract_host(&origin) else {
            return Vec::new();
        };
        let Some((owner, repo)) = extract_owner_repo(&origin) else {
            return Vec::new();
        };

        let url = format!(
            "{}/repos/{}/{}/issues?state=open&type=issues&limit=50",
            api_base(&host),
            owner,
            repo
        );

        let issues: Vec<ApiIssue> = match get_json(&url, &token) {
            Some(v) => v,
            None => return Vec::new(),
        };

        issues
            .into_iter()
            .filter(|i| i.pull_request.is_none())
            .map(|i| RemoteIssue {
                number: i.number,
                title: i.title,
                labels: i.labels.into_iter().map(|l| l.name).collect(),
                assignee_logins: i.assignees.into_iter().map(|u| u.login).collect(),
            })
            .collect()
    }

    fn current_user(&self, repo_path: &Path) -> Option<String> {
        let token = env::var(ENV_TOKEN).ok()?;
        let origin = read_origin_url(repo_path)?;
        let host = extract_host(&origin)?;
        let url = format!("{}/user", api_base(&host));
        let user: ApiSelfUser = get_json(&url, &token)?;
        if user.login.is_empty() {
            None
        } else {
            Some(user.login)
        }
    }
}
