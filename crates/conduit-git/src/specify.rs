//! Spec-kit (specify) spec detection

use std::fs;
use std::path::Path;
use std::process::Command;

/// A spec-kit spec with remaining tasks
#[derive(Debug, Clone)]
pub struct SpecifySpec {
    pub spec_id: String,
    pub remaining_tasks: usize,
    pub total_tasks: usize,
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

/// Scan `.specify/specs/*/tasks.md` in `repo_path`, returning specs that have
/// at least one unchecked task, sorted by remaining tasks descending.
pub fn fetch_specify_specs(repo_path: &Path) -> Vec<SpecifySpec> {
    let specs_dir = repo_path.join(".specify").join("specs");
    let Ok(entries) = fs::read_dir(&specs_dir) else {
        return Vec::new();
    };

    let mut specs: Vec<SpecifySpec> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            if !path.is_dir() {
                return None;
            }

            let spec_id = path.file_name()?.to_string_lossy().into_owned();
            let tasks_path = path.join("tasks.md");
            let content = fs::read_to_string(&tasks_path).ok()?;

            let (remaining, completed) = parse_tasks(&content);
            if remaining == 0 {
                return None;
            }

            Some(SpecifySpec {
                spec_id,
                remaining_tasks: remaining,
                total_tasks: remaining + completed,
            })
        })
        .collect();

    specs.sort_by_key(|s| std::cmp::Reverse(s.remaining_tasks));
    specs
}

/// Scan `.specify/specs/*/tasks.md` at the given git ref rather than the working
/// tree. Reads via `git ls-tree` + `git show`. Returns an empty `Vec` on any git
/// error (caller may fall back to `fetch_specify_specs`).
pub fn fetch_specify_specs_from_ref(repo_path: &Path, git_ref: &str) -> Vec<SpecifySpec> {
    let ls = match Command::new("git")
        .args(["ls-tree", "-d", "--name-only", git_ref, ".specify/specs/"])
        .current_dir(repo_path)
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let dir_listing = String::from_utf8_lossy(&ls.stdout).into_owned();
    let mut specs: Vec<SpecifySpec> = dir_listing
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim().trim_end_matches('/');
            let id = trimmed.strip_prefix(".specify/specs/")?;
            if id.is_empty() || id.contains('/') {
                return None;
            }
            let path_in_ref = format!("{}:.specify/specs/{}/tasks.md", git_ref, id);
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
            Some(SpecifySpec {
                spec_id: id.to_string(),
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

    fn write_spec(repo: &Path, id: &str, body: &str) {
        let dir = repo.join(".specify/specs").join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("tasks.md"), body).unwrap();
    }

    #[test]
    fn from_ref_skips_uncommitted_specify_spec() {
        let dir = tempdir().unwrap();
        let (local, _seed, branch) = make_seed_remote_local(dir.path());
        write_spec(&local, "local-only", "- [ ] something\n");

        let specs = fetch_specify_specs_from_ref(&local, &format!("origin/{}", branch));
        assert!(specs.iter().all(|s| s.spec_id != "local-only"));
    }

    #[test]
    fn from_ref_lists_committed_open_specify_spec() {
        let dir = tempdir().unwrap();
        let (local, seed, branch) = make_seed_remote_local(dir.path());
        write_spec(&seed, "feature-y", "- [ ] a\n- [x] b\n");
        run_git(&seed, &["add", "."]);
        run_git(&seed, &["commit", "-q", "-m", "add feature-y"]);
        run_git(&seed, &["push", "-q", "origin", &branch]);
        run_git(&local, &["fetch", "-q", "origin"]);

        let specs = fetch_specify_specs_from_ref(&local, &format!("origin/{}", branch));
        let found = specs.iter().find(|s| s.spec_id == "feature-y").unwrap();
        assert_eq!(found.remaining_tasks, 1);
        assert_eq!(found.total_tasks, 2);
    }

    #[test]
    fn from_ref_returns_empty_when_ref_missing() {
        let dir = tempdir().unwrap();
        init(dir.path());
        let specs = fetch_specify_specs_from_ref(dir.path(), "origin/does-not-exist");
        assert!(specs.is_empty());
    }
}
