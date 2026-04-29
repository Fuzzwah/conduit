//! Model configuration and registry

use std::sync::{OnceLock, RwLock};

use tracing::error;

use crate::agent::claude::{load_claude_models, ClaudeModelEntry};
use crate::agent::codex::CodexModelEntry;
use crate::agent::gemini::GeminiModelEntry;
use crate::agent::opencode::load_opencode_models;
use crate::agent::pi::PiModelEntry;
use crate::agent::AgentType;

/// Information about a model
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Internal model ID (passed to CLI)
    pub id: String,
    /// Display name for UI
    pub display_name: String,
    /// Short alias for quick selection
    pub alias: String,
    /// Description of model capabilities
    pub description: String,
    /// Whether this is the default model for the agent type
    pub is_default: bool,
    /// Agent type this model belongs to
    pub agent_type: AgentType,
    /// Maximum context window in tokens
    pub context_window: i64,
}

impl ModelInfo {
    pub fn new(
        agent_type: AgentType,
        id: &str,
        display_name: &str,
        alias: &str,
        description: &str,
        context_window: i64,
    ) -> Self {
        Self {
            id: id.to_string(),
            display_name: display_name.to_string(),
            alias: alias.to_string(),
            description: description.to_string(),
            is_default: false,
            agent_type,
            context_window,
        }
    }

    pub fn as_default(mut self) -> Self {
        self.is_default = true;
        self
    }
}

/// Registry of available models for each agent type
#[derive(Debug, Default)]
pub struct ModelRegistry;

impl ModelRegistry {
    /// Default context window for Claude models (200K tokens)
    pub const CLAUDE_CONTEXT_WINDOW: i64 = 200_000;
    /// Extended context window for Claude 1M variants
    pub const CLAUDE_1M_CONTEXT_WINDOW: i64 = 1_000_000;

    /// Fallback context window for Codex models when model-specific value is unknown
    pub const CODEX_CONTEXT_WINDOW: i64 = 272_000;
    /// Context window for GPT-5.3 Codex
    pub const CODEX_GPT53_CONTEXT_WINDOW: i64 = 400_000;
    /// Context window for GPT-5.3 Codex Spark
    pub const CODEX_GPT53_SPARK_CONTEXT_WINDOW: i64 = 128_000;

    /// Default context window for Gemini models (approximate)
    pub const GEMINI_CONTEXT_WINDOW: i64 = 1_000_000;

    /// Default context window for OpenCode models (approximate)
    pub const OPENCODE_CONTEXT_WINDOW: i64 = 200_000;

    /// Default context window for GitHub Copilot models (conservative estimate)
    pub const COPILOT_CONTEXT_WINDOW: i64 = 128_000;
    /// Default context window for Pi when model metadata is unknown
    pub const PI_CONTEXT_WINDOW: i64 = 200_000;
    /// Default context window for Dirac when model metadata is unknown
    pub const DIRAC_CONTEXT_WINDOW: i64 = 200_000;

    const OPENCODE_DEFAULT_MODEL_ID: &'static str = "default";

    fn opencode_store() -> &'static RwLock<Vec<ModelInfo>> {
        static OPENCODE_MODELS: OnceLock<RwLock<Vec<ModelInfo>>> = OnceLock::new();
        OPENCODE_MODELS.get_or_init(|| RwLock::new(Vec::new()))
    }

    fn opencode_default_model() -> ModelInfo {
        ModelInfo::new(
            AgentType::Opencode,
            Self::OPENCODE_DEFAULT_MODEL_ID,
            "OpenCode Default",
            Self::OPENCODE_DEFAULT_MODEL_ID,
            "Use OpenCode's default model selection",
            Self::OPENCODE_CONTEXT_WINDOW,
        )
        .as_default()
    }

    fn build_opencode_models(model_ids: Vec<String>) -> Vec<ModelInfo> {
        let mut models = vec![Self::opencode_default_model()];
        for id in model_ids {
            if id == Self::OPENCODE_DEFAULT_MODEL_ID {
                continue;
            }
            models.push(ModelInfo::new(
                AgentType::Opencode,
                &id,
                &id,
                &id,
                "OpenCode model",
                Self::OPENCODE_CONTEXT_WINDOW,
            ));
        }
        models
    }

    pub fn set_opencode_models(model_ids: Vec<String>) {
        let mut models = Self::build_opencode_models(model_ids);
        models.sort_by(|a, b| a.id.cmp(&b.id));
        models.dedup_by(|a, b| a.id == b.id);
        if let Some(pos) = models
            .iter()
            .position(|model| model.id == Self::OPENCODE_DEFAULT_MODEL_ID)
        {
            let default = models.remove(pos);
            models.insert(0, default);
        }
        let mut store = match Self::opencode_store().write() {
            Ok(guard) => guard,
            Err(err) => {
                error!(error = %err, "opencode_store poisoned in set_opencode_models");
                err.into_inner()
            }
        };
        *store = models;
    }

    pub fn clear_opencode_models() {
        let mut store = match Self::opencode_store().write() {
            Ok(guard) => guard,
            Err(err) => {
                error!(error = %err, "opencode_store poisoned in clear_opencode_models");
                err.into_inner()
            }
        };
        store.clear();
    }

    pub fn drop_opencode_model(model_id: &str) {
        if model_id == Self::OPENCODE_DEFAULT_MODEL_ID {
            return;
        }
        let mut store = match Self::opencode_store().write() {
            Ok(guard) => guard,
            Err(err) => {
                error!(error = %err, "opencode_store poisoned in drop_opencode_model");
                err.into_inner()
            }
        };
        store.retain(|model| model.id != model_id);
    }

    pub fn refresh_opencode_models() {
        let models = load_opencode_models(None);
        if models.is_empty() {
            return;
        }
        Self::set_opencode_models(models);
    }

    pub fn opencode_models() -> Vec<ModelInfo> {
        match Self::opencode_store().read() {
            Ok(guard) => guard.clone(),
            Err(err) => {
                error!(error = %err, "opencode_store poisoned in opencode_models");
                Vec::new()
            }
        }
    }

    fn claude_store() -> &'static RwLock<Vec<ModelInfo>> {
        static CLAUDE_MODELS: OnceLock<RwLock<Vec<ModelInfo>>> = OnceLock::new();
        CLAUDE_MODELS.get_or_init(|| RwLock::new(Vec::new()))
    }

    pub fn set_claude_models(entries: Vec<ClaudeModelEntry>) {
        let models: Vec<ModelInfo> = entries
            .into_iter()
            .enumerate()
            .map(|(i, entry)| {
                let context_window = if entry.id.contains("1m") || entry.id.contains("1M") {
                    Self::CLAUDE_1M_CONTEXT_WINDOW
                } else {
                    Self::CLAUDE_CONTEXT_WINDOW
                };
                let mut info = ModelInfo::new(
                    AgentType::Claude,
                    &entry.id,
                    &entry.display_name,
                    &entry.id,
                    "",
                    context_window,
                );
                if i == 0 {
                    info = info.as_default();
                }
                info
            })
            .collect();
        let mut store = match Self::claude_store().write() {
            Ok(guard) => guard,
            Err(err) => {
                error!(error = %err, "claude_store poisoned in set_claude_models");
                err.into_inner()
            }
        };
        *store = models;
    }

    pub fn clear_claude_models() {
        let mut store = match Self::claude_store().write() {
            Ok(guard) => guard,
            Err(err) => {
                error!(error = %err, "claude_store poisoned in clear_claude_models");
                err.into_inner()
            }
        };
        store.clear();
    }

    pub fn refresh_claude_models() {
        let models = load_claude_models(None);
        if models.is_empty() {
            return;
        }
        Self::set_claude_models(models);
    }

    /// Get available models for Claude Code.
    /// Returns dynamically discovered models when available, else the static fallback list.
    pub fn claude_models() -> Vec<ModelInfo> {
        let dynamic = match Self::claude_store().read() {
            Ok(guard) => guard.clone(),
            Err(err) => {
                error!(error = %err, "claude_store poisoned in claude_models");
                Vec::new()
            }
        };
        if !dynamic.is_empty() {
            return dynamic;
        }
        Self::claude_models_static()
    }

    fn claude_models_static() -> Vec<ModelInfo> {
        vec![
            ModelInfo::new(
                AgentType::Claude,
                "opus",
                "Opus 4.7",
                "opus",
                "Most capable for complex work",
                Self::CLAUDE_CONTEXT_WINDOW,
            )
            .as_default(),
            ModelInfo::new(
                AgentType::Claude,
                "opus[1m]",
                "Opus 4.7 [1m]",
                "opus[1m]",
                "Opus 4.7 with 1M context",
                Self::CLAUDE_1M_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Claude,
                "sonnet",
                "Sonnet 4.6",
                "sonnet",
                "Sonnet 4.6, best for everyday tasks",
                Self::CLAUDE_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Claude,
                "sonnet[1m]",
                "Sonnet 4.6 [1m]",
                "sonnet[1m]",
                "Sonnet 4.6 with 1M context",
                Self::CLAUDE_1M_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Claude,
                "haiku",
                "Haiku 4.5",
                "haiku",
                "Haiku 4.5, fastest for quick answers",
                Self::CLAUDE_CONTEXT_WINDOW,
            ),
        ]
    }

    fn codex_store() -> &'static RwLock<Vec<ModelInfo>> {
        static CODEX_MODELS: OnceLock<RwLock<Vec<ModelInfo>>> = OnceLock::new();
        CODEX_MODELS.get_or_init(|| RwLock::new(Vec::new()))
    }

    pub fn set_codex_models(entries: Vec<CodexModelEntry>) {
        let mut models: Vec<ModelInfo> = entries
            .into_iter()
            .enumerate()
            .map(|(i, entry)| {
                let mut info = ModelInfo::new(
                    AgentType::Codex,
                    &entry.id,
                    &entry.display_name,
                    &entry.id,
                    "",
                    Self::CODEX_CONTEXT_WINDOW,
                );
                if i == 0 {
                    info = info.as_default();
                }
                info
            })
            .collect();
        models.sort_by(|a, b| b.id.cmp(&a.id));
        let mut store = match Self::codex_store().write() {
            Ok(guard) => guard,
            Err(err) => {
                error!(error = %err, "codex_store poisoned in set_codex_models");
                err.into_inner()
            }
        };
        *store = models;
    }

    pub fn clear_codex_models() {
        let mut store = match Self::codex_store().write() {
            Ok(guard) => guard,
            Err(err) => {
                error!(error = %err, "codex_store poisoned in clear_codex_models");
                err.into_inner()
            }
        };
        store.clear();
    }

    fn gemini_store() -> &'static RwLock<Vec<ModelInfo>> {
        static GEMINI_MODELS: OnceLock<RwLock<Vec<ModelInfo>>> = OnceLock::new();
        GEMINI_MODELS.get_or_init(|| RwLock::new(Vec::new()))
    }

    pub fn set_gemini_models(entries: Vec<GeminiModelEntry>) {
        let mut models: Vec<ModelInfo> = entries
            .into_iter()
            .enumerate()
            .map(|(i, entry)| {
                let mut info = ModelInfo::new(
                    AgentType::Gemini,
                    &entry.id,
                    &entry.display_name,
                    &entry.id,
                    "",
                    Self::GEMINI_CONTEXT_WINDOW,
                );
                if i == 0 {
                    info = info.as_default();
                }
                info
            })
            .collect();
        models.sort_by(|a, b| b.id.cmp(&a.id));
        let mut store = match Self::gemini_store().write() {
            Ok(guard) => guard,
            Err(err) => {
                error!(error = %err, "gemini_store poisoned in set_gemini_models");
                err.into_inner()
            }
        };
        *store = models;
    }

    pub fn clear_gemini_models() {
        let mut store = match Self::gemini_store().write() {
            Ok(guard) => guard,
            Err(err) => {
                error!(error = %err, "gemini_store poisoned in clear_gemini_models");
                err.into_inner()
            }
        };
        store.clear();
    }

    fn pi_store() -> &'static RwLock<Vec<ModelInfo>> {
        static PI_MODELS: OnceLock<RwLock<Vec<ModelInfo>>> = OnceLock::new();
        PI_MODELS.get_or_init(|| RwLock::new(Vec::new()))
    }

    fn dirac_store() -> &'static RwLock<Vec<ModelInfo>> {
        static DIRAC_MODELS: OnceLock<RwLock<Vec<ModelInfo>>> = OnceLock::new();
        DIRAC_MODELS.get_or_init(|| RwLock::new(Vec::new()))
    }

    pub fn set_pi_models(entries: Vec<PiModelEntry>) {
        let models: Vec<ModelInfo> = entries
            .into_iter()
            .enumerate()
            .map(|(i, entry)| {
                let mut info = ModelInfo::new(
                    AgentType::Pi,
                    &entry.id,
                    &entry.display_name,
                    &entry.id,
                    "",
                    Self::PI_CONTEXT_WINDOW,
                );
                if i == 0 {
                    info = info.as_default();
                }
                info
            })
            .collect();
        let mut store = match Self::pi_store().write() {
            Ok(guard) => guard,
            Err(err) => {
                error!(error = %err, "pi_store poisoned in set_pi_models");
                err.into_inner()
            }
        };
        *store = models;
    }

    pub fn clear_pi_models() {
        let mut store = match Self::pi_store().write() {
            Ok(guard) => guard,
            Err(err) => {
                error!(error = %err, "pi_store poisoned in clear_pi_models");
                err.into_inner()
            }
        };
        store.clear();
    }

    pub fn clear_dirac_models() {
        let mut store = match Self::dirac_store().write() {
            Ok(guard) => guard,
            Err(err) => {
                error!(error = %err, "dirac_store poisoned in clear_dirac_models");
                err.into_inner()
            }
        };
        store.clear();
    }

    /// Get available models for Codex CLI
    pub fn codex_models() -> Vec<ModelInfo> {
        let dynamic = match Self::codex_store().read() {
            Ok(guard) => guard.clone(),
            Err(err) => {
                error!(error = %err, "codex_store poisoned in codex_models");
                Vec::new()
            }
        };
        if !dynamic.is_empty() {
            return dynamic;
        }
        Self::codex_models_static()
    }

    fn codex_models_static() -> Vec<ModelInfo> {
        vec![
            ModelInfo::new(
                AgentType::Codex,
                "gpt-5.4",
                "GPT-5.4",
                "gpt-5.4",
                "Latest frontier agentic coding model",
                Self::CODEX_CONTEXT_WINDOW,
            )
            .as_default(),
            ModelInfo::new(
                AgentType::Codex,
                "gpt-5.3-codex",
                "GPT-5.3-Codex",
                "gpt-5.3-codex",
                "Frontier Codex-optimized agentic coding model",
                Self::CODEX_GPT53_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Codex,
                "gpt-5.3-codex-spark",
                "GPT-5.3-Codex-Spark",
                "gpt-5.3-codex-spark",
                "Ultra-fast coding model",
                Self::CODEX_GPT53_SPARK_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Codex,
                "gpt-5.2-codex",
                "GPT-5.2-Codex",
                "gpt-5.2-codex",
                "Frontier agentic coding model",
                Self::CODEX_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Codex,
                "gpt-5.1-codex-max",
                "GPT-5.1-Codex-Max",
                "gpt-5.1-codex-max",
                "Codex-optimized flagship for deep and fast reasoning",
                Self::CODEX_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Codex,
                "gpt-5.2",
                "GPT-5.2",
                "gpt-5.2",
                "Latest frontier model with improvements across knowledge, reasoning and coding",
                Self::CODEX_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Codex,
                "gpt-5.1-codex-mini",
                "GPT-5.1-Codex-Mini",
                "gpt-5.1-codex-mini",
                "Optimized for Codex: cheaper and faster, but less capable",
                Self::CODEX_CONTEXT_WINDOW,
            ),
        ]
    }

    /// Get available models for Gemini CLI
    pub fn gemini_models() -> Vec<ModelInfo> {
        let dynamic = match Self::gemini_store().read() {
            Ok(guard) => guard.clone(),
            Err(err) => {
                error!(error = %err, "gemini_store poisoned in gemini_models");
                Vec::new()
            }
        };
        if !dynamic.is_empty() {
            return dynamic;
        }
        Self::gemini_models_static()
    }

    fn gemini_models_static() -> Vec<ModelInfo> {
        vec![
            ModelInfo::new(
                AgentType::Gemini,
                "gemini-2.5-pro",
                "Gemini 2.5 Pro",
                "gemini-2.5-pro",
                "Highest quality Gemini model",
                Self::GEMINI_CONTEXT_WINDOW,
            )
            .as_default(),
            ModelInfo::new(
                AgentType::Gemini,
                "gemini-2.5-flash",
                "Gemini 2.5 Flash",
                "gemini-2.5-flash",
                "Fast and capable Gemini model",
                Self::GEMINI_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Gemini,
                "gemini-2.5-flash-lite",
                "Gemini 2.5 Flash Lite",
                "gemini-2.5-flash-lite",
                "Lowest-latency Gemini model",
                Self::GEMINI_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Gemini,
                "gemini-3-pro-preview",
                "Gemini 3 Pro Preview",
                "gemini-3-pro-preview",
                "Preview Gemini 3 model",
                Self::GEMINI_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Gemini,
                "gemini-3-flash-preview",
                "Gemini 3 Flash Preview",
                "gemini-3-flash-preview",
                "Preview Gemini 3 flash model",
                Self::GEMINI_CONTEXT_WINDOW,
            ),
        ]
    }

    /// Get available models for GitHub Copilot
    pub fn copilot_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo::new(
                AgentType::Copilot,
                "claude-haiku-4.5",
                "Claude Haiku 4.5",
                "claude-haiku-4.5",
                "Anthropic Claude Haiku 4.5",
                Self::CLAUDE_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "claude-sonnet-4",
                "Claude Sonnet 4",
                "claude-sonnet-4",
                "Anthropic Claude Sonnet 4",
                Self::CLAUDE_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "claude-sonnet-4.5",
                "Claude Sonnet 4.5",
                "claude-sonnet-4.5",
                "Anthropic Claude Sonnet 4.5",
                Self::CLAUDE_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "claude-sonnet-4.6",
                "Claude Sonnet 4.6",
                "claude-sonnet-4.6",
                "Anthropic Claude Sonnet 4.6",
                Self::CLAUDE_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "gemini-2.5-pro",
                "Gemini 2.5 Pro",
                "gemini-2.5-pro",
                "Google Gemini 2.5 Pro",
                Self::GEMINI_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "gemini-3-flash",
                "Gemini 3 Flash",
                "gemini-3-flash",
                "Google Gemini 3 Flash",
                Self::GEMINI_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "gemini-3.1-pro",
                "Gemini 3.1 Pro",
                "gemini-3.1-pro",
                "Google Gemini 3.1 Pro",
                Self::GEMINI_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "gpt-4.1",
                "GPT-4.1",
                "gpt-4.1",
                "OpenAI GPT-4.1",
                Self::COPILOT_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "gpt-4o",
                "GPT-4o",
                "gpt-4o",
                "OpenAI GPT-4o",
                Self::COPILOT_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "gpt-5-mini",
                "GPT-5 mini",
                "gpt-5-mini",
                "OpenAI GPT-5 mini",
                Self::COPILOT_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "gpt-5.2",
                "GPT-5.2",
                "gpt-5.2",
                "OpenAI GPT-5.2",
                Self::CODEX_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "gpt-5.2-codex",
                "GPT-5.2-Codex",
                "gpt-5.2-codex",
                "OpenAI GPT-5.2-Codex",
                Self::CODEX_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "gpt-5.3-codex",
                "GPT-5.3-Codex",
                "gpt-5.3-codex",
                "Frontier Codex-optimized agentic coding model",
                Self::CODEX_GPT53_CONTEXT_WINDOW,
            )
            .as_default(),
            ModelInfo::new(
                AgentType::Copilot,
                "gpt-5.3-codex-spark",
                "GPT-5.3-Codex-Spark",
                "gpt-5.3-codex-spark",
                "Ultra-fast Codex model",
                Self::CODEX_GPT53_SPARK_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "gpt-5.4",
                "GPT-5.4",
                "gpt-5.4",
                "Latest frontier agentic coding model",
                Self::CODEX_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "gpt-5.4-mini",
                "GPT-5.4 mini",
                "gpt-5.4-mini",
                "OpenAI GPT-5.4 mini",
                Self::COPILOT_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "gpt-5.4-nano",
                "GPT-5.4 nano",
                "gpt-5.4-nano",
                "OpenAI GPT-5.4 nano",
                Self::COPILOT_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "goldeneye",
                "Goldeneye",
                "goldeneye",
                "GitHub evaluation model",
                Self::COPILOT_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "grok-code-fast-1",
                "Grok Code Fast 1",
                "grok-code-fast-1",
                "xAI Grok Code Fast 1",
                Self::COPILOT_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Copilot,
                "raptor-mini",
                "Raptor mini",
                "raptor-mini",
                "GitHub evaluation model",
                Self::COPILOT_CONTEXT_WINDOW,
            ),
        ]
    }

    /// Get available models for Pi
    pub fn pi_models() -> Vec<ModelInfo> {
        let dynamic = match Self::pi_store().read() {
            Ok(guard) => guard.clone(),
            Err(err) => {
                error!(error = %err, "pi_store poisoned in pi_models");
                Vec::new()
            }
        };
        if !dynamic.is_empty() {
            return dynamic;
        }
        Self::pi_models_static()
    }

    /// Get available models for Dirac
    pub fn dirac_models() -> Vec<ModelInfo> {
        let dynamic = match Self::dirac_store().read() {
            Ok(guard) => guard.clone(),
            Err(err) => {
                error!(error = %err, "dirac_store poisoned in dirac_models");
                Vec::new()
            }
        };
        if !dynamic.is_empty() {
            return dynamic;
        }
        Self::dirac_models_static()
    }

    fn pi_models_static() -> Vec<ModelInfo> {
        Self::build_pi_models()
    }

    fn build_pi_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo::new(
                AgentType::Pi,
                "openrouter/deepseek/deepseek-v4-flash",
                "deepseek/deepseek-v4-flash",
                "openrouter/deepseek/deepseek-v4-flash",
                "DeepSeek model via OpenRouter",
                Self::PI_CONTEXT_WINDOW,
            )
            .as_default(),
            ModelInfo::new(
                AgentType::Pi,
                "openrouter/google/gemini-3.1-flash-lite-preview",
                "google/gemini-3.1-flash-lite-preview",
                "openrouter/google/gemini-3.1-flash-lite-preview",
                "Google model via OpenRouter",
                Self::PI_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Pi,
                "openrouter/mistralai/mistral-nemo",
                "mistralai/mistral-nemo",
                "openrouter/mistralai/mistral-nemo",
                "Mistral model via OpenRouter",
                Self::PI_CONTEXT_WINDOW,
            ),
        ]
    }

    fn dirac_models_static() -> Vec<ModelInfo> {
        vec![
            ModelInfo::new(
                AgentType::Dirac,
                "claude-sonnet-4-5-20250929",
                "Claude Sonnet 4.5",
                "claude-sonnet-4-5-20250929",
                "Anthropic model via Dirac",
                Self::CLAUDE_CONTEXT_WINDOW,
            )
            .as_default(),
            ModelInfo::new(
                AgentType::Dirac,
                "gemini-3-flash-preview",
                "Gemini 3 Flash Preview",
                "gemini-3-flash-preview",
                "Google model via Dirac",
                Self::GEMINI_CONTEXT_WINDOW,
            ),
            ModelInfo::new(
                AgentType::Dirac,
                "gpt-4o",
                "GPT-4o",
                "gpt-4o",
                "OpenAI model via Dirac",
                Self::COPILOT_CONTEXT_WINDOW,
            ),
        ]
    }

    /// Get all models grouped by agent type
    pub fn all_models() -> Vec<ModelInfo> {
        let mut models = Self::claude_models();
        models.extend(Self::codex_models());
        models.extend(Self::dirac_models());
        models.extend(Self::gemini_models());
        models.extend(Self::opencode_models());
        models.extend(Self::copilot_models());
        models.extend(Self::pi_models());
        models
    }

    /// Get models for a specific agent type
    pub fn models_for(agent_type: AgentType) -> Vec<ModelInfo> {
        match agent_type {
            AgentType::Claude => Self::claude_models(),
            AgentType::Codex => Self::codex_models(),
            AgentType::Dirac => Self::dirac_models(),
            AgentType::Gemini => Self::gemini_models(),
            AgentType::Opencode => Self::opencode_models(),
            AgentType::Copilot => Self::copilot_models(),
            AgentType::Pi => Self::pi_models(),
        }
    }

    /// Get the default model for an agent type
    pub fn default_model(agent_type: AgentType) -> String {
        match agent_type {
            AgentType::Claude => "opus".to_string(),
            AgentType::Codex => "gpt-5.4".to_string(),
            AgentType::Dirac => "claude-sonnet-4-5-20250929".to_string(),
            AgentType::Gemini => "gemini-2.5-pro".to_string(),
            AgentType::Opencode => Self::OPENCODE_DEFAULT_MODEL_ID.to_string(),
            AgentType::Copilot => "gpt-5.3-codex".to_string(),
            AgentType::Pi => "openrouter/deepseek/deepseek-v4-flash".to_string(),
        }
    }

    /// Find a model by ID or alias
    pub fn find_model(agent_type: AgentType, id_or_alias: &str) -> Option<ModelInfo> {
        if agent_type == AgentType::Opencode {
            let trimmed = id_or_alias.trim();
            if trimmed.is_empty() {
                return None;
            }
            if let Some(model) = Self::opencode_models()
                .into_iter()
                .find(|m| m.id == trimmed || m.alias == trimmed)
            {
                return Some(model);
            }
            return Some(ModelInfo::new(
                AgentType::Opencode,
                trimmed,
                trimmed,
                trimmed,
                "OpenCode model",
                Self::OPENCODE_CONTEXT_WINDOW,
            ));
        }

        Self::models_for(agent_type)
            .into_iter()
            .find(|m| m.id == id_or_alias || m.alias == id_or_alias)
    }

    /// Get the icon for an agent type
    pub fn agent_icon(agent_type: AgentType) -> &'static str {
        match agent_type {
            AgentType::Claude => "✻",
            AgentType::Codex => "◎",
            AgentType::Dirac => "◉",
            AgentType::Gemini => "◆",
            AgentType::Opencode => "◍",
            AgentType::Copilot => "⊙",
            AgentType::Pi => "◌",
        }
    }

    /// Get the section title for an agent type
    pub fn agent_section_title(agent_type: AgentType) -> &'static str {
        match agent_type {
            AgentType::Claude => "Claude Code",
            AgentType::Codex => "Codex",
            AgentType::Dirac => "Dirac",
            AgentType::Gemini => "Gemini",
            AgentType::Opencode => "OpenCode",
            AgentType::Copilot => "GitHub Copilot",
            AgentType::Pi => "Pi",
        }
    }

    /// Get context window limit for a specific model
    pub fn context_window(agent_type: AgentType, model_id: &str) -> i64 {
        Self::find_model(agent_type, model_id)
            .map(|m| m.context_window)
            .unwrap_or_else(|| Self::default_context_window(agent_type))
    }

    /// Default context window when model not found
    pub fn default_context_window(agent_type: AgentType) -> i64 {
        match agent_type {
            AgentType::Claude => Self::CLAUDE_CONTEXT_WINDOW,
            AgentType::Codex => Self::CODEX_CONTEXT_WINDOW,
            AgentType::Dirac => Self::DIRAC_CONTEXT_WINDOW,
            AgentType::Gemini => Self::GEMINI_CONTEXT_WINDOW,
            AgentType::Opencode => Self::OPENCODE_CONTEXT_WINDOW,
            AgentType::Copilot => Self::COPILOT_CONTEXT_WINDOW,
            AgentType::Pi => Self::PI_CONTEXT_WINDOW,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codex_models_include_gpt54_as_default() {
        let models = ModelRegistry::codex_models();
        let default_model = models
            .iter()
            .find(|model| model.is_default)
            .expect("expected default Codex model");

        assert_eq!(default_model.id, "gpt-5.4");
        assert_eq!(ModelRegistry::default_model(AgentType::Codex), "gpt-5.4");
        assert!(models.iter().any(|model| model.id == "gpt-5.3-codex"));
    }

    #[test]
    fn test_pi_models_match_guardrails_allowlist() {
        ModelRegistry::clear_pi_models();

        let models = ModelRegistry::pi_models();
        let default_model = models
            .iter()
            .find(|model| model.is_default)
            .expect("expected default Pi model");

        assert_eq!(default_model.id, "openrouter/deepseek/deepseek-v4-flash");
        assert_eq!(
            ModelRegistry::default_model(AgentType::Pi),
            "openrouter/deepseek/deepseek-v4-flash"
        );
        assert_eq!(
            models
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "openrouter/deepseek/deepseek-v4-flash",
                "openrouter/google/gemini-3.1-flash-lite-preview",
                "openrouter/mistralai/mistral-nemo",
            ]
        );
    }
}
