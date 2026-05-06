use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use unicode_width::UnicodeWidthStr;

use super::{
    accent_primary, bg_highlight, dialog_bg, ensure_contrast_bg, ensure_contrast_fg, text_primary,
    text_secondary, DialogFrame,
};

const DIALOG_WIDTH: u16 = 58;
const DIALOG_HEIGHT: u16 = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrchestrationOption {
    pub enabled: bool,
    pub label: &'static str,
    pub description: &'static str,
}

const OPTIONS: [OrchestrationOption; 2] = [
    OrchestrationOption {
        enabled: false,
        label: "Disabled",
        description: "Single model — no sub-agent delegation",
    },
    OrchestrationOption {
        enabled: true,
        label: "Enabled",
        description: "Delegate exploration/review to cheaper Haiku sub-agents",
    },
];

#[derive(Debug, Clone)]
pub struct OrchestrationSelectorState {
    pub visible: bool,
    pub selected: usize,
}

impl OrchestrationSelectorState {
    pub fn new() -> Self {
        Self {
            visible: false,
            selected: 0,
        }
    }

    pub fn show(&mut self, current: bool) {
        self.visible = true;
        self.selected = if current { 1 } else { 0 };
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn selected_option(&self) -> &OrchestrationOption {
        &OPTIONS[self.selected]
    }

    pub fn select_next(&mut self) {
        if self.selected + 1 < OPTIONS.len() {
            self.selected += 1;
        }
    }

    pub fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn select_at_row(&mut self, row: usize) -> bool {
        if row < OPTIONS.len() {
            self.selected = row;
            return true;
        }
        false
    }
}

impl Default for OrchestrationSelectorState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct OrchestrationSelector;

impl OrchestrationSelector {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, state: &OrchestrationSelectorState) {
        if !state.visible {
            return;
        }

        let frame = DialogFrame::new(" Orchestration Mode ", DIALOG_WIDTH, DIALOG_HEIGHT)
            .instructions(vec![("Enter", "apply"), ("Esc", "cancel")]);
        let inner = frame.render(area, buf);

        if inner.height < 3 || inner.width < 10 {
            return;
        }

        let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);

        self.render_list(chunks[0], buf, state);
        self.render_hint(chunks[1], buf);
    }

    fn render_list(&self, area: Rect, buf: &mut Buffer, state: &OrchestrationSelectorState) {
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf[(x, y)].set_bg(dialog_bg());
            }
        }

        let selected_bg = ensure_contrast_bg(bg_highlight(), dialog_bg(), 2.0);
        let selected_fg = ensure_contrast_fg(text_primary(), selected_bg, 4.5);
        let selected_muted = ensure_contrast_fg(text_secondary(), selected_bg, 3.0);

        for (idx, option) in OPTIONS.iter().enumerate() {
            if idx >= area.height as usize {
                break;
            }
            let selected = idx == state.selected;
            let y = area.y + idx as u16;
            let bg = if selected { selected_bg } else { dialog_bg() };
            let primary = if selected {
                selected_fg
            } else {
                text_primary()
            };
            let secondary = if selected {
                selected_muted
            } else {
                text_secondary()
            };

            let mut spans = vec![
                Span::styled(
                    format!("{:>2}. {}", idx + 1, option.label),
                    Style::default().fg(primary).bg(bg),
                ),
                Span::styled("  ", Style::default().bg(bg)),
                Span::styled(option.description, Style::default().fg(secondary).bg(bg)),
            ];

            let width_used: usize = spans
                .iter()
                .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
                .sum();
            if width_used < area.width as usize {
                spans.push(Span::styled(
                    " ".repeat(area.width as usize - width_used),
                    Style::default().bg(bg),
                ));
            }

            Paragraph::new(Line::from(spans)).render(
                Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: 1,
                },
                buf,
            );
        }
    }

    fn render_hint(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Claude only — delegates read/summarize/review tasks to Haiku")
            .style(Style::default().fg(accent_primary()))
            .render(area, buf);
    }
}

impl Default for OrchestrationSelector {
    fn default() -> Self {
        Self::new()
    }
}
