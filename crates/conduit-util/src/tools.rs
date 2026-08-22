//! Tool availability detection and management
//!
//! This module provides functionality to detect and track the availability
//! of external tools required by Conduit (git, gh, claude, codex, dirac, gemini, deepseek, opencode, pi).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// External tools that Conduit depends on
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    /// Git version control - REQUIRED for core functionality
    Git,
    /// GitHub CLI - optional, needed for PR operations
    Gh,
    /// Claude Code CLI agent
    Claude,
    /// OpenAI Codex CLI agent
    Codex,
    /// Dirac CLI agent
    Dirac,
    /// Google Gemini CLI agent
    Gemini,
    /// DeepSeek TUI agent
    DeepseekTui,
    /// OpenCode CLI agent
    Opencode,
    /// GitHub Copilot CLI agent
    Copilot,
    /// Pi coding agent
    Pi,
    /// Maki coding agent
    Maki,
}

impl Tool {
    /// Get the binary name for this tool
    pub fn binary_name(&self) -> &'static str {
        match self {
            Tool::Git => "git",
            Tool::Gh => "gh",
            Tool::Claude => "claude",
            Tool::Codex => "codex",
            Tool::Dirac => "dirac",
            Tool::Gemini => "gemini",
            Tool::DeepseekTui => "deepseek",
            Tool::Opencode => "opencode",
            Tool::Copilot => "copilot",
            Tool::Pi => "pi",
            Tool::Maki => "maki",
        }
    }

    /// Get the display name for this tool
    pub fn display_name(&self) -> &'static str {
        match self {
            Tool::Git => "Git",
            Tool::Gh => "GitHub CLI",
            Tool::Claude => "Claude Code",
            Tool::Codex => "Codex CLI",
            Tool::Dirac => "Dirac CLI",
            Tool::Gemini => "Gemini CLI",
            Tool::DeepseekTui => "DeepSeek TUI",
            Tool::Opencode => "OpenCode",
            Tool::Copilot => "GitHub Copilot",
            Tool::Pi => "Pi",
            Tool::Maki => "Maki",
        }
    }

    /// Get installation instructions for this tool
    pub fn install_instructions(&self) -> &'static str {
        match self {
            Tool::Git => "brew install git\nhttps://git-scm.com/downloads",
            Tool::Gh => "brew install gh\nhttps://cli.github.com/",
            Tool::Claude => "npm install -g @anthropic-ai/claude-code\nhttps://docs.anthropic.com/en/docs/claude-code",
            Tool::Codex => "npm install -g @openai/codex\nhttps://github.com/openai/codex-cli",
            Tool::Dirac => "npm install -g dirac-cli\nhttps://github.com/dirac-run/dirac",
            Tool::Gemini => "npm install -g @google/gemini-cli\nhttps://github.com/google-gemini/gemini-cli",
            Tool::DeepseekTui => "npm install -g deepseek-tui\nhttps://github.com/Hmbown/DeepSeek-TUI",
            Tool::Opencode => "brew install anomalyco/tap/opencode\nhttps://opencode.ai/docs",
            Tool::Copilot => "https://docs.github.com/en/copilot/concepts/agents/about-copilot-cli",
            Tool::Pi => "npm install -g @mariozechner/pi-coding-agent\nhttps://github.com/badlogic/pi-mono/tree/main/packages/coding-agent",
            Tool::Maki => "curl -fsSL https://maki.sh/install.sh | sh\nhttps://github.com/tontinton/maki",
        }
    }

    /// Get a description of what this tool is used for
    pub fn description(&self) -> &'static str {
        match self {
            Tool::Git => "Conduit manages git worktrees and cannot function without git installed.",
            Tool::Gh => "GitHub CLI is needed for PR operations (create, view, open in browser).",
            Tool::Claude => "Claude Code is an AI coding assistant from Anthropic.",
            Tool::Codex => "Codex is an AI coding assistant from OpenAI.",
            Tool::Dirac => "Dirac is an AI coding assistant focused on efficient context curation.",
            Tool::Gemini => "Gemini CLI is an AI coding assistant from Google.",
            Tool::DeepseekTui => "DeepSeek TUI is a terminal coding agent for DeepSeek models.",
            Tool::Opencode => "OpenCode is a multi-provider AI coding assistant.",
            Tool::Copilot => "GitHub Copilot CLI is an AI coding assistant from GitHub.",
            Tool::Pi => {
                "Pi is a terminal coding agent with persistent sessions and structured RPC."
            }
            Tool::Maki => "Maki is an efficient AI coding agent extendable by Lua plugins.",
        }
    }

    /// Check if this tool is required (app cannot start without it)
    pub fn is_required(&self) -> bool {
        matches!(self, Tool::Git)
    }

    /// Check if this tool is an agent
    pub fn is_agent(&self) -> bool {
        matches!(
            self,
            Tool::Claude
                | Tool::Codex
                | Tool::Dirac
                | Tool::Gemini
                | Tool::DeepseekTui
                | Tool::Opencode
                | Tool::Copilot
                | Tool::Pi
                | Tool::Maki
        )
    }

    /// Get all tools
    pub fn all() -> &'static [Tool] {
        &[
            Tool::Git,
            Tool::Gh,
            Tool::Claude,
            Tool::Codex,
            Tool::Dirac,
            Tool::Gemini,
            Tool::DeepseekTui,
            Tool::Opencode,
            Tool::Copilot,
            Tool::Pi,
            Tool::Maki,
        ]
    }
}

/// Status of a tool's availability
#[derive(Debug, Clone, Default)]
pub enum ToolStatus {
    /// Tool is available at the given path
    Available(PathBuf),
    /// Tool is available via npx at the given path
    AvailableViaNpx(PathBuf),
    /// Tool was not found in PATH or configured location
    #[default]
    NotFound,
    /// A path was configured in config.toml but it's invalid
    ConfiguredPathInvalid(PathBuf),
}

impl ToolStatus {
    /// Check if the tool is available
    pub fn is_available(&self) -> bool {
        matches!(
            self,
            ToolStatus::Available(_) | ToolStatus::AvailableViaNpx(_)
        )
    }

    /// Get the path if available
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            ToolStatus::Available(p) => Some(p),
            // AvailableViaNpx intentionally returns None; runners resolve npx usage internally.
            _ => None,
        }
    }
}

/// Configuration for tool paths from config.toml
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ToolPaths {
    pub git: Option<PathBuf>,
    pub gh: Option<PathBuf>,
    pub claude: Option<PathBuf>,
    pub codex: Option<PathBuf>,
    pub dirac: Option<PathBuf>,
    pub gemini: Option<PathBuf>,
    pub deepseek_tui: Option<PathBuf>,
    pub opencode: Option<PathBuf>,
    pub copilot: Option<PathBuf>,
    pub pi: Option<PathBuf>,
    pub maki: Option<PathBuf>,
}

impl ToolPaths {
    /// Get the configured path for a tool
    pub fn get(&self, tool: Tool) -> Option<&PathBuf> {
        match tool {
            Tool::Git => self.git.as_ref(),
            Tool::Gh => self.gh.as_ref(),
            Tool::Claude => self.claude.as_ref(),
            Tool::Codex => self.codex.as_ref(),
            Tool::Dirac => self.dirac.as_ref(),
            Tool::Gemini => self.gemini.as_ref(),
            Tool::DeepseekTui => self.deepseek_tui.as_ref(),
            Tool::Opencode => self.opencode.as_ref(),
            Tool::Copilot => self.copilot.as_ref(),
            Tool::Pi => self.pi.as_ref(),
            Tool::Maki => self.maki.as_ref(),
        }
    }

    /// Set the path for a tool
    pub fn set(&mut self, tool: Tool, path: PathBuf) {
        match tool {
            Tool::Git => self.git = Some(path),
            Tool::Gh => self.gh = Some(path),
            Tool::Claude => self.claude = Some(path),
            Tool::Codex => self.codex = Some(path),
            Tool::Dirac => self.dirac = Some(path),
            Tool::Gemini => self.gemini = Some(path),
            Tool::DeepseekTui => self.deepseek_tui = Some(path),
            Tool::Opencode => self.opencode = Some(path),
            Tool::Copilot => self.copilot = Some(path),
            Tool::Pi => self.pi = Some(path),
            Tool::Maki => self.maki = Some(path),
        }
    }
}

/// Tracks the availability of all tools
#[derive(Debug, Clone, Default)]
pub struct ToolAvailability {
    git: ToolStatus,
    gh: ToolStatus,
    claude: ToolStatus,
    codex: ToolStatus,
    dirac: ToolStatus,
    gemini: ToolStatus,
    deepseek_tui: ToolStatus,
    opencode: ToolStatus,
    copilot: ToolStatus,
    pi: ToolStatus,
    maki: ToolStatus,
}

impl ToolAvailability {
    /// Detect availability of all tools
    ///
    /// For each tool:
    /// 1. Check if a path is configured in config.toml
    /// 2. If configured, validate that path exists and is executable
    /// 3. If not configured, use `which` to find it in PATH
    pub fn detect(configured_paths: &ToolPaths) -> Self {
        Self {
            git: Self::detect_tool(Tool::Git, configured_paths.git.as_ref()),
            gh: Self::detect_tool(Tool::Gh, configured_paths.gh.as_ref()),
            claude: Self::detect_tool(Tool::Claude, configured_paths.claude.as_ref()),
            codex: Self::detect_tool(Tool::Codex, configured_paths.codex.as_ref()),
            dirac: Self::detect_tool(Tool::Dirac, configured_paths.dirac.as_ref()),
            gemini: Self::detect_tool(Tool::Gemini, configured_paths.gemini.as_ref()),
            deepseek_tui: Self::detect_tool(
                Tool::DeepseekTui,
                configured_paths.deepseek_tui.as_ref(),
            ),
            opencode: Self::detect_tool(Tool::Opencode, configured_paths.opencode.as_ref()),
            copilot: Self::detect_tool(Tool::Copilot, configured_paths.copilot.as_ref()),
            pi: Self::detect_tool(Tool::Pi, configured_paths.pi.as_ref()),
            maki: Self::detect_tool(Tool::Maki, configured_paths.maki.as_ref()),
        }
    }

    /// Detect a single tool's availability
    fn detect_tool(tool: Tool, configured_path: Option<&PathBuf>) -> ToolStatus {
        // If a path is configured, validate it
        if let Some(path) = configured_path {
            if Self::is_valid_executable(path) {
                return ToolStatus::Available(path.clone());
            } else {
                return ToolStatus::ConfiguredPathInvalid(path.clone());
            }
        }

        // Otherwise, try to find it in PATH using `which`
        match which::which(tool.binary_name()) {
            Ok(path) => ToolStatus::Available(path),
            Err(_) => {
                if matches!(tool, Tool::Codex | Tool::Gemini) {
                    match which::which("npx") {
                        Ok(path) => ToolStatus::AvailableViaNpx(path),
                        Err(_) => ToolStatus::NotFound,
                    }
                } else {
                    ToolStatus::NotFound
                }
            }
        }
    }

    /// Check if a path points to a valid executable
    fn is_valid_executable(path: &Path) -> bool {
        if !path.exists() {
            return false;
        }

        // On Unix, check if the file is executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = path.metadata() {
                let permissions = metadata.permissions();
                return permissions.mode() & 0o111 != 0;
            }
            false
        }

        // On Windows, just check if the file exists
        #[cfg(not(unix))]
        {
            path.is_file()
        }
    }

    /// Get the status for a specific tool
    pub fn status(&self, tool: Tool) -> &ToolStatus {
        match tool {
            Tool::Git => &self.git,
            Tool::Gh => &self.gh,
            Tool::Claude => &self.claude,
            Tool::Codex => &self.codex,
            Tool::Dirac => &self.dirac,
            Tool::Gemini => &self.gemini,
            Tool::DeepseekTui => &self.deepseek_tui,
            Tool::Opencode => &self.opencode,
            Tool::Copilot => &self.copilot,
            Tool::Pi => &self.pi,
            Tool::Maki => &self.maki,
        }
    }

    /// Check if a tool is available
    pub fn is_available(&self, tool: Tool) -> bool {
        self.status(tool).is_available()
    }

    /// Get the path to a tool if available
    pub fn get_path(&self, tool: Tool) -> Option<&PathBuf> {
        self.status(tool).path()
    }

    /// Get list of missing tools
    pub fn missing_tools(&self) -> Vec<Tool> {
        Tool::all()
            .iter()
            .filter(|&&tool| !self.is_available(tool))
            .copied()
            .collect()
    }

    /// Get list of missing required tools
    pub fn missing_required_tools(&self) -> Vec<Tool> {
        self.missing_tools()
            .into_iter()
            .filter(|tool| tool.is_required())
            .collect()
    }

    /// Check if at least one agent is available
    pub fn has_any_agent(&self) -> bool {
        self.is_available(Tool::Claude)
            || self.is_available(Tool::Codex)
            || self.is_available(Tool::Dirac)
            || self.is_available(Tool::Gemini)
            || self.is_available(Tool::DeepseekTui)
            || self.is_available(Tool::Opencode)
            || self.is_available(Tool::Copilot)
            || self.is_available(Tool::Pi)
            || self.is_available(Tool::Maki)
    }

    /// Get list of available agents
    pub fn available_agents(&self) -> Vec<Tool> {
        Tool::all()
            .iter()
            .filter(|&&tool| tool.is_agent() && self.is_available(tool))
            .copied()
            .collect()
    }

    /// Update the status for a single tool (after user provides path)
    ///
    /// Returns true if the path is valid and the tool is now available
    pub fn update_tool(&mut self, tool: Tool, path: PathBuf) -> bool {
        let status = if Self::is_valid_executable(&path) {
            ToolStatus::Available(path)
        } else {
            ToolStatus::ConfiguredPathInvalid(path)
        };

        let is_available = status.is_available();

        match tool {
            Tool::Git => self.git = status,
            Tool::Gh => self.gh = status,
            Tool::Claude => self.claude = status,
            Tool::Codex => self.codex = status,
            Tool::Dirac => self.dirac = status,
            Tool::Gemini => self.gemini = status,
            Tool::DeepseekTui => self.deepseek_tui = status,
            Tool::Opencode => self.opencode = status,
            Tool::Copilot => self.copilot = status,
            Tool::Pi => self.pi = status,
            Tool::Maki => self.maki = status,
        }

        is_available
    }

    /// Validate a path for a tool without updating state
    ///
    /// Returns Ok(canonical_path) if valid, Err(message) if invalid
    pub fn validate_path(path: &str) -> Result<PathBuf, String> {
        let path = PathBuf::from(path);

        if path.as_os_str().is_empty() {
            return Err("Path cannot be empty".to_string());
        }

        if !path.exists() {
            return Err(format!("File not found: {}", path.display()));
        }

        if !path.is_file() {
            return Err(format!("Not a file: {}", path.display()));
        }

        if !Self::is_valid_executable(&path) {
            return Err(format!("File is not executable: {}", path.display()));
        }

        // Return canonical path
        path.canonicalize()
            .map_err(|e| format!("Failed to resolve path: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_binary_names() {
        assert_eq!(Tool::Git.binary_name(), "git");
        assert_eq!(Tool::Gh.binary_name(), "gh");
        assert_eq!(Tool::Claude.binary_name(), "claude");
        assert_eq!(Tool::Codex.binary_name(), "codex");
        assert_eq!(Tool::Gemini.binary_name(), "gemini");
        assert_eq!(Tool::Opencode.binary_name(), "opencode");
    }

    #[test]
    fn test_tool_is_required() {
        assert!(Tool::Git.is_required());
        assert!(!Tool::Gh.is_required());
        assert!(!Tool::Claude.is_required());
        assert!(!Tool::Codex.is_required());
        assert!(!Tool::Gemini.is_required());
        assert!(!Tool::Opencode.is_required());
    }

    #[test]
    fn test_tool_is_agent() {
        assert!(!Tool::Git.is_agent());
        assert!(!Tool::Gh.is_agent());
        assert!(Tool::Claude.is_agent());
        assert!(Tool::Codex.is_agent());
        assert!(Tool::Gemini.is_agent());
        assert!(Tool::Opencode.is_agent());
    }

    #[test]
    fn test_tool_status_is_available() {
        assert!(ToolStatus::Available(PathBuf::from("/bin/test")).is_available());
        assert!(ToolStatus::AvailableViaNpx(PathBuf::from("/bin/npx")).is_available());
        assert!(!ToolStatus::NotFound.is_available());
        assert!(!ToolStatus::ConfiguredPathInvalid(PathBuf::from("/bad")).is_available());
    }

    #[test]
    fn test_detect_git_in_path() {
        // Git should almost always be available in development environments
        let availability = ToolAvailability::detect(&ToolPaths::default());
        // We can't guarantee git is installed, so just test the structure
        let status = availability.status(Tool::Git);
        assert!(matches!(
            status,
            ToolStatus::Available(_) | ToolStatus::NotFound
        ));
    }
}
