//! Git operations module

mod actions;
mod issues;
mod openspec_archive;
mod pr;
mod specify;
mod specs;
mod status;
mod work_complete;
mod workspace_mode;
mod workspace_repo;
mod worktree;

pub use actions::{commit_all, push_branch};
pub use issues::{
    close_issue, current_user, fetch_open_issues, infer_active_issue, view_issue, IssueProvider,
    IssueView, IssuesConfig, RemoteIssue,
};
pub use openspec_archive::{archive_change, ArchiveError, ArchiveResult};
pub use pr::{
    CheckState, CheckStatus, MergeMethod, MergeReadiness, MergeableStatus, PrCreateOpts, PrInfo,
    PrManager, PrPreflightResult, PrState, PrStatus, ReviewDecision,
};
pub use specify::{fetch_specify_specs, fetch_specify_specs_from_ref, SpecifySpec};
pub use specs::{
    fetch_change_detail, fetch_open_specs, fetch_open_specs_from_ref, infer_active_change,
    OpenSpec, SpecDetail,
};
pub use status::{get_ahead_behind, git_diff_files, DirtyFile, GitDiffStats};
pub use work_complete::{
    classify, ContextSource, GitState, IssueSnapshot, PrSnapshot, Scenario, SpecSnapshot,
    SuggestedAction,
};
pub use workspace_mode::WorkspaceMode;
pub use workspace_repo::WorkspaceRepoManager;
pub use worktree::{
    detect_default_branch, sync_remote, sync_remote_with_progress, WorktreeInfo, WorktreeManager,
};
