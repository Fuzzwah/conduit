//! OpenSpec change detection

use std::fs;
use std::path::Path;
use std::process::Command;

/// An openspec change with remaining tasks
#[derive(Debug, Clone)]
pub struct OpenSpec {
    pub change_id: String,
    pub remaining_tasks: usize,
    pub total_tasks: usize,
}

/// Task completion counts for a single OpenSpec change (used by Work Complete preflight).
#[derive(Debug, Clone)]
pub struct SpecDetail {
    pub change_id: String,
    pub total: usize,
    pub completed: usize,
}

/// Return task completion counts for a specific change, reading from the working tree.
///
/// Returns `None` when `openspec/changes/<change_id>/tasks.md` is absent or unreadable.
pub fn fetch_change_detail(repo_path: &Path, change_id: &str) -> Option<SpecDetail> {
    let tasks_path = repo_path
        .join("openspec")
        .join("changes")
        .join(change_id)
        .join("tasks.md");
    let content = fs::read_to_string(&tasks_path).ok()?;
    let (remaining, completed) = parse_tasks(&content);
    Some(SpecDetail {
        change_id: change_id.to_string(),
        total: remaining + completed,
        completed,
    })
}

/// Infer the OpenSpec change id created on this branch by inspecting `git log`.
///
/// Runs `git log --diff-filter=A --name-only origin/<base_branch>..HEAD -- openspec/changes/`
/// and returns the basename of the most-recently-added change directory, if any.
/// When multiple directories were added, picks the most recently modified one on disk.
pub fn infer_active_change(repo_path: &Path, base_branch: &str) -> Option<String> {
    let range = format!("origin/{}..HEAD", base_branch);
    let output = Command::new("git")
        .args([
            "log",
            "--diff-filter=A",
            "--name-only",
            "--format=",
            &range,
            "--",
            "openspec/changes/",
        ])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let changes_prefix = "openspec/changes/";
    let mut candidates: Vec<String> = stdout
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            // Match paths like "openspec/changes/<id>/..." or "openspec/changes/<id>"
            let rest = line.strip_prefix(changes_prefix)?;
            let id = rest.split('/').next()?;
            if id.is_empty() || id == "archive" {
                return None;
            }
            Some(id.to_string())
        })
        .collect();

    candidates.dedup();

    if candidates.is_empty() {
        return None;
    }

    if candidates.len() == 1 {
        return Some(candidates.remove(0));
    }

    // Multiple candidates: pick the most recently modified directory
    candidates.into_iter().max_by_key(|id| {
        let dir = repo_path.join("openspec").join("changes").join(id);
        fs::metadata(&dir)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    })
}

fn parse_tasks(content: &str) -> (usize, usize) {
    let mut remaining = 0usize;
    let mut completed = 0usize;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("- [ ]") {
            remaining += 1;
        } else if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
            completed += 1;
        }
    }
    (remaining, completed)
}

/// Scan `openspec/changes/*/tasks.md` in `repo_path`, returning changes that have
/// at least one unchecked task, sorted by remaining tasks descending.
pub fn fetch_open_specs(repo_path: &Path) -> Vec<OpenSpec> {
    let changes_dir = repo_path.join("openspec").join("changes");
    let Ok(entries) = fs::read_dir(&changes_dir) else {
        return Vec::new();
    };

    let mut specs: Vec<OpenSpec> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            // Skip archive directory
            if path.file_name()?.to_string_lossy() == "archive" {
                return None;
            }

            if !path.is_dir() {
                return None;
            }

            let change_id = path.file_name()?.to_string_lossy().into_owned();
            let tasks_path = path.join("tasks.md");
            let content = fs::read_to_string(&tasks_path).ok()?;

            let (remaining, completed) = parse_tasks(&content);
            if remaining == 0 {
                return None;
            }

            Some(OpenSpec {
                change_id,
                remaining_tasks: remaining,
                total_tasks: remaining + completed,
            })
        })
        .collect();

    specs.sort_by_key(|s| std::cmp::Reverse(s.remaining_tasks));
    specs
}

/// Returns true when `path` in `git_ref` is stored as a git symlink (mode 120000).
///
/// This happens when the working tree uses a symlink (e.g. `openspec -> ../openspec`)
/// instead of a real directory committed inside the repo.
fn git_path_is_symlink(repo_path: &Path, git_ref: &str, path: &str) -> bool {
    match Command::new("git")
        .args(["ls-tree", git_ref, path])
        .current_dir(repo_path)
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .any(|line| line.starts_with("120000 ")),
        _ => false,
    }
}

/// Scan `openspec/changes/*/tasks.md` at the given git ref (e.g. `origin/master`)
/// rather than the working tree. Reads via `git ls-tree` + `git show`. Returns an
/// empty `Vec` on any git error (caller may fall back to `fetch_open_specs`).
///
/// When `openspec/` is a git symlink (mode 120000) rather than a real tree, git
/// cannot traverse into it. In that case this function falls back to the working-tree
/// scan via `fetch_open_specs`, because the symlink resolves correctly on the
/// filesystem and the archived-change concern does not apply (the symlink target is
/// managed by a separate repository).
pub fn fetch_open_specs_from_ref(repo_path: &Path, git_ref: &str) -> Vec<OpenSpec> {
    let ls = match Command::new("git")
        .args(["ls-tree", "-d", "--name-only", git_ref, "openspec/changes/"])
        .current_dir(repo_path)
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => {
            if git_path_is_symlink(repo_path, git_ref, "openspec") {
                return fetch_open_specs(repo_path);
            }
            return Vec::new();
        }
    };

    let dir_listing = String::from_utf8_lossy(&ls.stdout).into_owned();

    // git ls-tree returns empty output when `openspec/` is a symlink blob rather than
    // a tree — it cannot traverse into it. Fall back to the working-tree scan.
    if dir_listing.trim().is_empty() && git_path_is_symlink(repo_path, git_ref, "openspec") {
        return fetch_open_specs(repo_path);
    }
    let mut specs: Vec<OpenSpec> = dir_listing
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim().trim_end_matches('/');
            let id = trimmed.strip_prefix("openspec/changes/")?;
            if id.is_empty() || id == "archive" || id.contains('/') {
                return None;
            }
            let path_in_ref = format!("{}:openspec/changes/{}/tasks.md", git_ref, id);
            let show = Command::new("git")
                .args(["show", &path_in_ref])
                .current_dir(repo_path)
                .output()
                .ok()?;
            if !show.status.success() {
                return None;
            }
            let content = String::from_utf8_lossy(&show.stdout);
            let (remaining, completed) = parse_tasks(&content);
            if remaining == 0 {
                return None;
            }
            Some(OpenSpec {
                change_id: id.to_string(),
                remaining_tasks: remaining,
                total_tasks: remaining + completed,
            })
        })
        .collect();

    specs.sort_by_key(|s| std::cmp::Reverse(s.remaining_tasks));
    specs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn run_git(path: &Path, args: &[&str]) {
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

    fn init(path: &Path) {
        std::fs::create_dir_all(path).unwrap();
        run_git(path, &["init", "-q"]);
        run_git(path, &["config", "user.email", "t@t"]);
        run_git(path, &["config", "user.name", "T"]);
    }

    fn make_seed_remote_local(dir: &Path) -> (PathBuf, PathBuf, String) {
        let seed = dir.join("seed");
        init(&seed);
        std::fs::write(seed.join("README.md"), "x").unwrap();
        run_git(&seed, &["add", "."]);
        run_git(&seed, &["commit", "-q", "-m", "init"]);
        let branch = {
            let out = Command::new("git")
                .args(["symbolic-ref", "--short", "HEAD"])
                .current_dir(&seed)
                .output()
                .unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };

        let remote = dir.join("remote.git");
        run_git(
            dir,
            &[
                "clone",
                "--bare",
                "-q",
                seed.to_str().unwrap(),
                remote.to_str().unwrap(),
            ],
        );
        run_git(
            &seed,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        run_git(&seed, &["push", "-q", "-u", "origin", &branch]);

        let local = dir.join("local");
        run_git(
            dir,
            &[
                "clone",
                "-q",
                remote.to_str().unwrap(),
                local.to_str().unwrap(),
            ],
        );
        run_git(&local, &["config", "user.email", "t@t"]);
        run_git(&local, &["config", "user.name", "T"]);
        (local, seed, branch)
    }

    fn write_change(repo: &Path, id: &str, body: &str) {
        let dir = repo.join("openspec/changes").join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("tasks.md"), body).unwrap();
    }

    #[test]
    fn from_ref_skips_change_archived_on_remote() {
        let dir = tempdir().unwrap();
        let (local, seed, branch) = make_seed_remote_local(dir.path());

        // Add a change in the local working tree only — it should NOT appear when
        // reading from origin/<branch>.
        write_change(&local, "local-only", "- [ ] do work\n");

        // Push a change from seed, then archive it (rename to archive/) on seed
        // and push again. The local working tree still has the unarchived dir.
        write_change(&seed, "stale-archived", "- [ ] do work\n");
        run_git(&seed, &["add", "."]);
        run_git(&seed, &["commit", "-q", "-m", "add stale-archived"]);
        run_git(&seed, &["push", "-q", "origin", &branch]);

        // Pull into local so it has a working-tree copy of stale-archived.
        run_git(&local, &["pull", "-q", "origin", &branch]);
        assert!(local.join("openspec/changes/stale-archived").exists());

        // Now archive it on seed and push.
        std::fs::create_dir_all(seed.join("openspec/changes/archive")).unwrap();
        std::fs::rename(
            seed.join("openspec/changes/stale-archived"),
            seed.join("openspec/changes/archive/stale-archived"),
        )
        .unwrap();
        run_git(&seed, &["add", "-A"]);
        run_git(&seed, &["commit", "-q", "-m", "archive stale-archived"]);
        run_git(&seed, &["push", "-q", "origin", &branch]);

        // From local, fetch but do NOT pull — the working tree still has the
        // change, but origin/<branch> no longer does.
        run_git(&local, &["fetch", "-q", "origin"]);
        assert!(local.join("openspec/changes/stale-archived").exists());

        let from_ref = fetch_open_specs_from_ref(&local, &format!("origin/{}", branch));
        let ids: Vec<&str> = from_ref.iter().map(|s| s.change_id.as_str()).collect();
        assert!(
            !ids.contains(&"stale-archived"),
            "archived change leaked through: {:?}",
            ids
        );
        assert!(
            !ids.contains(&"local-only"),
            "uncommitted local change leaked through: {:?}",
            ids
        );
    }

    #[test]
    fn from_ref_lists_committed_open_change() {
        let dir = tempdir().unwrap();
        let (local, seed, branch) = make_seed_remote_local(dir.path());

        write_change(&seed, "feature-x", "- [ ] step one\n- [x] step two\n");
        run_git(&seed, &["add", "."]);
        run_git(&seed, &["commit", "-q", "-m", "add feature-x"]);
        run_git(&seed, &["push", "-q", "origin", &branch]);
        run_git(&local, &["fetch", "-q", "origin"]);

        let specs = fetch_open_specs_from_ref(&local, &format!("origin/{}", branch));
        let found = specs.iter().find(|s| s.change_id == "feature-x").unwrap();
        assert_eq!(found.remaining_tasks, 1);
        assert_eq!(found.total_tasks, 2);
    }

    #[test]
    fn from_ref_returns_empty_when_ref_missing() {
        let dir = tempdir().unwrap();
        init(dir.path());
        let specs = fetch_open_specs_from_ref(dir.path(), "origin/does-not-exist");
        assert!(specs.is_empty());
    }

    // --- infer_active_change tests ---

    fn setup_feature_branch(dir: &Path) -> (PathBuf, String) {
        let (local, _seed, base) = make_seed_remote_local(dir);
        // Create a feature branch
        run_git(&local, &["checkout", "-q", "-b", "fuz/my-feature"]);
        (local, base)
    }

    #[test]
    fn infer_returns_single_added_change() {
        let dir = tempdir().unwrap();
        let (local, base) = setup_feature_branch(dir.path());

        write_change(&local, "my-feature", "- [ ] step\n");
        run_git(&local, &["add", "."]);
        run_git(&local, &["commit", "-q", "-m", "add change"]);

        let result = infer_active_change(&local, &base);
        assert_eq!(result.as_deref(), Some("my-feature"));
    }

    #[test]
    fn infer_returns_none_when_no_dirs_added() {
        let dir = tempdir().unwrap();
        let (local, base) = setup_feature_branch(dir.path());

        // Commit something unrelated
        std::fs::write(local.join("some.txt"), "x").unwrap();
        run_git(&local, &["add", "."]);
        run_git(&local, &["commit", "-q", "-m", "no change dir"]);

        let result = infer_active_change(&local, &base);
        assert!(result.is_none());
    }

    #[test]
    fn infer_picks_most_recently_modified_for_multiple_dirs() {
        let dir = tempdir().unwrap();
        let (local, base) = setup_feature_branch(dir.path());

        write_change(&local, "first-change", "- [ ] a\n");
        run_git(&local, &["add", "."]);
        run_git(&local, &["commit", "-q", "-m", "add first"]);

        write_change(&local, "second-change", "- [ ] b\n");
        run_git(&local, &["add", "."]);
        run_git(&local, &["commit", "-q", "-m", "add second"]);

        // Touch second-change to make it the most recently modified
        let second_dir = local.join("openspec/changes/second-change");
        std::fs::write(second_dir.join("extra.txt"), "touch").unwrap();

        let result = infer_active_change(&local, &base);
        // Should pick second-change (most recently modified)
        assert_eq!(result.as_deref(), Some("second-change"));
    }

    // --- fetch_change_detail tests ---

    #[test]
    fn fetch_detail_returns_counts() {
        let dir = tempdir().unwrap();
        write_change(
            dir.path(),
            "my-change",
            "- [x] done\n- [ ] todo\n- [ ] todo2\n",
        );

        let detail = fetch_change_detail(dir.path(), "my-change").unwrap();
        assert_eq!(detail.total, 3);
        assert_eq!(detail.completed, 1);
    }

    #[test]
    fn fetch_detail_returns_none_for_missing_change() {
        let dir = tempdir().unwrap();
        let result = fetch_change_detail(dir.path(), "nonexistent");
        assert!(result.is_none());
    }

    /// When `openspec/` is a git symlink (as in a child repo pointing to a shared parent
    /// workspace openspec directory), `git ls-tree -d` cannot traverse it. The function
    /// should fall back to the working-tree scan and surface the in-progress changes.
    #[test]
    fn from_ref_falls_back_when_openspec_is_symlink() {
        let dir = tempdir().unwrap();

        // Parent "workspace" repo: has the real openspec directory with an open change.
        let parent = dir.path().join("parent");
        std::fs::create_dir_all(&parent).unwrap();
        write_change(
            &parent,
            "cross-repo-feature",
            "- [ ] step one\n- [ ] step two\n",
        );

        // Child repo: has a committed symlink openspec -> ../parent/openspec
        let (local, _seed, branch) = make_seed_remote_local(dir.path());
        let symlink_path = local.join("openspec");
        #[cfg(unix)]
        std::os::unix::fs::symlink("../parent/openspec", &symlink_path).unwrap();
        run_git(&local, &["add", "openspec"]);
        run_git(&local, &["commit", "-q", "-m", "add openspec symlink"]);
        run_git(&local, &["push", "-q", "origin", &branch]);
        run_git(&local, &["fetch", "-q", "origin"]);

        let specs = fetch_open_specs_from_ref(&local, &format!("origin/{}", branch));
        let found = specs
            .iter()
            .find(|s| s.change_id == "cross-repo-feature")
            .expect("should find change via symlink fallback");
        assert_eq!(found.remaining_tasks, 2);
        assert_eq!(found.total_tasks, 2);
    }
}
