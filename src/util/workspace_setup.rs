//! Workspace setup script support.

use std::path::Path;

/// Run `workspace_setup.sh` from the base repo path inside the new workspace directory.
///
/// The script is looked up in the repository root (not the workspace). If it doesn't
/// exist the function is a no-op. Failures are logged as warnings but do not prevent
/// workspace creation from succeeding.
pub fn run_workspace_setup_script(repo_path: &Path, workspace_path: &Path) {
    let script = repo_path.join("workspace_setup.sh");
    if !script.exists() {
        return;
    }

    tracing::info!(
        script = %script.display(),
        workspace = %workspace_path.display(),
        "Running workspace setup script"
    );

    match std::process::Command::new(&script)
        .current_dir(workspace_path)
        .env("WORKSPACE_PATH", workspace_path)
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                tracing::info!(
                    workspace = %workspace_path.display(),
                    "Workspace setup script completed successfully"
                );
            } else {
                tracing::warn!(
                    workspace = %workspace_path.display(),
                    stderr = %String::from_utf8_lossy(&output.stderr),
                    stdout = %String::from_utf8_lossy(&output.stdout),
                    status = %output.status,
                    "Workspace setup script failed"
                );
            }
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                script = %script.display(),
                workspace = %workspace_path.display(),
                "Failed to run workspace setup script"
            );
        }
    }
}
