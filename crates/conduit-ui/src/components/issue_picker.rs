//! GitHub issue picker dialog for linking a new workspace to an open issue

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use uuid::Uuid;

use super::{
    accent_primary, bg_highlight, dialog_bg, ensure_contrast_bg, ensure_contrast_fg,
    render_minimal_scrollbar, text_muted, text_primary, DialogFrame,
};
use conduit_git::GithubIssue;

const MAX_VISIBLE: usize = 10;
const DIALOG_WIDTH: u16 = 72;
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// State for the GitHub issue picker dialog
#[derive(Debug, Clone)]
pub struct IssuePickerState {
    pub visible: bool,
    /// The repository for which a workspace is being created
    pub repo_id: Uuid,
    pub issues: Vec<GithubIssue>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub loading: bool,
    /// True while the remote sync is in progress (before issue fetch begins)
    pub syncing: bool,
    pub spinner_frame: usize,
}

impl Default for IssuePickerState {
    fn default() -> Self {
        Self {
            visible: false,
            repo_id: Uuid::nil(),
            issues: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            loading: false,
            syncing: false,
            spinner_frame: 0,
        }
    }
}

impl IssuePickerState {
    /// Show the picker in "syncing remote" state before issue fetching begins.
    pub fn show_syncing(repo_id: Uuid) -> Self {
        Self {
            visible: true,
            repo_id,
            issues: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            loading: false,
            syncing: true,
            spinner_frame: 0,
        }
    }

    pub fn show_loading(repo_id: Uuid) -> Self {
        Self {
            visible: true,
            repo_id,
            issues: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            loading: true,
            syncing: false,
            spinner_frame: 0,
        }
    }

    /// Transition from syncing state to actively fetching issues.
    pub fn start_loading(&mut self) {
        self.syncing = false;
        self.loading = true;
    }

    pub fn load_issues(&mut self, issues: Vec<GithubIssue>) {
        self.issues = issues;
        self.loading = false;
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.loading = false;
        self.syncing = false;
        self.issues.clear();
    }

    pub fn tick(&mut self) {
        if self.loading || self.syncing {
            self.spinner_frame = self.spinner_frame.wrapping_add(1);
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
    }

    pub fn select_next(&mut self) {
        let max = self.issues.len().saturating_sub(1);
        if self.selected < max {
            self.selected += 1;
            if self.selected >= self.scroll_offset + MAX_VISIBLE {
                self.scroll_offset = self.selected - MAX_VISIBLE + 1;
            }
        }
    }

    pub fn selected_issue(&self) -> Option<&GithubIssue> {
        self.issues.get(self.selected)
    }
}

/// GitHub issue picker dialog widget
#[derive(Default)]
pub struct IssuePicker;

impl IssuePicker {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, state: &IssuePickerState) {
        if !state.visible {
            return;
        }

        let list_height = if state.syncing || state.loading {
            3u16
        } else {
            MAX_VISIBLE.min(state.issues.len()).max(1) as u16
        };
        let dialog_height = 4 + list_height; // border(2) + padding(1) + list + padding(1)

        let frame = DialogFrame::new(
            "Link Workspace to GitHub Issue",
            DIALOG_WIDTH,
            dialog_height,
        )
        .instructions(vec![
            ("↑↓", "Navigate"),
            ("Enter", "Select"),
            ("Esc", "Skip"),
        ]);
        let inner = frame.render(area, buf);

        let chunks = Layout::vertical([
            Constraint::Min(1),    // list area
            Constraint::Length(1), // bottom padding
        ])
        .split(inner);

        let list_area = chunks[0];

        if state.syncing || state.loading {
            let spinner = SPINNER_FRAMES[state.spinner_frame % SPINNER_FRAMES.len()];
            let msg = if state.syncing {
                format!("{} Syncing with remote...", spinner)
            } else {
                format!("{} Fetching open issues...", spinner)
            };
            let loading = Paragraph::new(msg)
                .style(Style::default().fg(accent_primary()))
                .alignment(Alignment::Center);
            loading.render(list_area, buf);
            return;
        }

        if state.issues.is_empty() {
            let msg = Paragraph::new("No open issues found.")
                .style(Style::default().fg(text_muted()))
                .alignment(Alignment::Center);
            msg.render(list_area, buf);
            return;
        }

        let selected_bg_color = ensure_contrast_bg(bg_highlight(), dialog_bg(), 2.0);
        let selected_fg_color = ensure_contrast_fg(text_primary(), selected_bg_color, 4.5);

        let visible_count = list_area.height as usize;
        let total = state.issues.len();

        for row in 0..visible_count {
            let idx = state.scroll_offset + row;
            if idx >= total {
                break;
            }
            let issue = &state.issues[idx];
            let y = list_area.y + row as u16;
            let is_selected = state.selected == idx;

            let prefix = if is_selected { "> " } else { "  " };
            let number_str = format!("#{:<5}", issue.number);
            let max_title = list_area
                .width
                .saturating_sub(prefix.len() as u16 + number_str.len() as u16 + 2)
                as usize;
            let title = if issue.title.len() > max_title {
                format!("{}…", &issue.title[..max_title.saturating_sub(1)])
            } else {
                issue.title.clone()
            };

            let line = Line::from(vec![
                Span::raw(prefix),
                Span::styled(
                    number_str,
                    Style::default().fg(if is_selected {
                        selected_fg_color
                    } else {
                        accent_primary()
                    }),
                ),
                Span::raw("  "),
                Span::styled(
                    title,
                    Style::default().fg(if is_selected {
                        selected_fg_color
                    } else {
                        text_primary()
                    }),
                ),
            ]);

            let row_style = if is_selected {
                Style::default().bg(selected_bg_color)
            } else {
                Style::default()
            };

            let row_area = Rect {
                x: list_area.x,
                y,
                width: list_area.width,
                height: 1,
            };
            Paragraph::new(line).style(row_style).render(row_area, buf);
        }

        // Scrollbar
        if total > visible_count {
            render_minimal_scrollbar(
                Rect {
                    x: list_area.x + list_area.width - 1,
                    y: list_area.y,
                    width: 1,
                    height: list_area.height,
                },
                buf,
                total,
                visible_count,
                state.scroll_offset,
            );
        }
    }
}
