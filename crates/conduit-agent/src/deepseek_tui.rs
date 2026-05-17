use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::rc::Rc;
use std::time::Duration;

use agent_client_protocol as acp;
use agent_client_protocol::Agent as _;
use async_trait::async_trait;
use futures::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tokio_util::io::ReaderStream;

use crate::error::AgentError;
use crate::events::{
    AgentEvent, AssistantMessageEvent, ErrorEvent, ReasoningEvent, SessionInitEvent,
    ToolCompletedEvent, ToolStartedEvent, TurnCompletedEvent,
};
use crate::runner::{AgentHandle, AgentInput, AgentRunner, AgentStartConfig, AgentType};
use crate::session::SessionId;

const INIT_TIMEOUT: Duration = Duration::from_secs(10);
const SESSION_TIMEOUT: Duration = Duration::from_secs(10);

pub struct DeepseekTuiRunner {
    binary_path: PathBuf,
}

struct DeepseekAcpClient {
    event_tx: mpsc::Sender<AgentEvent>,
    saw_message: Rc<Cell<bool>>,
    tool_titles: RefCell<HashMap<String, String>>,
}

impl DeepseekAcpClient {
    fn new(event_tx: mpsc::Sender<AgentEvent>, saw_message: Rc<Cell<bool>>) -> Self {
        Self {
            event_tx,
            saw_message,
            tool_titles: RefCell::new(HashMap::new()),
        }
    }

    async fn send_event(&self, event: AgentEvent) {
        if self.event_tx.send(event).await.is_err() {
            tracing::debug!("Failed to send DeepSeek TUI agent event");
        }
    }

    fn store_tool_title(&self, tool_id: &str, title: String) {
        self.tool_titles
            .borrow_mut()
            .insert(tool_id.to_string(), title);
    }

    fn content_to_string(content: &[acp::ToolCallContent]) -> Option<String> {
        let mut parts = Vec::new();
        for item in content {
            match item {
                acp::ToolCallContent::Content(content) => {
                    if let acp::ContentBlock::Text(text) = &content.content {
                        if !text.text.is_empty() {
                            parts.push(text.text.clone());
                        }
                    }
                }
                acp::ToolCallContent::Diff(diff) => {
                    parts.push(format!("Diff applied: {}", diff.path.display()));
                }
                acp::ToolCallContent::Terminal(terminal) => {
                    parts.push(format!(
                        "Terminal output available (id: {})",
                        terminal.terminal_id.0
                    ));
                }
                _ => {}
            }
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n"))
        }
    }
}

#[async_trait(?Send)]
impl acp::Client for DeepseekAcpClient {
    async fn request_permission(
        &self,
        args: acp::RequestPermissionRequest,
    ) -> Result<acp::RequestPermissionResponse, acp::Error> {
        let chosen = args
            .options
            .iter()
            .find(|o| matches!(o.kind, acp::PermissionOptionKind::AllowAlways))
            .or_else(|| {
                args.options
                    .iter()
                    .find(|o| matches!(o.kind, acp::PermissionOptionKind::AllowOnce))
            })
            .or_else(|| args.options.first());

        let outcome = if let Some(option) = chosen {
            acp::RequestPermissionOutcome::Selected(acp::SelectedPermissionOutcome::new(
                option.option_id.clone(),
            ))
        } else {
            acp::RequestPermissionOutcome::Cancelled
        };

        Ok(acp::RequestPermissionResponse::new(outcome))
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> Result<(), acp::Error> {
        match args.update {
            acp::SessionUpdate::AgentMessageChunk(chunk) => {
                if let acp::ContentBlock::Text(text) = chunk.content {
                    self.saw_message.set(true);
                    self.send_event(AgentEvent::AssistantMessage(AssistantMessageEvent {
                        text: text.text,
                        is_final: false,
                    }))
                    .await;
                }
            }
            acp::SessionUpdate::AgentThoughtChunk(chunk) => {
                if let acp::ContentBlock::Text(text) = chunk.content {
                    self.send_event(AgentEvent::AssistantReasoning(ReasoningEvent {
                        text: text.text,
                    }))
                    .await;
                }
            }
            acp::SessionUpdate::ToolCall(tool_call) => {
                let tool_id = tool_call.tool_call_id.0.to_string();
                let title = if tool_call.title.is_empty() {
                    "tool".to_string()
                } else {
                    tool_call.title.clone()
                };
                self.store_tool_title(&tool_id, title.clone());
                let arguments = tool_call.raw_input.unwrap_or(serde_json::Value::Null);
                self.send_event(AgentEvent::ToolStarted(ToolStartedEvent {
                    tool_name: title,
                    tool_id,
                    arguments,
                }))
                .await;
            }
            acp::SessionUpdate::ToolCallUpdate(update) => {
                let tool_id = update.tool_call_id.0.to_string();
                if let Some(title) = update.fields.title.clone() {
                    self.store_tool_title(&tool_id, title);
                }
                if let Some(acp::ToolCallStatus::Completed) = update.fields.status {
                    let result = update
                        .fields
                        .content
                        .as_ref()
                        .and_then(|content| Self::content_to_string(content))
                        .or_else(|| update.fields.raw_output.as_ref().map(|v| v.to_string()));
                    self.send_event(AgentEvent::ToolCompleted(ToolCompletedEvent {
                        tool_id,
                        success: true,
                        result,
                        error: None,
                    }))
                    .await;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn write_text_file(
        &self,
        _: acp::WriteTextFileRequest,
    ) -> Result<acp::WriteTextFileResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn read_text_file(
        &self,
        _: acp::ReadTextFileRequest,
    ) -> Result<acp::ReadTextFileResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn create_terminal(
        &self,
        _: acp::CreateTerminalRequest,
    ) -> Result<acp::CreateTerminalResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn terminal_output(
        &self,
        _: acp::TerminalOutputRequest,
    ) -> Result<acp::TerminalOutputResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn release_terminal(
        &self,
        _: acp::ReleaseTerminalRequest,
    ) -> Result<acp::ReleaseTerminalResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn wait_for_terminal_exit(
        &self,
        _: acp::WaitForTerminalExitRequest,
    ) -> Result<acp::WaitForTerminalExitResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn kill_terminal_command(
        &self,
        _: acp::KillTerminalCommandRequest,
    ) -> Result<acp::KillTerminalCommandResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn ext_method(&self, _: acp::ExtRequest) -> Result<acp::ExtResponse, acp::Error> {
        Err(acp::Error::method_not_found())
    }

    async fn ext_notification(&self, _: acp::ExtNotification) -> Result<(), acp::Error> {
        Ok(())
    }
}

impl DeepseekTuiRunner {
    pub fn new() -> Self {
        Self {
            binary_path: Self::find_binary().unwrap_or_else(|| PathBuf::from("deepseek")),
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { binary_path: path }
    }

    fn find_binary() -> Option<PathBuf> {
        which::which("deepseek").ok()
    }

    fn build_command(&self, config: &AgentStartConfig) -> Command {
        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("serve").arg("--acp");
        if let Some(model) = &config.model {
            cmd.arg("--model").arg(model);
        }
        for arg in &config.additional_args {
            cmd.arg(arg);
        }
        cmd.current_dir(&config.working_dir);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.env("NO_COLOR", "1");
        cmd
    }
}

impl Default for DeepseekTuiRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRunner for DeepseekTuiRunner {
    fn agent_type(&self) -> AgentType {
        AgentType::DeepseekTui
    }

    async fn start(&self, config: AgentStartConfig) -> Result<AgentHandle, AgentError> {
        if !config.images.is_empty() {
            return Err(AgentError::NotSupported(
                "DeepSeek TUI ACP runner does not support image attachments".to_string(),
            ));
        }
        if config.resume_session.is_some() {
            tracing::debug!(
                "DeepSeek TUI ACP does not support session resume; starting new session"
            );
        }

        let mut cmd = self.build_command(&config);
        conduit_util::process::configure_command_process_group(&mut cmd);
        let mut child = cmd.spawn()?;

        let pid = child.id().ok_or(AgentError::ProcessSpawnFailed)?;
        let stdout = child.stdout.take().ok_or(AgentError::StdoutCaptureFailed)?;
        let stderr = child.stderr.take();
        let child_stdin = child.stdin.take().ok_or_else(|| {
            AgentError::Config("Failed to capture stdin for DeepSeek TUI".to_string())
        })?;

        let (tx, rx) = mpsc::channel::<AgentEvent>(256);
        let tx_for_monitor = tx.clone();

        let (mut to_acp_writer, acp_incoming_reader) = tokio::io::duplex(64 * 1024);
        let (acp_out_writer, acp_out_reader) = tokio::io::duplex(64 * 1024);

        tokio::spawn(async move {
            let mut stdout_stream = ReaderStream::new(stdout);
            while let Some(res) = stdout_stream.next().await {
                match res {
                    Ok(data) => {
                        if to_acp_writer.write_all(&data).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        tokio::spawn(async move {
            let mut child_stdin = child_stdin;
            let mut reader = ReaderStream::new(acp_out_reader);
            while let Some(res) = reader.next().await {
                match res {
                    Ok(data) => {
                        if child_stdin.write_all(&data).await.is_err() {
                            break;
                        }
                        let _ = child_stdin.flush().await;
                    }
                    Err(_) => break,
                }
            }
        });

        let outgoing = acp_out_writer.compat_write();
        let incoming = acp_incoming_reader.compat();
        let prompt = config.prompt.clone();
        let working_dir = config.working_dir.clone();
        let tx_for_session = tx.clone();

        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build DeepSeek TUI ACP runtime");
            rt.block_on(async move {
                let local = tokio::task::LocalSet::new();
                local
                    .run_until(async move {
                        let saw_message = Rc::new(Cell::new(false));
                        let client =
                            DeepseekAcpClient::new(tx_for_session.clone(), saw_message.clone());
                        let (conn, io_fut) =
                            acp::ClientSideConnection::new(client, outgoing, incoming, |fut| {
                                tokio::task::spawn_local(fut);
                            });
                        let conn = Rc::new(conn);

                        tokio::task::spawn_local(async move {
                            let _ = io_fut.await;
                        });

                        let init_result = tokio::time::timeout(
                            INIT_TIMEOUT,
                            conn.initialize(acp::InitializeRequest::new(acp::ProtocolVersion::V1)),
                        )
                        .await;

                        match init_result {
                            Ok(Ok(_)) => {}
                            Ok(Err(err)) => {
                                let _ = tx_for_session
                                    .send(AgentEvent::Error(ErrorEvent {
                                        message: format!(
                                            "Failed to initialize DeepSeek TUI ACP: {}",
                                            err
                                        ),
                                        is_fatal: true,
                                        code: None,
                                        details: None,
                                    }))
                                    .await;
                                return;
                            }
                            Err(_) => {
                                let _ = tx_for_session
                                    .send(AgentEvent::Error(ErrorEvent {
                                        message: "Timed out initializing DeepSeek TUI ACP."
                                            .to_string(),
                                        is_fatal: true,
                                        code: None,
                                        details: None,
                                    }))
                                    .await;
                                return;
                            }
                        }

                        let session_id = match tokio::time::timeout(
                            SESSION_TIMEOUT,
                            conn.new_session(acp::NewSessionRequest::new(working_dir)),
                        )
                        .await
                        {
                            Ok(Ok(response)) => response.session_id,
                            Ok(Err(err)) => {
                                let _ = tx_for_session
                                    .send(AgentEvent::Error(ErrorEvent {
                                        message: format!(
                                            "Failed to create DeepSeek TUI session: {}",
                                            err
                                        ),
                                        is_fatal: true,
                                        code: None,
                                        details: None,
                                    }))
                                    .await;
                                return;
                            }
                            Err(_) => {
                                let _ = tx_for_session
                                    .send(AgentEvent::Error(ErrorEvent {
                                        message: "Timed out creating DeepSeek TUI session."
                                            .to_string(),
                                        is_fatal: true,
                                        code: None,
                                        details: None,
                                    }))
                                    .await;
                                return;
                            }
                        };

                        let _ = tx_for_session
                            .send(AgentEvent::SessionInit(SessionInitEvent {
                                session_id: SessionId::from_string(session_id.0.to_string()),
                                model: None,
                            }))
                            .await;

                        let _ = tx_for_session.send(AgentEvent::TurnStarted).await;

                        if !prompt.is_empty() {
                            let req = acp::PromptRequest::new(
                                session_id.clone(),
                                vec![acp::ContentBlock::Text(acp::TextContent::new(prompt))],
                            );
                            if let Err(err) = conn.prompt(req).await {
                                let _ = tx_for_session
                                    .send(AgentEvent::Error(ErrorEvent {
                                        message: format!("DeepSeek TUI prompt failed: {}", err),
                                        is_fatal: true,
                                        code: None,
                                        details: None,
                                    }))
                                    .await;
                                return;
                            }
                        }

                        if saw_message.get() {
                            let _ = tx_for_session
                                .send(AgentEvent::AssistantMessage(AssistantMessageEvent {
                                    text: String::new(),
                                    is_final: true,
                                }))
                                .await;
                        }

                        let _ = tx_for_session
                            .send(AgentEvent::TurnCompleted(TurnCompletedEvent {
                                usage: Default::default(),
                            }))
                            .await;

                        let _ = conn.cancel(acp::CancelNotification::new(session_id)).await;
                    })
                    .await;
            });
        });

        tokio::spawn(async move {
            use tokio::io::AsyncReadExt;

            let status = child.wait().await;
            let stderr_content = if let Some(mut stderr) = stderr {
                let mut buf = String::new();
                if let Err(err) = stderr.read_to_string(&mut buf).await {
                    tracing::debug!(error = %err, "Failed to read DeepSeek TUI stderr");
                }
                buf
            } else {
                String::new()
            };

            match status {
                Ok(exit_status) if !exit_status.success() => {
                    let message = if stderr_content.is_empty() {
                        format!("DeepSeek TUI process exited with status: {}", exit_status)
                    } else {
                        format!(
                            "DeepSeek TUI process failed ({}): {}",
                            exit_status,
                            stderr_content.trim()
                        )
                    };
                    let _ = tx_for_monitor
                        .send(AgentEvent::Error(ErrorEvent {
                            message,
                            is_fatal: true,
                            code: None,
                            details: None,
                        }))
                        .await;
                }
                Err(err) => {
                    let _ = tx_for_monitor
                        .send(AgentEvent::Error(ErrorEvent {
                            message: format!("Failed to wait for DeepSeek TUI process: {}", err),
                            is_fatal: true,
                            code: None,
                            details: None,
                        }))
                        .await;
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
            "DeepSeek TUI ACP runner does not support interactive input".into(),
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
