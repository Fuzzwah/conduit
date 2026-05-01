use std::path::PathBuf;
use std::process::Stdio;

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::agent::error::AgentError;
use crate::agent::events::{
    AgentEvent, AssistantMessageEvent, CommandOutputEvent, ErrorEvent, ReasoningEvent,
    SessionInitEvent, TokenUsage, TokenUsageEvent, ToolCompletedEvent, ToolStartedEvent,
    TurnCompletedEvent, TurnFailedEvent,
};
use crate::agent::runner::{
    AgentHandle, AgentInput, AgentMode, AgentRunner, AgentStartConfig, AgentType,
};
use crate::agent::session::SessionId;

#[derive(Debug, Default, Clone)]
struct DiracUsageAccumulator {
    input_tokens: i64,
    output_tokens: i64,
    cached_tokens: i64,
    total_tokens: i64,
}

impl DiracUsageAccumulator {
    fn update_from_metrics(&mut self, metrics: &Value) -> Option<TokenUsageEvent> {
        let input_tokens = metrics.get("tokensIn").and_then(Value::as_i64).unwrap_or(0);
        let output_tokens = metrics
            .get("tokensOut")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let reasoning_tokens = metrics
            .get("reasoningTokens")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let cached_tokens = metrics
            .get("cacheReads")
            .and_then(Value::as_i64)
            .unwrap_or(0);

        self.input_tokens += input_tokens;
        self.output_tokens += output_tokens + reasoning_tokens;
        self.cached_tokens += cached_tokens;
        self.total_tokens = self.input_tokens + self.output_tokens;

        let context_window = metrics.get("contextWindow").and_then(Value::as_i64);
        let usage_percent = metrics
            .get("contextUsagePercentage")
            .and_then(Value::as_f64)
            .map(|pct| (pct / 100.0) as f32);

        Some(TokenUsageEvent {
            usage: self.token_usage(),
            context_window,
            usage_percent,
        })
    }

    fn token_usage(&self) -> TokenUsage {
        TokenUsage {
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            cached_tokens: self.cached_tokens,
            total_tokens: self.total_tokens,
        }
    }
}

pub struct DiracRunner {
    binary_path: PathBuf,
}

impl DiracRunner {
    pub fn new() -> Self {
        Self {
            binary_path: Self::find_binary().unwrap_or_else(|| PathBuf::from("dirac")),
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { binary_path: path }
    }

    fn find_binary() -> Option<PathBuf> {
        which::which("dirac").ok()
    }

    fn build_command(&self, config: &AgentStartConfig) -> Command {
        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("--json");
        cmd.arg("--yolo");
        cmd.arg("--cwd").arg(&config.working_dir);

        if config.agent_mode == AgentMode::Plan {
            cmd.arg("--plan");
        }

        if let Some(timeout_ms) = config.timeout_ms {
            let timeout_secs = (timeout_ms.saturating_add(999) / 1000).max(1);
            cmd.arg("--timeout").arg(timeout_secs.to_string());
        }

        if let Some(model) = &config.model {
            cmd.arg("--model").arg(model);
        }

        if !config.images.is_empty() {
            cmd.arg("--images");
            for image in &config.images {
                cmd.arg(image);
            }
        }

        if let Some(session_id) = &config.resume_session {
            cmd.arg("--taskId").arg(session_id.as_str());
        }

        for arg in &config.additional_args {
            cmd.arg(arg);
        }

        if !config.prompt.trim().is_empty() {
            cmd.arg(&config.prompt);
        }

        cmd.current_dir(&config.working_dir);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.env("NO_COLOR", "1");
        cmd
    }

    async fn complete_pending_tool(
        tx: &mpsc::Sender<AgentEvent>,
        pending_tool: &mut Option<(String, String)>,
    ) -> Result<(), AgentError> {
        if let Some((tool_id, _)) = pending_tool.take() {
            tx.send(AgentEvent::ToolCompleted(ToolCompletedEvent {
                tool_id,
                success: true,
                result: None,
                error: None,
            }))
            .await
            .map_err(|_| AgentError::ChannelClosed)?;
        }
        Ok(())
    }

    fn next_tool_id(kind: &str, next_tool_id: &mut u64) -> String {
        let tool_id = format!("dirac-{kind}-{}", *next_tool_id);
        *next_tool_id += 1;
        tool_id
    }

    async fn handle_json_line(
        value: Value,
        tx: &mpsc::Sender<AgentEvent>,
        usage: &mut DiracUsageAccumulator,
        pending_tool: &mut Option<(String, String)>,
        next_tool_id: &mut u64,
    ) -> Result<(), AgentError> {
        let event_type = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match event_type {
            "task_started" => {
                if let Some(task_id) = value.get("taskId").and_then(Value::as_str) {
                    tx.send(AgentEvent::SessionInit(SessionInitEvent {
                        session_id: SessionId::from_string(task_id),
                        model: None,
                    }))
                    .await
                    .map_err(|_| AgentError::ChannelClosed)?;
                }
            }
            "error" => {
                let message = value
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Dirac reported an error")
                    .to_string();
                tx.send(AgentEvent::Error(ErrorEvent {
                    message,
                    is_fatal: false,
                    code: None,
                    details: Some(value),
                }))
                .await
                .map_err(|_| AgentError::ChannelClosed)?;
            }
            "say" | "ask" => {
                let say = value.get("say").and_then(Value::as_str);
                let ask = value.get("ask").and_then(Value::as_str);
                let text = value
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let partial = value
                    .get("partial")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);

                if partial {
                    return Ok(());
                }

                if let Some(reasoning) = value.get("reasoning").and_then(Value::as_str) {
                    if !reasoning.is_empty() {
                        tx.send(AgentEvent::AssistantReasoning(ReasoningEvent {
                            text: reasoning.to_string(),
                        }))
                        .await
                        .map_err(|_| AgentError::ChannelClosed)?;
                    }
                } else if say == Some("reasoning") && !text.is_empty() {
                    tx.send(AgentEvent::AssistantReasoning(ReasoningEvent {
                        text: text.clone(),
                    }))
                    .await
                    .map_err(|_| AgentError::ChannelClosed)?;
                }

                match (say, ask) {
                    (Some("text"), _) => {
                        Self::complete_pending_tool(tx, pending_tool).await?;
                        if !text.is_empty() {
                            tx.send(AgentEvent::AssistantMessage(AssistantMessageEvent {
                                text,
                                is_final: true,
                            }))
                            .await
                            .map_err(|_| AgentError::ChannelClosed)?;
                        }
                    }
                    (Some("completion_result"), _) | (_, Some("completion_result")) => {
                        Self::complete_pending_tool(tx, pending_tool).await?;
                        if !text.is_empty() {
                            tx.send(AgentEvent::AssistantMessage(AssistantMessageEvent {
                                text,
                                is_final: true,
                            }))
                            .await
                            .map_err(|_| AgentError::ChannelClosed)?;
                        }
                    }
                    (Some("command"), _) => {
                        Self::complete_pending_tool(tx, pending_tool).await?;
                        let tool_id = Self::next_tool_id("command", next_tool_id);
                        tx.send(AgentEvent::ToolStarted(ToolStartedEvent {
                            tool_name: "command".to_string(),
                            tool_id: tool_id.clone(),
                            arguments: json!({ "command": text }),
                        }))
                        .await
                        .map_err(|_| AgentError::ChannelClosed)?;
                        *pending_tool = Some((tool_id, text));
                    }
                    (Some("command_output"), _) => {
                        let command = pending_tool
                            .as_ref()
                            .map(|(_, command)| command.clone())
                            .unwrap_or_default();
                        tx.send(AgentEvent::CommandOutput(CommandOutputEvent {
                            command,
                            output: text,
                            exit_code: None,
                            is_streaming: false,
                        }))
                        .await
                        .map_err(|_| AgentError::ChannelClosed)?;
                    }
                    (Some("tool"), _) => {
                        Self::complete_pending_tool(tx, pending_tool).await?;
                        let tool_id = Self::next_tool_id("tool", next_tool_id);
                        tx.send(AgentEvent::ToolStarted(ToolStartedEvent {
                            tool_name: "tool".to_string(),
                            tool_id: tool_id.clone(),
                            arguments: json!({ "description": text }),
                        }))
                        .await
                        .map_err(|_| AgentError::ChannelClosed)?;
                        *pending_tool = Some((tool_id, String::new()));
                    }
                    (Some("api_req_finished"), _) => {
                        let metrics = serde_json::from_str::<Value>(&text).unwrap_or(Value::Null);
                        if let Some(usage_event) = usage.update_from_metrics(&metrics) {
                            tx.send(AgentEvent::TokenUsage(usage_event))
                                .await
                                .map_err(|_| AgentError::ChannelClosed)?;
                        }
                    }
                    (Some("error"), _) | (_, Some("api_req_failed")) => {
                        Self::complete_pending_tool(tx, pending_tool).await?;
                        if !text.is_empty() {
                            tx.send(AgentEvent::TurnFailed(TurnFailedEvent { error: text }))
                                .await
                                .map_err(|_| AgentError::ChannelClosed)?;
                        }
                    }
                    _ => {}
                }
            }
            _ => {
                tx.send(AgentEvent::Raw { data: value })
                    .await
                    .map_err(|_| AgentError::ChannelClosed)?;
            }
        }

        Ok(())
    }
}

impl Default for DiracRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRunner for DiracRunner {
    fn agent_type(&self) -> AgentType {
        AgentType::Dirac
    }

    async fn start(&self, config: AgentStartConfig) -> Result<AgentHandle, AgentError> {
        let mut cmd = self.build_command(&config);
        crate::util::process::configure_command_process_group(&mut cmd);
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
                    if let Err(err) = stderr.read_to_string(&mut buf).await {
                        tracing::debug!(error = %err, "Failed to read dirac stderr");
                    }
                }
                buf
            });

            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut usage = DiracUsageAccumulator::default();
            let mut pending_tool: Option<(String, String)> = None;
            let mut next_tool_id: u64 = 1;

            while let Ok(Some(line)) = lines.next_line().await {
                match serde_json::from_str::<Value>(&line) {
                    Ok(value) => {
                        if let Err(err) = Self::handle_json_line(
                            value,
                            &tx,
                            &mut usage,
                            &mut pending_tool,
                            &mut next_tool_id,
                        )
                        .await
                        {
                            tracing::debug!(error = %err, "Failed to forward dirac event");
                            return;
                        }
                    }
                    Err(err) => {
                        let _ = tx
                            .send(AgentEvent::Raw {
                                data: json!({
                                    "type": "dirac_unparsed_line",
                                    "line": line,
                                    "error": err.to_string(),
                                }),
                            })
                            .await;
                    }
                }
            }

            let _ = Self::complete_pending_tool(&tx, &mut pending_tool).await;

            let stderr_content = stderr_task.await.unwrap_or_default();
            match child.wait().await {
                Ok(status) if status.success() => {
                    let _ = tx
                        .send(AgentEvent::TurnCompleted(TurnCompletedEvent {
                            usage: usage.token_usage(),
                        }))
                        .await;
                }
                Ok(status) => {
                    let error_msg = if stderr_content.trim().is_empty() {
                        format!("dirac exited with status: {}", status)
                    } else {
                        format!(
                            "dirac exited with status {}: {}",
                            status,
                            stderr_content.trim()
                        )
                    };
                    let _ = tx
                        .send(AgentEvent::TurnFailed(TurnFailedEvent { error: error_msg }))
                        .await;
                }
                Err(err) => {
                    let _ = tx_monitor
                        .send(AgentEvent::Error(ErrorEvent {
                            message: format!("Failed to wait for dirac process: {}", err),
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
            "Dirac follow-up input is handled by starting a new turn".into(),
        ))
    }

    async fn stop(&self, handle: &AgentHandle) -> Result<(), AgentError> {
        #[cfg(unix)]
        {
            crate::util::process::signal_process_tree(handle.pid, libc::SIGTERM)
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
            crate::util::process::signal_process_tree(handle.pid, libc::SIGKILL)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirac_usage_accumulates_metrics() {
        let mut usage = DiracUsageAccumulator::default();
        let event = usage
            .update_from_metrics(&json!({
                "tokensIn": 120,
                "tokensOut": 40,
                "reasoningTokens": 10,
                "cacheReads": 5,
                "contextWindow": 200000,
                "contextUsagePercentage": 25.0
            }))
            .expect("usage event");

        assert_eq!(event.usage.input_tokens, 120);
        assert_eq!(event.usage.output_tokens, 50);
        assert_eq!(event.usage.cached_tokens, 5);
        assert_eq!(event.usage.total_tokens, 170);
        assert_eq!(event.context_window, Some(200000));
        assert_eq!(event.usage_percent, Some(0.25));
    }
}
