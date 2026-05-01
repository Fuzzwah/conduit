//! Tiny blocking HTTP helper shared by Gitea and Forgejo providers.
//!
//! Both forges expose the Gitea v1 REST API at `/api/v1`, take a bearer token,
//! and return JSON. We keep this helper minimal: a single GET that deserializes
//! the body into `T` or returns `None`. All errors collapse to `None` so the
//! callers can keep their silent-empty-on-error contract.

use std::time::Duration;

use serde::de::DeserializeOwned;

const TIMEOUT_SECS: u64 = 10;

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .build()
}

/// GET `url` with `Authorization: token <token>` and `Accept: application/json`.
/// Returns `Some(T)` on a 2xx with parseable JSON, otherwise `None`.
pub fn get_json<T: DeserializeOwned>(url: &str, token: &str) -> Option<T> {
    let response = agent()
        .get(url)
        .set("Authorization", &format!("token {}", token))
        .set("Accept", "application/json")
        .call()
        .ok()?;
    response.into_json::<T>().ok()
}

/// Parse `host[:port]` out of either an HTTPS URL (`https://host[:port]/foo/bar`)
/// or an SSH URL (`git@host:foo/bar.git`). Returns the lowercased host. Used by
/// both the Gitea and Forgejo providers to match the repo's origin against the
/// user's host allowlist.
pub fn extract_host(remote_url: &str) -> Option<String> {
    let trimmed = remote_url.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rest) = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .or_else(|| trimmed.strip_prefix("ssh://"))
    {
        // Strip optional `user@`, then split off the path.
        let after_user = rest.split_once('@').map(|(_, h)| h).unwrap_or(rest);
        let host = after_user
            .split(['/', ':'])
            .next()
            .unwrap_or(after_user)
            .to_lowercase();
        return if host.is_empty() { None } else { Some(host) };
    }

    // SCP-style: user@host:path
    if let Some((_, after_user)) = trimmed.split_once('@') {
        if let Some((host, _)) = after_user.split_once(':') {
            let h = host.to_lowercase();
            return if h.is_empty() { None } else { Some(h) };
        }
    }

    None
}

/// Parse `owner/repo` out of a Gitea/Forgejo origin URL. Strips a trailing
/// `.git` suffix.
pub fn extract_owner_repo(remote_url: &str) -> Option<(String, String)> {
    let trimmed = remote_url.trim().trim_end_matches('/');
    let path = if let Some(rest) = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .or_else(|| trimmed.strip_prefix("ssh://"))
    {
        let after_user = rest.split_once('@').map(|(_, h)| h).unwrap_or(rest);
        // Drop host[:port], keep the rest.
        after_user.split_once('/').map(|(_, p)| p)?
    } else if let Some((_, after_user)) = trimmed.split_once('@') {
        // SCP-style: user@host:owner/repo
        after_user.split_once(':').map(|(_, p)| p)?
    } else {
        return None;
    };

    let path = path.trim_start_matches('/').trim_end_matches(".git");
    let mut parts = path.splitn(3, '/');
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner, repo))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_host_https() {
        assert_eq!(
            extract_host("https://gitea.example.com/foo/bar.git"),
            Some("gitea.example.com".into())
        );
    }

    #[test]
    fn extract_host_https_with_port() {
        assert_eq!(
            extract_host("https://gitea.example.com:3000/foo/bar.git"),
            Some("gitea.example.com".into())
        );
    }

    #[test]
    fn extract_host_ssh_scp_style() {
        assert_eq!(
            extract_host("git@codeberg.org:foo/bar.git"),
            Some("codeberg.org".into())
        );
    }

    #[test]
    fn extract_host_lowercases_host() {
        // Real git remotes always use lowercase scheme; only the host needs
        // case folding for allowlist matching.
        assert_eq!(
            extract_host("https://Gitea.Example.COM/foo"),
            Some("gitea.example.com".into())
        );
    }

    #[test]
    fn extract_host_rejects_garbage() {
        assert!(extract_host("").is_none());
        assert!(extract_host("not-a-url").is_none());
    }

    #[test]
    fn extract_owner_repo_https() {
        assert_eq!(
            extract_owner_repo("https://gitea.example.com/alice/proj.git"),
            Some(("alice".into(), "proj".into()))
        );
    }

    #[test]
    fn extract_owner_repo_ssh() {
        assert_eq!(
            extract_owner_repo("git@codeberg.org:bob/widget.git"),
            Some(("bob".into(), "widget".into()))
        );
    }

    #[test]
    fn extract_owner_repo_no_dot_git() {
        assert_eq!(
            extract_owner_repo("https://gitea.example.com/alice/proj"),
            Some(("alice".into(), "proj".into()))
        );
    }

    #[test]
    fn extract_owner_repo_rejects_short_path() {
        assert!(extract_owner_repo("https://gitea.example.com/onlyone").is_none());
        assert!(extract_owner_repo("").is_none());
    }
}
