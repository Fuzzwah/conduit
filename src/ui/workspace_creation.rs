//! Workspace creation state machine.
//!
//! Drives the strict sequence:
//!   `Idle` → `SyncingRemote` → `FetchingIssues` → (`PickingIssue`?) →
//!   `FetchingSpecs` → (`PickingSpec`?) → `Naming`.
//!
//! `transition()` is a pure function: given a phase and an event it returns
//! the next phase plus a list of abstract commands. The caller (`app.rs`)
//! maps each command to a concrete `Effect`. This keeps the ordering rules
//! testable in isolation and prevents the historical race where spec scans
//! kicked off before `git fetch` finished.

use uuid::Uuid;

use crate::git::{OpenSpec, RemoteIssue, SpecifySpec};

/// Active workspace-creation flow context.
///
/// Tracks the repo being targeted, the current phase, and the user's
/// picks at each stage so the final `CreateWorkspace` effect can be
/// assembled from a single source.
#[derive(Debug, Clone)]
pub struct WorkspaceCreationSession {
    pub repo_id: Uuid,
    pub phase: WorkspaceCreationPhase,
    pub picked_issue: Option<RemoteIssue>,
    pub picked_spec: Option<OpenSpec>,
    pub picked_specify_spec: Option<SpecifySpec>,
}

impl WorkspaceCreationSession {
    pub fn new(repo_id: Uuid) -> Self {
        Self {
            repo_id,
            phase: WorkspaceCreationPhase::Idle,
            picked_issue: None,
            picked_spec: None,
            picked_specify_spec: None,
        }
    }
}

/// Phases of workspace creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceCreationPhase {
    /// No creation in progress.
    Idle,
    /// Running `git fetch` (and opportunistic FF) on the base path.
    SyncingRemote,
    /// Fetching open issues from the remote provider.
    FetchingIssues,
    /// User is choosing an issue (or about to dismiss with Esc).
    PickingIssue,
    /// Fetching openspec + spec-kit specs from `origin/<default>`.
    FetchingSpecs,
    /// User is choosing a spec (or about to dismiss with Esc).
    PickingSpec,
    /// Ready to create the workspace (name + branch resolution).
    Naming,
}

/// Events that drive phase transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceCreationEvent {
    /// User initiated workspace creation (Alt+N on a repo row).
    Start,
    /// `git fetch` finished (success or failure — both advance the flow).
    RemoteSynced,
    /// Issue list arrived; `has_issues == true` when the picker should appear.
    IssuesFetched { has_issues: bool },
    /// User selected an issue or pressed Esc on the issue picker.
    IssuePicked,
    /// Spec list arrived; `has_specs == true` when either picker has results.
    SpecsFetched { has_specs: bool },
    /// User selected a spec or pressed Esc on the spec picker.
    SpecPicked,
}

/// Abstract effects emitted by transitions.
///
/// These are intentionally provider-agnostic; `app.rs` resolves the
/// `repo_id`, base path, and origin ref at the point it converts each
/// command to a concrete `Effect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceCreationCommand {
    /// Run `git fetch` (and opportunistic FF) on the base path.
    SyncRemote,
    /// Fetch open issues via the matching `IssueProvider`.
    FetchRemoteIssues,
    /// Fetch openspec + spec-kit specs from `origin/<default>`.
    FetchAllSpecs,
    /// Reveal the issue picker (issues already loaded into state).
    ShowIssuePicker,
    /// Reveal the spec picker (specs already loaded into state).
    ShowSpecPicker,
    /// Hand off to the naming + create-workspace step.
    StartNaming,
}

/// Pure phase + effect transition.
///
/// Invalid `(phase, event)` combinations are no-ops: the phase is preserved
/// and no commands are emitted. This makes the function safe to call for
/// every event without the caller needing to gate on the current phase.
pub fn transition(
    phase: WorkspaceCreationPhase,
    event: WorkspaceCreationEvent,
) -> (WorkspaceCreationPhase, Vec<WorkspaceCreationCommand>) {
    use WorkspaceCreationCommand as C;
    use WorkspaceCreationEvent as E;
    use WorkspaceCreationPhase as P;

    match (phase, event) {
        (P::Idle, E::Start) => (P::SyncingRemote, vec![C::SyncRemote]),

        (P::SyncingRemote, E::RemoteSynced) => (P::FetchingIssues, vec![C::FetchRemoteIssues]),

        (P::FetchingIssues, E::IssuesFetched { has_issues: true }) => {
            (P::PickingIssue, vec![C::ShowIssuePicker])
        }
        (P::FetchingIssues, E::IssuesFetched { has_issues: false }) => {
            (P::FetchingSpecs, vec![C::FetchAllSpecs])
        }

        (P::PickingIssue, E::IssuePicked) => (P::FetchingSpecs, vec![C::FetchAllSpecs]),

        (P::FetchingSpecs, E::SpecsFetched { has_specs: true }) => {
            (P::PickingSpec, vec![C::ShowSpecPicker])
        }
        (P::FetchingSpecs, E::SpecsFetched { has_specs: false }) => {
            (P::Naming, vec![C::StartNaming])
        }

        (P::PickingSpec, E::SpecPicked) => (P::Naming, vec![C::StartNaming]),

        // Anything else (e.g. a stale event arriving after the user already
        // advanced past the relevant phase) is silently ignored.
        (current, _) => (current, Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use WorkspaceCreationCommand as C;
    use WorkspaceCreationEvent as E;
    use WorkspaceCreationPhase as P;

    fn step(phase: P, event: E) -> (P, Vec<C>) {
        transition(phase, event)
    }

    #[test]
    fn happy_path_with_issue_and_spec() {
        let (phase, cmds) = step(P::Idle, E::Start);
        assert_eq!(phase, P::SyncingRemote);
        assert_eq!(cmds, vec![C::SyncRemote]);

        let (phase, cmds) = step(phase, E::RemoteSynced);
        assert_eq!(phase, P::FetchingIssues);
        assert_eq!(cmds, vec![C::FetchRemoteIssues]);

        let (phase, cmds) = step(phase, E::IssuesFetched { has_issues: true });
        assert_eq!(phase, P::PickingIssue);
        assert_eq!(cmds, vec![C::ShowIssuePicker]);

        let (phase, cmds) = step(phase, E::IssuePicked);
        assert_eq!(phase, P::FetchingSpecs);
        assert_eq!(cmds, vec![C::FetchAllSpecs]);

        let (phase, cmds) = step(phase, E::SpecsFetched { has_specs: true });
        assert_eq!(phase, P::PickingSpec);
        assert_eq!(cmds, vec![C::ShowSpecPicker]);

        let (phase, cmds) = step(phase, E::SpecPicked);
        assert_eq!(phase, P::Naming);
        assert_eq!(cmds, vec![C::StartNaming]);
    }

    #[test]
    fn empty_issues_skips_to_spec_fetch() {
        let (phase, cmds) = step(P::FetchingIssues, E::IssuesFetched { has_issues: false });
        assert_eq!(phase, P::FetchingSpecs);
        assert_eq!(cmds, vec![C::FetchAllSpecs]);
    }

    #[test]
    fn empty_specs_skips_to_naming() {
        let (phase, cmds) = step(P::FetchingSpecs, E::SpecsFetched { has_specs: false });
        assert_eq!(phase, P::Naming);
        assert_eq!(cmds, vec![C::StartNaming]);
    }

    #[test]
    fn esc_on_issue_picker_advances_to_specs() {
        // The picker treats Esc and Enter the same: both fire `IssuePicked`
        // (the picked issue is None on Esc), and the state machine moves
        // forward unconditionally.
        let (phase, cmds) = step(P::PickingIssue, E::IssuePicked);
        assert_eq!(phase, P::FetchingSpecs);
        assert_eq!(cmds, vec![C::FetchAllSpecs]);
    }

    #[test]
    fn esc_on_spec_picker_advances_to_naming() {
        let (phase, cmds) = step(P::PickingSpec, E::SpecPicked);
        assert_eq!(phase, P::Naming);
        assert_eq!(cmds, vec![C::StartNaming]);
    }

    #[test]
    fn sync_failure_still_advances() {
        // `sync_remote()` is fire-and-forget: it always emits RemoteSynced,
        // success or failure. The state machine therefore can't distinguish
        // and must always advance to FetchingIssues.
        let (phase, cmds) = step(P::SyncingRemote, E::RemoteSynced);
        assert_eq!(phase, P::FetchingIssues);
        assert_eq!(cmds, vec![C::FetchRemoteIssues]);
    }

    #[test]
    fn happy_path_no_issues_no_specs() {
        let mut phase = P::Idle;
        let (next, _) = step(phase, E::Start);
        phase = next;
        let (next, _) = step(phase, E::RemoteSynced);
        phase = next;
        let (next, _) = step(phase, E::IssuesFetched { has_issues: false });
        phase = next;
        assert_eq!(phase, P::FetchingSpecs);
        let (next, cmds) = step(phase, E::SpecsFetched { has_specs: false });
        assert_eq!(next, P::Naming);
        assert_eq!(cmds, vec![C::StartNaming]);
    }

    #[test]
    fn stale_events_are_ignored() {
        // A late `RemoteSynced` arriving after we already advanced should
        // not regress us back to FetchingIssues.
        let (phase, cmds) = step(P::PickingSpec, E::RemoteSynced);
        assert_eq!(phase, P::PickingSpec);
        assert!(cmds.is_empty());

        // Likewise, IssuesFetched arriving while picking a spec must be a no-op.
        let (phase, cmds) = step(P::PickingSpec, E::IssuesFetched { has_issues: true });
        assert_eq!(phase, P::PickingSpec);
        assert!(cmds.is_empty());
    }

    #[test]
    fn start_only_fires_from_idle() {
        // Starting again mid-flow must not reset us.
        let (phase, cmds) = step(P::PickingIssue, E::Start);
        assert_eq!(phase, P::PickingIssue);
        assert!(cmds.is_empty());
    }
}
