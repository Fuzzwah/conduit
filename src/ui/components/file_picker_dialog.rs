//! Two-step dialog for copying a file from the local filesystem into the project repository.
//!
//! Step 1 (`SelectFile`): browse the local filesystem to choose a source file.
//! Step 2 (`SelectDirectory`): browse the repo's directory tree to choose a copy destination.

use std::path::PathBuf;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, Widget},
};
use uuid::Uuid;

use super::{dialog_bg, ensure_contrast_fg, text_muted, text_primary, DialogFrame};

const DIALOG_WIDTH: u16 = 65;
const DIALOG_HEIGHT: u16 = 22;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilePickerMode {
    /// Step 1: select any file from the local filesystem
    SelectFile,
    /// Step 2: select a destination directory within the repository
    SelectDirectory,
}

#[derive(Debug, Clone)]
pub enum FilePickerEntry {
    Dir(String),
    File(String, PathBuf),
}

impl FilePickerEntry {
    pub fn name(&self) -> &str {
        match self {
            FilePickerEntry::Dir(n) => n,
            FilePickerEntry::File(n, _) => n,
        }
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, FilePickerEntry::Dir(_))
    }
}

#[derive(Debug, Clone)]
pub struct FilePickerDialogState {
    pub visible: bool,
    pub mode: FilePickerMode,
    pub repo_id: Option<Uuid>,
    /// Root of the repository — used to clamp ascent in step 2
    pub repo_root: Option<PathBuf>,
    pub current_dir: PathBuf,
    pub entries: Vec<FilePickerEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    /// Populated after the user selects a file in step 1
    pub source_file: Option<PathBuf>,
}

impl Default for FilePickerDialogState {
    fn default() -> Self {
        Self {
            visible: false,
            mode: FilePickerMode::SelectFile,
            repo_id: None,
            repo_root: None,
            current_dir: PathBuf::new(),
            entries: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            source_file: None,
        }
    }
}

impl FilePickerDialogState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open step 1: browse local FS from `start_dir` to choose a source file.
    pub fn show_source_picker(
        &mut self,
        repo_id: Uuid,
        repo_root: Option<PathBuf>,
        start_dir: PathBuf,
    ) {
        self.visible = true;
        self.mode = FilePickerMode::SelectFile;
        self.repo_id = Some(repo_id);
        self.repo_root = repo_root;
        self.current_dir = start_dir;
        self.source_file = None;
        self.selected = 0;
        self.scroll_offset = 0;
        self.refresh_entries();
    }

    /// Transition to step 2: browse repo dirs to choose copy destination.
    pub fn show_dest_picker(&mut self) {
        let repo_root = match &self.repo_root {
            Some(p) => p.clone(),
            None => return,
        };
        self.mode = FilePickerMode::SelectDirectory;
        self.current_dir = repo_root;
        self.selected = 0;
        self.scroll_offset = 0;
        self.refresh_entries();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.source_file = None;
        self.entries.clear();
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Reload directory entries for `current_dir`, sorted dirs-first then files.
    pub fn refresh_entries(&mut self) {
        self.entries.clear();
        let Ok(read_dir) = std::fs::read_dir(&self.current_dir) else {
            return;
        };

        let mut dirs: Vec<String> = Vec::new();
        let mut files: Vec<(String, PathBuf)> = Vec::new();

        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = match entry.file_name().into_string() {
                Ok(n) => n,
                Err(_) => continue,
            };
            // Skip hidden entries
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                dirs.push(name);
            } else if self.mode == FilePickerMode::SelectFile {
                files.push((name, path));
            }
        }

        dirs.sort_unstable();
        files.sort_unstable_by(|a, b| a.0.cmp(&b.0));

        for d in dirs {
            self.entries.push(FilePickerEntry::Dir(d));
        }
        for (name, path) in files {
            self.entries.push(FilePickerEntry::File(name, path));
        }

        // Clamp selection
        if self.selected >= self.entries.len() {
            self.selected = self.entries.len().saturating_sub(1);
        }
    }

    /// Descend into the currently selected directory.
    pub fn descend(&mut self) {
        if let Some(FilePickerEntry::Dir(name)) = self.entries.get(self.selected) {
            let child = self.current_dir.join(name);
            if child.is_dir() {
                self.current_dir = child;
                self.selected = 0;
                self.scroll_offset = 0;
                self.refresh_entries();
            }
        }
    }

    /// Go up one directory level. In step 2 this is clamped at `repo_root`.
    pub fn ascend(&mut self) -> bool {
        if let Some(parent) = self.current_dir.parent() {
            // In step 2, don't go above the repo root
            if self.mode == FilePickerMode::SelectDirectory {
                if let Some(ref root) = self.repo_root.clone() {
                    if &self.current_dir == root {
                        return false; // already at repo root
                    }
                }
            }
            self.current_dir = parent.to_path_buf();
            self.selected = 0;
            self.scroll_offset = 0;
            self.refresh_entries();
            true
        } else {
            false
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.entries.is_empty() && self.selected < self.entries.len() - 1 {
            self.selected += 1;
        }
    }

    /// Returns the full path of the selected entry, if any.
    pub fn selected_path(&self) -> Option<PathBuf> {
        match self.entries.get(self.selected) {
            Some(FilePickerEntry::File(_, path)) => Some(path.clone()),
            Some(FilePickerEntry::Dir(name)) => Some(self.current_dir.join(name)),
            None => None,
        }
    }

    /// Returns the current directory as the chosen destination.
    pub fn dest_dir(&self) -> &PathBuf {
        &self.current_dir
    }
}

pub struct FilePickerDialog;

impl FilePickerDialog {
    pub fn new() -> Self {
        Self
    }

    pub fn dialog_area(area: Rect) -> Rect {
        let w = DIALOG_WIDTH.min(area.width.saturating_sub(4));
        let h = DIALOG_HEIGHT.min(area.height.saturating_sub(2));
        let x = (area.width.saturating_sub(w)) / 2;
        let y = (area.height.saturating_sub(h)) / 2;
        Rect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, state: &FilePickerDialogState) {
        let title = match state.mode {
            FilePickerMode::SelectFile => " Add File to Project — Step 1/2 ",
            FilePickerMode::SelectDirectory => " Add File to Project — Step 2/2 ",
        };
        let instructions = match state.mode {
            FilePickerMode::SelectFile => vec![
                ("Enter", "open/select"),
                ("\u{2190}", "up"),
                ("Esc", "cancel"),
            ],
            FilePickerMode::SelectDirectory => vec![
                ("c", "copy here"),
                ("Enter", "open dir"),
                ("\u{2190}", "up"),
                ("Esc", "cancel"),
            ],
        };

        let frame = DialogFrame::new(title, DIALOG_WIDTH, DIALOG_HEIGHT).instructions(instructions);
        let inner = frame.render(area, buf);

        // Layout: path header | optional source reminder | separator | file list
        let has_source_line =
            state.mode == FilePickerMode::SelectDirectory && state.source_file.is_some();

        let constraints = if has_source_line {
            vec![
                Constraint::Length(1), // current path
                Constraint::Length(1), // copying: <filename>
                Constraint::Length(1), // separator
                Constraint::Min(1),    // list
            ]
        } else {
            vec![
                Constraint::Length(1), // current path
                Constraint::Length(1), // separator
                Constraint::Min(1),    // list
            ]
        };

        let chunks = Layout::vertical(constraints).split(inner);

        // Current directory path (truncated from left)
        let path_str = state.current_dir.to_string_lossy();
        let max_path = chunks[0].width as usize;
        let display_path = if path_str.len() > max_path && max_path > 3 {
            format!("…{}", &path_str[path_str.len() - (max_path - 1)..])
        } else {
            path_str.to_string()
        };
        Paragraph::new(display_path)
            .style(Style::default().fg(text_muted()))
            .render(chunks[0], buf);

        let (sep_idx, list_idx) = if has_source_line {
            // Source file reminder
            let src_name = state
                .source_file
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            Paragraph::new(Line::from(vec![
                Span::styled("Copying: ", Style::default().fg(text_muted())),
                Span::styled(src_name, Style::default().fg(text_primary())),
            ]))
            .render(chunks[1], buf);
            (2, 3)
        } else {
            (1, 2)
        };

        Paragraph::new("─".repeat(chunks[sep_idx].width as usize))
            .style(Style::default().fg(text_muted()))
            .render(chunks[sep_idx], buf);

        // File list
        let list_area = chunks[list_idx];
        let visible_height = list_area.height as usize;

        // Compute scroll offset to keep selected visible
        let scroll = compute_scroll(state.selected, state.scroll_offset, visible_height);

        let fg_primary = ensure_contrast_fg(text_primary(), dialog_bg(), 4.5);
        let fg_muted = ensure_contrast_fg(text_muted(), dialog_bg(), 2.0);

        let items: Vec<ListItem> = state
            .entries
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible_height)
            .map(|(i, entry)| {
                let is_selected = i == state.selected;
                let (label, style) = match entry {
                    FilePickerEntry::Dir(name) => {
                        let label = format!("{}/", name);
                        let fg = if is_selected { fg_primary } else { fg_muted };
                        let mut s = Style::default().fg(fg);
                        if is_selected {
                            s = s.add_modifier(Modifier::REVERSED);
                        }
                        (label, s)
                    }
                    FilePickerEntry::File(name, _) => {
                        let fg = fg_primary;
                        let mut s = Style::default().fg(fg);
                        if is_selected {
                            s = s.add_modifier(Modifier::REVERSED);
                        }
                        (name.clone(), s)
                    }
                };
                ListItem::new(Line::from(Span::styled(label, style)))
            })
            .collect();

        if items.is_empty() {
            Paragraph::new(if state.entries.is_empty() {
                "(empty)"
            } else {
                ""
            })
            .style(Style::default().fg(text_muted()))
            .render(list_area, buf);
        } else {
            let mut list_state = ListState::default();
            // ListState offset is already handled by our skip(scroll)
            list_state.select(None);
            List::new(items).render(list_area, buf);
        }
    }
}

impl Default for FilePickerDialog {
    fn default() -> Self {
        Self::new()
    }
}

fn compute_scroll(selected: usize, prev_offset: usize, visible: usize) -> usize {
    if visible == 0 {
        return 0;
    }
    let mut offset = prev_offset;
    if selected < offset {
        offset = selected;
    } else if selected >= offset + visible {
        offset = selected + 1 - visible;
    }
    offset
}
