use std::path::PathBuf;

use crate::git_tracker::GitTrackerUpdate;
use conduit_agent::{AgentEvent, AgentInput, AgentType};
use conduit_git::{OpenSpec, PrPreflightResult, RemoteIssue, SpecifySpec};
use tokio::sync::mpsc;
use uuid::Uuid;

/// Application-level events
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Terminal input event
    Input(crossterm::event::Event),

    /// Agent event from a session (identified by stable session ID)
    Agent { session_id: Uuid, event: AgentEvent },

    /// Agent event stream ended (process exited)
    AgentStreamEnded { session_id: Uuid },

    /// Agent subprocess started with given PID
    AgentStarted {
        session_id: Uuid,
        pid: u32,
        input_tx: Option<mpsc::Sender<AgentInput>>,
    },
    /// Agent failed to start for a specific session
    AgentStartFailed { session_id: Uuid, error: String },
    /// Agent termination result (used for async termination feedback)
    AgentTerminationResult {
        session_id: Option<Uuid>,
        pid: u32,
        context: String,
        success: bool,
    },

    /// User submitted a prompt
    PromptSubmit { tab_index: usize, prompt: String },

    /// Request to create a new tab
    NewTab(AgentType),

    /// Request to close a tab
    CloseTab(usize),

    /// Request to switch to a tab
    SwitchTab(usize),

    /// Agent selection dialog requested
    ShowAgentSelector,

    /// Agent selected from dialog
    AgentSelected(AgentType),

    /// Request to interrupt current agent
    InterruptAgent(usize),

    /// Toggle sidebar visibility
    ToggleSidebar,

    /// Open a workspace (creates/switches to tab)
    OpenWorkspace(Uuid),

    /// Refresh sidebar data from database
    RefreshSidebar,

    /// Tick event for animations/updates
    Tick,

    /// Request to quit the application
    Quit,

    /// Error occurred
    Error(String),

    /// PR preflight check completed
    PrPreflightCompleted {
        tab_index: usize,
        working_dir: PathBuf,
        result: PrPreflightResult,
    },

    /// Open PR in browser completed
    OpenPrCompleted { result: Result<(), String> },

    /// Debug export completed
    DebugDumped { result: Result<String, String> },

    /// Progress update during workspace creation (git fetch output, stage labels)
    WorkspaceCreationProgress { message: String },

    /// Streaming line of `git fetch` output during the SyncingRemote phase.
    RemoteSyncProgress { message: String },

    /// Remote sync completed; issue and spec fetches can now begin
    RemoteSynced { repo_id: Uuid },

    /// GitHub issues fetched for workspace creation issue picker
    RemoteIssuesFetched {
        repo_id: Uuid,
        issues: Vec<RemoteIssue>,
    },

    /// Current user resolved for the repo (None = lookup failed/unavailable)
    CurrentUserFetched { repo_id: Uuid, user: Option<String> },

    /// All specs fetched for workspace creation (openspec + specify, combined pass)
    AllSpecsFetched {
        repo_id: Uuid,
        open_specs: Vec<OpenSpec>,
        specify_specs: Vec<SpecifySpec>,
        /// Source ref used to read specs (e.g. `origin/master`); `None` when the
        /// fallback working-tree scan was used instead.
        source_ref: Option<String>,
    },

    /// Workspace creation completed
    WorkspaceCreated {
        repo_id: Uuid,
        result: Result<WorkspaceCreated, String>,
    },
    /// Fork workspace creation completed
    ForkWorkspaceCreated {
        parent_workspace_id: Uuid,
        result: Result<ForkWorkspaceCreated, String>,
    },

    /// Workspace archive completed
    WorkspaceArchived {
        workspace_id: Uuid,
        result: Result<WorkspaceArchived, String>,
    },

    /// Archive-dialog preflight completed.
    ArchiveWorkspaceDialogPreflightCompleted {
        workspace_id: Uuid,
        result: Result<ArchiveWorkspaceDialogPreflightResult, String>,
    },

    /// Archive preflight completed (remote branch prompt check)
    ArchiveWorkspacePreflightCompleted {
        workspace_id: Uuid,
        result: Result<ArchiveWorkspacePreflightResult, String>,
    },

    /// Remove-project dialog preflight completed.
    RemoveProjectDialogPreflightCompleted {
        repo_id: Uuid,
        result: Result<RemoveProjectDialogPreflightResult, String>,
    },

    /// Fork-session dialog preflight completed.
    ForkSessionDialogPreflightCompleted {
        parent_workspace_id: Uuid,
        result: Result<ForkSessionDialogPreflightResult, String>,
    },

    /// Project picker scan completed.
    ProjectsDiscovered {
        base_dir: PathBuf,
        result: Result<Vec<ProjectDiscoveryEntry>, String>,
    },

    /// Project removal completed
    ProjectRemoved { result: RemoveProjectResult },

    /// Remote repository clone completed
    RepositoryCloned {
        result: Result<std::path::PathBuf, String>,
    },

    /// Cached sessions loaded (fast path from disk cache)
    SessionsCacheLoaded {
        sessions: Vec<conduit_session::ExternalSession>,
    },

    /// Single session updated during background refresh
    SessionUpdated {
        session: conduit_session::ExternalSession,
    },

    /// Session removed (file no longer exists)
    SessionRemoved { file_path: PathBuf },

    /// Background session discovery complete
    SessionDiscoveryComplete,

    /// Git tracker update (PR status, git stats, branch changes)
    GitTracker(GitTrackerUpdate),

    /// Title/branch generation completed
    TitleGenerated {
        /// Stable session ID for correlation (avoids stale tab_index after close/reorder)
        session_id: Uuid,
        result: Result<TitleGeneratedResult, String>,
    },

    /// Shell command execution completed
    ShellCommandCompleted {
        session_id: Uuid,
        message_index: usize,
        result: Result<ShellCommandResult, String>,
    },

    /// OpenCode question response sent (success or error)
    OpencodeQuestionResponseCompleted {
        session_id: Uuid,
        result: Result<(), String>,
    },
}

/// Result of successful title/branch generation
#[derive(Debug, Clone)]
pub struct TitleGeneratedResult {
    /// AI-generated session title
    pub title: String,
    /// New branch name (None if rename failed/skipped)
    pub new_branch: Option<String>,
    /// Associated workspace ID
    pub workspace_id: Option<Uuid>,
    /// Tool used to generate the title
    pub tool_used: Option<String>,
    /// Whether the generation fell back to a secondary tool
    pub used_fallback: bool,
}

#[derive(Debug, Clone)]
pub struct ShellCommandResult {
    pub output: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceCreated {
    pub repo_id: Uuid,
    pub workspace_id: Uuid,
    pub initial_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ForkWorkspaceCreated {
    pub repo_id: Uuid,
    pub workspace_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct WorkspaceArchived {
    pub workspace_id: Uuid,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ArchiveWorkspaceDialogPreflightResult {
    pub workspace_name: String,
    pub message: String,
    pub warnings: Vec<String>,
    /// Informational items shown with a green tick (not warnings)
    pub info_items: Vec<String>,
    pub has_dirty: bool,
    pub has_unmerged: bool,
}

#[derive(Debug, Clone)]
pub struct ArchiveWorkspacePreflightResult {
    pub should_prompt_remote_delete: bool,
}

#[derive(Debug, Clone)]
pub struct RemoveProjectDialogPreflightResult {
    pub repo_name: String,
    pub warnings: Vec<String>,
    pub has_dirty: bool,
    pub has_unmerged: bool,
    pub workspace_count: usize,
}

#[derive(Debug, Clone)]
pub struct ForkSessionDialogPreflightResult {
    pub base_branch: String,
    pub dirty_warning: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectDiscoveryEntry {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct RemoveProjectResult {
    pub repo_id: Uuid,
    pub workspace_ids: Vec<Uuid>,
    pub errors: Vec<String>,
}

pub use conduit_types::{InputMode, ViewMode};
