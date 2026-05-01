//! Dialog for renaming a project.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use uuid::Uuid;

use super::{dialog_bg, ensure_contrast_fg, text_muted, text_primary, DialogFrame, TextInputState};

const DIALOG_WIDTH: u16 = 50;
const DIALOG_HEIGHT: u16 = 7;

#[derive(Debug, Clone, Default)]
pub struct RenameProjectDialogState {
    pub visible: bool,
    pub repo_id: Option<Uuid>,
    pub input: TextInputState,
}

impl RenameProjectDialogState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, repo_id: Uuid, current_name: &str) {
        self.visible = true;
        self.repo_id = Some(repo_id);
        self.input = TextInputState::with_value(current_name);
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.repo_id = None;
        self.input.clear();
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn value(&self) -> &str {
        self.input.value()
    }

    pub fn insert_char(&mut self, c: char) {
        self.input.insert_char(c);
    }

    pub fn delete_char(&mut self) {
        self.input.delete_char();
    }

    pub fn delete_forward(&mut self) {
        self.input.delete_forward();
    }

    pub fn move_left(&mut self) {
        self.input.move_left();
    }

    pub fn move_right(&mut self) {
        self.input.move_right();
    }

    pub fn move_start(&mut self) {
        self.input.move_start();
    }

    pub fn move_end(&mut self) {
        self.input.move_end();
    }
}

pub struct RenameProjectDialog;

impl RenameProjectDialog {
    pub fn new() -> Self {
        Self
    }

    pub fn dialog_area(area: Rect) -> Rect {
        let dialog_width = DIALOG_WIDTH.min(area.width.saturating_sub(4));
        let dialog_height = DIALOG_HEIGHT.min(area.height.saturating_sub(2));
        let dialog_x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
        Rect {
            x: dialog_x,
            y: dialog_y,
            width: dialog_width,
            height: dialog_height,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, state: &RenameProjectDialogState) {
        let frame = DialogFrame::new(" Rename Project ", DIALOG_WIDTH, DIALOG_HEIGHT)
            .instructions(vec![("Enter", "save"), ("Esc", "cancel")]);
        let inner = frame.render(area, buf);

        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

        Paragraph::new("New project name:")
            .style(Style::default().fg(text_muted()))
            .render(chunks[0], buf);

        Paragraph::new("─".repeat(chunks[1].width as usize))
            .style(Style::default().fg(text_muted()))
            .render(chunks[1], buf);

        let input_area = chunks[2];
        let fg = ensure_contrast_fg(text_primary(), dialog_bg(), 4.5);
        let style = Style::default().fg(fg).bg(dialog_bg());

        if state.input.is_empty() {
            Paragraph::new(Line::from(vec![Span::styled(
                "enter name...",
                Style::default().fg(text_muted()),
            )]))
            .render(input_area, buf);
        } else {
            Paragraph::new(Line::from(vec![Span::styled(state.input.value(), style)]))
                .render(input_area, buf);
        }

        // Render cursor
        if input_area.width > 0 {
            let cursor_x =
                input_area.x + (state.input.cursor as u16).min(input_area.width.saturating_sub(1));
            if cursor_x < input_area.x + input_area.width {
                buf[(cursor_x, input_area.y)]
                    .set_style(Style::default().add_modifier(ratatui::style::Modifier::REVERSED));
            }
        }
    }
}

impl Default for RenameProjectDialog {
    fn default() -> Self {
        Self::new()
    }
}
