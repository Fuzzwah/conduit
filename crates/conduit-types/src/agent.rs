use serde::{Deserialize, Serialize};

/// Agent type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    Claude,
    Codex,
    Dirac,
    Gemini,
    DeepseekTui,
    Opencode,
    Copilot,
    Pi,
    Maki,
}

/// Agent mode (Build vs Plan)
///
/// Build mode (default): agent can read, write, and execute commands
/// Plan mode: read-only analysis, no modifications allowed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AgentMode {
    #[default]
    Build,
    Plan,
}

impl AgentMode {
    /// Convert to Claude's --permission-mode argument value
    pub fn as_permission_mode(&self) -> &'static str {
        match self {
            AgentMode::Build => "default",
            AgentMode::Plan => "plan",
        }
    }

    /// Display name for the UI
    pub fn display_name(&self) -> &'static str {
        match self {
            AgentMode::Build => "Build",
            AgentMode::Plan => "Plan",
        }
    }

    /// String representation for storage
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentMode::Build => "build",
            AgentMode::Plan => "plan",
        }
    }

    /// Parse from string
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "plan" => AgentMode::Plan,
            _ => AgentMode::Build,
        }
    }

    /// Toggle between Build and Plan
    pub fn toggle(&self) -> Self {
        match self {
            AgentMode::Build => AgentMode::Plan,
            AgentMode::Plan => AgentMode::Build,
        }
    }
}

impl AgentType {
    /// Preferred provider priority order used for defaults and UI listing.
    pub const fn preferred_order() -> [AgentType; 9] {
        [
            AgentType::Codex,
            AgentType::Claude,
            AgentType::Dirac,
            AgentType::Gemini,
            AgentType::DeepseekTui,
            AgentType::Opencode,
            AgentType::Copilot,
            AgentType::Pi,
            AgentType::Maki,
        ]
    }

    pub fn supports_plan_mode(&self) -> bool {
        matches!(
            self,
            AgentType::Claude
                | AgentType::Codex
                | AgentType::Dirac
                | AgentType::Gemini
                | AgentType::DeepseekTui
                | AgentType::Pi
        )
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AgentType::Claude => "claude",
            AgentType::Codex => "codex",
            AgentType::Dirac => "dirac",
            AgentType::Gemini => "gemini",
            AgentType::DeepseekTui => "deepseek-tui",
            AgentType::Opencode => "opencode",
            AgentType::Copilot => "copilot",
            AgentType::Pi => "pi",
            AgentType::Maki => "maki",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "codex" => AgentType::Codex,
            "dirac" => AgentType::Dirac,
            "gemini" => AgentType::Gemini,
            "deepseek-tui" | "deepseek_tui" | "deepseek" => AgentType::DeepseekTui,
            "opencode" => AgentType::Opencode,
            "copilot" => AgentType::Copilot,
            "pi" => AgentType::Pi,
            "maki" => AgentType::Maki,
            _ => AgentType::Claude,
        }
    }

    /// Short name for role labels in the chat view
    pub fn short_name(&self) -> &'static str {
        match self {
            AgentType::Claude => "Claude",
            AgentType::Codex => "Codex",
            AgentType::Dirac => "Dirac",
            AgentType::Gemini => "Gemini",
            AgentType::DeepseekTui => "DeepSeek",
            AgentType::Opencode => "OpenCode",
            AgentType::Copilot => "Copilot",
            AgentType::Pi => "Pi",
            AgentType::Maki => "Maki",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AgentType::Claude => "Claude Code",
            AgentType::Codex => "Codex CLI",
            AgentType::Dirac => "Dirac CLI",
            AgentType::Gemini => "Gemini CLI",
            AgentType::DeepseekTui => "DeepSeek TUI",
            AgentType::Opencode => "OpenCode",
            AgentType::Copilot => "GitHub Copilot",
            AgentType::Pi => "Pi",
            AgentType::Maki => "Maki",
        }
    }
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
