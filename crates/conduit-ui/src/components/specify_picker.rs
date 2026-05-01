//! Spec-kit (specify) spec picker dialog for selecting a spec when creating a new workspace.

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
    render_minimal_scrollbar, text_muted, text_primary, DialogFrame, SearchableListState,
};
use conduit_git::{RemoteIssue, SpecifySpec};

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

/// State for the spec-kit (specify) spec picker dialog.
#[derive(Debug, Clone)]
pub struct SpecifyPickerState {
    pub visible: bool,
    /// The repository for which a workspace is being created.
    pub repo_id: Uuid,
    /// The remote issue selected in the previous step (carried through).
    pub issue: Option<RemoteIssue>,
    pub specs: Vec<SpecifySpec>,
    pub list: SearchableListState,
    pub loading: bool,
    pub spinner_frame: usize,
    pub sort_order: SpecifySortOrder,
    /// Optional source-ref label (e.g. `origin/master`) shown in the footer.
    pub source_ref: Option<String>,
    /// Waiting for specs to load before deciding whether to show or skip.
    pub pending_show: bool,
}

impl Default for SpecifyPickerState {
    fn default() -> Self {
        Self {
            visible: false,
            repo_id: Uuid::nil(),
            issue: None,
            specs: Vec::new(),
            list: SearchableListState::new(MAX_VISIBLE),
            loading: false,
            spinner_frame: 0,
            sort_order: SpecifySortOrder::default(),
            source_ref: None,
            pending_show: false,
        }
    }
}

impl SpecifyPickerState {
    pub fn show_loading(repo_id: Uuid, issue: Option<RemoteIssue>) -> Self {
        Self {
            visible: false,
            repo_id,
            issue,
            loading: true,
            ..Self::default()
        }
    }

    pub fn load_specs(&mut self, specs: Vec<SpecifySpec>) {
        self.specs = specs;
        self.loading = false;
        self.list.reset();
        self.apply_sort();
        self.recompute_filter();
    }

    pub fn show(&mut self, issue: Option<RemoteIssue>) {
        self.issue = issue;
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.loading = false;
        self.pending_show = false;
        self.specs.clear();
        self.list.reset();
    }

    pub fn tick(&mut self) {
        if self.loading {
            self.spinner_frame = self.spinner_frame.wrapping_add(1);
        }
    }

    pub fn select_prev(&mut self) {
        self.list.select_prev();
    }

    pub fn select_next(&mut self) {
        self.list.select_next();
    }

    pub fn selected_spec(&self) -> Option<&SpecifySpec> {
        self.list
            .filtered
            .get(self.list.selected)
            .and_then(|&i| self.specs.get(i))
    }

    pub fn cycle_sort(&mut self) {
        self.sort_order = self.sort_order.cycle();
        self.apply_sort();
        self.recompute_filter();
    }

    pub fn recompute_filter(&mut self) {
        let needle = self.list.search.value().to_lowercase();
        let filtered: Vec<usize> = self
            .specs
            .iter()
            .enumerate()
            .filter(|(_, s)| needle.is_empty() || s.spec_id.to_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .collect();
        self.list.set_filtered(filtered);
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

/// Spec-kit (specify) spec picker dialog widget.
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
            MAX_VISIBLE.min(state.list.filtered.len()).max(1) as u16
        };
        let show_chrome = !state.loading;
        let search_height: u16 = if show_chrome { 1 } else { 0 };
        // border(2) + padding(1) + search + list + footer(1) + padding(1)
        let dialog_height = 5 + search_height + list_height;

        let frame = DialogFrame::new("Select Specify Spec", DIALOG_WIDTH, dialog_height)
            .instructions(if show_chrome {
                vec![
                    ("type", "Filter"),
                    ("s", "Sort"),
                    ("Enter", "Select"),
                    ("Esc", "Skip"),
                ]
            } else {
                vec![("Esc", "Cancel")]
            });
        let inner = frame.render(area, buf);

        let chunks = Layout::vertical([
            Constraint::Length(search_height),
            Constraint::Min(1),
            Constraint::Length(1), // footer
            Constraint::Length(1), // bottom padding
        ])
        .split(inner);

        let mut idx = 0;
        let search_area = chunks[idx];
        idx += 1;
        let list_area = chunks[idx];
        idx += 1;
        let footer_area = chunks[idx];

        if show_chrome {
            self.render_search(search_area, buf, state);
        }

        if state.loading {
            let spinner = SPINNER_FRAMES[state.spinner_frame % SPINNER_FRAMES.len()];
            Paragraph::new(format!("{} Fetching specs...", spinner))
                .style(Style::default().fg(accent_primary()))
                .alignment(Alignment::Center)
                .render(list_area, buf);
        } else if state.specs.is_empty() {
            Paragraph::new("No open specs found.")
                .style(Style::default().fg(text_muted()))
                .alignment(Alignment::Center)
                .render(list_area, buf);
        } else if state.list.filtered.is_empty() {
            Paragraph::new("No specs match the current filter.")
                .style(Style::default().fg(text_muted()))
                .alignment(Alignment::Center)
                .render(list_area, buf);
        } else {
            self.render_list(list_area, buf, state);
        }

        let mut footer = format!(
            "{}/{} specs · sorted by {}",
            state.list.filtered.len(),
            state.specs.len(),
            state.sort_order.label()
        );
        if let Some(src) = state.source_ref.as_deref() {
            footer.push_str(" · reading from ");
            footer.push_str(src);
        }
        Paragraph::new(footer)
            .style(Style::default().fg(text_muted()))
            .alignment(Alignment::Right)
            .render(footer_area, buf);
    }

    fn render_search(&self, area: Rect, buf: &mut Buffer, state: &SpecifyPickerState) {
        let prefix = "filter: ";
        Paragraph::new(Line::from(vec![Span::styled(
            prefix,
            Style::default().fg(text_muted()),
        )]))
        .render(area, buf);
        let prefix_w = prefix.len() as u16;
        if area.width > prefix_w {
            let input_area = Rect {
                x: area.x + prefix_w,
                y: area.y,
                width: area.width - prefix_w,
                height: 1,
            };
            state.list.search.render_with_placeholder(
                input_area,
                buf,
                Style::default().fg(text_primary()),
                "type to filter",
                Style::default().fg(text_muted()),
            );
        }
    }

    fn render_list(&self, list_area: Rect, buf: &mut Buffer, state: &SpecifyPickerState) {
        let selected_bg_color = ensure_contrast_bg(bg_highlight(), dialog_bg(), 2.0);
        let selected_fg_color = ensure_contrast_fg(text_primary(), selected_bg_color, 4.5);

        let visible_count = list_area.height as usize;
        let total = state.list.filtered.len();

        for row in 0..visible_count {
            let f_idx = state.list.scroll_offset + row;
            if f_idx >= total {
                break;
            }
            let spec_idx = state.list.filtered[f_idx];
            let spec = &state.specs[spec_idx];
            let y = list_area.y + row as u16;
            let is_selected = state.list.selected == f_idx;

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
                state.list.scroll_offset,
            );
        }
    }
}
