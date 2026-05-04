//! Issue provider abstraction.
//!
//! `fetch_open_issues` resolves the repo's `origin` URL, looks up a matching
//! `IssueProvider`, and dispatches to it. Today the registry contains GitHub
//! (via `gh` CLI), Gitea (REST), and Forgejo (REST). All providers fail
//! silently to an empty list — the workspace-creation flow treats "no issues"
//! and "couldn't reach the host" identically.

mod _rest;
mod forgejo;
mod gitea;
mod github;

pub use github::{close_issue, infer_active_issue, view_issue, IssueView};

use std::path::Path;
use std::process::Command;

/// Configuration for remote issue providers (Gitea, Forgejo).
///
/// GitHub is detected by `github.com` in the origin URL and needs no config.
/// Gitea and Forgejo cannot be reliably detected from the URL alone, so users
/// must list their hosts here. `GITEA_TOKEN` / `FORGEJO_TOKEN` env vars supply
/// authentication; without a token, the provider returns an empty list.
#[derive(Debug, Clone, Default)]
pub struct IssuesConfig {
    pub gitea_hosts: Vec<String>,
    pub forgejo_hosts: Vec<String>,
}

/// A remote issue. The shape is shared across providers so the picker UI doesn't
/// have to care which forge served it.
#[derive(Debug, Clone)]
pub struct RemoteIssue {
    pub number: u32,
    pub title: String,
    pub labels: Vec<String>,
    pub assignee_logins: Vec<String>,
}

/// Source of open issues for a repository. Implementations are stateless and
/// resolved once via `provider_for(remote_url, config)`.
pub trait IssueProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn supports(&self, remote_url: &str, config: &IssuesConfig) -> bool;
    fn fetch_open_issues(&self, repo_path: &Path) -> Vec<RemoteIssue>;
    /// Login of the currently-authenticated user, if known. Used by the
    /// "mine only" toggle. `None` means "couldn't determine" — the toggle is
    /// rendered as unavailable.
    fn current_user(&self, repo_path: &Path) -> Option<String>;
}

fn all_providers() -> [&'static dyn IssueProvider; 3] {
    [
        &github::GitHubProvider,
        &gitea::GiteaProvider,
        &forgejo::ForgejoProvider,
    ]
}

/// Resolve the provider that handles `remote_url`, if any.
pub fn provider_for(remote_url: &str, config: &IssuesConfig) -> Option<&'static dyn IssueProvider> {
    all_providers()
        .into_iter()
        .find(|p| p.supports(remote_url, config))
}

fn read_origin_url(repo_path: &Path) -> Option<String> {
    let out = Command::new("git")
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

/// Top-level helper used by the workspace-creation effect handler.
pub fn fetch_open_issues(repo_path: &Path, config: &IssuesConfig) -> Vec<RemoteIssue> {
    let Some(url) = read_origin_url(repo_path) else {
        return Vec::new();
    };
    let Some(provider) = provider_for(&url, config) else {
        tracing::debug!(remote = %url, "no issue provider matched origin URL");
        return Vec::new();
    };
    tracing::debug!(provider = provider.name(), "fetching open issues");
    provider.fetch_open_issues(repo_path)
}

/// Resolve the current user for whichever provider serves this repo. Used by
/// the picker's "mine only" toggle. Returns `None` if no provider matches or
/// if the provider couldn't determine the user.
pub fn current_user(repo_path: &Path, config: &IssuesConfig) -> Option<String> {
    let url = read_origin_url(repo_path)?;
    let provider = provider_for(&url, config)?;
    provider.current_user(repo_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_for_recognises_github() {
        let cfg = IssuesConfig::default();
        let p = provider_for("https://github.com/foo/bar.git", &cfg).unwrap();
        assert_eq!(p.name(), "github");
    }

    #[test]
    fn provider_for_recognises_github_ssh() {
        let cfg = IssuesConfig::default();
        let p = provider_for("git@github.com:foo/bar.git", &cfg).unwrap();
        assert_eq!(p.name(), "github");
    }

    #[test]
    fn provider_for_recognises_gitea_via_config() {
        let cfg = IssuesConfig {
            gitea_hosts: vec!["gitea.example.com".into()],
            forgejo_hosts: vec![],
        };
        let p = provider_for("https://gitea.example.com/foo/bar.git", &cfg).unwrap();
        assert_eq!(p.name(), "gitea");

        let p_ssh = provider_for("git@gitea.example.com:foo/bar.git", &cfg).unwrap();
        assert_eq!(p_ssh.name(), "gitea");
    }

    #[test]
    fn provider_for_recognises_forgejo_via_config() {
        let cfg = IssuesConfig {
            gitea_hosts: vec![],
            forgejo_hosts: vec!["codeberg.org".into()],
        };
        let p = provider_for("https://codeberg.org/foo/bar.git", &cfg).unwrap();
        assert_eq!(p.name(), "forgejo");
    }

    #[test]
    fn provider_for_returns_none_for_unknown_host() {
        let cfg = IssuesConfig::default();
        assert!(provider_for("https://gitlab.example.com/foo/bar.git", &cfg).is_none());
        assert!(provider_for("", &cfg).is_none());
    }

    #[test]
    fn github_takes_precedence_over_misconfigured_hosts() {
        // Even if a user accidentally lists github.com as a Gitea host, the
        // GitHub provider must still win because it's first in the registry.
        let cfg = IssuesConfig {
            gitea_hosts: vec!["github.com".into()],
            forgejo_hosts: vec![],
        };
        let p = provider_for("https://github.com/foo/bar.git", &cfg).unwrap();
        assert_eq!(p.name(), "github");
    }
}
