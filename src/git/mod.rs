//! Git operations module

mod issues;
mod pr;
mod specs;
mod status;
mod workspace_mode;
mod workspace_repo;
mod worktree;

pub use issues::{fetch_open_issues, GithubIssue};
pub use pr::{
    CheckState, CheckStatus, MergeReadiness, MergeableStatus, PrManager, PrPreflightResult,
    PrState, PrStatus, ReviewDecision,
};
pub use specs::{fetch_open_specs, OpenSpec};
pub use status::{get_ahead_behind, GitDiffStats};
pub use workspace_mode::WorkspaceMode;
pub use workspace_repo::WorkspaceRepoManager;
pub use worktree::{WorktreeInfo, WorktreeManager};
