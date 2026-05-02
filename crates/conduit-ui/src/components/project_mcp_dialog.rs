//! Dialog for viewing and updating project MCP settings.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use uuid::Uuid;

use super::{
    bg_highlight, dialog_bg, ensure_contrast_bg, ensure_contrast_fg, text_muted, text_primary,
    DialogFrame,
};

const DIALOG_WIDTH: u16 = 64;
const DIALOG_HEIGHT: u16 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectMcpField {
    Enabled,
    Save,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectMcpDialogState {
    pub visible: bool,
    pub repo_id: Option<Uuid>,
    pub project_name: String,
    pub selected: usize,
    pub mcp_enabled: bool,
    pub detected_config_summary: String,
}

impl ProjectMcpDialogState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(
        &mut self,
        repo_id: Uuid,
        project_name: impl Into<String>,
        mcp_enabled: bool,
        detected_config_summary: impl Into<String>,
    ) {
        self.visible = true;
        self.repo_id = Some(repo_id);
        self.project_name = project_name.into();
        self.selected = 0;
        self.mcp_enabled = mcp_enabled;
        self.detected_config_summary = detected_config_summary.into();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.repo_id = None;
        self.project_name.clear();
        self.selected = 0;
        self.mcp_enabled = true;
        self.detected_config_summary.clear();
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

    pub fn activate_selected(&mut self) -> bool {
        match Self::fields()[self.selected] {
            ProjectMcpField::Enabled => {
                self.mcp_enabled = !self.mcp_enabled;
                false
            }
            ProjectMcpField::Save => true,
        }
    }

    fn fields() -> [ProjectMcpField; 2] {
        [ProjectMcpField::Enabled, ProjectMcpField::Save]
    }
}

pub struct ProjectMcpDialog;

impl ProjectMcpDialog {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, state: &ProjectMcpDialogState) {
        let frame = DialogFrame::new(" Project MCP ", DIALOG_WIDTH, DIALOG_HEIGHT)
            .instructions(vec![("↑↓", "select"), ("Enter", "change/save"), ("Esc", "cancel")]);
        let inner = frame.render(area, buf);

        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(2),
        ])
        .split(inner);

        Paragraph::new(format!("Project: {}", state.project_name))
            .style(Style::default().fg(text_muted()))
            .render(chunks[0], buf);

        Paragraph::new(state.detected_config_summary.as_str())
            .style(Style::default().fg(text_muted()))
            .render(chunks[1], buf);

        Paragraph::new("─".repeat(chunks[2].width as usize))
            .style(Style::default().fg(text_muted()))
            .render(chunks[2], buf);

        self.render_rows(chunks[3], buf, state);
    }

    fn render_rows(&self, area: Rect, buf: &mut Buffer, state: &ProjectMcpDialogState) {
        for (row, field) in ProjectMcpDialogState::fields().iter().enumerate() {
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
                ProjectMcpField::Enabled => {
                    format!("Project MCP: {}", on_off(state.mcp_enabled))
                }
                ProjectMcpField::Save => "Save changes".to_string(),
            };

            Paragraph::new(Line::from(vec![Span::styled(
                content,
                Style::default().fg(fg).bg(bg),
            )]))
            .render(line_area, buf);
        }
    }
}

impl Default for ProjectMcpDialog {
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
