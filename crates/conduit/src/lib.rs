//! Conduit umbrella crate.
//!
//! Re-exports the public surface of the per-tier workspace crates so that
//! external consumers and integration tests can use a single `conduit::*`
//! prefix.

pub use conduit_agent as agent;
pub use conduit_config as config;
pub use conduit_core as core;
pub use conduit_data as data;
pub use conduit_git as git;
pub use conduit_resolver as resolver;
pub use conduit_session as session;
pub use conduit_theme as theme;
pub use conduit_types as types;
pub use conduit_ui as ui;
pub use conduit_util as util;
pub use conduit_web as web;

#[deprecated(since = "0.6.0", note = "use `conduit::resolver` instead")]
pub use conduit_resolver as command_resolver;

pub use agent::{
    AgentError, AgentEvent, AgentHandle, AgentMode, AgentRunner, AgentStartConfig, AgentType,
    ClaudeCodeRunner, CodexCliRunner, GeminiCliRunner, MockAgentRunner, MockConfig,
    MockEventBuilder, MockStartError, ModelInfo, ModelRegistry, OpencodeRunner, SessionId,
    SessionMetadata, SessionStatus,
};
pub use config::Config;
pub use core::ConduitCore;
pub use data::{Database, Repository, RepositoryStore, Workspace, WorkspaceStore};
pub use git::{
    CheckState, CheckStatus, MergeReadiness, MergeableStatus, PrManager, PrPreflightResult,
    PrState, PrStatus, ReviewDecision, WorkspaceMode, WorkspaceRepoManager, WorktreeInfo,
    WorktreeManager,
};
pub use resolver::{
    CommandResolver, ConduitCommand, MenuEntry as ResolvedMenuEntry, MenuEntryKind,
    ProviderInvocation, ResolveResult, ResolvedPrompt, SkillReference,
};
pub use session::{
    discover_all_sessions, discover_claude_sessions, discover_codex_sessions,
    discover_opencode_sessions, discover_pi_sessions, ExternalSession,
};
pub use ui::App;
pub use util::{generate_branch_name, generate_workspace_name, get_git_username};
