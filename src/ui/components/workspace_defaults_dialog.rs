//! Dialog for editing global workspace defaults.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use super::{
    bg_highlight, dialog_bg, ensure_contrast_bg, ensure_contrast_fg, text_muted, text_primary,
    DialogFrame,
};
use crate::git::WorkspaceMode;

const DIALOG_WIDTH: u16 = 62;
const DIALOG_HEIGHT: u16 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceDefaultsField {
    Mode,
    DeleteBranch,
    RemotePrompt,
    Save,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceDefaultsDraft {
    pub mode: WorkspaceMode,
    pub archive_delete_branch: bool,
    pub archive_remote_prompt: bool,
}

#[derive(Debug, Clone)]
pub struct WorkspaceDefaultsDialogState {
    pub visible: bool,
    pub selected: usize,
    pub draft: WorkspaceDefaultsDraft,
}

impl WorkspaceDefaultsDialogState {
    pub fn new() -> Self {
        Self {
            visible: false,
            selected: 0,
            draft: WorkspaceDefaultsDraft {
                mode: WorkspaceMode::Worktree,
                archive_delete_branch: true,
                archive_remote_prompt: true,
            },
        }
    }

    pub fn show(&mut self, draft: WorkspaceDefaultsDraft) {
        self.visible = true;
        self.selected = 0;
        self.draft = draft;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn select_next(&mut self) {
        self.selected = (self.selected + 1) % Self::fields().len();
    }

    pub fn select_prev(&mut self) {
        if self.selected == 0 {
            self.selected = Self::fields().len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    pub fn select_at_row(&mut self, row: usize) -> bool {
        if row < Self::fields().len() {
            self.selected = row;
            true
        } else {
            false
        }
    }

    pub fn activate_selected(&mut self) -> bool {
        match Self::fields()[self.selected] {
            WorkspaceDefaultsField::Mode => {
                self.draft.mode = match self.draft.mode {
                    WorkspaceMode::Worktree => WorkspaceMode::Checkout,
                    WorkspaceMode::Checkout => WorkspaceMode::Worktree,
                };
                false
            }
            WorkspaceDefaultsField::DeleteBranch => {
                self.draft.archive_delete_branch = !self.draft.archive_delete_branch;
                false
            }
            WorkspaceDefaultsField::RemotePrompt => {
                self.draft.archive_remote_prompt = !self.draft.archive_remote_prompt;
                false
            }
            WorkspaceDefaultsField::Save => true,
        }
    }

    pub fn fields() -> [WorkspaceDefaultsField; 4] {
        [
            WorkspaceDefaultsField::Mode,
            WorkspaceDefaultsField::DeleteBranch,
            WorkspaceDefaultsField::RemotePrompt,
            WorkspaceDefaultsField::Save,
        ]
    }
}

impl Default for WorkspaceDefaultsDialogState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct WorkspaceDefaultsDialog;

impl WorkspaceDefaultsDialog {
    pub fn new() -> Self {
        Self
    }

    pub fn dialog_area(area: Rect) -> Rect {
        let dialog_width = DIALOG_WIDTH.min(area.width.saturating_sub(4));
        let dialog_height = DIALOG_HEIGHT.min(area.height.saturating_sub(2));
        let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;
        Rect {
            x: dialog_x,
            y: dialog_y,
            width: dialog_width,
            height: dialog_height,
        }
    }

    pub fn list_area(area: Rect) -> Rect {
        let dialog = Self::dialog_area(area);
        let inner = super::dialog_content_area(dialog);
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(4),
        ])
        .split(inner);
        chunks[2]
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, state: &WorkspaceDefaultsDialogState) {
        let frame = DialogFrame::new(" Workspace Defaults ", DIALOG_WIDTH, DIALOG_HEIGHT)
            .instructions(vec![
                ("↑↓", "select"),
                ("Enter", "change/save"),
                ("Esc", "cancel"),
            ]);
        let inner = frame.render(area, buf);

        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(4),
        ])
        .split(inner);
        Paragraph::new("Set defaults used when a repository has no override.")
            .style(Style::default().fg(text_muted()))
            .render(chunks[0], buf);

        Paragraph::new("─".repeat(chunks[1].width as usize))
            .style(Style::default().fg(text_muted()))
            .render(chunks[1], buf);

        self.render_rows(chunks[2], buf, state);
    }

    fn render_rows(&self, area: Rect, buf: &mut Buffer, state: &WorkspaceDefaultsDialogState) {
        for (row, field) in WorkspaceDefaultsDialogState::fields().iter().enumerate() {
            if row as u16 >= area.height {
                break;
            }

            let line_area = Rect {
                x: area.x,
                y: area.y + row as u16,
                width: area.width,
                height: 1,
            };
            let is_selected = row == state.selected;
            let bg = if is_selected {
                ensure_contrast_bg(bg_highlight(), dialog_bg(), 2.0)
            } else {
                dialog_bg()
            };
            let fg = if is_selected {
                ensure_contrast_fg(text_primary(), bg, 4.5)
            } else {
                text_primary()
            };
            for x in line_area.x..line_area.x.saturating_add(line_area.width) {
                buf[(x, line_area.y)].set_bg(bg);
            }

            let content = match field {
                WorkspaceDefaultsField::Mode => format!(
                    "Workspace mode: {}",
                    match state.draft.mode {
                        WorkspaceMode::Worktree => "worktree",
                        WorkspaceMode::Checkout => "checkout",
                    }
                ),
                WorkspaceDefaultsField::DeleteBranch => format!(
                    "Delete local branch on archive: {}",
                    on_off(state.draft.archive_delete_branch)
                ),
                WorkspaceDefaultsField::RemotePrompt => format!(
                    "Prompt for remote branch deletion: {}",
                    on_off(state.draft.archive_remote_prompt)
                ),
                WorkspaceDefaultsField::Save => "Save changes".to_string(),
            };

            Paragraph::new(Line::from(vec![Span::styled(
                content,
                Style::default().fg(fg).bg(bg),
            )]))
            .render(line_area, buf);
        }
    }
}

impl Default for WorkspaceDefaultsDialog {
    fn default() -> Self {
        Self::new()
    }
}

fn on_off(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}
