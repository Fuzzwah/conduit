//! OpenSpec change archiving (rename only — no spec-sync in v1).

use std::fs;
use std::path::Path;

use chrono::NaiveDate;

/// Result of a successful spec archive operation.
#[derive(Debug, Clone)]
pub struct ArchiveResult {
    /// Absolute path to the new archive directory.
    pub new_path: std::path::PathBuf,
    /// Warnings to surface to the user (e.g. delta specs not synced).
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("Source directory not found: {0}")]
    SourceNotFound(std::path::PathBuf),
    #[error("Target already exists: {0}")]
    TargetExists(std::path::PathBuf),
    #[error("Rename failed: {0}")]
    Rename(std::io::Error),
}

/// Rename `openspec/changes/<change_id>` → `openspec/changes/archive/<today>-<change_id>`.
///
/// Does NOT commit the rename — the caller (Work Complete dialog) handles that
/// by cycling back through preflight, which surfaces the rename as a dirty change.
///
/// Returns `ArchiveError::SourceNotFound` (404-style) when the source is absent,
/// `ArchiveError::TargetExists` (409-style) when the target already exists.
///
/// Includes a warning in the result when the change directory contains delta
/// `.md` files under `specs/` (because spec-sync is out of scope for v1).
pub fn archive_change(
    repo_path: &Path,
    change_id: &str,
    today: NaiveDate,
) -> Result<ArchiveResult, ArchiveError> {
    let source = repo_path.join("openspec").join("changes").join(change_id);

    if !source.exists() {
        return Err(ArchiveError::SourceNotFound(source));
    }

    let archive_dir = repo_path.join("openspec").join("changes").join("archive");
    let target_name = format!("{}-{}", today.format("%Y-%m-%d"), change_id);
    let target = archive_dir.join(&target_name);

    if target.exists() {
        return Err(ArchiveError::TargetExists(target));
    }

    // Ensure the archive directory exists
    fs::create_dir_all(&archive_dir).map_err(ArchiveError::Rename)?;

    let warnings = collect_delta_warnings(&source);

    fs::rename(&source, &target).map_err(ArchiveError::Rename)?;

    Ok(ArchiveResult {
        new_path: target,
        warnings,
    })
}

/// Return a warning message when the change directory contains delta spec files.
fn collect_delta_warnings(change_dir: &Path) -> Vec<String> {
    let specs_dir = change_dir.join("specs");
    if !specs_dir.is_dir() {
        return Vec::new();
    }

    let has_delta = walkdir_has_md(&specs_dir);
    if has_delta {
        vec![
            "Spec deltas not auto-synced — run /opsx:archive in the agent session if you have spec changes.".to_string(),
        ]
    } else {
        Vec::new()
    }
}

fn walkdir_has_md(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && walkdir_has_md(&path) {
            return true;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn make_change(repo: &Path, id: &str) -> PathBuf {
        let dir = repo.join("openspec").join("changes").join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("tasks.md"), "- [x] done\n").unwrap();
        dir
    }

    fn make_change_with_specs(repo: &Path, id: &str) -> PathBuf {
        let dir = make_change(repo, id);
        let spec_cap = dir.join("specs").join("my-cap");
        fs::create_dir_all(&spec_cap).unwrap();
        fs::write(spec_cap.join("spec.md"), "## MODIFIED\n").unwrap();
        dir
    }

    #[test]
    fn happy_path_renames_directory() {
        let dir = tempdir().unwrap();
        let today = NaiveDate::from_ymd_opt(2025, 5, 3).unwrap();
        make_change(dir.path(), "my-feature");

        let result = archive_change(dir.path(), "my-feature", today).unwrap();

        let expected = dir
            .path()
            .join("openspec/changes/archive/2025-05-03-my-feature");
        assert_eq!(result.new_path, expected);
        assert!(expected.exists());
        assert!(!dir.path().join("openspec/changes/my-feature").exists());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn preserves_nested_files() {
        let dir = tempdir().unwrap();
        let today = NaiveDate::from_ymd_opt(2025, 5, 3).unwrap();
        let change_dir = make_change(dir.path(), "nested");
        fs::create_dir_all(change_dir.join("specs/cap")).unwrap();
        fs::write(change_dir.join("specs/cap/spec.md"), "content").unwrap();

        let result = archive_change(dir.path(), "nested", today).unwrap();
        assert!(result.new_path.join("tasks.md").exists());
        assert!(result.new_path.join("specs/cap/spec.md").exists());
    }

    #[test]
    fn refuses_missing_source() {
        let dir = tempdir().unwrap();
        let today = NaiveDate::from_ymd_opt(2025, 5, 3).unwrap();
        fs::create_dir_all(dir.path().join("openspec/changes")).unwrap();

        let err = archive_change(dir.path(), "does-not-exist", today).unwrap_err();
        assert!(matches!(err, ArchiveError::SourceNotFound(_)));
    }

    #[test]
    fn refuses_duplicate_target() {
        let dir = tempdir().unwrap();
        let today = NaiveDate::from_ymd_opt(2025, 5, 3).unwrap();
        make_change(dir.path(), "dup-feature");

        // Pre-create the target
        let target = dir
            .path()
            .join("openspec/changes/archive/2025-05-03-dup-feature");
        fs::create_dir_all(&target).unwrap();

        let err = archive_change(dir.path(), "dup-feature", today).unwrap_err();
        assert!(matches!(err, ArchiveError::TargetExists(_)));
        // Source must still be intact
        assert!(dir.path().join("openspec/changes/dup-feature").exists());
    }

    #[test]
    fn surfaces_delta_spec_warning() {
        let dir = tempdir().unwrap();
        let today = NaiveDate::from_ymd_opt(2025, 5, 3).unwrap();
        make_change_with_specs(dir.path(), "delta-change");

        let result = archive_change(dir.path(), "delta-change", today).unwrap();
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("opsx:archive"));
    }

    #[test]
    fn no_warning_when_no_specs() {
        let dir = tempdir().unwrap();
        let today = NaiveDate::from_ymd_opt(2025, 5, 3).unwrap();
        make_change(dir.path(), "plain-change");

        let result = archive_change(dir.path(), "plain-change", today).unwrap();
        assert!(result.warnings.is_empty());
    }
}
