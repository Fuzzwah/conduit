use std::path::PathBuf;
use std::process::Stdio;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::error::AgentError;
use crate::events::{
    AgentEvent, AssistantMessageEvent, ErrorEvent, TokenUsage, TurnCompletedEvent, TurnFailedEvent,
};
use crate::runner::{AgentHandle, AgentInput, AgentRunner, AgentStartConfig, AgentType};

pub struct CecliRunner {
    binary_path: PathBuf,
}

impl CecliRunner {
    pub fn new() -> Self {
        Self {
            binary_path: Self::find_binary().unwrap_or_else(|| PathBuf::from("cecli")),
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { binary_path: path }
    }

    fn find_binary() -> Option<PathBuf> {
        which::which("cecli")
            .ok()
            .or_else(|| which::which("aider-ce").ok())
    }
}

impl Default for CecliRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRunner for CecliRunner {
    fn agent_type(&self) -> AgentType {
        AgentType::Cecli
    }

    async fn start(&self, config: AgentStartConfig) -> Result<AgentHandle, AgentError> {
        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("--yes-always");
        cmd.arg("--no-pretty");
        cmd.arg("--stream");
        cmd.arg("--no-auto-commits");
        cmd.arg("--message").arg(&config.prompt);
        if let Some(model) = &config.model {
            if !model.trim().is_empty() && model != "default" {
                cmd.arg("--model").arg(model);
            }
        }
        cmd.current_dir(&config.working_dir);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        conduit_util::process::configure_command_process_group(&mut cmd);

        let mut child = cmd.spawn().map_err(|_| AgentError::ProcessSpawnFailed)?;
        let pid = child.id().ok_or(AgentError::ProcessSpawnFailed)?;
        let stdout = child.stdout.take().ok_or(AgentError::StdoutCaptureFailed)?;
        let stderr = child.stderr.take();

        let (tx, rx) = mpsc::channel::<AgentEvent>(256);
        let tx_monitor = tx.clone();

        tokio::spawn(async move {
            let _ = tx.send(AgentEvent::TurnStarted).await;

            let stderr_task = tokio::spawn(async move {
                let mut buf = String::new();
                if let Some(mut stderr) = stderr {
                    if let Err(e) = stderr.read_to_string(&mut buf).await {
                        tracing::debug!(error = %e, "Failed to read cecli stderr");
                    }
                }
                buf
            });

            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let text = line + "\n";
                if tx
                    .send(AgentEvent::AssistantMessage(AssistantMessageEvent {
                        text,
                        is_final: false,
                    }))
                    .await
                    .is_err()
                {
                    return;
                }
            }

            let _ = tx
                .send(AgentEvent::AssistantMessage(AssistantMessageEvent {
                    text: String::new(),
                    is_final: true,
                }))
                .await;

            let stderr_content = stderr_task.await.unwrap_or_default();

            match child.wait().await {
                Ok(status) if status.success() => {
                    let _ = tx
                        .send(AgentEvent::TurnCompleted(TurnCompletedEvent {
                            usage: TokenUsage::default(),
                        }))
                        .await;
                }
                Ok(status) => {
                    let error_msg = if stderr_content.is_empty() {
                        format!("cecli exited with status: {}", status)
                    } else {
                        format!("cecli exited with status {}: {}", status, stderr_content.trim())
                    };
                    let _ = tx
                        .send(AgentEvent::TurnFailed(TurnFailedEvent { error: error_msg }))
                        .await;
                }
                Err(e) => {
                    let _ = tx_monitor
                        .send(AgentEvent::Error(ErrorEvent {
                            message: format!("Failed to wait for cecli process: {}", e),
                            is_fatal: true,
                            code: None,
                            details: None,
                        }))
                        .await;
                }
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
            "CE CLI does not support interactive input in conduit".into(),
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
