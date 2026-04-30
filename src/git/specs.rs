//! OpenSpec change detection

use std::fs;
use std::path::Path;

/// An openspec change with remaining tasks
#[derive(Debug, Clone)]
pub struct OpenSpec {
    pub change_id: String,
    pub remaining_tasks: usize,
    pub total_tasks: usize,
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
