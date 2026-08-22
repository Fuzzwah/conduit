use std::path::PathBuf;
use std::process::Stdio;

use async_trait::async_trait;
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::error::AgentError;
use crate::events::{AgentEvent, ErrorEvent};
use crate::runner::{AgentHandle, AgentInput, AgentRunner, AgentStartConfig, AgentType};
use crate::stream::{ClaudeRawEvent, JsonlStreamParser};

pub struct MakiRunner {
    binary_path: PathBuf,
}

impl MakiRunner {
    pub fn new() -> Self {
        Self {
            binary_path: Self::find_binary().unwrap_or_else(|| PathBuf::from("maki")),
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { binary_path: path }
    }

    fn find_binary() -> Option<PathBuf> {
        which::which("maki").ok()
    }

    fn build_command(&self, config: &AgentStartConfig) -> Command {
        let mut cmd = Command::new(&self.binary_path);

        cmd.arg("--print");
        cmd.arg("--output-format").arg("stream-json");
        // Headless mode has no way to answer maki's interactive permission
        // prompts, so allow everything (equivalent to Copilot's --allow-all).
        cmd.arg("--yolo");

        if let Some(model) = &config.model {
            cmd.arg("--model").arg(model);
        }

        // Resume a specific session when conduit has a stored session id.
        if let Some(session_id) = &config.resume_session {
            cmd.arg("--session").arg(session_id.as_str());
        }

        cmd.current_dir(&config.working_dir);

        for arg in &config.additional_args {
            cmd.arg(arg);
        }

        if !config.prompt.is_empty() {
            cmd.arg("--").arg(&config.prompt);
        }

        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        cmd
    }

    fn convert_event(raw: ClaudeRawEvent) -> Vec<AgentEvent> {
        super::claude::ClaudeCodeRunner::convert_event(raw)
    }
}

impl Default for MakiRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRunner for MakiRunner {
    fn agent_type(&self) -> AgentType {
        AgentType::Maki
    }

    async fn start(&self, config: AgentStartConfig) -> Result<AgentHandle, AgentError> {
        let mut cmd = self.build_command(&config);
        conduit_util::process::configure_command_process_group(&mut cmd);
        let mut child = cmd.spawn()?;

        let pid = child.id().ok_or(AgentError::ProcessSpawnFailed)?;
        let stdout = child.stdout.take().ok_or(AgentError::StdoutCaptureFailed)?;
        let stderr = child.stderr.take();

        let (tx, rx) = mpsc::channel::<AgentEvent>(256);
        let tx_for_monitor = tx.clone();

        tokio::spawn(async move {
            let (raw_tx, mut raw_rx) = mpsc::channel::<ClaudeRawEvent>(256);
            let tx_for_parser = tx.clone();

            let parse_handle = tokio::spawn(async move {
                if let Err(e) = JsonlStreamParser::parse_stream(stdout, raw_tx).await {
                    if let Err(send_err) = tx_for_parser
                        .send(AgentEvent::Error(ErrorEvent {
                            message: format!("Stream parsing error: {}", e),
                            is_fatal: true,
                            code: None,
                            details: None,
                        }))
                        .await
                    {
                        tracing::debug!(
                            error = ?send_err,
                            "Failed to send maki stream parsing error"
                        );
                    }
                }
            });

            while let Some(raw_event) = raw_rx.recv().await {
                for event in Self::convert_event(raw_event) {
                    if tx.send(event).await.is_err() {
                        if let Err(join_err) = parse_handle.await {
                            tracing::warn!(error = ?join_err, "Maki parser task failed to join");
                        }
                        return;
                    }
                }
            }

            if let Err(join_err) = parse_handle.await {
                tracing::warn!(error = ?join_err, "Maki parser task failed to join");
            }
        });

        tokio::spawn(async move {
            use tokio::io::AsyncReadExt;

            let status = child.wait().await;

            let stderr_content = if let Some(mut stderr) = stderr {
                let mut buf = String::new();
                if let Err(e) = stderr.read_to_string(&mut buf).await {
                    tracing::debug!(error = %e, "Failed to read maki stderr");
                }
                buf
            } else {
                String::new()
            };

            match status {
                Ok(exit_status) if !exit_status.success() => {
                    let error_msg = if stderr_content.is_empty() {
                        format!("maki process exited with status: {}", exit_status)
                    } else {
                        format!(
                            "maki process failed ({}): {}",
                            exit_status,
                            stderr_content.trim()
                        )
                    };
                    if let Err(send_err) = tx_for_monitor
                        .send(AgentEvent::Error(ErrorEvent {
                            message: error_msg,
                            is_fatal: true,
                            code: None,
                            details: None,
                        }))
                        .await
                    {
                        tracing::debug!(
                            error = ?send_err,
                            "Failed to send maki process failure"
                        );
                    }
                }
                Err(e) => {
                    if let Err(send_err) = tx_for_monitor
                        .send(AgentEvent::Error(ErrorEvent {
                            message: format!("Failed to wait for maki process: {}", e),
                            is_fatal: true,
                            code: None,
                            details: None,
                        }))
                        .await
                    {
                        tracing::debug!(
                            error = ?send_err,
                            "Failed to send maki wait error"
                        );
                    }
                }
                Ok(_) => {}
            }
        });

        Ok(AgentHandle::new(rx, pid, None))
    }

    async fn send_input(
        &self,
        _handle: &AgentHandle,
        _input: AgentInput,
    ) -> Result<(), AgentError> {
        Err(AgentError::NotSupported(
            "Maki print mode doesn't support interactive input".into(),
        ))
    }

    async fn stop(&self, handle: &AgentHandle) -> Result<(), AgentError> {
        #[cfg(unix)]
        {
            conduit_util::process::signal_process_tree(handle.pid, libc::SIGTERM)
                .map_err(AgentError::Io)?;
        }
        #[cfg(not(unix))]
        {
            let _ = handle;
            return Err(AgentError::NotSupported(
                "Stop not implemented on this platform".into(),
            ));
        }
        Ok(())
    }

    async fn kill(&self, handle: &AgentHandle) -> Result<(), AgentError> {
        #[cfg(unix)]
        {
            conduit_util::process::signal_process_tree(handle.pid, libc::SIGKILL)
                .map_err(AgentError::Io)?;
        }
        #[cfg(not(unix))]
        {
            let _ = handle;
            return Err(AgentError::NotSupported(
                "Kill not implemented on this platform".into(),
            ));
        }
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.binary_path.exists() || Self::find_binary().is_some()
    }

    fn binary_path(&self) -> Option<PathBuf> {
        if self.binary_path.exists() {
            Some(self.binary_path.clone())
        } else {
            Self::find_binary()
        }
    }
}
