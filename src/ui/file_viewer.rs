//! File viewer session for displaying local files in tabs
//!
//! This module provides the FileViewerSession struct which holds the state
//! for viewing a local file in a tab.

use std::path::{Path, PathBuf};

use ratatui::text::Line;
use uuid::Uuid;

use crate::ui::components::source_highlighter::highlight_source_lines;
use crate::ui::components::MarkdownRenderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Markdown,
    Json,
    Rust,
    Toml,
    Yaml,
    PlainText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileViewMode {
    Raw,
    Rendered,
}

/// State for a file viewer tab
#[derive(Debug)]
pub struct FileViewerSession {
    /// Unique identifier for this session
    pub id: Uuid,
    /// Path to the file being viewed
    pub file_path: PathBuf,
    /// Raw file content
    content: String,
    /// Raw lines of the file
    lines: Vec<String>,
    /// Total number of raw lines
    pub total_lines: usize,
    /// Cached highlighted lines for raw mode
    raw_highlighted_lines: Vec<Line<'static>>,
    /// Cached formatted lines for markdown rendering
    rendered_lines: Vec<Line<'static>>,
    /// Total number of rendered lines in formatted mode
    rendered_total_lines: usize,
    /// Width used to produce the current markdown render cache
    last_render_width: Option<usize>,
    /// Current scroll offset (in lines)
    pub scroll_offset: usize,
    /// Whether to show line numbers
    pub show_line_numbers: bool,
    /// Whether this is a markdown file
    pub is_markdown: bool,
    /// Detected file kind
    file_kind: FileKind,
    /// Current display mode
    view_mode: FileViewMode,
}

impl FileViewerSession {
    /// Create a new file viewer session by reading a file
    pub fn new(file_path: PathBuf) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&file_path)?;
        let file_kind = Self::detect_file_kind(&file_path);
        let is_markdown = matches!(file_kind, FileKind::Markdown);

        let lines: Vec<String> = content.lines().map(String::from).collect();
        let total_lines = lines.len();

        let mut session = Self {
            id: Uuid::new_v4(),
            file_path,
            content,
            lines,
            total_lines,
            raw_highlighted_lines: Vec::new(),
            rendered_lines: Vec::new(),
            rendered_total_lines: 0,
            last_render_width: None,
            scroll_offset: 0,
            show_line_numbers: true,
            is_markdown,
            file_kind,
            view_mode: if is_markdown {
                FileViewMode::Rendered
            } else {
                FileViewMode::Raw
            },
        };
        session.rebuild_raw_highlight_cache();
        Ok(session)
    }

    fn detect_file_kind(file_path: &Path) -> FileKind {
        let ext = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase());

        match ext.as_deref() {
            Some("md" | "markdown") => FileKind::Markdown,
            Some("json" | "jsonl") => FileKind::Json,
            Some("rs") => FileKind::Rust,
            Some("toml") => FileKind::Toml,
            Some("yaml" | "yml") => FileKind::Yaml,
            _ => FileKind::PlainText,
        }
    }

    fn active_total_lines(&self) -> usize {
        match self.view_mode {
            FileViewMode::Raw => self.raw_highlighted_lines.len(),
            FileViewMode::Rendered => {
                if self.rendered_total_lines == 0 && !self.content.is_empty() {
                    self.total_lines
                } else {
                    self.rendered_total_lines
                }
            }
        }
    }

    /// Get display name for the tab (filename only)
    pub fn tab_name(&self) -> String {
        self.file_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(String::from)
            .unwrap_or_else(|| "File".to_string())
    }

    /// Get the full file path as a string
    pub fn file_path_display(&self) -> String {
        self.file_path.display().to_string()
    }

    /// Get the raw content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get detected file kind
    pub fn file_kind(&self) -> FileKind {
        self.file_kind
    }

    /// Display label for the detected file kind.
    pub fn file_kind_label(&self) -> &'static str {
        match self.file_kind {
            FileKind::Markdown => "md",
            FileKind::Json => "json",
            FileKind::Rust => "rust",
            FileKind::Toml => "toml",
            FileKind::Yaml => "yaml",
            FileKind::PlainText => "text",
        }
    }

    /// Display label for the active view mode.
    pub fn view_mode_label(&self) -> &'static str {
        match self.view_mode {
            FileViewMode::Raw => "raw",
            FileViewMode::Rendered => "rendered",
        }
    }

    /// Get current file viewer mode
    pub fn active_view_mode(&self) -> FileViewMode {
        self.view_mode
    }

    /// Get total lines used by the current mode
    pub fn effective_total_lines(&self) -> usize {
        self.active_total_lines()
    }

    /// Whether line numbers should be shown in the current mode
    pub fn should_show_line_numbers(&self) -> bool {
        self.show_line_numbers && self.view_mode == FileViewMode::Raw
    }

    /// Ensure markdown render cache is up to date for the provided width
    pub fn ensure_render_cache(&mut self, content_width: usize) {
        if self.view_mode != FileViewMode::Rendered || self.file_kind != FileKind::Markdown {
            return;
        }

        if content_width == 0 {
            self.rendered_lines.clear();
            self.rendered_total_lines = 0;
            self.last_render_width = None;
            self.scroll_offset = 0;
            return;
        }

        if self.last_render_width == Some(content_width) && !self.rendered_lines.is_empty() {
            return;
        }

        let mut renderer = MarkdownRenderer::new();
        self.rendered_lines = renderer.render_wrapped(&self.content, content_width);
        self.rendered_total_lines = self.rendered_lines.len();
        self.last_render_width = Some(content_width);

        let max_scroll = self.active_total_lines().saturating_sub(1);
        self.scroll_offset = self.scroll_offset.min(max_scroll);
    }

    /// Scroll up by N lines
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    /// Scroll down by N lines
    /// Note: For proper clamping, use scroll_down_clamped with visible height
    pub fn scroll_down(&mut self, lines: usize) {
        let max_scroll = self.active_total_lines().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + lines).min(max_scroll);
    }

    /// Scroll down by N lines, clamped to visible height
    pub fn scroll_down_clamped(&mut self, lines: usize, visible_height: usize) {
        let max_scroll = self
            .active_total_lines()
            .saturating_sub(visible_height.max(1));
        self.scroll_offset = (self.scroll_offset + lines).min(max_scroll);
    }

    /// Scroll to top of file
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// Scroll to bottom of file (uses default visible height approximation)
    pub fn scroll_to_bottom(&mut self) {
        // Default to assuming ~30 lines visible
        let default_visible = 30;
        self.scroll_offset = self.active_total_lines().saturating_sub(default_visible);
    }

    /// Scroll to bottom with exact visible height
    pub fn scroll_to_bottom_exact(&mut self, visible_height: usize) {
        self.scroll_offset = self
            .active_total_lines()
            .saturating_sub(visible_height.max(1));
    }

    /// Set scroll offset explicitly, clamped to current view range
    pub fn set_scroll_offset(&mut self, offset: usize, visible_height: usize) {
        let max_scroll = self
            .active_total_lines()
            .saturating_sub(visible_height.max(1));
        self.scroll_offset = offset.min(max_scroll);
    }

    /// Page up (scroll by default page size)
    pub fn page_up(&mut self) {
        let page_size = 20; // Default page size
        self.scroll_up(page_size);
    }

    /// Page up with exact visible height
    pub fn page_up_exact(&mut self, visible_height: usize) {
        let page_size = visible_height.saturating_sub(2).max(1);
        self.scroll_up(page_size);
    }

    /// Page down (scroll by default page size)
    pub fn page_down(&mut self) {
        let page_size = 20; // Default page size
        self.scroll_down(page_size);
    }

    /// Page down with exact visible height
    pub fn page_down_exact(&mut self, visible_height: usize) {
        let page_size = visible_height.saturating_sub(2).max(1);
        self.scroll_down_clamped(page_size, visible_height);
    }

    /// Get visible raw lines for rendering
    pub fn visible_lines(&self, visible_height: usize) -> &[String] {
        let start = self.scroll_offset;
        let end = (start + visible_height).min(self.lines.len());
        &self.lines[start..end]
    }

    /// Get visible highlighted raw lines for rendering
    pub fn visible_highlighted_raw_lines(&self, visible_height: usize) -> &[Line<'static>] {
        let start = self.scroll_offset;
        let end = (start + visible_height).min(self.raw_highlighted_lines.len());
        &self.raw_highlighted_lines[start..end]
    }

    /// Get visible rendered markdown lines for rendering
    pub fn visible_rendered_lines(&self, visible_height: usize) -> &[Line<'static>] {
        let start = self.scroll_offset;
        let end = (start + visible_height).min(self.rendered_lines.len());
        &self.rendered_lines[start..end]
    }

    /// Get all raw lines
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Toggle line numbers display
    pub fn toggle_line_numbers(&mut self) {
        self.show_line_numbers = !self.show_line_numbers;
    }

    /// Reload the file from disk
    pub fn reload(&mut self) -> std::io::Result<()> {
        let content = std::fs::read_to_string(&self.file_path)?;
        self.lines = content.lines().map(String::from).collect();
        self.total_lines = self.lines.len();
        self.content = content;

        self.file_kind = Self::detect_file_kind(&self.file_path);
        self.is_markdown = self.file_kind == FileKind::Markdown;
        if !self.is_markdown {
            self.view_mode = FileViewMode::Raw;
        }

        self.rendered_lines.clear();
        self.rendered_total_lines = 0;
        self.last_render_width = None;
        self.rebuild_raw_highlight_cache();

        // Clamp scroll offset if file got shorter
        let max_scroll = self.active_total_lines().saturating_sub(1);
        self.scroll_offset = self.scroll_offset.min(max_scroll);

        Ok(())
    }

    fn rebuild_raw_highlight_cache(&mut self) {
        self.raw_highlighted_lines =
            highlight_source_lines(self.file_kind, &self.file_path, &self.lines);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_file_viewer_session_new() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Line 1").unwrap();
        writeln!(file, "Line 2").unwrap();
        writeln!(file, "Line 3").unwrap();

        let session = FileViewerSession::new(file.path().to_path_buf()).unwrap();
        assert_eq!(session.total_lines, 3);
        assert_eq!(session.scroll_offset, 0);
        assert!(session.show_line_numbers);
        assert_eq!(session.file_kind(), FileKind::PlainText);
        assert_eq!(session.active_view_mode(), FileViewMode::Raw);
    }

    #[test]
    fn test_scroll_operations() {
        let mut file = NamedTempFile::new().unwrap();
        for i in 1..=100 {
            writeln!(file, "Line {}", i).unwrap();
        }

        let mut session = FileViewerSession::new(file.path().to_path_buf()).unwrap();
        let visible_height = 20;

        // Scroll down (using clamped version for exact behavior)
        session.scroll_down_clamped(10, visible_height);
        assert_eq!(session.scroll_offset, 10);

        // Scroll up
        session.scroll_up(5);
        assert_eq!(session.scroll_offset, 5);

        // Scroll to bottom (using exact version)
        session.scroll_to_bottom_exact(visible_height);
        assert_eq!(session.scroll_offset, 80); // 100 - 20

        // Scroll to top
        session.scroll_to_top();
        assert_eq!(session.scroll_offset, 0);
    }

    #[test]
    fn test_markdown_detection() {
        let mut file = NamedTempFile::with_suffix(".md").unwrap();
        writeln!(file, "# Markdown").unwrap();

        let session = FileViewerSession::new(file.path().to_path_buf()).unwrap();
        assert!(session.is_markdown);
        assert_eq!(session.file_kind(), FileKind::Markdown);
        assert_eq!(session.active_view_mode(), FileViewMode::Rendered);
    }

    #[test]
    fn test_json_detection() {
        let mut file = NamedTempFile::with_suffix(".json").unwrap();
        writeln!(file, "{{\"key\": true}}").unwrap();

        let session = FileViewerSession::new(file.path().to_path_buf()).unwrap();
        assert_eq!(session.file_kind(), FileKind::Json);
        assert_eq!(session.active_view_mode(), FileViewMode::Raw);
    }

    #[test]
    fn test_rust_detection() {
        let mut file = NamedTempFile::with_suffix(".rs").unwrap();
        writeln!(file, "fn main() {{}}").unwrap();

        let session = FileViewerSession::new(file.path().to_path_buf()).unwrap();
        assert_eq!(session.file_kind(), FileKind::Rust);
        assert_eq!(session.active_view_mode(), FileViewMode::Raw);
    }

    #[test]
    fn test_markdown_render_cache_changes_line_totals() {
        let mut file = NamedTempFile::with_suffix(".md").unwrap();
        writeln!(file, "# Title").unwrap();
        writeln!(
            file,
            "This paragraph is intentionally long to verify wrapped markdown rendering in narrow layouts."
        )
        .unwrap();

        let mut session = FileViewerSession::new(file.path().to_path_buf()).unwrap();
        session.ensure_render_cache(16);

        assert!(session.effective_total_lines() > session.total_lines);
        assert!(!session.visible_rendered_lines(3).is_empty());
    }

    #[test]
    fn test_markdown_scroll_uses_rendered_totals() {
        let mut file = NamedTempFile::with_suffix(".md").unwrap();
        writeln!(file, "# Header").unwrap();
        writeln!(
            file,
            "This is a longer markdown line that should wrap and increase render line count."
        )
        .unwrap();

        let mut session = FileViewerSession::new(file.path().to_path_buf()).unwrap();
        session.ensure_render_cache(18);

        let total = session.effective_total_lines();
        session.scroll_to_bottom_exact(5);
        assert_eq!(session.scroll_offset, total.saturating_sub(5));

        session.set_scroll_offset(usize::MAX, 5);
        assert_eq!(session.scroll_offset, total.saturating_sub(5));
    }

    #[test]
    fn test_tab_name() {
        let mut file = NamedTempFile::with_suffix(".txt").unwrap();
        writeln!(file, "content").unwrap();

        let session = FileViewerSession::new(file.path().to_path_buf()).unwrap();
        let name = session.tab_name();
        assert!(name.ends_with(".txt"));
    }
}
