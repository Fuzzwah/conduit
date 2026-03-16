//! Searchable settings hub dialog.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use super::{
    bg_highlight, dialog_bg, dialog_content_area, ensure_contrast_bg, ensure_contrast_fg,
    render_minimal_scrollbar, text_muted, text_primary, text_secondary, truncate_to_width,
    DialogFrame, SearchableListState,
};

const DIALOG_WIDTH: u16 = 72;
const DIALOG_HEIGHT: u16 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingsMenuEntryId {
    ProjectsDirectory,
    DefaultModel,
    EnabledProviders,
    Theme,
    WorkspaceDefaults,
}

#[derive(Debug, Clone)]
pub struct SettingsMenuEntry {
    pub id: SettingsMenuEntryId,
    pub title: String,
    pub description: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct SettingsMenuState {
    pub visible: bool,
    pub entries: Vec<SettingsMenuEntry>,
    pub list: SearchableListState,
}

impl SettingsMenuState {
    pub fn new() -> Self {
        Self {
            visible: false,
            entries: Vec::new(),
            list: SearchableListState::new(8),
        }
    }

    pub fn show(&mut self, entries: Vec<SettingsMenuEntry>) {
        self.visible = true;
        self.update_entries(entries);
    }

    pub fn update_entries(&mut self, entries: Vec<SettingsMenuEntry>) {
        let selected_id = self.selected_entry().map(|entry| entry.id);
        self.entries = entries;
        self.filter();

        if let Some(selected_id) = selected_id {
            if let Some(idx) = self.list.filtered.iter().position(|entry_idx| {
                self.entries
                    .get(*entry_idx)
                    .is_some_and(|entry| entry.id == selected_id)
            }) {
                self.list.selected = idx;
            }
        }
        self.list.clamp_selection();
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn filter(&mut self) {
        let query = self.list.search.value().trim().to_lowercase();
        let filtered = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                query.is_empty()
                    || entry.title.to_lowercase().contains(&query)
                    || entry.description.to_lowercase().contains(&query)
                    || entry.value.to_lowercase().contains(&query)
            })
            .map(|(idx, _)| idx)
            .collect();
        self.list.set_filtered(filtered);
    }

    pub fn insert_char(&mut self, c: char) {
        self.list.search.insert_char(c);
        self.filter();
    }

    pub fn delete_char(&mut self) {
        self.list.search.delete_char();
        self.filter();
    }

    pub fn select_next(&mut self) {
        self.list.select_next();
    }

    pub fn select_prev(&mut self) {
        self.list.select_prev();
    }

    pub fn selected_entry(&self) -> Option<&SettingsMenuEntry> {
        let idx = *self.list.filtered.get(self.list.selected)?;
        self.entries.get(idx)
    }

    pub fn select_at_row(&mut self, row: usize) -> bool {
        self.list.select_at_row(row)
    }
}

impl Default for SettingsMenuState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SettingsMenu;

impl SettingsMenu {
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
        let inner = dialog_content_area(dialog);
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(4),
        ])
        .split(inner);
        chunks[3]
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, state: &SettingsMenuState) {
        let frame = DialogFrame::new(" Settings ", DIALOG_WIDTH, DIALOG_HEIGHT).instructions(vec![
            ("↑↓", "select"),
            ("Enter", "open"),
            ("Esc", "close"),
        ]);
        let inner = frame.render(area, buf);

        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(4),
        ])
        .split(inner);

        Paragraph::new("Configure Conduit defaults and project discovery.")
            .style(Style::default().fg(text_muted()))
            .render(chunks[0], buf);

        self.render_search(chunks[1], buf, state);

        Paragraph::new("─".repeat(chunks[2].width as usize))
            .style(Style::default().fg(text_muted()))
            .render(chunks[2], buf);

        self.render_list(chunks[3], buf, state);
    }

    fn render_search(&self, area: Rect, buf: &mut Buffer, state: &SettingsMenuState) {
        let prefix = "Search: ";
        let text = state.list.search.value();
        let cursor = state.list.search.cursor;

        let mut spans = vec![Span::styled(prefix, Style::default().fg(text_muted()))];
        if text.is_empty() {
            spans.push(Span::styled(
                "type to filter settings",
                Style::default().fg(text_muted()),
            ));
        } else {
            spans.push(Span::styled(text, Style::default().fg(text_primary())));
        }
        Paragraph::new(Line::from(spans)).render(area, buf);

        if state.visible {
            let cursor_x = area
                .x
                .saturating_add(prefix.len() as u16)
                .saturating_add(cursor as u16);
            if cursor_x < area.x.saturating_add(area.width) {
                buf[(cursor_x, area.y)]
                    .set_fg(text_primary())
                    .set_bg(bg_highlight());
            }
        }
    }

    fn render_list(&self, area: Rect, buf: &mut Buffer, state: &SettingsMenuState) {
        if area.height == 0 {
            return;
        }

        for y in area.y..area.y.saturating_add(area.height) {
            for x in area.x..area.x.saturating_add(area.width) {
                buf[(x, y)].set_bg(dialog_bg());
            }
        }

        if state.list.filtered.is_empty() {
            Paragraph::new("No settings match your search.")
                .style(Style::default().fg(text_muted()))
                .render(area, buf);
            return;
        }

        let visible = area.height as usize;
        let has_scrollbar = state.list.filtered.len() > visible;
        let content_width = if has_scrollbar {
            area.width.saturating_sub(1)
        } else {
            area.width
        };
        let start = state.list.scroll_offset;
        let end = (start + visible).min(state.list.filtered.len());
        let selected_bg = ensure_contrast_bg(bg_highlight(), dialog_bg(), 2.0);
        let selected_fg = ensure_contrast_fg(text_primary(), selected_bg, 4.5);
        let selected_secondary = ensure_contrast_fg(text_secondary(), selected_bg, 3.0);

        for (row, filtered_idx) in state.list.filtered[start..end].iter().enumerate() {
            let Some(entry) = state.entries.get(*filtered_idx) else {
                continue;
            };

            let y = area.y + row as u16;
            let line_area = Rect {
                x: area.x,
                y,
                width: area.width,
                height: 1,
            };
            let is_selected = start + row == state.list.selected;

            let (fg, bg) = if is_selected {
                (selected_fg, selected_bg)
            } else {
                (text_primary(), dialog_bg())
            };

            for x in line_area.x..line_area.x.saturating_add(line_area.width) {
                buf[(x, line_area.y)].set_bg(bg);
            }

            let title = truncate_to_width(&entry.title, (content_width as usize / 3).max(18));
            let value = truncate_to_width(&entry.value, (content_width as usize / 3).max(18));
            let description = truncate_to_width(
                &format!("  {}", entry.description),
                (content_width as usize / 2).max(20),
            );

            Paragraph::new(Line::from(vec![
                Span::styled(format!("{title}  {value}"), Style::default().fg(fg).bg(bg)),
                Span::styled(
                    description,
                    Style::default()
                        .fg(if is_selected {
                            selected_secondary
                        } else {
                            text_secondary()
                        })
                        .bg(bg),
                ),
            ]))
            .render(line_area, buf);
        }

        if has_scrollbar {
            render_minimal_scrollbar(
                Rect {
                    x: area.x.saturating_add(area.width.saturating_sub(1)),
                    y: area.y,
                    width: 1,
                    height: area.height,
                },
                buf,
                state.list.filtered.len(),
                area.height as usize,
                state.list.scroll_offset,
            );
        }
    }
}

impl Default for SettingsMenu {
    fn default() -> Self {
        Self::new()
    }
}
