use std::io;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::{
    event::{
        EnableBracketedPaste, EnableMouseCapture, Event, EventStream, KeyCode, KeyModifiers,
        MouseEventKind,
    },
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::action::Action;
use crate::app_prompt;
use crate::app_queue;
use crate::app_state::{
    AppState, BaseDirDialogContext, ModelPickerContext, NewProjectTarget, PendingSessionConfig,
};
#[cfg(test)]
use crate::app_state::{PendingForkRequest, PendingHandoffRequest};
use crate::capabilities::AgentCapabilities;
use crate::components::{
    build_keybinding_items, ChatMessage, ConfirmationContext, ConfirmationType,
    DefaultModelSelection, McpServer, McpSource, MessageRole, ProcessingState, ProjectEntry,
    SettingsMenuEntry, SettingsMenuEntryId, SidebarData, WorkspaceDefaultsDraft,
};
#[cfg(test)]
use crate::components::{dialog_content_area, EventDirection};
use crate::effect::Effect;
use crate::events::{
    AppEvent, ForkWorkspaceCreated, InputMode, ProjectDiscoveryEntry,
    RemoveProjectDialogPreflightResult, RemoveProjectResult, TitleGeneratedResult,
    WorkspaceCreated,
};
#[cfg(test)]
use crate::events::{ForkSessionDialogPreflightResult, ViewMode};
use crate::session::AgentSession;
use crate::terminal_guard::TerminalGuard;
use conduit_agent::events::UserQuestion;
#[cfg(test)]
use conduit_agent::AgentEvent;
use conduit_agent::{
    load_claude_history_with_debug, load_codex_history_with_debug,
    load_opencode_history_for_dir_with_debug, load_opencode_history_with_debug,
    load_pi_history_with_debug, AgentMode, AgentRunner, AgentType, ClaudeCodeRunner,
    CodexCliRunner, CopilotRunner, DeepseekTuiRunner, DiracRunner, GeminiCliRunner,
    HistoryDebugEntry, MessageDisplay, ModelRegistry, OpencodeRunner, PiRunner, SessionId,
};
#[cfg(test)]
use conduit_config::KeyContext;
use conduit_config::{parse_action, Config, COMMAND_NAMES};
use conduit_core::resolve_repo_workspace_settings;
use conduit_core::ConduitCore;
use conduit_data::{
    AppStateStore, ForkSeedStore, QueuedMessage, QueuedMessageMode, Repository, RepositoryStore,
    SessionTab, SessionTabStore, WorkspaceStore,
};
use conduit_git::{PrManager, PrStatus, WorkspaceMode, WorkspaceRepoManager};
use conduit_resolver::{CommandResolver, ConduitCommand, MenuEntryKind};
use conduit_util::ToolAvailability;
#[cfg(test)]
use ratatui::layout::Rect;

mod app_actions_confirm;
mod app_actions_confirmation;
mod app_actions_dialog;
mod app_actions_global;
mod app_actions_input_edit;
mod app_actions_list;
mod app_actions_overlay;
mod app_actions_pr;
mod app_actions_queue;
mod app_actions_raw_events;
mod app_actions_scroll;
mod app_actions_sidebar;
mod app_actions_submit;
mod app_actions_tabs;
mod app_agent_events;
mod app_input;
mod app_mouse;
mod app_render;
mod app_scroll;
mod app_selection;
mod app_submit_action;

#[cfg(target_os = "macos")]
const PROC_PIDTBSDINFO: libc::c_int = 3;

#[cfg(target_os = "macos")]
const MAXCOMLEN: usize = 16;

#[cfg(target_os = "macos")]
#[repr(C)]
struct ProcBsdInfo {
    pbi_flags: u32,
    pbi_status: u32,
    pbi_xstatus: u32,
    pbi_pid: u32,
    pbi_ppid: u32,
    pbi_uid: libc::uid_t,
    pbi_gid: libc::gid_t,
    pbi_ruid: libc::uid_t,
    pbi_rgid: libc::gid_t,
    pbi_svuid: libc::uid_t,
    pbi_svgid: libc::gid_t,
    rfu_1: u32,
    pbi_comm: [u8; MAXCOMLEN],
    pbi_name: [u8; 2 * MAXCOMLEN],
    pbi_nfiles: u32,
    pbi_pgid: u32,
    pbi_pjobc: u32,
    e_tdev: u32,
    e_tpgid: u32,
    pbi_nice: i32,
    pbi_start_tvsec: u64,
    pbi_start_tvusec: u64,
}

#[cfg(target_os = "macos")]
extern "C" {
    fn proc_pidinfo(
        pid: libc::c_int,
        flavor: libc::c_int,
        arg: u64,
        buffer: *mut libc::c_void,
        buffersize: libc::c_int,
    ) -> libc::c_int;
}

/// Timeout for double-press detection (ms)
const DOUBLE_PRESS_TIMEOUT_MS: u64 = 500;
/// Timeout for shell command execution.
const SHELL_COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

/// Wrapper for AskUserQuestion tool arguments
#[derive(serde::Deserialize)]
struct AskUserQuestionWrapper {
    questions: Vec<UserQuestion>,
}

/// Wrapper for ExitPlanMode tool arguments
#[derive(serde::Deserialize)]
struct ExitPlanModeWrapper {
    plan: String,
}
// 20s allows slow CLI agents to shut down on congested machines without UI hangs.
const AGENT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(20);
// 500ms grace keeps UI responsive while giving SIGTERM a brief chance to exit.
const AGENT_TERMINATION_GRACE: Duration = Duration::from_millis(500);
// 50ms polling keeps wait loops short without a busy spin.
const AGENT_TERMINATION_POLL_INTERVAL: Duration = Duration::from_millis(50);
// Limit shell output to keep memory bounded.
const SHELL_COMMAND_OUTPUT_LIMIT: usize = 1024 * 1024;
// Bound process reaping after a timeout.
const SHELL_COMMAND_REAP_TIMEOUT: Duration = Duration::from_secs(2);
const PLAN_MODE_INLINE_REMINDER_ENV: &str = "CONDUIT_PLAN_MODE_INLINE_REMINDER";

/// Main application state
pub struct App {
    /// Core infrastructure (database, runners, config)
    core: ConduitCore,
    /// In-memory UI state
    state: AppState,
    /// Event channel sender
    event_tx: mpsc::UnboundedSender<AppEvent>,
    /// Event channel receiver
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
    /// Background git/PR status tracker
    git_tracker: Option<crate::git_tracker::GitTrackerHandle>,
    /// When true, render to the primary buffer (no alternate screen) for screenshot capture
    demo_mode: bool,
}

// Convenience accessors for backward compatibility during refactoring
impl App {
    /// Get the application configuration.
    #[inline]
    fn config(&self) -> &Config {
        self.core.config()
    }

    /// Get the tool availability.
    #[inline]
    fn tools(&self) -> &ToolAvailability {
        self.core.tools()
    }

    /// Get the repository DAO.
    #[inline]
    fn repo_dao(&self) -> Option<&RepositoryStore> {
        self.core.repo_store()
    }

    fn repo_dao_clone(&self) -> Option<RepositoryStore> {
        self.core.repo_store_clone()
    }

    /// Get the workspace DAO.
    #[inline]
    fn workspace_dao(&self) -> Option<&WorkspaceStore> {
        self.core.workspace_store()
    }

    /// Get a clone of the workspace DAO.
    #[inline]
    fn workspace_dao_clone(&self) -> Option<WorkspaceStore> {
        self.core.workspace_store_clone()
    }

    /// Get the app state DAO.
    #[inline]
    fn app_state_dao(&self) -> Option<&AppStateStore> {
        self.core.app_state_store()
    }

    /// Get a clone of the app state DAO.
    #[inline]
    fn app_state_dao_clone(&self) -> Option<AppStateStore> {
        self.core.app_state_store_clone()
    }

    /// Get the session tab DAO.
    #[inline]
    fn session_tab_dao(&self) -> Option<&SessionTabStore> {
        self.core.session_tab_store()
    }

    /// Get a clone of the session tab DAO.
    #[inline]
    fn session_tab_dao_clone(&self) -> Option<SessionTabStore> {
        self.core.session_tab_store_clone()
    }

    /// Get the fork seed DAO.
    #[inline]
    fn fork_seed_dao(&self) -> Option<&ForkSeedStore> {
        self.core.fork_seed_store()
    }

    /// Get a clone of the fork seed DAO.
    #[inline]
    #[allow(dead_code)] // Will be used by web interface
    fn fork_seed_dao_clone(&self) -> Option<ForkSeedStore> {
        self.core.fork_seed_store_clone()
    }

    /// Get the Claude runner.
    #[inline]
    fn claude_runner(&self) -> &Arc<ClaudeCodeRunner> {
        self.core.claude_runner()
    }

    /// Get the Codex runner.
    #[inline]
    fn codex_runner(&self) -> &Arc<CodexCliRunner> {
        self.core.codex_runner()
    }

    /// Get the Gemini runner.
    #[inline]
    fn gemini_runner(&self) -> &Arc<GeminiCliRunner> {
        self.core.gemini_runner()
    }

    /// Get the DeepSeek TUI runner.
    #[inline]
    fn deepseek_tui_runner(&self) -> &Arc<DeepseekTuiRunner> {
        self.core.deepseek_tui_runner()
    }

    /// Get the Dirac runner.
    #[inline]
    fn dirac_runner(&self) -> &Arc<DiracRunner> {
        self.core.dirac_runner()
    }

    /// Get the OpenCode runner.
    #[inline]
    fn opencode_runner(&self) -> &Arc<OpencodeRunner> {
        self.core.opencode_runner()
    }

    /// Get the GitHub Copilot runner.
    #[inline]
    fn copilot_runner(&self) -> &Arc<CopilotRunner> {
        self.core.copilot_runner()
    }

    /// Get the Pi runner.
    #[inline]
    fn pi_runner(&self) -> &Arc<PiRunner> {
        self.core.pi_runner()
    }

    /// Get the worktree manager.
    #[inline]
    fn worktree_manager(&self) -> &WorkspaceRepoManager {
        self.core.worktree_manager()
    }

    /// Get a mutable reference to the worktree manager.
    #[inline]
    #[allow(dead_code)] // Will be used by web interface
    fn worktree_manager_mut(&mut self) -> &mut WorkspaceRepoManager {
        self.core.worktree_manager_mut()
    }

    /// Get a mutable reference to the tools.
    #[inline]
    fn tools_mut(&mut self) -> &mut ToolAvailability {
        self.core.tools_mut()
    }

    /// Get a mutable reference to the config.
    #[inline]
    fn config_mut(&mut self) -> &mut Config {
        self.core.config_mut()
    }

    /// Refresh agent runners (delegates to core) and update UI state.
    fn refresh_runners(&mut self) {
        self.core.refresh_runners();
        let tools = self.tools().clone();
        self.state
            .agent_selector_state
            .update_available_agents(&tools);
        if self.state.provider_selector_state.is_visible() {
            self.state.provider_selector_state =
                crate::components::ProviderSelectorState::configure_for(self.config(), &tools);
            self.state.provider_selector_state.show();
        }
    }

    /// Re-detect installed tools and refresh runners and UI selector state.
    fn redetect_tools(&mut self) {
        self.core.redetect_tools();
        let tools = self.tools().clone();
        self.state
            .agent_selector_state
            .update_available_agents(&tools);
    }
}

fn send_app_event(
    event_tx: &mpsc::UnboundedSender<AppEvent>,
    event: AppEvent,
    context: &'static str,
) -> bool {
    match event_tx.send(event) {
        Ok(()) => true,
        Err(err) => {
            let event_kind = std::mem::discriminant(&err.0);
            tracing::debug!(
                context,
                event_kind = ?event_kind,
                receiver_dropped = true,
                "Failed to send AppEvent"
            );
            false
        }
    }
}

impl App {
    // When true, selection drag auto-scrolls as soon as the cursor hits the first/last row.
    // When false, auto-scroll only starts after the cursor leaves the chat area.
    const AUTO_SCROLL_ON_EDGE_INCLUSIVE: bool = true;
    pub fn new(config: Config, tools: ToolAvailability) -> Self {
        Self::new_with_progress(config, tools, |_| {})
    }

    /// Like `new`, but calls `progress` with a human-readable label at each initialization phase.
    pub(crate) fn new_with_progress(
        config: Config,
        tools: ToolAvailability,
        mut progress: impl FnMut(&str),
    ) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Create core infrastructure (database, runners, worktree manager)
        let core = ConduitCore::new_with_progress(config.clone(), tools, &mut progress);

        // Initialize git tracker
        let (git_update_tx, mut git_update_rx) = mpsc::unbounded_channel();
        let git_tracker = Some(crate::git_tracker::spawn_git_tracker(git_update_tx));

        // Forward git tracker updates to main event channel
        let event_tx_for_tracker = event_tx.clone();
        tokio::spawn(async move {
            while let Some(update) = git_update_rx.recv().await {
                if event_tx_for_tracker
                    .send(AppEvent::GitTracker(update))
                    .is_err()
                {
                    break;
                }
            }
        });

        let mut app = Self {
            core,
            state: AppState::new(config.max_tabs),
            event_tx,
            event_rx,
            git_tracker,
            demo_mode: false,
        };

        // Update agent selector based on available tools
        let tools = app.tools().clone();
        app.state
            .agent_selector_state
            .update_available_agents(&tools);

        progress("Loading projects");

        progress("Restoring sessions");
        app.restore_session_state();
        app.refresh_sidebar_data();
        app.sync_theme_to_active_tab();

        // Honour always_show_sidebar: force visible regardless of saved state
        if app.config().ui.always_show_sidebar {
            app.state.sidebar_state.visible = true;
        }

        app
    }

    /// Overwrite app state with hardcoded demo data (for `conduit demo`).
    /// Load a minimal demo state that shows the splash screen with a single repo
    /// in the sidebar but no active workspace tab — used for clean-start screenshots.
    pub fn load_demo_data_splash(&mut self) {
        use crate::demo;

        self.demo_mode = true;

        self.state.show_first_time_splash = false;
        self.state.sidebar_state.visible = true;
        self.state.sidebar_data = crate::components::SidebarData::new();
        while !self.state.tab_manager.is_empty() {
            self.state.tab_manager.close_tab(0);
        }

        // One repo with no workspaces — sidebar shows just "New workspace"
        let splash_repos = vec![demo::DemoRepo {
            id: uuid::Uuid::new_v4(),
            name: "conduit".to_string(),
            workspaces: vec![],
        }];
        demo::populate_sidebar(&mut self.state.sidebar_data, &splash_repos);

        // Select "+ New workspace" (index 1 in the visible list)
        let visible = self.state.sidebar_data.visible_nodes();
        if visible.len() > 1 {
            self.state.sidebar_state.tree_state.selected = 1;
        }
        // Tab manager remains empty → splash screen renders
    }

    /// Pre-open a UI overlay so VHS screenshot tapes don't need to drive keyboard input.
    /// Called immediately after `load_demo_data()` or `load_demo_data_splash()`.
    pub fn open_overlay_for_demo(&mut self, overlay: &str) {
        match overlay {
            "help" => {
                self.state.close_overlays();
                let keybindings = self.config().keybindings.clone();
                self.state.help_dialog_state.show(&keybindings);
                self.state.input_mode = InputMode::ShowingHelp;
            }
            "model" => {
                let has_session = self.state.tab_manager.active_session().is_some();
                if has_session {
                    let model = self
                        .state
                        .tab_manager
                        .active_session()
                        .and_then(|s| s.model.clone());
                    let agent_type = self
                        .state
                        .tab_manager
                        .active_session()
                        .map(|s| s.agent_type)
                        .unwrap_or(conduit_agent::AgentType::Claude);
                    let mut allowed = self.config().effective_enabled_providers(self.tools());
                    if !allowed.contains(&agent_type) {
                        let tool = Self::required_tool(agent_type);
                        if self.tools().is_available(tool) {
                            allowed.push(agent_type);
                        }
                    }
                    if allowed.is_empty() {
                        allowed.push(conduit_agent::AgentType::Claude);
                    }
                    let defaults = self.model_selector_defaults();
                    self.state.close_overlays();
                    self.state
                        .model_selector_state
                        .set_allowed_providers(Some(allowed));
                    self.state.model_selector_state.show(model, defaults);
                    self.state.model_picker_context = ModelPickerContext::SessionSelection;
                    self.state.input_mode = InputMode::SelectingModel;
                }
            }
            "theme" => {
                self.state.close_overlays();
                let theme_path = self.config().theme_path.clone();
                let project_theme = self
                    .state
                    .tab_manager
                    .active_session()
                    .and_then(|s| s.project_theme.clone());
                let has_project_context = self
                    .state
                    .tab_manager
                    .active_session()
                    .and_then(|s| s.workspace_id)
                    .is_some();
                self.state.theme_picker_state.show_with_project_context(
                    theme_path.as_deref(),
                    project_theme.as_deref(),
                    has_project_context,
                );
                self.state.input_mode = InputMode::SelectingTheme;
            }
            "providers" => {
                self.state.close_overlays();
                self.state.pending_new_project_target = None;
                self.redetect_tools();
                self.state.provider_selector_state =
                    crate::components::ProviderSelectorState::configure_for(
                        self.config(),
                        self.tools(),
                    );
                self.state.provider_selector_state.show();
                self.state.input_mode = InputMode::SelectingProviders;
            }
            "archive" => {
                if let Some(workspace_id) = self
                    .state
                    .tab_manager
                    .active_session()
                    .and_then(|s| s.workspace_id)
                {
                    self.initiate_work_complete(workspace_id);
                }
            }
            "file-mention" => {
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    session.working_dir = Some(std::env::current_dir().unwrap_or_default());
                    session.input_box.insert_char('@');
                }
                self.open_file_mention_menu();
                for c in "docs/".chars() {
                    self.state.file_mention_state.insert_char(c);
                    if let Some(session) = self.state.tab_manager.active_session_mut() {
                        session.input_box.insert_char(c);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn load_demo_data(&mut self) {
        use crate::demo;

        let demo = demo::build_demo();

        self.demo_mode = true;

        // Reset any real state loaded from DB
        self.state.show_first_time_splash = false;
        self.state.sidebar_state.visible = true;
        self.state.sidebar_data = crate::components::SidebarData::new();
        while !self.state.tab_manager.is_empty() {
            self.state.tab_manager.close_tab(0);
        }

        // Populate sidebar
        demo::populate_sidebar(&mut self.state.sidebar_data, &demo.repos);

        // Select the active workspace in the sidebar
        let visible = self.state.sidebar_data.visible_nodes();
        if let Some(pos) = visible
            .iter()
            .position(|n| n.id == demo.active_workspace_id)
        {
            self.state.sidebar_state.tree_state.selected = pos;
        }

        // Add demo session as the active tab
        self.state.tab_manager.add_session(demo.session);
        self.state.tab_manager.switch_to(0);
    }

    /// Restore session state from database
    fn restore_session_state(&mut self) {
        tracing::info!("Restoring session state");
        // Check repository count first
        let repo_count = self
            .repo_dao()
            .and_then(|dao| dao.get_all().ok())
            .map(|repos| repos.len())
            .unwrap_or(0);

        // If no repos, show first-time splash
        if repo_count == 0 {
            self.state.show_first_time_splash = true;
            tracing::info!("No repositories found; skipping session restore");
            return;
        }

        // Has repos, don't show first-time splash
        self.state.show_first_time_splash = false;

        // Try to restore saved tabs
        let Some(session_tab_dao) = self.session_tab_dao_clone() else {
            tracing::warn!("Session tab DAO unavailable; skipping session restore");
            return;
        };
        let Some(app_state_dao) = self.app_state_dao_clone() else {
            tracing::warn!("App state DAO unavailable; skipping session restore");
            return;
        };

        let saved_tabs = match session_tab_dao.get_all() {
            Ok(tabs) => tabs,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to load saved tabs");
                return;
            }
        };

        if saved_tabs.is_empty() {
            // Has repos but no saved tabs - activate sidebar so user can pick a workspace
            tracing::info!("No saved tabs found; skipping session restore");
            self.state.input_mode = InputMode::SidebarNavigation;
            return;
        }

        tracing::info!(tab_count = saved_tabs.len(), "Restoring saved tabs");

        // Restore each tab
        for tab in saved_tabs {
            let required_tool = Self::required_tool(tab.agent_type);
            if !self.tools().is_available(required_tool) {
                self.show_missing_tool(
                    required_tool,
                    format!(
                        "{} is required to restore this session.",
                        required_tool.display_name()
                    ),
                );
                break;
            }

            let mut session = AgentSession::new(tab.agent_type);
            session.id = tab.id;
            session.workspace_id = tab.workspace_id;
            session.model = tab.model;
            session.model_invalid = tab.model_invalid;
            session.pr_number = tab.pr_number.map(|n| n as u32);
            session.fork_seed_id = tab.fork_seed_id;
            // Restore AI-generated session title
            session.title = tab.title.clone();
            // Restore agent mode (defaults to Build if not set)
            let parsed_mode = tab
                .agent_mode
                .as_deref()
                .map(AgentMode::parse)
                .unwrap_or_default();
            session.agent_mode = Self::clamp_agent_mode(tab.agent_type, parsed_mode);

            // Look up workspace to get working_dir, workspace_name, and project_name
            if let Some(workspace_id) = tab.workspace_id {
                if let Some(workspace_dao) = self.workspace_dao() {
                    if let Ok(Some(workspace)) = workspace_dao.get_by_id(workspace_id) {
                        session.working_dir = Some(workspace.path);
                        session.workspace_name = Some(workspace.name.clone());
                        session.branch_name = Some(workspace.branch.clone());

                        // Look up repository for project name and project theme
                        if let Some(repo_dao) = self.repo_dao() {
                            if let Ok(Some(repo)) = repo_dao.get_by_id(workspace.repository_id) {
                                session.repository_id = Some(repo.id);
                                session.project_name = Some(repo.name);
                                session.project_theme = repo.theme_name;
                            }
                        }
                    }
                }
            }

            // Set resume session ID if available
            if let Some(ref session_id_str) = tab.agent_session_id {
                let session_id = SessionId::from_string(session_id_str.clone());
                session.resume_session_id = Some(session_id.clone());
                if tab.agent_type != AgentType::Codex {
                    session.agent_session_id = Some(session_id.clone());
                }

                // Load chat history from agent files
                match tab.agent_type {
                    AgentType::Claude => {
                        if let Ok((msgs, debug_entries, file_path)) =
                            load_claude_history_with_debug(session_id_str)
                        {
                            // Populate debug pane with history load info
                            Self::populate_debug_from_history(
                                &mut session.raw_events_view,
                                &debug_entries,
                                &file_path,
                            );
                            for msg in msgs {
                                session.chat_view.push(msg);
                            }
                        }
                    }
                    AgentType::Codex => {
                        if let Ok((msgs, debug_entries, file_path)) =
                            load_codex_history_with_debug(session_id_str)
                        {
                            // Populate debug pane with history load info
                            Self::populate_debug_from_history(
                                &mut session.raw_events_view,
                                &debug_entries,
                                &file_path,
                            );
                            for msg in msgs {
                                session.chat_view.push(msg);
                            }
                        }
                    }
                    AgentType::Dirac => {
                        session.chat_view.push(
                            MessageDisplay::System {
                                content: "Dirac history import isn't supported yet, so previous messages won't be shown.".to_string(),
                            }
                            .to_chat_message(),
                        );
                    }
                    AgentType::Gemini => {
                        session.chat_view.push(
                            MessageDisplay::System {
                                content: "Gemini CLI history import isn't supported yet, so previous messages won't be shown.".to_string(),
                            }
                            .to_chat_message(),
                        );
                    }
                    AgentType::DeepseekTui => {
                        session.chat_view.push(
                            MessageDisplay::System {
                                content: "DeepSeek TUI history import isn't supported yet, so previous messages won't be shown.".to_string(),
                            }
                            .to_chat_message(),
                        );
                    }
                    AgentType::Opencode => {
                        if let Ok((msgs, debug_entries, file_path)) =
                            load_opencode_history_with_debug(session_id_str)
                        {
                            Self::populate_debug_from_history(
                                &mut session.raw_events_view,
                                &debug_entries,
                                &file_path,
                            );
                            for msg in msgs {
                                session.chat_view.push(msg);
                            }
                        }
                    }
                    AgentType::Copilot => {
                        session.chat_view.push(
                            MessageDisplay::System {
                                content: "GitHub Copilot history import isn't supported yet, so previous messages won't be shown.".to_string(),
                            }
                            .to_chat_message(),
                        );
                    }
                    AgentType::Pi => {
                        if let Ok((msgs, debug_entries, file_path)) =
                            load_pi_history_with_debug(session_id_str)
                        {
                            Self::populate_debug_from_history(
                                &mut session.raw_events_view,
                                &debug_entries,
                                &file_path,
                            );
                            for msg in msgs {
                                session.chat_view.push(msg);
                            }
                        }
                    }
                }
            } else if tab.agent_type == AgentType::Opencode {
                if let Some(working_dir) = session.working_dir.as_ref() {
                    if let Ok((session_id_str, msgs, debug_entries, file_path)) =
                        load_opencode_history_for_dir_with_debug(working_dir)
                    {
                        let session_id = SessionId::from_string(session_id_str.clone());
                        session.resume_session_id = Some(session_id.clone());
                        session.agent_session_id = Some(session_id);

                        Self::populate_debug_from_history(
                            &mut session.raw_events_view,
                            &debug_entries,
                            &file_path,
                        );
                        for msg in msgs {
                            session.chat_view.push(msg);
                        }
                    }
                }
            }

            // Restore pending user message if it exists and isn't already in history
            if let Some(ref pending) = tab.pending_user_message {
                // Check if last user message in chat matches pending
                let already_in_history = session
                    .chat_view
                    .messages()
                    .iter()
                    .rev()
                    .find(|m| m.role == MessageRole::User)
                    .map(|m| m.content.as_str() == pending.as_str())
                    .unwrap_or(false);

                if !already_in_history {
                    let display = MessageDisplay::User {
                        content: pending.clone(),
                    };
                    session.chat_view.push(display.to_chat_message());
                    session.pending_user_message = Some(pending.clone());
                }
            }

            if !tab.queued_messages.is_empty() {
                session.queued_messages = tab.queued_messages.clone();
            }

            session.input_box.set_history(tab.input_history.clone());

            // Derive fork_welcome_shown: if restoring a forked session that has messages,
            // the welcome message was already shown in the previous session
            if session.fork_seed_id.is_some() && !session.chat_view.messages().is_empty() {
                session.fork_welcome_shown = true;
            }

            session.update_status();

            // Register workspace with git tracker if available
            let track_info = session.workspace_id.zip(session.working_dir.clone());
            let sidebar_pr_update = session
                .pr_number
                .and_then(|pr_num| Self::apply_pr_number_to_session(&mut session, pr_num));

            self.state.tab_manager.add_session(session);

            if let Some((workspace_id, status)) = sidebar_pr_update {
                self.state
                    .sidebar_data
                    .update_workspace_pr_status(workspace_id, Some(status));
            }

            // Track workspace after session is added
            if let Some((workspace_id, working_dir)) = track_info {
                if let Some(ref tracker) = self.git_tracker {
                    tracker.track_workspace(workspace_id, working_dir);
                }
            }
        }

        // If all tab restores failed, fall back to sidebar navigation
        if self.state.tab_manager.is_empty() {
            self.state.input_mode = InputMode::SidebarNavigation;
        }

        // Restore active tab
        if let Ok(Some(index_str)) = app_state_dao.get("active_tab_index") {
            if let Ok(index) = index_str.parse::<usize>() {
                let tab_count = self.state.tab_manager.len();
                if tab_count > 0 {
                    let max_index = tab_count.saturating_sub(1);
                    let clamped_index = index.min(max_index);
                    self.state.tab_manager.switch_to(clamped_index);
                }
            }
        }

        // Restore sidebar visibility
        if let Ok(Some(visible_str)) = app_state_dao.get("sidebar_visible") {
            self.state.sidebar_state.visible = visible_str == "true";
        }

        // Restore collapsed repos (repos default to expanded, so we collapse the saved ones)
        if let Ok(Some(collapsed_str)) = app_state_dao.get("tree_collapsed_repos") {
            if !collapsed_str.is_empty() {
                for id_str in collapsed_str.split(',') {
                    if let Ok(id) = uuid::Uuid::parse_str(id_str) {
                        self.state.sidebar_data.collapse_repo(id);
                    }
                }
            }
        }

        // Restore tree selection index (after expanding repos so visible count is correct)
        if let Ok(Some(index_str)) = app_state_dao.get("tree_selected_index") {
            if let Ok(index) = index_str.parse::<usize>() {
                let visible_count = self.state.sidebar_data.visible_nodes().len();
                self.state.sidebar_state.tree_state.selected =
                    index.min(visible_count.saturating_sub(1));
            }
        }

        tracing::info!("Session state restoration complete");
    }

    /// Refresh sidebar data from database
    fn refresh_sidebar_data(&mut self) {
        // Capture current expansion state before rebuild
        let expanded_repos = self.state.sidebar_data.expanded_repo_ids();

        // Preserve runtime PR statuses (not stored in DB)
        let pr_statuses: Vec<(uuid::Uuid, conduit_git::PrStatus)> = self
            .state
            .sidebar_data
            .nodes
            .iter()
            .flat_map(|repo| &repo.children)
            .filter_map(|ws| ws.pr_status.clone().map(|s| (ws.id, s)))
            .collect();

        // Collect all repo/workspace data first to avoid borrow conflicts
        type RepoWorkspaceData = Vec<(Uuid, String, Option<String>, Vec<(Uuid, String, String)>)>;

        let repo_workspace_data: RepoWorkspaceData = {
            let Some(repo_dao) = self.repo_dao() else {
                self.state.sidebar_data = SidebarData::new();
                return;
            };
            let Some(workspace_dao) = self.workspace_dao() else {
                self.state.sidebar_data = SidebarData::new();
                return;
            };

            let mut data = Vec::new();
            if let Ok(repos) = repo_dao.get_all() {
                for repo in repos {
                    if let Ok(workspaces) = workspace_dao.get_by_repository(repo.id) {
                        let workspace_info: Vec<_> = workspaces
                            .into_iter()
                            .map(|ws| (ws.id, ws.name, ws.branch))
                            .collect();
                        data.push((repo.id, repo.name, repo.theme_name, workspace_info));
                    }
                }
            }
            data
        };

        // Now update state (no more borrows on self.core)
        self.state.sidebar_data = SidebarData::new();
        for (repo_id, repo_name, theme_name, workspace_info) in repo_workspace_data {
            self.state
                .sidebar_data
                .add_repository(repo_id, &repo_name, workspace_info, theme_name);
        }

        // Restore expansion state
        for repo_id in expanded_repos {
            self.state.sidebar_data.expand_repo(repo_id);
        }

        // Restore runtime PR statuses
        for (workspace_id, status) in pr_statuses {
            self.state
                .sidebar_data
                .update_workspace_pr_status(workspace_id, Some(status));
        }

        // Populate tab numbers for workspaces that are open in a tab
        {
            use crate::components::NodeType;
            let workspace_ids: Vec<Uuid> = self
                .state
                .sidebar_data
                .nodes
                .iter()
                .flat_map(|repo| repo.children.iter())
                .filter(|child| child.node_type == NodeType::Workspace)
                .map(|child| child.id)
                .collect();
            for workspace_id in workspace_ids {
                let tab_number = self.find_tab_for_workspace(workspace_id).map(|idx| idx + 1);
                self.state
                    .sidebar_data
                    .set_workspace_tab_number(workspace_id, tab_number);
            }
        }

        self.sync_sidebar_busy_state();
    }

    fn sync_sidebar_busy_state(&mut self) {
        let busy_repos: Vec<Uuid> = self.state.busy_repos.iter().copied().collect();
        let busy_repo_actions: Vec<Uuid> = self.state.busy_repo_actions.iter().copied().collect();
        let busy_workspaces: Vec<Uuid> = self.state.busy_workspaces.iter().copied().collect();

        for repo_id in busy_repos {
            self.state.sidebar_data.set_repo_busy(repo_id, true);
        }
        for repo_id in busy_repo_actions {
            self.state.sidebar_data.set_action_busy(repo_id, true);
        }
        for workspace_id in busy_workspaces {
            self.state
                .sidebar_data
                .set_workspace_busy(workspace_id, true);
        }
    }

    fn busy_footer_message(&self) -> Option<String> {
        if !self.state.busy_repos.is_empty() {
            return Some("Removing project...".to_string());
        }
        if !self.state.busy_repo_actions.is_empty() {
            return Some("Creating workspace...".to_string());
        }
        if !self.state.busy_workspaces.is_empty() {
            return Some("Working on workspace...".to_string());
        }
        None
    }

    fn sync_busy_footer_message(&mut self) {
        let desired = self.busy_footer_message();

        if desired.is_none() {
            if self.state.busy_footer_message_active {
                if self.state.footer_message.as_deref() == self.state.busy_footer_message.as_deref()
                {
                    self.state.set_footer_message(None);
                }
                self.state.busy_footer_message_active = false;
                self.state.busy_footer_message = None;
            }
            return;
        }

        self.state.busy_footer_message = desired.clone();

        if self.state.footer_message_expires_at.is_some() {
            self.state.busy_footer_message_active = true;
            return;
        }

        if self.state.footer_message.is_some() && !self.state.busy_footer_message_active {
            self.state.busy_footer_message_active = true;
            return;
        }

        self.state.set_footer_message(desired);
        self.state.busy_footer_message_active = true;
    }

    fn mark_workspace_busy(&mut self, workspace_id: Uuid) {
        if self.state.busy_workspaces.insert(workspace_id) {
            self.state
                .sidebar_data
                .set_workspace_busy(workspace_id, true);
            self.sync_busy_footer_message();
        }
    }

    fn clear_workspace_busy(&mut self, workspace_id: Uuid) {
        if self.state.busy_workspaces.remove(&workspace_id) {
            self.state
                .sidebar_data
                .set_workspace_busy(workspace_id, false);
            self.sync_busy_footer_message();
            if let Some(branch) = self.state.pending_branch_updates.remove(&workspace_id) {
                self.apply_branch_update(workspace_id, branch);
            }
        }
    }

    fn mark_repo_busy(&mut self, repo_id: Uuid) {
        if self.state.busy_repos.insert(repo_id) {
            self.state.sidebar_data.set_repo_busy(repo_id, true);
            self.sync_busy_footer_message();
        }
    }

    fn clear_repo_busy(&mut self, repo_id: Uuid) {
        if self.state.busy_repos.remove(&repo_id) {
            self.state.sidebar_data.set_repo_busy(repo_id, false);
            self.sync_busy_footer_message();
        }
    }

    fn mark_repo_action_busy(&mut self, repo_id: Uuid) {
        if self.state.busy_repo_actions.insert(repo_id) {
            self.state.sidebar_data.set_action_busy(repo_id, true);
            self.sync_busy_footer_message();
        }
    }

    fn clear_repo_action_busy(&mut self, repo_id: Uuid) {
        if self.state.busy_repo_actions.remove(&repo_id) {
            self.state.sidebar_data.set_action_busy(repo_id, false);
            self.sync_busy_footer_message();
        }
    }

    /// Save session state to database for restoration on next startup.
    fn snapshot_session_state(&self) -> SessionStateSnapshot {
        let tabs = self
            .state
            .tab_manager
            .sessions()
            .iter()
            .enumerate()
            .map(|(index, session)| {
                let mut tab = SessionTab::new(
                    index as i32,
                    session.agent_type,
                    session.workspace_id,
                    session
                        .agent_session_id
                        .as_ref()
                        .or(session.resume_session_id.as_ref())
                        .map(|s| s.as_str().to_string()),
                    session.model.clone(),
                    session.pr_number.map(|n| n as i32),
                );
                tab.id = session.id;
                tab.model_invalid = session.model_invalid;
                // Preserve agent mode for session restoration
                tab.agent_mode = Some(session.agent_mode.as_str().to_string());
                // Preserve pending user message for interrupted sessions
                tab.pending_user_message = session.pending_user_message.clone();
                // Preserve queued messages for interrupted sessions
                tab.queued_messages = session.queued_messages.clone();
                // Preserve input history for arrow-up restoration
                tab.input_history = session.input_box.history_snapshot();
                tab.fork_seed_id = session.fork_seed_id;
                // Preserve AI-generated session title
                tab.title = session.title.clone();
                tab.title_generated = false;
                tab
            })
            .collect();

        SessionStateSnapshot {
            tabs,
            active_tab_index: self.state.tab_manager.active_index(),
            sidebar_visible: self.state.sidebar_state.visible,
            tree_selected_index: self.state.sidebar_state.tree_state.selected,
            collapsed_repo_ids: self.state.sidebar_data.collapsed_repo_ids(),
        }
    }

    fn persist_session_state(
        snapshot: SessionStateSnapshot,
        session_tab_dao: Option<SessionTabStore>,
        app_state_dao: Option<AppStateStore>,
    ) -> SessionPersistenceReport {
        let mut report = SessionPersistenceReport::default();

        let Some(session_tab_dao) = session_tab_dao else {
            tracing::warn!("Session tab DAO unavailable; skipping session persistence");
            report.push("Session tab DAO unavailable; skipping session persistence".to_string());
            return report;
        };
        let Some(app_state_dao) = app_state_dao else {
            tracing::warn!("App state DAO unavailable; skipping session persistence");
            report.push("App state DAO unavailable; skipping session persistence".to_string());
            return report;
        };

        tracing::info!(
            tab_count = snapshot.tabs.len(),
            active_tab_index = snapshot.active_tab_index,
            "Persisting session state"
        );

        for tab in &snapshot.tabs {
            if let Err(e) = session_tab_dao.upsert(tab) {
                tracing::warn!(error = %e, tab_index = tab.tab_index, "Failed to save session tab");
                report.push(format!(
                    "Failed to save session tab at index {}: {}",
                    tab.tab_index, e
                ));
            }
        }

        if let Err(e) =
            app_state_dao.set("active_tab_index", &snapshot.active_tab_index.to_string())
        {
            tracing::warn!(error = %e, "Failed to save active tab index");
            report.push(format!("Failed to save active tab index: {}", e));
        }

        if let Err(e) = app_state_dao.set(
            "sidebar_visible",
            if snapshot.sidebar_visible {
                "true"
            } else {
                "false"
            },
        ) {
            tracing::warn!(error = %e, "Failed to save sidebar visibility");
            report.push(format!("Failed to save sidebar visibility: {}", e));
        }

        if let Err(e) = app_state_dao.set(
            "tree_selected_index",
            &snapshot.tree_selected_index.to_string(),
        ) {
            tracing::warn!(error = %e, "Failed to save tree selection");
            report.push(format!("Failed to save tree selection: {}", e));
        }

        let collapsed_ids: Vec<String> = snapshot
            .collapsed_repo_ids
            .iter()
            .map(|id| id.to_string())
            .collect();
        if let Err(e) = app_state_dao.set("tree_collapsed_repos", &collapsed_ids.join(",")) {
            tracing::warn!(error = %e, "Failed to save collapsed repos");
            report.push(format!("Failed to save collapsed repos: {}", e));
        }

        tracing::info!("Session state persistence complete");
        report
    }

    fn apply_session_persistence_report(&mut self, report: SessionPersistenceReport) {
        if report.has_errors() {
            tracing::warn!(
                error_count = report.error_count(),
                first_error = %report.first_error_or_unknown(),
                "Session state persistence completed with warnings"
            );
            self.state.set_timed_footer_message(
                "Warning: some session state could not be saved. Check logs.".to_string(),
                Duration::from_secs(5),
            );
        }
    }

    /// Run the application main loop
    pub async fn run(&mut self) -> anyhow::Result<()> {
        self.spawn_shutdown_listeners();

        // Setup terminal
        let keyboard_enhancement_enabled = false;
        let mut stdout = io::stdout();

        if self.demo_mode {
            // Demo mode: render to primary buffer (no raw mode, no alternate screen)
            // so VHS can capture the frame. Render once then sleep.
            let backend = CrosstermBackend::new(stdout);
            let mut terminal = Terminal::new(backend)?;
            terminal.draw(|frame| self.draw(frame))?;
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            return Ok(());
        }

        enable_raw_mode()?;
        // Create terminal guard AFTER enabling features - Drop will clean up on any exit path
        let mut guard = TerminalGuard::new(keyboard_enhancement_enabled);

        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste
        )?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Clear screen
        terminal.clear()?;

        self.run_terminal_event_loop(&mut terminal, &mut guard)
            .await
    }

    /// Run with an already-prepared terminal (raw mode + alternate screen already active).
    /// Used when the caller has entered the terminal before constructing the App, e.g. to
    /// show a startup splash while initialization runs.
    pub async fn run_with_prepared_terminal(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        guard: &mut TerminalGuard,
    ) -> anyhow::Result<()> {
        self.spawn_shutdown_listeners();
        terminal.clear()?;
        self.run_terminal_event_loop(terminal, guard).await
    }

    async fn run_terminal_event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        guard: &mut TerminalGuard,
    ) -> anyhow::Result<()> {
        let result = self.event_loop(terminal, guard).await;

        // Best-effort persistence on any exit path.
        self.persist_session_state_on_exit();

        // Kill any agent processes that are still running. This handles cases where
        // processes accumulate during a session (e.g. interrupted turns whose SIGTERM
        // didn't fire in time). We send SIGKILL directly here — no grace period needed
        // since we're exiting anyway and the stdio pipes are about to close.
        self.kill_all_running_agents();

        // Explicit cleanup with error handling (prevents double-cleanup in Drop)
        terminal.show_cursor()?;
        guard.cleanup()?;

        result
    }

    fn spawn_shutdown_listeners(&self) {
        let tx = self.event_tx.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                send_app_event(&tx, AppEvent::Quit, "shutdown:ctrl_c");
            }
        });

        #[cfg(unix)]
        {
            let tx = self.event_tx.clone();
            tokio::spawn(async move {
                if let Ok(mut sigterm) =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                {
                    sigterm.recv().await;
                    send_app_event(&tx, AppEvent::Quit, "shutdown:sigterm");
                }
            });

            let tx = self.event_tx.clone();
            tokio::spawn(async move {
                if let Ok(mut sighup) =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
                {
                    sighup.recv().await;
                    send_app_event(&tx, AppEvent::Quit, "shutdown:sighup");
                }
            });
        }
    }

    fn persist_session_state_on_exit(&self) {
        let snapshot = self.snapshot_session_state();
        let report = Self::persist_session_state(
            snapshot,
            self.session_tab_dao_clone(),
            self.app_state_dao_clone(),
        );
        if report.has_errors() {
            tracing::warn!(
                error_count = report.error_count(),
                first_error = %report.first_error_or_unknown(),
                "Session state persistence on exit completed with warnings"
            );
        }
    }

    async fn event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        guard: &mut TerminalGuard,
    ) -> anyhow::Result<()> {
        const FRAME_INTERVAL_ACTIVE: Duration = Duration::from_millis(16); // ~60 FPS for animations
        const FRAME_INTERVAL_IDLE: Duration = Duration::from_millis(250); // ~4 FPS when idle

        // Create async event stream for terminal input
        let mut event_stream = EventStream::new();

        // Scroll batching state (moved outside loop to accumulate across frames)
        let mut pending_scroll_up = 0usize;
        let mut pending_scroll_down = 0usize;
        let mut last_tick = Instant::now();

        loop {
            let frame_start = Instant::now();

            // Only draw if needed to save CPU when idle
            if self.state.need_redraw {
                let draw_start = Instant::now();
                terminal.draw(|f| self.draw(f))?;
                let draw_end = Instant::now();
                self.state.metrics.draw_time = draw_end.duration_since(draw_start);
                self.state.metrics.on_draw_end(draw_end);
                self.state.need_redraw = false;
            }

            // Use shorter interval when animations are active, longer when idle
            let target_frame =
                if self.state.needs_animation() || pending_scroll_up > 0 || pending_scroll_down > 0
                {
                    FRAME_INTERVAL_ACTIVE
                } else {
                    FRAME_INTERVAL_IDLE
                };

            // Handle periodic updates (fixed time step)
            // This ensures we always process ticks/animations even if input events flood the queue
            if last_tick.elapsed() >= target_frame {
                let event_start = Instant::now();

                // Flush any pending scroll events accumulated this frame
                if pending_scroll_up > 0 || pending_scroll_down > 0 {
                    self.state.need_redraw = true;
                }
                self.flush_scroll_deltas(&mut pending_scroll_up, &mut pending_scroll_down);

                // Trigger redraw when animations are active
                if self.state.needs_animation() {
                    self.state.need_redraw = true;
                }

                // Handle tick and trigger redraw if UI state was mutated
                if self.handle_tick() {
                    self.state.need_redraw = true;
                }

                self.state.metrics.event_time = event_start.elapsed();
                last_tick = Instant::now();
            }

            let wait = target_frame.saturating_sub(last_tick.elapsed());

            tokio::select! {
                // Prioritize terminal input for immediate response
                biased;

                // Terminal input events via async EventStream - responds immediately
                Some(result) = event_stream.next() => {
                    let event_start = Instant::now();
                    match result {
                        Ok(Event::Key(key)) => {
                            self.state.need_redraw = true;
                            self.flush_scroll_deltas(&mut pending_scroll_up, &mut pending_scroll_down);
                            self.dispatch_event(AppEvent::Input(Event::Key(key)), terminal, guard)
                                .await?;
                        }
                        Ok(Event::Mouse(mouse)) => {
                            match mouse.kind {
                                MouseEventKind::ScrollUp => {
                                    if self.handle_tab_bar_wheel(
                                        mouse.column,
                                        mouse.row,
                                        true,
                                    ) {
                                        // Handled by tab bar, skip
                                    } else {
                                        if self.should_route_scroll_to_chat() {
                                            self.record_scroll(1);
                                        }
                                        pending_scroll_up = pending_scroll_up.saturating_add(1);
                                        // Don't set need_redraw here - batch scroll events
                                        // and redraw on clean tick for smoother scrolling
                                    }
                                }
                                MouseEventKind::ScrollDown => {
                                    if self.handle_tab_bar_wheel(
                                        mouse.column,
                                        mouse.row,
                                        false,
                                    ) {
                                        // Handled by tab bar, skip
                                    } else {
                                        if self.should_route_scroll_to_chat() {
                                            self.record_scroll(1);
                                        }
                                        pending_scroll_down = pending_scroll_down.saturating_add(1);
                                        // Don't set need_redraw here - batch scroll events
                                        // and redraw on clean tick for smoother scrolling
                                    }
                                }
                                _ => {
                                    self.state.need_redraw = true;
                                    self.flush_scroll_deltas(
                                        &mut pending_scroll_up,
                                        &mut pending_scroll_down,
                                    );
                                    self.dispatch_event(
                                        AppEvent::Input(Event::Mouse(mouse)),
                                        terminal,
                                        guard,
                                    )
                                    .await?;
                                }
                            }
                        }
                        Ok(event) => {
                            // Other input events (resize, focus, paste, etc.)
                            self.state.need_redraw = true;
                            self.flush_scroll_deltas(
                                &mut pending_scroll_up,
                                &mut pending_scroll_down,
                            );
                            self.dispatch_event(AppEvent::Input(event), terminal, guard)
                                .await?;
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "Error reading terminal event");
                        }
                    }
                    self.state.metrics.event_time = event_start.elapsed();
                }

                // Sleep until next tick time
                _ = tokio::time::sleep(wait) => {}

                // App events from channel
                Some(event) = self.event_rx.recv() => {
                    // All app events trigger a redraw
                    self.state.need_redraw = true;
                    let event_start = Instant::now();
                    self.dispatch_event(event, terminal, guard).await?;
                    self.state.metrics.event_time = event_start.elapsed();
                }
            }

            // Record total frame time (includes sleep for accurate FPS)
            let frame_end = Instant::now();
            self.state
                .metrics
                .record_frame(frame_end.duration_since(frame_start));
            self.state.metrics.on_frame_end(frame_end);

            if self.state.should_quit {
                break;
            }
        }

        Ok(())
    }

    async fn dispatch_event(
        &mut self,
        event: AppEvent,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        guard: &mut TerminalGuard,
    ) -> anyhow::Result<()> {
        let effects = match event {
            AppEvent::Input(input) => {
                // All input events trigger a redraw
                self.state.need_redraw = true;
                self.handle_input_event(input, terminal, guard).await?
            }
            AppEvent::Tick => {
                if self.handle_tick() {
                    self.state.need_redraw = true;
                }
                Vec::new()
            }
            _ => self.handle_app_event(event).await?,
        };

        self.run_effects(effects).await
    }

    /// Handle periodic tick updates. Returns true if visible UI state was mutated
    /// and a redraw is needed.
    fn handle_tick(&mut self) -> bool {
        let mut state_changed = false;
        self.state.tick_count += 1;

        // Tick footer Knight Rider spinner every 2 frames (~40ms at 50 FPS, matches opencode)
        if self.state.tick_count.is_multiple_of(2) {
            self.state.tick_footer_spinner();
        }

        // Tick logo shine animation every 3 frames (~50ms for smooth diagonal sweep)
        // Only tick when splash screen is visible (no sessions open)
        let splash_visible = self.state.tab_manager.is_empty();
        if self.state.tick_count.is_multiple_of(3) {
            if splash_visible {
                // Reset animation when transitioning back to splash screen
                if !self.state.was_splash_visible {
                    self.state.logo_shine.reset();
                }
                self.state.logo_shine.tick();
            }
            self.state.was_splash_visible = splash_visible;
        }

        // Clear stale double-press state and messages
        let now = Instant::now();
        let timeout = Duration::from_millis(DOUBLE_PRESS_TIMEOUT_MS);

        if let Some(last) = self.state.last_ctrl_c_press {
            if now.duration_since(last) > timeout {
                self.state.last_ctrl_c_press = None;
                // Clear associated message
                if matches!(
                    self.state.footer_message.as_deref(),
                    Some("Press Ctrl+C again to interrupt and quit")
                        | Some("Press Ctrl+C again to quit")
                ) {
                    self.state.footer_message = None;
                    state_changed = true;
                }
            }
        }

        if let Some(last) = self.state.last_esc_press {
            if now.duration_since(last) > timeout {
                self.state.last_esc_press = None;
                if matches!(
                    self.state.footer_message.as_deref(),
                    Some("Press Esc again to interrupt") | Some("Press Esc again to clear")
                ) {
                    self.state.footer_message = None;
                    state_changed = true;
                }
            }
        }

        // Clear expired timed footer messages
        let had_timed_message = self.state.footer_message_expires_at.is_some();
        self.state.clear_expired_footer_message();
        self.sync_busy_footer_message();
        if had_timed_message && self.state.footer_message_expires_at.is_none() {
            state_changed = true;
        }

        self.state.theme_picker_state.tick();
        let can_show_picker_error = self.state.theme_picker_state.is_visible()
            || (self.state.footer_message.is_none()
                && self.state.footer_message_expires_at.is_none());
        if can_show_picker_error {
            if let Some(error) = self.state.theme_picker_state.take_error() {
                self.state
                    .set_timed_footer_message(error, Duration::from_secs(5));
                state_changed = true;
            }
        }

        // Tick other animations every 6 frames (~100ms)
        if !self.state.tick_count.is_multiple_of(6) {
            return state_changed;
        }

        // Advance spinner frame for PR processing indicator
        self.state.spinner_frame = self.state.spinner_frame.wrapping_add(1);

        // Tick confirmation dialog spinner (for loading state)
        self.state.confirmation_dialog_state.tick();

        // Tick workspace creation progress dialog spinner
        self.state.workspace_progress_dialog_state.tick();

        // Tick remote-sync dialog spinner
        self.state.remote_sync_dialog_state.tick();

        // Tick session import spinner (for loading state)
        self.state.session_import_state.tick();

        // Tick project picker spinner (for loading state)
        self.state.project_picker_state.tick();

        // Tick issue picker spinner (for loading state)
        self.state.issue_picker_state.tick();
        self.state.spec_picker_state.tick();
        self.state.specify_picker_state.tick();

        if let Some(session) = self.state.tab_manager.active_session_mut() {
            session.tick();
        }

        // Poll the database every 5 seconds to detect projects added externally (e.g. via web UI)
        if now.duration_since(self.state.last_sidebar_db_check) >= Duration::from_secs(5) {
            self.state.last_sidebar_db_check = now;
            let current_repo_count = self.state.sidebar_data.nodes.len();
            let db_repo_count = self
                .repo_dao()
                .and_then(|dao| dao.get_all().ok())
                .map(|repos| repos.len())
                .unwrap_or(current_repo_count);
            if db_repo_count != current_repo_count {
                self.refresh_sidebar_data();
                state_changed = true;
            }
        }

        state_changed
    }

    /// Interrupt the current agent processing
    fn interrupt_agent(&mut self) {
        let mut pid = None;
        let mut pid_start_time = None;
        let mut was_processing = false;
        let mut session_id = None;

        if let Some(session) = self.state.tab_manager.active_session_mut() {
            session_id = Some(session.id);
            pid = session.agent_pid.take();
            pid_start_time = session.agent_pid_start_time.take();
            session.agent_input_tx = None;
            // Clear any active inline prompt and pending permissions since the agent is gone
            session.inline_prompt = None;
            session.pending_tool_permissions.clear();
            session.pending_tool_permission_responses.clear();
            if session.is_processing {
                was_processing = true;
                session.stop_processing();
                session.chat_view.finalize_streaming();
            }
        }

        if let Some(pid) = pid {
            self.spawn_agent_termination(pid, pid_start_time, "interrupt_agent", session_id, true);
        }

        if let Some(sid) = session_id {
            if let Some(store) = self.session_tab_dao() {
                if let Err(e) = store.clear_agent_pid(sid) {
                    tracing::warn!(error = %e, session_id = %sid, "Failed to clear agent PID");
                }
            }
        }

        if was_processing {
            if let Some(session_id) = session_id {
                if let Some(session) = self.state.tab_manager.session_by_id_mut(session_id) {
                    Self::flush_pending_agent_output(session);
                    let display = MessageDisplay::System {
                        content: "Interrupted".to_string(),
                    };
                    session.chat_view.push(display.to_chat_message());
                }
            }
            self.state.stop_footer_spinner();
        }
    }

    fn format_session_status(session: &crate::session::AgentSession) -> String {
        let agent = session.agent_type.to_string();
        let model = session.model.as_deref().unwrap_or("—");
        let session_id = session
            .agent_session_id
            .as_ref()
            .map(|s| s.as_str().to_string())
            .unwrap_or_else(|| "—".to_string());
        let ctx_pct = (session.context_state.usage_percent() * 100.0) as u32;
        let current = session.context_state.current_tokens;
        let max = session.context_state.max_tokens;
        let turns = session.turn_count;
        let dir = session
            .working_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "—".to_string());
        format!(
            "Agent:    {agent}\nModel:    {model}\nSession:  {session_id}\nContext:  {ctx_pct}% ({current} / {max} tokens)\nTurns:    {turns}\nDir:      {dir}"
        )
    }

    fn truncate_claude_session(
        session_id: &SessionId,
        working_dir: &std::path::Path,
    ) -> std::io::Result<()> {
        let encoded = working_dir.to_str().unwrap_or("").replace('/', "-");
        let Some(home) = dirs::home_dir() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "home directory not found",
            ));
        };
        let file_path = home
            .join(".claude")
            .join("projects")
            .join(&encoded)
            .join(format!("{}.jsonl", session_id.as_str()));

        if !file_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&file_path)?;
        let lines: Vec<&str> = content.lines().collect();

        // Find the last non-meta user message — that's the start of the turn to remove.
        let last_user_idx = lines.iter().rposition(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .ok()
                .filter(|v| {
                    v.get("type").and_then(|t| t.as_str()) == Some("user")
                        && v.get("isMeta").and_then(|m| m.as_bool()) != Some(true)
                })
                .is_some()
        });

        let Some(idx) = last_user_idx else {
            return Ok(());
        };

        let truncated = lines[..idx].join("\n");
        let truncated = if truncated.is_empty() {
            truncated
        } else {
            truncated + "\n"
        };
        std::fs::write(&file_path, truncated)
    }

    fn spawn_agent_termination(
        &self,
        pid: u32,
        pid_start_time: Option<u64>,
        context: &'static str,
        session_id: Option<Uuid>,
        report_result: bool,
    ) {
        let event_tx = self.event_tx.clone();
        let context = context.to_string();
        tokio::task::spawn_blocking(move || {
            let success = App::terminate_agent_pid(pid, pid_start_time, &context);
            if report_result {
                send_app_event(
                    &event_tx,
                    AppEvent::AgentTerminationResult {
                        session_id,
                        pid,
                        context,
                        success,
                    },
                    "agent_termination_result",
                );
            } else if !success {
                tracing::warn!(
                    pid,
                    context = %context,
                    "Agent termination failed"
                );
            }
        });
    }

    fn terminate_agent_pid(pid: u32, pid_start_time: Option<u64>, context: &str) -> bool {
        conduit_util::process::terminate_process_tree(
            pid,
            pid_start_time,
            context,
            AGENT_TERMINATION_GRACE,
            AGENT_TERMINATION_POLL_INTERVAL,
        )
    }

    /// Send SIGKILL to every agent process still tracked in any open session.
    /// Called on exit so stray processes don't outlive the TUI.
    fn kill_all_running_agents(&self) {
        #[cfg(unix)]
        {
            for session in self.state.tab_manager.sessions() {
                if let Some(pid) = session.agent_pid {
                    if let Err(err) = conduit_util::process::signal_process_tree(pid, libc::SIGKILL)
                    {
                        tracing::debug!(
                            pid,
                            error = %err,
                            "Failed to SIGKILL agent on exit (may have already exited)"
                        );
                    }
                }
            }
        }
    }

    #[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
    fn pid_start_time(_pid: u32) -> Option<u64> {
        None
    }

    #[cfg(not(unix))]
    fn pid_start_time(_pid: u32) -> Option<u64> {
        None
    }

    fn stop_agent_for_tab(&mut self, tab_index: usize) {
        let mut pid = None;
        let mut pid_start_time = None;
        let mut session_id = None;
        {
            if let Some(session) = self.state.tab_manager.session_mut(tab_index) {
                session_id = Some(session.id);
                Self::flush_pending_agent_output(session);
                if session.is_processing {
                    session.stop_processing();
                }
                pid = session.agent_pid.take();
                pid_start_time = session.agent_pid_start_time.take();
            }
        }

        if let Some(pid) = pid {
            self.spawn_agent_termination(pid, pid_start_time, "stop_agent_for_tab", None, false);
        }

        if let Some(sid) = session_id {
            if let Some(store) = self.session_tab_dao() {
                if let Err(e) = store.clear_agent_pid(sid) {
                    tracing::warn!(error = %e, session_id = %sid, "Failed to clear agent PID");
                }
            }
        }
    }

    /// Handle Ctrl+C press with double-press detection
    fn handle_ctrl_c_press(&mut self) -> Vec<Effect> {
        let mut effects = Vec::new();
        let now = Instant::now();
        let is_double = self
            .state
            .last_ctrl_c_press
            .map(|t| now.duration_since(t) < Duration::from_millis(DOUBLE_PRESS_TIMEOUT_MS))
            .unwrap_or(false);

        let is_processing = self
            .state
            .tab_manager
            .active_session()
            .map(|s| s.is_processing)
            .unwrap_or(false);

        tracing::debug!(
            "handle_ctrl_c_press: is_double={}, is_processing={}",
            is_double,
            is_processing
        );

        if is_processing {
            if is_double {
                // Second press while processing: interrupt + quit
                tracing::debug!("Ctrl+C: second press while processing, interrupting and quitting");
                self.interrupt_agent();
                self.state.should_quit = true;
                effects.push(Effect::SaveSessionState);
            } else {
                // First press: show warning
                tracing::debug!("Ctrl+C: first press while processing, showing warning");
                self.state.footer_message = Some("Press Ctrl+C again to interrupt and quit".into());
                self.state.last_ctrl_c_press = Some(now);
            }
        } else if is_double {
            // Second press while idle: quit
            tracing::debug!("Ctrl+C: second press while idle, quitting");
            self.state.should_quit = true;
            effects.push(Effect::SaveSessionState);
        } else {
            // First press while idle: save to history + clear input + show warning
            tracing::debug!("Ctrl+C: first press while idle, saving to history, clearing input and showing warning");
            if let Some(session) = self.state.tab_manager.active_session_mut() {
                // Save current input to history before clearing (if non-empty)
                let current_input = session.input_box.input().to_string();
                if !current_input.trim().is_empty() {
                    session.input_box.add_to_history(&current_input);
                }
                session.input_box.clear();
            }
            self.state.footer_message = Some("Press Ctrl+C again to quit".into());
            self.state.last_ctrl_c_press = Some(now);
        }
        tracing::debug!("footer_message after: {:?}", self.state.footer_message);
        effects
    }

    /// Handle Esc press with double-press detection (only when no dialog is active)
    fn handle_esc_press(&mut self) -> bool {
        let now = Instant::now();
        let is_double = self
            .state
            .last_esc_press
            .map(|t| now.duration_since(t) < Duration::from_millis(DOUBLE_PRESS_TIMEOUT_MS))
            .unwrap_or(false);

        let is_processing = self
            .state
            .tab_manager
            .active_session()
            .map(|s| s.is_processing)
            .unwrap_or(false);

        if is_processing {
            if is_double {
                // Second press while processing: interrupt only
                self.interrupt_agent();
                self.state.footer_message = None;
                self.state.last_esc_press = None;
            } else {
                // First press: show warning
                self.state.footer_message = Some("Press Esc again to interrupt".into());
                self.state.last_esc_press = Some(now);
            }
        } else if is_double {
            // Second press while idle: clear input
            if let Some(session) = self.state.tab_manager.active_session_mut() {
                session.input_box.clear();
            }
            self.state.footer_message = None;
            self.state.last_esc_press = None;
        } else {
            // First press while idle: show warning
            self.state.footer_message = Some("Press Esc again to clear".into());
            self.state.last_esc_press = Some(now);
        }
        true
    }

    /// Check if any overlay is currently active
    fn has_active_dialog(&self) -> bool {
        self.state.has_active_overlay()
    }

    /// Execute a keybinding action
    async fn execute_action(
        &mut self,
        action: Action,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        guard: &mut TerminalGuard,
    ) -> anyhow::Result<Vec<Effect>> {
        let mut effects = Vec::new();
        match action {
            // ========== Global Actions ==========
            Action::ToggleSidebar
            | Action::HideSidebar
            | Action::EnterSidebarMode
            | Action::ExitSidebarMode
            | Action::ExpandOrSelect
            | Action::Collapse
            | Action::ProjectMoveUp
            | Action::ProjectMoveDown
            | Action::ToggleOrchestrationDefault => {
                self.handle_sidebar_action(action, &mut effects);
            }
            Action::RefreshSidebar => {
                self.refresh_sidebar_data();
                self.state.set_timed_footer_message(
                    "Sidebar refreshed".to_string(),
                    Duration::from_secs(2),
                );
            }
            Action::Quit
            | Action::NewProject
            | Action::NewWorkspaceUnderCursor
            | Action::ForkSession
            | Action::HandoffSession
            | Action::InterruptAgent
            | Action::ToggleViewMode
            | Action::ShowModelSelector
            | Action::ShowReasoningSelector
            | Action::ShowOrchestrationSelector
            | Action::ShowThemePicker
            | Action::ShowProvidersSelector
            | Action::OpenSessionImport
            | Action::ImportSession
            | Action::CycleImportFilter
            | Action::ToggleMetrics
            | Action::ToggleAgentMode
            | Action::DumpDebugState
            | Action::CopyWorkspacePath
            | Action::CopySelection
            | Action::CopyCodeBlock
            | Action::CopyCodeBlockPrev
            | Action::CopyFileContents => {
                self.handle_global_action(action, &mut effects);
            }
            Action::OpenPr => {
                if let Some(effect) = self.handle_pr_action() {
                    effects.push(effect);
                }
            }
            Action::Suspend => {
                if let Err(err) = self.suspend_app(terminal, guard) {
                    tracing::warn!(error = %err, "Suspend failed: {err}");
                    self.state.set_timed_footer_message(
                        format!("Suspend failed: {err}"),
                        Duration::from_secs(3),
                    );
                }
            }

            // ========== Tab Management ==========
            Action::CloseTab
            | Action::NextTab
            | Action::PrevTab
            | Action::SwitchToTab(_)
            | Action::MoveTabLeft
            | Action::MoveTabRight => {
                self.handle_tab_action(action, &mut effects);
            }

            // ========== File Viewer ==========
            Action::OpenFile(path) => {
                self.handle_open_file(path, &mut effects);
            }

            // ========== Chat Scrolling ==========
            Action::ScrollUp(_)
            | Action::ScrollDown(_)
            | Action::ScrollPageUp
            | Action::ScrollPageDown
            | Action::ScrollToTop
            | Action::ScrollToBottom
            | Action::ScrollPrevUserMessage
            | Action::ScrollNextUserMessage => {
                self.handle_scroll_action(action);
            }

            // ========== Input Box Editing ==========
            Action::InsertNewline
            | Action::Backspace
            | Action::Delete
            | Action::DeleteWordBack
            | Action::DeleteWordForward
            | Action::DeleteToStart
            | Action::DeleteToEnd
            | Action::MoveCursorLeft
            | Action::MoveCursorRight
            | Action::MoveCursorStart
            | Action::MoveCursorEnd
            | Action::MoveWordLeft
            | Action::MoveWordRight
            | Action::MoveCursorUp
            | Action::MoveCursorDown
            | Action::HistoryPrev
            | Action::HistoryNext => {
                self.handle_input_edit_action(action);
            }
            Action::Submit | Action::SubmitSteer => {
                self.handle_submit_related_action(action, &mut effects)?;
            }
            Action::OpenQueueEditor
            | Action::CloseQueueEditor
            | Action::QueueMoveUp
            | Action::QueueMoveDown
            | Action::QueueEdit
            | Action::QueueDelete => {
                self.handle_queue_action(action);
            }
            Action::EditPromptExternal => {
                if let Err(err) = self.edit_prompt_external(terminal, guard) {
                    tracing::warn!(error = %err, "External editor failed");
                    self.state.set_timed_footer_message(
                        format!("External editor failed: {err}"),
                        Duration::from_secs(3),
                    );
                }
            }

            // ========== List/Tree Navigation ==========
            Action::SelectNext
            | Action::SelectPrev
            | Action::SelectPageDown
            | Action::SelectPageUp
            | Action::ToggleMcpScope => {
                self.handle_list_action(action);
            }
            Action::Confirm => {
                // Defensive normalization: when overlay visibility and input mode diverge,
                // prioritize the top-most visible modal for confirm handling.
                if self.state.model_selector_state.is_visible()
                    && self.state.input_mode != InputMode::SelectingModel
                {
                    self.state.input_mode = InputMode::SelectingModel;
                } else if self.state.provider_selector_state.is_visible()
                    && self.state.input_mode != InputMode::SelectingProviders
                {
                    self.state.input_mode = InputMode::SelectingProviders;
                }

                if self.state.input_mode == InputMode::SlashMenu {
                    if let Some(entry) = self.state.slash_menu_state.selected_entry() {
                        let kind = entry.kind.clone();
                        let label = entry.label.clone();
                        self.state.slash_menu_state.hide();
                        self.state.input_mode = InputMode::Normal;
                        match kind {
                            MenuEntryKind::ConduitCommand(command) => {
                                let active_tab_index = self.state.tab_manager.active_index();
                                effects.extend(
                                    self.execute_resolved_conduit_command(
                                        active_tab_index,
                                        command,
                                    )?,
                                );
                            }
                            MenuEntryKind::ProviderInvocation(_) => {
                                if let Some(session) = self.state.tab_manager.active_session_mut() {
                                    session.input_box.clear();
                                    session.input_box.insert_str(&label);
                                    session.input_box.insert_char(' ');
                                }
                            }
                            MenuEntryKind::FilePath(_) => {}
                        }
                    }
                } else if self.state.input_mode == InputMode::FileMention {
                    let filter_char_count = self
                        .state
                        .file_mention_state
                        .list
                        .search
                        .value()
                        .chars()
                        .count();
                    let selected = self
                        .state
                        .file_mention_state
                        .selected_entry()
                        .map(|e| e.label.clone());
                    self.state.file_mention_state.hide();
                    self.state.input_mode = InputMode::Normal;
                    if let Some(path) = selected {
                        if let Some(session) = self.state.tab_manager.active_session_mut() {
                            // Delete '@' + typed filter from the input box
                            let delete_count = 1 + filter_char_count;
                            for _ in 0..delete_count {
                                session.input_box.backspace();
                            }
                            session.input_box.insert_str(&path);
                            session.input_box.insert_char(' ');
                        }
                    }
                } else if self.state.input_mode == InputMode::KeybindingsEditor {
                    self.state.keybindings_editor_state.enter_capture_mode();
                    if self.state.keybindings_editor_state.capture_mode {
                        self.state.input_mode = InputMode::KeybindingsEditorCapture;
                    }
                } else if self.state.input_mode == InputMode::CommandPalette {
                    if let Some(entry) = self.state.command_palette_state.selected_entry() {
                        let action = entry.action.clone();
                        self.state.command_palette_state.hide();
                        self.state.input_mode = InputMode::Normal;
                        // Execute the selected action (avoid recursion if it's Confirm)
                        if !matches!(action, Action::Confirm | Action::OpenCommandPalette) {
                            effects.extend(
                                Box::pin(self.execute_action(action, terminal, guard)).await?,
                            );
                        }
                    }
                } else {
                    self.handle_confirm_action(&mut effects)?;
                }
            }
            Action::SetDefaultModel => {
                if self.state.input_mode == InputMode::SelectingModel {
                    if let Some(model) = self.state.model_selector_state.selected_model().cloned() {
                        if self.persist_default_model_selection(&model) {
                            if self.state.model_picker_context
                                == ModelPickerContext::OnboardingDefaultSelection
                            {
                                self.state.model_selector_state.hide();
                                self.state.model_picker_context =
                                    ModelPickerContext::SessionSelection;
                                self.continue_new_project_flow();
                            } else if self.state.model_picker_context
                                == ModelPickerContext::SettingsDefaultSelection
                            {
                                self.state.model_selector_state.hide();
                                self.state.model_picker_context =
                                    ModelPickerContext::SessionSelection;
                                self.reopen_settings_menu();
                            }
                        }
                    }
                }
            }
            Action::Cancel
            | Action::AddRepository
            | Action::OpenSettings
            | Action::ArchiveOrRemove
            | Action::CompleteWorkspaceWork
            | Action::RenameProject
            | Action::ManageMcp => {
                self.handle_dialog_action(action);
            }

            // ========== Raw Events View ==========
            Action::RawEventsSelectNext
            | Action::RawEventsSelectPrev
            | Action::RawEventsToggleExpand
            | Action::RawEventsCollapse
            | Action::EventDetailToggle
            | Action::EventDetailScrollUp
            | Action::EventDetailScrollDown
            | Action::EventDetailPageUp
            | Action::EventDetailPageDown
            | Action::EventDetailScrollToTop
            | Action::EventDetailScrollToBottom
            | Action::EventDetailCopy => {
                self.handle_raw_events_action(action, &mut effects);
            }

            // ========== Confirmation Dialog ==========
            Action::ConfirmYes | Action::ConfirmNo | Action::ConfirmToggle => {
                self.handle_confirmation_action(action, &mut effects)?;
            }
            Action::ToggleDetails => {
                self.handle_overlay_action(action, &mut effects)?;
            }

            // ========== Agent Selection ==========
            Action::SelectAgent => {
                self.handle_overlay_action(action, &mut effects)?;
            }

            // ========== Command Mode ==========
            Action::ShowHelp => {
                self.handle_overlay_action(action, &mut effects)?;
            }
            Action::ExecuteCommand => {
                if self.state.input_mode == InputMode::Command {
                    if let Some(action) = self.execute_command() {
                        // Prevent recursion - ExecuteCommand can't call itself
                        if !matches!(action, Action::ExecuteCommand) {
                            effects.extend(
                                Box::pin(self.execute_action(action, terminal, guard)).await?,
                            );
                        }
                    }
                }
            }
            Action::CompleteCommand => {
                if self.state.input_mode == InputMode::Command {
                    self.complete_command();
                }
            }

            // ========== Command Palette ==========
            Action::OpenCommandPalette => {
                self.handle_overlay_action(action, &mut effects)?;
            }

            Action::AddFileToProject | Action::UploadFileToProject => {
                self.handle_dialog_action(action);
            }
        }

        Ok(effects)
    }

    async fn run_effects(&mut self, effects: Vec<Effect>) -> anyhow::Result<()> {
        for effect in effects {
            match effect {
                Effect::SaveSessionState => {
                    tracing::debug!("SaveSessionState effect triggered");
                    let snapshot = self.snapshot_session_state();
                    let session_tab_dao = self.session_tab_dao_clone();
                    let app_state_dao = self.app_state_dao_clone();
                    let save_result = tokio::task::spawn_blocking(move || {
                        Self::persist_session_state(snapshot, session_tab_dao, app_state_dao)
                    })
                    .await;
                    match save_result {
                        Ok(report) => self.apply_session_persistence_report(report),
                        Err(e) => {
                            tracing::warn!(error = %e, "Failed to save session state task");
                            self.state.set_timed_footer_message(
                                "Warning: failed to save session state. Check logs.".to_string(),
                                Duration::from_secs(5),
                            );
                        }
                    }
                }
                Effect::StartAgent {
                    session_id,
                    agent_type,
                    config,
                } => {
                    let runner: Arc<dyn AgentRunner> = match agent_type {
                        AgentType::Claude => self.claude_runner().clone(),
                        AgentType::Codex => self.codex_runner().clone(),
                        AgentType::Dirac => self.dirac_runner().clone(),
                        AgentType::Gemini => self.gemini_runner().clone(),
                        AgentType::DeepseekTui => self.deepseek_tui_runner().clone(),
                        AgentType::Opencode => self.opencode_runner().clone(),
                        AgentType::Copilot => self.copilot_runner().clone(),
                        AgentType::Pi => self.pi_runner().clone(),
                    };

                    let event_tx = self.event_tx.clone();

                    tokio::spawn(async move {
                        match runner.start(*config).await {
                            Ok(mut handle) => {
                                // Send PID (and input channel when available) to main app for interrupt support
                                let pid = handle.pid;
                                let input_tx = handle.take_input_sender();
                                send_app_event(
                                    &event_tx,
                                    AppEvent::AgentStarted {
                                        session_id,
                                        pid,
                                        input_tx,
                                    },
                                    "agent_started",
                                );

                                while let Some(event) = handle.events.recv().await {
                                    if !send_app_event(
                                        &event_tx,
                                        AppEvent::Agent { session_id, event },
                                        "agent_stream",
                                    ) {
                                        tracing::debug!(
                                            session_id = %session_id,
                                            "Failed to send AppEvent for agent stream"
                                        );
                                        let stop_result = tokio::time::timeout(
                                            AGENT_SHUTDOWN_TIMEOUT,
                                            runner.stop(&handle),
                                        )
                                        .await;
                                        let mut stop_ok = false;
                                        match stop_result {
                                            Ok(Ok(())) => {
                                                stop_ok = true;
                                            }
                                            Ok(Err(stop_err)) => {
                                                tracing::debug!(
                                                    session_id = %session_id,
                                                    error = %stop_err,
                                                    "Failed to stop agent after event channel closed"
                                                );
                                            }
                                            Err(_) => {
                                                tracing::debug!(
                                                    session_id = %session_id,
                                                    timeout_secs = AGENT_SHUTDOWN_TIMEOUT.as_secs(),
                                                    "Timed out stopping agent after event channel closed"
                                                );
                                            }
                                        }

                                        if !stop_ok {
                                            let kill_result = tokio::time::timeout(
                                                AGENT_SHUTDOWN_TIMEOUT,
                                                runner.kill(&handle),
                                            )
                                            .await;
                                            match kill_result {
                                                Ok(Ok(())) => {}
                                                Ok(Err(kill_err)) => {
                                                    tracing::debug!(
                                                        session_id = %session_id,
                                                        error = %kill_err,
                                                        "Failed to kill agent after event channel closed"
                                                    );
                                                }
                                                Err(_) => {
                                                    tracing::debug!(
                                                        session_id = %session_id,
                                                        timeout_secs = AGENT_SHUTDOWN_TIMEOUT.as_secs(),
                                                        "Timed out killing agent after event channel closed"
                                                    );
                                                }
                                            }
                                        }
                                        break;
                                    }
                                }
                                send_app_event(
                                    &event_tx,
                                    AppEvent::AgentStreamEnded { session_id },
                                    "agent_stream_ended",
                                );
                            }
                            Err(e) => {
                                send_app_event(
                                    &event_tx,
                                    AppEvent::AgentStartFailed {
                                        session_id,
                                        error: format!("Agent error: {}", e),
                                    },
                                    "agent_start_error",
                                );
                                send_app_event(
                                    &event_tx,
                                    AppEvent::AgentStreamEnded { session_id },
                                    "agent_stream_ended",
                                );
                            }
                        }
                    });
                }
                Effect::PrPreflight {
                    tab_index,
                    working_dir,
                } => {
                    let event_tx = self.event_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let result = PrManager::preflight_check(&working_dir);
                        send_app_event(
                            &event_tx,
                            AppEvent::PrPreflightCompleted {
                                tab_index,
                                working_dir,
                                result,
                            },
                            "pr_preflight_completed",
                        );
                    });
                }
                Effect::OpenPrInBrowser { working_dir } => {
                    let event_tx = self.event_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let result =
                            PrManager::open_pr_in_browser(&working_dir).map_err(|e| e.to_string());
                        send_app_event(
                            &event_tx,
                            AppEvent::OpenPrCompleted { result },
                            "open_pr_completed",
                        );
                    });
                }
                Effect::DumpDebugState => {
                    let result = self.dump_debug_state();
                    send_app_event(
                        &self.event_tx,
                        AppEvent::DebugDumped { result },
                        "debug_dumped",
                    );
                }
                Effect::RunShellCommand {
                    session_id,
                    message_index,
                    command,
                    working_dir,
                } => {
                    let event_tx = self.event_tx.clone();
                    let config_working_dir = self.config().working_dir.clone();
                    tokio::spawn(async move {
                        let result = async {
                            let effective_working_dir =
                                working_dir.as_ref().or(Some(&config_working_dir));
                            let effective_working_dir = match effective_working_dir {
                                Some(dir) => dir,
                                None => {
                                    return Err("No working directory available for shell command"
                                        .to_string())
                                }
                            };
                            let (shell, flag) = if cfg!(windows) {
                                ("cmd", "/C")
                            } else {
                                ("sh", "-c")
                            };
                            let mut cmd = tokio::process::Command::new(shell);
                            cmd.arg(flag).arg(&command);
                            cmd.kill_on_drop(true);
                            cmd.stdin(Stdio::null());
                            cmd.stdout(Stdio::piped());
                            cmd.stderr(Stdio::piped());
                            cmd.current_dir(effective_working_dir);

                            let mut child = cmd
                                .spawn()
                                .map_err(|e| format!("Failed to run shell command: {e}"))?;
                            let stdout = child.stdout.take().ok_or_else(|| {
                                "Failed to run shell command: stdout unavailable".to_string()
                            })?;
                            let stderr = child.stderr.take().ok_or_else(|| {
                                "Failed to run shell command: stderr unavailable".to_string()
                            })?;

                            let stdout_task = tokio::spawn(async move {
                                App::read_bounded_output(stdout, SHELL_COMMAND_OUTPUT_LIMIT).await
                            });
                            let stderr_task = tokio::spawn(async move {
                                App::read_bounded_output(stderr, SHELL_COMMAND_OUTPUT_LIMIT).await
                            });

                            let status =
                                match tokio::time::timeout(SHELL_COMMAND_TIMEOUT, child.wait())
                                    .await
                                {
                                    Ok(status) => status
                                        .map_err(|e| format!("Failed to run shell command: {e}"))?,
                                    Err(_) => {
                                        if let Err(err) = child.kill().await {
                                            tracing::debug!(
                                                error = %err,
                                                "Failed to kill timed out shell command"
                                            );
                                        }
                                        match tokio::time::timeout(
                                            SHELL_COMMAND_REAP_TIMEOUT,
                                            child.wait(),
                                        )
                                        .await
                                        {
                                            Ok(Ok(_)) => {}
                                            Ok(Err(err)) => {
                                                tracing::debug!(
                                                    error = %err,
                                                    "Failed to reap timed out shell command"
                                                );
                                            }
                                            Err(_) => {
                                                tracing::debug!(
                                                    timeout_secs =
                                                        SHELL_COMMAND_REAP_TIMEOUT.as_secs(),
                                                    "Timed out waiting to reap shell command"
                                                );
                                            }
                                        }
                                        stdout_task.abort();
                                        stderr_task.abort();
                                        if let Err(err) = stdout_task.await {
                                            tracing::debug!(
                                                error = %err,
                                                "Failed to abort stdout reader task"
                                            );
                                        }
                                        if let Err(err) = stderr_task.await {
                                            tracing::debug!(
                                                error = %err,
                                                "Failed to abort stderr reader task"
                                            );
                                        }
                                        return Err(format!(
                                            "Shell command timed out after {}s",
                                            SHELL_COMMAND_TIMEOUT.as_secs()
                                        ));
                                    }
                                };

                            let (stdout_bytes, stdout_truncated, stdout_timed_out) =
                                App::join_reader_with_timeout(stdout_task, "stdout").await?;
                            let (stderr_bytes, stderr_truncated, _stderr_timed_out) =
                                if stdout_timed_out {
                                    stderr_task.abort();
                                    if let Err(err) = stderr_task.await {
                                        tracing::debug!(
                                            error = %err,
                                            "Failed to abort stderr reader task"
                                        );
                                    }
                                    (Vec::new(), true, true)
                                } else {
                                    App::join_reader_with_timeout(stderr_task, "stderr").await?
                                };
                            let stdout = String::from_utf8_lossy(&stdout_bytes);
                            let stderr = String::from_utf8_lossy(&stderr_bytes);
                            let mut combined = String::new();
                            if !stdout.is_empty() {
                                combined.push_str(&stdout);
                            }
                            if !stderr.is_empty() {
                                if !combined.is_empty() && !combined.ends_with('\n') {
                                    combined.push('\n');
                                }
                                combined.push_str(&stderr);
                            }
                            if stdout_truncated || stderr_truncated {
                                if !combined.is_empty() && !combined.ends_with('\n') {
                                    combined.push('\n');
                                }
                                combined.push_str("[output truncated]\n");
                            }
                            Ok(crate::events::ShellCommandResult {
                                output: combined,
                                exit_code: status.code(),
                            })
                        }
                        .await;

                        send_app_event(
                            &event_tx,
                            AppEvent::ShellCommandCompleted {
                                session_id,
                                message_index,
                                result,
                            },
                            "shell_command_completed",
                        );
                    });
                }
                Effect::SyncRemote { repo_id } => {
                    let repo_dao = self.repo_dao_clone();
                    let event_tx = self.event_tx.clone();

                    tokio::task::spawn_blocking(move || {
                        if let Some(path) = repo_dao
                            .and_then(|dao| dao.get_by_id(repo_id).ok().flatten())
                            .and_then(|repo| repo.base_path)
                        {
                            let progress_tx = event_tx.clone();
                            conduit_git::sync_remote_with_progress(&path, |line| {
                                send_app_event(
                                    &progress_tx,
                                    AppEvent::RemoteSyncProgress {
                                        message: line.to_string(),
                                    },
                                    "remote_sync_progress",
                                );
                            });
                        }

                        send_app_event(
                            &event_tx,
                            AppEvent::RemoteSynced { repo_id },
                            "remote_synced",
                        );
                    });
                }
                Effect::FetchRemoteIssues { repo_id } => {
                    let repo_dao = self.repo_dao_clone();
                    let event_tx = self.event_tx.clone();
                    let issues_config = self.config().issues.clone();

                    tokio::task::spawn_blocking(move || {
                        let issues = repo_dao
                            .and_then(|dao| dao.get_by_id(repo_id).ok().flatten())
                            .and_then(|repo| repo.base_path)
                            .map(|path| conduit_git::fetch_open_issues(&path, &issues_config))
                            .unwrap_or_default();

                        send_app_event(
                            &event_tx,
                            AppEvent::RemoteIssuesFetched { repo_id, issues },
                            "remote_issues_fetched",
                        );
                    });
                }
                Effect::FetchCurrentUser { repo_id } => {
                    let repo_dao = self.repo_dao_clone();
                    let event_tx = self.event_tx.clone();
                    let issues_config = self.config().issues.clone();

                    tokio::task::spawn_blocking(move || {
                        let user = repo_dao
                            .and_then(|dao| dao.get_by_id(repo_id).ok().flatten())
                            .and_then(|repo| repo.base_path)
                            .and_then(|path| conduit_git::current_user(&path, &issues_config));

                        send_app_event(
                            &event_tx,
                            AppEvent::CurrentUserFetched { repo_id, user },
                            "current_user_fetched",
                        );
                    });
                }
                Effect::FetchAllSpecs { repo_id } => {
                    let repo_dao = self.repo_dao_clone();
                    let event_tx = self.event_tx.clone();

                    tokio::task::spawn_blocking(move || {
                        let (open_specs, specify_specs, source_ref) = repo_dao
                            .and_then(|dao| dao.get_by_id(repo_id).ok().flatten())
                            .and_then(|repo| repo.base_path)
                            .map(|path| {
                                // Prefer reading specs from `origin/<default>` so that
                                // changes archived/merged on the remote are not seen
                                // even when the local working tree is stale (e.g. on a
                                // feature branch). Fall back to the working tree if
                                // the ref can't be resolved (no origin / no remote).
                                let default_branch = conduit_git::detect_default_branch(&path)
                                    .unwrap_or_else(|| "main".to_string());
                                let git_ref = format!("origin/{}", default_branch);
                                let ref_resolves = std::process::Command::new("git")
                                    .args(["rev-parse", "--verify", &git_ref])
                                    .current_dir(&path)
                                    .output()
                                    .map(|o| o.status.success())
                                    .unwrap_or(false);
                                if ref_resolves {
                                    let open =
                                        conduit_git::fetch_open_specs_from_ref(&path, &git_ref);
                                    let specify =
                                        conduit_git::fetch_specify_specs_from_ref(&path, &git_ref);
                                    (open, specify, Some(git_ref))
                                } else {
                                    let open = conduit_git::fetch_open_specs(&path);
                                    let specify = conduit_git::fetch_specify_specs(&path);
                                    (open, specify, None)
                                }
                            })
                            .unwrap_or_default();

                        send_app_event(
                            &event_tx,
                            AppEvent::AllSpecsFetched {
                                repo_id,
                                open_specs,
                                specify_specs,
                                source_ref,
                            },
                            "all_specs_fetched",
                        );
                    });
                }
                Effect::CreateWorkspace {
                    repo_id,
                    issue,
                    spec,
                    specify_spec,
                } => {
                    let repo_dao = self.repo_dao_clone();
                    let workspace_dao = self.workspace_dao_clone();
                    let worktree_manager = self.worktree_manager().clone();
                    let config = self.config().clone();
                    let event_tx = self.event_tx.clone();

                    self.state.workspace_progress_dialog_state.show();
                    self.state.input_mode = InputMode::CreatingWorkspace;

                    tokio::task::spawn_blocking(move || {
                        let send_progress = |msg: &str| {
                            send_app_event(
                                &event_tx,
                                AppEvent::WorkspaceCreationProgress {
                                    message: msg.to_string(),
                                },
                                "workspace_creation_progress",
                            );
                        };

                        let result: Result<WorkspaceCreated, String> = (|| {
                            let repo_dao = repo_dao
                                .ok_or_else(|| "No repository DAO available".to_string())?;
                            let workspace_dao = workspace_dao
                                .ok_or_else(|| "No workspace DAO available".to_string())?;

                            let repo = repo_dao
                                .get_by_id(repo_id)
                                .map_err(|e| format!("Failed to load repository: {}", e))?
                                .ok_or_else(|| "Repository not found".to_string())?;

                            let base_path = repo
                                .base_path
                                .clone()
                                .ok_or_else(|| "Repository has no base path".to_string())?;
                            let settings = resolve_repo_workspace_settings(&config, &repo);

                            // Sync with remote immediately before cutting the worktree so
                            // create_workspace can branch from the freshest origin/<default>.
                            send_progress("Syncing with remote...");
                            conduit_git::sync_remote(&base_path);

                            // Get ALL workspace names (including archived) to prevent resurrection
                            // of old workspace names when creating new ones
                            let existing_names: Vec<String> = workspace_dao
                                .get_all_names_by_repository(repo_id)
                                .unwrap_or_default();

                            let username = conduit_util::get_git_username();
                            let (workspace_name, branch_name) = match (&issue, &spec, &specify_spec)
                            {
                                (Some(gh), _, _) => {
                                    let name = format!("gh#{}", gh.number);
                                    let branch = conduit_util::generate_branch_name(
                                        &username,
                                        &format!("gh-{}", gh.number),
                                    );
                                    (name, branch)
                                }
                                (None, Some(s), _) => {
                                    let branch =
                                        conduit_util::generate_branch_name(&username, &s.change_id);
                                    (s.change_id.clone(), branch)
                                }
                                (None, None, Some(ss)) => {
                                    let branch =
                                        conduit_util::generate_branch_name(&username, &ss.spec_id);
                                    (ss.spec_id.clone(), branch)
                                }
                                (None, None, None) => {
                                    let name =
                                        conduit_util::generate_workspace_name(&existing_names);
                                    let branch =
                                        conduit_util::generate_branch_name(&username, &name);
                                    (name, branch)
                                }
                            };

                            let worktree_path = worktree_manager
                                .create_workspace(
                                    settings.mode,
                                    &base_path,
                                    &branch_name,
                                    &workspace_name,
                                    send_progress,
                                )
                                .map_err(|e| format!("Failed to create workspace: {}", e))?;

                            let mut workspace = conduit_data::Workspace::new(
                                repo_id,
                                &workspace_name,
                                &branch_name,
                                worktree_path,
                            );
                            if let Some(ref s) = spec {
                                workspace = workspace.with_active_change(s.change_id.clone());
                            }
                            if let Some(ref gh) = issue {
                                workspace = workspace.with_active_issue(gh.number as i32);
                            }
                            let workspace_id = workspace.id;

                            if let Err(e) = workspace_dao.create(&workspace) {
                                if let Err(cleanup_err) = worktree_manager.remove_workspace(
                                    settings.mode,
                                    &base_path,
                                    &workspace.path,
                                ) {
                                    tracing::error!(
                                        error = %cleanup_err,
                                        base_path = %base_path.display(),
                                        workspace_path = %workspace.path.display(),
                                        "Failed to clean up workspace after DB error"
                                    );
                                }
                                if let Err(branch_err) = worktree_manager.delete_branch(
                                    settings.mode,
                                    &base_path,
                                    &workspace.path,
                                    &branch_name,
                                ) {
                                    tracing::error!(
                                        error = %branch_err,
                                        base_path = %base_path.display(),
                                        workspace_path = %workspace.path.display(),
                                        branch = %branch_name,
                                        "Failed to delete branch after DB error"
                                    );
                                }
                                return Err(format!("Failed to save workspace to database: {}", e));
                            }

                            conduit_util::workspace_setup::run_workspace_setup_script(
                                &base_path,
                                &workspace.path,
                                || send_progress("Running workspace setup..."),
                            );

                            let initial_message = match (&spec, &specify_spec) {
                                (Some(s), _) => Some(format!(
                                    "I just created this workspace for openspec change `{}`. \
                                     Please read the spec files in `openspec/changes/{}/` \
                                     (proposal.md, design.md, and tasks.md) and give me a \
                                     summary of what still needs to be done.",
                                    s.change_id, s.change_id
                                )),
                                (None, Some(ss)) => Some(format!(
                                    "I just created this workspace for spec `{}`. \
                                     Please read `.specify/specs/{}/tasks.md` and give me a \
                                     summary of what still needs to be done.",
                                    ss.spec_id, ss.spec_id
                                )),
                                _ => None,
                            };

                            Ok(WorkspaceCreated {
                                repo_id,
                                workspace_id,
                                initial_message,
                            })
                        })();

                        send_app_event(
                            &event_tx,
                            AppEvent::WorkspaceCreated { repo_id, result },
                            "workspace_created",
                        );
                    });
                }
                Effect::ForkWorkspace {
                    parent_workspace_id,
                    base_branch,
                } => {
                    let repo_dao = self.repo_dao_clone();
                    let workspace_dao = self.workspace_dao_clone();
                    let worktree_manager = self.worktree_manager().clone();
                    let config = self.config().clone();
                    let event_tx = self.event_tx.clone();

                    tokio::task::spawn_blocking(move || {
                        let result: Result<ForkWorkspaceCreated, String> = (|| {
                            let workspace_dao = workspace_dao
                                .ok_or_else(|| "No workspace DAO available".to_string())?;
                            let repo_dao = repo_dao
                                .ok_or_else(|| "No repository DAO available".to_string())?;

                            let parent_workspace = workspace_dao
                                .get_by_id(parent_workspace_id)
                                .map_err(|e| format!("Failed to load workspace: {}", e))?
                                .ok_or_else(|| "Workspace not found".to_string())?;

                            let repo = repo_dao
                                .get_by_id(parent_workspace.repository_id)
                                .map_err(|e| format!("Failed to load repository: {}", e))?
                                .ok_or_else(|| "Repository not found".to_string())?;

                            let base_path = repo
                                .base_path
                                .clone()
                                .ok_or_else(|| "Repository has no base path".to_string())?;
                            let settings = resolve_repo_workspace_settings(&config, &repo);

                            // Use the base_branch that was computed when the dialog was shown
                            // to ensure consistency between what was displayed and what is used

                            // Get ALL workspace names (including archived) to prevent resurrection
                            // of old workspace names when creating new ones
                            let existing_names: Vec<String> = workspace_dao
                                .get_all_names_by_repository(parent_workspace.repository_id)
                                .unwrap_or_default();

                            let workspace_name =
                                conduit_util::generate_workspace_name(&existing_names);
                            let username = conduit_util::get_git_username();
                            let branch_name =
                                conduit_util::generate_branch_name(&username, &workspace_name);

                            let worktree_path = worktree_manager
                                .create_workspace_from_branch(
                                    settings.mode,
                                    &base_path,
                                    &base_branch,
                                    &branch_name,
                                    &workspace_name,
                                    |_| {},
                                )
                                .map_err(|e| format!("Failed to create workspace: {}", e))?;

                            let workspace = conduit_data::Workspace::new(
                                parent_workspace.repository_id,
                                &workspace_name,
                                &branch_name,
                                worktree_path,
                            );
                            let workspace_id = workspace.id;

                            if let Err(e) = workspace_dao.create(&workspace) {
                                if let Err(cleanup_err) = worktree_manager.remove_workspace(
                                    settings.mode,
                                    &base_path,
                                    &workspace.path,
                                ) {
                                    tracing::error!(
                                        error = %cleanup_err,
                                        base_path = %base_path.display(),
                                        workspace_path = %workspace.path.display(),
                                        "Failed to clean up workspace after DB error"
                                    );
                                }
                                if let Err(branch_err) = worktree_manager.delete_branch(
                                    settings.mode,
                                    &base_path,
                                    &workspace.path,
                                    &branch_name,
                                ) {
                                    tracing::error!(
                                        error = %branch_err,
                                        base_path = %base_path.display(),
                                        workspace_path = %workspace.path.display(),
                                        branch = %branch_name,
                                        "Failed to delete branch after DB error"
                                    );
                                }
                                return Err(format!("Failed to save workspace to database: {}", e));
                            }

                            conduit_util::workspace_setup::run_workspace_setup_script(
                                &base_path,
                                &workspace.path,
                                || {},
                            );

                            Ok(ForkWorkspaceCreated {
                                repo_id: parent_workspace.repository_id,
                                workspace_id,
                            })
                        })(
                        );

                        send_app_event(
                            &event_tx,
                            AppEvent::ForkWorkspaceCreated {
                                parent_workspace_id,
                                result,
                            },
                            "fork_workspace_created",
                        );
                    });
                }
                Effect::RemoveProject { repo_id } => {
                    let repo_dao = self.repo_dao_clone();
                    let workspace_dao = self.workspace_dao_clone();
                    let worktree_manager = self.worktree_manager().clone();
                    let config = self.config().clone();
                    let event_tx = self.event_tx.clone();

                    tokio::task::spawn_blocking(move || {
                        let mut errors = Vec::new();
                        let mut workspace_ids = Vec::new();

                        let Some(repo_dao) = repo_dao else {
                            errors.push("No repository DAO available".to_string());
                            send_app_event(
                                &event_tx,
                                AppEvent::ProjectRemoved {
                                    result: RemoveProjectResult {
                                        repo_id,
                                        workspace_ids,
                                        errors,
                                    },
                                },
                                "project_removed",
                            );
                            return;
                        };
                        let Some(workspace_dao) = workspace_dao else {
                            errors.push("No workspace DAO available".to_string());
                            send_app_event(
                                &event_tx,
                                AppEvent::ProjectRemoved {
                                    result: RemoveProjectResult {
                                        repo_id,
                                        workspace_ids,
                                        errors,
                                    },
                                },
                                "project_removed",
                            );
                            return;
                        };

                        let (repo_base_path, repo_name, repo_settings) =
                            match repo_dao.get_by_id(repo_id) {
                                Ok(Some(repo)) => {
                                    let settings = resolve_repo_workspace_settings(&config, &repo);
                                    (repo.base_path, repo.name, Some(settings))
                                }
                                Ok(None) => {
                                    errors.push("Repository not found".to_string());
                                    send_app_event(
                                        &event_tx,
                                        AppEvent::ProjectRemoved {
                                            result: RemoveProjectResult {
                                                repo_id,
                                                workspace_ids,
                                                errors,
                                            },
                                        },
                                        "project_removed",
                                    );
                                    return;
                                }
                                Err(e) => {
                                    errors.push(format!("Failed to load repository: {}", e));
                                    send_app_event(
                                        &event_tx,
                                        AppEvent::ProjectRemoved {
                                            result: RemoveProjectResult {
                                                repo_id,
                                                workspace_ids,
                                                errors,
                                            },
                                        },
                                        "project_removed",
                                    );
                                    return;
                                }
                            };

                        let workspaces =
                            workspace_dao.get_by_repository(repo_id).unwrap_or_default();
                        for ws in workspaces {
                            workspace_ids.push(ws.id);
                            let mut archived_commit_sha = None;
                            if let (Some(base_path), Some(settings)) =
                                (repo_base_path.as_ref(), repo_settings)
                            {
                                match worktree_manager.get_branch_sha(
                                    settings.mode,
                                    base_path,
                                    &ws.path,
                                    &ws.branch,
                                ) {
                                    Ok(sha) => {
                                        archived_commit_sha = Some(sha);
                                    }
                                    Err(e) => {
                                        errors.push(format!(
                                            "Failed to read branch SHA for workspace '{}': {}",
                                            ws.name, e
                                        ));
                                    }
                                }

                                if let Err(e) = worktree_manager.remove_workspace(
                                    settings.mode,
                                    base_path,
                                    &ws.path,
                                ) {
                                    errors.push(format!(
                                        "Failed to remove worktree '{}': {}",
                                        ws.name, e
                                    ));
                                }

                                if let Err(e) = worktree_manager.delete_branch(
                                    settings.mode,
                                    base_path,
                                    &ws.path,
                                    &ws.branch,
                                ) {
                                    errors.push(format!(
                                        "Failed to delete branch '{}' for workspace '{}': {}",
                                        ws.branch, ws.name, e
                                    ));
                                }
                            }
                            if let Err(e) = workspace_dao.archive(ws.id, archived_commit_sha) {
                                errors.push(format!(
                                    "Failed to archive workspace '{}': {}",
                                    ws.name, e
                                ));
                            }
                        }

                        let workspaces_dir = conduit_util::workspaces_dir();
                        if let Some(e) =
                            conduit_util::remove_project_workspaces_dir(&workspaces_dir, &repo_name)
                        {
                            errors.push(e);
                        }

                        if let Err(e) = repo_dao.delete(repo_id) {
                            errors
                                .push(format!("Failed to delete repository from database: {}", e));
                        }

                        send_app_event(
                            &event_tx,
                            AppEvent::ProjectRemoved {
                                result: RemoveProjectResult {
                                    repo_id,
                                    workspace_ids,
                                    errors,
                                },
                            },
                            "project_removed",
                        );
                    });
                }
                Effect::CopyToClipboard(text) => {
                    use arboard::Clipboard;
                    match Clipboard::new() {
                        Ok(mut clipboard) => {
                            if let Err(e) = clipboard.set_text(&text) {
                                tracing::debug!(error = %e, "Failed to copy text to clipboard");
                            }
                        }
                        Err(e) => {
                            tracing::debug!(error = %e, "Failed to initialize clipboard");
                        }
                    }
                    // Also emit OSC 52 so the clipboard is set on the SSH client terminal.
                    // Requires tmux: set -g set-clipboard on
                    {
                        use base64::Engine as _;
                        let encoded =
                            base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
                        let osc52 = format!("\x1b]52;c;{}\x07", encoded);
                        let _ = std::io::Write::write_all(&mut std::io::stdout(), osc52.as_bytes());
                    }
                }
                Effect::DiscoverSessions => {
                    use conduit_session::{discover_sessions_incremental, SessionDiscoveryUpdate};
                    let event_tx = self.event_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        discover_sessions_incremental(|update| {
                            let event = match update {
                                SessionDiscoveryUpdate::CachedLoaded(sessions) => {
                                    AppEvent::SessionsCacheLoaded { sessions }
                                }
                                SessionDiscoveryUpdate::SessionUpdated(session) => {
                                    AppEvent::SessionUpdated { session }
                                }
                                SessionDiscoveryUpdate::SessionRemoved(file_path) => {
                                    AppEvent::SessionRemoved { file_path }
                                }
                                SessionDiscoveryUpdate::Complete => {
                                    AppEvent::SessionDiscoveryComplete
                                }
                            };
                            send_app_event(&event_tx, event, "session_discovery_update");
                        });
                    });
                }
                Effect::ImportSession(session) => {
                    // Create a new tab with the session's agent type and working directory
                    let agent_type = session.agent_type;
                    let working_dir = session
                        .project
                        .clone()
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(|| self.config().working_dir.clone());

                    // Load the session history into a new tab
                    self.create_imported_session_tab(
                        agent_type,
                        session.file_path.clone(),
                        working_dir,
                    )
                    .await?;
                }
                Effect::GenerateTitleAndBranch {
                    session_id,
                    user_message,
                    working_dir,
                    workspace_id,
                    current_branch,
                } => {
                    let tools = self.tools().clone();
                    let event_tx = self.event_tx.clone();
                    let worktree_manager = self.worktree_manager().clone();
                    let workspace_dao = self.workspace_dao_clone();

                    tokio::spawn(async move {
                        // No outer timeout here - timeout is applied inside generate_title_and_branch
                        // for the AI call. This ensures:
                        // 1. The event_tx.send always runs (not cancelled by outer timeout)
                        // 2. spawn_blocking git/db work always completes or fails deterministically
                        // 3. AI call has its own 10-second timeout in title_generator.rs
                        let result = generate_title_and_branch_impl(
                            tools,
                            user_message,
                            working_dir,
                            workspace_id,
                            current_branch,
                            worktree_manager,
                            workspace_dao,
                        )
                        .await;

                        if !send_app_event(
                            &event_tx,
                            AppEvent::TitleGenerated { session_id, result },
                            "title_generated",
                        ) {
                            tracing::debug!(%session_id, "Failed to send TitleGenerated event");
                        }
                    });
                }
                Effect::WorkCompletePreflight { workspace_id } => {
                    self.spawn_work_complete_preflight(workspace_id);
                }
                Effect::WorkCompleteAction {
                    workspace_id,
                    action,
                    payload,
                } => {
                    let workspace_dao = self.workspace_dao_clone();
                    let repo_dao = self.repo_dao_clone();
                    let worktree_manager = self.worktree_manager().clone();
                    let config = self.config().clone();
                    let event_tx = self.event_tx.clone();

                    tokio::task::spawn_blocking(move || {
                        let result = run_work_complete_action(
                            workspace_id,
                            action,
                            payload,
                            workspace_dao,
                            repo_dao,
                            worktree_manager,
                            &config,
                        );
                        send_app_event(
                            &event_tx,
                            AppEvent::WorkCompleteActionFinished {
                                workspace_id,
                                action,
                                result,
                            },
                            "work_complete_action_finished",
                        );
                    });
                }
                Effect::WorkCompleteCiMonitor {
                    workspace_id,
                    pr_url,
                } => {
                    let event_tx = self.event_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let result = conduit_git::wait_for_ci_checks(&pr_url);
                        send_app_event(
                            &event_tx,
                            AppEvent::WorkCompleteCiFinished {
                                workspace_id,
                                result,
                            },
                            "work_complete_ci_finished",
                        );
                    });
                }
            }
        }

        Ok(())
    }

    /// Helper to check if a colon keypress should trigger command mode.
    fn should_trigger_command_mode(
        key_code: KeyCode,
        key_modifiers: KeyModifiers,
        input_mode: InputMode,
        input_is_empty: bool,
        shell_mode: bool,
        has_inline_prompt: bool,
    ) -> bool {
        key_code == KeyCode::Char(':')
            && key_modifiers.is_empty()
            && input_is_empty
            && !shell_mode
            && !has_inline_prompt
            && !matches!(
                input_mode,
                InputMode::Command
                    | InputMode::ShowingHelp
                    | InputMode::AddingRepository
                    | InputMode::SettingBaseDir
                    | InputMode::PickingProject
                    | InputMode::ShowingError
                    | InputMode::SelectingAgent
                    | InputMode::Confirming
                    | InputMode::ImportingSession
                    | InputMode::CommandPalette
                    | InputMode::SlashMenu
                    | InputMode::SelectingTheme
                    | InputMode::SelectingProviders
                    | InputMode::SelectingModel
                    | InputMode::SelectingReasoning
            )
    }

    /// Helper to check if a slash or skill keypress should trigger the resolver menu.
    fn should_trigger_slash_menu(
        key_code: KeyCode,
        key_modifiers: KeyModifiers,
        input_mode: InputMode,
        input_is_empty: bool,
        shell_mode: bool,
        has_inline_prompt: bool,
        has_active_session: bool,
    ) -> bool {
        matches!(key_code, KeyCode::Char('/') | KeyCode::Char('$'))
            && key_modifiers.is_empty()
            && input_is_empty
            && has_active_session
            && !shell_mode
            && !has_inline_prompt
            && input_mode == InputMode::Normal
    }

    fn open_resolver_menu(&mut self, trigger: char) {
        let default_working_dir = self.config().working_dir.clone();
        let active_session = self.state.tab_manager.active_session();
        let working_dir = active_session
            .and_then(|session| session.working_dir.clone())
            .unwrap_or(default_working_dir);
        let active_provider = active_session.map_or(AgentType::Codex, |session| session.agent_type);
        let entries = CommandResolver::menu_entries(&working_dir, active_provider);
        self.state.close_overlays();
        self.state
            .slash_menu_state
            .show_with_entries(trigger, entries);
        self.state.input_mode = InputMode::SlashMenu;
    }

    pub(super) fn open_file_mention_menu(&mut self) {
        let default_working_dir = self.config().working_dir.clone();
        let working_dir = self
            .state
            .tab_manager
            .active_session()
            .and_then(|s| s.working_dir.clone())
            .unwrap_or(default_working_dir);
        let files = Self::scan_files_for_mention(&working_dir);
        let entries: Vec<conduit_resolver::MenuEntry> = files
            .into_iter()
            .map(|path| conduit_resolver::MenuEntry {
                label: path.clone(),
                description: String::new(),
                source_badge: String::new(),
                trigger: '@',
                kind: conduit_resolver::MenuEntryKind::FilePath(path),
            })
            .collect();
        self.state.close_overlays();
        self.state
            .file_mention_state
            .show_with_entries('@', entries);
        self.state.input_mode = InputMode::FileMention;
    }

    fn scan_files_for_mention(dir: &std::path::Path) -> Vec<String> {
        const MAX_FILES: usize = 500;
        const MAX_DEPTH: usize = 5;
        const EXCLUDED_DIRS: &[&str] = &[
            ".git",
            "target",
            "node_modules",
            "__pycache__",
            ".cargo",
            ".next",
            ".nuxt",
            "dist",
            "build",
            "out",
            ".cache",
            "vendor",
            ".venv",
            "venv",
            "env",
        ];

        let mut files = Vec::new();
        let mut dirs_to_visit: std::collections::VecDeque<(std::path::PathBuf, usize)> =
            std::collections::VecDeque::new();
        dirs_to_visit.push_back((dir.to_path_buf(), 0));

        while let Some((current_dir, depth)) = dirs_to_visit.pop_front() {
            if depth >= MAX_DEPTH {
                continue;
            }
            let read_dir = match std::fs::read_dir(&current_dir) {
                Ok(rd) => rd,
                Err(_) => continue,
            };
            for entry in read_dir.flatten() {
                let path = entry.path();
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if path.is_dir() {
                    if !EXCLUDED_DIRS.contains(&name_str.as_ref()) && !name_str.starts_with('.') {
                        dirs_to_visit.push_back((path, depth + 1));
                    }
                } else if path.is_file() {
                    if let Ok(rel) = path.strip_prefix(dir) {
                        files.push(rel.to_string_lossy().into_owned());
                        if files.len() >= MAX_FILES {
                            files.sort();
                            return files;
                        }
                    }
                }
            }
        }

        files.sort();
        files
    }

    fn slash_command_action(command: ConduitCommand) -> Option<Action> {
        match command {
            ConduitCommand::Model => Some(Action::ShowModelSelector),
            ConduitCommand::Reasoning => Some(Action::ShowReasoningSelector),
            ConduitCommand::Providers => Some(Action::ShowProvidersSelector),
            ConduitCommand::Fork => Some(Action::ForkSession),
            ConduitCommand::Handoff => Some(Action::HandoffSession),
            ConduitCommand::NewSession
            | ConduitCommand::Btw
            | ConduitCommand::Status
            | ConduitCommand::Rewind
            | ConduitCommand::AdversarialReview => None,
        }
    }

    fn execute_resolved_conduit_command(
        &mut self,
        tab_index: usize,
        command: ConduitCommand,
    ) -> anyhow::Result<Vec<Effect>> {
        if tab_index != self.state.tab_manager.active_index() {
            self.state.tab_manager.switch_to(tab_index);
            self.sync_input_mode_for_active_tab();
            self.sync_sidebar_to_active_tab();
            self.sync_footer_spinner();
            self.sync_theme_to_active_tab();
        }
        let mut effects = Vec::new();
        if let Some(action) = Self::slash_command_action(command) {
            self.handle_global_action(action, &mut effects);
            Ok(effects)
        } else if matches!(command, ConduitCommand::NewSession) {
            self.start_new_session_in_place();
            Ok(Vec::new())
        } else if matches!(command, ConduitCommand::Btw) {
            self.open_queue_editor();
            Ok(Vec::new())
        } else if matches!(command, ConduitCommand::AdversarialReview) {
            let prompt = build_adversarial_review_prompt();
            self.submit_prompt(prompt, vec![], vec![])
        } else {
            Ok(Vec::new())
        }
    }

    async fn read_bounded_output<R>(mut reader: R, limit: usize) -> io::Result<(Vec<u8>, bool)>
    where
        R: AsyncRead + Unpin,
    {
        let mut buf = Vec::with_capacity(limit.min(8192));
        let mut truncated = false;
        let mut chunk = [0u8; 8192];

        loop {
            let read = reader.read(&mut chunk).await?;
            if read == 0 {
                break;
            }

            if buf.len() < limit {
                let remaining = limit - buf.len();
                let take = remaining.min(read);
                buf.extend_from_slice(&chunk[..take]);
                if take < read {
                    truncated = true;
                }
            } else {
                truncated = true;
            }
        }

        Ok((buf, truncated))
    }

    async fn join_reader_with_timeout(
        mut task: tokio::task::JoinHandle<io::Result<(Vec<u8>, bool)>>,
        label: &'static str,
    ) -> Result<(Vec<u8>, bool, bool), String> {
        tokio::select! {
            res = &mut task => {
                let (bytes, truncated) = res
                    .map_err(|e| format!("Failed to run shell command: {e}"))?
                    .map_err(|e| format!("Failed to run shell command: {e}"))?;
                Ok((bytes, truncated, false))
            }
            _ = tokio::time::sleep(SHELL_COMMAND_REAP_TIMEOUT) => {
                task.abort();
                if let Err(err) = task.await {
                    tracing::debug!(
                        error = %err,
                        reader = label,
                        "Failed to abort reader task"
                    );
                }
                Ok((Vec::new(), true, true))
            }
        }
    }

    fn confirm_theme_picker(&mut self) -> anyhow::Result<Vec<Effect>> {
        let scope = self.state.theme_picker_state.scope();
        let previous_theme_name = self.config().theme_name.clone();
        let previous_theme_path = self.config().theme_path.clone();

        let confirmed = self.state.theme_picker_state.confirm();
        if let Some(error) = self.state.theme_picker_state.take_error() {
            self.state
                .set_timed_footer_message(error, Duration::from_secs(5));
            return Ok(Vec::new());
        }

        if let Some(theme) = confirmed {
            let (name, path) = match &theme.source {
                crate::components::ThemeSource::CustomPath { path } => (None, Some(path.clone())),
                _ => (Some(theme.name.clone()), None),
            };
            let display_name = theme.display_name.clone();

            use crate::components::ThemeScope;
            if scope == ThemeScope::Project && name.is_some() {
                // Save theme to repository record
                let repository_id = self
                    .state
                    .tab_manager
                    .active_session()
                    .and_then(|s| s.repository_id);
                if let Some(repo_id) = repository_id {
                    let dao = self.repo_dao_clone();
                    if let Some(dao) = dao {
                        if let Err(err) = dao.update_theme(repo_id, name.as_deref()) {
                            self.state.theme_picker_state.hide(true);
                            self.state.theme_picker_state.take_error();
                            self.state.set_timed_footer_message(
                                format!("Failed to save project theme: {err}"),
                                Duration::from_secs(5),
                            );
                            if !self.return_to_settings_menu_if_needed() {
                                self.state.input_mode = InputMode::Normal;
                            }
                            return Ok(Vec::new());
                        }
                    }
                    if let Some(session) = self.state.tab_manager.active_session_mut() {
                        session.project_theme = name.clone();
                    }
                    self.state
                        .sidebar_data
                        .set_repo_theme(repo_id, name.clone());
                }
                self.state.set_timed_footer_message(
                    format!("Project theme: {}", display_name),
                    Duration::from_secs(3),
                );
            } else {
                // Save theme globally (original behavior)
                if let Err(err) =
                    conduit_config::save_theme_config(name.as_deref(), path.as_deref())
                {
                    self.config_mut().theme_name = previous_theme_name;
                    self.config_mut().theme_path = previous_theme_path;
                    self.state.theme_picker_state.hide(true); // Restore original theme
                    self.state.theme_picker_state.take_error();
                    self.state.set_timed_footer_message(
                        format!("Failed to save theme: {err}"),
                        Duration::from_secs(5),
                    );
                    if !self.return_to_settings_menu_if_needed() {
                        self.state.input_mode = InputMode::Normal;
                    }
                    return Ok(Vec::new());
                }
                self.config_mut().theme_name = name;
                self.config_mut().theme_path = path;
                self.state.set_timed_footer_message(
                    format!("Theme: {}", display_name),
                    Duration::from_secs(3),
                );
            }
        }

        self.state.theme_picker_state.hide(false); // Not cancelled
        if !self.return_to_settings_menu_if_needed() {
            self.state.input_mode = InputMode::Normal;
        }
        // Re-apply the active tab's project theme if one is set, so that a global
        // theme change does not visually override a project-specific override.
        self.sync_theme_to_active_tab();
        Ok(Vec::new())
    }

    /// Clear the project theme override from the active session's repository.
    pub(super) fn clear_project_theme(&mut self) {
        let repository_id = self
            .state
            .tab_manager
            .active_session()
            .and_then(|s| s.repository_id);
        let Some(repo_id) = repository_id else {
            return;
        };
        let dao = self.repo_dao_clone();
        if let Some(dao) = dao {
            if let Err(err) = dao.update_theme(repo_id, None) {
                self.state.set_timed_footer_message(
                    format!("Failed to clear project theme: {err}"),
                    Duration::from_secs(5),
                );
                return;
            }
        }
        if let Some(session) = self.state.tab_manager.active_session_mut() {
            session.project_theme = None;
        }
        self.state.sidebar_data.set_repo_theme(repo_id, None);
        self.state.theme_picker_state.hide(true); // Close picker and restore original
        if !self.return_to_settings_menu_if_needed() {
            self.state.input_mode = InputMode::Normal;
        }
        // Apply global fallback theme
        self.sync_theme_to_active_tab();
        self.state
            .set_timed_footer_message("Project theme cleared".to_string(), Duration::from_secs(3));
    }

    fn detect_codex_project_mcp_servers(
        project_root: &std::path::Path,
    ) -> Vec<(String, McpSource)> {
        let path = project_root.join(".codex").join("config.toml");
        let Ok(contents) = std::fs::read_to_string(path) else {
            return Vec::new();
        };
        let Ok(value) = contents.parse::<toml::Value>() else {
            return Vec::new();
        };
        let mut servers: Vec<String> = value
            .get("mcp_servers")
            .and_then(toml::Value::as_table)
            .map(|table| table.keys().cloned().collect())
            .unwrap_or_default();
        servers.sort();
        servers
            .into_iter()
            .map(|name| (name, McpSource::Codex))
            .collect()
    }

    fn detect_generic_project_mcp_servers(
        project_root: &std::path::Path,
    ) -> Vec<(String, McpSource)> {
        let path = project_root.join(".mcp.json");
        let Ok(contents) = std::fs::read_to_string(path) else {
            return Vec::new();
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) else {
            return Vec::new();
        };
        let mut servers: Vec<String> = value
            .get("mcpServers")
            .and_then(serde_json::Value::as_object)
            .map(|table| table.keys().cloned().collect())
            .unwrap_or_default();
        servers.sort();
        servers
            .into_iter()
            .map(|name| (name, McpSource::McpJson))
            .collect()
    }

    pub(super) fn detect_all_mcp_servers(project_root: &std::path::Path) -> Vec<McpServer> {
        let mut servers = Self::detect_codex_project_mcp_servers(project_root);
        servers.extend(Self::detect_generic_project_mcp_servers(project_root));
        servers
            .into_iter()
            .map(|(name, source)| McpServer {
                name,
                source,
                enabled: true,
            })
            .collect()
    }

    fn resolve_disabled_servers(
        repo: &conduit_data::Repository,
        workspace: Option<&conduit_data::Workspace>,
    ) -> Vec<String> {
        match workspace.and_then(|w| w.mcp_disabled_servers.as_ref()) {
            Some(ws_list) => ws_list.clone(),
            None => repo.mcp_disabled_servers.clone(),
        }
    }

    fn extract_mcp_server_name(tool_name: &str) -> Option<&str> {
        if let Some(rest) = tool_name.strip_prefix("mcp__") {
            return rest.split("__").next();
        }
        if let Some(rest) = tool_name.strip_prefix("mcp:") {
            return rest.split(':').next();
        }
        if let Some(rest) = tool_name.strip_prefix("mcp/") {
            return rest.split('/').next();
        }
        None
    }

    fn is_claude_mcp_tool_name(tool_name: &str) -> bool {
        tool_name.starts_with("mcp__")
            || tool_name.starts_with("mcp:")
            || tool_name.starts_with("mcp/")
    }

    /// Execute a command from command mode
    /// Returns an action to execute if the command maps to one
    fn execute_command(&mut self) -> Option<Action> {
        let command = std::mem::take(&mut self.state.command_buffer);
        let command = command.trim();
        self.state.input_mode = InputMode::Normal;

        // Check for :open command first (preserve path case, case-insensitive command)
        let mut parts = command.splitn(2, char::is_whitespace);
        let cmd = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("").trim();
        if cmd.eq_ignore_ascii_case("open") || cmd.eq_ignore_ascii_case("o") {
            if rest.is_empty() {
                self.state.set_timed_footer_message(
                    "Usage: :open <path>".to_string(),
                    Duration::from_secs(3),
                );
                return None;
            }

            let mut path = rest;
            // Allow quoted paths (common for paths with spaces)
            path = path
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .or_else(|| path.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
                .unwrap_or(path);

            if !path.is_empty() {
                // Expand tilde to home directory
                // Only expand ~ or ~/path (not ~username which would require system lookup)
                let needs_home = path == "~" || path.starts_with("~/") || path.starts_with("~\\");

                if needs_home && dirs::home_dir().is_none() {
                    self.state.set_timed_footer_message(
                        "Home directory not found; cannot expand ~".to_string(),
                        Duration::from_secs(3),
                    );
                    return None;
                }

                let home = dirs::home_dir()?;
                let mut expanded_path = match path {
                    "~" => home,
                    _ => {
                        if let Some(rest) =
                            path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\"))
                        {
                            home.join(rest)
                        } else {
                            std::path::PathBuf::from(path)
                        }
                    }
                };

                // Resolve relative paths against the active workspace (fallback to config working dir)
                if expanded_path.is_relative() {
                    let base_dir = self
                        .state
                        .tab_manager
                        .active_session()
                        .and_then(|s| s.working_dir.clone())
                        .unwrap_or_else(|| self.config().working_dir.clone());
                    expanded_path = base_dir.join(expanded_path);
                }

                return Some(Action::OpenFile(expanded_path));
            }
        }

        let command_lower = command.to_lowercase();

        // First check for built-in command aliases
        match command_lower.as_str() {
            "help" | "h" | "?" => {
                self.state.close_overlays();
                let keybindings = self.config().keybindings.clone();
                self.state.help_dialog_state.show(&keybindings);
                self.state.input_mode = InputMode::ShowingHelp;
                return None;
            }
            "q" => {
                return Some(Action::Quit);
            }
            _ => {}
        }

        // Try to parse as an action name
        parse_action(&command_lower)
    }

    /// Autocomplete the command buffer
    fn complete_command(&mut self) {
        let prefix = self.state.command_buffer.trim().to_lowercase();
        if prefix.is_empty() {
            return;
        }

        // Find all matching commands
        let matches: Vec<&str> = COMMAND_NAMES
            .iter()
            .filter(|cmd| cmd.starts_with(&prefix))
            .copied()
            .collect();

        if matches.is_empty() {
            return;
        }

        if matches.len() == 1 {
            // Single match - complete fully
            self.state.command_buffer = matches[0].to_string();
        } else {
            // Multiple matches - complete to longest common prefix
            let common = Self::longest_common_prefix(&matches);
            if common.len() > prefix.len() {
                self.state.command_buffer = common;
            } else {
                // Already at common prefix - cycle to next match
                let current = &self.state.command_buffer;
                let Some(next) = matches
                    .iter()
                    .find(|&&cmd| cmd > current.as_str())
                    .or(matches.first())
                else {
                    return;
                };
                self.state.command_buffer = (*next).to_string();
            }
        }
    }

    /// Find longest common prefix among strings
    fn longest_common_prefix(strings: &[&str]) -> String {
        if strings.is_empty() {
            return String::new();
        }
        if strings.len() == 1 {
            return strings[0].to_string();
        }

        let first = strings[0];
        let mut prefix_len = first.len();

        for s in &strings[1..] {
            prefix_len = first
                .chars()
                .zip(s.chars())
                .take_while(|(a, b)| a == b)
                .count()
                .min(prefix_len);
        }

        first[..prefix_len].to_string()
    }

    /// Open a workspace (create or switch to tab)
    /// If `close_sidebar` is true, the sidebar will be hidden after opening.
    fn open_workspace_with_options(&mut self, workspace_id: uuid::Uuid, close_sidebar: bool) {
        // Check if there's already a tab with this workspace - switch to it
        if let Some(existing_index) = self.find_tab_for_workspace(workspace_id) {
            self.state.tab_manager.switch_to(existing_index);
            self.sync_footer_spinner();
            self.sync_theme_to_active_tab();
            if close_sidebar {
                self.state.sidebar_state.hide();
                self.state.input_mode = InputMode::Normal;
            }
            return;
        }

        // Find the workspace
        let Some(workspace_dao) = self.workspace_dao() else {
            return;
        };

        let Ok(Some(workspace)) = workspace_dao.get_by_id(workspace_id) else {
            return;
        };

        // Verify workspace path exists
        if !workspace.path.exists() {
            tracing::error!(
                workspace_id = %workspace_id,
                path = %workspace.path.display(),
                "Workspace path does not exist"
            );
            // TODO: Could offer to recreate the worktree or delete the workspace
            return;
        }

        // Get the repository name, ID, project theme, and orchestration default for the tab
        let (
            project_name,
            repository_id,
            project_theme,
            repo_orchestration,
            repo_adversarial_review_enabled,
            repo_adversarial_review_model,
        ) = self
            .repo_dao()
            .and_then(|dao| dao.get_by_id(workspace.repository_id).ok().flatten())
            .map(|repo| {
                (
                    Some(repo.name),
                    Some(repo.id),
                    repo.theme_name,
                    repo.orchestration_enabled,
                    repo.adversarial_review_enabled,
                    repo.adversarial_review_model,
                )
            })
            .unwrap_or((None, None, None, None, None, None));

        // Check if there's a saved session for this workspace (to restore chat history)
        let saved_tab = self
            .session_tab_dao()
            .and_then(|dao| dao.get_by_workspace_id(workspace_id).ok().flatten());

        // Update last accessed
        if let Err(e) = workspace_dao.update_last_accessed(workspace_id) {
            tracing::debug!(
                error = %e,
                workspace_id = %workspace_id,
                "Failed to update workspace last accessed time"
            );
        }

        let has_saved_session = saved_tab.is_some();
        let no_agents_available = !self.tools().is_available(conduit_util::Tool::Claude)
            && !self.tools().is_available(conduit_util::Tool::Codex)
            && !self.tools().is_available(conduit_util::Tool::Gemini)
            && !self.tools().is_available(conduit_util::Tool::Opencode);
        let tab_agent_type = saved_tab
            .as_ref()
            .map(|saved| saved.agent_type)
            .unwrap_or_else(|| {
                self.preferred_provider_for_new_sessions()
                    .unwrap_or(self.config().default_agent)
            });

        let saved_agent_mode = saved_tab.as_ref().map(|saved| {
            let parsed_mode = saved
                .agent_mode
                .as_deref()
                .map(AgentMode::parse)
                .unwrap_or_default();
            Self::clamp_agent_mode(saved.agent_type, parsed_mode)
        });

        let required_tool = Self::required_tool(tab_agent_type);
        if !self.tools().is_available(required_tool) {
            self.show_missing_tool(
                required_tool,
                if has_saved_session {
                    format!(
                        "{} is required to open this workspace's saved session.",
                        required_tool.display_name()
                    )
                } else if no_agents_available {
                    "An agent tool (Claude Code, Codex CLI, Gemini CLI, or OpenCode) is required to open this workspace."
                        .to_string()
                } else {
                    format!(
                        "{} is required to open this workspace.",
                        required_tool.display_name()
                    )
                },
            );
            if close_sidebar {
                self.state.sidebar_state.hide();
            }
            return;
        }

        // Create a new tab with the workspace's working directory
        if self
            .state
            .tab_manager
            .new_tab_with_working_dir(tab_agent_type, workspace.path.clone())
            .is_none()
        {
            self.show_error(
                "Too many tabs",
                "Maximum number of tabs reached. Close a tab before opening another workspace.",
            );
            return;
        }

        // Get default model and orchestration config before the mutable borrow
        let default_model = self.config().default_model_for(tab_agent_type);
        let global_orchestration_default = self.config().orchestration.enabled_by_default;

        let session_tab_dao = self.session_tab_dao_clone();

        // Store workspace info in session and restore chat history if available
        if let Some(session) = self.state.tab_manager.active_session_mut() {
            session.workspace_id = Some(workspace_id);
            session.repository_id = repository_id;
            session.project_name = project_name;
            session.project_theme = project_theme;
            session.workspace_name = Some(workspace.name.clone());
            session.branch_name = Some(workspace.branch.clone());

            // Restore saved session data if available
            if let Some(saved) = saved_tab.as_ref() {
                session.id = saved.id;
                if let Some(session_tab_dao) = session_tab_dao.as_ref() {
                    if let Err(e) = session_tab_dao.set_open(saved.id, true) {
                        tracing::warn!(error = %e, "Failed to mark saved session as open");
                    }
                }
            }
            if let Some(saved) = saved_tab {
                session.set_agent_and_model(saved.agent_type, saved.model);
                session.title = saved.title.clone();
                if let Some(saved_mode) = saved_agent_mode {
                    session.agent_mode = saved_mode; // Pre-clamped above
                }
                session.fork_seed_id = saved.fork_seed_id;

                // Restore chat history from agent files
                if let Some(ref session_id_str) = saved.agent_session_id {
                    let session_id = SessionId::from_string(session_id_str.clone());
                    session.resume_session_id = Some(session_id.clone());
                    if saved.agent_type != AgentType::Codex {
                        session.agent_session_id = Some(session_id);
                    }

                    // Load chat history
                    match saved.agent_type {
                        AgentType::Claude => {
                            if let Ok((msgs, debug_entries, file_path)) =
                                load_claude_history_with_debug(session_id_str)
                            {
                                // Populate debug pane with history load info
                                Self::populate_debug_from_history(
                                    &mut session.raw_events_view,
                                    &debug_entries,
                                    &file_path,
                                );
                                for msg in msgs {
                                    session.chat_view.push(msg);
                                }
                            }
                        }
                        AgentType::Codex => {
                            if let Ok((msgs, debug_entries, file_path)) =
                                load_codex_history_with_debug(session_id_str)
                            {
                                // Populate debug pane with history load info
                                Self::populate_debug_from_history(
                                    &mut session.raw_events_view,
                                    &debug_entries,
                                    &file_path,
                                );
                                for msg in msgs {
                                    session.chat_view.push(msg);
                                }
                            }
                        }
                        AgentType::Dirac => {
                            session.chat_view.push(
                                MessageDisplay::System {
                                    content: "Dirac history import isn't supported yet, so previous messages won't be shown.".to_string(),
                                }
                                .to_chat_message(),
                            );
                        }
                        AgentType::Gemini => {
                            session.chat_view.push(
                                MessageDisplay::System {
                                    content: "Gemini CLI history import isn't supported yet, so previous messages won't be shown.".to_string(),
                                }
                                .to_chat_message(),
                            );
                        }
                        AgentType::DeepseekTui => {
                            session.chat_view.push(
                                MessageDisplay::System {
                                    content: "DeepSeek TUI history import isn't supported yet, so previous messages won't be shown.".to_string(),
                                }
                                .to_chat_message(),
                            );
                        }
                        AgentType::Opencode => {
                            if let Ok((msgs, debug_entries, file_path)) =
                                load_opencode_history_with_debug(session_id_str)
                            {
                                Self::populate_debug_from_history(
                                    &mut session.raw_events_view,
                                    &debug_entries,
                                    &file_path,
                                );
                                for msg in msgs {
                                    session.chat_view.push(msg);
                                }
                            }
                        }
                        AgentType::Copilot => {
                            session.chat_view.push(
                                MessageDisplay::System {
                                    content: "GitHub Copilot history import isn't supported yet, so previous messages won't be shown.".to_string(),
                                }
                                .to_chat_message(),
                            );
                        }
                        AgentType::Pi => {
                            if let Ok((msgs, debug_entries, file_path)) =
                                load_pi_history_with_debug(session_id_str)
                            {
                                Self::populate_debug_from_history(
                                    &mut session.raw_events_view,
                                    &debug_entries,
                                    &file_path,
                                );
                                for msg in msgs {
                                    session.chat_view.push(msg);
                                }
                            }
                        }
                    }
                } else if saved.agent_type == AgentType::Opencode {
                    if let Some(working_dir) = session.working_dir.as_ref() {
                        if let Ok((session_id_str, msgs, debug_entries, file_path)) =
                            load_opencode_history_for_dir_with_debug(working_dir)
                        {
                            let session_id = SessionId::from_string(session_id_str.clone());
                            session.resume_session_id = Some(session_id.clone());
                            session.agent_session_id = Some(session_id);

                            Self::populate_debug_from_history(
                                &mut session.raw_events_view,
                                &debug_entries,
                                &file_path,
                            );
                            for msg in msgs {
                                session.chat_view.push(msg);
                            }
                        }
                    }
                }

                // Restore pending user message if it exists and isn't already in history
                if let Some(ref pending) = saved.pending_user_message {
                    let already_in_history = session
                        .chat_view
                        .messages()
                        .iter()
                        .rev()
                        .find(|m| m.role == MessageRole::User)
                        .map(|m| m.content.as_str() == pending.as_str())
                        .unwrap_or(false);

                    if !already_in_history {
                        let display = MessageDisplay::User {
                            content: pending.clone(),
                        };
                        session.chat_view.push(display.to_chat_message());
                        session.pending_user_message = Some(pending.clone());
                    }
                }

                if !saved.queued_messages.is_empty() {
                    session.queued_messages = saved.queued_messages.clone();
                }

                // Derive fork_welcome_shown: if restoring a forked session that has messages,
                // the welcome message was already shown in the previous session
                if session.fork_seed_id.is_some() && !session.chat_view.messages().is_empty() {
                    session.fork_welcome_shown = true;
                }
            } else {
                session.model = Some(default_model.clone());
                session.model_invalid = false;
                session.init_context_for_model();
            }

            // Resolve orchestration default: workspace override → project override → global config
            if session.agent_type == conduit_agent::AgentType::Claude {
                session.orchestration_enabled = workspace
                    .orchestration_enabled
                    .or(repo_orchestration)
                    .unwrap_or(global_orchestration_default);
                session.adversarial_review_enabled = workspace
                    .adversarial_review_enabled
                    .or(repo_adversarial_review_enabled)
                    .unwrap_or(false);
                session.adversarial_review_model = workspace
                    .adversarial_review_model
                    .clone()
                    .or(repo_adversarial_review_model.clone());
            }

            session.update_status();
        }

        // Register workspace with git tracker for background status updates
        if let Some(ref tracker) = self.git_tracker {
            tracker.track_workspace(workspace_id, workspace.path.clone());
        }

        // Close the sidebar and switch to normal mode (if requested)
        if close_sidebar {
            self.state.sidebar_state.hide();
            self.state.input_mode = InputMode::Normal;
        }

        self.sync_theme_to_active_tab();
        self.refresh_sidebar_data();
    }

    /// Open a workspace (create or switch to tab), closing the sidebar unless always_show_sidebar is set
    fn open_workspace(&mut self, workspace_id: uuid::Uuid) {
        let close_sidebar = !self.config().ui.always_show_sidebar;
        self.open_workspace_with_options(workspace_id, close_sidebar);
    }

    /// Clamp unsupported agent modes to a safe default.
    fn clamp_agent_mode(agent_type: AgentType, mode: AgentMode) -> AgentMode {
        if AgentCapabilities::for_agent(agent_type).supports_plan_mode {
            mode
        } else {
            AgentMode::Build
        }
    }

    /// Map an agent type to its required tool.
    fn required_tool(agent_type: AgentType) -> conduit_util::Tool {
        match agent_type {
            AgentType::Claude => conduit_util::Tool::Claude,
            AgentType::Codex => conduit_util::Tool::Codex,
            AgentType::Dirac => conduit_util::Tool::Dirac,
            AgentType::Gemini => conduit_util::Tool::Gemini,
            AgentType::DeepseekTui => conduit_util::Tool::DeepseekTui,
            AgentType::Opencode => conduit_util::Tool::Opencode,
            AgentType::Copilot => conduit_util::Tool::Copilot,
            AgentType::Pi => conduit_util::Tool::Pi,
        }
    }

    fn reasoning_supported(agent_type: AgentType) -> bool {
        matches!(agent_type, AgentType::Claude | AgentType::Codex)
    }

    fn session_started(session: &AgentSession) -> bool {
        session.agent_session_id.is_some()
            || session.resume_session_id.is_some()
            || session.agent_input_tx.is_some()
            || session.turn_count > 0
    }

    fn reject_cross_agent_switch(session: &mut AgentSession, target_agent: AgentType) -> bool {
        if session.agent_type == target_agent || !Self::session_started(session) {
            return false;
        }

        let display = MessageDisplay::Error {
            content: "Switching agent type after a session has started is not supported. Start a new session/tab to change agents."
                .to_string(),
        };
        session.chat_view.push(display.to_chat_message());
        true
    }

    fn preferred_provider_for_new_sessions(&self) -> Option<AgentType> {
        let enabled = self.config().effective_enabled_providers(self.tools());
        if enabled.is_empty() {
            return None;
        }

        let default_provider = self.config().default_agent;
        if enabled.contains(&default_provider) {
            return Some(default_provider);
        }

        AgentType::preferred_order()
            .into_iter()
            .find(|provider| enabled.contains(provider))
    }

    fn preferred_provider_for_handoff(&self, source_agent: AgentType) -> AgentType {
        let enabled = self.config().effective_enabled_providers(self.tools());

        if let Some(provider) = AgentType::preferred_order()
            .into_iter()
            .find(|provider| enabled.contains(provider) && *provider != source_agent)
        {
            return provider;
        }

        if let Some(provider) = self.preferred_provider_for_new_sessions() {
            return provider;
        }

        source_agent
    }

    fn model_selector_defaults(&self) -> DefaultModelSelection {
        let agent_type = self
            .preferred_provider_for_new_sessions()
            .unwrap_or(self.config().default_agent);
        DefaultModelSelection {
            agent_type: Some(agent_type),
            model_id: Some(self.config().default_model_for(agent_type)),
        }
    }

    fn resolve_new_project_target(&self) -> NewProjectTarget {
        let base_dir = self
            .app_state_dao()
            .and_then(|dao| dao.get("projects_base_dir").ok().flatten());

        if let Some(base_dir_str) = base_dir {
            let base_path = if base_dir_str.starts_with('~') {
                dirs::home_dir()
                    .map(|h| h.join(base_dir_str[1..].trim_start_matches('/')))
                    .unwrap_or_else(|| PathBuf::from(&base_dir_str))
            } else {
                PathBuf::from(&base_dir_str)
            };
            NewProjectTarget::ProjectPicker(base_path)
        } else {
            NewProjectTarget::BaseDirDialog
        }
    }

    fn open_new_project_target(&mut self, target: NewProjectTarget) {
        match target {
            NewProjectTarget::ProjectPicker(base_path) => {
                self.start_project_discovery(base_path);
            }
            NewProjectTarget::BaseDirDialog => {
                self.state.base_dir_dialog_context = BaseDirDialogContext::ProjectDiscovery;
                self.state.base_dir_dialog_state.show();
                self.state.input_mode = InputMode::SettingBaseDir;
            }
        }
    }

    fn start_project_discovery(&mut self, base_dir: PathBuf) {
        self.state.close_overlays();
        self.state
            .project_picker_state
            .show_loading(base_dir.clone());
        self.state.input_mode = InputMode::PickingProject;

        let event_base_dir = base_dir.clone();
        self.spawn_blocking_preflight(
            move || {
                let projects = crate::components::ProjectPickerState::scan_projects(base_dir)?;
                Ok(projects
                    .into_iter()
                    .map(|project| ProjectDiscoveryEntry {
                        name: project.name,
                        path: project.path,
                    })
                    .collect())
            },
            move |result| AppEvent::ProjectsDiscovered {
                base_dir: event_base_dir,
                result,
            },
            "projects_discovered",
        );
    }

    fn show_onboarding_provider_selector(&mut self) {
        self.state.provider_selector_state =
            crate::components::ProviderSelectorState::configure_for(self.config(), self.tools());
        self.state.provider_selector_state.show();
        self.state.input_mode = InputMode::SelectingProviders;
    }

    fn show_onboarding_model_selector(&mut self) -> bool {
        let allowed = self.config().effective_enabled_providers(self.tools());
        if allowed.is_empty() {
            self.state.set_timed_footer_message(
                "No enabled providers available. Use /providers.".to_string(),
                Duration::from_secs(4),
            );
            self.state.pending_new_project_target = None;
            self.state.input_mode = InputMode::Normal;
            return false;
        }

        self.state
            .model_selector_state
            .set_allowed_providers(Some(allowed));
        self.state.model_selector_state.show_with_title(
            None,
            DefaultModelSelection::default(),
            "Pick your default model".to_string(),
        );
        self.state.model_picker_context = ModelPickerContext::OnboardingDefaultSelection;
        self.state.input_mode = InputMode::SelectingModel;
        true
    }

    fn continue_new_project_flow(&mut self) {
        if self.state.pending_new_project_target.is_none() {
            self.state.input_mode = InputMode::Normal;
            return;
        }

        if self.config().enabled_providers.is_none() {
            self.show_onboarding_provider_selector();
            return;
        }

        if self.config().default_model.is_none() {
            let _ = self.show_onboarding_model_selector();
            return;
        }

        if let Some(target) = self.state.pending_new_project_target.take() {
            self.open_new_project_target(target);
        } else {
            self.state.input_mode = InputMode::Normal;
        }
    }

    fn persist_default_model_selection(&mut self, model: &conduit_agent::ModelInfo) -> bool {
        let model_id = model.id.clone();
        let agent_type = model.agent_type;
        self.state
            .model_selector_state
            .set_default_model(agent_type, model_id.clone());

        if let Err(err) = conduit_core::services::ConfigService::set_default_model(
            &mut self.core,
            agent_type,
            &model_id,
        ) {
            tracing::warn!(error = %err, "Failed to save default model");
            self.state.set_timed_footer_message(
                format!("Failed to save default model: {err}"),
                Duration::from_secs(5),
            );
            return false;
        }

        self.state.set_timed_footer_message(
            format!("Default model set to: {}", model.display_name),
            Duration::from_secs(5),
        );
        true
    }

    fn open_project_picker_or_base_dir(&mut self) {
        self.state.close_overlays();
        self.state.pending_new_project_target = Some(self.resolve_new_project_target());
        self.continue_new_project_flow();
    }

    fn projects_base_dir_value(&self) -> String {
        self.app_state_dao()
            .and_then(|dao| dao.get("projects_base_dir").ok().flatten())
            .unwrap_or_else(|| "Not set".to_string())
    }

    fn settings_menu_entries(&self) -> Vec<SettingsMenuEntry> {
        let default_agent = self
            .preferred_provider_for_new_sessions()
            .unwrap_or(self.config().default_agent);
        let default_model_id = self.config().default_model_for(default_agent);
        let default_model = ModelRegistry::find_model(default_agent, &default_model_id)
            .map(|model| format!("{}: {}", default_agent.display_name(), model.display_name))
            .unwrap_or_else(|| format!("{}: {}", default_agent.display_name(), default_model_id));

        let enabled_providers = self
            .config()
            .effective_enabled_providers(self.tools())
            .into_iter()
            .map(|provider| provider.display_name().to_string())
            .collect::<Vec<_>>();

        let theme_value = self
            .config()
            .theme_name
            .clone()
            .or_else(|| {
                self.config()
                    .theme_path
                    .as_ref()
                    .map(|path| path.display().to_string())
            })
            .unwrap_or_else(crate::components::current_theme_name);

        let workspace_defaults = format!(
            "{}, delete branch {}, remote prompt {}",
            self.config().workspaces.default_mode.as_str(),
            if self.config().workspaces.archive_delete_branch {
                "on"
            } else {
                "off"
            },
            if self.config().workspaces.archive_remote_prompt {
                "on"
            } else {
                "off"
            }
        );

        vec![
            SettingsMenuEntry {
                id: SettingsMenuEntryId::ProjectsDirectory,
                title: "Projects Directory".to_string(),
                description: "Where Conduit scans for local git projects".to_string(),
                value: self.projects_base_dir_value(),
            },
            SettingsMenuEntry {
                id: SettingsMenuEntryId::DefaultModel,
                title: "Default Model".to_string(),
                description: "Agent + model used for new sessions".to_string(),
                value: default_model,
            },
            SettingsMenuEntry {
                id: SettingsMenuEntryId::EnabledProviders,
                title: "Agent CLIs".to_string(),
                description: "Agent CLIs available for new sessions".to_string(),
                value: if enabled_providers.is_empty() {
                    "None".to_string()
                } else {
                    enabled_providers.join(", ")
                },
            },
            SettingsMenuEntry {
                id: SettingsMenuEntryId::Theme,
                title: "Theme".to_string(),
                description: "Active color theme".to_string(),
                value: theme_value,
            },
            SettingsMenuEntry {
                id: SettingsMenuEntryId::WorkspaceDefaults,
                title: "Workspace Defaults".to_string(),
                description: "Defaults applied when a repo has no override".to_string(),
                value: workspace_defaults,
            },
            SettingsMenuEntry {
                id: SettingsMenuEntryId::Keybindings,
                title: "Keybindings".to_string(),
                description: "Customize keyboard shortcuts".to_string(),
                value: String::new(),
            },
        ]
    }

    fn open_settings_menu(&mut self) {
        self.state.close_overlays();
        self.state
            .settings_menu_state
            .show(self.settings_menu_entries());
        self.state.input_mode = InputMode::SettingsMenu;
    }

    fn reopen_settings_menu(&mut self) {
        self.state
            .settings_menu_state
            .show(self.settings_menu_entries());
        self.state.input_mode = InputMode::SettingsMenu;
    }

    fn open_settings_child(&mut self) {
        self.state.settings_menu_state.hide();
        self.state.settings_menu_return = true;
    }

    fn return_to_settings_menu_if_needed(&mut self) -> bool {
        if self.state.settings_menu_return {
            self.state.settings_menu_return = false;
            self.reopen_settings_menu();
            return true;
        }
        false
    }

    fn open_selected_setting(&mut self) {
        let Some(entry) = self.state.settings_menu_state.selected_entry().cloned() else {
            return;
        };

        match entry.id {
            SettingsMenuEntryId::ProjectsDirectory => {
                self.open_settings_child();
                self.state.base_dir_dialog_context = BaseDirDialogContext::Settings;
                if let Some(dao) = self.app_state_dao() {
                    if let Ok(Some(current_dir)) = dao.get("projects_base_dir") {
                        self.state
                            .base_dir_dialog_state
                            .show_with_path(&current_dir);
                    } else {
                        self.state.base_dir_dialog_state.show();
                    }
                } else {
                    self.state.base_dir_dialog_state.show();
                }
                self.state.input_mode = InputMode::SettingBaseDir;
            }
            SettingsMenuEntryId::DefaultModel => {
                let allowed = self.config().effective_enabled_providers(self.tools());
                if allowed.is_empty() {
                    self.state.set_timed_footer_message(
                        "No enabled providers available. Use providers first.".to_string(),
                        Duration::from_secs(4),
                    );
                    return;
                }
                self.open_settings_child();
                self.state
                    .model_selector_state
                    .set_allowed_providers(Some(allowed));
                self.state.model_selector_state.show_with_title(
                    None,
                    self.model_selector_defaults(),
                    "Pick your default model".to_string(),
                );
                self.state.model_picker_context = ModelPickerContext::SettingsDefaultSelection;
                self.state.input_mode = InputMode::SelectingModel;
            }
            SettingsMenuEntryId::EnabledProviders => {
                self.open_settings_child();
                self.state.provider_selector_state =
                    crate::components::ProviderSelectorState::configure_for(
                        self.config(),
                        self.tools(),
                    );
                self.state.provider_selector_state.show();
                self.state.input_mode = InputMode::SelectingProviders;
            }
            SettingsMenuEntryId::Theme => {
                self.open_settings_child();
                let theme_path = self.config().theme_path.clone();
                let project_theme = self
                    .state
                    .tab_manager
                    .active_session()
                    .and_then(|s| s.project_theme.clone());
                let has_project_context = self
                    .state
                    .tab_manager
                    .active_session()
                    .and_then(|s| s.workspace_id)
                    .is_some();
                self.state.theme_picker_state.show_with_project_context(
                    theme_path.as_deref(),
                    project_theme.as_deref(),
                    has_project_context,
                );
                self.state.input_mode = InputMode::SelectingTheme;
            }
            SettingsMenuEntryId::WorkspaceDefaults => {
                self.open_settings_child();
                self.state
                    .workspace_defaults_dialog_state
                    .show(WorkspaceDefaultsDraft {
                        mode: self.config().workspaces.default_mode,
                        archive_delete_branch: self.config().workspaces.archive_delete_branch,
                        archive_remote_prompt: self.config().workspaces.archive_remote_prompt,
                    });
                self.state.input_mode = InputMode::WorkspaceDefaults;
            }
            SettingsMenuEntryId::Keybindings => {
                self.open_settings_child();
                let items = build_keybinding_items(&self.config().keybindings);
                self.state.keybindings_editor_state.show(items);
                self.state.input_mode = InputMode::KeybindingsEditor;
            }
        }
    }

    /// Show missing tool dialog and enter MissingTool mode.
    fn show_missing_tool(&mut self, tool: conduit_util::Tool, message: impl Into<String>) {
        self.state.close_overlays();
        self.state
            .missing_tool_dialog_state
            .show_with_context(tool, message);
        self.state.input_mode = InputMode::MissingTool;
    }

    /// Find the tab index for a workspace if it's already open
    fn find_tab_for_workspace(&self, workspace_id: uuid::Uuid) -> Option<usize> {
        self.state.tab_manager.tabs().iter().position(|tab| {
            tab.as_agent()
                .is_some_and(|session| session.workspace_id == Some(workspace_id))
        })
    }

    /// Extract PR number from text containing a GitHub PR URL
    /// Looks for patterns like "github.com/owner/repo/pull/123"
    fn extract_pr_number_from_text(text: &str) -> Option<u32> {
        // Look for GitHub PR URLs in the text
        for word in text.split_whitespace() {
            // Check if this word contains a GitHub PR URL
            if let Some(pull_idx) = word.find("/pull/") {
                // Extract the part after "/pull/"
                let after_pull = &word[pull_idx + 6..];
                // Parse the number (stop at any non-digit character)
                let num_str: String = after_pull
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if !num_str.is_empty() {
                    if let Ok(num) = num_str.parse::<u32>() {
                        return Some(num);
                    }
                }
            }
        }
        None
    }

    /// Build a minimal PR status from a known PR number (used when full status is unavailable).
    fn synthesize_pr_status(number: u32) -> PrStatus {
        PrStatus {
            exists: true,
            number: Some(number),
            ..Default::default()
        }
    }

    /// Apply PR status to a session and return the workspace_id for sidebar updates.
    fn apply_pr_status_to_session(
        session: &mut AgentSession,
        mut status: PrStatus,
    ) -> Option<(Uuid, PrStatus)> {
        let effective_number = status.number.or(session.pr_number);
        status.number = effective_number;
        session.pr_number = effective_number;
        session.status_bar.set_pr_status(Some(status.clone()));
        session.workspace_id.map(|id| (id, status))
    }

    fn apply_pr_number_to_session(
        session: &mut AgentSession,
        pr_num: u32,
    ) -> Option<(Uuid, PrStatus)> {
        let status = Self::synthesize_pr_status(pr_num);
        Self::apply_pr_status_to_session(session, status)
    }

    /// Estimate token usage for a prompt (rough heuristic)
    fn estimate_tokens(text: &str) -> i64 {
        let chars = text.chars().count().max(1);
        ((chars as f64) / 4.0).ceil() as i64
    }

    /// Populate the debug pane with history loading debug entries
    fn populate_debug_from_history(
        raw_events_view: &mut crate::components::RawEventsView,
        debug_entries: &[HistoryDebugEntry],
        file_path: &std::path::Path,
    ) {
        use crate::components::EventDirection;

        // First, add a header entry showing the file being loaded
        let header_json = serde_json::json!({
            "action": "history_load",
            "file": file_path.to_string_lossy(),
            "total_entries": debug_entries.len(),
            "included": debug_entries.iter().filter(|e| e.status == "INCLUDE").count(),
            "skipped": debug_entries.iter().filter(|e| e.status == "SKIP").count(),
        });
        raw_events_view.push_event(EventDirection::Received, "history_load", header_json);

        // Add each debug entry
        for entry in debug_entries {
            // Create a summary JSON that includes status info
            let summary_json = serde_json::json!({
                "line": entry.line_number,
                "type": entry.entry_type,
                "status": entry.status,
                "reason": entry.reason,
                "raw": entry.raw_json,
            });

            let event_type = format!(
                "L{} {} {}",
                entry.line_number, entry.status, entry.entry_type
            );
            raw_events_view.push_event(EventDirection::Received, event_type, summary_json);
        }
    }

    /// Schedule the workspace creation process for a repository.
    fn start_workspace_creation(&mut self, repo_id: uuid::Uuid) -> Vec<Effect> {
        let Some(repo_dao) = self.repo_dao() else {
            return Vec::new();
        };

        let Ok(Some(repo)) = repo_dao.get_by_id(repo_id) else {
            tracing::error!(repo_id = %repo_id, "Repository not found");
            return Vec::new();
        };

        if repo.workspace_mode.is_none() {
            let description = format!(
                "Choose how Conduit should create workspaces for \"{}\".\n\nYou can change this later when no active workspaces exist.",
                repo.name
            );
            self.state.close_overlays();
            self.state.confirmation_dialog_state.show(
                "Select Workspace Mode",
                description,
                Vec::new(),
                ConfirmationType::Info,
                "Use Worktrees",
                Some(ConfirmationContext::SelectWorkspaceMode { repo_id }),
            );
            self.state.confirmation_dialog_state.cancel_text = "Use Checkouts".to_string();
            if self.config().workspaces.default_mode == WorkspaceMode::Worktree {
                self.state.confirmation_dialog_state.select_confirm();
            } else {
                self.state.confirmation_dialog_state.select_cancel();
            }
            self.state.input_mode = InputMode::Confirming;
            return Vec::new();
        }

        self.mark_repo_action_busy(repo_id);
        // Show the dedicated remote-sync dialog. The issue picker remains
        // hidden until RemoteSynced fires and the issue-fetch phase begins.
        self.state.remote_sync_dialog_state.show();
        self.state.input_mode = InputMode::SyncingRemote;
        self.state.workspace_creation = Some(
            crate::workspace_creation::WorkspaceCreationSession::new(repo_id),
        );
        self.dispatch_workspace_creation_event(
            crate::workspace_creation::WorkspaceCreationEvent::Start,
        )
    }

    /// Drive the workspace-creation state machine and translate its abstract
    /// commands into concrete effects + UI mutations.
    pub(crate) fn dispatch_workspace_creation_event(
        &mut self,
        event: crate::workspace_creation::WorkspaceCreationEvent,
    ) -> Vec<Effect> {
        use crate::workspace_creation::{
            transition, WorkspaceCreationCommand as Cmd, WorkspaceCreationPhase as Phase,
        };

        let Some((repo_id, phase)) = self
            .state
            .workspace_creation
            .as_ref()
            .map(|s| (s.repo_id, s.phase))
        else {
            tracing::debug!(
                ?event,
                "workspace creation event ignored — no active session"
            );
            return Vec::new();
        };

        let (next_phase, commands) = transition(phase, event);
        if let Some(session) = self.state.workspace_creation.as_mut() {
            session.phase = next_phase;
        }

        let mut effects = Vec::new();
        for cmd in commands {
            match cmd {
                Cmd::SyncRemote => effects.push(Effect::SyncRemote { repo_id }),
                Cmd::FetchRemoteIssues => {
                    effects.push(Effect::FetchRemoteIssues { repo_id });
                }
                Cmd::FetchAllSpecs => {
                    let issue = self
                        .state
                        .workspace_creation
                        .as_ref()
                        .and_then(|s| s.picked_issue.clone());
                    self.state.spec_picker_state =
                        crate::components::SpecPickerState::show_loading(repo_id, issue.clone());
                    self.state.spec_picker_state.show(issue.clone());
                    self.state.specify_picker_state =
                        crate::components::SpecifyPickerState::show_loading(repo_id, issue);
                    self.state.input_mode = InputMode::SelectingSpec;
                    effects.push(Effect::FetchAllSpecs { repo_id });
                }
                Cmd::ShowIssuePicker => {
                    self.state.input_mode = InputMode::SelectingIssue;
                }
                Cmd::ShowSpecPicker => {
                    let issue = self
                        .state
                        .workspace_creation
                        .as_ref()
                        .and_then(|s| s.picked_issue.clone());
                    if !self.state.specify_picker_state.specs.is_empty() {
                        self.state.spec_picker_state.hide();
                        self.state.specify_picker_state.show(issue);
                        self.state.input_mode = InputMode::SelectingSpecifySpec;
                    } else if !self.state.spec_picker_state.specs.is_empty() {
                        self.state.specify_picker_state.hide();
                        self.state.spec_picker_state.show(issue);
                        self.state.input_mode = InputMode::SelectingSpec;
                    }
                }
                Cmd::StartNaming => {
                    if let Some(session) = self.state.workspace_creation.take() {
                        debug_assert_eq!(session.phase, Phase::Naming);
                        // Tear down any picker UI that the loading-spinner phase
                        // left visible (e.g. when SpecsFetched arrives with no
                        // specs, the spec picker dialog was already shown for
                        // its "Fetching..." state and would otherwise linger as
                        // "No open specs found." with no way to dismiss it).
                        self.state.issue_picker_state.hide();
                        self.state.spec_picker_state.hide();
                        self.state.specify_picker_state.hide();
                        self.state.input_mode = InputMode::SidebarNavigation;
                        effects.push(Effect::CreateWorkspace {
                            repo_id: session.repo_id,
                            issue: session.picked_issue,
                            spec: session.picked_spec,
                            specify_spec: session.picked_specify_spec,
                        });
                    }
                }
            }
        }
        effects
    }

    /// Find the visible index of a workspace by its ID
    fn find_workspace_index(&self, workspace_id: uuid::Uuid) -> Option<usize> {
        use crate::components::NodeType;
        self.state
            .sidebar_data
            .visible_nodes()
            .iter()
            .position(|node| node.id == workspace_id && node.node_type == NodeType::Workspace)
    }

    /// Sync sidebar selection to the active tab's workspace (if sidebar is visible)
    fn sync_sidebar_to_active_tab(&mut self) {
        if let Some(session) = self.state.tab_manager.active_session() {
            if let Some(workspace_id) = session.workspace_id {
                if let Some(index) = self.state.sidebar_data.focus_workspace(workspace_id) {
                    self.state.sidebar_state.tree_state.selected = index;
                }
            }
        }
    }

    /// Keep the non-modal input mode aligned with the currently active tab type.
    fn sync_input_mode_for_active_tab(&mut self) {
        match self.state.input_mode {
            InputMode::Normal | InputMode::Scrolling | InputMode::FileViewer => {
                if self.state.tab_manager.active_is_file() {
                    self.state.input_mode = InputMode::FileViewer;
                } else if self.state.input_mode == InputMode::FileViewer {
                    self.state.input_mode = InputMode::Normal;
                }
            }
            _ => {}
        }
    }

    /// Sync footer spinner state to the active tab's processing state
    fn sync_footer_spinner(&mut self) {
        let active_session = self.state.tab_manager.active_session();
        let is_active_processing = active_session.is_some_and(|s| s.is_processing);
        let has_inline_prompt = active_session.is_some_and(|s| s.inline_prompt.is_some());

        // Don't show spinner when awaiting user response (inline prompt active)
        if is_active_processing && !has_inline_prompt {
            // Start spinner if active tab is processing and spinner not already running
            if self.state.footer_spinner.is_none() {
                self.state.start_footer_spinner(None);
            }
        } else if self.state.footer_spinner.is_some() {
            // Stop spinner if not processing, or awaiting response
            self.state.stop_footer_spinner();
        }
    }

    /// Apply the active tab's project theme, or fall back to the global config theme.
    pub(super) fn sync_theme_to_active_tab(&self) {
        let project_theme = self
            .state
            .tab_manager
            .active_session()
            .and_then(|s| s.project_theme.as_deref());

        if let Some(name) = project_theme {
            if crate::components::current_theme_name() == name {
                return;
            }
            if !crate::components::load_theme_by_name(name) {
                tracing::warn!(theme = %name, "Failed to apply project theme; keeping current theme");
            }
        } else {
            let desired = self.config().theme_name.as_deref().unwrap_or("Night Owl");
            if self.config().theme_path.is_none()
                && crate::components::current_theme_name() == desired
            {
                return;
            }
            crate::components::init_theme(
                self.config().theme_name.as_deref(),
                self.config().theme_path.as_deref(),
            );
        }
    }

    /// Apply the project theme for whichever repository is currently highlighted in the sidebar,
    /// falling back to the global config theme when no project theme is set.
    pub(super) fn sync_theme_to_sidebar_selection(&self) {
        use crate::components::{ActionType, NodeType};

        let selected = self.state.sidebar_state.tree_state.selected;
        let theme_name =
            self.state
                .sidebar_data
                .get_at(selected)
                .and_then(|node| match node.node_type {
                    NodeType::Repository => node.theme_name.as_deref(),
                    NodeType::Workspace | NodeType::Action(ActionType::NewWorkspace) => node
                        .parent_id
                        .and_then(|id| self.state.sidebar_data.get_repo_theme(id))
                        .map(String::as_str),
                });

        if let Some(name) = theme_name {
            if crate::components::current_theme_name() == name {
                return;
            }
            if !crate::components::load_theme_by_name(name) {
                tracing::warn!(theme = %name, "Failed to apply sidebar selection theme");
            }
        } else {
            let desired = self.config().theme_name.as_deref().unwrap_or("Night Owl");
            if self.config().theme_path.is_none()
                && crate::components::current_theme_name() == desired
            {
                return;
            }
            crate::components::init_theme(
                self.config().theme_name.as_deref(),
                self.config().theme_path.as_deref(),
            );
        }
    }

    /// Dismiss the confirmation dialog and clean up fork state if applicable.
    /// Returns the input mode to transition to.
    fn dismiss_confirmation_dialog(&mut self) -> InputMode {
        // Cache context before hide() clears it
        let ctx = self.state.confirmation_dialog_state.context.clone();

        // Clear pending fork request if dismissing a fork confirmation
        if matches!(
            &ctx,
            Some(ConfirmationContext::ForkSession { .. })
                | Some(ConfirmationContext::ForkSessionPreflightInProgress { .. })
        ) {
            self.state.pending_fork_request = None;
        }

        self.state.confirmation_dialog_state.hide();

        // Return appropriate input mode based on context
        match ctx {
            // PR/Fork/Steer dialogs originated from chat view, return to Normal
            Some(ConfirmationContext::CreatePullRequest { .. })
            | Some(ConfirmationContext::OpenExistingPr { .. })
            | Some(ConfirmationContext::ForkSession { .. })
            | Some(ConfirmationContext::ForkSessionPreflightInProgress { .. })
            | Some(ConfirmationContext::SteerFallback { .. }) => InputMode::Normal,
            Some(ConfirmationContext::RemoveProject(_))
            | Some(ConfirmationContext::RemoveProjectPreflightInProgress { .. })
            | Some(ConfirmationContext::SelectWorkspaceMode { .. }) => InputMode::SidebarNavigation,
            Some(ConfirmationContext::Quit) => InputMode::Normal,
            // No context: return to Normal if tabs exist, otherwise SidebarNavigation
            // (avoids unexpectedly flipping to sidebar when user has active tabs)
            None => {
                if !self.state.tab_manager.is_empty() {
                    InputMode::Normal
                } else {
                    InputMode::SidebarNavigation
                }
            }
        }
    }

    fn is_blocking_confirmation_loading_dialog(&self) -> bool {
        self.state.confirmation_dialog_state.visible
            && self.state.confirmation_dialog_state.loading
            && self
                .state
                .confirmation_dialog_state
                .context
                .as_ref()
                .is_some_and(ConfirmationContext::is_blocking_loading)
    }

    fn show_blocking_confirmation_loading(
        &mut self,
        title: impl Into<String>,
        loading_message: impl Into<String>,
        context: ConfirmationContext,
    ) {
        self.state.close_overlays();
        self.state
            .confirmation_dialog_state
            .show_loading_with_context(title, loading_message, Some(context));
        self.state.input_mode = InputMode::Confirming;
    }

    fn spawn_blocking_preflight<T, W, E>(&self, work: W, event_builder: E, context: &'static str)
    where
        T: Send + 'static,
        W: FnOnce() -> Result<T, String> + Send + 'static,
        E: FnOnce(Result<T, String>) -> AppEvent + Send + 'static,
    {
        let event_tx = self.event_tx.clone();
        tokio::task::spawn_blocking(move || {
            let result = work();
            send_app_event(&event_tx, event_builder(result), context);
        });
    }

    /// Open the Work Complete dialog for `workspace_id`.
    pub(crate) fn initiate_work_complete(&mut self, workspace_id: uuid::Uuid) {
        if self.state.work_complete_session.is_some() {
            return;
        }

        let session = crate::work_complete::WorkCompleteSession::new(workspace_id);
        self.state.work_complete_session = Some(session);
        self.state.input_mode = InputMode::WorkCompleting;

        self.spawn_work_complete_preflight(workspace_id);
    }

    fn spawn_work_complete_preflight(&mut self, workspace_id: uuid::Uuid) {
        let workspace_dao = self.workspace_dao_clone();
        let repo_dao = self.repo_dao_clone();
        let worktree_manager = self.worktree_manager().clone();
        let config = self.config().clone();

        self.spawn_blocking_preflight(
            move || {
                run_work_complete_preflight(
                    workspace_id,
                    workspace_dao,
                    repo_dao,
                    worktree_manager,
                    &config,
                )
            },
            move |result| AppEvent::WorkCompletePreflightLoaded {
                workspace_id,
                result,
            },
            "work_complete_preflight",
        );
    }

    /// Dispatch a `WorkCompleteEvent` through the state machine and handle resulting commands.
    pub(crate) fn dispatch_work_complete_event(
        &mut self,
        event: crate::work_complete::WorkCompleteEvent,
    ) -> Vec<Effect> {
        use crate::work_complete::{WorkCompleteCommand as C, WorkCompleteEvent as E};

        let Some(session) = self.state.work_complete_session.as_ref() else {
            return vec![];
        };

        let workspace_id = session.workspace_id;
        let action_from_event = if let E::ActionSelected(a) = &event {
            Some(*a)
        } else {
            None
        };
        let (next_phase, commands) =
            crate::work_complete::transition(&session.phase, event.clone());

        if let Some(session) = self.state.work_complete_session.as_mut() {
            session.phase = next_phase;
            if let Some(action) = action_from_event {
                if commands.iter().any(|c| matches!(c, C::SendAgentPrompt(_))) {
                    session.pending_agent_action = Some(action);
                }
            }
        }

        let mut effects = vec![];
        for cmd in commands {
            match cmd {
                C::FetchPreflight | C::RefreshPreflight => {
                    effects.push(Effect::WorkCompletePreflight { workspace_id });
                }
                C::RequestCommitMessage { suggestion: _ } => {
                    // Pre-fill from session data
                    let suggestion = self
                        .state
                        .work_complete_session
                        .as_ref()
                        .and_then(|s| s.data.as_ref())
                        .map(|d| {
                            crate::work_complete::suggest_commit_message(
                                &d.branch_name,
                                &d.dirty_files,
                                d.spec.as_ref().map(|s| s.change_id.as_str()),
                                d.issue.as_ref().map(|i| i.number),
                            )
                        })
                        .unwrap_or_default();
                    if let Some(session) = self.state.work_complete_session.as_mut() {
                        session.commit_message_input = suggestion;
                    }
                }
                C::ExecuteAction(action) => {
                    effects.push(Effect::WorkCompleteAction {
                        workspace_id,
                        action,
                        payload: None,
                    });
                }
                C::ExecuteCommit(msg) => {
                    effects.push(Effect::WorkCompleteAction {
                        workspace_id,
                        action: conduit_git::SuggestedAction::Commit,
                        payload: Some(msg),
                    });
                }
                C::SendAgentPrompt(_) => {
                    let pending_action = self
                        .state
                        .work_complete_session
                        .as_ref()
                        .and_then(|s| s.pending_agent_action);
                    if pending_action == Some(conduit_git::SuggestedAction::AdversarialReview) {
                        let prompt = build_adversarial_review_prompt();
                        if let Ok(mut e) = self.submit_prompt(prompt, vec![], vec![]) {
                            effects.append(&mut e);
                        }
                    } else if let Some(change_id) = self
                        .state
                        .work_complete_session
                        .as_ref()
                        .and_then(|s| s.data.as_ref())
                        .and_then(|d| d.spec.as_ref())
                        .map(|s| s.change_id.clone())
                    {
                        let prompt = format!("show incomplete tasks in {}", change_id);
                        if let Ok(mut e) = self.submit_prompt(prompt, vec![], vec![]) {
                            effects.append(&mut e);
                        }
                    }
                }
                C::MonitorCi { pr_url } => {
                    effects.push(Effect::WorkCompleteCiMonitor {
                        workspace_id,
                        pr_url,
                    });
                }
                C::Close => {
                    self.close_work_complete_dialog();
                    return effects;
                }
            }
        }

        // Check if transition moved to Done
        if matches!(
            self.state.work_complete_session.as_ref().map(|s| &s.phase),
            Some(crate::work_complete::WorkCompletePhase::Done)
        ) {
            self.close_work_complete_dialog();
        }

        // Special: if we get PreflightLoaded from an E::ActionCompleted, store data
        if let E::ActionCompleted(_) = event {
            // Phase was set to LoadingPreflight by transition; preflight effect already queued
        }

        effects
    }

    fn close_work_complete_dialog(&mut self) {
        self.state.work_complete_session = None;
        self.state.input_mode = InputMode::Normal;
    }

    /// Close the workspace creation progress dialog and open the created workspace (if successful).
    pub(crate) fn close_workspace_progress_dialog(&mut self) -> Vec<Effect> {
        // Extract config choices before hiding (hide() drops the config).
        let session_config = self
            .state
            .workspace_progress_dialog_state
            .config
            .as_ref()
            .map(|cfg| {
                (
                    PendingSessionConfig {
                        provider: cfg.provider,
                        model_id: cfg.model_id.clone(),
                        mode: cfg.mode,
                        orchestration_enabled: cfg.orchestration_enabled,
                        adversarial_review_enabled: cfg.adversarial_review_enabled,
                        adversarial_review_model: Some(cfg.adversarial_review_model.clone()),
                    },
                    cfg.save_as_project_default,
                )
            });

        self.state.workspace_progress_dialog_state.hide();
        self.state.input_mode = InputMode::Normal;
        let mut effects = Vec::new();

        if let Some(workspace_id) = self.state.pending_created_workspace_id.take() {
            // Save project defaults if requested.
            if let Some((ref cfg, true)) = session_config {
                if let Some(repo_dao) = self.repo_dao_clone() {
                    let repo_id = self
                        .workspace_dao()
                        .and_then(|dao| dao.get_by_id(workspace_id).ok().flatten())
                        .map(|ws| ws.repository_id);
                    if let Some(repo_id) = repo_id {
                        if let Ok(Some(mut repo)) = repo_dao.get_by_id(repo_id) {
                            repo.default_provider = Some(cfg.provider.as_str().to_string());
                            repo.default_model = Some(cfg.model_id.clone());
                            repo.orchestration_enabled = Some(cfg.orchestration_enabled);
                            repo.adversarial_review_enabled = Some(cfg.adversarial_review_enabled);
                            repo.adversarial_review_model = cfg.adversarial_review_model.clone();
                            if let Err(err) = repo_dao.update(&repo) {
                                tracing::warn!("Failed to save project defaults: {err}");
                            }
                        }
                    }
                }
            }

            // Open workspace, close sidebar (unless always_show_sidebar), focus prompt
            let close_sidebar = !self.config().ui.always_show_sidebar;
            self.open_workspace_with_options(workspace_id, close_sidebar);
            self.state.sidebar_state.set_focused(false);

            // Apply chosen session config to the newly opened tab.
            if let Some((cfg, _)) = session_config {
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    session.agent_type = cfg.provider;
                    session.model = Some(cfg.model_id);
                    session.agent_mode = cfg.mode;
                    session.orchestration_enabled = cfg.orchestration_enabled;
                    session.adversarial_review_enabled = cfg.adversarial_review_enabled;
                    session.adversarial_review_model = cfg.adversarial_review_model.clone();
                    session.update_status();
                }
            }

            if let Some(msg) = self.state.pending_created_workspace_initial_message.take() {
                match self.submit_prompt(msg, vec![], vec![]) {
                    Ok(mut e) => effects.append(&mut e),
                    Err(err) => tracing::warn!("Failed to auto-send spec context message: {err}"),
                }
            }
        }

        effects
    }

    pub(crate) fn open_workspace_ready_provider_selector(&mut self) {
        self.redetect_tools();
        let providers = self.config().effective_enabled_providers(self.tools());
        let items: Vec<(String, String)> = providers
            .iter()
            .map(|p| (p.as_str().to_string(), p.display_name().to_string()))
            .collect();
        self.state
            .workspace_progress_dialog_state
            .open_provider_picker(items);
    }

    pub(crate) fn open_workspace_ready_model_selector(&mut self) {
        let provider = self
            .state
            .workspace_progress_dialog_state
            .config
            .as_ref()
            .map(|c| c.provider)
            .unwrap_or_else(|| {
                self.preferred_provider_for_new_sessions()
                    .unwrap_or(AgentType::Claude)
            });
        let models = conduit_agent::ModelRegistry::models_for(provider);
        let items: Vec<(String, String)> = models
            .iter()
            .map(|m| (m.id.clone(), m.display_name.clone()))
            .collect();
        self.state
            .workspace_progress_dialog_state
            .open_model_picker(items);
    }

    pub(crate) fn open_workspace_ready_adversarial_model_selector(&mut self) {
        let provider = self
            .state
            .workspace_progress_dialog_state
            .config
            .as_ref()
            .map(|c| c.provider)
            .unwrap_or_else(|| {
                self.preferred_provider_for_new_sessions()
                    .unwrap_or(AgentType::Claude)
            });
        let current_model = self
            .state
            .workspace_progress_dialog_state
            .config
            .as_ref()
            .map(|c| c.adversarial_review_model.clone());
        let defaults = DefaultModelSelection {
            agent_type: Some(provider),
            model_id: current_model.clone(),
        };
        self.state
            .model_selector_state
            .set_allowed_providers(Some(vec![provider]));
        self.state.model_selector_state.show_with_title(
            current_model,
            defaults,
            "Select Adversarial Review Model".to_string(),
        );
        self.state.model_picker_context = ModelPickerContext::WorkspaceReadyAdversarialConfig;
        self.state.input_mode = InputMode::SelectingModel;
    }

    /// Show an error dialog with a simple message
    fn show_error(&mut self, title: &str, message: &str) {
        self.state.close_overlays();
        self.state.error_dialog_state.show(title, message);
        self.state.input_mode = InputMode::ShowingError;
    }

    /// Show an error dialog with technical details
    fn show_error_with_details(&mut self, title: &str, message: &str, details: &str) {
        self.state.close_overlays();
        self.state
            .error_dialog_state
            .show_with_details(title, message, details);
        self.state.input_mode = InputMode::ShowingError;
    }

    fn apply_repo_workspace_mode(
        &mut self,
        repo_id: uuid::Uuid,
        mode: WorkspaceMode,
    ) -> Result<(), String> {
        let repo_dao = self
            .repo_dao()
            .ok_or_else(|| "Repository database unavailable".to_string())?;
        let workspace_dao = self
            .workspace_dao()
            .ok_or_else(|| "Workspace database unavailable".to_string())?;

        let repo = repo_dao
            .get_by_id(repo_id)
            .map_err(|e| format!("Failed to load repository: {}", e))?
            .ok_or_else(|| "Repository not found".to_string())?;

        if let Some(existing_mode) = repo.workspace_mode {
            if existing_mode == mode {
                return Ok(());
            }
        }

        let active_count = workspace_dao
            .count_active_by_repository(repo_id)
            .map_err(|e| format!("Failed to check workspaces: {}", e))?;
        if active_count > 0 {
            return Err("Cannot change workspace mode while active workspaces exist.".to_string());
        }

        repo_dao
            .update_settings(
                repo_id,
                Some(mode),
                repo.archive_delete_branch,
                repo.archive_remote_prompt,
            )
            .map_err(|e| format!("Failed to update repository settings: {}", e))?;

        Ok(())
    }

    /// Initiate project removal - shows confirmation dialog
    fn initiate_remove_project(&mut self, repo_id: uuid::Uuid) {
        self.show_blocking_confirmation_loading(
            "Remove Project",
            "Analyzing project workspaces...",
            ConfirmationContext::RemoveProjectPreflightInProgress { repo_id },
        );

        let repo_dao = self.repo_dao_clone();
        let workspace_dao = self.workspace_dao_clone();
        let worktree_manager = self.worktree_manager().clone();
        let config = self.config().clone();

        self.spawn_blocking_preflight(
            move || {
                let repo_dao =
                    repo_dao.ok_or_else(|| "Repository database unavailable".to_string())?;
                let workspace_dao =
                    workspace_dao.ok_or_else(|| "Workspace database unavailable".to_string())?;

                let repo = repo_dao
                    .get_by_id(repo_id)
                    .map_err(|e| format!("Failed to load repository: {}", e))?
                    .ok_or_else(|| "Repository not found".to_string())?;

                let workspaces = workspace_dao
                    .get_by_repository(repo_id)
                    .map_err(|e| format!("Failed to load workspaces: {}", e))?;

                let mut warnings = Vec::new();
                let mut has_dirty = false;
                let mut has_unmerged = false;

                for workspace in &workspaces {
                    if let Ok(status) = worktree_manager.get_branch_status_with_gh_option(
                        &workspace.path,
                        config.workspaces.use_gh_cli_merge_status,
                    ) {
                        if status.is_dirty {
                            has_dirty = true;
                        }
                        if !status.is_merged {
                            has_unmerged = true;
                        }
                    }
                }

                let workspace_count = workspaces.len();
                if workspace_count > 0 {
                    warnings.push(format!(
                        "{} workspace{} will be archived",
                        workspace_count,
                        if workspace_count == 1 { "" } else { "s" }
                    ));
                }
                if has_dirty {
                    warnings.push("Some workspaces have uncommitted changes".to_string());
                }
                if has_unmerged {
                    warnings.push("Some branches are not merged to main".to_string());
                }

                Ok(RemoveProjectDialogPreflightResult {
                    repo_name: repo.name,
                    warnings,
                    has_dirty,
                    has_unmerged,
                    workspace_count,
                })
            },
            move |result| AppEvent::RemoveProjectDialogPreflightCompleted { repo_id, result },
            "remove_project_dialog_preflight_completed",
        );
    }

    /// Execute project removal after confirmation
    fn execute_remove_project(&mut self, repo_id: uuid::Uuid) -> Effect {
        // Set spinner mode
        self.state.input_mode = InputMode::RemovingProject;
        self.mark_repo_busy(repo_id);

        Effect::RemoveProject { repo_id }
    }

    fn close_tab_at_index(&mut self, index: usize) {
        if let Some(session) = self.state.tab_manager.session(index) {
            if let Some(session_tab_dao) = self.session_tab_dao_clone() {
                if let Err(e) = session_tab_dao.set_open(session.id, false) {
                    tracing::warn!(error = %e, "Failed to mark session as closed");
                }
            }
        }
        self.state.tab_manager.close_tab(index);
    }

    /// Close any tabs that are using the specified workspace
    fn close_tabs_for_workspace(&mut self, workspace_id: uuid::Uuid) {
        // Unregister workspace from git tracker
        if let Some(ref tracker) = self.git_tracker {
            tracker.untrack_workspace(workspace_id);
        }

        // Find tabs with this workspace and close them (in reverse order to maintain indices)
        let indices_to_close: Vec<usize> = self
            .state
            .tab_manager
            .sessions()
            .iter()
            .enumerate()
            .filter_map(|(idx, session)| {
                if session.workspace_id == Some(workspace_id) {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();

        for idx in indices_to_close.into_iter().rev() {
            self.stop_agent_for_tab(idx);
            self.close_tab_at_index(idx);
        }

        // Switch to sidebar navigation if all tabs are closed
        // But don't override if we're showing an error dialog
        if self.state.tab_manager.is_empty() && self.state.input_mode != InputMode::ShowingError {
            self.state.sidebar_state.visible = true;
            self.state.input_mode = InputMode::SidebarNavigation;
        } else {
            self.sync_theme_to_active_tab();
            self.sync_sidebar_to_active_tab();
        }
    }

    /// Add a project to the sidebar (repository only, no workspace)
    /// Returns the repository ID - either existing or newly created
    fn add_project_to_sidebar(&mut self, path: std::path::PathBuf) -> Option<uuid::Uuid> {
        let repo_dao = self.repo_dao()?;

        // Check if project already exists
        if let Ok(Some(existing_repo)) = repo_dao.get_by_path(&path) {
            // Project already exists, just return its ID (caller will expand/select it)
            return Some(existing_repo.id);
        }

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        // Create repository (without default workspace)
        let repo = Repository::from_local_path(&name, path);
        if repo_dao.create(&repo).is_err() {
            return None;
        }

        let repo_id = repo.id;

        // Refresh sidebar
        self.refresh_sidebar_data();

        Some(repo_id)
    }

    /// Add a repository from the custom path dialog
    /// Returns the repository ID - either existing or newly created
    fn add_repository(&mut self) -> Option<uuid::Uuid> {
        let path = self.state.add_repo_dialog_state.expanded_path();

        let repo_dao = self.repo_dao()?;

        // Check if project already exists
        if let Ok(Some(existing_repo)) = repo_dao.get_by_path(&path) {
            // Project already exists, just return its ID (caller will expand/select it)
            return Some(existing_repo.id);
        }

        let name = self
            .state
            .add_repo_dialog_state
            .repo_name
            .clone()
            .unwrap_or_else(|| "Unknown".to_string());

        // Create repository (without default workspace)
        let repo = Repository::from_local_path(&name, path);
        if repo_dao.create(&repo).is_err() {
            return None;
        }

        let repo_id = repo.id;

        // Refresh sidebar
        self.refresh_sidebar_data();

        Some(repo_id)
    }

    /// Return the projects base directory as a PathBuf, defaulting to `~/code`.
    fn projects_base_dir_path(&self) -> std::path::PathBuf {
        let base_dir_str = self
            .app_state_dao()
            .and_then(|dao| dao.get("projects_base_dir").ok().flatten())
            .unwrap_or_else(|| "~/code".to_string());

        if base_dir_str.starts_with('~') {
            dirs::home_dir()
                .map(|h| h.join(base_dir_str[1..].trim_start_matches('/')))
                .unwrap_or_else(|| std::path::PathBuf::from(&base_dir_str))
        } else {
            std::path::PathBuf::from(&base_dir_str)
        }
    }

    /// Clone a remote git repository and add it as a project.
    ///
    /// The clone is performed on a background thread so the TUI stays responsive.
    /// The target directory is `<projects_base_dir>/<repo_name>`.
    fn clone_repository(&mut self) {
        use crate::components::RepoInputKind;

        let (url, repo_name) = match &self.state.add_repo_dialog_state.input_kind {
            RepoInputKind::GitUrl { url } => (
                url.clone(),
                self.state
                    .add_repo_dialog_state
                    .repo_name
                    .clone()
                    .unwrap_or_else(|| "repo".to_string()),
            ),
            RepoInputKind::LocalPath => return,
        };

        let target_path = self.projects_base_dir_path().join(&repo_name);

        if target_path.exists() {
            self.state
                .add_repo_dialog_state
                .path
                .set_error(format!("Directory '{}' already exists", repo_name));
            return;
        }

        self.state.add_repo_dialog_state.hide();
        self.state.input_mode = InputMode::CloningRepository;
        self.state
            .set_timed_footer_message(format!("Cloning {}…", repo_name), Duration::from_secs(300));

        self.spawn_blocking_preflight(
            move || {
                let output = std::process::Command::new("git")
                    .args(["clone", &url, target_path.to_str().unwrap_or("")])
                    .output()
                    .map_err(|e| format!("Failed to run git: {e}"))?;

                if output.status.success() {
                    Ok(target_path)
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(format!(
                        "git clone failed: {}",
                        stderr.trim().lines().last().unwrap_or("unknown error")
                    ))
                }
            },
            |result| AppEvent::RepositoryCloned { result },
            "repository_cloned",
        );
    }

    /// Create a new tab with the selected agent type
    fn create_tab_with_agent(&mut self, agent_type: AgentType) {
        let target_provider = if self
            .config()
            .is_provider_enabled_effective(agent_type, self.tools())
        {
            agent_type
        } else {
            self.preferred_provider_for_new_sessions()
                .unwrap_or(agent_type)
        };

        self.state.tab_manager.new_tab(target_provider);
        let model_id = self.config().default_model_for(target_provider);
        let orchestration_default = self.config().orchestration.enabled_by_default;
        if let Some(session) = self.state.tab_manager.active_session_mut() {
            session.model = Some(model_id);
            session.model_invalid = false;
            session.init_context_for_model();
            if session.agent_type == AgentType::Claude {
                session.orchestration_enabled = orchestration_default;
            }
            session.update_status();
        }
        self.state.input_mode = InputMode::Normal;
    }

    /// Replace the active session with a fresh one (same workspace, reset history).
    fn start_new_session_in_place(&mut self) {
        if self.state.tab_manager.is_empty() {
            self.state.set_timed_footer_message(
                "No active session to reset".to_string(),
                Duration::from_secs(3),
            );
            return;
        }

        let active_index = self.state.tab_manager.active_index();
        let (
            agent_type,
            working_dir,
            workspace_id,
            repository_id,
            project_name,
            workspace_name,
            branch_name,
            project_theme,
            pr_number,
            is_processing,
        ) = match self.state.tab_manager.session(active_index) {
            Some(session) => (
                session.agent_type,
                session.working_dir.clone(),
                session.workspace_id,
                session.repository_id,
                session.project_name.clone(),
                session.workspace_name.clone(),
                session.branch_name.clone(),
                session.project_theme.clone(),
                session.pr_number,
                session.is_processing,
            ),
            None => {
                self.state.set_timed_footer_message(
                    "No active session to reset".to_string(),
                    Duration::from_secs(3),
                );
                return;
            }
        };

        if is_processing {
            self.state.set_timed_footer_message(
                "Stop the agent before starting a new session".to_string(),
                Duration::from_secs(3),
            );
            return;
        }

        let mut new_session = if let Some(dir) = working_dir {
            AgentSession::with_working_dir(agent_type, dir)
        } else {
            AgentSession::new(agent_type)
        };
        new_session.workspace_id = workspace_id;
        new_session.repository_id = repository_id;
        new_session.project_name = project_name;
        new_session.workspace_name = workspace_name;
        new_session.branch_name = branch_name;
        new_session.project_theme = project_theme;
        new_session.pr_number = pr_number;
        new_session.model = Some(self.config().default_model_for(agent_type));
        new_session.model_invalid = false;
        new_session.init_context_for_model();
        new_session.update_status();

        if let Some(session) = self.state.tab_manager.session_mut(active_index) {
            *session = new_session;
        }

        self.state
            .set_timed_footer_message("Started a new session".to_string(), Duration::from_secs(3));
    }

    /// Create a new tab by importing an external session
    async fn create_imported_session_tab(
        &mut self,
        agent_type: AgentType,
        session_file: std::path::PathBuf,
        working_dir: std::path::PathBuf,
    ) -> anyhow::Result<()> {
        // Extract session ID from the file path
        let session_id_str = session_file
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Create a new session with working directory
        let mut session = AgentSession::with_working_dir(agent_type, working_dir);
        // Set both resume and agent session IDs so the session can be restored after restart
        let session_id = SessionId::from_string(&session_id_str);
        session.resume_session_id = Some(session_id.clone());
        if agent_type != AgentType::Codex {
            session.agent_session_id = Some(session_id);
        }

        // Load history based on agent type
        match agent_type {
            AgentType::Claude => {
                if let Ok((msgs, debug_entries, file_path)) =
                    load_claude_history_with_debug(&session_id_str)
                {
                    Self::populate_debug_from_history(
                        &mut session.raw_events_view,
                        &debug_entries,
                        &file_path,
                    );
                    for msg in msgs {
                        session.chat_view.push(msg);
                    }
                }
            }
            AgentType::Codex => {
                if let Ok((msgs, debug_entries, file_path)) =
                    load_codex_history_with_debug(&session_id_str)
                {
                    Self::populate_debug_from_history(
                        &mut session.raw_events_view,
                        &debug_entries,
                        &file_path,
                    );
                    for msg in msgs {
                        session.chat_view.push(msg);
                    }
                }
            }
            AgentType::Dirac => {
                session.resume_session_id = None;
                session.agent_session_id = None;
                session.chat_view.push(
                    MessageDisplay::System {
                        content: "Dirac session import isn't supported yet.".to_string(),
                    }
                    .to_chat_message(),
                );
            }
            AgentType::Gemini => {
                session.resume_session_id = None;
                session.agent_session_id = None;
                session.chat_view.push(
                    MessageDisplay::System {
                        content: "Gemini CLI session import isn't supported yet.".to_string(),
                    }
                    .to_chat_message(),
                );
            }
            AgentType::DeepseekTui => {
                session.resume_session_id = None;
                session.agent_session_id = None;
                session.chat_view.push(
                    MessageDisplay::System {
                        content: "DeepSeek TUI session import isn't supported yet.".to_string(),
                    }
                    .to_chat_message(),
                );
            }
            AgentType::Opencode => {
                if let Ok((msgs, debug_entries, file_path)) =
                    load_opencode_history_with_debug(&session_id_str)
                {
                    Self::populate_debug_from_history(
                        &mut session.raw_events_view,
                        &debug_entries,
                        &file_path,
                    );
                    for msg in msgs {
                        session.chat_view.push(msg);
                    }
                }
            }
            AgentType::Copilot => {
                session.resume_session_id = None;
                session.agent_session_id = None;
                session.chat_view.push(
                    MessageDisplay::System {
                        content: "GitHub Copilot session import isn't supported yet.".to_string(),
                    }
                    .to_chat_message(),
                );
            }
            AgentType::Pi => {
                if let Ok((msgs, debug_entries, file_path)) =
                    load_pi_history_with_debug(&session_id_str)
                {
                    Self::populate_debug_from_history(
                        &mut session.raw_events_view,
                        &debug_entries,
                        &file_path,
                    );
                    for msg in msgs {
                        session.chat_view.push(msg);
                    }
                }
            }
        }

        session.update_status();

        // Add the session to the tab manager
        self.state.tab_manager.add_session(session);

        // Switch to the new tab
        let tab_count = self.state.tab_manager.sessions().len();
        self.state
            .tab_manager
            .switch_to(tab_count.saturating_sub(1));
        self.sync_footer_spinner();
        self.sync_theme_to_active_tab();

        Ok(())
    }

    async fn handle_app_event(&mut self, event: AppEvent) -> anyhow::Result<Vec<Effect>> {
        let mut effects = Vec::new();
        match event {
            AppEvent::Agent { session_id, event } => {
                self.handle_agent_event(session_id, event).await?;
            }
            AppEvent::Quit => {
                self.state.should_quit = true;
                effects.push(Effect::SaveSessionState);
            }
            AppEvent::Error(msg) => {
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    let display = MessageDisplay::Error { content: msg };
                    session.chat_view.push(display.to_chat_message());
                    session.stop_processing();
                    self.state.stop_footer_spinner();
                }
            }
            AppEvent::PrPreflightCompleted {
                tab_index,
                working_dir,
                result,
            } => {
                effects.extend(self.handle_pr_preflight_result(tab_index, working_dir, result));
            }
            AppEvent::OpenPrCompleted { result: Err(err) } => {
                self.show_error(
                    "Failed to Open PR",
                    &format!("Could not open PR in browser: {}", err),
                );
            }
            AppEvent::OpenPrCompleted { result: Ok(_) } => {}
            AppEvent::DebugDumped { result } => match result {
                Ok(path) => {
                    self.show_error_with_details(
                        "Debug Export Complete",
                        "Session debug info has been exported.",
                        &format!("File saved to:\n{}", path),
                    );
                }
                Err(err) => {
                    self.show_error("Export Failed", &err);
                }
            },
            AppEvent::WorkspaceCreationProgress { message } => {
                self.state.workspace_progress_dialog_state.push(message);
            }
            AppEvent::RemoteSyncProgress { message }
                if self.state.remote_sync_dialog_state.visible =>
            {
                self.state.remote_sync_dialog_state.push(message);
            }
            AppEvent::RemoteSyncProgress { .. } => {}
            AppEvent::RemoteSynced { repo_id } => {
                self.state.remote_sync_dialog_state.hide();
                self.state.issue_picker_state =
                    crate::components::IssuePickerState::show_loading(repo_id);
                self.state.input_mode = InputMode::SelectingIssue;
                effects.append(&mut self.dispatch_workspace_creation_event(
                    crate::workspace_creation::WorkspaceCreationEvent::RemoteSynced,
                ));
            }
            AppEvent::CurrentUserFetched { repo_id, user }
                if self.state.issue_picker_state.repo_id == repo_id =>
            {
                self.state.issue_picker_state.set_current_user(user);
            }
            AppEvent::CurrentUserFetched { .. } => {}
            AppEvent::RemoteIssuesFetched { repo_id: _, issues } => {
                let has_issues = !issues.is_empty();
                if has_issues {
                    self.state.issue_picker_state.load_issues(issues);
                } else {
                    self.state.issue_picker_state.hide();
                }
                effects.append(&mut self.dispatch_workspace_creation_event(
                    crate::workspace_creation::WorkspaceCreationEvent::IssuesFetched { has_issues },
                ));
            }
            AppEvent::AllSpecsFetched {
                repo_id: _,
                open_specs,
                specify_specs,
                source_ref,
            } => {
                self.state.spec_picker_state.source_ref = source_ref.clone();
                self.state.specify_picker_state.source_ref = source_ref;
                self.state.spec_picker_state.load_specs(open_specs);
                self.state.specify_picker_state.load_specs(specify_specs);
                let has_specs = !self.state.spec_picker_state.specs.is_empty()
                    || !self.state.specify_picker_state.specs.is_empty();
                effects.append(&mut self.dispatch_workspace_creation_event(
                    crate::workspace_creation::WorkspaceCreationEvent::SpecsFetched { has_specs },
                ));
            }
            AppEvent::WorkspaceCreated { repo_id, result } => {
                self.clear_repo_action_busy(repo_id);
                match result {
                    Ok(created) => {
                        self.refresh_sidebar_data();
                        self.state.sidebar_data.expand_repo(created.repo_id);
                        if let Some(index) = self.find_workspace_index(created.workspace_id) {
                            self.state.sidebar_state.tree_state.selected = index;
                        }
                        self.state.pending_created_workspace_id = Some(created.workspace_id);
                        self.state.pending_created_workspace_initial_message =
                            created.initial_message.clone();

                        // Resolve provider/model/orchestration defaults:
                        // workspace override → repo override → global config.
                        let global_provider = self
                            .preferred_provider_for_new_sessions()
                            .unwrap_or(AgentType::Claude);
                        let (
                            resolved_provider,
                            resolved_model,
                            resolved_orch,
                            resolved_ar_enabled,
                            resolved_ar_model,
                        ) = {
                            let workspace = self
                                .workspace_dao()
                                .and_then(|dao| dao.get_by_id(created.workspace_id).ok().flatten());
                            let workspace_orch =
                                workspace.as_ref().and_then(|ws| ws.orchestration_enabled);
                            let workspace_ar_enabled = workspace
                                .as_ref()
                                .and_then(|ws| ws.adversarial_review_enabled);
                            let workspace_ar_model = workspace
                                .as_ref()
                                .and_then(|ws| ws.adversarial_review_model.clone());
                            let repo = self
                                .repo_dao()
                                .and_then(|dao| dao.get_by_id(created.repo_id).ok().flatten());
                            let repo_provider = repo
                                .as_ref()
                                .and_then(|r| r.default_provider.as_deref())
                                .map(AgentType::parse);
                            let repo_model = repo.as_ref().and_then(|r| r.default_model.clone());
                            let repo_orch = repo.as_ref().and_then(|r| r.orchestration_enabled);
                            let repo_ar_enabled =
                                repo.as_ref().and_then(|r| r.adversarial_review_enabled);
                            let repo_ar_model = repo
                                .as_ref()
                                .and_then(|r| r.adversarial_review_model.clone());

                            let provider = repo_provider.unwrap_or(global_provider);
                            let model = repo_model
                                .unwrap_or_else(|| self.config().default_model_for(provider));
                            let orch = workspace_orch
                                .or(repo_orch)
                                .unwrap_or(self.config().orchestration.enabled_by_default);
                            let ar_enabled =
                                workspace_ar_enabled.or(repo_ar_enabled).unwrap_or(false);
                            let ar_model = workspace_ar_model
                                .or(repo_ar_model)
                                .unwrap_or_else(|| "claude-sonnet-4-6".to_string());
                            (provider, model, orch, ar_enabled, ar_model)
                        };

                        self.state.workspace_progress_dialog_state.finish(
                            resolved_provider,
                            resolved_model,
                            resolved_orch,
                            resolved_ar_enabled,
                            resolved_ar_model,
                        );
                    }
                    Err(ref err) => {
                        self.state
                            .workspace_progress_dialog_state
                            .finish_with_error(err);
                    }
                }
            }
            AppEvent::ForkWorkspaceCreated {
                parent_workspace_id,
                result,
            } => {
                self.clear_workspace_busy(parent_workspace_id);
                match result {
                    Ok(created) => {
                        self.refresh_sidebar_data();
                        self.state.sidebar_data.expand_repo(created.repo_id);
                        if let Some(index) = self.find_workspace_index(created.workspace_id) {
                            self.state.sidebar_state.tree_state.selected = index;
                        }
                        match self.finish_fork_session(created.workspace_id) {
                            Ok(mut fork_effects) => {
                                effects.append(&mut fork_effects);
                            }
                            Err(err) => {
                                // Clean up fork seed
                                if let Some(pending) = self.state.pending_fork_request.take() {
                                    if let Some(seed_id) = pending.fork_seed_id {
                                        if let Some(dao) = self.fork_seed_dao() {
                                            if let Err(e) = dao.delete(seed_id) {
                                                tracing::debug!(
                                                    error = %e,
                                                    seed_id = %seed_id,
                                                    "Failed to delete fork seed after fork error"
                                                );
                                            }
                                        }
                                    }
                                }
                                // Attempt to clean up the created workspace
                                let cleanup_msg = self
                                    .cleanup_fork_workspace(created.workspace_id, created.repo_id);
                                let error_msg = match cleanup_msg {
                                    Some(cleanup_err) => format!(
                                        "{}\n\nWorkspace cleanup failed: {}. \
                                         You may need to manually remove it from the sidebar.",
                                        err, cleanup_err
                                    ),
                                    None => err.to_string(),
                                };
                                self.show_error("Fork Failed", &error_msg);
                            }
                        }
                    }
                    Err(err) => {
                        if let Some(pending) = self.state.pending_fork_request.take() {
                            if let Some(seed_id) = pending.fork_seed_id {
                                if let Some(dao) = self.fork_seed_dao() {
                                    if let Err(e) = dao.delete(seed_id) {
                                        tracing::debug!(
                                            error = %e,
                                            seed_id = %seed_id,
                                            "Failed to delete fork seed after fork error"
                                        );
                                    }
                                }
                            }
                        }
                        self.show_error("Fork Failed", &err);
                    }
                }
            }
            AppEvent::RemoveProjectDialogPreflightCompleted { repo_id, result } => {
                let is_active_preflight = self.state.confirmation_dialog_state.loading
                    && matches!(
                        self.state.confirmation_dialog_state.context,
                        Some(ConfirmationContext::RemoveProjectPreflightInProgress {
                            repo_id: id
                        }) if id == repo_id
                    );
                if !is_active_preflight {
                    return Ok(effects);
                }

                match result {
                    Ok(preflight) => {
                        let confirmation_type = match (preflight.has_dirty, preflight.has_unmerged)
                        {
                            (true, true) => ConfirmationType::Danger,
                            (true, false) | (false, true) => ConfirmationType::Warning,
                            (false, false) => {
                                if preflight.workspace_count > 0 {
                                    ConfirmationType::Warning
                                } else {
                                    ConfirmationType::Info
                                }
                            }
                        };

                        self.state.confirmation_dialog_state.show(
                            format!("Remove \"{}\"?", preflight.repo_name),
                            "This will archive all workspaces and remove the project.",
                            preflight.warnings,
                            confirmation_type,
                            "Remove",
                            Some(ConfirmationContext::RemoveProject(repo_id)),
                        );
                        self.state.input_mode = InputMode::Confirming;
                    }
                    Err(err) => {
                        self.state.confirmation_dialog_state.hide();
                        self.show_error("Project Removal Failed", &err);
                    }
                }
            }
            AppEvent::ForkSessionDialogPreflightCompleted {
                parent_workspace_id,
                result,
            } => {
                let is_active_preflight = self.state.confirmation_dialog_state.loading
                    && matches!(
                        self.state.confirmation_dialog_state.context,
                        Some(ConfirmationContext::ForkSessionPreflightInProgress {
                            parent_workspace_id: id
                        }) if id == parent_workspace_id
                    );
                if !is_active_preflight {
                    return Ok(effects);
                }

                match result {
                    Ok(preflight) => {
                        let Some(pending) = self.state.pending_fork_request.clone() else {
                            self.state.confirmation_dialog_state.hide();
                            self.state.input_mode = InputMode::Normal;
                            return Ok(effects);
                        };

                        if pending.parent_workspace_id != parent_workspace_id {
                            return Ok(effects);
                        }

                        let usage_pct = if pending.context_window > 0 {
                            (pending.token_estimate as f64 / pending.context_window as f64) * 100.0
                        } else {
                            0.0
                        };

                        let mut warnings = Vec::new();
                        let has_dirty = if let Some(desc) = preflight.dirty_warning {
                            warnings.push(desc);
                            warnings.push("Commit before forking to preserve changes.".to_string());
                            true
                        } else {
                            false
                        };

                        if usage_pct >= 100.0 {
                            warnings.push(format!(
                                "Seed exceeds context window ({} / {} tokens, ~{:.0}%).",
                                pending.token_estimate, pending.context_window, usage_pct
                            ));
                        } else if usage_pct >= 80.0 {
                            warnings.push(format!(
                                "Seed uses ~{:.0}% of context window ({} / {}).",
                                usage_pct, pending.token_estimate, pending.context_window
                            ));
                        }

                        let confirmation_type = if usage_pct >= 100.0 {
                            ConfirmationType::Danger
                        } else if has_dirty || usage_pct >= 80.0 {
                            ConfirmationType::Warning
                        } else {
                            ConfirmationType::Info
                        };

                        let message = format!(
                            "Fork this session into a new workspace based on branch \"{}\".\nSeed size: {} / {} tokens (~{:.0}%).",
                            preflight.base_branch,
                            pending.token_estimate,
                            pending.context_window,
                            usage_pct
                        );

                        self.state.confirmation_dialog_state.show(
                            "Fork session?",
                            message,
                            warnings,
                            confirmation_type,
                            "Fork",
                            Some(ConfirmationContext::ForkSession {
                                parent_workspace_id,
                                base_branch: preflight.base_branch,
                            }),
                        );
                        self.state.input_mode = InputMode::Confirming;
                    }
                    Err(err) => {
                        self.state.pending_fork_request = None;
                        self.state.confirmation_dialog_state.hide();
                        self.show_error("Fork Failed", &err);
                    }
                }
            }
            AppEvent::WorkCompletePreflightLoaded {
                workspace_id,
                result,
            } => {
                let is_our_session = self
                    .state
                    .work_complete_session
                    .as_ref()
                    .map(|s| s.workspace_id == workspace_id)
                    .unwrap_or(false);
                if !is_our_session {
                    return Ok(effects);
                }
                match result {
                    Ok(data) => {
                        if let Some(session) = self.state.work_complete_session.as_mut() {
                            session.data = Some(data.clone());
                        }
                        let sub_effects = self.dispatch_work_complete_event(
                            crate::work_complete::WorkCompleteEvent::PreflightLoaded(Box::new(
                                data,
                            )),
                        );
                        effects.extend(sub_effects);
                    }
                    Err(err) => {
                        let sub_effects = self.dispatch_work_complete_event(
                            crate::work_complete::WorkCompleteEvent::PreflightFailed(err),
                        );
                        effects.extend(sub_effects);
                    }
                }
            }
            AppEvent::WorkCompleteActionFinished {
                workspace_id,
                action,
                result,
            } => {
                let is_our_session = self
                    .state
                    .work_complete_session
                    .as_ref()
                    .map(|s| s.workspace_id == workspace_id)
                    .unwrap_or(false);
                if !is_our_session {
                    return Ok(effects);
                }
                match result {
                    Ok(log_lines) => {
                        if let Some(session) = self.state.work_complete_session.as_mut() {
                            session.log.extend(log_lines.clone());
                        }
                        // Archive closes tabs (and kills agents) exactly like WorkspaceArchived
                        if action == conduit_git::SuggestedAction::Archive {
                            self.close_tabs_for_workspace(workspace_id);
                            self.close_work_complete_dialog();
                            self.refresh_sidebar_data();
                            self.sync_sidebar_to_active_tab();
                            self.state.set_timed_footer_message(
                                "Workspace archived".to_string(),
                                Duration::from_secs(3),
                            );
                        } else if action == conduit_git::SuggestedAction::OpenPr {
                            // Parse the PR URL from the log ("Created PR #N: <url>") and
                            // automatically enter CI monitoring.
                            let pr_url = log_lines.iter().find_map(|l| {
                                let idx = l.find("Created PR #")?;
                                l[idx..].split_once(": ").map(|x| x.1.to_string())
                            });
                            if let Some(pr_url) = pr_url {
                                let sub_effects = self.dispatch_work_complete_event(
                                    crate::work_complete::WorkCompleteEvent::CiStarted { pr_url },
                                );
                                effects.extend(sub_effects);
                            } else {
                                let sub_effects = self.dispatch_work_complete_event(
                                    crate::work_complete::WorkCompleteEvent::ActionCompleted(
                                        log_lines,
                                    ),
                                );
                                effects.extend(sub_effects);
                            }
                        } else if action == conduit_git::SuggestedAction::Push {
                            // If there is already an open PR, enter CI monitoring.
                            let pr_url = self
                                .state
                                .work_complete_session
                                .as_ref()
                                .and_then(|s| s.data.as_ref())
                                .and_then(|d| d.pr.as_ref())
                                .filter(|pr| pr.is_open)
                                .and_then(|pr| pr.url.clone());
                            if let Some(pr_url) = pr_url {
                                let sub_effects = self.dispatch_work_complete_event(
                                    crate::work_complete::WorkCompleteEvent::CiStarted { pr_url },
                                );
                                effects.extend(sub_effects);
                            } else {
                                let sub_effects = self.dispatch_work_complete_event(
                                    crate::work_complete::WorkCompleteEvent::ActionCompleted(
                                        log_lines,
                                    ),
                                );
                                effects.extend(sub_effects);
                            }
                        } else {
                            let sub_effects = self.dispatch_work_complete_event(
                                crate::work_complete::WorkCompleteEvent::ActionCompleted(log_lines),
                            );
                            effects.extend(sub_effects);
                        }
                    }
                    Err(err) => {
                        let sub_effects = self.dispatch_work_complete_event(
                            crate::work_complete::WorkCompleteEvent::ActionFailed(err),
                        );
                        effects.extend(sub_effects);
                    }
                }
            }
            AppEvent::WorkCompleteCiFinished {
                workspace_id,
                result,
            } => {
                let is_our_session = self
                    .state
                    .work_complete_session
                    .as_ref()
                    .map(|s| s.workspace_id == workspace_id)
                    .unwrap_or(false);
                if !is_our_session {
                    return Ok(effects);
                }
                let (passed, log) = match result {
                    Ok((passed, lines)) => (passed, lines),
                    Err(err) => (false, vec![err]),
                };
                if let Some(session) = self.state.work_complete_session.as_mut() {
                    if passed {
                        // Clear the log on success so ReviewingState comes up clean.
                        session.log.clear();
                    } else {
                        session.log.extend(log.clone());
                    }
                }
                let sub_effects = self.dispatch_work_complete_event(
                    crate::work_complete::WorkCompleteEvent::CiCompleted { passed, log },
                );
                effects.extend(sub_effects);
            }
            AppEvent::ProjectsDiscovered { base_dir, result } => {
                if !self.state.project_picker_state.visible
                    || self.state.project_picker_state.base_dir != base_dir
                {
                    return Ok(effects);
                }

                match result {
                    Ok(entries) => {
                        let projects: Vec<ProjectEntry> = entries
                            .into_iter()
                            .map(|entry| ProjectEntry {
                                name: entry.name,
                                path: entry.path,
                            })
                            .collect();
                        self.state.project_picker_state.load_projects(projects);
                    }
                    Err(err) => {
                        self.state.project_picker_state.set_error(err);
                    }
                }
            }
            AppEvent::ProjectRemoved { result } => {
                self.clear_repo_busy(result.repo_id);
                self.clear_repo_action_busy(result.repo_id);
                for workspace_id in &result.workspace_ids {
                    self.close_tabs_for_workspace(*workspace_id);
                }

                let has_errors = !result.errors.is_empty();
                if has_errors {
                    tracing::warn!(
                        repo_id = %result.repo_id,
                        error_count = result.errors.len(),
                        errors = ?result.errors,
                        "Project removal completed with errors"
                    );
                    self.show_error_with_details(
                        "Project Removal Errors",
                        "Some operations failed during project removal",
                        &result.errors.join("\n"),
                    );
                }

                let current_selection = self.state.sidebar_state.tree_state.selected;
                self.refresh_sidebar_data();

                let visible_count = self.state.sidebar_data.visible_nodes().len();
                if visible_count > 0 {
                    let new_selection = if current_selection > 0 {
                        current_selection - 1
                    } else {
                        0
                    };
                    self.state.sidebar_state.tree_state.selected =
                        new_selection.min(visible_count - 1);
                    if !has_errors {
                        self.state.input_mode = InputMode::SidebarNavigation;
                    }
                } else {
                    self.state.sidebar_state.tree_state.selected = 0;
                    self.state.show_first_time_splash = true;
                    if !has_errors {
                        self.state.input_mode = InputMode::Normal;
                    }
                }
            }
            AppEvent::RepositoryCloned { result }
                if self.state.input_mode == InputMode::CloningRepository =>
            {
                match result {
                    Ok(path) => {
                        if let Some(repo_id) = self.add_project_to_sidebar(path) {
                            self.state.sidebar_data.expand_repo(repo_id);
                            if let Some(repo_index) =
                                self.state.sidebar_data.find_repo_index(repo_id)
                            {
                                self.state.sidebar_state.tree_state.selected = repo_index + 1;
                            }
                            self.state.sidebar_state.show();
                            self.state.sidebar_state.set_focused(true);
                            self.state.show_first_time_splash = false;
                            self.state.input_mode = InputMode::SidebarNavigation;
                            self.state.set_timed_footer_message(
                                "Repository cloned successfully".to_string(),
                                Duration::from_secs(4),
                            );
                        } else {
                            self.state.input_mode = InputMode::Normal;
                            self.state.set_timed_footer_message(
                                "Clone succeeded but failed to add project".to_string(),
                                Duration::from_secs(5),
                            );
                        }
                    }
                    Err(err) => {
                        self.state.input_mode = InputMode::Normal;
                        self.state.set_timed_footer_message(
                            format!("Clone failed: {err}"),
                            Duration::from_secs(6),
                        );
                    }
                }
            }
            AppEvent::AgentStarted {
                session_id,
                pid,
                input_tx,
            } => {
                // Store the PID for interrupt support
                let Some(tab_index) = self.state.tab_manager.session_index_by_id(session_id) else {
                    tracing::debug!(
                        %session_id,
                        "AgentStarted for unknown session; ignoring"
                    );
                    return Ok(effects);
                };
                let pid_start_time = conduit_util::process::pid_start_time(pid);
                if let Some(session) = self.state.tab_manager.session_mut(tab_index) {
                    session.agent_pid = Some(pid);
                    session.agent_pid_start_time = pid_start_time;
                    session.agent_input_tx = input_tx;
                    tracing::debug!(
                        session_id = %session_id,
                        "Agent started with PID {} for tab {}",
                        pid,
                        tab_index
                    );

                    // Display fork success message once when agent has started successfully
                    if session.fork_seed_id.is_some() && !session.fork_welcome_shown {
                        session.fork_welcome_shown = true;
                        let display = MessageDisplay::System {
                            content:
                                "Fork created; context injected. Waiting for your next prompt."
                                    .to_string(),
                        };
                        session.chat_view.push(display.to_chat_message());
                    }
                }
                if let Some(store) = self.session_tab_dao() {
                    if let Err(e) = store.set_agent_pid(session_id, pid, pid_start_time) {
                        tracing::warn!(error = %e, %session_id, "Failed to persist agent PID");
                    }
                }
            }
            AppEvent::AgentStartFailed { session_id, error } => {
                let Some(tab_index) = self.state.tab_manager.session_index_by_id(session_id) else {
                    tracing::debug!(
                        %session_id,
                        "AgentStartFailed for unknown session; ignoring"
                    );
                    return Ok(effects);
                };
                let is_active_tab = self.state.tab_manager.active_index() == tab_index;
                if let Some(session) = self.state.tab_manager.session_mut(tab_index) {
                    session.stop_processing();
                    session.chat_view.finalize_streaming();
                    session.tools_in_flight = 0;
                    session.set_processing_state(ProcessingState::Thinking);
                    session.agent_input_tx = None;
                    let display = MessageDisplay::Error { content: error };
                    session.chat_view.push(display.to_chat_message());
                }
                if is_active_tab {
                    self.state.stop_footer_spinner();
                }
            }
            AppEvent::AgentTerminationResult {
                session_id,
                pid,
                context,
                success,
            } if !success => {
                tracing::warn!(
                    pid,
                    context = %context,
                    "Agent termination did not complete"
                );
                if session_id
                    .and_then(|id| self.state.tab_manager.session_index_by_id(id))
                    .is_some()
                {
                    self.state.set_timed_footer_message(
                        "Failed to terminate agent; process may still be running".to_string(),
                        Duration::from_secs(5),
                    );
                }
            }
            AppEvent::AgentStreamEnded { session_id } => {
                let Some(tab_index) = self.state.tab_manager.session_index_by_id(session_id) else {
                    tracing::debug!(
                        %session_id,
                        "AgentStreamEnded for unknown session; ignoring"
                    );
                    return Ok(effects);
                };
                // Agent event stream ended (process exited) - ensure processing is stopped
                let is_active_tab = self.state.tab_manager.active_index() == tab_index;
                let was_processing =
                    if let Some(session) = self.state.tab_manager.session_mut(tab_index) {
                        // Clear PID since process has exited
                        session.agent_pid = None;
                        session.agent_pid_start_time = None;
                        session.agent_input_tx = None;
                        // Safety: don't let fork-seed suppression leak into future runs
                        session.suppress_next_assistant_reply = false;
                        session.suppress_next_turn_summary = false;
                        let was_processing = if session.is_processing {
                            session.stop_processing();
                            true
                        } else {
                            false
                        };

                        Self::flush_pending_agent_output(session);
                        session.tools_in_flight = 0;
                        was_processing
                    } else {
                        false
                    };
                // Only stop footer spinner if this was the active tab
                if was_processing && is_active_tab {
                    self.state.stop_footer_spinner();
                }

                if let Some(store) = self.session_tab_dao() {
                    if let Err(e) = store.clear_agent_pid(session_id) {
                        tracing::warn!(error = %e, %session_id, "Failed to clear agent PID");
                    }
                }

                match self.drain_queue_for_tab(tab_index) {
                    Ok(mut queued_effects) => effects.append(&mut queued_effects),
                    Err(err) => {
                        tracing::warn!(error = %err, "Failed to drain queued messages");
                    }
                }
            }
            AppEvent::SessionsCacheLoaded { sessions } => {
                // Load cached sessions immediately - fast path
                self.state.session_import_state.load_sessions(sessions);
                // Keep loading=true since background refresh continues
            }
            AppEvent::SessionUpdated { session } => {
                // Add or update single session during background refresh
                self.state.session_import_state.upsert_session(session);
            }
            AppEvent::SessionRemoved { file_path } => {
                // Remove session by file path (file no longer exists)
                self.state
                    .session_import_state
                    .remove_session_by_path(&file_path);
            }
            AppEvent::SessionDiscoveryComplete => {
                // Background refresh done - stop spinner
                self.state.session_import_state.set_loading(false);
            }
            AppEvent::GitTracker(update) => {
                self.handle_git_tracker_update(update);
            }
            AppEvent::ShellCommandCompleted {
                session_id,
                message_index,
                result,
            } => {
                let Some(session) = self.state.tab_manager.session_by_id_mut(session_id) else {
                    tracing::debug!(
                        %session_id,
                        "ShellCommandCompleted for unknown session; ignoring"
                    );
                    return Ok(effects);
                };

                let (output, exit_code) = match result {
                    Ok(output) => (output.output, output.exit_code),
                    Err(err) => (format!("Error: {}", err), Some(1)),
                };

                if !session
                    .chat_view
                    .update_tool_at(message_index, output, exit_code)
                {
                    tracing::warn!(
                        session_id = %session_id,
                        message_index,
                        "ShellCommandCompleted: no matching tool message found to update"
                    );
                }
            }
            AppEvent::OpencodeQuestionResponseCompleted { session_id, result } => {
                let is_active_tab = self
                    .state
                    .tab_manager
                    .active_session()
                    .map(|active| active.id == session_id)
                    .unwrap_or(false);
                let Some(session) = self.state.tab_manager.session_by_id_mut(session_id) else {
                    tracing::debug!(
                        %session_id,
                        "OpencodeQuestionResponseCompleted for unknown session; ignoring"
                    );
                    return Ok(effects);
                };

                let mut should_stop_footer_spinner = false;
                session.tools_in_flight = match session.tools_in_flight.checked_sub(1) {
                    Some(value) => value,
                    None => {
                        tracing::warn!("tools_in_flight underflow on OpencodeQuestionResponse");
                        0
                    }
                };
                session.set_processing_state(ProcessingState::Thinking);

                if session.tools_in_flight == 0 {
                    session.stop_processing();
                    should_stop_footer_spinner = is_active_tab;
                }

                if let Err(err) = result {
                    session.chat_view.push(
                        MessageDisplay::Error {
                            content: format!("OpenCode question response failed: {}", err),
                        }
                        .to_chat_message(),
                    );
                }
                if should_stop_footer_spinner {
                    self.state.stop_footer_spinner();
                }
            }
            AppEvent::TitleGenerated { session_id, result } => {
                // Single lookup - session must exist to proceed
                let Some(session) = self.state.tab_manager.session_by_id_mut(session_id) else {
                    tracing::debug!(
                        %session_id,
                        "Stale TitleGenerated event: session no longer exists"
                    );
                    return Ok(effects);
                };
                // Clear pending flag once, regardless of result
                session.title_generation_pending = false;

                match result {
                    Ok(generated) => {
                        tracing::info!(
                            %session_id,
                            title = %generated.title,
                            new_branch = ?generated.new_branch,
                            "Session title generated"
                        );

                        // Update session title and branch display
                        session.title = Some(generated.title.clone());
                        if let Some(new_branch) = &generated.new_branch {
                            session.status_bar.set_branch_name(Some(new_branch.clone()));
                        }

                        if generated.used_fallback {
                            let tool = generated.tool_used.as_deref().unwrap_or("fallback tool");
                            self.state.set_timed_footer_message(
                                format!("Title generated via {}", tool),
                                Duration::from_secs(4),
                            );
                        }

                        // Update sidebar directly with new branch name
                        // (avoids stale DB read if DB update failed but git rename succeeded)
                        if let (Some(ws_id), Some(ref new_branch)) =
                            (generated.workspace_id, &generated.new_branch)
                        {
                            self.state
                                .sidebar_data
                                .update_workspace_branch(ws_id, Some(new_branch.clone()));
                        }

                        // Save session state to persist the title
                        effects.push(Effect::SaveSessionState);
                    }
                    Err(e) => {
                        tracing::warn!(%session_id, error = %e, "Failed to generate session title");
                        // Show transient footer message (less noisy than chat message)
                        self.state.set_timed_footer_message(
                            format!("Title generation failed: {}", e),
                            Duration::from_secs(5),
                        );
                    }
                }
            }
            _ => {}
        }

        Ok(effects)
    }

    /// Handle updates from the background git tracker
    fn handle_git_tracker_update(&mut self, update: crate::git_tracker::GitTrackerUpdate) {
        use crate::git_tracker::GitTrackerUpdate;

        match update {
            GitTrackerUpdate::PrStatusChanged {
                workspace_id,
                status,
            } => {
                tracing::debug!(
                    workspace_id = %workspace_id,
                    pr_exists = status.as_ref().map(|s| s.exists),
                    pr_number = status.as_ref().and_then(|s| s.number),
                    pr_state = ?status.as_ref().map(|s| s.state),
                    check_state = ?status.as_ref().map(|s| s.checks.state()),
                    merge_readiness = ?status.as_ref().map(|s| s.merge_readiness),
                    "Received PR status update"
                );
                let is_stale_pr = status.as_ref().is_some_and(|s| {
                    matches!(
                        s.state,
                        conduit_git::PrState::Merged | conduit_git::PrState::Closed
                    )
                });
                let mut any_session_updated = false;
                // Update all sessions with this workspace
                for session in self.state.tab_manager.sessions_mut() {
                    if session.workspace_id == Some(workspace_id) {
                        // CRITICAL: Stale PR Prevention
                        // If session has no PR yet, don't auto-associate merged/closed PRs.
                        // This prevents "ghost" PRs from reused branch names from being resurrected.
                        let is_new_association = session.pr_number.is_none();

                        if is_new_association && is_stale_pr {
                            tracing::debug!(
                                workspace_id = %workspace_id,
                                pr_number = status.as_ref().and_then(|s| s.number),
                                "Ignoring stale (merged/closed) PR for new session"
                            );
                            self.state
                                .sidebar_data
                                .clear_workspace_pr_status(workspace_id);
                            continue;
                        }

                        if let Some(status) = status.clone() {
                            Self::apply_pr_status_to_session(session, status);
                            any_session_updated = true;
                        }
                    }
                }
                // Update sidebar data when we have an accepted association or when not stale.
                if !is_stale_pr || any_session_updated {
                    self.state
                        .sidebar_data
                        .update_workspace_pr_status(workspace_id, status);
                } else {
                    self.state
                        .sidebar_data
                        .clear_workspace_pr_status(workspace_id);
                }
            }
            GitTrackerUpdate::GitStatsChanged {
                workspace_id,
                stats,
            } => {
                tracing::info!(
                    workspace_id = %workspace_id,
                    additions = stats.additions,
                    deletions = stats.deletions,
                    files_changed = stats.files_changed,
                    "Received GitStatsChanged event"
                );

                // Update all sessions with this workspace
                for session in self.state.tab_manager.sessions_mut() {
                    if session.workspace_id == Some(workspace_id) {
                        session.status_bar.set_git_diff_stats(stats.clone());
                    }
                }
                // Also update sidebar data
                self.state
                    .sidebar_data
                    .update_workspace_git_stats(workspace_id, stats);
            }
            GitTrackerUpdate::AheadBehindChanged {
                workspace_id,
                commits_ahead,
                commits_behind,
            } => {
                self.state.sidebar_data.update_workspace_ahead_behind(
                    workspace_id,
                    commits_ahead,
                    commits_behind,
                );
            }
            GitTrackerUpdate::BranchChanged {
                workspace_id,
                branch,
            } => {
                if self.state.busy_workspaces.contains(&workspace_id) {
                    tracing::debug!(
                        workspace_id = %workspace_id,
                        "Skipping branch update for busy workspace"
                    );
                    self.state
                        .pending_branch_updates
                        .insert(workspace_id, branch);
                    return;
                }
                self.apply_branch_update(workspace_id, branch);
            }
        }
    }

    fn apply_branch_update(&mut self, workspace_id: uuid::Uuid, branch: Option<String>) {
        for session in self.state.tab_manager.sessions_mut() {
            if session.workspace_id == Some(workspace_id) {
                session.status_bar.set_branch_name(branch.clone());
                session.branch_name = branch.clone();
            }
        }
        self.state
            .sidebar_data
            .update_workspace_branch(workspace_id, branch);
    }

    fn flush_pending_agent_output(session: &mut crate::session::AgentSession) {
        // Safety: ensure no partial streaming buffer remains before pushing buffered messages.
        session.chat_view.finalize_streaming();
        if let Some(summary) = session.pending_turn_summary.take() {
            session.chat_view.push(ChatMessage::turn_summary(summary));
        }
    }

    /// Handle the result of the PR preflight check
    fn handle_pr_preflight_result(
        &mut self,
        tab_index: usize,
        working_dir: std::path::PathBuf,
        preflight: conduit_git::PrPreflightResult,
    ) -> Vec<Effect> {
        let effects = Vec::new();
        let mut sidebar_pr_update: Option<(Uuid, PrStatus)> = None;
        let mut sidebar_pr_clear: Option<Uuid> = None;
        // Tab indices may shift while preflight runs; only trust tab_index if it still matches.
        let mut initiating_session_id = self
            .state
            .tab_manager
            .session(tab_index)
            .and_then(|session| {
                let still_same_dir = session
                    .working_dir
                    .as_ref()
                    .is_some_and(|dir| dir == &working_dir);
                still_same_dir.then_some(session.id)
            })
            // Fallback: resolve by working_dir (more stable than tab index).
            .or_else(|| {
                self.state
                    .tab_manager
                    .sessions()
                    .iter()
                    .find(|session| {
                        session
                            .working_dir
                            .as_ref()
                            .is_some_and(|dir| dir == &working_dir)
                    })
                    .map(|session| session.id)
            });
        let preflight_workspace_id = initiating_session_id.and_then(|id| {
            self.state
                .tab_manager
                .sessions()
                .iter()
                .find(|session| session.id == id)
                .and_then(|session| session.workspace_id)
        });
        // Handle blocking errors
        if !preflight.gh_installed {
            self.state.confirmation_dialog_state.hide();
            // Show missing tool dialog with context about PR creation
            self.state.close_overlays();
            self.state.missing_tool_dialog_state.show_with_context(
                conduit_util::Tool::Gh,
                "GitHub CLI (gh) is required for PR operations.",
            );
            self.state.input_mode = crate::events::InputMode::MissingTool;
            return effects;
        }

        if !preflight.gh_authenticated {
            self.state.confirmation_dialog_state.hide();
            self.show_error_with_details(
                "Not Authenticated",
                "GitHub CLI is not authenticated.",
                "Run: gh auth login",
            );
            return effects;
        }

        if preflight.on_main_branch {
            self.state.confirmation_dialog_state.hide();
            self.show_error(
                "Cannot Create PR",
                &format!(
                    "You're on the '{}' branch. Create a feature branch first.",
                    preflight.branch_name
                ),
            );
            return effects;
        }

        // If we explicitly determined no PR exists, clear any stale PR UI state.
        if matches!(preflight.existing_pr.as_ref(), Some(pr) if !pr.exists) {
            if let Some(workspace_id) = preflight_workspace_id {
                for session in self.state.tab_manager.sessions_mut() {
                    if session.workspace_id == Some(workspace_id) {
                        session.pr_number = None;
                        session.status_bar.set_pr_status(None);
                    }
                }
                sidebar_pr_clear = Some(workspace_id);
            } else if let Some(session_id) = initiating_session_id.take() {
                if let Some(session) = self.state.tab_manager.session_by_id_mut(session_id) {
                    session.pr_number = None;
                    session.status_bar.set_pr_status(None);
                }
            }
        }

        // If PR exists, show confirmation dialog to open in browser
        if let Some(ref pr) = preflight.existing_pr {
            if pr.exists {
                // Update session's pr_number
                if let Some(workspace_id) = preflight_workspace_id {
                    let status = pr.clone();
                    for session in self.state.tab_manager.sessions_mut() {
                        if session.workspace_id == Some(workspace_id) {
                            Self::apply_pr_status_to_session(session, status.clone());
                        }
                    }
                    sidebar_pr_update = Some((workspace_id, status));
                } else if let Some(session_id) = initiating_session_id.take() {
                    if let Some(session) = self.state.tab_manager.session_by_id_mut(session_id) {
                        let status = pr.clone();
                        Self::apply_pr_status_to_session(session, status);
                    }
                }

                let pr_url = pr.url.clone().unwrap_or_else(|| "Unknown URL".to_string());
                self.state.close_overlays();
                self.state.confirmation_dialog_state.show(
                    "Pull Request Exists",
                    format!(
                        "PR #{} already exists for branch '{}'.\n\nOpen in browser?",
                        pr.number.unwrap_or(0),
                        preflight.branch_name
                    ),
                    vec![],
                    ConfirmationType::Info,
                    "Open PR",
                    Some(ConfirmationContext::OpenExistingPr {
                        working_dir,
                        pr_url,
                    }),
                );
                if let Some((workspace_id, status)) = sidebar_pr_update {
                    self.state
                        .sidebar_data
                        .update_workspace_pr_status(workspace_id, Some(status));
                }
                // Already in Confirming mode
                return effects;
            }
        }

        if let Some(workspace_id) = sidebar_pr_clear {
            self.state
                .sidebar_data
                .clear_workspace_pr_status(workspace_id);
        }

        // Build warnings for confirmation dialog
        let mut warnings = Vec::new();
        if preflight.uncommitted_count > 0 {
            warnings.push(format!(
                "{} file(s) will be auto-committed",
                preflight.uncommitted_count
            ));
        }
        if !preflight.has_upstream {
            warnings.push("Branch will be pushed to remote".to_string());
        }

        // Show confirmation dialog (replace loading state)
        self.state.close_overlays();
        self.state.confirmation_dialog_state.show(
            "Create Pull Request",
            format!(
                "Branch: {}\nTarget: {}",
                preflight.branch_name, preflight.target_branch
            ),
            warnings,
            ConfirmationType::Info,
            "Create PR",
            Some(ConfirmationContext::CreatePullRequest {
                tab_index,
                working_dir,
                preflight,
            }),
        );
        // Already in Confirming mode
        effects
    }

    /// Submit the PR workflow prompt to the current chat
    fn submit_pr_workflow(
        &mut self,
        tab_index: usize,
        working_dir: std::path::PathBuf,
        preflight: conduit_git::PrPreflightResult,
    ) -> anyhow::Result<Vec<Effect>> {
        let target_tab_index = self
            .state
            .tab_manager
            .session(tab_index)
            .and_then(|session| {
                let matches_dir = session
                    .working_dir
                    .as_ref()
                    .is_some_and(|dir| dir == &working_dir);
                matches_dir.then_some(tab_index)
            })
            .or_else(|| {
                self.state
                    .tab_manager
                    .sessions()
                    .iter()
                    .position(|session| {
                        session
                            .working_dir
                            .as_ref()
                            .is_some_and(|dir| dir == &working_dir)
                    })
            });
        // Generate prompt for PR creation
        let prompt = PrManager::generate_pr_prompt(&preflight);

        let Some(target_tab_index) = target_tab_index else {
            self.show_error(
                "Cannot Create PR",
                "No session found for the PR preflight workspace.",
            );
            return Ok(Vec::new());
        };

        // Submit to the intended chat session
        self.submit_prompt_for_tab(
            target_tab_index,
            prompt,
            Vec::new(),
            Vec::new(),
            false,
            None,
        )
    }

    fn restore_queued_to_input(&mut self, message: conduit_data::QueuedMessage) {
        if let Some(session) = self.state.tab_manager.active_session_mut() {
            let attachments = message
                .images
                .iter()
                .map(|img| (img.path.clone(), img.placeholder.clone()))
                .collect();
            session
                .input_box
                .set_input_with_attachments(message.text, attachments);
            session.input_box.move_end();
        }
    }

    fn open_queue_editor(&mut self) {
        let has_queue = {
            let Some(session) = self.state.tab_manager.active_session_mut() else {
                return;
            };
            !session.queued_messages.is_empty()
        };

        if !has_queue {
            self.state
                .set_timed_footer_message("No queued messages".to_string(), Duration::from_secs(3));
            return;
        }

        self.state.close_overlays();
        if let Some(session) = self.state.tab_manager.active_session_mut() {
            if session.queue_selection.is_none() {
                session.queue_selection = Some(session.queued_messages.len() - 1);
            }
        }
        self.state.input_mode = InputMode::QueueEditing;
    }

    fn close_queue_editor(&mut self) {
        if let Some(session) = self.state.tab_manager.active_session_mut() {
            session.queue_selection = None;
        }
        self.state.input_mode = InputMode::Normal;
    }

    fn show_steer_fallback_prompt(&mut self, message_id: Uuid) {
        self.state.close_overlays();
        self.state.confirmation_dialog_state.show(
            "Interrupt to Steer",
            "Steering isn't supported by this harness.\nInterrupt the current run and send now?",
            vec![
                "In-flight tool execution will be stopped.".to_string(),
                "Queued message will be sent immediately.".to_string(),
            ],
            ConfirmationType::Warning,
            "Interrupt",
            Some(ConfirmationContext::SteerFallback { message_id }),
        );
        self.state.input_mode = InputMode::Confirming;
    }

    fn confirm_steer_fallback(&mut self, message_id: Uuid) -> anyhow::Result<Vec<Effect>> {
        let mut effects = Vec::new();
        let mut queued: Option<QueuedMessage> = None;

        {
            if let Some(session) = self.state.tab_manager.active_session_mut() {
                if let Some(idx) = session
                    .queued_messages
                    .iter()
                    .position(|msg| msg.id == message_id)
                {
                    queued = session.remove_queue_at(idx);
                }
            }
        }

        if let Some(message) = queued {
            self.interrupt_agent();
            let (text, images, placeholders) = app_queue::queued_to_submission(&message);
            effects.extend(self.submit_prompt(text, images, placeholders)?);
        } else {
            self.state.set_timed_footer_message(
                "Queued steering message not found".to_string(),
                Duration::from_secs(3),
            );
        }

        Ok(effects)
    }

    fn drain_queue_for_tab(&mut self, tab_index: usize) -> anyhow::Result<Vec<Effect>> {
        let mut effects = Vec::new();
        let mut queued: Vec<QueuedMessage> = Vec::new();
        let (queue_mode, queue_delivery) = (self.config().queue.mode, self.config().queue.delivery);

        {
            let Some(session) = self.state.tab_manager.session_mut(tab_index) else {
                return Ok(effects);
            };

            if session.queued_messages.is_empty() {
                return Ok(effects);
            }

            let mut remaining = Vec::new();
            match queue_mode {
                conduit_config::QueueMode::OneAtATime => {
                    let idx = session
                        .queued_messages
                        .iter()
                        .position(|msg| msg.mode == QueuedMessageMode::Steer)
                        .unwrap_or(0);
                    for (pos, msg) in session.queued_messages.drain(..).enumerate() {
                        if pos == idx {
                            queued.push(msg);
                        } else {
                            remaining.push(msg);
                        }
                    }
                }
                conduit_config::QueueMode::All => {
                    let mut steers = Vec::new();
                    let mut followups = Vec::new();
                    for msg in session.queued_messages.drain(..) {
                        if msg.mode == QueuedMessageMode::Steer {
                            steers.push(msg);
                        } else {
                            followups.push(msg);
                        }
                    }
                    queued.extend(steers);
                    queued.extend(followups);
                }
            }

            if queue_delivery == conduit_config::QueueDelivery::Separate && queued.len() > 1 {
                let mut requeue = queued.split_off(1);
                requeue.extend(remaining);
                session.queued_messages = requeue;
            } else {
                session.queued_messages = remaining;
            }
            session.queue_selection = None;
            session.update_status();
        }

        if queued.is_empty() {
            return Ok(effects);
        }

        let (prompt, images, placeholders) =
            app_queue::build_queued_submission(&queued, queue_delivery);
        effects.extend(self.submit_prompt_for_tab(
            tab_index,
            prompt,
            images,
            placeholders,
            false,
            None,
        )?);

        Ok(effects)
    }
}

struct SessionStateSnapshot {
    tabs: Vec<SessionTab>,
    active_tab_index: usize,
    sidebar_visible: bool,
    tree_selected_index: usize,
    collapsed_repo_ids: Vec<uuid::Uuid>,
}

#[derive(Debug, Default)]
struct SessionPersistenceReport {
    errors: Vec<String>,
}

impl SessionPersistenceReport {
    fn push(&mut self, message: String) {
        self.errors.push(message);
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    fn error_count(&self) -> usize {
        self.errors.len()
    }

    fn first_error_or_unknown(&self) -> &str {
        self.errors
            .first()
            .map(std::string::String::as_str)
            .unwrap_or("unknown error")
    }
}

fn build_adversarial_review_prompt() -> String {
    "Perform an adversarial code review of the changes in this workspace.\n\n\
Steps:\n\
1. Check for an open PR: `gh pr view --json url,number,title 2>/dev/null`\n\
2. If a PR exists, get the full diff: `gh pr diff`\n\
   Otherwise get local branch changes: \
`git diff $(git merge-base HEAD origin/master 2>/dev/null || echo HEAD)..HEAD`\n\
3. Use the conduit-adversarial-review sub-agent to analyse the diff critically.\n\
4. Report findings by severity: CRITICAL / HIGH / MEDIUM / LOW.\n\
5. For every CRITICAL or HIGH finding, offer to fix it immediately."
        .to_string()
}

/// Blocking helper that collects all inputs for the Work Complete preflight.
fn run_work_complete_preflight(
    workspace_id: uuid::Uuid,
    workspace_dao: Option<conduit_data::WorkspaceStore>,
    repo_dao: Option<conduit_data::RepositoryStore>,
    worktree_manager: conduit_git::WorkspaceRepoManager,
    config: &conduit_config::Config,
) -> Result<crate::work_complete::WorkCompleteData, String> {
    use conduit_git::{
        classify, fetch_change_detail, git_diff_files, infer_active_change, infer_active_issue,
        view_issue, ContextSource, GitState, IssueSnapshot, PrManager, PrSnapshot, PrState,
        SpecSnapshot,
    };

    let workspace_dao = workspace_dao.ok_or("Workspace database unavailable")?;
    let repo_dao = repo_dao.ok_or("Repository database unavailable")?;

    let workspace = workspace_dao
        .get_by_id(workspace_id)
        .map_err(|e| format!("Failed to load workspace: {e}"))?
        .ok_or("Workspace not found")?;

    let repo = repo_dao
        .get_by_id(workspace.repository_id)
        .map_err(|e| format!("Failed to load repository: {e}"))?
        .ok_or("Repository not found")?;

    let path = &workspace.path;

    // --- Branch status ---
    let branch_name = worktree_manager
        .get_current_branch(path)
        .unwrap_or_else(|_| workspace.branch.clone());
    let branch_status = worktree_manager
        .get_branch_status_with_gh_option(path, config.workspaces.use_gh_cli_merge_status)
        .unwrap_or_default();
    let dirty_files: Vec<String> = git_diff_files(path).into_iter().map(|f| f.path).collect();

    // --- PR preflight ---
    let pr_preflight = PrManager::preflight_check(path);
    let target_branch = pr_preflight.target_branch.clone();

    let pr = pr_preflight
        .existing_pr
        .as_ref()
        .filter(|p| p.exists)
        .map(|p| crate::work_complete::PrData {
            number: p.number.unwrap_or(0),
            url: p.url.clone(),
            title: p.title.clone(),
            is_open: p.state == PrState::Open,
            is_merged: p.state == PrState::Merged,
            merge_readiness: p.merge_readiness,
        });

    // --- Spec resolution ---
    let (spec_change_id, spec_source) = if let Some(ref id) = workspace.active_change_id {
        (Some(id.clone()), ContextSource::Linked)
    } else {
        let inferred = infer_active_change(path, &target_branch);
        if let Some(ref cid) = inferred {
            let _ = workspace_dao.update_active_links(workspace.id, Some(cid.clone()), None);
        }
        (inferred, ContextSource::Detected)
    };

    let spec = spec_change_id.as_deref().and_then(|change_id| {
        fetch_change_detail(path, change_id).map(|detail| crate::work_complete::SpecData {
            change_id: change_id.to_string(),
            total: detail.total,
            completed: detail.completed,
            source: spec_source,
        })
    });

    // --- Issue resolution ---
    let (issue_number, issue_source) = if let Some(n) = workspace.active_issue_number {
        (Some(n), ContextSource::Linked)
    } else {
        let inferred = infer_active_issue(&workspace.branch);
        if let Some(n) = inferred {
            let _ = workspace_dao.update_active_links(workspace.id, None, Some(n));
        }
        (inferred, ContextSource::Detected)
    };

    let issue = issue_number.map(|n| {
        let view = view_issue(path, n);
        crate::work_complete::IssueData {
            number: n,
            title: view.as_ref().map(|v| v.title.clone()),
            is_open: view.as_ref().map(|v| v.state == "OPEN").unwrap_or(true),
            source: issue_source,
        }
    });

    // --- Classify ---
    let git_state = GitState {
        is_dirty: branch_status.is_dirty,
        commits_ahead: branch_status.commits_ahead as u32,
        commits_behind: branch_status.commits_behind as u32,
        is_merged: branch_status.is_merged,
        has_upstream: pr_preflight.has_upstream,
    };
    let pr_snapshot = pr.as_ref().map(|p| PrSnapshot {
        number: p.number,
        is_open: p.is_open,
        is_merged: p.is_merged,
        merge_readiness: p.merge_readiness,
    });
    let spec_snapshot = spec.as_ref().map(|s| SpecSnapshot {
        change_id: s.change_id.clone(),
        total: s.total,
        completed: s.completed,
        source: s.source,
    });
    let issue_snapshot = issue.as_ref().map(|i| IssueSnapshot {
        number: i.number,
        is_open: i.is_open,
        source: i.source,
    });

    let adversarial_review_enabled = workspace
        .adversarial_review_enabled
        .or(repo.adversarial_review_enabled)
        .unwrap_or(false);

    let (scenario, suggested_actions) = classify(
        &git_state,
        pr_snapshot.as_ref(),
        spec_snapshot.as_ref(),
        issue_snapshot.as_ref(),
        adversarial_review_enabled,
    );

    Ok(crate::work_complete::WorkCompleteData {
        branch_name,
        is_dirty: branch_status.is_dirty,
        dirty_files,
        commits_ahead: git_state.commits_ahead,
        commits_behind: git_state.commits_behind,
        is_merged: git_state.is_merged,
        has_upstream: git_state.has_upstream,
        pr,
        spec,
        issue,
        scenario,
        suggested_actions,
        adversarial_review_model: workspace
            .adversarial_review_model
            .or(repo.adversarial_review_model),
    })
}

/// Blocking helper that executes a single Work Complete action.
fn run_work_complete_action(
    workspace_id: uuid::Uuid,
    action: conduit_git::SuggestedAction,
    payload: Option<String>,
    workspace_dao: Option<conduit_data::WorkspaceStore>,
    repo_dao: Option<conduit_data::RepositoryStore>,
    worktree_manager: conduit_git::WorkspaceRepoManager,
    config: &conduit_config::Config,
) -> Result<Vec<String>, String> {
    use conduit_git::{
        archive_change, close_issue, commit_all, infer_active_issue, push_branch, MergeMethod,
        MergeReadiness, PrCreateOpts, PrManager, SuggestedAction,
    };

    let workspace_dao = workspace_dao.ok_or("Workspace database unavailable")?;
    let workspace = workspace_dao
        .get_by_id(workspace_id)
        .map_err(|e| format!("Failed to load workspace: {e}"))?
        .ok_or("Workspace not found")?;
    let path = &workspace.path;

    match action {
        SuggestedAction::Commit => {
            let message = payload.ok_or("Commit message required")?;
            let sha = commit_all(path, &message).map_err(|e| format!("Commit failed: {e}"))?;
            Ok(vec![format!("Committed {}", sha)])
        }
        SuggestedAction::Push => {
            let set_upstream = !workspace.branch.is_empty();
            push_branch(path, &workspace.branch, set_upstream)
                .map_err(|e| format!("Push failed: {e}"))?;
            Ok(vec![format!("Pushed {}", workspace.branch)])
        }
        SuggestedAction::OpenPr => {
            // Push first so the branch exists on the remote before creating the PR.
            let set_upstream = !workspace.branch.is_empty();
            push_branch(path, &workspace.branch, set_upstream)
                .map_err(|e| format!("Push failed: {e}"))?;
            let preflight = PrManager::preflight_check(path);
            let opts = PrCreateOpts {
                base_branch: preflight.target_branch.clone(),
                title: None,
                body: None,
            };
            let pr =
                PrManager::create(path, &opts).map_err(|e| format!("gh pr create failed: {e}"))?;
            Ok(vec![
                format!("Pushed {}", workspace.branch),
                format!("Created PR #{}: {}", pr.number, pr.url),
            ])
        }
        SuggestedAction::MergePr => {
            let preflight = PrManager::preflight_check(path);
            if let Some(pr) = &preflight.existing_pr {
                if !matches!(pr.merge_readiness, MergeReadiness::Ready) {
                    return Err(format!(
                        "PR is not ready to merge ({:?})",
                        pr.merge_readiness
                    ));
                }
            }
            PrManager::merge(path, MergeMethod::Squash, false)
                .map_err(|e| format!("gh pr merge failed: {e}"))?;
            Ok(vec!["PR merged".to_string()])
        }
        SuggestedAction::CloseIssue => {
            let issue_number = workspace
                .active_issue_number
                .or_else(|| infer_active_issue(&workspace.branch));
            let number = issue_number.ok_or("No linked issue found for this workspace")?;
            close_issue(path, number).map_err(|e| format!("gh issue close failed: {e}"))?;
            Ok(vec![format!("Closed issue #{}", number)])
        }
        SuggestedAction::ArchiveSpec => {
            let change_id = workspace
                .active_change_id
                .ok_or("No linked spec found for this workspace")?;
            let today = chrono::Local::now().date_naive();
            let result = archive_change(path, &change_id, today)
                .map_err(|e| format!("Spec archive failed: {e}"))?;
            Ok(vec![format!(
                "Archived spec to {}",
                result.new_path.display()
            )])
        }
        SuggestedAction::Archive => {
            let repo_dao = repo_dao.ok_or("Repository database unavailable")?;
            let repo = repo_dao
                .get_by_id(workspace.repository_id)
                .map_err(|e| format!("Failed to load repository: {e}"))?
                .ok_or("Repository not found")?;
            let settings = resolve_repo_workspace_settings(config, &repo);
            let mut warnings = Vec::new();
            let mut archived_commit_sha = None;

            if let Some(base_path) = repo.base_path {
                match worktree_manager.get_branch_sha(
                    settings.mode,
                    &base_path,
                    path,
                    &workspace.branch,
                ) {
                    Ok(sha) => archived_commit_sha = Some(sha),
                    Err(e) => warnings.push(format!("Failed to read branch SHA: {e}")),
                }

                if let Err(e) = worktree_manager.remove_workspace(settings.mode, &base_path, path) {
                    warnings.push(format!("Failed to remove worktree: {e}"));
                }

                if settings.archive_delete_branch {
                    if let Err(e) = worktree_manager.delete_branch(
                        settings.mode,
                        &base_path,
                        path,
                        &workspace.branch,
                    ) {
                        warnings.push(format!("Failed to delete branch: {e}"));
                    }
                }
            }

            workspace_dao
                .archive(workspace_id, archived_commit_sha)
                .map_err(|e| format!("Failed to archive workspace in database: {e}"))?;

            let mut log = vec!["Workspace archived".to_string()];
            log.extend(warnings);
            Ok(log)
        }
        SuggestedAction::ShowRemainingTasks => Err(
            "ShowRemainingTasks is handled by the TUI and should not be executed here".to_string(),
        ),
        SuggestedAction::AdversarialReview => Err(
            "AdversarialReview is handled by the TUI and should not be executed here".to_string(),
        ),
    }
}

/// Async helper for generating title and branch name
async fn generate_title_and_branch_impl(
    tools: ToolAvailability,
    user_message: String,
    working_dir: PathBuf,
    workspace_id: Option<uuid::Uuid>,
    current_branch: String,
    worktree_manager: WorkspaceRepoManager,
    workspace_dao: Option<WorkspaceStore>,
) -> Result<TitleGeneratedResult, String> {
    use conduit_agent::{generate_title_and_branch, sanitize_branch_suffix};
    use conduit_util::get_git_username;

    // Call AI for title generation
    let metadata = generate_title_and_branch(&tools, &user_message, &working_dir)
        .await
        .map_err(|e| e.to_string())?;

    // Try to rename branch if workspace exists
    let new_branch = if workspace_id.is_some() {
        // Always fetch fresh branch from git - the passed-in current_branch may be stale
        // Only fall back to passed-in value if git lookup fails or returns empty
        let resolved_branch = {
            let wd = working_dir.clone();
            let wm = worktree_manager.clone();
            let wd_for_log = wd.clone();
            let fresh_branch = match tokio::task::spawn_blocking(move || {
                wm.get_current_branch(&wd).map_err(|e| e.to_string())
            })
            .await
            {
                Ok(Ok(branch)) => branch,
                Ok(Err(err)) => {
                    tracing::warn!(
                        error = %err,
                        working_dir = %wd_for_log.display(),
                        "Failed to fetch current branch from worktree"
                    );
                    String::new()
                }
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        "spawn_blocking failed while fetching current branch"
                    );
                    String::new()
                }
            };
            if fresh_branch.is_empty() {
                current_branch.clone()
            } else {
                fresh_branch
            }
        };

        if resolved_branch.is_empty() {
            tracing::debug!("Skipping branch rename: could not determine current branch");
            None
        } else {
            let raw_username = get_git_username();
            // Sanitize username to ensure valid git ref (spaces, special chars become hyphens)
            // Note: sanitize_branch_suffix returns "task" for empty input, so we only check for "task"
            let username = sanitize_branch_suffix(&raw_username);
            let suffix = sanitize_branch_suffix(&metadata.branch_suffix);

            // Skip branch rename if suffix is just the fallback "task"
            // (this can happen with non-ASCII only input or empty AI response)
            if suffix == "task" {
                tracing::debug!(
                    suffix = %suffix,
                    "Skipping branch rename: sanitized suffix is generic fallback"
                );
                None
            } else {
                // If username sanitizes to fallback, drop the prefix and use the suffix alone.
                // (Suffix is already sanitized to ASCII kebab-case with no slashes.)
                let new_branch_name = if username == "task" {
                    tracing::debug!(
                        raw_username = %raw_username,
                        sanitized = %username,
                        "Username unusable; generating branch without username prefix"
                    );
                    suffix.clone()
                } else {
                    format!("{}/{}", username, suffix)
                };

                // Only rename if the new name differs from current
                if new_branch_name != resolved_branch {
                    let wd = working_dir.clone();
                    let old = resolved_branch.clone();
                    let new_name = new_branch_name.clone();
                    let wm = worktree_manager.clone();

                    // Capture full error result instead of just is_ok()
                    // Branch rename is best-effort: join errors shouldn't prevent applying the title
                    let rename_join_result = tokio::task::spawn_blocking(move || {
                        wm.rename_branch(&wd, &old, &new_name)
                            .map_err(|e| e.to_string())
                    })
                    .await;

                    match rename_join_result {
                        Ok(Ok(())) => {
                            // Update database if rename succeeded
                            if let (Some(ws_id), Some(ref dao)) = (workspace_id, &workspace_dao) {
                                let db_update_result = tokio::task::spawn_blocking({
                                    let dao = dao.clone();
                                    let new_branch = new_branch_name.clone();
                                    move || {
                                        if let Ok(Some(mut ws)) = dao.get_by_id(ws_id) {
                                            ws.branch = new_branch.clone();
                                            dao.update(&ws).map_err(|e| {
                                                format!(
                                                    "Failed to update workspace branch to {}: {}",
                                                    new_branch, e
                                                )
                                            })
                                        } else {
                                            Err(format!(
                                                "Workspace {} not found for branch update",
                                                ws_id
                                            ))
                                        }
                                    }
                                })
                                .await;

                                // Log any errors from the DB update (don't fail the whole operation)
                                match db_update_result {
                                    Ok(Ok(())) => {}
                                    Ok(Err(e)) => {
                                        tracing::warn!(
                                            error = %e,
                                            workspace_id = %ws_id,
                                            "Failed to persist branch rename to database"
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            error = %e,
                                            workspace_id = %ws_id,
                                            "spawn_blocking failed for database update"
                                        );
                                    }
                                }
                            }
                            Some(new_branch_name)
                        }
                        Ok(Err(e)) => {
                            tracing::warn!(
                                error = %e,
                                old_branch = %resolved_branch,
                                new_branch = %new_branch_name,
                                "Failed to rename git branch"
                            );
                            None
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                old_branch = %resolved_branch,
                                new_branch = %new_branch_name,
                                "spawn_blocking join failed during branch rename"
                            );
                            None
                        }
                    }
                } else {
                    None
                }
            }
        }
    } else {
        None
    };

    Ok(TitleGeneratedResult {
        title: app_prompt::sanitize_title(&metadata.title),
        new_branch,
        workspace_id,
        tool_used: metadata.tool_used.clone(),
        used_fallback: metadata.used_fallback,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::MessageRole;
    use crate::session::AgentSession;
    use chrono::Utc;
    use conduit_agent::events::{AssistantMessageEvent, ReasoningEvent, TurnCompletedEvent};
    use conduit_agent::{AgentType, ModelRegistry, ReasoningEffort, SessionId, TokenUsage};
    use conduit_config::Config;
    use conduit_data::{QueuedMessage, QueuedMessageMode};
    use conduit_util::{Tool, ToolAvailability};
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::{Arc, OnceLock};
    use tokio::sync::mpsc;
    use uuid::Uuid;

    fn init_test_data_dir() -> PathBuf {
        static TEST_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
        TEST_DATA_DIR
            .get_or_init(|| {
                let dir = tempfile::Builder::new()
                    .prefix("conduit-test-data-")
                    .tempdir()
                    .expect("Failed to create test data dir");
                let path = dir.path().to_path_buf();
                // Keep temp dir alive for test process lifetime.
                std::mem::forget(dir);
                conduit_util::init_data_dir(Some(path.clone()));
                path
            })
            .clone()
    }

    fn build_test_app_with_sessions(session_ids: &[Uuid]) -> App {
        init_test_data_dir();
        let config = Config::default();
        let tools = ToolAvailability::default();
        let core = conduit_core::ConduitCore::new(config, tools);
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let mut state = AppState::new(10);

        for session_id in session_ids {
            let mut session = AgentSession::new(AgentType::Codex);
            session.id = *session_id;
            state.tab_manager.add_session(session);
        }

        App {
            core,
            state,
            event_tx,
            event_rx,
            git_tracker: None,
            demo_mode: false,
        }
    }

    fn create_test_file(contents: &str) -> PathBuf {
        let path = init_test_data_dir().join(format!("file-viewer-{}.txt", Uuid::new_v4()));
        std::fs::write(&path, contents).expect("failed to write test file");
        path
    }

    fn create_test_markdown_file(contents: &str) -> PathBuf {
        let path = init_test_data_dir().join(format!("file-viewer-{}.md", Uuid::new_v4()));
        std::fs::write(&path, contents).expect("failed to write markdown test file");
        path
    }

    fn status_bar_model_click_position(app: &App, status_bar_area: Rect) -> (u16, u16) {
        let session = app
            .state
            .tab_manager
            .active_session()
            .expect("session missing");

        let show_mode = session.capabilities.supports_plan_mode;
        let mode_width = if show_mode {
            session.agent_mode.display_name().len()
        } else {
            0
        };
        let model_id = session
            .model
            .clone()
            .unwrap_or_else(|| ModelRegistry::default_model(session.agent_type));
        let model_display = ModelRegistry::find_model(session.agent_type, &model_id)
            .map(|m| m.display_name.to_string())
            .unwrap_or(model_id);
        let model_width = model_display.len();
        let agent_width = session.agent_type.display_name().len();

        let leading: usize = 2;
        let relative_x = if show_mode {
            let model_start = leading + mode_width + 2 - 1;
            let model_end = leading + mode_width + 2 + model_width + 1 + agent_width + 1;
            (model_start + model_end) / 2
        } else {
            let model_start = leading.saturating_sub(1);
            let model_end = leading + model_width + 1 + agent_width + 1;
            (model_start + model_end) / 2
        };

        (status_bar_area.x + relative_x as u16, status_bar_area.y)
    }

    #[test]
    fn test_apply_session_persistence_report_sets_footer_warning() {
        let mut app = build_test_app_with_sessions(&[]);
        let mut report = SessionPersistenceReport::default();
        report.push("failed to save state".to_string());

        app.apply_session_persistence_report(report);

        assert_eq!(
            app.state.footer_message.as_deref(),
            Some("Warning: some session state could not be saved. Check logs.")
        );
        assert!(app.state.footer_message_expires_at.is_some());
    }

    #[test]
    fn test_handle_open_file_sets_file_viewer_mode() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);
        let path = create_test_file("line-1\nline-2\n");
        let mut effects = Vec::new();

        app.handle_open_file(path.clone(), &mut effects);

        assert!(app.state.tab_manager.active_is_file());
        assert_eq!(app.state.input_mode, InputMode::FileViewer);
        assert_eq!(
            app.state
                .tab_manager
                .active_file_viewer()
                .expect("file viewer missing")
                .file_path,
            path
        );
        assert!(matches!(effects.as_slice(), [Effect::SaveSessionState]));
    }

    #[test]
    fn test_markdown_file_defaults_to_rendered_mode() {
        let mut app = build_test_app_with_sessions(&[]);
        let path = create_test_markdown_file(
            "# Title\nThis markdown paragraph is long enough to wrap in a narrow viewport.\n",
        );
        app.state
            .tab_manager
            .open_file(path)
            .expect("open markdown file");
        app.sync_input_mode_for_active_tab();

        let viewer = app
            .state
            .tab_manager
            .active_file_viewer_mut()
            .expect("file viewer missing");
        assert_eq!(
            viewer.active_view_mode(),
            crate::file_viewer::FileViewMode::Rendered
        );

        viewer.ensure_render_cache(20);
        assert!(viewer.effective_total_lines() >= viewer.total_lines);
    }

    #[test]
    fn test_sync_input_mode_for_active_tab_tracks_file_tab_transitions() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);
        let path = create_test_file("line-1\nline-2\nline-3\n");
        app.state.tab_manager.open_file(path).expect("open file");

        app.sync_input_mode_for_active_tab();
        assert_eq!(app.state.input_mode, InputMode::FileViewer);

        app.state.tab_manager.switch_to(0);
        app.sync_input_mode_for_active_tab();
        assert_eq!(app.state.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_lookup_footer_action_uses_file_viewer_context() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);
        let path = create_test_file("line-1\nline-2\nline-3\n");
        app.state.tab_manager.open_file(path).expect("open file");
        app.sync_input_mode_for_active_tab();

        assert_eq!(app.lookup_footer_action("tab"), Some(Action::NextTab));
        assert_eq!(app.lookup_footer_action("j"), Some(Action::ScrollDown(1)));
        assert_eq!(app.lookup_footer_action("q"), Some(Action::CloseTab));
        assert_eq!(app.lookup_footer_action("esc"), Some(Action::CloseTab));
    }

    #[test]
    fn test_flush_scroll_deltas_scrolls_file_viewer() {
        let mut app = build_test_app_with_sessions(&[]);
        let content = (0..50)
            .map(|i| format!("line-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let path = create_test_file(&content);
        app.state.tab_manager.open_file(path).expect("open file");
        app.sync_input_mode_for_active_tab();

        let mut pending_up = 0usize;
        let mut pending_down = 4usize;
        app.flush_scroll_deltas(&mut pending_up, &mut pending_down);

        assert_eq!(pending_up, 0);
        assert_eq!(pending_down, 0);
        assert_eq!(
            app.state
                .tab_manager
                .active_file_viewer()
                .expect("file viewer missing")
                .scroll_offset,
            4
        );
    }

    #[test]
    fn test_should_handle_as_text_input_false_for_file_viewer_context() {
        let mut app = build_test_app_with_sessions(&[]);
        let path = create_test_file("line-1\nline-2\n");
        app.state.tab_manager.open_file(path).expect("open file");
        app.sync_input_mode_for_active_tab();

        let key = crossterm::event::KeyEvent {
            code: crossterm::event::KeyCode::Char('j'),
            modifiers: crossterm::event::KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        let context = app.key_context_for_active_tab();

        assert_eq!(context, KeyContext::FileViewer);
        assert!(!app.should_handle_as_text_input(&key, context));
    }

    #[test]
    fn test_colon_triggers_command_mode_on_empty_input() {
        // Typing ":" on empty input SHOULD trigger command mode
        let result = App::should_trigger_command_mode(
            KeyCode::Char(':'),
            KeyModifiers::NONE,
            InputMode::Normal,
            true, // input_is_empty
            false,
            false,
        );
        assert!(result, "Colon should trigger command mode on empty input");
    }

    #[test]
    fn test_colon_with_modifiers_does_not_trigger_command_mode() {
        // Typing "Shift+:" should NOT trigger command mode
        let result = App::should_trigger_command_mode(
            KeyCode::Char(':'),
            KeyModifiers::SHIFT,
            InputMode::Normal,
            true,
            false,
            false,
        );
        assert!(
            !result,
            "Colon with modifiers should not trigger command mode"
        );
    }

    /// Test that ":" does NOT trigger command mode when input box has content.
    /// This verifies the fix for the bug where pasting "hello:world" would
    /// incorrectly trigger command mode when the ":" character was encountered.
    #[test]
    fn test_colon_does_not_trigger_command_mode_with_existing_input() {
        // Simulate: user has typed "hello" and now types ":"
        // ":" should be inserted as a regular character, not trigger command mode
        let result = App::should_trigger_command_mode(
            KeyCode::Char(':'),
            KeyModifiers::NONE,
            InputMode::Normal,
            false, // input already has content
            false,
            false,
        );

        assert!(
            !result,
            "Colon should NOT trigger command mode when input has existing content"
        );
    }

    /// Test case: pasting "url:port" pattern should not trigger command mode
    #[test]
    fn test_paste_url_with_port_does_not_trigger_command_mode() {
        // Simulate: user pastes "localhost:8080"
        // After pasting "localhost", the ":" should be inserted, not trigger command mode
        let result = App::should_trigger_command_mode(
            KeyCode::Char(':'),
            KeyModifiers::NONE,
            InputMode::Normal,
            false, // input has content from paste
            false,
            false,
        );

        assert!(
            !result,
            "Pasting 'localhost:8080' should not trigger command mode at ':'"
        );
    }

    #[test]
    fn test_colon_does_not_trigger_in_selecting_model() {
        let result = App::should_trigger_command_mode(
            KeyCode::Char(':'),
            KeyModifiers::NONE,
            InputMode::SelectingModel,
            true,
            false,
            false,
        );

        assert!(
            !result,
            "Colon should NOT trigger command mode while selecting a model"
        );
    }

    #[test]
    fn test_slash_triggers_menu_on_empty_input() {
        let result = App::should_trigger_slash_menu(
            KeyCode::Char('/'),
            KeyModifiers::NONE,
            InputMode::Normal,
            true,
            false,
            false,
            true,
        );
        assert!(result, "Slash should trigger menu on empty input");
    }

    #[test]
    fn test_dollar_triggers_menu_on_empty_input() {
        let result = App::should_trigger_slash_menu(
            KeyCode::Char('$'),
            KeyModifiers::NONE,
            InputMode::Normal,
            true,
            false,
            false,
            true,
        );
        assert!(result, "Dollar should trigger menu on empty input");
    }

    #[test]
    fn test_slash_does_not_trigger_with_existing_input() {
        let result = App::should_trigger_slash_menu(
            KeyCode::Char('/'),
            KeyModifiers::NONE,
            InputMode::Normal,
            false,
            false,
            false,
            true,
        );
        assert!(
            !result,
            "Slash should not trigger menu when input has content"
        );
    }

    #[test]
    fn test_slash_does_not_trigger_without_session() {
        let result = App::should_trigger_slash_menu(
            KeyCode::Char('/'),
            KeyModifiers::NONE,
            InputMode::Normal,
            true,
            false,
            false,
            false,
        );
        assert!(
            !result,
            "Slash should not trigger menu without an active session"
        );
    }

    #[test]
    fn test_slash_command_action_maps_fork_to_fork_session() {
        assert_eq!(
            App::slash_command_action(ConduitCommand::Fork),
            Some(Action::ForkSession)
        );
    }

    #[test]
    fn test_slash_command_action_maps_handoff_when_present() {
        let mut slash_state = crate::components::SlashMenuState::new();
        let entries = CommandResolver::menu_entries(std::path::Path::new("."), AgentType::Codex);
        slash_state.show_with_entries('/', entries);

        let entry = slash_state
            .commands
            .iter()
            .find(|entry| entry.label == "/handoff")
            .expect("Expected /handoff command to be present");
        assert_eq!(
            App::slash_command_action(match entry.kind {
                MenuEntryKind::ConduitCommand(command) => command,
                _ => panic!("expected conduit command"),
            }),
            Some(Action::HandoffSession)
        );
    }

    #[test]
    fn test_first_time_splash_shortcuts_only_active_in_normal_without_overlay() {
        assert!(App::should_handle_first_time_splash_shortcuts(
            true,
            InputMode::Normal,
            false
        ));
        assert!(App::should_handle_first_time_splash_shortcuts(
            true,
            InputMode::Scrolling,
            false
        ));
        assert!(!App::should_handle_first_time_splash_shortcuts(
            true,
            InputMode::SelectingModel,
            false
        ));
        assert!(!App::should_handle_first_time_splash_shortcuts(
            true,
            InputMode::Normal,
            true
        ));
        assert!(!App::should_handle_first_time_splash_shortcuts(
            false,
            InputMode::Normal,
            false
        ));
    }

    #[test]
    fn test_build_fork_seed_prompt_includes_roles() {
        use crate::components::ChatMessage;

        let mut summary = crate::components::TurnSummary::new();
        summary.duration_secs = 12;
        summary.input_tokens = 100;
        summary.output_tokens = 200;

        let messages = vec![
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there"),
            ChatMessage::tool_with_exit("Bash", "ls -la", "file.txt", Some(0)),
            ChatMessage::turn_summary(summary),
        ];

        let prompt = app_prompt::build_fork_seed_prompt(&messages);

        // Check header and structure
        assert!(prompt.contains("[CONDUIT_FORK_SEED]"));
        assert!(prompt.contains("<previous-session-transcript>"));
        assert!(prompt.contains("</previous-session-transcript>"));
        assert!(prompt.contains("[END OF CONTEXT]"));
        assert!(prompt.contains("reply with ONLY"));
        assert!(prompt.contains("Ready"));

        // Check message content
        assert!(prompt.contains("[role=user]"));
        assert!(prompt.contains("[role=assistant]"));
        assert!(prompt.contains("name=\"Bash\""));
        assert!(prompt.contains("args=\"ls -la\""));
        assert!(prompt.contains("exit=0"));
        assert!(prompt.contains("[role=summary]"));
        assert!(prompt.contains("tokens_in=100"));
        assert!(prompt.contains("tokens_out=200"));
    }

    #[test]
    fn test_build_fork_seed_prompt_truncates_large_transcript() {
        use crate::components::ChatMessage;

        let oversized = "a".repeat(app_prompt::MAX_SEED_PROMPT_SIZE + 10_000);
        let messages = vec![ChatMessage::user(oversized)];

        let prompt = app_prompt::build_fork_seed_prompt(&messages);

        assert!(
            prompt.contains("[TRUNCATED: transcript exceeded size limit]"),
            "Expected truncation marker"
        );
        assert!(prompt.contains("[END OF CONTEXT]"));
        assert!(prompt.ends_with("Ready"));
    }

    #[test]
    fn test_strip_image_placeholders_removes_placeholders() {
        let prompt = "Hello [img] world".to_string();
        let placeholders = vec!["[img]".to_string()];

        let cleaned = App::strip_image_placeholders(prompt, &placeholders);

        assert_eq!(cleaned, "Hello  world");
    }

    #[test]
    fn test_build_user_prompt_jsonl_with_no_images() {
        let result = App::build_user_prompt_jsonl("Test prompt", &[]).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(result.trim()).unwrap();

        assert_eq!(parsed["type"], "user");
        assert_eq!(parsed["message"]["role"], "user");

        let content = parsed["message"]["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "Test prompt");
    }

    #[test]
    fn test_build_user_prompt_jsonl_with_missing_images_fallback() {
        // Test with non-existent image paths - should add fallback text blocks
        let images = vec![PathBuf::from("/nonexistent/image.png")];
        let result = App::build_user_prompt_jsonl("Test prompt", &images).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(result.trim()).unwrap();

        let content = parsed["message"]["content"].as_array().unwrap();
        // Should have fallback text for failed image + the prompt text
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "text");
        assert!(content[0]["text"]
            .as_str()
            .unwrap()
            .contains("Failed to load image"));
        assert_eq!(content[1]["type"], "text");
        assert_eq!(content[1]["text"], "Test prompt");
    }

    #[test]
    fn test_build_user_prompt_jsonl_with_real_image() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a minimal valid PNG file (1x1 red pixel)
        let png_data: [u8; 70] = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 dimensions
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, // 8-bit RGB
            0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, // IDAT chunk
            0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0x00, 0x18,
            0xDD, 0x8D, 0xB5, // compressed image data
            0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60,
            0x82, // IEND chunk
        ];

        let mut temp_file = NamedTempFile::with_suffix(".png").unwrap();
        temp_file.write_all(&png_data).unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = App::build_user_prompt_jsonl(
            "What is in this image?",
            std::slice::from_ref(&temp_path),
        )
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(result.trim()).unwrap();

        assert_eq!(parsed["type"], "user");
        assert_eq!(parsed["message"]["role"], "user");

        let content = parsed["message"]["content"].as_array().unwrap();
        // Should have image block + text block
        assert_eq!(content.len(), 2, "Expected 2 content blocks (image + text)");

        // First block should be an image
        assert_eq!(content[0]["type"], "image", "First block should be image");
        assert_eq!(content[0]["source"]["type"], "base64");
        assert_eq!(content[0]["source"]["media_type"], "image/png");
        // Verify base64 data is non-empty
        let base64_data = content[0]["source"]["data"].as_str().unwrap();
        assert!(!base64_data.is_empty(), "base64 data should not be empty");

        // Second block should be text
        assert_eq!(content[1]["type"], "text");
        assert_eq!(content[1]["text"], "What is in this image?");
    }

    #[test]
    fn test_truncate_queue_line_handles_small_widths() {
        assert_eq!(app_queue::truncate_queue_line("abcdef", 4), "a...");
        assert_eq!(app_queue::truncate_queue_line("abcdef", 3), "...");
        assert_eq!(app_queue::truncate_queue_line("abcdef", 2), "..");
        assert_eq!(app_queue::truncate_queue_line("abcdef", 0), "");
    }

    #[test]
    fn test_build_queued_submission_concat_vs_separate() {
        let msg_a = QueuedMessage {
            id: Uuid::new_v4(),
            mode: QueuedMessageMode::FollowUp,
            text: "First".to_string(),
            images: Vec::new(),
            created_at: Utc::now(),
        };
        let msg_b = QueuedMessage {
            id: Uuid::new_v4(),
            mode: QueuedMessageMode::Steer,
            text: "Second".to_string(),
            images: Vec::new(),
            created_at: Utc::now(),
        };

        let (concat, _, _) = app_queue::build_queued_submission(
            &[msg_a.clone(), msg_b.clone()],
            conduit_config::QueueDelivery::Concat,
        );
        let (separate, _, _) = app_queue::build_queued_submission(
            &[msg_a.clone(), msg_b.clone()],
            conduit_config::QueueDelivery::Separate,
        );

        assert_eq!(concat, "First\n\nSecond");
        assert!(separate.contains("[Queued 1 of 2]"));
        assert!(separate.contains("[Queued 2 of 2]"));
    }

    #[test]
    fn test_sanitize_title_collapses_whitespace_and_bounds_length() {
        let title = "  Hello\n\tworld  ".to_string();
        let cleaned = app_prompt::sanitize_title(&title);
        assert_eq!(cleaned, "Hello world");

        let long = "a".repeat(250);
        let bounded = app_prompt::sanitize_title(&long);
        assert!(bounded.chars().count() <= 200);

        let empty = "\n\t\r".to_string();
        let fallback = app_prompt::sanitize_title(&empty);
        assert_eq!(fallback, "Untitled task");
    }

    #[tokio::test]
    async fn test_agent_event_routes_streaming_by_session_id_after_tab_close() {
        let session_a = Uuid::new_v4();
        let session_b = Uuid::new_v4();
        let session_c = Uuid::new_v4();

        let mut app = build_test_app_with_sessions(&[session_a, session_b, session_c]);

        // Close the first tab so indices shift: B -> 0, C -> 1
        assert!(app.state.tab_manager.close_tab(0));
        assert_eq!(
            app.state.tab_manager.session_index_by_id(session_b),
            Some(0)
        );
        assert_eq!(
            app.state.tab_manager.session_index_by_id(session_c),
            Some(1)
        );

        let event = AgentEvent::AssistantMessage(AssistantMessageEvent {
            text: "message for B".to_string(),
            is_final: false,
        });

        app.handle_agent_event(session_b, event).await.unwrap();

        {
            let session = app
                .state
                .tab_manager
                .session_by_id_mut(session_b)
                .expect("session B missing");
            assert_eq!(session.chat_view.streaming_buffer(), Some("message for B"));
            assert!(session.chat_view.messages().is_empty());
        }

        {
            let session = app
                .state
                .tab_manager
                .session_by_id_mut(session_c)
                .expect("session C missing");
            assert!(session.chat_view.streaming_buffer().is_none());
            assert!(session.chat_view.messages().is_empty());
        }
    }

    #[tokio::test]
    async fn test_agent_event_routes_reasoning_by_session_id_after_tab_close() {
        let session_a = Uuid::new_v4();
        let session_b = Uuid::new_v4();
        let session_c = Uuid::new_v4();

        let mut app = build_test_app_with_sessions(&[session_a, session_b, session_c]);

        // Close the first tab so indices shift: B -> 0, C -> 1
        assert!(app.state.tab_manager.close_tab(0));
        assert_eq!(
            app.state.tab_manager.session_index_by_id(session_b),
            Some(0)
        );
        assert_eq!(
            app.state.tab_manager.session_index_by_id(session_c),
            Some(1)
        );

        let event = AgentEvent::AssistantReasoning(ReasoningEvent {
            text: "thinking...".to_string(),
        });

        app.handle_agent_event(session_b, event).await.unwrap();

        {
            let session = app
                .state
                .tab_manager
                .session_by_id_mut(session_b)
                .expect("session B missing");
            assert_eq!(
                session
                    .chat_view
                    .streaming_message_for(MessageRole::Reasoning),
                Some("thinking...")
            );
        }

        {
            let session = app
                .state
                .tab_manager
                .session_by_id_mut(session_c)
                .expect("session C missing");
            assert!(session
                .chat_view
                .streaming_message_for(MessageRole::Reasoning)
                .is_none());
        }
    }

    #[tokio::test]
    async fn test_agent_event_routes_final_by_session_id_after_tab_close() {
        let session_a = Uuid::new_v4();
        let session_b = Uuid::new_v4();
        let session_c = Uuid::new_v4();

        let mut app = build_test_app_with_sessions(&[session_a, session_b, session_c]);

        // Close the first tab so indices shift: B -> 0, C -> 1
        assert!(app.state.tab_manager.close_tab(0));
        assert_eq!(
            app.state.tab_manager.session_index_by_id(session_b),
            Some(0)
        );
        assert_eq!(
            app.state.tab_manager.session_index_by_id(session_c),
            Some(1)
        );

        let event = AgentEvent::AssistantMessage(AssistantMessageEvent {
            text: "message for B".to_string(),
            is_final: true,
        });

        app.handle_agent_event(session_b, event).await.unwrap();

        {
            let session = app
                .state
                .tab_manager
                .session_by_id_mut(session_b)
                .expect("session B missing");
            assert!(session.chat_view.streaming_buffer().is_none());
            let messages = session.chat_view.messages();
            let last = messages.last().expect("missing assistant message");
            assert_eq!(last.role, MessageRole::Assistant);
            assert_eq!(last.content, "message for B");
        }

        {
            let session = app
                .state
                .tab_manager
                .session_by_id_mut(session_c)
                .expect("session C missing");
            assert!(session.chat_view.streaming_buffer().is_none());
            assert!(session.chat_view.messages().is_empty());
        }
    }

    #[tokio::test]
    async fn test_turn_completed_keeps_interactive_input_channel() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);

        let (tx, _rx) = mpsc::channel(1);
        {
            let session = app
                .state
                .tab_manager
                .session_by_id_mut(session_id)
                .expect("session missing");
            session.agent_type = AgentType::Codex;
            session.agent_input_tx = Some(tx);
            session.start_processing();
        }

        app.handle_agent_event(
            session_id,
            AgentEvent::TurnCompleted(TurnCompletedEvent {
                usage: TokenUsage::default(),
            }),
        )
        .await
        .unwrap();

        let session = app
            .state
            .tab_manager
            .session_by_id_mut(session_id)
            .expect("session missing");
        assert!(session.agent_input_tx.is_some());
        assert!(!session.is_processing);
    }

    #[test]
    fn test_submit_prompt_for_tab_does_not_resume_stale_codex_session_without_live_channel() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);
        let cwd = std::env::current_dir().expect("cwd");
        let saved_session = SessionId::from_string("codex-thread-123");
        let default_model = app.config().default_model_for(AgentType::Codex);

        {
            let session = app
                .state
                .tab_manager
                .session_by_id_mut(session_id)
                .expect("session missing");
            session.agent_type = AgentType::Codex;
            session.model = Some(default_model);
            session.model_invalid = false;
            session.working_dir = Some(cwd.clone());
            session.resume_session_id = Some(saved_session);
            session.agent_input_tx = None;
        }

        let effects = app
            .submit_prompt_for_tab(0, "hi".to_string(), vec![], vec![], false, None)
            .expect("submit should succeed");

        let (agent_type, config) = effects
            .iter()
            .find_map(|effect| match effect {
                Effect::StartAgent {
                    agent_type, config, ..
                } => Some((agent_type, config)),
                _ => None,
            })
            .expect("expected StartAgent effect");

        assert_eq!(*agent_type, AgentType::Codex);
        assert!(config.resume_session.is_none());
        assert_eq!(config.prompt, "hi");
    }

    #[test]
    fn test_submit_prompt_for_tab_resumes_live_codex_session_after_turn_completion() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);
        let cwd = std::env::current_dir().expect("cwd");
        let live_session = SessionId::from_string("codex-thread-live");
        let default_model = app.config().default_model_for(AgentType::Codex);

        {
            let session = app
                .state
                .tab_manager
                .session_by_id_mut(session_id)
                .expect("session missing");
            session.agent_type = AgentType::Codex;
            session.model = Some(default_model);
            session.model_invalid = false;
            session.working_dir = Some(cwd.clone());
            session.agent_session_id = Some(live_session.clone());
            session.resume_session_id = Some(SessionId::from_string("historic-session"));
            session.agent_input_tx = None;
        }

        let effects = app
            .submit_prompt_for_tab(0, "hi again".to_string(), vec![], vec![], false, None)
            .expect("submit should succeed");

        let (agent_type, config) = effects
            .iter()
            .find_map(|effect| match effect {
                Effect::StartAgent {
                    agent_type, config, ..
                } => Some((agent_type, config)),
                _ => None,
            })
            .expect("expected StartAgent effect");

        assert_eq!(*agent_type, AgentType::Codex);
        assert_eq!(config.resume_session.as_ref(), Some(&live_session));
        assert_eq!(config.prompt, "hi again");
    }

    #[test]
    fn test_handle_list_action_select_next_for_project_picker() {
        let mut app = build_test_app_with_sessions(&[]);
        app.state.input_mode = InputMode::PickingProject;
        app.state
            .project_picker_state
            .list
            .set_filtered(vec![0, 1, 2]);
        app.state.project_picker_state.list.selected = 0;

        app.handle_list_action(Action::SelectNext);

        assert_eq!(app.state.project_picker_state.list.selected, 1);
    }

    #[test]
    fn test_handle_list_action_page_down_up_for_session_import() {
        let mut app = build_test_app_with_sessions(&[]);
        app.state.input_mode = InputMode::ImportingSession;
        let filtered: Vec<usize> = (0..15).collect();
        app.state.session_import_state.list.set_filtered(filtered);
        app.state.session_import_state.list.selected = 0;

        app.handle_list_action(Action::SelectPageDown);
        assert_eq!(app.state.session_import_state.list.selected, 10);

        app.handle_list_action(Action::SelectPageUp);
        assert_eq!(app.state.session_import_state.list.selected, 0);
    }

    #[test]
    fn test_handle_raw_events_toggle_expand_and_collapse() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);

        {
            let session = app
                .state
                .tab_manager
                .active_session_mut()
                .expect("session missing");
            session.raw_events_view.push_event(
                EventDirection::Received,
                "test_event",
                json!({ "ok": true }),
            );
            assert!(!session.raw_events_view.is_expanded());
        }

        let mut effects = Vec::new();
        app.handle_raw_events_action(Action::RawEventsToggleExpand, &mut effects);
        assert!(app
            .state
            .tab_manager
            .active_session()
            .expect("session missing")
            .raw_events_view
            .is_expanded());

        app.handle_raw_events_action(Action::RawEventsCollapse, &mut effects);
        assert!(!app
            .state
            .tab_manager
            .active_session()
            .expect("session missing")
            .raw_events_view
            .is_expanded());
    }

    #[test]
    fn test_handle_raw_events_copy_selected_json() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);

        {
            let session = app
                .state
                .tab_manager
                .active_session_mut()
                .expect("session missing");
            session.raw_events_view.push_event(
                EventDirection::Received,
                "test_event",
                json!({ "foo": "bar" }),
            );
        }

        let mut effects = Vec::new();
        app.handle_raw_events_action(Action::EventDetailCopy, &mut effects);

        let expected = serde_json::to_string_pretty(&json!({ "foo": "bar" })).unwrap();
        assert!(matches!(
            effects.as_slice(),
            [Effect::CopyToClipboard(content)] if content == &expected
        ));
    }

    #[test]
    fn test_handle_dialog_cancel_keeps_remove_project_preflight_loading_visible() {
        let mut app = build_test_app_with_sessions(&[]);
        let repo_id = Uuid::new_v4();
        app.state.input_mode = InputMode::Confirming;
        app.state
            .confirmation_dialog_state
            .show_loading_with_context(
                "Remove Project",
                "Analyzing project workspaces...",
                Some(ConfirmationContext::RemoveProjectPreflightInProgress { repo_id }),
            );

        app.handle_dialog_action(Action::Cancel);

        assert!(app.state.confirmation_dialog_state.visible);
        assert!(app.state.confirmation_dialog_state.loading);
        assert!(matches!(
            app.state.confirmation_dialog_state.context,
            Some(ConfirmationContext::RemoveProjectPreflightInProgress {
                repo_id: id
            }) if id == repo_id
        ));
        assert_eq!(app.state.input_mode, InputMode::Confirming);
    }

    #[test]
    fn test_handle_dialog_cancel_keeps_fork_preflight_loading_visible() {
        let mut app = build_test_app_with_sessions(&[]);
        let parent_workspace_id = Uuid::new_v4();
        app.state.input_mode = InputMode::Confirming;
        app.state
            .confirmation_dialog_state
            .show_loading_with_context(
                "Fork Session",
                "Analyzing workspace state...",
                Some(ConfirmationContext::ForkSessionPreflightInProgress {
                    parent_workspace_id,
                }),
            );

        app.handle_dialog_action(Action::Cancel);

        assert!(app.state.confirmation_dialog_state.visible);
        assert!(app.state.confirmation_dialog_state.loading);
        assert!(matches!(
            app.state.confirmation_dialog_state.context,
            Some(ConfirmationContext::ForkSessionPreflightInProgress {
                parent_workspace_id: id
            }) if id == parent_workspace_id
        ));
        assert_eq!(app.state.input_mode, InputMode::Confirming);
    }

    #[tokio::test]
    async fn test_remove_project_dialog_preflight_completed_shows_confirmation() {
        let mut app = build_test_app_with_sessions(&[]);
        let repo_id = Uuid::new_v4();
        app.state.input_mode = InputMode::Confirming;
        app.state
            .confirmation_dialog_state
            .show_loading_with_context(
                "Remove Project",
                "Analyzing project workspaces...",
                Some(ConfirmationContext::RemoveProjectPreflightInProgress { repo_id }),
            );

        let event = AppEvent::RemoveProjectDialogPreflightCompleted {
            repo_id,
            result: Ok(RemoveProjectDialogPreflightResult {
                repo_name: "demo-repo".to_string(),
                warnings: vec!["2 workspaces will be archived".to_string()],
                has_dirty: false,
                has_unmerged: false,
                workspace_count: 2,
            }),
        };

        let effects = app.handle_app_event(event).await.unwrap();
        assert!(effects.is_empty());
        assert!(app.state.confirmation_dialog_state.visible);
        assert!(!app.state.confirmation_dialog_state.loading);
        assert_eq!(app.state.input_mode, InputMode::Confirming);
        assert!(matches!(
            app.state.confirmation_dialog_state.context,
            Some(ConfirmationContext::RemoveProject(id)) if id == repo_id
        ));
    }

    #[tokio::test]
    async fn test_fork_session_dialog_preflight_completed_shows_confirmation() {
        let mut app = build_test_app_with_sessions(&[]);
        let parent_workspace_id = Uuid::new_v4();
        app.state.input_mode = InputMode::Confirming;
        app.state
            .confirmation_dialog_state
            .show_loading_with_context(
                "Fork Session",
                "Analyzing workspace state...",
                Some(ConfirmationContext::ForkSessionPreflightInProgress {
                    parent_workspace_id,
                }),
            );
        app.state.pending_fork_request = Some(PendingForkRequest {
            agent_type: AgentType::Codex,
            agent_mode: conduit_agent::AgentMode::Build,
            model: Some("o3".to_string()),
            reasoning_effort: None,
            parent_session_id: None,
            parent_workspace_id,
            seed_prompt: std::sync::Arc::from("seed prompt"),
            token_estimate: 1600,
            context_window: 2000,
            fork_seed_id: None,
        });

        let event = AppEvent::ForkSessionDialogPreflightCompleted {
            parent_workspace_id,
            result: Ok(ForkSessionDialogPreflightResult {
                base_branch: "feature/branch".to_string(),
                dirty_warning: Some("Uncommitted changes detected".to_string()),
            }),
        };

        let effects = app.handle_app_event(event).await.unwrap();
        assert!(effects.is_empty());
        assert!(app.state.confirmation_dialog_state.visible);
        assert!(!app.state.confirmation_dialog_state.loading);
        assert_eq!(app.state.input_mode, InputMode::Confirming);
        assert!(matches!(
            app.state.confirmation_dialog_state.context,
            Some(ConfirmationContext::ForkSession {
                parent_workspace_id: id,
                base_branch
            }) if id == parent_workspace_id && base_branch == "feature/branch"
        ));
    }

    #[tokio::test]
    async fn test_projects_discovered_populates_picker() {
        let mut app = build_test_app_with_sessions(&[]);
        let base_dir = PathBuf::from("/tmp/projects");
        app.state
            .project_picker_state
            .show_loading(base_dir.clone());
        app.state.input_mode = InputMode::PickingProject;

        let event = AppEvent::ProjectsDiscovered {
            base_dir: base_dir.clone(),
            result: Ok(vec![
                ProjectDiscoveryEntry {
                    name: "alpha".to_string(),
                    path: base_dir.join("alpha"),
                },
                ProjectDiscoveryEntry {
                    name: "beta".to_string(),
                    path: base_dir.join("beta"),
                },
            ]),
        };

        let effects = app.handle_app_event(event).await.unwrap();
        assert!(effects.is_empty());
        assert!(app.state.project_picker_state.visible);
        assert!(!app.state.project_picker_state.loading);
        assert_eq!(app.state.project_picker_state.projects.len(), 2);
        assert_eq!(app.state.project_picker_state.projects[0].name, "alpha");
        assert_eq!(app.state.project_picker_state.projects[1].name, "beta");
    }

    #[test]
    fn test_handle_input_edit_backspace_exits_command_mode_when_empty() {
        let mut app = build_test_app_with_sessions(&[]);
        app.state.input_mode = InputMode::Command;
        app.state.command_buffer.clear();

        app.handle_input_edit_action(Action::Backspace);

        assert_eq!(app.state.input_mode, InputMode::Normal);
        assert!(app.state.command_buffer.is_empty());
    }

    #[test]
    fn test_snapshot_session_state_persists_resume_session_id_when_live_id_missing() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);
        let restored_session_id = SessionId::from_string("codex-restored-session");

        {
            let session = app
                .state
                .tab_manager
                .session_by_id_mut(session_id)
                .expect("session missing");
            session.agent_type = AgentType::Codex;
            session.agent_session_id = None;
            session.resume_session_id = Some(restored_session_id.clone());
        }

        let snapshot = app.snapshot_session_state();
        let tab = snapshot.tabs.first().expect("expected saved tab");

        assert_eq!(
            tab.agent_session_id.as_deref(),
            Some(restored_session_id.as_str())
        );
    }

    #[test]
    fn test_handle_input_edit_backspace_exits_command_palette_when_search_empty() {
        let mut app = build_test_app_with_sessions(&[]);
        app.state.command_palette_state.visible = true;
        app.state.command_palette_state.list.search.clear();
        app.state.input_mode = InputMode::CommandPalette;

        app.handle_input_edit_action(Action::Backspace);

        assert_eq!(app.state.input_mode, InputMode::Normal);
        assert!(!app.state.command_palette_state.is_visible());
    }

    #[test]
    fn test_handle_input_edit_backspace_exits_slash_menu_when_search_empty() {
        let mut app = build_test_app_with_sessions(&[]);
        app.state.slash_menu_state.visible = true;
        app.state.slash_menu_state.list.search.clear();
        app.state.input_mode = InputMode::SlashMenu;

        app.handle_input_edit_action(Action::Backspace);

        assert_eq!(app.state.input_mode, InputMode::Normal);
        assert!(!app.state.slash_menu_state.is_visible());
    }

    #[test]
    fn test_handle_input_edit_backspace_keeps_slash_menu_open_when_search_present() {
        let mut app = build_test_app_with_sessions(&[]);
        app.state.slash_menu_state.visible = true;
        app.state.slash_menu_state.list.search.set("a");
        app.state.input_mode = InputMode::SlashMenu;

        app.handle_input_edit_action(Action::Backspace);

        assert_eq!(app.state.input_mode, InputMode::SlashMenu);
        assert!(app.state.slash_menu_state.is_visible());
        assert!(app.state.slash_menu_state.list.search.is_empty());
    }

    #[test]
    fn test_handle_input_edit_move_cursor_up_dequeues_queue() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);
        let queued = QueuedMessage {
            id: Uuid::new_v4(),
            mode: QueuedMessageMode::FollowUp,
            text: "queued message".to_string(),
            images: Vec::new(),
            created_at: Utc::now(),
        };

        {
            let session = app
                .state
                .tab_manager
                .active_session_mut()
                .expect("session missing");
            session.queue_message(queued);
            assert!(session.input_box.input().is_empty());
        }

        app.handle_input_edit_action(Action::MoveCursorUp);

        let session = app
            .state
            .tab_manager
            .active_session()
            .expect("session missing");
        assert_eq!(session.input_box.input(), "queued message");
        assert!(session.queued_messages.is_empty());
    }

    #[test]
    fn test_handle_overlay_show_help() {
        let mut app = build_test_app_with_sessions(&[]);
        app.state.input_mode = InputMode::Normal;
        let mut effects = Vec::new();

        app.handle_overlay_action(Action::ShowHelp, &mut effects)
            .unwrap();

        assert_eq!(app.state.input_mode, InputMode::ShowingHelp);
        assert!(app.state.help_dialog_state.is_visible());
        assert!(effects.is_empty());
    }

    #[test]
    fn test_handle_overlay_open_command_palette() {
        let mut app = build_test_app_with_sessions(&[]);
        app.state.input_mode = InputMode::Normal;
        let mut effects = Vec::new();

        app.handle_overlay_action(Action::OpenCommandPalette, &mut effects)
            .unwrap();

        assert_eq!(app.state.input_mode, InputMode::CommandPalette);
        assert!(app.state.command_palette_state.is_visible());
        assert!(effects.is_empty());
    }

    #[test]
    fn test_handle_overlay_toggle_details() {
        let mut app = build_test_app_with_sessions(&[]);
        app.state.input_mode = InputMode::ShowingError;
        let mut effects = Vec::new();
        app.state
            .error_dialog_state
            .show_with_details("Oops", "Something broke", "trace");
        assert!(!app.state.error_dialog_state.details_expanded);

        app.handle_overlay_action(Action::ToggleDetails, &mut effects)
            .unwrap();

        assert!(app.state.error_dialog_state.details_expanded);
        assert!(effects.is_empty());
    }

    #[tokio::test]
    async fn test_project_removed_errors_keep_error_dialog_focused() {
        let mut app = build_test_app_with_sessions(&[]);
        app.state.input_mode = InputMode::SidebarNavigation;

        let event = AppEvent::ProjectRemoved {
            result: RemoveProjectResult {
                repo_id: Uuid::new_v4(),
                workspace_ids: Vec::new(),
                errors: vec!["Failed to canonicalize workspaces dir: not found".to_string()],
            },
        };

        let effects = app.handle_app_event(event).await.unwrap();
        assert!(effects.is_empty());
        assert!(app.state.error_dialog_state.is_visible());
        assert!(app.state.error_dialog_state.has_details());
        assert_eq!(app.state.input_mode, InputMode::ShowingError);

        let mut effects = Vec::new();
        app.handle_overlay_action(Action::ToggleDetails, &mut effects)
            .unwrap();
        assert!(app.state.error_dialog_state.details_expanded);
        assert!(effects.is_empty());
    }

    #[test]
    fn test_handle_overlay_select_agent_creates_tab() {
        let mut app = build_test_app_with_sessions(&[]);
        app.state.input_mode = InputMode::SelectingAgent;
        app.state.agent_selector_state.show();
        let mut effects = Vec::new();

        app.handle_overlay_action(Action::SelectAgent, &mut effects)
            .unwrap();

        assert_eq!(app.state.input_mode, InputMode::Normal);
        assert!(!app.state.agent_selector_state.is_visible());
        let session = app
            .state
            .tab_manager
            .active_session()
            .expect("session missing");
        assert_eq!(session.agent_type, AgentType::Codex);
        assert!(effects.is_empty());
    }

    #[test]
    fn test_handle_submit_related_action_with_no_session() {
        let mut app = build_test_app_with_sessions(&[]);
        let mut effects = Vec::new();

        app.handle_submit_related_action(Action::Submit, &mut effects)
            .unwrap();

        assert!(effects.is_empty());
    }

    #[test]
    fn test_handle_global_quit_shows_dialog_then_quits_on_second_press() {
        let mut app = build_test_app_with_sessions(&[]);
        let mut effects = Vec::new();

        // First press: should not quit, shows confirmation dialog with Quit selected
        app.handle_global_action(Action::Quit, &mut effects);
        assert!(!app.state.should_quit);
        assert!(effects.is_empty());
        assert_eq!(app.state.input_mode, InputMode::Confirming);
        assert!(app.state.confirmation_dialog_state.is_confirm_selected());
        assert!(matches!(
            app.state.confirmation_dialog_state.context,
            Some(ConfirmationContext::Quit)
        ));

        // Second press (Ctrl+Q again): should quit
        app.handle_global_action(Action::Quit, &mut effects);
        assert!(app.state.should_quit);
        assert!(matches!(effects.as_slice(), [Effect::SaveSessionState]));
    }

    #[test]
    fn test_handle_global_toggle_view_mode() {
        let mut app = build_test_app_with_sessions(&[]);
        let mut effects = Vec::new();

        app.state.view_mode = ViewMode::Chat;
        app.handle_global_action(Action::ToggleViewMode, &mut effects);

        assert_eq!(app.state.view_mode, ViewMode::RawEvents);
        assert!(effects.is_empty());
    }

    #[test]
    fn test_handle_global_handoff_session_opens_model_selector() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);
        let executable = std::env::current_exe().expect("test executable path");
        assert!(app.tools_mut().update_tool(Tool::Codex, executable));
        let mut effects = Vec::new();

        app.handle_global_action(Action::HandoffSession, &mut effects);

        assert!(effects.is_empty());
        assert_eq!(app.state.input_mode, InputMode::SelectingModel);
        assert!(app.state.model_selector_state.is_visible());
        assert_eq!(
            app.state.model_picker_context,
            ModelPickerContext::HandoffSelection
        );
        assert!(app.state.pending_handoff_request.is_some());
    }

    #[tokio::test]
    async fn test_handle_global_fork_session_uses_model_window_and_observed_tokens() {
        let session_id = Uuid::new_v4();
        let parent_workspace_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);
        let observed_key = format!(
            "model_context_window::{}::{}",
            AgentType::Codex.as_str(),
            "gpt-5.3-codex-spark"
        );
        if let Some(store) = app.core.app_state_store() {
            let _ = store.delete(&observed_key);
        }

        {
            let session = app
                .state
                .tab_manager
                .active_session_mut()
                .expect("session missing");
            session.workspace_id = Some(parent_workspace_id);
            session.model = Some("gpt-5.3-codex-spark".to_string());
            session.context_state.current_tokens = 111_000;
        }

        let mut effects = Vec::new();
        app.handle_global_action(Action::ForkSession, &mut effects);

        assert!(effects.is_empty());
        let pending = app
            .state
            .pending_fork_request
            .as_ref()
            .expect("expected pending fork request");
        assert_eq!(
            pending.context_window,
            ModelRegistry::CODEX_GPT53_SPARK_CONTEXT_WINDOW
        );
        assert_eq!(pending.token_estimate, 111_000);
        assert_eq!(pending.model.as_deref(), Some("gpt-5.3-codex-spark"));
    }

    #[test]
    fn test_handle_global_copy_workspace_path() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);
        let mut effects = Vec::new();

        {
            let session = app
                .state
                .tab_manager
                .active_session_mut()
                .expect("session missing");
            session.working_dir = Some(PathBuf::from("workspace"));
        }

        app.handle_global_action(Action::CopyWorkspacePath, &mut effects);

        assert!(matches!(
            effects.as_slice(),
            [Effect::CopyToClipboard(content)] if content == "workspace"
        ));
    }

    #[test]
    fn test_handle_dialog_cancel_clears_command_buffer() {
        let mut app = build_test_app_with_sessions(&[]);
        app.state.input_mode = InputMode::Command;
        app.state.command_buffer = "cmd".to_string();

        app.handle_dialog_action(Action::Cancel);

        assert_eq!(app.state.input_mode, InputMode::Normal);
        assert!(app.state.command_buffer.is_empty());
    }

    #[test]
    fn test_handle_dialog_cancel_selecting_model_clears_pending_handoff() {
        let mut app = build_test_app_with_sessions(&[]);
        app.state.input_mode = InputMode::SelectingModel;
        app.state
            .model_selector_state
            .show(None, app.model_selector_defaults());
        app.state.model_picker_context = ModelPickerContext::HandoffSelection;
        app.state.pending_handoff_request = Some(PendingHandoffRequest {
            source_agent_type: AgentType::Codex,
            agent_mode: AgentMode::Build,
            reasoning_effort: None,
            workspace_id: None,
            working_dir: None,
            project_name: None,
            workspace_name: None,
            branch_name: None,
            pr_number: None,
            handoff_prompt: Arc::from("handoff"),
        });

        app.handle_dialog_action(Action::Cancel);

        assert_eq!(app.state.input_mode, InputMode::Normal);
        assert!(!app.state.model_selector_state.is_visible());
        assert_eq!(
            app.state.model_picker_context,
            ModelPickerContext::SessionSelection
        );
        assert!(app.state.pending_handoff_request.is_none());
    }

    #[test]
    fn test_handle_dialog_add_repository_from_sidebar() {
        let mut app = build_test_app_with_sessions(&[]);
        app.state.input_mode = InputMode::SidebarNavigation;

        app.handle_dialog_action(Action::AddRepository);

        assert_eq!(app.state.input_mode, InputMode::AddingRepository);
        assert!(app.state.add_repo_dialog_state.path.is_visible());
    }

    #[test]
    fn test_handle_confirm_action_hides_error_dialog() {
        let mut app = build_test_app_with_sessions(&[]);
        app.state.input_mode = InputMode::ShowingError;
        app.state.error_dialog_state.show("Error", "Boom");

        let mut effects = Vec::new();
        app.handle_confirm_action(&mut effects).unwrap();

        assert_eq!(app.state.input_mode, InputMode::Normal);
        assert!(!app.state.error_dialog_state.is_visible());
        assert!(effects.is_empty());
    }

    #[test]
    fn test_handle_confirm_action_selecting_agent_creates_tab() {
        let mut app = build_test_app_with_sessions(&[]);
        app.state.input_mode = InputMode::SelectingAgent;
        app.state.agent_selector_state.show();

        let mut effects = Vec::new();
        app.handle_confirm_action(&mut effects).unwrap();

        assert_eq!(app.state.input_mode, InputMode::Normal);
        assert!(!app.state.agent_selector_state.is_visible());
        assert!(app.state.tab_manager.active_session().is_some());
        assert!(effects.is_empty());
    }

    #[test]
    fn test_handle_confirm_action_selecting_model_executes_pending_handoff() {
        let source_session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[source_session_id]);
        let executable = std::env::current_exe().expect("test executable path");
        assert!(app.tools_mut().update_tool(Tool::Codex, executable));
        let workspace_id = Uuid::new_v4();
        let working_dir = std::env::current_dir().expect("cwd");

        app.state.pending_handoff_request = Some(PendingHandoffRequest {
            source_agent_type: AgentType::Codex,
            agent_mode: AgentMode::Build,
            reasoning_effort: None,
            workspace_id: Some(workspace_id),
            working_dir: Some(working_dir.clone()),
            project_name: Some("project-a".to_string()),
            workspace_name: Some("workspace-a".to_string()),
            branch_name: None,
            pr_number: Some(42),
            handoff_prompt: Arc::from("[CONDUIT_HANDOFF]\nReady"),
        });
        app.state.input_mode = InputMode::SelectingModel;
        app.state.model_picker_context = ModelPickerContext::HandoffSelection;
        app.state
            .model_selector_state
            .set_allowed_providers(Some(vec![AgentType::Codex]));
        app.state.model_selector_state.show(
            Some(app.config().default_model_for(AgentType::Codex)),
            app.model_selector_defaults(),
        );

        let mut effects = Vec::new();
        app.handle_confirm_action(&mut effects).unwrap();

        assert!(app.state.pending_handoff_request.is_none());
        assert_eq!(app.state.input_mode, InputMode::Normal);
        assert_eq!(app.state.tab_manager.len(), 2);
        assert!(matches!(
            effects.as_slice(),
            [Effect::StartAgent {
                agent_type: AgentType::Codex,
                ..
            }]
        ));

        let session = app
            .state
            .tab_manager
            .active_session()
            .expect("session missing");
        let expected_model = app.config().default_model_for(AgentType::Codex);
        assert_eq!(session.agent_type, AgentType::Codex);
        assert_eq!(session.workspace_id, Some(workspace_id));
        assert_eq!(session.working_dir.as_ref(), Some(&working_dir));
        assert_eq!(session.project_name.as_deref(), Some("project-a"));
        assert_eq!(session.workspace_name.as_deref(), Some("workspace-a"));
        assert_eq!(session.pr_number, Some(42));
        assert_eq!(session.model.as_deref(), Some(expected_model.as_str()));
        assert!(session.suppress_next_assistant_reply);
        assert!(session.suppress_next_turn_summary);
        assert!(session
            .chat_view
            .messages()
            .iter()
            .any(|message| message.role == MessageRole::System
                && message.content.contains("Handoff context was injected")));
    }

    #[test]
    fn test_execute_handoff_session_fallback_provider_uses_default_model() {
        let source_session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[source_session_id]);
        let executable = std::env::current_exe().expect("test executable path");
        assert!(app.tools_mut().update_tool(Tool::Codex, executable.clone()));
        assert!(app.tools_mut().update_tool(Tool::Claude, executable));
        app.config_mut()
            .set_enabled_providers(vec![AgentType::Codex]);

        app.state.pending_handoff_request = Some(PendingHandoffRequest {
            source_agent_type: AgentType::Codex,
            agent_mode: AgentMode::Build,
            reasoning_effort: None,
            workspace_id: None,
            working_dir: None,
            project_name: None,
            workspace_name: None,
            branch_name: None,
            pr_number: None,
            handoff_prompt: Arc::from("[CONDUIT_HANDOFF]\nReady"),
        });

        let effects = app
            .execute_handoff_session(AgentType::Claude, "opus".to_string())
            .expect("handoff should succeed");

        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::StartAgent {
                agent_type: AgentType::Codex,
                ..
            }
        )));

        let session = app
            .state
            .tab_manager
            .active_session()
            .expect("session missing");
        let expected_model = app.config().default_model_for(AgentType::Codex);
        assert_eq!(session.agent_type, AgentType::Codex);
        assert_eq!(session.model.as_deref(), Some(expected_model.as_str()));
    }

    #[test]
    fn test_handle_confirm_action_selecting_reasoning_sets_effort() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);
        {
            let session = app
                .state
                .tab_manager
                .active_session_mut()
                .expect("session missing");
            session.agent_type = AgentType::Codex;
        }
        app.state
            .reasoning_selector_state
            .show(AgentType::Codex, None);
        app.state.reasoning_selector_state.insert_str("xhigh");
        app.state.input_mode = InputMode::SelectingReasoning;

        let mut effects = Vec::new();
        app.handle_confirm_action(&mut effects).unwrap();

        assert_eq!(app.state.input_mode, InputMode::Normal);
        assert!(!app.state.reasoning_selector_state.is_visible());
        assert!(effects.is_empty());

        let session = app
            .state
            .tab_manager
            .active_session()
            .expect("session missing");
        assert_eq!(session.reasoning_effort, Some(ReasoningEffort::XHigh));
    }

    #[test]
    fn test_handle_confirm_action_model_selector_wins_over_stale_mode() {
        let mut app = build_test_app_with_sessions(&[]);
        let executable = std::env::current_exe().expect("test executable path");
        assert!(app.tools_mut().update_tool(Tool::Claude, executable));
        app.config_mut()
            .set_enabled_providers(vec![AgentType::Claude]);

        app.state.pending_new_project_target = Some(NewProjectTarget::BaseDirDialog);
        app.state.model_picker_context = ModelPickerContext::OnboardingDefaultSelection;
        app.state
            .model_selector_state
            .set_allowed_providers(Some(vec![AgentType::Claude]));
        app.state.model_selector_state.show_with_title(
            None,
            DefaultModelSelection::default(),
            "Pick your default model".to_string(),
        );
        // Simulate stale mode mismatch observed in TUI.
        app.state.input_mode = InputMode::SelectingProviders;

        let mut effects = Vec::new();
        app.handle_confirm_action(&mut effects).unwrap();

        assert!(effects.is_empty());
        assert_eq!(app.config().default_agent, AgentType::Claude);
        assert_eq!(app.config().default_model.as_deref(), Some("opus"));
        assert_eq!(app.state.input_mode, InputMode::SettingBaseDir);
        assert!(!app.state.model_selector_state.is_visible());
        assert!(app.state.pending_new_project_target.is_none());
    }

    #[test]
    fn test_handle_confirm_action_selecting_reasoning_blocked_after_session_started() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);
        {
            let session = app
                .state
                .tab_manager
                .active_session_mut()
                .expect("session missing");
            session.turn_count = 1;
        }
        app.state
            .reasoning_selector_state
            .show(AgentType::Codex, None);
        app.state.reasoning_selector_state.insert_str("low");
        app.state.input_mode = InputMode::SelectingReasoning;

        let mut effects = Vec::new();
        app.handle_confirm_action(&mut effects).unwrap();

        assert_eq!(app.state.input_mode, InputMode::SelectingReasoning);
        assert!(app.state.reasoning_selector_state.is_visible());
        assert!(effects.is_empty());

        let session = app
            .state
            .tab_manager
            .active_session()
            .expect("session missing");
        assert_eq!(session.reasoning_effort, None);
        let last = session
            .chat_view
            .messages()
            .last()
            .expect("expected error message");
        assert_eq!(last.role, MessageRole::Error);
        assert!(last
            .content
            .contains("Changing reasoning effort after a session has started"));
    }

    #[test]
    fn test_handle_confirm_action_blocks_cross_agent_switch_after_session_started() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);
        let executable = std::env::current_exe().expect("test executable path");
        assert!(app.tools_mut().update_tool(Tool::Claude, executable));

        {
            let session = app
                .state
                .tab_manager
                .active_session_mut()
                .expect("session missing");
            session.turn_count = 1;
        }

        app.state
            .model_selector_state
            .show(None, DefaultModelSelection::default());
        app.state.model_selector_state.insert_str("Opus 4.7");
        app.state.input_mode = InputMode::SelectingModel;

        let mut effects = Vec::new();
        app.handle_confirm_action(&mut effects).unwrap();

        assert_eq!(app.state.input_mode, InputMode::SelectingModel);
        assert!(app.state.model_selector_state.is_visible());
        assert!(effects.is_empty());

        let session = app
            .state
            .tab_manager
            .active_session()
            .expect("session missing");
        assert_eq!(session.agent_type, AgentType::Codex);
        let last = session
            .chat_view
            .messages()
            .last()
            .expect("expected error message");
        assert_eq!(last.role, MessageRole::Error);
        assert!(last
            .content
            .contains("Switching agent type after a session has started"));
    }

    #[test]
    fn test_handle_status_bar_click_model_picker_respects_enabled_providers() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);
        let executable = std::env::current_exe().expect("test executable path");
        assert!(app.tools_mut().update_tool(Tool::Codex, executable.clone()));
        assert!(app.tools_mut().update_tool(Tool::Claude, executable));
        app.config_mut()
            .set_enabled_providers(vec![AgentType::Codex]);

        let status_bar_area = Rect {
            x: 0,
            y: 0,
            width: 120,
            height: 1,
        };
        let (x, y) = status_bar_model_click_position(&app, status_bar_area);

        let _ = app.handle_status_bar_click(x, y, status_bar_area);

        assert_eq!(app.state.input_mode, InputMode::SelectingModel);
        assert!(app.state.model_selector_state.is_visible());

        app.state.model_selector_state.insert_str("Opus 4.7");
        assert!(
            app.state.model_selector_state.selected_model().is_none(),
            "Claude model should be filtered out when only Codex is enabled"
        );
    }

    #[test]
    fn test_handle_model_selector_click_executes_pending_handoff() {
        let source_session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[source_session_id]);
        let executable = std::env::current_exe().expect("test executable path");
        assert!(app.tools_mut().update_tool(Tool::Codex, executable.clone()));
        assert!(app.tools_mut().update_tool(Tool::Claude, executable));
        app.config_mut()
            .set_enabled_providers(vec![AgentType::Codex, AgentType::Claude]);

        app.state.pending_handoff_request = Some(PendingHandoffRequest {
            source_agent_type: AgentType::Codex,
            agent_mode: AgentMode::Build,
            reasoning_effort: None,
            workspace_id: None,
            working_dir: None,
            project_name: None,
            workspace_name: None,
            branch_name: None,
            pr_number: None,
            handoff_prompt: Arc::from("[CONDUIT_HANDOFF]\nReady"),
        });
        app.state.input_mode = InputMode::SelectingModel;
        app.state.model_picker_context = ModelPickerContext::HandoffSelection;
        app.state
            .model_selector_state
            .set_allowed_providers(Some(vec![AgentType::Codex, AgentType::Claude]));
        app.state
            .model_selector_state
            .show(None, app.model_selector_defaults());
        app.state.model_selector_state.insert_str("Opus 4.7");

        let terminal_size = crossterm::terminal::size().unwrap_or((80, 24));
        let dialog_width = 60u16.min(terminal_size.0.saturating_sub(4));
        let dialog_height = 18u16.min(terminal_size.1.saturating_sub(2));
        let dialog_x = (terminal_size.0.saturating_sub(dialog_width)) / 2;
        let dialog_y = (terminal_size.1.saturating_sub(dialog_height)) / 2;
        let inner = dialog_content_area(Rect {
            x: dialog_x,
            y: dialog_y,
            width: dialog_width,
            height: dialog_height,
        });
        let x = inner.x + 1;
        let y = inner.y + 3;

        let effects = app.handle_model_selector_click(x, y);

        assert_eq!(app.state.input_mode, InputMode::Normal);
        assert!(!app.state.model_selector_state.is_visible());
        assert_eq!(
            app.state.model_picker_context,
            ModelPickerContext::SessionSelection
        );
        assert!(app.state.pending_handoff_request.is_none());
        assert_eq!(app.state.tab_manager.len(), 2);
        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::StartAgent {
                agent_type: AgentType::Claude,
                ..
            }
        )));

        let source = app
            .state
            .tab_manager
            .session(0)
            .expect("source session missing");
        assert_eq!(source.agent_type, AgentType::Codex);

        let target = app
            .state
            .tab_manager
            .active_session()
            .expect("target session missing");
        assert_eq!(target.agent_type, AgentType::Claude);
    }

    #[test]
    fn test_handle_model_selector_click_blocks_cross_agent_switch_after_session_started() {
        let session_id = Uuid::new_v4();
        let mut app = build_test_app_with_sessions(&[session_id]);
        let executable = std::env::current_exe().expect("test executable path");
        assert!(app.tools_mut().update_tool(Tool::Claude, executable));

        {
            let session = app
                .state
                .tab_manager
                .active_session_mut()
                .expect("session missing");
            session.turn_count = 1;
        }

        app.state
            .model_selector_state
            .show(None, DefaultModelSelection::default());
        app.state.model_selector_state.insert_str("Opus 4.7");
        app.state.input_mode = InputMode::SelectingModel;

        let terminal_size = crossterm::terminal::size().unwrap_or((80, 24));
        let dialog_width = 60u16.min(terminal_size.0.saturating_sub(4));
        let dialog_height = 18u16.min(terminal_size.1.saturating_sub(2));
        let dialog_x = (terminal_size.0.saturating_sub(dialog_width)) / 2;
        let dialog_y = (terminal_size.1.saturating_sub(dialog_height)) / 2;
        let inner = dialog_content_area(Rect {
            x: dialog_x,
            y: dialog_y,
            width: dialog_width,
            height: dialog_height,
        });
        let x = inner.x + 1;
        let y = inner.y + 3;

        assert!(app.handle_model_selector_click(x, y).is_empty());

        assert_eq!(app.state.input_mode, InputMode::SelectingModel);
        assert!(app.state.model_selector_state.is_visible());

        let session = app
            .state
            .tab_manager
            .active_session()
            .expect("session missing");
        assert_eq!(session.agent_type, AgentType::Codex);
        let last = session
            .chat_view
            .messages()
            .last()
            .expect("expected error message");
        assert_eq!(last.role, MessageRole::Error);
        assert!(last
            .content
            .contains("Switching agent type after a session has started"));
    }
}
