use std::io;
use std::path::{Component, Path};

/// Remove the managed project folder under the given workspaces root.
///
/// Returns `Some(error_message)` when a safety or IO error should be surfaced
/// to the user, or `None` when there is nothing to remove or the operation
/// succeeds.
pub fn remove_project_workspaces_dir(workspaces_root: &Path, repo_name: &str) -> Option<String> {
    let repo_name_path = Path::new(repo_name);
    let mut components = repo_name_path.components();
    let is_safe_repo_name =
        matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none();

    if !is_safe_repo_name {
        return Some(format!(
            "Skipping project folder removal due to unsafe repo name: {}",
            repo_name
        ));
    }

    let project_path = workspaces_root.join(repo_name);
    match (
        std::fs::canonicalize(workspaces_root),
        std::fs::canonicalize(&project_path),
    ) {
        (Ok(canonical_root), Ok(canonical_project)) => {
            if canonical_project.starts_with(&canonical_root) {
                if let Err(e) = std::fs::remove_dir_all(&canonical_project) {
                    return Some(format!("Failed to remove project folder: {}", e));
                }
            } else {
                return Some(format!(
                    "Skipping project folder removal outside managed root: {}",
                    canonical_project.display()
                ));
            }
        }
        (Err(e), _) => {
            if e.kind() != io::ErrorKind::NotFound {
                return Some(format!("Failed to canonicalize workspaces dir: {}", e));
            }
        }
        (_, Err(e)) => {
            if e.kind() != io::ErrorKind::NotFound {
                return Some(format!("Failed to canonicalize project folder: {}", e));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::remove_project_workspaces_dir;
    use tempfile::tempdir;

    #[test]
    fn missing_workspaces_root_is_not_an_error() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("workspaces");

        let err = remove_project_workspaces_dir(&root, "my-repo");
        assert!(err.is_none());
    }

    #[test]
    fn removes_existing_project_folder() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("workspaces");
        std::fs::create_dir_all(root.join("my-repo")).expect("create project dir");

        let err = remove_project_workspaces_dir(&root, "my-repo");
        assert!(err.is_none());
        assert!(!root.join("my-repo").exists());
    }

    #[test]
    fn unsafe_repo_name_is_rejected() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("workspaces");
        std::fs::create_dir_all(&root).expect("create root");

        let err = remove_project_workspaces_dir(&root, "../bad").expect("expected error");
        assert!(err.contains("unsafe repo name"));
    }
}
