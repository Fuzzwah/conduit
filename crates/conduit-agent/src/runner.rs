use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::error::AgentError;
use crate::events::AgentEvent;
use crate::session::SessionId;
use conduit_types::SkillReference;

pub use conduit_types::{AgentMode, AgentType};

/// Provider-agnostic reasoning effort profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningEffort {
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
}

impl ReasoningEffort {
    pub fn as_str(self) -> &'static str {
        match self {
            ReasoningEffort::Minimal => "minimal",
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
            ReasoningEffort::XHigh => "xhigh",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            ReasoningEffort::Minimal => "Minimal",
            ReasoningEffort::Low => "Low",
            ReasoningEffort::Medium => "Medium",
            ReasoningEffort::High => "High",
            ReasoningEffort::XHigh => "XHigh",
        }
    }

    pub fn claude_arg_value(self) -> Option<&'static str> {
        match self {
            ReasoningEffort::Low => Some("low"),
            ReasoningEffort::Medium => Some("medium"),
            ReasoningEffort::High => Some("high"),
            ReasoningEffort::Minimal | ReasoningEffort::XHigh => None,
        }
    }

    pub fn codex_config_value(self) -> &'static str {
        self.as_str()
    }
}

/// Configuration for starting an agent
#[derive(Debug, Clone)]
pub struct AgentStartConfig {
    pub prompt: String,
    pub working_dir: PathBuf,
    pub allowed_tools: Vec<String>,
    pub resume_session: Option<SessionId>,
    pub timeout_ms: Option<u64>,
    pub additional_args: Vec<String>,
    /// Model to use (e.g., "sonnet", "opus" for Claude; "o4-mini" for Codex)
    pub model: Option<String>,
    /// Optional reasoning effort profile.
    pub reasoning_effort: Option<ReasoningEffort>,
    /// Optional image paths to attach to the initial prompt
    pub images: Vec<PathBuf>,
    /// Agent mode (Build vs Plan)
    pub agent_mode: AgentMode,
    /// Optional input format override (e.g. "stream-json" for Claude)
    pub input_format: Option<String>,
    /// Optional stdin payload for structured input (e.g. JSONL)
    pub stdin_payload: Option<String>,
    /// Optional structured skill reference for providers that support it.
    pub skill: Option<SkillReference>,
    /// Optional per-session config overrides for providers that support them.
    pub session_config_overrides: std::collections::HashMap<String, Value>,
    /// Enable model orchestration (Claude only): write sub-agent definitions and inject
    /// delegation instructions so the orchestrator delegates cheap ops to Haiku.
    pub orchestration_enabled: bool,
}

impl AgentStartConfig {
    pub fn new(prompt: impl Into<String>, working_dir: PathBuf) -> Self {
        Self {
            prompt: prompt.into(),
            working_dir,
            allowed_tools: vec![],
            resume_session: None,
            timeout_ms: None,
            additional_args: vec![],
            model: None,
            reasoning_effort: None,
            images: Vec::new(),
            agent_mode: AgentMode::default(),
            input_format: None,
            stdin_payload: None,
            skill: None,
            session_config_overrides: std::collections::HashMap::new(),
            orchestration_enabled: false,
        }
    }

    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = tools;
        self
    }

    pub fn with_resume(mut self, session_id: SessionId) -> Self {
        self.resume_session = Some(session_id);
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_reasoning_effort(mut self, effort: ReasoningEffort) -> Self {
        self.reasoning_effort = Some(effort);
        self
    }

    pub fn with_images(mut self, images: Vec<PathBuf>) -> Self {
        self.images = images;
        self
    }

    pub fn with_agent_mode(mut self, mode: AgentMode) -> Self {
        self.agent_mode = mode;
        self
    }

    pub fn with_input_format(mut self, format: impl Into<String>) -> Self {
        self.input_format = Some(format.into());
        self
    }

    pub fn with_stdin_payload(mut self, payload: impl Into<String>) -> Self {
        self.stdin_payload = Some(payload.into());
        self
    }

    pub fn with_skill(mut self, skill: SkillReference) -> Self {
        self.skill = Some(skill);
        self
    }

    pub fn with_session_config_override(mut self, key: impl Into<String>, value: Value) -> Self {
        self.session_config_overrides.insert(key.into(), value);
        self
    }

    pub fn with_orchestration(mut self, enabled: bool) -> Self {
        self.orchestration_enabled = enabled;
        self
    }
}

/// Input payload for running agents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentInput {
    /// Raw JSONL payload for Claude streaming input.
    ClaudeJsonl(String),
    /// Codex prompt with optional local images and model override.
    CodexPrompt {
        text: String,
        images: Vec<PathBuf>,
        model: Option<String>,
        skill: Option<SkillReference>,
    },
    /// OpenCode question response (None means reject).
    OpencodeQuestion {
        request_id: String,
        answers: Option<Vec<Vec<String>>>,
    },
}

/// Handle to a running agent process
pub struct AgentHandle {
    /// Receiver for agent events
    pub events: mpsc::Receiver<AgentEvent>,
    /// Current session ID (may be set after init event)
    pub session_id: Option<SessionId>,
    /// Process ID for monitoring
    pub pid: u32,
    /// Optional input channel for streaming stdin payloads
    pub input_tx: Option<mpsc::Sender<AgentInput>>,
}

impl AgentHandle {
    pub fn new(
        events: mpsc::Receiver<AgentEvent>,
        pid: u32,
        input_tx: Option<mpsc::Sender<AgentInput>>,
    ) -> Self {
        Self {
            events,
            session_id: None,
            pid,
            input_tx,
        }
    }

    pub fn set_session_id(&mut self, session_id: SessionId) {
        self.session_id = Some(session_id);
    }

    pub fn take_input_sender(&mut self) -> Option<mpsc::Sender<AgentInput>> {
        self.input_tx.take()
    }
}

/// Trait for agent runners that can spawn and manage agent processes
#[async_trait]
pub trait AgentRunner: Send + Sync {
    /// Agent type identifier (e.g., "claude", "codex")
    fn agent_type(&self) -> AgentType;

    /// Start the agent with the given configuration
    async fn start(&self, config: AgentStartConfig) -> Result<AgentHandle, AgentError>;

    /// Send input to a running agent (for interactive prompts)
    async fn send_input(&self, handle: &AgentHandle, input: AgentInput) -> Result<(), AgentError>;

    /// Request graceful shutdown
    async fn stop(&self, handle: &AgentHandle) -> Result<(), AgentError>;

    /// Force kill the agent process
    async fn kill(&self, handle: &AgentHandle) -> Result<(), AgentError>;

    /// Check if the agent binary is available
    fn is_available(&self) -> bool;

    /// Get the path to the agent binary
    fn binary_path(&self) -> Option<PathBuf>;
}
