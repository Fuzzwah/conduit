//! CI check monitoring for the Work Complete flow.

use std::process::Command;

/// Run `gh pr checks --watch <pr_url>` and wait for all checks to reach a
/// terminal state.  Returns `(passed, log_lines)` on success (exit 0 =
/// passed) or an error string if the command cannot run at all.
pub fn wait_for_ci_checks(pr_url: &str) -> Result<(bool, Vec<String>), String> {
    let output = Command::new("gh")
        .args(["pr", "checks", "--watch", pr_url])
        .output()
        .map_err(|e| format!("Failed to run gh pr checks: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut lines: Vec<String> = stdout
        .lines()
        .map(str::to_string)
        .filter(|l| !l.trim().is_empty())
        .collect();

    if !stderr.trim().is_empty() {
        for line in stderr.lines() {
            if !line.trim().is_empty() {
                lines.push(format!("gh: {line}"));
            }
        }
    }

    Ok((output.status.success(), lines))
}
