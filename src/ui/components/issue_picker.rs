//! Remote issue picker dialog for linking a new workspace to an open issue.

use std::collections::BTreeSet;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use uuid::Uuid;

use super::{
    accent_primary, bg_highlight, dialog_bg, ensure_contrast_bg, ensure_contrast_fg,
    render_minimal_scrollbar, text_muted, text_primary, DialogFrame, SearchableListState,
};
use crate::git::RemoteIssue;

const MAX_VISIBLE: usize = 10;
const DIALOG_WIDTH: u16 = 72;
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// State for the remote issue picker dialog.
#[derive(Debug, Clone)]
pub struct IssuePickerState {
    pub visible: bool,
    /// The repository for which a workspace is being created.
    pub repo_id: Uuid,
    pub issues: Vec<RemoteIssue>,
    /// Search/filter input + selection/scroll state.
    pub list: SearchableListState,
    /// Labels the user has selected to filter on (AND across labels).
    pub selected_labels: BTreeSet<String>,
    /// True when "mine only" filter is active.
    pub mine_only: bool,
    /// Cached current-user lookup. Outer Option = "have we tried"; inner = the
    /// user login (or `None` if the provider couldn't resolve one).
    pub cached_current_user: Option<Option<String>>,
    /// True while the label-multiselect popover is open.
    pub label_popover_open: bool,
    /// State for the label popover list.
    pub label_popover: SearchableListState,
    /// Snapshot of unique labels from `issues` (sorted) used by the popover.
    pub all_labels: Vec<String>,
    pub loading: bool,
    pub spinner_frame: usize,
}

impl Default for IssuePickerState {
    fn default() -> Self {
        Self {
            visible: false,
            repo_id: Uuid::nil(),
            issues: Vec::new(),
            list: SearchableListState::new(MAX_VISIBLE),
            selected_labels: BTreeSet::new(),
            mine_only: false,
            cached_current_user: None,
            label_popover_open: false,
            label_popover: SearchableListState::new(MAX_VISIBLE),
            all_labels: Vec::new(),
            loading: false,
            spinner_frame: 0,
        }
    }
}

impl IssuePickerState {
    pub fn show_loading(repo_id: Uuid) -> Self {
        Self {
            visible: true,
            repo_id,
            loading: true,
            ..Self::default()
        }
    }

    pub fn load_issues(&mut self, issues: Vec<RemoteIssue>) {
        // Snapshot the union of labels for the popover.
        let mut labels: BTreeSet<String> = BTreeSet::new();
        for issue in &issues {
            for label in &issue.labels {
                labels.insert(label.clone());
            }
        }
        self.all_labels = labels.into_iter().collect();
        self.issues = issues;
        self.loading = false;
        self.list.reset();
        self.recompute_filter();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.loading = false;
        self.label_popover_open = false;
        self.issues.clear();
        self.list.reset();
        self.selected_labels.clear();
        self.mine_only = false;
        self.all_labels.clear();
    }

    pub fn tick(&mut self) {
        if self.loading {
            self.spinner_frame = self.spinner_frame.wrapping_add(1);
        }
    }

    /// Recompute the filtered indices from the current text/labels/mine filters.
    pub fn recompute_filter(&mut self) {
        let needle = self.list.search.value().to_lowercase();
        let current_user = self.current_user_login();
        let filter_mine = self.mine_only && current_user.is_some();
        let user = current_user.unwrap_or_default().to_string();
        let labels = &self.selected_labels;
        let filtered: Vec<usize> = self
            .issues
            .iter()
            .enumerate()
            .filter(|(_, issue)| {
                if !needle.is_empty() {
                    let number_str = format!("#{}", issue.number);
                    let title_lower = issue.title.to_lowercase();
                    if !number_str.contains(&needle) && !title_lower.contains(&needle) {
                        return false;
                    }
                }
                if !labels.is_empty()
                    && !labels.iter().all(|l| issue.labels.iter().any(|il| il == l))
                {
                    return false;
                }
                if filter_mine
                    && !issue
                        .assignee_logins
                        .iter()
                        .any(|a| a.as_str() == user.as_str())
                {
                    return false;
                }
                true
            })
            .map(|(i, _)| i)
            .collect();
        self.list.set_filtered(filtered);
    }

    /// Returns the currently-known user login if one has been cached.
    pub fn current_user_login(&self) -> Option<&str> {
        self.cached_current_user
            .as_ref()
            .and_then(|inner| inner.as_deref())
    }

    /// Returns true once a current-user lookup has been attempted.
    pub fn has_attempted_user_lookup(&self) -> bool {
        self.cached_current_user.is_some()
    }

    /// Apply a fetched current-user value (None = lookup failed/unavailable).
    pub fn set_current_user(&mut self, user: Option<String>) {
        self.cached_current_user = Some(user);
        self.recompute_filter();
    }

    pub fn select_prev(&mut self) {
        self.list.select_prev();
    }

    pub fn select_next(&mut self) {
        self.list.select_next();
    }

    pub fn selected_issue(&self) -> Option<&RemoteIssue> {
        self.list
            .filtered
            .get(self.list.selected)
            .and_then(|&idx| self.issues.get(idx))
    }

    pub fn toggle_mine(&mut self) {
        self.mine_only = !self.mine_only;
        self.recompute_filter();
    }

    pub fn toggle_label(&mut self, label: &str) {
        if self.selected_labels.contains(label) {
            self.selected_labels.remove(label);
        } else {
            self.selected_labels.insert(label.to_string());
        }
        self.recompute_filter();
    }

    pub fn open_label_popover(&mut self) {
        self.label_popover_open = true;
        self.label_popover.reset();
        let filtered = (0..self.all_labels.len()).collect();
        self.label_popover.set_filtered(filtered);
    }

    pub fn close_label_popover(&mut self) {
        self.label_popover_open = false;
    }

    pub fn selected_popover_label(&self) -> Option<&str> {
        self.label_popover
            .filtered
            .get(self.label_popover.selected)
            .and_then(|&i| self.all_labels.get(i).map(String::as_str))
    }
}

/// Remote issue picker dialog widget.
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

        let list_height = if state.loading {
            3u16
        } else {
            MAX_VISIBLE.min(state.list.filtered.len()).max(1) as u16
        };
        let show_chrome = !state.loading;
        let chips_height: u16 = if show_chrome && !state.selected_labels.is_empty() {
            1
        } else {
            0
        };
        let search_height: u16 = if show_chrome { 1 } else { 0 };
        let footer_height: u16 = if show_chrome { 1 } else { 0 };
        // border(2) + padding(1) + search + chips + list + footer + padding(1)
        let dialog_height = 4 + search_height + chips_height + list_height + footer_height;

        let frame = DialogFrame::new(
            "Link Workspace to Remote Issue",
            DIALOG_WIDTH,
            dialog_height,
        )
        .instructions(if show_chrome {
            vec![
                ("type", "Filter"),
                ("Tab", "Labels"),
                ("m", "Mine"),
                ("Enter", "Select"),
                ("Esc", "Skip"),
            ]
        } else {
            vec![("Esc", "Cancel")]
        });
        let inner = frame.render(area, buf);

        if state.loading {
            let spinner = SPINNER_FRAMES[state.spinner_frame % SPINNER_FRAMES.len()];
            let msg = format!("{} Fetching open issues...", spinner);
            let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
            Paragraph::new(msg)
                .style(Style::default().fg(accent_primary()))
                .alignment(Alignment::Center)
                .render(chunks[0], buf);
            return;
        }

        // Layout: search row, optional label chips row, list, footer, padding.
        let mut constraints: Vec<Constraint> = Vec::with_capacity(5);
        constraints.push(Constraint::Length(search_height));
        if chips_height > 0 {
            constraints.push(Constraint::Length(chips_height));
        }
        constraints.push(Constraint::Min(1));
        constraints.push(Constraint::Length(footer_height));
        constraints.push(Constraint::Length(1));
        let chunks = Layout::vertical(constraints).split(inner);

        let mut idx = 0;
        let search_area = chunks[idx];
        idx += 1;
        let chips_area = if chips_height > 0 {
            let a = chunks[idx];
            idx += 1;
            Some(a)
        } else {
            None
        };
        let list_area = chunks[idx];
        idx += 1;
        let footer_area = chunks[idx];

        // Search input row.
        self.render_search(search_area, buf, state);

        // Selected-label chips.
        if let Some(area) = chips_area {
            self.render_chips(area, buf, state);
        }

        // List or empty message.
        if state.issues.is_empty() {
            Paragraph::new("No open issues found.")
                .style(Style::default().fg(text_muted()))
                .alignment(Alignment::Center)
                .render(list_area, buf);
        } else if state.list.filtered.is_empty() {
            Paragraph::new("No issues match the current filters.")
                .style(Style::default().fg(text_muted()))
                .alignment(Alignment::Center)
                .render(list_area, buf);
        } else {
            self.render_list(list_area, buf, state);
        }

        // Footer.
        self.render_footer(footer_area, buf, state);

        // Label popover overlay (rendered last so it sits on top).
        if state.label_popover_open {
            self.render_label_popover(area, buf, state);
        }
    }

    fn render_search(&self, area: Rect, buf: &mut Buffer, state: &IssuePickerState) {
        let prefix = "filter: ";
        let prefix_span = Span::styled(prefix, Style::default().fg(text_muted()));
        Paragraph::new(Line::from(vec![prefix_span])).render(area, buf);
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

    fn render_chips(&self, area: Rect, buf: &mut Buffer, state: &IssuePickerState) {
        let chip_bg = ensure_contrast_bg(bg_highlight(), dialog_bg(), 2.0);
        let chip_fg = ensure_contrast_fg(text_primary(), chip_bg, 4.5);
        let mut spans: Vec<Span> = Vec::new();
        spans.push(Span::styled("labels: ", Style::default().fg(text_muted())));
        for (i, label) in state.selected_labels.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(
                format!(" {label} "),
                Style::default().fg(chip_fg).bg(chip_bg),
            ));
        }
        Paragraph::new(Line::from(spans)).render(area, buf);
    }

    fn render_list(&self, list_area: Rect, buf: &mut Buffer, state: &IssuePickerState) {
        let selected_bg_color = ensure_contrast_bg(bg_highlight(), dialog_bg(), 2.0);
        let selected_fg_color = ensure_contrast_fg(text_primary(), selected_bg_color, 4.5);

        let visible_count = list_area.height as usize;
        let total = state.list.filtered.len();

        for row in 0..visible_count {
            let f_idx = state.list.scroll_offset + row;
            if f_idx >= total {
                break;
            }
            let issue_idx = state.list.filtered[f_idx];
            let issue = &state.issues[issue_idx];
            let y = list_area.y + row as u16;
            let is_selected = state.list.selected == f_idx;

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
                state.list.scroll_offset,
            );
        }
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer, state: &IssuePickerState) {
        let mine_label = if state.mine_only {
            if state.current_user_login().is_some() {
                "on"
            } else {
                "unavailable"
            }
        } else {
            "off"
        };
        let labels_str = if state.selected_labels.is_empty() {
            "none".to_string()
        } else {
            state
                .selected_labels
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(",")
        };
        let footer = format!(
            "{}/{} issues · mine: {} · labels: {}",
            state.list.filtered.len(),
            state.issues.len(),
            mine_label,
            labels_str
        );
        Paragraph::new(footer)
            .style(Style::default().fg(text_muted()))
            .alignment(Alignment::Right)
            .render(area, buf);
    }

    fn render_label_popover(&self, area: Rect, buf: &mut Buffer, state: &IssuePickerState) {
        let total = state.label_popover.filtered.len();
        let list_height = MAX_VISIBLE.min(total).max(1) as u16;
        let popover_height = list_height + 4;
        let popover_width = 40u16;

        let frame =
            DialogFrame::new("Filter Labels", popover_width, popover_height).instructions(vec![
                ("Space", "Toggle"),
                ("Enter", "Done"),
                ("Esc", "Close"),
            ]);
        let inner = frame.render(area, buf);
        let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
        let list_area = chunks[0];

        if state.all_labels.is_empty() {
            Paragraph::new("No labels found.")
                .style(Style::default().fg(text_muted()))
                .alignment(Alignment::Center)
                .render(list_area, buf);
            return;
        }

        let selected_bg_color = ensure_contrast_bg(bg_highlight(), dialog_bg(), 2.0);
        let selected_fg_color = ensure_contrast_fg(text_primary(), selected_bg_color, 4.5);
        let visible = list_area.height as usize;
        for row in 0..visible {
            let f_idx = state.label_popover.scroll_offset + row;
            if f_idx >= total {
                break;
            }
            let label_idx = state.label_popover.filtered[f_idx];
            let label = &state.all_labels[label_idx];
            let is_selected = state.label_popover.selected == f_idx;
            let checked = state.selected_labels.contains(label);
            let prefix = if is_selected { "> " } else { "  " };
            let mark = if checked { "[x] " } else { "[ ] " };

            let fg = if is_selected {
                selected_fg_color
            } else {
                text_primary()
            };
            let mark_style = Style::default()
                .fg(if checked { accent_primary() } else { fg })
                .add_modifier(if checked {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                });
            let line = Line::from(vec![
                Span::raw(prefix),
                Span::styled(mark, mark_style),
                Span::styled(label.clone(), Style::default().fg(fg)),
            ]);
            let row_style = if is_selected {
                Style::default().bg(selected_bg_color)
            } else {
                Style::default()
            };
            Paragraph::new(line).style(row_style).render(
                Rect {
                    x: list_area.x,
                    y: list_area.y + row as u16,
                    width: list_area.width,
                    height: 1,
                },
                buf,
            );
        }

        if total > visible {
            render_minimal_scrollbar(
                Rect {
                    x: list_area.x + list_area.width - 1,
                    y: list_area.y,
                    width: 1,
                    height: list_area.height,
                },
                buf,
                total,
                visible,
                state.label_popover.scroll_offset,
            );
        }
    }
}
