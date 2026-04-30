//! Spec-kit (specify) spec detection

use std::fs;
use std::path::Path;

/// A spec-kit spec with remaining tasks
#[derive(Debug, Clone)]
pub struct SpecifySpec {
    pub spec_id: String,
    pub remaining_tasks: usize,
    pub total_tasks: usize,
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
