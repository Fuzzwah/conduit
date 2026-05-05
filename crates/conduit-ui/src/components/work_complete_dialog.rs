//! Work Complete dialog component.
//!
//! Renders the multi-phase Work Complete flow: loading → reviewing state →
//! commit message input → force-confirm → executing → done.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};

use super::{
    accent_error, accent_primary, accent_success, accent_warning, bg_highlight, dialog_bg,
    ensure_contrast_fg, text_muted, text_primary, text_secondary, DialogFrame,
};
use crate::work_complete::{ForceKind, WorkCompleteData, WorkCompletePhase, WorkCompleteSession};
use conduit_git::{Scenario, SuggestedAction};

const DIALOG_WIDTH: u16 = 72;
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Widget rendering the Work Complete dialog from a `WorkCompleteSession`.
pub struct WorkCompleteDialog<'a> {
    pub session: &'a WorkCompleteSession,
    /// Spinner tick counter (from AppState).
    pub spinner_frame: usize,
}

impl<'a> WorkCompleteDialog<'a> {
    pub fn new(session: &'a WorkCompleteSession, spinner_frame: usize) -> Self {
        Self {
            session,
            spinner_frame,
        }
    }

    fn dialog_height(&self) -> u16 {
        match &self.session.phase {
            WorkCompletePhase::LoadingPreflight => 7,
            WorkCompletePhase::ReviewingState { .. } => {
                if let Some(data) = &self.session.data {
                    compute_review_height(data)
                } else {
                    7
                }
            }
            WorkCompletePhase::AwaitingCommitMessage => 10,
            WorkCompletePhase::ConfirmingForce { .. } => 10,
            WorkCompletePhase::Executing { .. } => 7,
            WorkCompletePhase::Done => 5,
        }
    }
}

impl Widget for WorkCompleteDialog<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let phase = &self.session.phase;

        let (title, instructions): (&str, Vec<(&str, &str)>) = match phase {
            WorkCompletePhase::LoadingPreflight => ("Work Complete", vec![("Esc", "Cancel")]),
            WorkCompletePhase::ReviewingState { .. } => (
                "Work Complete",
                vec![("↑↓", "select"), ("Enter", "run"), ("Esc", "close")],
            ),
            WorkCompletePhase::AwaitingCommitMessage => (
                "Commit Message",
                vec![("Enter", "commit"), ("Esc", "cancel")],
            ),
            WorkCompletePhase::ConfirmingForce { .. } => (
                "Work Complete — Confirm",
                vec![("Enter", "confirm"), ("Esc", "cancel")],
            ),
            WorkCompletePhase::Executing { .. } => ("Work Complete", vec![]),
            WorkCompletePhase::Done => ("Work Complete", vec![]),
        };

        let inner = DialogFrame::new(title, DIALOG_WIDTH, self.dialog_height())
            .instructions(instructions)
            .render(area, buf);

        if inner.height == 0 {
            return;
        }

        match phase {
            WorkCompletePhase::LoadingPreflight | WorkCompletePhase::Executing { .. } => {
                render_spinner(inner, buf, self.spinner_frame, phase);
            }
            WorkCompletePhase::ReviewingState { .. } => {
                if let Some(data) = &self.session.data {
                    render_review(inner, buf, data, self.session.selected_action_idx);
                }
            }
            WorkCompletePhase::AwaitingCommitMessage => {
                render_commit_input(inner, buf, &self.session.commit_message_input);
            }
            WorkCompletePhase::ConfirmingForce { kind, pending } => {
                render_force_confirm(inner, buf, *kind, *pending);
            }
            WorkCompletePhase::Done => {}
        }
    }
}

// ---------- Spinner ----------

fn render_spinner(inner: Rect, buf: &mut Buffer, frame: usize, phase: &WorkCompletePhase) {
    let spinner = SPINNER_FRAMES[frame % SPINNER_FRAMES.len()];
    let label = match phase {
        WorkCompletePhase::LoadingPreflight => "Analyzing workspace…",
        WorkCompletePhase::Executing { action } => match action {
            SuggestedAction::Commit => "Committing…",
            SuggestedAction::Push => "Pushing…",
            SuggestedAction::OpenPr => "Opening PR…",
            SuggestedAction::MergePr => "Merging PR…",
            SuggestedAction::CloseIssue => "Closing issue…",
            SuggestedAction::ArchiveSpec => "Archiving spec…",
            SuggestedAction::Archive => "Archiving workspace…",
            SuggestedAction::ShowRemainingTasks => "Working…",
        },
        _ => "Working…",
    };

    let line = Line::from(vec![
        Span::styled(
            format!("{} ", spinner),
            Style::default().fg(accent_primary()),
        ),
        Span::styled(label, Style::default().fg(text_primary())),
    ]);
    Paragraph::new(line).render(
        Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
        buf,
    );
}

// ---------- Review ----------

fn render_review(inner: Rect, buf: &mut Buffer, data: &WorkCompleteData, selected: usize) {
    let mut y = inner.y;
    let w = inner.width;

    // --- Scenario badge ---
    let scenario_text = scenario_label(data.scenario);
    let scenario_color = scenario_color(data.scenario);
    let badge = Line::from(Span::styled(
        scenario_text,
        Style::default().fg(scenario_color),
    ));
    Paragraph::new(badge).render(
        Rect {
            x: inner.x,
            y,
            width: w,
            height: 1,
        },
        buf,
    );
    y += 1;

    // --- Branch line ---
    let branch_style = Style::default().fg(text_secondary());
    let mut branch_spans = vec![
        Span::styled("  branch: ", Style::default().fg(text_muted())),
        Span::styled(data.branch_name.clone(), branch_style),
    ];
    if data.commits_ahead > 0 || data.commits_behind > 0 {
        branch_spans.push(Span::styled(
            format!("  ↑{} ↓{}", data.commits_ahead, data.commits_behind),
            Style::default().fg(text_muted()),
        ));
    }
    if data.is_dirty {
        branch_spans.push(Span::styled(
            format!("  {} modified", data.dirty_files.len()),
            Style::default().fg(accent_warning()),
        ));
    }
    Paragraph::new(Line::from(branch_spans)).render(
        Rect {
            x: inner.x,
            y,
            width: w,
            height: 1,
        },
        buf,
    );
    y += 1;

    // --- PR line ---
    if let Some(pr) = &data.pr {
        let pr_color = if pr.is_merged {
            accent_success()
        } else if pr.is_open {
            accent_primary()
        } else {
            text_muted()
        };
        let pr_state = if pr.is_merged {
            "merged"
        } else if pr.is_open {
            "open"
        } else {
            "closed"
        };
        let pr_title = pr.title.as_deref().unwrap_or("");
        let pr_line = Line::from(vec![
            Span::styled("  PR #", Style::default().fg(text_muted())),
            Span::styled(pr.number.to_string(), Style::default().fg(pr_color)),
            Span::styled(format!(" ({pr_state})"), Style::default().fg(pr_color)),
            Span::styled(
                if pr_title.is_empty() {
                    String::new()
                } else {
                    format!("  {}", pr_title)
                },
                Style::default().fg(text_muted()),
            ),
        ]);
        Paragraph::new(pr_line).render(
            Rect {
                x: inner.x,
                y,
                width: w,
                height: 1,
            },
            buf,
        );
        y += 1;
    }

    // --- Spec line ---
    if let Some(spec) = &data.spec {
        let spec_color = if spec.is_complete() {
            accent_success()
        } else {
            accent_warning()
        };
        let source_label = match spec.source {
            conduit_git::ContextSource::Linked => "linked",
            conduit_git::ContextSource::Detected => "detected",
        };
        let spec_line = Line::from(vec![
            Span::styled("  spec: ", Style::default().fg(text_muted())),
            Span::styled(
                spec.change_id.clone(),
                Style::default().fg(text_secondary()),
            ),
            Span::styled(
                format!(" ({source_label})"),
                Style::default().fg(text_muted()),
            ),
            Span::styled(
                format!("  {} of {} tasks complete", spec.completed, spec.total),
                Style::default().fg(spec_color),
            ),
        ]);
        Paragraph::new(spec_line).render(
            Rect {
                x: inner.x,
                y,
                width: w,
                height: 1,
            },
            buf,
        );
        y += 1;
    }

    // --- Issue line ---
    if let Some(issue) = &data.issue {
        let issue_color = if issue.is_open {
            accent_warning()
        } else {
            accent_success()
        };
        let issue_state = if issue.is_open { "open" } else { "closed" };
        let issue_title = issue.title.as_deref().unwrap_or("");
        let issue_line = Line::from(vec![
            Span::styled("  issue #", Style::default().fg(text_muted())),
            Span::styled(issue.number.to_string(), Style::default().fg(issue_color)),
            Span::styled(
                format!(" ({issue_state})"),
                Style::default().fg(issue_color),
            ),
            Span::styled(
                if issue_title.is_empty() {
                    String::new()
                } else {
                    format!("  {}", issue_title)
                },
                Style::default().fg(text_muted()),
            ),
        ]);
        Paragraph::new(issue_line).render(
            Rect {
                x: inner.x,
                y,
                width: w,
                height: 1,
            },
            buf,
        );
        y += 1;
    }

    // --- Separator ---
    if y < inner.y + inner.height {
        let sep = "─".repeat(w as usize);
        Paragraph::new(Line::from(Span::styled(
            sep,
            Style::default().fg(text_muted()),
        )))
        .render(
            Rect {
                x: inner.x,
                y,
                width: w,
                height: 1,
            },
            buf,
        );
        y += 1;
    }

    // --- Action list ---
    for (i, action) in data.suggested_actions.iter().enumerate() {
        if y >= inner.y + inner.height {
            break;
        }
        let is_selected = i == selected;
        let row_bg = if is_selected {
            bg_highlight()
        } else {
            dialog_bg()
        };
        let row_style = Style::default().bg(row_bg);

        // Fill the row background
        let fill = " ".repeat(w as usize);
        buf.set_string(inner.x, y, &fill, row_style);

        let selector = if is_selected { "❯ " } else { "  " };
        let (action_label, action_desc) = action_label(*action);
        let label_color = if is_selected {
            ensure_contrast_fg(accent_primary(), row_bg, 4.5)
        } else {
            text_primary()
        };

        let line = Line::from(vec![
            Span::styled(
                selector,
                Style::default()
                    .fg(accent_primary())
                    .bg(row_bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(action_label, Style::default().fg(label_color).bg(row_bg)),
            Span::styled(
                format!("  {}", action_desc),
                Style::default().fg(text_muted()).bg(row_bg),
            ),
        ]);
        Paragraph::new(line).render(
            Rect {
                x: inner.x,
                y,
                width: w,
                height: 1,
            },
            buf,
        );
        y += 1;
    }
}

fn compute_review_height(data: &WorkCompleteData) -> u16 {
    let mut rows: u16 = 1; // scenario
    rows += 1; // branch
    if data.pr.is_some() {
        rows += 1;
    }
    if data.spec.is_some() {
        rows += 1;
    }
    if data.issue.is_some() {
        rows += 1;
    }
    rows += 1; // separator
    rows += data.suggested_actions.len() as u16;
    // borders(2) + top_padding(1) + instruction_bar(1)
    rows + 4
}

// ---------- Commit input ----------

fn render_commit_input(inner: Rect, buf: &mut Buffer, message: &str) {
    let label = Line::from(Span::styled(
        "Enter commit message:",
        Style::default().fg(text_secondary()),
    ));
    Paragraph::new(label).render(
        Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
        buf,
    );

    let input_y = inner.y + 2;
    if input_y < inner.y + inner.height {
        let cursor_line = format!("{}_", message);
        let input_line = Line::from(Span::styled(
            cursor_line,
            Style::default()
                .fg(text_primary())
                .add_modifier(Modifier::BOLD),
        ));
        Paragraph::new(input_line)
            .wrap(Wrap { trim: false })
            .render(
                Rect {
                    x: inner.x,
                    y: input_y,
                    width: inner.width,
                    height: inner.height.saturating_sub(3),
                },
                buf,
            );
    }
}

// ---------- Force confirm ----------

fn render_force_confirm(inner: Rect, buf: &mut Buffer, kind: ForceKind, pending: SuggestedAction) {
    let warning_msg = match kind {
        ForceKind::SpecIncomplete => {
            "The linked spec still has incomplete tasks.\nAre you sure you want to proceed?"
        }
        ForceKind::IssueOpen => {
            "The linked issue is still open.\nAre you sure you want to proceed?"
        }
    };

    let action_label = action_label(pending).0;
    let header = Line::from(vec![
        Span::styled("Action: ", Style::default().fg(text_muted())),
        Span::styled(
            action_label,
            Style::default()
                .fg(accent_warning())
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    Paragraph::new(header).render(
        Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
        buf,
    );

    let warn_y = inner.y + 2;
    if warn_y < inner.y + inner.height {
        Paragraph::new(warning_msg)
            .style(Style::default().fg(accent_error()))
            .wrap(Wrap { trim: false })
            .render(
                Rect {
                    x: inner.x,
                    y: warn_y,
                    width: inner.width,
                    height: inner.height.saturating_sub(3),
                },
                buf,
            );
    }
}

// ---------- Helpers ----------

fn scenario_label(scenario: Scenario) -> &'static str {
    match scenario {
        Scenario::CleanReady => "Clean — ready to archive",
        Scenario::EditsNoLink => "Uncommitted edits",
        Scenario::UnpushedCommits => "Unpushed commits",
        Scenario::SpecComplete => "Spec complete",
        Scenario::SpecIncomplete => "Spec in progress",
        Scenario::IssueOpen => "Issue open",
        Scenario::IssueClosed => "Issue closed",
    }
}

fn scenario_color(scenario: Scenario) -> ratatui::style::Color {
    match scenario {
        Scenario::CleanReady | Scenario::SpecComplete | Scenario::IssueClosed => accent_success(),
        Scenario::EditsNoLink
        | Scenario::UnpushedCommits
        | Scenario::SpecIncomplete
        | Scenario::IssueOpen => accent_warning(),
    }
}

fn action_label(action: SuggestedAction) -> (&'static str, &'static str) {
    match action {
        SuggestedAction::Commit => ("Commit", "stage and commit changes"),
        SuggestedAction::Push => ("Push", "push commits to remote"),
        SuggestedAction::OpenPr => ("Open PR", "create a pull request"),
        SuggestedAction::MergePr => ("Merge PR", "merge the open pull request"),
        SuggestedAction::CloseIssue => ("Close Issue", "close the linked GitHub issue"),
        SuggestedAction::ArchiveSpec => ("Archive Spec", "mark spec as done"),
        SuggestedAction::Archive => ("Archive Workspace", "close and archive this workspace"),
        SuggestedAction::ShowRemainingTasks => {
            ("Show Remaining Tasks", "ask agent to list incomplete tasks")
        }
    }
}
