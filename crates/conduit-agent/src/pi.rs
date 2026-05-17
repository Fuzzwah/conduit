use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::error::AgentError;
use crate::events::{
    AgentEvent, AssistantMessageEvent, CommandOutputEvent, ContextCompactionEvent, ErrorEvent,
    ReasoningEvent, SessionInitEvent, TokenUsage, ToolCompletedEvent, ToolStartedEvent,
    TurnCompletedEvent, TurnFailedEvent,
};
use crate::runner::{AgentHandle, AgentInput, AgentRunner, AgentStartConfig, AgentType};
use crate::session::SessionId;

const PI_RPC_TIMEOUT: Duration = Duration::from_secs(15);

struct PiRpcCommand {
    id: Option<String>,
    payload: Value,
    response_tx: Option<oneshot::Sender<Value>>,
}

// ---------------------------------------------------------------------------
// Extension & orchestration helpers
// ---------------------------------------------------------------------------

/// Base directory for the Pi agent extension file.
fn pi_extension_base_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".conduit").join("pi-agent-extensions"))
}

/// Embedded TypeScript source for the Agent tool extension.
const AGENT_EXTENSION_SOURCE: &str = include_str!("agent-tool.ts");

/// Write the Agent tool extension file to disk and return its path.
fn write_agent_extension() -> Result<PathBuf, io::Error> {
    let base = pi_extension_base_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "could not determine home directory",
        )
    })?;
    fs::create_dir_all(&base)?;
    let path = base.join("agent-tool.ts");
    fs::write(&path, AGENT_EXTENSION_SOURCE)?;
    Ok(path)
}

/// Pi skill directory base (where conduit-* skills are written).
fn pi_skills_base_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".pi").join("agent").join("skills"))
}

/// Paths to the three orchestration skill directories.
fn pi_skill_dirs() -> Vec<PathBuf> {
    let base = match pi_skills_base_dir() {
        Some(b) => b,
        None => return vec![],
    };
    vec![
        base.join("conduit-explore"),
        base.join("conduit-review"),
        base.join("conduit-adversarial-review"),
    ]
}

pub struct PiRunner {
    binary_path: PathBuf,
}

impl PiRunner {
    pub fn new() -> Self {
        Self {
            binary_path: Self::find_binary().unwrap_or_else(|| PathBuf::from("pi")),
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { binary_path: path }
    }

    fn find_binary() -> Option<PathBuf> {
        which::which("pi").ok()
    }

    fn build_command(&self, config: &AgentStartConfig) -> Command {
        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("--mode").arg("rpc");
        if let Some(session_id) = &config.resume_session {
            cmd.arg("--session").arg(session_id.as_str());
        }
        if let Some(model) = &config.model {
            cmd.arg("--model").arg(model);
        }
        cmd.current_dir(&config.working_dir);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd
    }

    async fn request(
        command_tx: &mpsc::Sender<PiRpcCommand>,
        request_id: &str,
        payload: Value,
    ) -> Result<Value, AgentError> {
        let (response_tx, response_rx) = oneshot::channel();
        command_tx
            .send(PiRpcCommand {
                id: Some(request_id.to_string()),
                payload,
                response_tx: Some(response_tx),
            })
            .await
            .map_err(|_| AgentError::ChannelClosed)?;

        tokio::time::timeout(PI_RPC_TIMEOUT, response_rx)
            .await
            .map_err(|_| AgentError::Timeout(PI_RPC_TIMEOUT.as_millis() as u64))?
            .map_err(|_| AgentError::ChannelClosed)
    }

    async fn send_prompt_command(
        command_tx: &mpsc::Sender<PiRpcCommand>,
        event_tx: &mpsc::Sender<AgentEvent>,
        request_id: &str,
        message: String,
    ) -> Result<(), AgentError> {
        if message.trim().is_empty() {
            return Ok(());
        }

        event_tx
            .send(AgentEvent::TurnStarted)
            .await
            .map_err(|_| AgentError::ChannelClosed)?;

        let response = Self::request(
            command_tx,
            request_id,
            json!({
                "id": request_id,
                "type": "prompt",
                "message": message,
            }),
        )
        .await?;

        if !response
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let error = response
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("Pi rejected the prompt")
                .to_string();
            event_tx
                .send(AgentEvent::TurnFailed(TurnFailedEvent { error }))
                .await
                .map_err(|_| AgentError::ChannelClosed)?;
        }

        Ok(())
    }

    fn extract_text_content(value: Option<&Value>) -> String {
        let Some(value) = value else {
            return String::new();
        };
        match value {
            Value::String(text) => text.clone(),
            Value::Array(items) => items
                .iter()
                .filter_map(|item| {
                    item.get("text")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                })
                .collect::<Vec<_>>()
                .join("\n"),
            _ => String::new(),
        }
    }

    fn extract_model_id(state: &Value) -> Option<String> {
        let model = state.get("data")?.get("model")?;
        model
            .get("id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| {
                model
                    .get("modelId")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
    }

    async fn convert_event(
        value: Value,
        event_tx: &mpsc::Sender<AgentEvent>,
    ) -> Result<(), AgentError> {
        let event_type = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match event_type {
            "message_update" => {
                let Some(assistant_event) = value.get("assistantMessageEvent") else {
                    return Ok(());
                };
                match assistant_event
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                {
                    "text_delta" => {
                        let text = assistant_event
                            .get("delta")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                        if !text.is_empty() {
                            event_tx
                                .send(AgentEvent::AssistantMessage(AssistantMessageEvent {
                                    text,
                                    is_final: false,
                                }))
                                .await
                                .map_err(|_| AgentError::ChannelClosed)?;
                        }
                    }
                    "thinking_delta" => {
                        let text = assistant_event
                            .get("delta")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                        if !text.is_empty() {
                            event_tx
                                .send(AgentEvent::AssistantReasoning(ReasoningEvent { text }))
                                .await
                                .map_err(|_| AgentError::ChannelClosed)?;
                        }
                    }
                    _ => {}
                }
            }
            "message_end" => {
                if value
                    .get("message")
                    .and_then(|message| message.get("role"))
                    .and_then(Value::as_str)
                    == Some("assistant")
                {
                    event_tx
                        .send(AgentEvent::AssistantMessage(AssistantMessageEvent {
                            text: String::new(),
                            is_final: true,
                        }))
                        .await
                        .map_err(|_| AgentError::ChannelClosed)?;
                }
            }
            "tool_execution_start" => {
                event_tx
                    .send(AgentEvent::ToolStarted(ToolStartedEvent {
                        tool_name: value
                            .get("toolName")
                            .and_then(Value::as_str)
                            .unwrap_or("tool")
                            .to_string(),
                        tool_id: value
                            .get("toolCallId")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        arguments: value.get("args").cloned().unwrap_or(Value::Null),
                    }))
                    .await
                    .map_err(|_| AgentError::ChannelClosed)?;
            }
            "tool_execution_update" => {
                // Per RPC protocol docs, ALL tools emit execution updates with
                // partialResult.content in the same format. Don't restrict to bash only.
                let tool_name = value
                    .get("toolName")
                    .and_then(Value::as_str)
                    .unwrap_or("tool");
                let output = Self::extract_text_content(
                    value
                        .get("partialResult")
                        .and_then(|result| result.get("content")),
                );
                if !output.is_empty() {
                    event_tx
                        .send(AgentEvent::CommandOutput(CommandOutputEvent {
                            command: if tool_name == "bash" {
                                value
                                    .get("args")
                                    .and_then(|args| args.get("command"))
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .to_string()
                            } else {
                                tool_name.to_string()
                            },
                            output,
                            exit_code: None,
                            is_streaming: true,
                        }))
                        .await
                        .map_err(|_| AgentError::ChannelClosed)?;
                }
            }
            "tool_execution_end" => {
                let result_text = Self::extract_text_content(
                    value.get("result").and_then(|result| result.get("content")),
                );
                let is_error = value
                    .get("isError")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                event_tx
                    .send(AgentEvent::ToolCompleted(ToolCompletedEvent {
                        tool_id: value
                            .get("toolCallId")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        success: !is_error,
                        result: (!result_text.is_empty()).then_some(result_text.clone()),
                        error: is_error.then_some(if result_text.is_empty() {
                            "Pi tool execution failed".to_string()
                        } else {
                            result_text
                        }),
                    }))
                    .await
                    .map_err(|_| AgentError::ChannelClosed)?;
            }
            "compaction_end" => {
                if let Some(result) = value.get("result") {
                    event_tx
                        .send(AgentEvent::ContextCompaction(ContextCompactionEvent {
                            reason: value
                                .get("reason")
                                .and_then(Value::as_str)
                                .unwrap_or("manual")
                                .to_string(),
                            tokens_before: result
                                .get("tokensBefore")
                                .and_then(Value::as_i64)
                                .unwrap_or_default(),
                            tokens_after: 0,
                        }))
                        .await
                        .map_err(|_| AgentError::ChannelClosed)?;
                }
            }
            "agent_end" => {
                event_tx
                    .send(AgentEvent::TurnCompleted(TurnCompletedEvent {
                        usage: TokenUsage::default(),
                    }))
                    .await
                    .map_err(|_| AgentError::ChannelClosed)?;
            }
            "extension_error" => {
                event_tx
                    .send(AgentEvent::Error(ErrorEvent {
                        message: value
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("Pi extension error")
                            .to_string(),
                        is_fatal: false,
                        code: None,
                        details: Some(value),
                    }))
                    .await
                    .map_err(|_| AgentError::ChannelClosed)?;
            }
            _ => {
                event_tx
                    .send(AgentEvent::Raw { data: value })
                    .await
                    .map_err(|_| AgentError::ChannelClosed)?;
            }
        }
        Ok(())
    }
}

impl Default for PiRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRunner for PiRunner {
    fn agent_type(&self) -> AgentType {
        AgentType::Pi
    }

    async fn start(&self, config: AgentStartConfig) -> Result<AgentHandle, AgentError> {
        if config.orchestration_enabled {
            // Write Pi-native skill files
            if let Err(err) = crate::orchestration::ensure_pi_orchestration_skills(
                config.adversarial_review.clone(),
            ) {
                tracing::warn!(error = %err, "Failed to write Pi orchestration skills");
            }
        }

        let mut cmd = self.build_command(&config);
        conduit_util::process::configure_command_process_group(&mut cmd);

        if config.orchestration_enabled {
            // Write and load the Agent tool extension
            if let Ok(ext_path) = write_agent_extension() {
                cmd.arg("--extension").arg(ext_path);
            }
            // Add skill directories
            for skill_dir in pi_skill_dirs() {
                if skill_dir.exists() {
                    cmd.arg("--skill").arg(skill_dir);
                }
            }
            // Inject orchestration instructions into system prompt
            cmd.arg("--append-system-prompt")
                .arg(crate::orchestration::orchestration_instructions());
        }

        let mut child = cmd.spawn().map_err(|_| AgentError::ProcessSpawnFailed)?;
        let pid = child.id().ok_or(AgentError::ProcessSpawnFailed)?;
        let stdin = child.stdin.take().ok_or(AgentError::ProcessSpawnFailed)?;
        let stdout = child.stdout.take().ok_or(AgentError::StdoutCaptureFailed)?;
        let stderr = child.stderr.take();

        let (event_tx, event_rx) = mpsc::channel::<AgentEvent>(256);
        let (command_tx, mut command_rx) = mpsc::channel::<PiRpcCommand>(32);
        let pending: Arc<Mutex<HashMap<String, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let pending_for_writer = pending.clone();
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(command) = command_rx.recv().await {
                if let (Some(id), Some(response_tx)) = (command.id.clone(), command.response_tx) {
                    pending_for_writer.lock().await.insert(id, response_tx);
                }
                match serde_json::to_string(&command.payload) {
                    Ok(line) => {
                        if stdin.write_all(line.as_bytes()).await.is_err()
                            || stdin.write_all(b"\n").await.is_err()
                            || stdin.flush().await.is_err()
                        {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let pending_for_reader = pending.clone();
        let event_tx_for_reader = event_tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                let value: Value = match serde_json::from_str(&line) {
                    Ok(value) => value,
                    Err(error) => {
                        let _ = event_tx_for_reader
                            .send(AgentEvent::Error(ErrorEvent {
                                message: format!("Failed to parse Pi RPC output: {error}"),
                                is_fatal: false,
                                code: None,
                                details: Some(Value::String(line)),
                            }))
                            .await;
                        continue;
                    }
                };

                if value.get("type").and_then(Value::as_str) == Some("response") {
                    if let Some(id) = value.get("id").and_then(Value::as_str) {
                        if let Some(response_tx) = pending_for_reader.lock().await.remove(id) {
                            let _ = response_tx.send(value);
                            continue;
                        }
                    }
                }

                let _ = Self::convert_event(value, &event_tx_for_reader).await;
            }
        });

        let event_tx_for_monitor = event_tx.clone();
        tokio::spawn(async move {
            let status = child.wait().await;
            let stderr_content = if let Some(mut stderr) = stderr {
                let mut buf = String::new();
                if let Err(err) = stderr.read_to_string(&mut buf).await {
                    tracing::debug!(error = %err, "Failed to read Pi stderr");
                }
                buf
            } else {
                String::new()
            };

            match status {
                Ok(exit_status) if !exit_status.success() => {
                    let message = if stderr_content.trim().is_empty() {
                        format!("Pi process exited with status: {exit_status}")
                    } else {
                        format!(
                            "Pi process failed ({exit_status}): {}",
                            stderr_content.trim()
                        )
                    };
                    let _ = event_tx_for_monitor
                        .send(AgentEvent::Error(ErrorEvent {
                            message,
                            is_fatal: true,
                            code: None,
                            details: None,
                        }))
                        .await;
                }
                Err(error) => {
                    let _ = event_tx_for_monitor
                        .send(AgentEvent::Error(ErrorEvent {
                            message: format!("Failed to wait for Pi process: {error}"),
                            is_fatal: true,
                            code: None,
                            details: None,
                        }))
                        .await;
                }
                Ok(_) => {
                    if !stderr_content.trim().is_empty() {
                        tracing::debug!("Pi stderr: {}", stderr_content.trim());
                    }
                }
            }
        });

        let state = Self::request(
            &command_tx,
            "conduit-get-state",
            json!({
                "id": "conduit-get-state",
                "type": "get_state",
            }),
        )
        .await?;

        let session_id = state
            .get("data")
            .and_then(|data| data.get("sessionId"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AgentError::Config("Pi RPC get_state did not return a session id".to_string())
            })?
            .to_string();

        event_tx
            .send(AgentEvent::SessionInit(SessionInitEvent {
                session_id: SessionId::from_string(session_id.clone()),
                model: Self::extract_model_id(&state).or_else(|| config.model.clone()),
            }))
            .await
            .map_err(|_| AgentError::ChannelClosed)?;

        let (input_tx, mut input_rx) = mpsc::channel::<AgentInput>(32);
        let command_tx_for_input = command_tx.clone();
        let event_tx_for_input = event_tx.clone();
        tokio::spawn(async move {
            let mut prompt_counter: u64 = 0;
            while let Some(input) = input_rx.recv().await {
                match input {
                    AgentInput::CodexPrompt { text, images, .. } => {
                        if !images.is_empty() {
                            let _ = event_tx_for_input
                                .send(AgentEvent::TurnFailed(TurnFailedEvent {
                                    error: "Pi image attachments are not supported in Conduit yet."
                                        .to_string(),
                                }))
                                .await;
                            continue;
                        }
                        prompt_counter = prompt_counter.saturating_add(1);
                        let request_id = format!("conduit-prompt-{prompt_counter}");
                        let _ = Self::send_prompt_command(
                            &command_tx_for_input,
                            &event_tx_for_input,
                            &request_id,
                            text,
                        )
                        .await;
                    }
                    AgentInput::ClaudeJsonl(_) | AgentInput::OpencodeQuestion { .. } => {
                        let _ = event_tx_for_input
                            .send(AgentEvent::TurnFailed(TurnFailedEvent {
                                error: "Pi does not support this input type.".to_string(),
                            }))
                            .await;
                    }
                }
            }
        });

        if !config.prompt.trim().is_empty() {
            Self::send_prompt_command(
                &command_tx,
                &event_tx,
                "conduit-initial-prompt",
                config.prompt.clone(),
            )
            .await?;
        }

        let mut handle = AgentHandle::new(event_rx, pid, Some(input_tx));
        handle.set_session_id(SessionId::from_string(session_id));
        Ok(handle)
    }

    async fn send_input(&self, handle: &AgentHandle, input: AgentInput) -> Result<(), AgentError> {
        let Some(ref input_tx) = handle.input_tx else {
            return Err(AgentError::ChannelClosed);
        };
        input_tx
            .send(input)
            .await
            .map_err(|_| AgentError::ChannelClosed)
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

// ============================================================================
// Model discovery
// ============================================================================

const PI_MODEL_CACHE_TTL_SECS: u64 = 60 * 60 * 24;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PiModelEntry {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PiModelCache {
    generated_at: u64,
    models: Vec<PiModelEntry>,
}

fn pi_cache_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|dir| dir.join("conduit").join("pi_models.json"))
}

fn pi_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn load_pi_cache(path: &PathBuf) -> Option<PiModelCache> {
    let data = match std::fs::read_to_string(path) {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
        Err(err) => {
            tracing::debug!(path = %path.display(), error = %err, "Failed to read Pi model cache");
            return None;
        }
    };
    match serde_json::from_str(&data) {
        Ok(cache) => Some(cache),
        Err(err) => {
            tracing::debug!(path = %path.display(), error = %err, "Failed to parse Pi model cache");
            None
        }
    }
}

fn pi_cache_is_fresh(cache: &PiModelCache) -> bool {
    pi_now_secs().saturating_sub(cache.generated_at) <= PI_MODEL_CACHE_TTL_SECS
}

fn save_pi_cache(path: &PathBuf, models: &[PiModelEntry]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let cache = PiModelCache {
        generated_at: pi_now_secs(),
        models: models.to_vec(),
    };
    let payload = serde_json::to_string_pretty(&cache)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    std::fs::write(path, payload)
}

fn discover_pi_models(binary_path: &PathBuf) -> Option<Vec<PiModelEntry>> {
    let output = std::process::Command::new(binary_path)
        .arg("--list-models")
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);

    // pi --list-models outputs a table: "provider  model  context  max-out  thinking  images"
    // Parse each data row and combine as "provider/model_id" so pi resolves it correctly.
    let table_entries: Vec<PiModelEntry> = text
        .lines()
        .filter_map(|line| {
            let mut cols = line.split_whitespace();
            let provider = cols.next()?;
            let model_id = cols.next()?;
            // Skip the header row
            if provider == "provider" && model_id == "model" {
                return None;
            }
            let id = format!("{provider}/{model_id}");
            Some(PiModelEntry {
                display_name: model_id.to_string(),
                id,
            })
        })
        .collect();

    if table_entries.is_empty() {
        None
    } else {
        Some(table_entries)
    }
}

pub fn load_pi_models(binary_path: Option<PathBuf>) -> Vec<PiModelEntry> {
    let binary = binary_path
        .filter(|p| p.exists())
        .or_else(|| which::which("pi").ok());

    let (cached_models, is_fresh) =
        match pi_cache_path().and_then(|p| load_pi_cache(&p).map(|c| (p, c))) {
            Some((_, cache)) => {
                let fresh = pi_cache_is_fresh(&cache);
                (cache.models, fresh)
            }
            None => (Vec::new(), false),
        };

    if !is_fresh {
        if let Some(bin) = binary {
            std::thread::spawn(move || {
                let entries = discover_pi_models(&bin);
                if let Some(entries) = entries {
                    if let Some(path) = pi_cache_path() {
                        if let Err(err) = save_pi_cache(&path, &entries) {
                            tracing::debug!(error = %err, "Failed to save Pi model cache");
                        }
                    }
                    crate::ModelRegistry::set_pi_models(entries);
                }
            });
        }
    }

    cached_models
}
