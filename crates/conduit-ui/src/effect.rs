use std::path::PathBuf;

use conduit_agent::{AgentStartConfig, AgentType};
use conduit_git::{OpenSpec, RemoteIssue, SpecifySpec, SuggestedAction};
use conduit_session::ExternalSession;
use uuid::Uuid;

/// Side effects that should be executed outside the reducer.
pub enum Effect {
    SaveSessionState,
    StartAgent {
        session_id: Uuid,
        agent_type: AgentType,
        config: AgentStartConfig,
    },
    PrPreflight {
        tab_index: usize,
        working_dir: PathBuf,
    },
    OpenPrInBrowser {
        working_dir: PathBuf,
    },
    DumpDebugState,
    /// Fetch from remote so local refs are up to date before issue/spec checks
    SyncRemote {
        repo_id: Uuid,
    },
    FetchRemoteIssues {
        repo_id: Uuid,
    },
    /// Resolve the current user login for the repo's remote provider (lazy, on
    /// first toggle of the "mine only" filter).
    FetchCurrentUser {
        repo_id: Uuid,
    },
    FetchAllSpecs {
        repo_id: Uuid,
    },
    CreateWorkspace {
        repo_id: Uuid,
        issue: Option<RemoteIssue>,
        spec: Option<OpenSpec>,
        specify_spec: Option<SpecifySpec>,
    },
    ForkWorkspace {
        parent_workspace_id: Uuid,
        base_branch: String,
    },
    RemoveProject {
        repo_id: Uuid,
    },
    CopyToClipboard(String),
    /// Discover external sessions (Claude Code and Codex CLI; Gemini not supported yet)
    DiscoverSessions,
    /// Import an external session
    ImportSession(ExternalSession),
    /// Generate session title and branch name from first message
    GenerateTitleAndBranch {
        /// Stable session ID for correlation (avoids stale tab_index after close/reorder)
        session_id: Uuid,
        user_message: String,
        working_dir: PathBuf,
        workspace_id: Option<Uuid>,
        current_branch: String,
    },
    /// Run the Work Complete preflight (or re-run after an action).
    WorkCompletePreflight {
        workspace_id: Uuid,
    },
    /// Execute a single Work Complete action.
    WorkCompleteAction {
        workspace_id: Uuid,
        action: SuggestedAction,
        /// Optional payload (e.g., commit message for Commit action).
        payload: Option<String>,
    },
    /// Monitor CI checks for the given PR URL (`gh pr checks --watch`).
    WorkCompleteCiMonitor {
        workspace_id: Uuid,
        pr_url: String,
    },
    /// Run a local shell command
    RunShellCommand {
        session_id: Uuid,
        message_index: usize,
        command: String,
        working_dir: Option<PathBuf>,
    },
}
