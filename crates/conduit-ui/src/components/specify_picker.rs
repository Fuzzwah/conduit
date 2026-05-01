//! Spec-kit (specify) spec picker dialog for selecting a spec when creating a new workspace

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
use conduit_git::{GithubIssue, SpecifySpec};

const MAX_VISIBLE: usize = 10;
const DIALOG_WIDTH: u16 = 72;
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpecifySortOrder {
    #[default]
    ByRemainingDesc,
    ByRemainingAsc,
    ByNameAsc,
}

impl SpecifySortOrder {
    pub fn cycle(self) -> Self {
        match self {
            Self::ByRemainingDesc => Self::ByRemainingAsc,
            Self::ByRemainingAsc => Self::ByNameAsc,
            Self::ByNameAsc => Self::ByRemainingDesc,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::ByRemainingDesc => "remaining ↓",
            Self::ByRemainingAsc => "remaining ↑",
            Self::ByNameAsc => "a–z",
        }
    }
}

/// State for the spec-kit (specify) spec picker dialog
#[derive(Debug, Clone)]
pub struct SpecifyPickerState {
    pub visible: bool,
    /// The repository for which a workspace is being created
    pub repo_id: Uuid,
    /// The GitHub issue selected in the previous step (carried through)
    pub issue: Option<GithubIssue>,
    pub specs: Vec<SpecifySpec>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub loading: bool,
    pub spinner_frame: usize,
    pub sort_order: SpecifySortOrder,
    /// Waiting for specs to load before deciding whether to show or skip
    pub pending_show: bool,
}

impl Default for SpecifyPickerState {
    fn default() -> Self {
        Self {
            visible: false,
            repo_id: Uuid::nil(),
            issue: None,
            specs: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            loading: false,
            spinner_frame: 0,
            sort_order: SpecifySortOrder::default(),
            pending_show: false,
        }
    }
}

impl SpecifyPickerState {
    pub fn show_loading(repo_id: Uuid, issue: Option<GithubIssue>) -> Self {
        Self {
            visible: false,
            repo_id,
            issue,
            specs: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            loading: true,
            spinner_frame: 0,
            sort_order: SpecifySortOrder::default(),
            pending_show: false,
        }
    }

    pub fn load_specs(&mut self, specs: Vec<SpecifySpec>) {
        self.specs = specs;
        self.loading = false;
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn show(&mut self, issue: Option<GithubIssue>) {
        self.issue = issue;
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.loading = false;
        self.pending_show = false;
        self.specs.clear();
    }

    pub fn tick(&mut self) {
        if self.loading {
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
        let max = self.specs.len().saturating_sub(1);
        if self.selected < max {
            self.selected += 1;
            if self.selected >= self.scroll_offset + MAX_VISIBLE {
                self.scroll_offset = self.selected - MAX_VISIBLE + 1;
            }
        }
    }

    pub fn selected_spec(&self) -> Option<&SpecifySpec> {
        self.specs.get(self.selected)
    }

    pub fn cycle_sort(&mut self) {
        self.sort_order = self.sort_order.cycle();
        self.apply_sort();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    fn apply_sort(&mut self) {
        match self.sort_order {
            SpecifySortOrder::ByRemainingDesc => {
                self.specs
                    .sort_by_key(|s| std::cmp::Reverse(s.remaining_tasks));
            }
            SpecifySortOrder::ByRemainingAsc => {
                self.specs.sort_by_key(|s| s.remaining_tasks);
            }
            SpecifySortOrder::ByNameAsc => {
                self.specs.sort_by(|a, b| a.spec_id.cmp(&b.spec_id));
            }
        }
    }
}

/// Spec-kit (specify) spec picker dialog widget
#[derive(Default)]
pub struct SpecifyPicker;

impl SpecifyPicker {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, state: &SpecifyPickerState) {
        if !state.visible {
            return;
        }

        let list_height = if state.loading {
            3u16
        } else {
            MAX_VISIBLE.min(state.specs.len()).max(1) as u16
        };
        // border(2) + padding(1) + list + sort-footer(1) + padding(1)
        let dialog_height = 5 + list_height;

        let frame = DialogFrame::new("Select Specify Spec", DIALOG_WIDTH, dialog_height)
            .instructions(vec![
                ("↑↓", "Navigate"),
                ("s", "Sort"),
                ("Enter", "Select"),
                ("Esc", "Skip"),
            ]);
        let inner = frame.render(area, buf);

        let chunks = Layout::vertical([
            Constraint::Min(1),    // list area
            Constraint::Length(1), // sort indicator
            Constraint::Length(1), // bottom padding
        ])
        .split(inner);

        let list_area = chunks[0];
        let footer_area = chunks[1];

        if state.loading {
            let spinner = SPINNER_FRAMES[state.spinner_frame % SPINNER_FRAMES.len()];
            let loading = Paragraph::new(format!("{} Fetching specs...", spinner))
                .style(Style::default().fg(accent_primary()))
                .alignment(Alignment::Center);
            loading.render(list_area, buf);
        } else if state.specs.is_empty() {
            let msg = Paragraph::new("No open specs found.")
                .style(Style::default().fg(text_muted()))
                .alignment(Alignment::Center);
            msg.render(list_area, buf);
        } else {
            self.render_list(list_area, buf, state);
        }

        let sort_label = format!("sorted by: {}", state.sort_order.label());
        let footer = Paragraph::new(sort_label)
            .style(Style::default().fg(text_muted()))
            .alignment(Alignment::Right);
        footer.render(footer_area, buf);
    }

    fn render_list(&self, list_area: Rect, buf: &mut Buffer, state: &SpecifyPickerState) {
        let selected_bg_color = ensure_contrast_bg(bg_highlight(), dialog_bg(), 2.0);
        let selected_fg_color = ensure_contrast_fg(text_primary(), selected_bg_color, 4.5);

        let visible_count = list_area.height as usize;
        let total = state.specs.len();

        for row in 0..visible_count {
            let idx = state.scroll_offset + row;
            if idx >= total {
                break;
            }
            let spec = &state.specs[idx];
            let y = list_area.y + row as u16;
            let is_selected = state.selected == idx;

            let prefix = if is_selected { "> " } else { "  " };
            let count_str = format!("[{}/{}]", spec.remaining_tasks, spec.total_tasks);
            let max_id = list_area
                .width
                .saturating_sub(prefix.len() as u16 + count_str.len() as u16 + 2)
                as usize;
            let spec_id = if spec.spec_id.len() > max_id {
                format!("{}…", &spec.spec_id[..max_id.saturating_sub(1)])
            } else {
                spec.spec_id.clone()
            };

            let line = Line::from(vec![
                Span::raw(prefix),
                Span::styled(
                    count_str,
                    Style::default().fg(if is_selected {
                        selected_fg_color
                    } else {
                        accent_primary()
                    }),
                ),
                Span::raw(" "),
                Span::styled(
                    spec_id,
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
