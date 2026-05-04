//! Work Complete flow state machine.
//!
//! Drives the multi-step dialog that replaces the old "Archive Workspace" confirm:
//!   `LoadingPreflight` → `ReviewingState` → (action-specific sub-phases) → `Done`
//!
//! `transition()` is a pure function: given the current phase and an event it
//! returns the next phase plus a list of abstract commands.  The caller (app.rs)
//! maps each command to a concrete `Effect`.

use uuid::Uuid;

use conduit_git::{ContextSource, MergeReadiness, Scenario, SuggestedAction};

// ---------- Session ----------

/// Active Work Complete dialog context.
#[derive(Debug, Clone)]
pub struct WorkCompleteSession {
    pub workspace_id: Uuid,
    pub phase: WorkCompletePhase,
    /// Loaded after preflight; `None` while loading.
    pub data: Option<WorkCompleteData>,
    /// Accumulated log lines from completed actions.
    pub log: Vec<String>,
    /// Currently highlighted row in the action list.
    pub selected_action_idx: usize,
    /// Text being typed in the commit message input.
    pub commit_message_input: String,
}

impl WorkCompleteSession {
    pub fn new(workspace_id: Uuid) -> Self {
        Self {
            workspace_id,
            phase: WorkCompletePhase::LoadingPreflight,
            data: None,
            log: Vec::new(),
            selected_action_idx: 0,
            commit_message_input: String::new(),
        }
    }
}

// ---------- Preflight data ----------

/// Data returned by the Work Complete preflight check.
#[derive(Debug, Clone)]
pub struct WorkCompleteData {
    pub branch_name: String,
    pub is_dirty: bool,
    pub dirty_files: Vec<String>,
    pub commits_ahead: u32,
    pub commits_behind: u32,
    pub is_merged: bool,
    pub has_upstream: bool,
    pub pr: Option<PrData>,
    pub spec: Option<SpecData>,
    pub issue: Option<IssueData>,
    pub scenario: Scenario,
    pub suggested_actions: Vec<SuggestedAction>,
}

#[derive(Debug, Clone)]
pub struct PrData {
    pub number: u32,
    pub url: Option<String>,
    pub title: Option<String>,
    pub is_open: bool,
    pub is_merged: bool,
    pub merge_readiness: MergeReadiness,
}

#[derive(Debug, Clone)]
pub struct SpecData {
    pub change_id: String,
    pub total: usize,
    pub completed: usize,
    pub source: ContextSource,
}

impl SpecData {
    pub fn is_complete(&self) -> bool {
        self.total > 0 && self.completed == self.total
    }
}

#[derive(Debug, Clone)]
pub struct IssueData {
    pub number: i32,
    pub title: Option<String>,
    pub is_open: bool,
    pub source: ContextSource,
}

// ---------- Phase ----------

/// Phases of the Work Complete dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkCompletePhase {
    /// Fetching git/PR/spec/issue status.
    LoadingPreflight,
    /// Main view: status sections + action list.
    ReviewingState { scenario: Scenario },
    /// User is typing the commit message.
    AwaitingCommitMessage,
    /// Prompting the user to confirm a force-complete action.
    ConfirmingForce {
        kind: ForceKind,
        pending: SuggestedAction,
    },
    /// Action is executing (spinner).
    Executing { action: SuggestedAction },
    /// All done — dialog about to close.
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForceKind {
    SpecIncomplete,
    IssueOpen,
}

// ---------- Events ----------

/// Events that drive Work Complete phase transitions.
#[derive(Debug, Clone)]
pub enum WorkCompleteEvent {
    /// Preflight finished successfully.
    PreflightLoaded(WorkCompleteData),
    /// Preflight errored.
    PreflightFailed(String),
    /// User picked an action from the list.
    ActionSelected(SuggestedAction),
    /// User submitted the commit message.
    CommitMessageSubmitted(String),
    /// User confirmed a force-complete prompt.
    ForceConfirmed,
    /// An action finished with log output.
    ActionCompleted(Vec<String>),
    /// An action failed with an error message.
    ActionFailed(String),
    /// User pressed Esc / closed the dialog.
    Close,
}

// ---------- Commands ----------

/// Abstract commands emitted by transitions.  App.rs converts these to
/// concrete `Effect`s and direct state mutations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkCompleteCommand {
    /// Kick off the preflight Effect.
    FetchPreflight,
    /// Transition to commit-message input with this pre-fill.
    RequestCommitMessage { suggestion: String },
    /// Execute the given action via an Effect (push, PR, issue-close, etc.).
    ExecuteAction(SuggestedAction),
    /// Execute a commit with the supplied message.
    ExecuteCommit(String),
    /// Re-run preflight after an action completes.
    RefreshPreflight,
    /// Dispatch a prompt to the workspace's active agent session.
    SendAgentPrompt(String),
    /// Close the dialog and restore the previous input mode.
    Close,
}

// ---------- Transition ----------

/// Pure phase + command transition.
///
/// Invalid `(phase, event)` combinations are no-ops.
pub fn transition(
    phase: &WorkCompletePhase,
    event: WorkCompleteEvent,
) -> (WorkCompletePhase, Vec<WorkCompleteCommand>) {
    use WorkCompleteCommand as C;
    use WorkCompleteEvent as E;
    use WorkCompletePhase as P;

    match (phase, event) {
        // Preflight done
        (P::LoadingPreflight, E::PreflightLoaded(data)) => {
            let scenario = data.scenario;
            (P::ReviewingState { scenario }, vec![])
        }
        (P::LoadingPreflight, E::PreflightFailed(_)) => (P::Done, vec![C::Close]),

        // User picks an action in the main review view
        (P::ReviewingState { scenario }, E::ActionSelected(action)) => {
            select_action(*scenario, action)
        }

        // Commit message flow
        (P::AwaitingCommitMessage, E::CommitMessageSubmitted(msg)) => (
            P::Executing {
                action: SuggestedAction::Commit,
            },
            vec![C::ExecuteCommit(msg)],
        ),
        (P::AwaitingCommitMessage, E::Close) => {
            // Cancel back to the reviewing state; we need the scenario.
            // We don't have it here — app.rs restores it from session.data.
            (P::Done, vec![C::Close])
        }

        // Force-confirm flow
        (P::ConfirmingForce { pending, .. }, E::ForceConfirmed) => {
            let action = *pending;
            (P::Executing { action }, vec![C::ExecuteAction(action)])
        }
        (
            P::ConfirmingForce {
                pending: _,
                kind: _,
            },
            E::Close,
        ) => {
            // Cancel force-confirm, go back to reviewing.
            // App.rs will re-set the phase from session.data.
            (P::Done, vec![C::Close])
        }

        // Action results
        (P::Executing { .. }, E::ActionCompleted(_)) => {
            (P::LoadingPreflight, vec![C::RefreshPreflight])
        }
        (P::Executing { .. }, E::ActionFailed(_)) => {
            // Stay at the phase; app.rs will log the error and go back to reviewing.
            (P::Done, vec![C::Close])
        }

        // Universal close — but not while an action is in flight
        (P::Executing { action }, E::Close) => (P::Executing { action: *action }, vec![]),
        (_, E::Close) => (P::Done, vec![C::Close]),

        // Stale / invalid combos are no-ops.
        (current, _) => (current.clone(), vec![]),
    }
}

fn select_action(
    scenario: Scenario,
    action: SuggestedAction,
) -> (WorkCompletePhase, Vec<WorkCompleteCommand>) {
    use WorkCompleteCommand as C;
    use WorkCompletePhase as P;

    // Some actions need force-confirm when the state is "incomplete"
    let needs_force = match action {
        SuggestedAction::ArchiveSpec | SuggestedAction::Archive => {
            matches!(scenario, Scenario::SpecIncomplete | Scenario::IssueOpen)
        }
        _ => false,
    };

    if needs_force {
        let kind = if matches!(scenario, Scenario::SpecIncomplete) {
            ForceKind::SpecIncomplete
        } else {
            ForceKind::IssueOpen
        };
        return (
            P::ConfirmingForce {
                kind,
                pending: action,
            },
            vec![],
        );
    }

    if action == SuggestedAction::Commit {
        return (
            P::AwaitingCommitMessage,
            vec![C::RequestCommitMessage {
                suggestion: String::new(), // app.rs fills this from session.data
            }],
        );
    }

    if action == SuggestedAction::ShowRemainingTasks {
        // app.rs builds the actual prompt from session.data
        return (P::Done, vec![C::SendAgentPrompt(String::new()), C::Close]);
    }

    (P::Executing { action }, vec![C::ExecuteAction(action)])
}

// ---------- Commit message suggestion ----------

/// Build a pre-filled commit message from workspace context.
pub fn suggest_commit_message(
    branch: &str,
    dirty_files: &[impl AsRef<str>],
    change_id: Option<&str>,
    issue_number: Option<i32>,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(id) = change_id {
        parts.push(format!("Implement {}", id));
    }

    if let Some(n) = issue_number {
        parts.push(format!("Fix #{}", n));
    }

    if parts.is_empty() {
        // Fall back to branch name as summary
        let summary = branch
            .split('/')
            .next_back()
            .unwrap_or(branch)
            .replace(['-', '_'], " ");
        parts.push(summary);
    }

    if !dirty_files.is_empty() {
        let snippets: Vec<&str> = dirty_files.iter().take(2).map(|f| f.as_ref()).collect();
        parts.push(snippets.join(", "));
    }

    parts.join("; ")
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    fn reviewing(scenario: Scenario) -> WorkCompletePhase {
        WorkCompletePhase::ReviewingState { scenario }
    }

    fn data(scenario: Scenario) -> WorkCompleteData {
        WorkCompleteData {
            branch_name: "fuz/feat".to_string(),
            is_dirty: false,
            dirty_files: vec![],
            commits_ahead: 0,
            commits_behind: 0,
            is_merged: false,
            has_upstream: true,
            pr: None,
            spec: None,
            issue: None,
            scenario,
            suggested_actions: vec![SuggestedAction::Archive],
        }
    }

    #[test]
    fn preflight_loaded_transitions_to_reviewing() {
        let (phase, cmds) = transition(
            &WorkCompletePhase::LoadingPreflight,
            WorkCompleteEvent::PreflightLoaded(data(Scenario::CleanReady)),
        );
        assert_eq!(phase, reviewing(Scenario::CleanReady));
        assert!(cmds.is_empty());
    }

    #[test]
    fn preflight_failed_closes() {
        let (phase, cmds) = transition(
            &WorkCompletePhase::LoadingPreflight,
            WorkCompleteEvent::PreflightFailed("oops".to_string()),
        );
        assert_eq!(phase, WorkCompletePhase::Done);
        assert!(cmds.contains(&WorkCompleteCommand::Close));
    }

    #[test]
    fn commit_action_goes_to_awaiting_message() {
        let (phase, cmds) = transition(
            &reviewing(Scenario::EditsNoLink),
            WorkCompleteEvent::ActionSelected(SuggestedAction::Commit),
        );
        assert_eq!(phase, WorkCompletePhase::AwaitingCommitMessage);
        assert!(cmds
            .iter()
            .any(|c| matches!(c, WorkCompleteCommand::RequestCommitMessage { .. })));
    }

    #[test]
    fn push_action_executes_directly() {
        let (phase, cmds) = transition(
            &reviewing(Scenario::EditsNoLink),
            WorkCompleteEvent::ActionSelected(SuggestedAction::Push),
        );
        assert_eq!(
            phase,
            WorkCompletePhase::Executing {
                action: SuggestedAction::Push
            }
        );
        assert!(cmds.contains(&WorkCompleteCommand::ExecuteAction(SuggestedAction::Push)));
    }

    #[test]
    fn archive_requires_force_confirm_when_spec_incomplete() {
        let (phase, cmds) = transition(
            &reviewing(Scenario::SpecIncomplete),
            WorkCompleteEvent::ActionSelected(SuggestedAction::Archive),
        );
        assert!(matches!(
            phase,
            WorkCompletePhase::ConfirmingForce {
                kind: ForceKind::SpecIncomplete,
                ..
            }
        ));
        assert!(cmds.is_empty());
    }

    #[test]
    fn archive_requires_force_confirm_when_issue_open() {
        let (phase, cmds) = transition(
            &reviewing(Scenario::IssueOpen),
            WorkCompleteEvent::ActionSelected(SuggestedAction::Archive),
        );
        assert!(matches!(
            phase,
            WorkCompletePhase::ConfirmingForce {
                kind: ForceKind::IssueOpen,
                ..
            }
        ));
        assert!(cmds.is_empty());
    }

    #[test]
    fn archive_does_not_require_force_when_clean() {
        let (phase, cmds) = transition(
            &reviewing(Scenario::CleanReady),
            WorkCompleteEvent::ActionSelected(SuggestedAction::Archive),
        );
        assert_eq!(
            phase,
            WorkCompletePhase::Executing {
                action: SuggestedAction::Archive
            }
        );
        assert!(cmds.contains(&WorkCompleteCommand::ExecuteAction(
            SuggestedAction::Archive
        )));
    }

    #[test]
    fn force_confirmed_executes_pending_action() {
        let (phase, cmds) = transition(
            &WorkCompletePhase::ConfirmingForce {
                kind: ForceKind::SpecIncomplete,
                pending: SuggestedAction::Archive,
            },
            WorkCompleteEvent::ForceConfirmed,
        );
        assert_eq!(
            phase,
            WorkCompletePhase::Executing {
                action: SuggestedAction::Archive
            }
        );
        assert!(cmds.contains(&WorkCompleteCommand::ExecuteAction(
            SuggestedAction::Archive
        )));
    }

    #[test]
    fn action_completed_triggers_refresh() {
        let (phase, cmds) = transition(
            &WorkCompletePhase::Executing {
                action: SuggestedAction::Push,
            },
            WorkCompleteEvent::ActionCompleted(vec!["Pushed".to_string()]),
        );
        assert_eq!(phase, WorkCompletePhase::LoadingPreflight);
        assert!(cmds.contains(&WorkCompleteCommand::RefreshPreflight));
    }

    #[test]
    fn close_from_any_phase_is_done() {
        for phase in [
            WorkCompletePhase::LoadingPreflight,
            reviewing(Scenario::CleanReady),
            WorkCompletePhase::AwaitingCommitMessage,
        ] {
            let (next, cmds) = transition(&phase, WorkCompleteEvent::Close);
            assert_eq!(next, WorkCompletePhase::Done, "phase: {:?}", phase);
            assert!(
                cmds.contains(&WorkCompleteCommand::Close),
                "phase: {:?}",
                phase
            );
        }
    }

    #[test]
    fn commit_message_submitted_executes_commit() {
        let (phase, cmds) = transition(
            &WorkCompletePhase::AwaitingCommitMessage,
            WorkCompleteEvent::CommitMessageSubmitted("feat: add thing".to_string()),
        );
        assert_eq!(
            phase,
            WorkCompletePhase::Executing {
                action: SuggestedAction::Commit
            }
        );
        assert!(cmds.contains(&WorkCompleteCommand::ExecuteCommit(
            "feat: add thing".to_string()
        )));
    }

    #[test]
    fn suggest_commit_with_change_and_issue() {
        let msg = suggest_commit_message("fuz/feat", &["src/main.rs"], Some("my-change"), Some(42));
        assert!(msg.contains("Implement my-change"));
        assert!(msg.contains("Fix #42"));
    }

    #[test]
    fn suggest_commit_falls_back_to_branch() {
        let msg = suggest_commit_message("fuz/feat-add-thing", &[] as &[&str], None, None);
        assert!(msg.contains("feat add thing") || msg.contains("add"));
    }

    #[test]
    fn action_failed_closes_dialog() {
        let (phase, cmds) = transition(
            &WorkCompletePhase::Executing {
                action: SuggestedAction::Push,
            },
            WorkCompleteEvent::ActionFailed("push rejected".to_string()),
        );
        assert_eq!(phase, WorkCompletePhase::Done);
        assert!(cmds.contains(&WorkCompleteCommand::Close));
    }

    #[test]
    fn cancel_from_confirming_force_closes() {
        let (phase, cmds) = transition(
            &WorkCompletePhase::ConfirmingForce {
                kind: ForceKind::IssueOpen,
                pending: SuggestedAction::Archive,
            },
            WorkCompleteEvent::Close,
        );
        assert_eq!(phase, WorkCompletePhase::Done);
        assert!(cmds.contains(&WorkCompleteCommand::Close));
    }

    #[test]
    fn cancel_from_executing_is_noop() {
        let (phase, cmds) = transition(
            &WorkCompletePhase::Executing {
                action: SuggestedAction::Push,
            },
            WorkCompleteEvent::Close,
        );
        // Close is ignored while an action is in flight
        assert_eq!(
            phase,
            WorkCompletePhase::Executing {
                action: SuggestedAction::Push
            }
        );
        assert!(cmds.is_empty());
    }

    #[test]
    fn spec_archive_requires_force_when_spec_incomplete() {
        let (phase, cmds) = transition(
            &reviewing(Scenario::SpecIncomplete),
            WorkCompleteEvent::ActionSelected(SuggestedAction::ArchiveSpec),
        );
        assert!(
            matches!(
                phase,
                WorkCompletePhase::ConfirmingForce {
                    kind: ForceKind::SpecIncomplete,
                    ..
                }
            ),
            "expected ConfirmingForce, got {:?}",
            phase
        );
        assert!(cmds.is_empty());
    }

    #[test]
    fn archive_does_not_require_force_when_spec_complete() {
        let (phase, cmds) = transition(
            &reviewing(Scenario::SpecComplete),
            WorkCompleteEvent::ActionSelected(SuggestedAction::Archive),
        );
        assert_eq!(
            phase,
            WorkCompletePhase::Executing {
                action: SuggestedAction::Archive
            }
        );
        assert!(cmds.contains(&WorkCompleteCommand::ExecuteAction(
            SuggestedAction::Archive
        )));
    }

    #[test]
    fn stale_event_on_wrong_phase_is_noop() {
        // PreflightLoaded arriving while already ReviewingState → no-op
        let phase = reviewing(Scenario::CleanReady);
        let (next, cmds) = transition(
            &phase,
            WorkCompleteEvent::PreflightLoaded(data(Scenario::EditsNoLink)),
        );
        assert_eq!(next, phase);
        assert!(cmds.is_empty());
    }

    #[test]
    fn happy_path_commit_walk() {
        // LoadingPreflight → ReviewingState
        let (phase, _) = transition(
            &WorkCompletePhase::LoadingPreflight,
            WorkCompleteEvent::PreflightLoaded(data(Scenario::EditsNoLink)),
        );
        assert_eq!(phase, reviewing(Scenario::EditsNoLink));

        // ReviewingState → AwaitingCommitMessage
        let (phase, cmds) = transition(
            &phase,
            WorkCompleteEvent::ActionSelected(SuggestedAction::Commit),
        );
        assert_eq!(phase, WorkCompletePhase::AwaitingCommitMessage);
        assert!(cmds
            .iter()
            .any(|c| matches!(c, WorkCompleteCommand::RequestCommitMessage { .. })));

        // AwaitingCommitMessage → Executing
        let (phase, cmds) = transition(
            &phase,
            WorkCompleteEvent::CommitMessageSubmitted("feat: thing".to_string()),
        );
        assert_eq!(
            phase,
            WorkCompletePhase::Executing {
                action: SuggestedAction::Commit
            }
        );
        assert!(cmds.contains(&WorkCompleteCommand::ExecuteCommit(
            "feat: thing".to_string()
        )));

        // Executing → LoadingPreflight (refresh loop)
        let (phase, cmds) = transition(
            &phase,
            WorkCompleteEvent::ActionCompleted(vec!["committed".to_string()]),
        );
        assert_eq!(phase, WorkCompletePhase::LoadingPreflight);
        assert!(cmds.contains(&WorkCompleteCommand::RefreshPreflight));
    }

    #[test]
    fn happy_path_force_archive_walk() {
        // ReviewingState(SpecIncomplete) → ConfirmingForce
        let (phase, cmds) = transition(
            &reviewing(Scenario::SpecIncomplete),
            WorkCompleteEvent::ActionSelected(SuggestedAction::Archive),
        );
        assert!(matches!(
            phase,
            WorkCompletePhase::ConfirmingForce {
                kind: ForceKind::SpecIncomplete,
                ..
            }
        ));
        assert!(cmds.is_empty());

        // ConfirmingForce → Executing
        let (phase, cmds) = transition(&phase, WorkCompleteEvent::ForceConfirmed);
        assert_eq!(
            phase,
            WorkCompletePhase::Executing {
                action: SuggestedAction::Archive
            }
        );
        assert!(cmds.contains(&WorkCompleteCommand::ExecuteAction(
            SuggestedAction::Archive
        )));

        // Executing → LoadingPreflight
        let (phase, cmds) = transition(
            &phase,
            WorkCompleteEvent::ActionCompleted(vec!["archived".to_string()]),
        );
        assert_eq!(phase, WorkCompletePhase::LoadingPreflight);
        assert!(cmds.contains(&WorkCompleteCommand::RefreshPreflight));
    }
}
