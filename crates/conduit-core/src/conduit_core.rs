//! Core infrastructure shared between TUI and web interfaces.

use std::sync::Arc;

use conduit_agent::codex::load_codex_models;
use conduit_agent::deepseek_tui::DeepseekTuiRunner;
use conduit_agent::gemini::load_gemini_models;
use conduit_agent::pi::load_pi_models;
use conduit_agent::{
    ClaudeCodeRunner, CodexCliRunner, CopilotRunner, GeminiCliRunner, MakiRunner, ModelRegistry,
    OpencodeRunner, PiRunner,
};
use conduit_config::Config;
use conduit_data::{
    AppStateStore, Database, ForkSeedStore, RepositoryStore, SessionTabStore, WorkspaceStore,
};
use conduit_git::WorkspaceRepoManager;
use conduit_util::{Tool, ToolAvailability};

/// Core infrastructure for Conduit, shared between TUI and web interfaces.
///
/// This struct owns all the foundational components:
/// - Database connection and DAO stores for persistent data
/// - Agent runners for Claude, Codex, Gemini, and OpenCode
/// - Configuration and tool availability
/// - Worktree manager for git workspace operations
pub struct ConduitCore {
    /// Application configuration
    config: Config,
    /// Tool availability (git, gh, claude, codex, gemini, opencode)
    tools: ToolAvailability,
    /// Database connection (owned to keep connection alive)
    _database: Option<Database>,
    /// Repository DAO
    repo_store: Option<RepositoryStore>,
    /// Workspace DAO
    workspace_store: Option<WorkspaceStore>,
    /// App state DAO (for persisting app settings)
    app_state_store: Option<AppStateStore>,
    /// Session tab DAO (for persisting open tabs)
    session_tab_store: Option<SessionTabStore>,
    /// Fork seed DAO (for persisting fork metadata)
    fork_seed_store: Option<ForkSeedStore>,
    /// Claude Code runner
    claude_runner: Arc<ClaudeCodeRunner>,
    /// Codex CLI runner
    codex_runner: Arc<CodexCliRunner>,
    /// Gemini CLI runner
    gemini_runner: Arc<GeminiCliRunner>,
    /// DeepSeek TUI runner
    deepseek_tui_runner: Arc<DeepseekTuiRunner>,
    /// OpenCode runner
    opencode_runner: Arc<OpencodeRunner>,
    /// GitHub Copilot runner
    copilot_runner: Arc<CopilotRunner>,
    /// Pi runner
    pi_runner: Arc<PiRunner>,
    /// Maki runner
    maki_runner: Arc<MakiRunner>,
    /// Worktree manager
    worktree_manager: WorkspaceRepoManager,
}

impl ConduitCore {
    /// Create a new ConduitCore with the given configuration and tool availability.
    pub fn new(config: Config, tools: ToolAvailability) -> Self {
        Self::new_with_progress(config, tools, &mut |_| {})
    }

    /// Like `new`, but calls `progress` with a human-readable label at each initialization phase.
    pub fn new_with_progress(
        config: Config,
        tools: ToolAvailability,
        progress: &mut impl FnMut(&str),
    ) -> Self {
        progress("Connecting to database");
        // Initialize database and DAOs
        let (
            database,
            repo_store,
            workspace_store,
            app_state_store,
            session_tab_store,
            fork_seed_store,
        ) = match Database::open_default() {
            Ok(db) => {
                let repo_store = RepositoryStore::new(db.connection());
                let workspace_store = WorkspaceStore::new(db.connection());
                let app_state_store = AppStateStore::new(db.connection());
                let session_tab_store = SessionTabStore::new(db.connection());
                let fork_seed_store = ForkSeedStore::new(db.connection());
                (
                    Some(db),
                    Some(repo_store),
                    Some(workspace_store),
                    Some(app_state_store),
                    Some(session_tab_store),
                    Some(fork_seed_store),
                )
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to open database");
                (None, None, None, None, None, None)
            }
        };

        // Migrate old worktrees folder to workspaces (one-time migration)
        conduit_util::migrate_worktrees_to_workspaces();

        // Initialize worktree manager with managed directory (~/.conduit/workspaces)
        let worktree_manager =
            WorkspaceRepoManager::with_managed_dir(conduit_util::workspaces_dir());

        // Create runners with configured paths if available, reporting each available agent
        if tools.is_available(Tool::Claude) {
            progress("Initializing Claude Code");
        }
        let claude_runner = match tools.get_path(Tool::Claude) {
            Some(path) => Arc::new(ClaudeCodeRunner::with_path(path.clone())),
            None => Arc::new(ClaudeCodeRunner::new()),
        };

        if tools.is_available(Tool::Codex) {
            progress("Initializing Codex");
        }
        let codex_runner = match tools.get_path(Tool::Codex) {
            Some(path) => Arc::new(CodexCliRunner::with_path(path.clone())),
            None => Arc::new(CodexCliRunner::new()),
        };

        if tools.is_available(Tool::Gemini) {
            progress("Initializing Gemini");
        }
        let gemini_runner = match tools.get_path(Tool::Gemini) {
            Some(path) => Arc::new(GeminiCliRunner::with_path(path.clone())),
            None => Arc::new(GeminiCliRunner::new()),
        };

        if tools.is_available(Tool::DeepseekTui) {
            progress("Initializing DeepSeek TUI");
        }
        let deepseek_tui_runner = match tools.get_path(Tool::DeepseekTui) {
            Some(path) => Arc::new(DeepseekTuiRunner::with_path(path.clone())),
            None => Arc::new(DeepseekTuiRunner::new()),
        };

        if tools.is_available(Tool::Opencode) {
            progress("Initializing OpenCode");
        }
        let opencode_runner = match tools.get_path(Tool::Opencode) {
            Some(path) => Arc::new(OpencodeRunner::with_path(path.clone())),
            None => Arc::new(OpencodeRunner::new()),
        };

        if tools.is_available(Tool::Copilot) {
            progress("Initializing GitHub Copilot");
        }
        let copilot_runner = match tools.get_path(Tool::Copilot) {
            Some(path) => Arc::new(CopilotRunner::with_path(path.clone())),
            None => Arc::new(CopilotRunner::new()),
        };

        if tools.is_available(Tool::Pi) {
            progress("Initializing Pi");
        }
        let pi_runner = match tools.get_path(Tool::Pi) {
            Some(path) => Arc::new(PiRunner::with_path(path.clone())),
            None => Arc::new(PiRunner::new()),
        };

        if tools.is_available(Tool::Maki) {
            progress("Initializing Maki");
        }
        let maki_runner = match tools.get_path(Tool::Maki) {
            Some(path) => Arc::new(MakiRunner::with_path(path.clone())),
            None => Arc::new(MakiRunner::new()),
        };

        if tools.is_available(Tool::Claude) {
            progress("Discovering Claude Code models");
            let models =
                conduit_agent::claude::load_claude_models(tools.get_path(Tool::Claude).cloned());
            if !models.is_empty() {
                ModelRegistry::set_claude_models(models);
            }
        } else {
            ModelRegistry::clear_claude_models();
        }

        if tools.is_available(Tool::Opencode) {
            progress("Discovering OpenCode models");
            let models = conduit_agent::opencode::load_opencode_models(
                tools.get_path(Tool::Opencode).cloned(),
            );
            ModelRegistry::set_opencode_models(models);
        } else {
            ModelRegistry::clear_opencode_models();
        }

        if tools.is_available(Tool::Codex) {
            progress("Discovering Codex models");
            let models = load_codex_models(tools.get_path(Tool::Codex).cloned());
            if !models.is_empty() {
                ModelRegistry::set_codex_models(models);
            }
        } else {
            ModelRegistry::clear_codex_models();
        }

        if tools.is_available(Tool::Gemini) {
            progress("Discovering Gemini models");
            let models = load_gemini_models();
            if !models.is_empty() {
                ModelRegistry::set_gemini_models(models);
            }
        } else {
            ModelRegistry::clear_gemini_models();
        }

        if tools.is_available(Tool::Pi) {
            progress("Discovering Pi models");
            let models = load_pi_models(tools.get_path(Tool::Pi).cloned());
            if !models.is_empty() {
                ModelRegistry::set_pi_models(models);
            }
        } else {
            ModelRegistry::clear_pi_models();
        }

        Self {
            config,
            tools,
            _database: database,
            repo_store,
            workspace_store,
            app_state_store,
            session_tab_store,
            fork_seed_store,
            claude_runner,
            codex_runner,
            gemini_runner,
            deepseek_tui_runner,
            opencode_runner,
            copilot_runner,
            pi_runner,
            maki_runner,
            worktree_manager,
        }
    }

    /// Get the application configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get the tool availability.
    pub fn tools(&self) -> &ToolAvailability {
        &self.tools
    }

    /// Get the repository store.
    pub fn repo_store(&self) -> Option<&RepositoryStore> {
        self.repo_store.as_ref()
    }

    /// Get a clone of the repository store.
    pub fn repo_store_clone(&self) -> Option<RepositoryStore> {
        self.repo_store.clone()
    }

    /// Get the workspace store.
    pub fn workspace_store(&self) -> Option<&WorkspaceStore> {
        self.workspace_store.as_ref()
    }

    /// Get a clone of the workspace store.
    pub fn workspace_store_clone(&self) -> Option<WorkspaceStore> {
        self.workspace_store.clone()
    }

    /// Get the app state store.
    pub fn app_state_store(&self) -> Option<&AppStateStore> {
        self.app_state_store.as_ref()
    }

    /// Get a clone of the app state store.
    pub fn app_state_store_clone(&self) -> Option<AppStateStore> {
        self.app_state_store.clone()
    }

    /// Get the session tab store.
    pub fn session_tab_store(&self) -> Option<&SessionTabStore> {
        self.session_tab_store.as_ref()
    }

    /// Get a clone of the session tab store.
    pub fn session_tab_store_clone(&self) -> Option<SessionTabStore> {
        self.session_tab_store.clone()
    }

    /// Get the fork seed store.
    pub fn fork_seed_store(&self) -> Option<&ForkSeedStore> {
        self.fork_seed_store.as_ref()
    }

    /// Get a clone of the fork seed store.
    pub fn fork_seed_store_clone(&self) -> Option<ForkSeedStore> {
        self.fork_seed_store.clone()
    }

    /// Get the Claude runner.
    pub fn claude_runner(&self) -> &Arc<ClaudeCodeRunner> {
        &self.claude_runner
    }

    /// Get the Codex runner.
    pub fn codex_runner(&self) -> &Arc<CodexCliRunner> {
        &self.codex_runner
    }

    /// Get the Gemini runner.
    pub fn gemini_runner(&self) -> &Arc<GeminiCliRunner> {
        &self.gemini_runner
    }

    /// Get the DeepSeek TUI runner.
    pub fn deepseek_tui_runner(&self) -> &Arc<DeepseekTuiRunner> {
        &self.deepseek_tui_runner
    }

    /// Get the OpenCode runner.
    pub fn opencode_runner(&self) -> &Arc<OpencodeRunner> {
        &self.opencode_runner
    }

    /// Get the GitHub Copilot runner.
    pub fn copilot_runner(&self) -> &Arc<CopilotRunner> {
        &self.copilot_runner
    }

    /// Get the Pi runner.
    pub fn pi_runner(&self) -> &Arc<PiRunner> {
        &self.pi_runner
    }

    /// Get the Maki runner.
    pub fn maki_runner(&self) -> &Arc<MakiRunner> {
        &self.maki_runner
    }

    /// Get the worktree manager.
    pub fn worktree_manager(&self) -> &WorkspaceRepoManager {
        &self.worktree_manager
    }

    /// Get a mutable reference to the worktree manager.
    pub fn worktree_manager_mut(&mut self) -> &mut WorkspaceRepoManager {
        &mut self.worktree_manager
    }

    /// Get a mutable reference to the tool availability.
    pub fn tools_mut(&mut self) -> &mut ToolAvailability {
        &mut self.tools
    }

    /// Get a mutable reference to the config.
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Re-run tool detection and refresh all agent runners.
    ///
    /// Call this when the user opens a selector that shows available providers,
    /// so newly installed tools are reflected without restarting.
    pub fn redetect_tools(&mut self) {
        self.tools = ToolAvailability::detect(&self.config.tool_paths);
        self.refresh_runners();
    }

    /// Refresh agent runners using the latest tool configuration.
    ///
    /// This should be called after updating tool paths (e.g., when the user
    /// provides a custom path for a missing tool).
    pub fn refresh_runners(&mut self) {
        self.claude_runner = match self.tools.get_path(Tool::Claude) {
            Some(path) => Arc::new(ClaudeCodeRunner::with_path(path.clone())),
            None => Arc::new(ClaudeCodeRunner::new()),
        };
        self.codex_runner = match self.tools.get_path(Tool::Codex) {
            Some(path) => Arc::new(CodexCliRunner::with_path(path.clone())),
            None => Arc::new(CodexCliRunner::new()),
        };
        self.gemini_runner = match self.tools.get_path(Tool::Gemini) {
            Some(path) => Arc::new(GeminiCliRunner::with_path(path.clone())),
            None => Arc::new(GeminiCliRunner::new()),
        };
        self.deepseek_tui_runner = match self.tools.get_path(Tool::DeepseekTui) {
            Some(path) => Arc::new(DeepseekTuiRunner::with_path(path.clone())),
            None => Arc::new(DeepseekTuiRunner::new()),
        };
        self.opencode_runner = match self.tools.get_path(Tool::Opencode) {
            Some(path) => Arc::new(OpencodeRunner::with_path(path.clone())),
            None => Arc::new(OpencodeRunner::new()),
        };
        self.copilot_runner = match self.tools.get_path(Tool::Copilot) {
            Some(path) => Arc::new(CopilotRunner::with_path(path.clone())),
            None => Arc::new(CopilotRunner::new()),
        };
        self.pi_runner = match self.tools.get_path(Tool::Pi) {
            Some(path) => Arc::new(PiRunner::with_path(path.clone())),
            None => Arc::new(PiRunner::new()),
        };
        self.maki_runner = match self.tools.get_path(Tool::Maki) {
            Some(path) => Arc::new(MakiRunner::with_path(path.clone())),
            None => Arc::new(MakiRunner::new()),
        };

        if self.tools.is_available(Tool::Claude) {
            let models = conduit_agent::claude::load_claude_models(
                self.tools.get_path(Tool::Claude).cloned(),
            );
            if !models.is_empty() {
                ModelRegistry::set_claude_models(models);
            }
        } else {
            ModelRegistry::clear_claude_models();
        }

        if self.tools.is_available(Tool::Opencode) {
            let models = conduit_agent::opencode::load_opencode_models(
                self.tools.get_path(Tool::Opencode).cloned(),
            );
            ModelRegistry::set_opencode_models(models);
        } else {
            ModelRegistry::clear_opencode_models();
        }

        if self.tools.is_available(Tool::Codex) {
            let models = load_codex_models(self.tools.get_path(Tool::Codex).cloned());
            if !models.is_empty() {
                ModelRegistry::set_codex_models(models);
            }
        } else {
            ModelRegistry::clear_codex_models();
        }

        if self.tools.is_available(Tool::Gemini) {
            let models = load_gemini_models();
            if !models.is_empty() {
                ModelRegistry::set_gemini_models(models);
            }
        } else {
            ModelRegistry::clear_gemini_models();
        }

        if self.tools.is_available(Tool::Pi) {
            let models = load_pi_models(self.tools.get_path(Tool::Pi).cloned());
            if !models.is_empty() {
                ModelRegistry::set_pi_models(models);
            }
        } else {
            ModelRegistry::clear_pi_models();
        }
    }
}
