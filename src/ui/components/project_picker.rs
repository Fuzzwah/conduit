//! Project picker dialog component with fuzzy search

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Paragraph, Widget},
};
use std::path::PathBuf;

use super::{
    dialog_bg, dialog_content_area, ensure_contrast_bg, ensure_contrast_fg,
    render_minimal_scrollbar, selected_bg, text_primary, DialogFrame, ScrollbarMetrics,
    SearchableListState,
};

/// A project entry (directory with .git)
#[derive(Debug, Clone)]
pub struct ProjectEntry {
    /// Display name (folder name)
    pub name: String,
    /// Full path to the project
    pub path: PathBuf,
}

/// State for the project picker dialog
#[derive(Debug, Clone)]
pub struct ProjectPickerState {
    /// Whether the dialog is visible
    pub visible: bool,
    /// All projects found in base directory
    pub projects: Vec<ProjectEntry>,
    /// Base directory being scanned
    pub base_dir: PathBuf,
    /// Searchable list state
    pub list: SearchableListState,
    /// Whether directory scanning is in progress
    pub loading: bool,
    /// Optional error from scanning
    pub error: Option<String>,
    /// Spinner frame for loading animation
    pub spinner_frame: usize,
}

impl Default for ProjectPickerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectPickerState {
    pub fn new() -> Self {
        Self {
            visible: false,
            projects: Vec::new(),
            base_dir: PathBuf::new(),
            list: SearchableListState::new(10),
            loading: false,
            error: None,
            spinner_frame: 0,
        }
    }

    /// Show the picker with projects from the given base directory
    pub fn show(&mut self, base_dir: PathBuf) {
        self.show_loading(base_dir.clone());
        match Self::scan_projects(base_dir) {
            Ok(projects) => self.load_projects(projects),
            Err(err) => self.set_error(err),
        }
    }

    /// Show the picker and enter loading state.
    pub fn show_loading(&mut self, base_dir: PathBuf) {
        self.visible = true;
        self.base_dir = base_dir;
        self.list.reset();
        self.projects.clear();
        self.loading = true;
        self.error = None;
        self.spinner_frame = 0;
    }

    /// Load discovered projects and leave loading state.
    pub fn load_projects(&mut self, projects: Vec<ProjectEntry>) {
        self.projects = projects;
        self.loading = false;
        self.error = None;
        self.filter();
    }

    /// Hide the dialog
    pub fn hide(&mut self) {
        self.visible = false;
        self.loading = false;
    }

    /// Set error state for picker discovery.
    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
        self.loading = false;
        self.projects.clear();
        self.filter();
    }

    /// Advance loading spinner.
    pub fn tick(&mut self) {
        if self.loading {
            self.spinner_frame = self.spinner_frame.wrapping_add(1);
        }
    }

    pub fn scrollbar_metrics(&self, area: Rect) -> Option<ScrollbarMetrics> {
        if !self.visible {
            return None;
        }

        let list_height = self.effective_visible_len() as u16;
        let dialog_height = 6 + list_height;
        let dialog_width: u16 = 60;

        let dialog_width = dialog_width.min(area.width.saturating_sub(4));
        let dialog_height = dialog_height.min(area.height.saturating_sub(2));

        let dialog_x = area.width.saturating_sub(dialog_width) / 2;
        let dialog_y = area.height.saturating_sub(dialog_height) / 2;

        let dialog_area = Rect {
            x: dialog_x,
            y: dialog_y,
            width: dialog_width,
            height: dialog_height,
        };

        let inner = dialog_content_area(dialog_area);

        // List starts after: search_label(1) + separator(1) = 2 rows
        let list_y = inner.y + 2;
        // List takes remaining space minus spacing(1) at bottom
        let list_height_actual = inner.height.saturating_sub(3);
        if list_height_actual == 0 {
            return None;
        }

        let list_area = Rect {
            x: inner.x,
            y: list_y,
            width: inner.width,
            height: list_height_actual,
        };

        let total = self.list.filtered.len() + 1; // +1 for "From git URL..." at top
        let visible = list_area.height as usize;
        if total <= visible {
            return None;
        }

        Some(ScrollbarMetrics {
            area: Rect {
                x: list_area.x + list_area.width - 1,
                y: list_area.y,
                width: 1,
                height: list_area.height,
            },
            total,
            visible,
        })
    }

    /// Scan a base directory for git projects.
    pub fn scan_projects(base_dir: PathBuf) -> Result<Vec<ProjectEntry>, String> {
        let entries = std::fs::read_dir(&base_dir)
            .map_err(|e| format!("Failed to read {}: {}", base_dir.display(), e))?;

        let mut projects: Vec<ProjectEntry> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
            .filter(|e| e.path().join(".git").exists())
            .map(|e| ProjectEntry {
                name: e.file_name().to_string_lossy().into_owned(),
                path: e.path(),
            })
            .collect();

        // Sort by last modified time (most recent first)
        projects.sort_by(|a, b| {
            let time_a = std::fs::metadata(&a.path)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let time_b = std::fs::metadata(&b.path)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            time_b.cmp(&time_a) // Descending order (most recent first)
        });
        Ok(projects)
    }

    /// Filter projects based on search string
    pub fn filter(&mut self) {
        let query = self.list.search.value().to_lowercase();
        let filtered = self
            .projects
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                if query.is_empty() {
                    true
                } else {
                    p.name.to_lowercase().contains(&query)
                }
            })
            .map(|(i, _)| i)
            .collect();
        self.list.set_filtered(filtered);
        // Index 0 is "From git URL..."; project indices are 1..=filtered.len().
        // Clamp to valid range and default to the first project when available.
        let max_idx = self.list.filtered.len();
        if self.list.selected > max_idx {
            self.list.selected = max_idx;
        } else if !self.list.filtered.is_empty() && self.list.selected == 0 {
            self.list.selected = 1;
        }
    }

    // Delegate search input methods
    pub fn insert_char(&mut self, c: char) {
        self.list.search.insert_char(c);
        self.filter();
    }

    pub fn delete_char(&mut self) {
        self.list.search.delete_char();
        self.filter();
    }

    pub fn delete_forward(&mut self) {
        self.list.search.delete_forward();
        self.filter();
    }

    pub fn move_cursor_left(&mut self) {
        self.list.search.move_left();
    }

    pub fn move_cursor_right(&mut self) {
        self.list.search.move_right();
    }

    pub fn move_cursor_start(&mut self) {
        self.list.search.move_start();
    }

    pub fn move_cursor_end(&mut self) {
        self.list.search.move_end();
    }

    pub fn clear_search(&mut self) {
        self.list.search.clear();
        self.filter();
    }

    /// Select previous item
    pub fn select_prev(&mut self) {
        if self.list.selected > 0 {
            self.list.selected -= 1;
            if self.list.selected < self.list.scroll_offset {
                self.list.scroll_offset = self.list.selected;
            }
        }
    }

    /// Select next item (git URL is at index 0, projects at 1..=filtered.len())
    pub fn select_next(&mut self) {
        let max_idx = self.list.filtered.len();
        if self.list.selected < max_idx {
            self.list.selected += 1;
            if self.list.selected >= self.list.scroll_offset + self.list.max_visible {
                self.list.scroll_offset = self.list.selected - self.list.max_visible + 1;
            }
        }
    }

    /// Page up (move up by visible count)
    pub fn page_up(&mut self) {
        self.list.page_up();
    }

    /// Page down (move down by visible count)
    pub fn page_down(&mut self) {
        let page_size = self.list.max_visible;
        let max_idx = self.list.filtered.len();
        if self.list.selected + page_size <= max_idx {
            self.list.selected += page_size;
        } else {
            self.list.selected = max_idx;
        }
        if self.list.selected >= self.list.scroll_offset + self.list.max_visible {
            self.list.scroll_offset = self.list.selected.saturating_sub(self.list.max_visible - 1);
        }
    }

    /// Select item at a given visual row (for mouse clicks)
    /// Returns true if an item was selected
    pub fn select_at_row(&mut self, row: usize) -> bool {
        let target_idx = self.list.scroll_offset + row;
        if target_idx <= self.list.filtered.len() {
            self.list.selected = target_idx;
            true
        } else {
            false
        }
    }

    /// Get the currently selected project, or None if "From git URL" is selected
    pub fn selected_project(&self) -> Option<&ProjectEntry> {
        if self.is_git_url_option_selected() {
            return None;
        }
        self.list
            .filtered
            .get(self.list.selected - 1)
            .and_then(|&idx| self.projects.get(idx))
    }

    /// Whether the "From git URL..." option at the top of the list is selected
    pub fn is_git_url_option_selected(&self) -> bool {
        self.list.selected == 0
    }

    /// Visible row count including the always-present "From git URL..." entry
    pub fn effective_visible_len(&self) -> usize {
        self.list.max_visible.min(self.list.filtered.len() + 1)
    }

    /// Check if dialog is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Check if there are no projects found
    pub fn is_empty(&self) -> bool {
        self.projects.is_empty()
    }
}

/// Project picker dialog widget
pub struct ProjectPicker;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

impl ProjectPicker {
    pub fn new() -> Self {
        Self
    }

    /// Render the dialog
    pub fn render(&self, area: Rect, buf: &mut Buffer, state: &ProjectPickerState) {
        if !state.visible {
            return;
        }

        // Calculate dialog size (+1 for always-present "From git URL..." entry)
        let list_height = state.effective_visible_len() as u16;
        let dialog_height = 6 + list_height; // border(2) + top_padding(1) + search_label(1) + separator(1) + spacing(1) + list

        // Render dialog frame (instructions render on bottom border)
        let frame = DialogFrame::new("Select Project", 60, dialog_height).instructions(vec![
            ("↑↓/^J^K", "Navigate"),
            ("^F/^B", "Page"),
            ("Enter", "Select"),
            ("Esc", "Cancel"),
        ]);
        let inner = frame.render(area, buf);

        // Layout inside dialog
        let chunks = Layout::vertical([
            Constraint::Length(1), // Search label
            Constraint::Length(1), // Separator
            Constraint::Min(1),    // Project list
            Constraint::Length(1), // Spacing
        ])
        .split(inner);

        // Render search with placeholder
        let search_display = if state.list.search.is_empty() {
            "Search: (type to filter)".to_string()
        } else {
            format!("Search: {}", state.list.search.value())
        };
        let search_style = if state.list.search.is_empty() {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };
        let search_label = Paragraph::new(search_display).style(search_style);
        search_label.render(chunks[0], buf);

        // Render cursor in search field
        if !state.list.search.is_empty() || state.list.search.cursor > 0 {
            let cursor_x = chunks[0].x + 8 + state.list.search.cursor as u16; // "Search: " is 8 chars
            if cursor_x < chunks[0].x + chunks[0].width {
                use ratatui::style::Modifier;
                buf[(cursor_x, chunks[0].y)]
                    .set_style(Style::default().add_modifier(Modifier::REVERSED));
            }
        }

        // Render separator
        let separator = "─".repeat(inner.width as usize);
        let sep_paragraph = Paragraph::new(separator).style(Style::default().fg(Color::DarkGray));
        sep_paragraph.render(chunks[1], buf);

        // Render project list
        let list_area = chunks[2];
        if state.loading {
            let spinner = SPINNER_FRAMES[state.spinner_frame % SPINNER_FRAMES.len()];
            let loading = Paragraph::new(format!("{} Scanning projects...", spinner))
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center);
            loading.render(list_area, buf);
        } else if let Some(error) = &state.error {
            let msg = format!("Failed to scan projects: {}", error);
            let error_p = Paragraph::new(msg)
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center);
            error_p.render(list_area, buf);
        } else {
            let selected_bg_color = ensure_contrast_bg(selected_bg(), dialog_bg(), 2.0);
            let selected_fg_color = ensure_contrast_fg(text_primary(), selected_bg_color, 4.5);

            let visible_count = list_area.height as usize;
            let home = dirs::home_dir().unwrap_or_default();
            let home_str = home.to_string_lossy();
            let total = state.list.filtered.len() + 1; // index 0 = git URL, 1..=N = projects

            for row in 0..visible_count {
                let logical_idx = state.list.scroll_offset + row;
                if logical_idx >= total {
                    break;
                }

                let y = list_area.y + row as u16;
                if y >= list_area.y + list_area.height {
                    break;
                }

                let is_selected = state.list.selected == logical_idx;
                let prefix = if is_selected { "> " } else { "  " };

                let (line_text, style) = if logical_idx == 0 {
                    let text = format!("{}+ From git URL...", prefix);
                    let s = if is_selected {
                        Style::default().fg(selected_fg_color).bg(selected_bg_color)
                    } else {
                        Style::default().fg(Color::Cyan)
                    };
                    (text, s)
                } else {
                    let project = &state.projects[state.list.filtered[logical_idx - 1]];
                    let name = &project.name;
                    let path_str = project
                        .path
                        .to_string_lossy()
                        .replace(home_str.as_ref(), "~");

                    let available_width = list_area.width as usize;
                    let name_width = 20.min(available_width / 2);
                    let path_width = available_width.saturating_sub(name_width + 4);

                    let name_display = if name.len() > name_width {
                        format!("{}...", &name[..name_width - 3])
                    } else {
                        format!("{:width$}", name, width = name_width)
                    };

                    let path_display = if path_str.len() > path_width {
                        format!("...{}", &path_str[path_str.len() - path_width + 3..])
                    } else {
                        path_str.to_string()
                    };

                    let text = format!("{}{} {}", prefix, name_display, path_display);
                    let s = if is_selected {
                        Style::default().fg(selected_fg_color).bg(selected_bg_color)
                    } else {
                        Style::default().fg(text_primary())
                    };
                    (text, s)
                };

                for (j, c) in line_text.chars().enumerate() {
                    if j < list_area.width as usize {
                        buf[(list_area.x + j as u16, y)]
                            .set_char(c)
                            .set_style(style);
                    }
                }
                if is_selected {
                    for j in line_text.len()..list_area.width as usize {
                        buf[(list_area.x + j as u16, y)].set_style(style);
                    }
                }
            }

            // Render scrollbar
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

impl Default for ProjectPicker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_picker_delete_char() {
        let mut state = ProjectPickerState::new();

        // Type "abc"
        state.insert_char('a');
        state.insert_char('b');
        state.insert_char('c');
        assert_eq!(state.list.search.input, "abc");

        // Backspace should delete 'c'
        state.delete_char();
        assert_eq!(state.list.search.input, "ab");

        // Backspace should delete 'b'
        state.delete_char();
        assert_eq!(state.list.search.input, "a");

        // Backspace should delete 'a'
        state.delete_char();
        assert_eq!(state.list.search.input, "");

        // Backspace on empty should do nothing
        state.delete_char();
        assert_eq!(state.list.search.input, "");
    }

    #[test]
    fn test_project_picker_cursor_position_after_delete() {
        let mut state = ProjectPickerState::new();

        // Type "abc"
        state.insert_char('a');
        state.insert_char('b');
        state.insert_char('c');
        assert_eq!(state.list.search.cursor, 3);

        // Delete should move cursor back
        state.delete_char();
        assert_eq!(state.list.search.input, "ab");
        assert_eq!(state.list.search.cursor, 2);

        state.delete_char();
        assert_eq!(state.list.search.input, "a");
        assert_eq!(state.list.search.cursor, 1);

        state.delete_char();
        assert_eq!(state.list.search.input, "");
        assert_eq!(state.list.search.cursor, 0);

        // Delete on empty should keep cursor at 0
        state.delete_char();
        assert_eq!(state.list.search.cursor, 0);
    }

    #[test]
    fn test_project_picker_mid_string_deletion() {
        let mut state = ProjectPickerState::new();

        // Type "abc"
        state.insert_char('a');
        state.insert_char('b');
        state.insert_char('c');
        assert_eq!(state.list.search.input, "abc");
        assert_eq!(state.list.search.cursor, 3);

        // Move cursor to middle (after 'a')
        state.move_cursor_start();
        assert_eq!(state.list.search.cursor, 0);

        state.move_cursor_right();
        assert_eq!(state.list.search.cursor, 1);

        // Delete should remove 'a' (char before cursor), leaving "bc"
        state.delete_char();
        assert_eq!(state.list.search.input, "bc");
        assert_eq!(state.list.search.cursor, 0);
    }

    #[test]
    fn test_project_picker_ascii_only() {
        // Note: The TextInputState uses byte-based cursor positioning, which means
        // multi-byte UTF-8 characters (like 'é' or emoji) are not fully supported.
        // This test verifies ASCII characters work correctly.
        let mut state = ProjectPickerState::new();

        // Type ASCII characters
        state.insert_char('h');
        state.insert_char('e');
        state.insert_char('l');
        state.insert_char('l');
        state.insert_char('o');
        assert_eq!(state.list.search.input, "hello");
        assert_eq!(state.list.search.cursor, 5);

        // Delete 'o'
        state.delete_char();
        assert_eq!(state.list.search.input, "hell");
        assert_eq!(state.list.search.cursor, 4);

        // Delete back to 'e'
        state.delete_char();
        state.delete_char();
        assert_eq!(state.list.search.input, "he");
        assert_eq!(state.list.search.cursor, 2);

        state.delete_char();
        assert_eq!(state.list.search.input, "h");
        assert_eq!(state.list.search.cursor, 1);
    }
}
