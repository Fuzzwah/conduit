//! Simple git action wrappers for the Work Complete flow.

use std::io;
use std::path::Path;
use std::process::Command;

/// Stage all changes and create a commit. Returns the new commit SHA.
pub fn commit_all(path: &Path, message: &str) -> io::Result<String> {
    let add = Command::new("git")
        .args(["add", "-A"])
        .current_dir(path)
        .output()?;
    if !add.status.success() {
        return Err(io::Error::other(format!(
            "git add -A failed: {}",
            String::from_utf8_lossy(&add.stderr).trim()
        )));
    }

    let commit = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(path)
        .output()?;
    if !commit.status.success() {
        return Err(io::Error::other(format!(
            "git commit failed: {}",
            String::from_utf8_lossy(&commit.stderr).trim()
        )));
    }

    // Extract the commit SHA from the output line "  [branch <sha>] ..."
    let stdout = String::from_utf8_lossy(&commit.stdout);
    let sha = stdout
        .lines()
        .find_map(|line| {
            // Match "[branch abc1234]" or "[branch (root-commit) abc1234]"
            let inside = line.trim().strip_prefix('[')?.split(']').next()?;
            inside.split_whitespace().next_back().map(str::to_string)
        })
        .map(io::Result::Ok)
        .unwrap_or_else(|| {
            // Fallback: ask git directly
            let out = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(path)
                .output()?;
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        })?;

    Ok(sha)
}

/// Push the given branch to `origin`. When `set_upstream` is true, passes `-u`.
pub fn push_branch(path: &Path, branch: &str, set_upstream: bool) -> io::Result<()> {
    let mut args = vec!["push"];
    if set_upstream {
        args.push("-u");
    }
    args.extend(["origin", branch]);

    let output = Command::new("git").args(&args).current_dir(path).output()?;

    if output.status.success() {
        return Ok(());
    }

    // "Everything up-to-date" is printed to stderr with exit 0 by git, so
    // the only way we get here is a real failure.
    Err(io::Error::other(format!(
        "git push failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::tempdir;

    fn git(path: &Path, args: &[&str]) {
        let out = Command::new("git")
            .args(args)
            .current_dir(path)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn init_repo(path: &Path) {
        git(path, &["init", "-q"]);
        git(path, &["config", "user.email", "t@t"]);
        git(path, &["config", "user.name", "T"]);
        git(path, &["config", "commit.gpgsign", "false"]);
    }

    fn make_local_with_remote(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let remote = dir.join("remote.git");
        let local = dir.join("local");

        // Bare remote
        std::fs::create_dir_all(&remote).unwrap();
        git(&remote, &["init", "-q", "--bare"]);

        // Local clone
        Command::new("git")
            .args([
                "clone",
                "-q",
                remote.to_str().unwrap(),
                local.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        git(&local, &["config", "user.email", "t@t"]);
        git(&local, &["config", "user.name", "T"]);
        git(&local, &["config", "commit.gpgsign", "false"]);

        // Initial commit so the repo isn't empty
        std::fs::write(local.join("README.md"), "init").unwrap();
        git(&local, &["add", "."]);
        git(&local, &["commit", "-q", "-m", "init"]);
        git(&local, &["push", "-q", "-u", "origin", "HEAD"]);

        (local, remote)
    }

    #[test]
    fn commit_all_stages_and_commits() {
        let dir = tempdir().unwrap();
        init_repo(dir.path());
        std::fs::write(dir.path().join("README.md"), "init").unwrap();
        git(dir.path(), &["add", "."]);
        git(dir.path(), &["commit", "-q", "-m", "init"]);

        std::fs::write(dir.path().join("feature.txt"), "new").unwrap();
        let sha = commit_all(dir.path(), "add feature").unwrap();

        assert!(!sha.is_empty(), "SHA should be returned");
        let log = Command::new("git")
            .args(["log", "--oneline", "-1"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&log.stdout).contains("add feature"));
    }

    #[test]
    fn commit_all_returns_err_when_nothing_to_commit() {
        let dir = tempdir().unwrap();
        init_repo(dir.path());
        std::fs::write(dir.path().join("README.md"), "init").unwrap();
        git(dir.path(), &["add", "."]);
        git(dir.path(), &["commit", "-q", "-m", "init"]);

        // Nothing new to commit
        let result = commit_all(dir.path(), "empty commit");
        assert!(result.is_err());
    }

    #[test]
    fn push_branch_uploads_to_remote() {
        let dir = tempdir().unwrap();
        let (local, _remote) = make_local_with_remote(dir.path());

        // Create a feature branch with a commit
        git(&local, &["checkout", "-q", "-b", "feature-x"]);
        std::fs::write(local.join("x.txt"), "x").unwrap();
        git(&local, &["add", "."]);
        git(&local, &["commit", "-q", "-m", "feat"]);

        push_branch(&local, "feature-x", true).unwrap();

        // Verify origin now has the branch
        let out = Command::new("git")
            .args(["ls-remote", "--heads", "origin", "feature-x"])
            .current_dir(&local)
            .output()
            .unwrap();
        assert!(
            !out.stdout.is_empty(),
            "Remote should have feature-x branch"
        );
    }

    #[test]
    fn push_branch_fails_on_no_remote() {
        let dir = tempdir().unwrap();
        init_repo(dir.path());
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        git(dir.path(), &["add", "."]);
        git(dir.path(), &["commit", "-q", "-m", "init"]);

        let result = push_branch(dir.path(), "main", false);
        assert!(result.is_err());
    }
}
