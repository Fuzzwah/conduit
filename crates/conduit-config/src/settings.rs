use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use toml_edit::{Array, DocumentMut, Item, Table};

use conduit_agent::{AgentType, ModelRegistry};
use conduit_git::WorkspaceMode;
use conduit_types::Action;
use conduit_util::paths::config_path;
use conduit_util::tools::{Tool, ToolAvailability, ToolPaths};

use super::default_keys::default_keybindings;
use super::keys::{parse_key_notation, KeyContext, KeybindingConfig};

/// Example configuration file contents (bundled with the binary)
pub const EXAMPLE_CONFIG: &str = include_str!("config.toml.example");

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Default agent type for new sessions
    pub default_agent: AgentType,
    /// Default model ID for the default agent
    pub default_model: Option<String>,
    /// Explicitly enabled providers (None = not configured yet)
    pub enabled_providers: Option<Vec<AgentType>>,
    /// Working directory for agent operations
    pub working_dir: PathBuf,
    /// Maximum number of tabs allowed
    pub max_tabs: usize,
    /// Show token usage in status bar
    pub show_token_usage: bool,
    /// Show estimated cost in status bar
    pub show_cost: bool,
    /// Default allowed tools for Claude
    pub claude_allowed_tools: Vec<String>,
    /// Claude model pricing (input tokens per $1M)
    pub claude_input_cost_per_million: f64,
    /// Claude model pricing (output tokens per $1M)
    pub claude_output_cost_per_million: f64,
    /// Keybinding configuration
    pub keybindings: KeybindingConfig,
    /// Configured paths for external tools (git, gh, claude, codex, gemini)
    pub tool_paths: ToolPaths,
    /// Theme name from config (None = use default)
    pub theme_name: Option<String>,
    /// Custom theme path from config (takes precedence over name)
    pub theme_path: Option<PathBuf>,
    /// Queue configuration
    pub queue: QueueConfig,
    /// Steering configuration
    pub steer: SteerConfig,
    /// Selection and clipboard configuration
    pub selection: SelectionConfig,
    /// UI configuration
    pub ui: UiConfig,
    /// Web workspace status configuration
    pub web_status: WebStatusConfig,
    /// Workspace defaults
    pub workspaces: WorkspacesConfig,
    /// Remote issue provider configuration
    pub issues: IssuesConfig,
}

pub use conduit_git::IssuesConfig;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TomlIssuesConfig {
    pub gitea_hosts: Option<Vec<String>>,
    pub forgejo_hosts: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum QueueDelivery {
    Separate,
    Concat,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum QueueMode {
    All,
    OneAtATime,
}

#[derive(Debug, Clone, Copy)]
pub struct QueueConfig {
    pub delivery: QueueDelivery,
    pub mode: QueueMode,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TomlQueueConfig {
    pub delivery: Option<QueueDelivery>,
    pub mode: Option<QueueMode>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SteerBehavior {
    Hard,
    Soft,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SteerFallback {
    Queue,
    Interrupt,
    Prompt,
}

#[derive(Debug, Clone, Copy)]
pub struct SteerConfig {
    pub behavior: SteerBehavior,
    pub fallback: SteerFallback,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TomlSteerConfig {
    pub behavior: Option<SteerBehavior>,
    pub fallback: Option<SteerFallback>,
}

#[derive(Debug, Clone, Copy)]
pub struct SelectionConfig {
    pub auto_copy_selection: bool,
    pub clear_selection_after_copy: bool,
}

/// Spinner animation style for the thinking indicator
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ThinkingSpinnerStyle {
    /// Claude Code-style rotating star frames (·, ✢, ✳, ∗, ✻, ✽)
    #[default]
    Star,
    /// Braille dot spinner (⣾, ⣽, ⣻, ⢿, ⡿, ⣟, ⣯, ⣷)
    Braille,
}

#[derive(Debug, Clone)]
pub struct UiConfig {
    pub show_chat_scrollbar: bool,
    /// Keep the sidebar always visible; Ctrl+T toggles focus instead of show/hide
    pub always_show_sidebar: bool,
    /// Include the sidebar as a stop in the Alt+Tab workspace cycle (default: true)
    pub sidebar_in_tab_cycle: bool,
    /// Show animated shimmer on the thinking indicator (default: true)
    pub thinking_indicator_shimmer: bool,
    /// Label shown in the thinking indicator (default: "Working")
    pub thinking_indicator_label: String,
    /// Spinner animation style: Star (·✻ Claude Code style) or Braille (⣾ dot style)
    pub thinking_indicator_spinner: ThinkingSpinnerStyle,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TomlSelectionConfig {
    pub auto_copy_selection: Option<bool>,
    pub clear_selection_after_copy: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TomlUiConfig {
    pub show_chat_scrollbar: Option<bool>,
    pub always_show_sidebar: Option<bool>,
    pub sidebar_in_tab_cycle: Option<bool>,
    pub thinking_indicator_shimmer: Option<bool>,
    pub thinking_indicator_label: Option<String>,
    pub thinking_indicator_spinner: Option<ThinkingSpinnerStyle>,
}

#[derive(Debug, Clone, Copy)]
pub struct WebStatusConfig {
    pub initial_scan: bool,
    pub status_scan_concurrency: usize,
    pub selected_refresh_interval_ms: u64,
    pub pr_refresh_interval_ms: u64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TomlWebStatusConfig {
    pub initial_scan: Option<bool>,
    pub status_scan_concurrency: Option<usize>,
    pub selected_refresh_interval_ms: Option<u64>,
    pub pr_refresh_interval_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
pub struct WorkspacesConfig {
    pub default_mode: WorkspaceMode,
    pub archive_delete_branch: bool,
    pub archive_remote_prompt: bool,
    pub use_gh_cli_merge_status: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TomlWorkspacesConfig {
    pub mode: Option<WorkspaceMode>,
    pub archive_delete_branch: Option<bool>,
    pub archive_remote_prompt: Option<bool>,
    pub use_gh_cli_merge_status: Option<bool>,
}

/// TOML representation of default model
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TomlDefaultModelConfig {
    pub agent: Option<String>,
    pub model: Option<String>,
}

/// TOML representation of provider selection configuration
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TomlProvidersConfig {
    pub enabled: Option<Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_agent: AgentType::Codex,
            default_model: None,
            enabled_providers: None,
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            max_tabs: 10,
            show_token_usage: true,
            show_cost: true,
            claude_allowed_tools: vec![
                "Read".into(),
                "Edit".into(),
                "Write".into(),
                "Bash".into(),
                "Glob".into(),
                "Grep".into(),
            ],
            // Claude Sonnet 3.5 pricing
            claude_input_cost_per_million: 3.0,
            claude_output_cost_per_million: 15.0,
            keybindings: default_keybindings(),
            tool_paths: ToolPaths::default(),
            theme_name: None,
            theme_path: None,
            queue: QueueConfig {
                delivery: QueueDelivery::Separate,
                mode: QueueMode::OneAtATime,
            },
            steer: SteerConfig {
                behavior: SteerBehavior::Hard,
                fallback: SteerFallback::Queue,
            },
            selection: SelectionConfig {
                auto_copy_selection: true,
                clear_selection_after_copy: true,
            },
            ui: UiConfig {
                show_chat_scrollbar: false,
                always_show_sidebar: false,
                sidebar_in_tab_cycle: true,
                thinking_indicator_shimmer: true,
                thinking_indicator_label: "Working".to_string(),
                thinking_indicator_spinner: ThinkingSpinnerStyle::Star,
            },
            web_status: WebStatusConfig {
                initial_scan: true,
                status_scan_concurrency: 2,
                selected_refresh_interval_ms: 5000,
                pr_refresh_interval_ms: 60000,
            },
            workspaces: WorkspacesConfig {
                default_mode: WorkspaceMode::Worktree,
                archive_delete_branch: true,
                archive_remote_prompt: true,
                use_gh_cli_merge_status: false,
            },
            issues: IssuesConfig::default(),
        }
    }
}

/// TOML representation of keybinding configuration
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TomlKeybindings {
    /// Global keybindings (apply to all contexts)
    #[serde(flatten)]
    pub global: HashMap<String, String>,

    /// Chat mode keybindings
    pub chat: Option<HashMap<String, String>>,

    /// Scrolling mode keybindings
    pub scrolling: Option<HashMap<String, String>>,

    /// Sidebar keybindings
    pub sidebar: Option<HashMap<String, String>>,

    /// Dialog keybindings
    pub dialog: Option<HashMap<String, String>>,

    /// Project picker keybindings
    pub project_picker: Option<HashMap<String, String>>,

    /// Model selector keybindings
    pub model_selector: Option<HashMap<String, String>>,

    /// Add repository dialog keybindings
    pub add_repository: Option<HashMap<String, String>>,

    /// Base directory dialog keybindings
    pub base_dir: Option<HashMap<String, String>>,

    /// Raw events view keybindings
    pub raw_events: Option<HashMap<String, String>>,

    /// Queue editor keybindings
    pub queue: Option<HashMap<String, String>>,
}

/// TOML representation of theme configuration
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TomlThemeConfig {
    /// Theme name (built-in or VS Code theme label/extension ID)
    pub name: Option<String>,
    /// Direct path to VS Code theme JSON file
    pub path: Option<PathBuf>,
}

/// TOML representation of the config file
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TomlConfig {
    /// Default model configuration
    pub model: Option<TomlDefaultModelConfig>,
    /// Enabled provider configuration
    pub providers: Option<TomlProvidersConfig>,
    /// Keybinding configuration
    pub keys: Option<TomlKeybindings>,
    /// Tool path configuration
    pub tools: Option<ToolPaths>,
    /// Theme configuration
    pub theme: Option<TomlThemeConfig>,
    /// Queue configuration
    pub queue: Option<TomlQueueConfig>,
    /// Steering configuration
    pub steer: Option<TomlSteerConfig>,
    /// Selection configuration
    pub selection: Option<TomlSelectionConfig>,
    /// UI configuration
    pub ui: Option<TomlUiConfig>,
    /// Web status configuration
    pub web_status: Option<TomlWebStatusConfig>,
    /// Workspace defaults
    pub workspaces: Option<TomlWorkspacesConfig>,
    /// Remote issue provider configuration
    pub issues: Option<TomlIssuesConfig>,
}

impl TomlKeybindings {
    /// Convert TOML keybindings to KeybindingConfig
    fn to_keybinding_config(&self) -> KeybindingConfig {
        let mut config = KeybindingConfig::new();

        // Parse global bindings
        for (action_name, key_str) in &self.global {
            // Skip context sections (they're handled separately)
            if matches!(
                action_name.as_str(),
                "chat"
                    | "scrolling"
                    | "sidebar"
                    | "dialog"
                    | "project_picker"
                    | "model_selector"
                    | "add_repository"
                    | "base_dir"
                    | "raw_events"
                    | "queue"
            ) {
                continue;
            }

            if let (Ok(combo), Some(action)) =
                (parse_key_notation(key_str), parse_action(action_name))
            {
                config.global.insert(combo, action);
            }
        }

        // Parse context-specific bindings
        if let Some(chat) = &self.chat {
            parse_context_bindings(&mut config, KeyContext::Chat, chat);
        }
        if let Some(scrolling) = &self.scrolling {
            parse_context_bindings(&mut config, KeyContext::Scrolling, scrolling);
        }
        if let Some(sidebar) = &self.sidebar {
            parse_context_bindings(&mut config, KeyContext::Sidebar, sidebar);
        }
        if let Some(dialog) = &self.dialog {
            parse_context_bindings(&mut config, KeyContext::Dialog, dialog);
        }
        if let Some(picker) = &self.project_picker {
            parse_context_bindings(&mut config, KeyContext::ProjectPicker, picker);
        }
        if let Some(model) = &self.model_selector {
            parse_context_bindings(&mut config, KeyContext::ModelSelector, model);
        }
        if let Some(add_repo) = &self.add_repository {
            parse_context_bindings(&mut config, KeyContext::AddRepository, add_repo);
        }
        if let Some(base_dir) = &self.base_dir {
            parse_context_bindings(&mut config, KeyContext::BaseDir, base_dir);
        }
        if let Some(raw) = &self.raw_events {
            parse_context_bindings(&mut config, KeyContext::RawEvents, raw);
        }
        if let Some(queue) = &self.queue {
            parse_context_bindings(&mut config, KeyContext::QueueEditing, queue);
        }

        config
    }
}

/// Parse context-specific keybindings
fn parse_context_bindings(
    config: &mut KeybindingConfig,
    context: KeyContext,
    bindings: &HashMap<String, String>,
) {
    let context_map = config.context.entry(context).or_default();
    for (action_name, key_str) in bindings {
        if let (Ok(combo), Some(action)) = (parse_key_notation(key_str), parse_action(action_name))
        {
            context_map.insert(combo, action);
        }
    }
}

/// Parse an action name string into an Action
pub fn parse_action(name: &str) -> Option<Action> {
    match name {
        // Global
        "quit" => Some(Action::Quit),
        "toggle_sidebar" => Some(Action::ToggleSidebar),
        "hide_sidebar" => Some(Action::HideSidebar),
        "new_project" => Some(Action::NewProject),
        "open_pr" => Some(Action::OpenPr),
        "fork_session" => Some(Action::ForkSession),
        "handoff_session" => Some(Action::HandoffSession),
        "interrupt_agent" => Some(Action::InterruptAgent),
        "toggle_view_mode" => Some(Action::ToggleViewMode),
        "show_model_selector" => Some(Action::ShowModelSelector),
        "show_reasoning_selector" => Some(Action::ShowReasoningSelector),
        "show_theme_picker" => Some(Action::ShowThemePicker),
        "show_providers_selector" => Some(Action::ShowProvidersSelector),
        "toggle_metrics" => Some(Action::ToggleMetrics),
        "dump_debug_state" => Some(Action::DumpDebugState),
        "suspend" => Some(Action::Suspend),
        "copy_selection" => Some(Action::CopySelection),

        // Tab management
        "close_tab" => Some(Action::CloseTab),
        "next_tab" => Some(Action::NextTab),
        "prev_tab" => Some(Action::PrevTab),
        "move_tab_left" => Some(Action::MoveTabLeft),
        "move_tab_right" => Some(Action::MoveTabRight),

        // Scrolling
        "scroll_up" => Some(Action::ScrollUp(1)),
        "scroll_down" => Some(Action::ScrollDown(1)),
        "scroll_page_up" => Some(Action::ScrollPageUp),
        "scroll_page_down" => Some(Action::ScrollPageDown),
        "scroll_to_top" => Some(Action::ScrollToTop),
        "scroll_to_bottom" => Some(Action::ScrollToBottom),
        "prev_user_message" => Some(Action::ScrollPrevUserMessage),
        "next_user_message" => Some(Action::ScrollNextUserMessage),

        // Input editing
        "insert_newline" => Some(Action::InsertNewline),
        "backspace" => Some(Action::Backspace),
        "delete" => Some(Action::Delete),
        "delete_word_back" => Some(Action::DeleteWordBack),
        "delete_word_forward" => Some(Action::DeleteWordForward),
        "delete_to_start" => Some(Action::DeleteToStart),
        "delete_to_end" => Some(Action::DeleteToEnd),
        "move_cursor_left" => Some(Action::MoveCursorLeft),
        "move_cursor_right" => Some(Action::MoveCursorRight),
        "move_cursor_start" => Some(Action::MoveCursorStart),
        "move_cursor_end" => Some(Action::MoveCursorEnd),
        "move_word_left" => Some(Action::MoveWordLeft),
        "move_word_right" => Some(Action::MoveWordRight),
        "move_cursor_up" => Some(Action::MoveCursorUp),
        "move_cursor_down" => Some(Action::MoveCursorDown),
        "history_prev" => Some(Action::HistoryPrev),
        "history_next" => Some(Action::HistoryNext),
        "submit" => Some(Action::Submit),
        "submit_steer" => Some(Action::SubmitSteer),
        "open_queue_editor" => Some(Action::OpenQueueEditor),
        "close_queue_editor" => Some(Action::CloseQueueEditor),
        "queue_move_up" => Some(Action::QueueMoveUp),
        "queue_move_down" => Some(Action::QueueMoveDown),
        "queue_edit" => Some(Action::QueueEdit),
        "queue_delete" => Some(Action::QueueDelete),
        "edit_prompt_external" => Some(Action::EditPromptExternal),

        // Navigation
        "select_next" => Some(Action::SelectNext),
        "select_prev" => Some(Action::SelectPrev),
        "select_page_down" => Some(Action::SelectPageDown),
        "select_page_up" => Some(Action::SelectPageUp),
        "confirm" => Some(Action::Confirm),
        "cancel" => Some(Action::Cancel),
        "set_default_model" => Some(Action::SetDefaultModel),
        "expand_or_select" => Some(Action::ExpandOrSelect),
        "collapse" => Some(Action::Collapse),
        "add_repository" => Some(Action::AddRepository),
        "open_settings" => Some(Action::OpenSettings),
        "archive_or_remove" => Some(Action::ArchiveOrRemove),
        "project_move_up" => Some(Action::ProjectMoveUp),
        "project_move_down" => Some(Action::ProjectMoveDown),

        // Sidebar
        "enter_sidebar_mode" => Some(Action::EnterSidebarMode),
        "exit_sidebar_mode" => Some(Action::ExitSidebarMode),

        // Raw events
        "raw_events_select_next" => Some(Action::RawEventsSelectNext),
        "raw_events_select_prev" => Some(Action::RawEventsSelectPrev),
        "raw_events_toggle_expand" => Some(Action::RawEventsToggleExpand),
        "raw_events_collapse" => Some(Action::RawEventsCollapse),

        // Dialog
        "confirm_yes" => Some(Action::ConfirmYes),
        "confirm_no" => Some(Action::ConfirmNo),
        "confirm_toggle" => Some(Action::ConfirmToggle),
        "toggle_details" => Some(Action::ToggleDetails),

        // Agent
        "select_agent" => Some(Action::SelectAgent),

        // Session import
        "open_session_import" | "import" => Some(Action::OpenSessionImport),
        "import_session" => Some(Action::ImportSession),
        "cycle_import_filter" => Some(Action::CycleImportFilter),

        // Command mode
        "show_help" => Some(Action::ShowHelp),
        "execute_command" => Some(Action::ExecuteCommand),
        "complete_command" => Some(Action::CompleteCommand),

        // Command palette
        "open_command_palette" => Some(Action::OpenCommandPalette),

        // Actions added for keybindings editor coverage
        "new_workspace_under_cursor" => Some(Action::NewWorkspaceUnderCursor),
        "archive_current_workspace" => Some(Action::ArchiveCurrentWorkspace),
        "toggle_agent_mode" => Some(Action::ToggleAgentMode),
        "copy_workspace_path" => Some(Action::CopyWorkspacePath),
        "copy_code_block" => Some(Action::CopyCodeBlock),
        "copy_code_block_prev" => Some(Action::CopyCodeBlockPrev),
        "rename_project" => Some(Action::RenameProject),
        "add_file_to_project" => Some(Action::AddFileToProject),
        "upload_file_to_project" => Some(Action::UploadFileToProject),
        "refresh_sidebar" => Some(Action::RefreshSidebar),
        "event_detail_toggle" => Some(Action::EventDetailToggle),
        "event_detail_scroll_up" => Some(Action::EventDetailScrollUp),
        "event_detail_scroll_down" => Some(Action::EventDetailScrollDown),
        "event_detail_page_up" => Some(Action::EventDetailPageUp),
        "event_detail_page_down" => Some(Action::EventDetailPageDown),
        "event_detail_scroll_to_top" => Some(Action::EventDetailScrollToTop),
        "event_detail_scroll_to_bottom" => Some(Action::EventDetailScrollToBottom),
        "event_detail_copy" => Some(Action::EventDetailCopy),

        _ => None,
    }
}

/// Map an Action to its canonical TOML action name string.
///
/// This is the inverse of `parse_action()`. Returns `None` for parametric
/// actions that have no static TOML representation (`ScrollUp`, `ScrollDown`,
/// `SwitchToTab`, `OpenFile`).
pub fn action_to_name(action: &Action) -> Option<&'static str> {
    match action {
        // Global
        Action::Quit => Some("quit"),
        Action::ToggleSidebar => Some("toggle_sidebar"),
        Action::HideSidebar => Some("hide_sidebar"),
        Action::NewProject => Some("new_project"),
        Action::NewWorkspaceUnderCursor => Some("new_workspace_under_cursor"),
        Action::OpenPr => Some("open_pr"),
        Action::ForkSession => Some("fork_session"),
        Action::HandoffSession => Some("handoff_session"),
        Action::InterruptAgent => Some("interrupt_agent"),
        Action::ToggleViewMode => Some("toggle_view_mode"),
        Action::ShowModelSelector => Some("show_model_selector"),
        Action::ShowReasoningSelector => Some("show_reasoning_selector"),
        Action::ShowThemePicker => Some("show_theme_picker"),
        Action::ShowProvidersSelector => Some("show_providers_selector"),
        Action::ToggleMetrics => Some("toggle_metrics"),
        Action::DumpDebugState => Some("dump_debug_state"),
        Action::Suspend => Some("suspend"),
        Action::CopyWorkspacePath => Some("copy_workspace_path"),
        Action::CopySelection => Some("copy_selection"),
        Action::CopyCodeBlock => Some("copy_code_block"),
        Action::CopyCodeBlockPrev => Some("copy_code_block_prev"),
        // Tab management
        Action::CloseTab => Some("close_tab"),
        Action::NextTab => Some("next_tab"),
        Action::PrevTab => Some("prev_tab"),
        Action::MoveTabLeft => Some("move_tab_left"),
        Action::MoveTabRight => Some("move_tab_right"),
        Action::SwitchToTab(_) => None, // parametric
        // File viewer
        Action::OpenFile(_) => None, // parametric
        // Scrolling (note: parse_action maps "scroll_up" -> ScrollUp(1))
        Action::ScrollUp(_) => Some("scroll_up"),
        Action::ScrollDown(_) => Some("scroll_down"),
        Action::ScrollPageUp => Some("scroll_page_up"),
        Action::ScrollPageDown => Some("scroll_page_down"),
        Action::ScrollToTop => Some("scroll_to_top"),
        Action::ScrollToBottom => Some("scroll_to_bottom"),
        Action::ScrollPrevUserMessage => Some("prev_user_message"),
        Action::ScrollNextUserMessage => Some("next_user_message"),
        // Input editing
        Action::InsertNewline => Some("insert_newline"),
        Action::Backspace => Some("backspace"),
        Action::Delete => Some("delete"),
        Action::DeleteWordBack => Some("delete_word_back"),
        Action::DeleteWordForward => Some("delete_word_forward"),
        Action::DeleteToStart => Some("delete_to_start"),
        Action::DeleteToEnd => Some("delete_to_end"),
        Action::MoveCursorLeft => Some("move_cursor_left"),
        Action::MoveCursorRight => Some("move_cursor_right"),
        Action::MoveCursorStart => Some("move_cursor_start"),
        Action::MoveCursorEnd => Some("move_cursor_end"),
        Action::MoveWordLeft => Some("move_word_left"),
        Action::MoveWordRight => Some("move_word_right"),
        Action::MoveCursorUp => Some("move_cursor_up"),
        Action::MoveCursorDown => Some("move_cursor_down"),
        Action::HistoryPrev => Some("history_prev"),
        Action::HistoryNext => Some("history_next"),
        Action::Submit => Some("submit"),
        Action::SubmitSteer => Some("submit_steer"),
        Action::OpenQueueEditor => Some("open_queue_editor"),
        Action::CloseQueueEditor => Some("close_queue_editor"),
        Action::QueueMoveUp => Some("queue_move_up"),
        Action::QueueMoveDown => Some("queue_move_down"),
        Action::QueueEdit => Some("queue_edit"),
        Action::QueueDelete => Some("queue_delete"),
        Action::EditPromptExternal => Some("edit_prompt_external"),
        // Navigation
        Action::SelectNext => Some("select_next"),
        Action::SelectPrev => Some("select_prev"),
        Action::SelectPageDown => Some("select_page_down"),
        Action::SelectPageUp => Some("select_page_up"),
        Action::Confirm => Some("confirm"),
        Action::Cancel => Some("cancel"),
        Action::SetDefaultModel => Some("set_default_model"),
        Action::ExpandOrSelect => Some("expand_or_select"),
        Action::Collapse => Some("collapse"),
        Action::AddRepository => Some("add_repository"),
        Action::OpenSettings => Some("open_settings"),
        Action::ArchiveOrRemove => Some("archive_or_remove"),
        Action::ArchiveCurrentWorkspace => Some("archive_current_workspace"),
        Action::ProjectMoveUp => Some("project_move_up"),
        Action::ProjectMoveDown => Some("project_move_down"),
        Action::RenameProject => Some("rename_project"),
        Action::AddFileToProject => Some("add_file_to_project"),
        Action::UploadFileToProject => Some("upload_file_to_project"),
        // Sidebar
        Action::EnterSidebarMode => Some("enter_sidebar_mode"),
        Action::ExitSidebarMode => Some("exit_sidebar_mode"),
        // Raw events
        Action::RawEventsSelectNext => Some("raw_events_select_next"),
        Action::RawEventsSelectPrev => Some("raw_events_select_prev"),
        Action::RawEventsToggleExpand => Some("raw_events_toggle_expand"),
        Action::RawEventsCollapse => Some("raw_events_collapse"),
        // Event detail
        Action::EventDetailToggle => Some("event_detail_toggle"),
        Action::EventDetailScrollUp => Some("event_detail_scroll_up"),
        Action::EventDetailScrollDown => Some("event_detail_scroll_down"),
        Action::EventDetailPageUp => Some("event_detail_page_up"),
        Action::EventDetailPageDown => Some("event_detail_page_down"),
        Action::EventDetailScrollToTop => Some("event_detail_scroll_to_top"),
        Action::EventDetailScrollToBottom => Some("event_detail_scroll_to_bottom"),
        Action::EventDetailCopy => Some("event_detail_copy"),
        // Confirmation
        Action::ConfirmYes => Some("confirm_yes"),
        Action::ConfirmNo => Some("confirm_no"),
        Action::ConfirmToggle => Some("confirm_toggle"),
        Action::ToggleDetails => Some("toggle_details"),
        // Agent
        Action::SelectAgent => Some("select_agent"),
        Action::ToggleAgentMode => Some("toggle_agent_mode"),
        // Session import
        Action::OpenSessionImport => Some("open_session_import"),
        Action::ImportSession => Some("import_session"),
        Action::CycleImportFilter => Some("cycle_import_filter"),
        // Command mode
        Action::ShowHelp => Some("show_help"),
        Action::ExecuteCommand => Some("execute_command"),
        Action::CompleteCommand => Some("complete_command"),
        // Command palette
        Action::OpenCommandPalette => Some("open_command_palette"),
        // Sidebar
        Action::RefreshSidebar => Some("refresh_sidebar"),
    }
}

/// All available command names for autocomplete
pub const COMMAND_NAMES: &[&str] = &[
    // Global
    "quit",
    "toggle_sidebar",
    "hide_sidebar",
    "new_project",
    "open_pr",
    "fork_session",
    "handoff_session",
    "interrupt_agent",
    "toggle_view_mode",
    "show_model_selector",
    "show_reasoning_selector",
    "show_theme_picker",
    "show_providers_selector",
    "toggle_metrics",
    "dump_debug_state",
    "suspend",
    "copy_selection",
    // Tab management
    "close_tab",
    "next_tab",
    "prev_tab",
    // Scrolling
    "scroll_up",
    "scroll_down",
    "scroll_page_up",
    "scroll_page_down",
    "scroll_to_top",
    "scroll_to_bottom",
    "prev_user_message",
    "next_user_message",
    // Input editing
    "insert_newline",
    "backspace",
    "delete",
    "delete_word_back",
    "delete_word_forward",
    "delete_to_start",
    "delete_to_end",
    "move_cursor_left",
    "move_cursor_right",
    "move_cursor_start",
    "move_cursor_end",
    "move_word_left",
    "move_word_right",
    "move_cursor_up",
    "move_cursor_down",
    "history_prev",
    "history_next",
    "submit",
    "submit_steer",
    "open_queue_editor",
    "close_queue_editor",
    "queue_move_up",
    "queue_move_down",
    "queue_edit",
    "queue_delete",
    "edit_prompt_external",
    // Navigation
    "select_next",
    "select_prev",
    "select_page_down",
    "select_page_up",
    "confirm",
    "cancel",
    "set_default_model",
    "expand_or_select",
    "collapse",
    "add_repository",
    "open_settings",
    "archive_or_remove",
    // Sidebar
    "enter_sidebar_mode",
    "exit_sidebar_mode",
    // Raw events
    "raw_events_select_next",
    "raw_events_select_prev",
    "raw_events_toggle_expand",
    "raw_events_collapse",
    // Dialog
    "confirm_yes",
    "confirm_no",
    "confirm_toggle",
    "toggle_details",
    // Agent
    "select_agent",
    // Session import
    "open_session_import",
    "import",
    "import_session",
    "cycle_import_filter",
    // Command mode
    "show_help",
    // Command palette
    "open_command_palette",
    // Aliases
    "help",
    "h",
    "q",
];

impl Config {
    fn parse_provider(value: &str) -> Option<AgentType> {
        match value.trim().to_ascii_lowercase().as_str() {
            "claude" => Some(AgentType::Claude),
            "codex" => Some(AgentType::Codex),
            "dirac" => Some(AgentType::Dirac),
            "gemini" => Some(AgentType::Gemini),
            "opencode" => Some(AgentType::Opencode),
            "cecli" => Some(AgentType::Cecli),
            "copilot" => Some(AgentType::Copilot),
            "pi" => Some(AgentType::Pi),
            _ => None,
        }
    }

    /// Load configuration from file, merging with defaults
    pub fn load() -> Self {
        let mut config = Config::default();

        let config_file = config_path();

        // Create example config on first run
        if !config_file.exists() {
            Self::create_default_config(&config_file);
        }

        // Try to load user config
        if config_file.exists() {
            if let Ok(contents) = fs::read_to_string(&config_file) {
                if let Ok(toml_config) = toml::from_str::<TomlConfig>(&contents) {
                    // Load default model (agent + model pair)
                    if let Some(model_cfg) = toml_config.model {
                        if let (Some(agent), Some(model_id)) =
                            (model_cfg.agent.as_deref(), model_cfg.model.as_deref())
                        {
                            let agent_type = AgentType::parse(agent);
                            if let Some(model) = ModelRegistry::find_model(agent_type, model_id) {
                                config.default_agent = agent_type;
                                config.default_model = Some(model.id);
                            }
                        }
                    }

                    // Load enabled providers
                    if let Some(providers_cfg) = toml_config.providers {
                        if let Some(enabled) = providers_cfg.enabled {
                            let mut providers = Vec::new();
                            for value in enabled {
                                if let Some(parsed) = Self::parse_provider(&value) {
                                    if !providers.contains(&parsed) {
                                        providers.push(parsed);
                                    }
                                }
                            }
                            config.enabled_providers = Some(providers);
                        }
                    }

                    // Merge user keybindings on top of defaults
                    if let Some(keys) = toml_config.keys {
                        let user_bindings = keys.to_keybinding_config();
                        config.keybindings.merge(user_bindings);
                    }

                    // Load tool paths if configured
                    if let Some(tools) = toml_config.tools {
                        config.tool_paths = tools;
                    }

                    // Load theme configuration
                    if let Some(theme) = toml_config.theme {
                        config.theme_path = theme.path;
                        config.theme_name = theme.name;
                    }

                    // Load queue configuration
                    if let Some(queue) = toml_config.queue {
                        if let Some(delivery) = queue.delivery {
                            config.queue.delivery = delivery;
                        }
                        if let Some(mode) = queue.mode {
                            config.queue.mode = mode;
                        }
                    }

                    // Load steering configuration
                    if let Some(steer) = toml_config.steer {
                        if let Some(behavior) = steer.behavior {
                            config.steer.behavior = behavior;
                        }
                        if let Some(fallback) = steer.fallback {
                            config.steer.fallback = fallback;
                        }
                    }

                    // Load selection configuration
                    if let Some(selection) = toml_config.selection {
                        if let Some(auto_copy_selection) = selection.auto_copy_selection {
                            config.selection.auto_copy_selection = auto_copy_selection;
                        }
                        if let Some(clear_selection_after_copy) =
                            selection.clear_selection_after_copy
                        {
                            config.selection.clear_selection_after_copy =
                                clear_selection_after_copy;
                        }
                    }

                    // Load UI configuration
                    if let Some(ui) = toml_config.ui {
                        if let Some(show_chat_scrollbar) = ui.show_chat_scrollbar {
                            config.ui.show_chat_scrollbar = show_chat_scrollbar;
                        }
                        if let Some(always_show_sidebar) = ui.always_show_sidebar {
                            config.ui.always_show_sidebar = always_show_sidebar;
                        }
                        if let Some(sidebar_in_tab_cycle) = ui.sidebar_in_tab_cycle {
                            config.ui.sidebar_in_tab_cycle = sidebar_in_tab_cycle;
                        }
                        if let Some(thinking_indicator_shimmer) = ui.thinking_indicator_shimmer {
                            config.ui.thinking_indicator_shimmer = thinking_indicator_shimmer;
                        }
                        if let Some(label) = ui.thinking_indicator_label {
                            config.ui.thinking_indicator_label = label;
                        }
                        if let Some(spinner) = ui.thinking_indicator_spinner {
                            config.ui.thinking_indicator_spinner = spinner;
                        }
                    }
                    // Load web status configuration
                    if let Some(web_status) = toml_config.web_status {
                        if let Some(initial_scan) = web_status.initial_scan {
                            config.web_status.initial_scan = initial_scan;
                        }
                        if let Some(status_scan_concurrency) = web_status.status_scan_concurrency {
                            config.web_status.status_scan_concurrency = status_scan_concurrency;
                        }
                        if let Some(selected_refresh_interval_ms) =
                            web_status.selected_refresh_interval_ms
                        {
                            config.web_status.selected_refresh_interval_ms =
                                selected_refresh_interval_ms;
                        }
                        if let Some(pr_refresh_interval_ms) = web_status.pr_refresh_interval_ms {
                            config.web_status.pr_refresh_interval_ms = pr_refresh_interval_ms;
                        }
                    }
                    // Load remote issue provider configuration
                    if let Some(issues) = toml_config.issues {
                        if let Some(hosts) = issues.gitea_hosts {
                            config.issues.gitea_hosts = hosts;
                        }
                        if let Some(hosts) = issues.forgejo_hosts {
                            config.issues.forgejo_hosts = hosts;
                        }
                    }
                    // Load workspace defaults
                    if let Some(workspaces) = toml_config.workspaces {
                        if let Some(mode) = workspaces.mode {
                            config.workspaces.default_mode = mode;
                        }
                        if let Some(delete_branch) = workspaces.archive_delete_branch {
                            config.workspaces.archive_delete_branch = delete_branch;
                        }
                        if let Some(remote_prompt) = workspaces.archive_remote_prompt {
                            config.workspaces.archive_remote_prompt = remote_prompt;
                        }
                        if let Some(use_gh_cli_merge_status) = workspaces.use_gh_cli_merge_status {
                            config.workspaces.use_gh_cli_merge_status = use_gh_cli_merge_status;
                        }
                    }
                }
            }
        }

        config
    }

    /// Create the default config file from the bundled example
    fn create_default_config(path: &Path) {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    tracing::warn!(error = %e, path = %parent.display(), "Failed to create config directory");
                    return;
                }
            }
        }

        // Write the example config
        if let Err(e) = fs::write(path, EXAMPLE_CONFIG) {
            tracing::warn!(error = %e, path = %path.display(), "Failed to write default config");
        }
    }

    pub fn with_working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = dir;
        self
    }

    pub fn with_default_agent(mut self, agent: AgentType) -> Self {
        self.default_agent = agent;
        self
    }

    fn tool_for_provider(provider: AgentType) -> Tool {
        match provider {
            AgentType::Claude => Tool::Claude,
            AgentType::Codex => Tool::Codex,
            AgentType::Dirac => Tool::Dirac,
            AgentType::Gemini => Tool::Gemini,
            AgentType::Opencode => Tool::Opencode,
            AgentType::Cecli => Tool::Cecli,
            AgentType::Copilot => Tool::Copilot,
            AgentType::Pi => Tool::Pi,
        }
    }

    /// Effective enabled providers after intersecting config with installed tools.
    ///
    /// - If providers are configured in config, only those providers are considered enabled.
    /// - If not configured, all installed providers are enabled.
    pub fn effective_enabled_providers(&self, tools: &ToolAvailability) -> Vec<AgentType> {
        let all = AgentType::preferred_order();

        let configured = &self.enabled_providers;
        all.into_iter()
            .filter(|provider| tools.is_available(Self::tool_for_provider(*provider)))
            .filter(|provider| {
                configured
                    .as_ref()
                    .is_none_or(|items| items.iter().any(|item| item == provider))
            })
            .collect()
    }

    /// Check if a provider is effectively enabled (configured + installed).
    pub fn is_provider_enabled_effective(
        &self,
        provider: AgentType,
        tools: &ToolAvailability,
    ) -> bool {
        self.effective_enabled_providers(tools)
            .into_iter()
            .any(|p| p == provider)
    }

    /// Update enabled providers in memory.
    pub fn set_enabled_providers(&mut self, providers: Vec<AgentType>) {
        self.enabled_providers = Some(providers);
    }

    /// Get the default model ID for an agent (config override with fallback)
    pub fn default_model_for(&self, agent_type: AgentType) -> String {
        if agent_type == self.default_agent {
            if let Some(id) = self.default_model.as_deref() {
                if let Some(model) = ModelRegistry::find_model(agent_type, id) {
                    return model.id;
                }
            }
        }

        ModelRegistry::default_model(agent_type)
    }

    /// Update the default model for an agent in memory
    pub fn set_default_model(&mut self, agent_type: AgentType, model_id: String) {
        self.default_agent = agent_type;
        self.default_model = Some(model_id);
    }

    /// Update global workspace defaults in memory.
    pub fn set_workspace_defaults(
        &mut self,
        mode: WorkspaceMode,
        archive_delete_branch: bool,
        archive_remote_prompt: bool,
    ) {
        self.workspaces.default_mode = mode;
        self.workspaces.archive_delete_branch = archive_delete_branch;
        self.workspaces.archive_remote_prompt = archive_remote_prompt;
    }

    /// Calculate cost for given token usage
    pub fn calculate_cost(&self, input_tokens: i64, output_tokens: i64) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.claude_input_cost_per_million;
        let output_cost =
            (output_tokens as f64 / 1_000_000.0) * self.claude_output_cost_per_million;
        input_cost + output_cost
    }
}

/// Save a tool path to the config file
///
/// This function reads the existing config.toml, adds or updates the tool path
/// in the [tools] section, and writes it back while preserving all other content.
pub fn save_tool_path(tool: Tool, path: &Path) -> std::io::Result<()> {
    let config_file = config_path();

    // Read existing config or start with empty document
    let contents = if config_file.exists() {
        fs::read_to_string(&config_file)?
    } else {
        String::new()
    };

    // Parse as TOML document
    let mut doc: DocumentMut = contents
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    // Ensure [tools] section exists
    if !doc.contains_key("tools") {
        doc["tools"] = Item::Table(Table::new());
    }

    // Set the tool path
    let path_str = path.to_string_lossy().to_string();
    doc["tools"][tool.binary_name()] = toml_edit::value(path_str);

    // Ensure parent directory exists
    if let Some(parent) = config_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    // Write back to file
    fs::write(&config_file, doc.to_string())?;

    Ok(())
}

/// Save the selected theme to the config file.
///
/// This updates the [theme] section, setting either "name" or "path"
/// and clearing the other to avoid ambiguity.
pub fn save_theme_config(name: Option<&str>, path: Option<&Path>) -> std::io::Result<()> {
    let config_file = config_path();

    // Read existing config or start with empty document
    let contents = if config_file.exists() {
        fs::read_to_string(&config_file)?
    } else {
        String::new()
    };

    // Parse as TOML document
    let mut doc: DocumentMut = contents
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    if name.is_none() && path.is_none() {
        if doc.contains_key("theme") {
            doc.remove("theme");
        }
    } else {
        // Ensure [theme] section exists
        if !doc.contains_key("theme") {
            doc["theme"] = Item::Table(Table::new());
        }

        if let Some(path) = path {
            let path_str = path.to_string_lossy().to_string();
            doc["theme"]["path"] = toml_edit::value(path_str);
            if let Item::Table(table) = &mut doc["theme"] {
                table.remove("name");
            }
        } else if let Some(name) = name {
            doc["theme"]["name"] = toml_edit::value(name);
            if let Item::Table(table) = &mut doc["theme"] {
                table.remove("path");
            }
        }
    }

    // Ensure parent directory exists
    if let Some(parent) = config_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::write(&config_file, doc.to_string())?;

    Ok(())
}

/// Save the default model for an agent type to the config file.
///
/// This updates the [model] section, setting "agent" and "model".
pub fn save_default_model(agent_type: AgentType, model_id: &str) -> std::io::Result<()> {
    let config_file = config_path();

    // Read existing config or start with empty document
    let contents = if config_file.exists() {
        fs::read_to_string(&config_file)?
    } else {
        String::new()
    };

    // Parse as TOML document
    let mut doc: DocumentMut = contents
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    // Ensure [model] section exists
    if !doc.contains_key("model") {
        doc["model"] = Item::Table(Table::new());
    }

    doc["model"]["agent"] = toml_edit::value(agent_type.as_str());
    doc["model"]["model"] = toml_edit::value(model_id);

    // Ensure parent directory exists
    if let Some(parent) = config_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::write(&config_file, doc.to_string())?;

    Ok(())
}

/// Save enabled providers to the config file.
///
/// This updates the [providers] section, setting "enabled" as an array of provider IDs.
pub fn save_enabled_providers(providers: &[AgentType]) -> std::io::Result<()> {
    let config_file = config_path();

    // Read existing config or start with empty document
    let contents = if config_file.exists() {
        fs::read_to_string(&config_file)?
    } else {
        String::new()
    };

    // Parse as TOML document
    let mut doc: DocumentMut = contents
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    // Ensure [providers] section exists
    if !doc.contains_key("providers") {
        doc["providers"] = Item::Table(Table::new());
    }

    let mut enabled_values = Array::default();
    for provider in providers {
        enabled_values.push(provider.as_str());
    }
    doc["providers"]["enabled"] = Item::Value(toml_edit::Value::Array(enabled_values));

    // Ensure parent directory exists
    if let Some(parent) = config_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::write(&config_file, doc.to_string())?;

    Ok(())
}

/// Save workspace defaults to the config file.
pub fn save_workspaces_config(
    mode: WorkspaceMode,
    archive_delete_branch: bool,
    archive_remote_prompt: bool,
) -> std::io::Result<()> {
    let config_file = config_path();

    let contents = if config_file.exists() {
        fs::read_to_string(&config_file)?
    } else {
        String::new()
    };

    let mut doc: DocumentMut = contents
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    if !doc.contains_key("workspaces") {
        doc["workspaces"] = Item::Table(Table::new());
    }

    doc["workspaces"]["mode"] = toml_edit::value(mode.as_str());
    doc["workspaces"]["archive_delete_branch"] = toml_edit::value(archive_delete_branch);
    doc["workspaces"]["archive_remote_prompt"] = toml_edit::value(archive_remote_prompt);

    if let Some(parent) = config_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::write(&config_file, doc.to_string())?;

    Ok(())
}

/// Save a single keybinding override to the config file.
///
/// For global keybindings (`context = None`), sets `action_name = key_str`
/// under `[keys]`. For context-specific bindings, sets it under `[keys.<section>]`.
/// Follows the same `toml_edit` pattern as `save_theme_config()`.
pub fn save_keybinding(
    context: Option<KeyContext>,
    action_name: &str,
    key_str: &str,
) -> std::io::Result<()> {
    let config_file = config_path();

    let contents = if config_file.exists() {
        fs::read_to_string(&config_file)?
    } else {
        String::new()
    };

    let mut doc: DocumentMut = contents
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    if !doc.contains_key("keys") {
        doc["keys"] = Item::Table(Table::new());
    }

    match context.and_then(|c| c.toml_section_name()) {
        None => {
            // Global binding: [keys] action_name = key_str
            doc["keys"][action_name] = toml_edit::value(key_str);
        }
        Some(section) => {
            // Context binding: [keys.section] action_name = key_str
            if let Item::Table(keys_table) = &mut doc["keys"] {
                if !keys_table.contains_key(section) {
                    keys_table.insert(section, Item::Table(Table::new()));
                }
            }
            doc["keys"][section][action_name] = toml_edit::value(key_str);
        }
    }

    if let Some(parent) = config_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::write(&config_file, doc.to_string())?;

    Ok(())
}

/// Remove a single keybinding override from the config file.
///
/// This removes the user's custom binding so the default takes effect.
/// No-op if the binding is not present in the config file.
pub fn remove_keybinding(context: Option<KeyContext>, action_name: &str) -> std::io::Result<()> {
    let config_file = config_path();

    if !config_file.exists() {
        return Ok(());
    }

    let contents = fs::read_to_string(&config_file)?;
    let mut doc: DocumentMut = contents
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    match context.and_then(|c| c.toml_section_name()) {
        None => {
            // Global binding under [keys]
            if let Some(Item::Table(keys_table)) = doc.get_mut("keys") {
                keys_table.remove(action_name);
            }
        }
        Some(section) => {
            // Context binding under [keys.section]
            if doc.contains_key("keys") {
                if let Item::Table(keys_table) = &mut doc["keys"] {
                    if let Some(Item::Table(section_table)) = keys_table.get_mut(section) {
                        section_table.remove(action_name);
                    }
                }
            }
        }
    }

    fs::write(&config_file, doc.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_action_handoff_session() {
        assert_eq!(
            parse_action("handoff_session"),
            Some(Action::HandoffSession)
        );
    }

    #[test]
    fn test_command_names_include_handoff_session() {
        assert!(
            COMMAND_NAMES.contains(&"handoff_session"),
            "handoff_session should be present in COMMAND_NAMES"
        );
    }
}
