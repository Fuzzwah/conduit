//! Git operations module

mod issues;
mod pr;
mod specify;
mod specs;
mod status;
mod workspace_mode;
mod workspace_repo;
mod worktree;

pub use issues::{current_user, fetch_open_issues, IssueProvider, IssuesConfig, RemoteIssue};
pub use pr::{
    CheckState, CheckStatus, MergeReadiness, MergeableStatus, PrManager, PrPreflightResult,
    PrState, PrStatus, ReviewDecision,
};
pub use specify::{fetch_specify_specs, fetch_specify_specs_from_ref, SpecifySpec};
pub use specs::{fetch_open_specs, fetch_open_specs_from_ref, OpenSpec};
pub use status::{get_ahead_behind, GitDiffStats};
pub use workspace_mode::WorkspaceMode;
pub use workspace_repo::WorkspaceRepoManager;
pub use worktree::{
    detect_default_branch, sync_remote, sync_remote_with_progress, WorktreeInfo, WorktreeManager,
};
