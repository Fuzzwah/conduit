//! Snapshot tests for `WorkCompleteDialog` phase rendering.

#[cfg(test)]
mod tests {
    use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
    use uuid::Uuid;

    use crate::components::WorkCompleteDialog;
    use crate::work_complete::{
        ForceKind, IssueData, PrData, SpecData, WorkCompleteData, WorkCompletePhase,
        WorkCompleteSession,
    };
    use conduit_git::{ContextSource, MergeReadiness, Scenario, SuggestedAction};

    const WIDTH: u16 = 80;
    const HEIGHT: u16 = 30;

    fn render_phase(phase: WorkCompletePhase, data: Option<WorkCompleteData>) -> String {
        let mut session = WorkCompleteSession::new(Uuid::nil());
        session.phase = phase;
        session.data = data;

        let dialog = WorkCompleteDialog::new(&session, 0);
        let area = Rect::new(0, 0, WIDTH, HEIGHT);
        let mut buf = Buffer::empty(area);
        dialog.render(area, &mut buf);

        buf_to_string(&buf)
    }

    fn buf_to_string(buf: &Buffer) -> String {
        let area = buf.area;
        let mut lines = Vec::new();
        for y in 0..area.height {
            let mut line = String::new();
            for x in 0..area.width {
                line.push_str(buf[(x, y)].symbol());
            }
            lines.push(line.trim_end().to_string());
        }
        // Drop trailing blank lines
        while lines.last().map(|l: &String| l.is_empty()).unwrap_or(false) {
            lines.pop();
        }
        lines.join("\n")
    }

    fn clean_data() -> WorkCompleteData {
        WorkCompleteData {
            branch_name: "fuz/my-feature".to_string(),
            is_dirty: false,
            dirty_files: vec![],
            commits_ahead: 0,
            commits_behind: 0,
            is_merged: false,
            has_upstream: true,
            pr: None,
            spec: None,
            issue: None,
            scenario: Scenario::CleanReady,
            suggested_actions: vec![SuggestedAction::Archive],
            adversarial_review_model: None,
        }
    }

    fn edits_data() -> WorkCompleteData {
        WorkCompleteData {
            is_dirty: true,
            dirty_files: vec!["src/main.rs".to_string()],
            commits_ahead: 1,
            scenario: Scenario::EditsNoLink,
            suggested_actions: vec![
                SuggestedAction::Commit,
                SuggestedAction::Push,
                SuggestedAction::Archive,
            ],
            ..clean_data()
        }
    }

    fn spec_complete_data() -> WorkCompleteData {
        WorkCompleteData {
            scenario: Scenario::SpecComplete,
            spec: Some(SpecData {
                change_id: "my-feature".to_string(),
                total: 5,
                completed: 5,
                source: ContextSource::Linked,
            }),
            suggested_actions: vec![SuggestedAction::ArchiveSpec, SuggestedAction::Archive],
            ..clean_data()
        }
    }

    fn spec_incomplete_data() -> WorkCompleteData {
        WorkCompleteData {
            scenario: Scenario::SpecIncomplete,
            spec: Some(SpecData {
                change_id: "my-feature".to_string(),
                total: 8,
                completed: 3,
                source: ContextSource::Detected,
            }),
            suggested_actions: vec![
                SuggestedAction::ShowRemainingTasks,
                SuggestedAction::Archive,
            ],
            ..clean_data()
        }
    }

    fn issue_open_data() -> WorkCompleteData {
        WorkCompleteData {
            scenario: Scenario::IssueOpen,
            issue: Some(IssueData {
                number: 42,
                title: Some("Add feature".to_string()),
                is_open: true,
                source: ContextSource::Linked,
            }),
            suggested_actions: vec![SuggestedAction::CloseIssue, SuggestedAction::Archive],
            ..clean_data()
        }
    }

    fn issue_closed_data() -> WorkCompleteData {
        WorkCompleteData {
            scenario: Scenario::IssueClosed,
            issue: Some(IssueData {
                number: 42,
                title: Some("Add feature".to_string()),
                is_open: false,
                source: ContextSource::Linked,
            }),
            suggested_actions: vec![SuggestedAction::Archive],
            ..clean_data()
        }
    }

    fn pr_data() -> PrData {
        PrData {
            number: 7,
            url: Some("https://github.com/owner/repo/pull/7".to_string()),
            title: Some("My feature PR".to_string()),
            is_open: true,
            is_merged: false,
            merge_readiness: MergeReadiness::Ready,
        }
    }

    #[test]
    fn snapshot_loading_preflight() {
        let output = render_phase(WorkCompletePhase::LoadingPreflight, None);
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_reviewing_clean_ready() {
        let output = render_phase(
            WorkCompletePhase::ReviewingState {
                scenario: Scenario::CleanReady,
            },
            Some(clean_data()),
        );
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_reviewing_edits_no_link() {
        let output = render_phase(
            WorkCompletePhase::ReviewingState {
                scenario: Scenario::EditsNoLink,
            },
            Some(edits_data()),
        );
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_reviewing_spec_complete() {
        let output = render_phase(
            WorkCompletePhase::ReviewingState {
                scenario: Scenario::SpecComplete,
            },
            Some(spec_complete_data()),
        );
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_reviewing_spec_incomplete() {
        let output = render_phase(
            WorkCompletePhase::ReviewingState {
                scenario: Scenario::SpecIncomplete,
            },
            Some(spec_incomplete_data()),
        );
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_reviewing_issue_open() {
        let output = render_phase(
            WorkCompletePhase::ReviewingState {
                scenario: Scenario::IssueOpen,
            },
            Some(issue_open_data()),
        );
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_reviewing_issue_closed() {
        let output = render_phase(
            WorkCompletePhase::ReviewingState {
                scenario: Scenario::IssueClosed,
            },
            Some(issue_closed_data()),
        );
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_reviewing_spec_complete_with_pr() {
        let mut data = spec_complete_data();
        data.pr = Some(pr_data());
        data.suggested_actions = vec![
            SuggestedAction::MergePr,
            SuggestedAction::ArchiveSpec,
            SuggestedAction::Archive,
        ];
        let output = render_phase(
            WorkCompletePhase::ReviewingState {
                scenario: Scenario::SpecComplete,
            },
            Some(data),
        );
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_awaiting_commit_message() {
        let mut session = WorkCompleteSession::new(Uuid::nil());
        session.phase = WorkCompletePhase::AwaitingCommitMessage;
        session.data = Some(edits_data());
        session.commit_message_input = "Implement my-feature; src/main.rs".to_string();

        let area = Rect::new(0, 0, WIDTH, HEIGHT);
        let mut buf = Buffer::empty(area);
        WorkCompleteDialog::new(&session, 0).render(area, &mut buf);
        insta::assert_snapshot!(buf_to_string(&buf));
    }

    #[test]
    fn snapshot_confirming_force_spec() {
        let output = render_phase(
            WorkCompletePhase::ConfirmingForce {
                kind: ForceKind::SpecIncomplete,
                pending: SuggestedAction::Archive,
            },
            Some(spec_incomplete_data()),
        );
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_confirming_force_issue() {
        let output = render_phase(
            WorkCompletePhase::ConfirmingForce {
                kind: ForceKind::IssueOpen,
                pending: SuggestedAction::Archive,
            },
            Some(issue_open_data()),
        );
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_executing() {
        let output = render_phase(
            WorkCompletePhase::Executing {
                action: SuggestedAction::Push,
            },
            Some(edits_data()),
        );
        insta::assert_snapshot!(output);
    }
}
