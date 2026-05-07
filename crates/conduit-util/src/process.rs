use std::time::{Duration, Instant};

#[cfg(unix)]
pub fn configure_command_process_group(cmd: &mut tokio::process::Command) {
    cmd.process_group(0);
}

#[cfg(not(unix))]
pub fn configure_command_process_group(_cmd: &mut tokio::process::Command) {}

#[cfg(unix)]
pub fn signal_process_tree(pid: u32, signal: i32) -> std::io::Result<()> {
    let process_group = -(pid as i32);
    let result = unsafe { libc::kill(process_group, signal) };
    if result == 0 {
        return Ok(());
    }

    let group_error = std::io::Error::last_os_error();
    if group_error.raw_os_error() != Some(libc::ESRCH) {
        return Err(group_error);
    }

    let direct_result = unsafe { libc::kill(pid as i32, signal) };
    if direct_result == -1 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}

#[cfg(not(unix))]
pub fn signal_process_tree(_pid: u32, _signal: i32) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Process-tree signaling is not supported on this platform",
    ))
}

pub fn terminate_process_tree(
    pid: u32,
    pid_start_time: Option<u64>,
    context: &str,
    grace: Duration,
    poll_interval: Duration,
) -> bool {
    #[cfg(unix)]
    {
        match signal_process_tree(pid, libc::SIGTERM) {
            Ok(()) => {}
            Err(err) if err.raw_os_error() == Some(libc::ESRCH) => return true,
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    pid,
                    context,
                    "Failed to send SIGTERM to agent process tree"
                );
            }
        }

        if wait_for_pid_exit(pid, grace, poll_interval, context, "SIGTERM") {
            return true;
        }

        // Log if we can't verify PID identity, but proceed with SIGKILL anyway.
        // The process is confirmed alive (wait_for_pid_exit just returned false), so
        // PID reuse since our last kill(pid,0) poll is essentially impossible.
        if let Some(expected) = pid_start_time {
            match crate::process::pid_start_time(pid) {
                Some(current) if current != expected => {
                    tracing::warn!(
                        pid,
                        context,
                        expected_start_time = expected,
                        current_start_time = current,
                        "Agent pid start time mismatch before SIGKILL; process may have been replaced"
                    );
                }
                None => {
                    tracing::warn!(
                        pid,
                        context,
                        "Unable to verify agent pid start time before SIGKILL; proceeding anyway"
                    );
                }
                _ => {}
            }
        }

        match signal_process_tree(pid, libc::SIGKILL) {
            Ok(()) => {}
            Err(err) if err.raw_os_error() == Some(libc::ESRCH) => return true,
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    pid,
                    context,
                    "Failed to send SIGKILL to agent process tree"
                );
                return false;
            }
        }

        if wait_for_pid_exit(pid, grace, poll_interval, context, "SIGKILL") {
            return true;
        }

        tracing::warn!(
            pid,
            context,
            "Agent process tree still running after SIGKILL grace period"
        );
        false
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, pid_start_time, grace, poll_interval);
        tracing::warn!(
            pid,
            context,
            "Process termination not implemented on this platform"
        );
        false
    }
}

#[cfg(unix)]
fn wait_for_pid_exit(
    pid: u32,
    timeout: Duration,
    poll_interval: Duration,
    context: &str,
    signal: &str,
) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        let result = unsafe { libc::kill(pid as i32, 0) };
        if result == 0 {
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(poll_interval);
            continue;
        }

        let err = std::io::Error::last_os_error();
        match err.raw_os_error() {
            Some(libc::ESRCH) => return true,
            Some(libc::EPERM) => {
                if Instant::now() >= deadline {
                    return false;
                }
                std::thread::sleep(poll_interval);
            }
            _ => {
                tracing::warn!(
                    error = %err,
                    pid,
                    context,
                    signal,
                    "Failed to poll agent pid after signal"
                );
                return false;
            }
        }
    }
}

#[cfg(target_os = "linux")]
pub fn pid_start_time(pid: u32) -> Option<u64> {
    let stat = match std::fs::read_to_string(format!("/proc/{}/stat", pid)) {
        Ok(contents) => contents,
        Err(err) => {
            tracing::debug!(
                pid,
                error = %err,
                "Failed to read /proc/{}/stat for pid start time",
                pid
            );
            return None;
        }
    };

    let end = stat.rfind(')')?;
    stat.get(end + 2..)?
        .split_whitespace()
        .nth(19)?
        .parse()
        .ok()
}

#[cfg(all(unix, not(target_os = "linux")))]
pub fn pid_start_time(_pid: u32) -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::signal_process_tree;
    #[cfg(unix)]
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::process::CommandExt;
    #[cfg(unix)]
    use std::process::{Command, Stdio};
    #[cfg(unix)]
    use std::time::{Duration, Instant};

    #[cfg(unix)]
    fn pid_exists(pid: u32) -> bool {
        let result = unsafe { libc::kill(pid as i32, 0) };
        if result == 0 {
            return true;
        }
        std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
    }

    #[cfg(unix)]
    #[test]
    fn signal_process_tree_terminates_background_children() {
        let marker =
            std::env::temp_dir().join(format!("conduit-process-test-{}", std::process::id()));
        let script = format!("sleep 300 & echo $! > '{}' && wait", marker.display());

        let mut child = Command::new("sh");
        child.arg("-c").arg(script);
        child.stdin(Stdio::null());
        child.stdout(Stdio::null());
        child.stderr(Stdio::null());
        child.process_group(0);

        let mut child = child.spawn().expect("spawn shell");
        let parent_pid = child.id();

        let deadline = Instant::now() + Duration::from_secs(5);
        let child_pid = loop {
            if let Ok(contents) = fs::read_to_string(&marker) {
                break contents.trim().parse::<u32>().expect("parse child pid");
            }
            assert!(Instant::now() < deadline, "timed out waiting for child pid");
            std::thread::sleep(Duration::from_millis(50));
        };

        signal_process_tree(parent_pid, libc::SIGTERM).expect("signal process tree");
        child.wait().expect("wait for parent");

        let child_deadline = Instant::now() + Duration::from_secs(5);
        while pid_exists(child_pid) && Instant::now() < child_deadline {
            std::thread::sleep(Duration::from_millis(50));
        }

        let _ = fs::remove_file(&marker);
        assert!(
            !pid_exists(child_pid),
            "background child should be terminated"
        );
    }
}
