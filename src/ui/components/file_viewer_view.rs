//! File viewer component for displaying file contents with scrolling
//!
//! This component renders a file's contents with optional line numbers
//! and a scrollbar.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use super::source_highlighter::truncate_spans_with_ellipsis;
use super::{render_minimal_scrollbar, text_muted};
use crate::ui::file_viewer::{FileViewMode, FileViewerSession};

/// Renders a file viewer with line numbers and scrolling
pub struct FileViewerView<'a> {
    session: &'a mut FileViewerSession,
}

impl<'a> FileViewerView<'a> {
    pub fn new(session: &'a mut FileViewerSession) -> Self {
        Self { session }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let visible_height = area.height as usize;

        match self.session.active_view_mode() {
            FileViewMode::Raw => self.render_raw(area, buf, visible_height),
            FileViewMode::Rendered => self.render_rendered(area, buf, visible_height),
        }

        // Render scrollbar for active mode.
        let total_lines = self.session.effective_total_lines();
        if total_lines > 0 {
            let scrollbar_area = Rect {
                x: area.x + area.width.saturating_sub(1),
                y: area.y,
                width: 1,
                height: area.height,
            };
            render_minimal_scrollbar(
                scrollbar_area,
                buf,
                total_lines,
                visible_height,
                self.session.scroll_offset,
            );
        }
    }

    fn render_raw(&self, area: Rect, buf: &mut Buffer, visible_height: usize) {
        const MARGIN: u16 = 2;

        // Calculate width needed for line numbers.
        let line_num_width = if self.session.should_show_line_numbers() {
            let max_line = self.session.total_lines;
            let digits = if max_line == 0 {
                1
            } else {
                (max_line as f64).log10().floor() as usize + 1
            };
            digits + 3 // digits + " │ "
        } else {
            0
        };

        // Reserve left margin and 1 column for scrollbar.
        let content_width =
            (area.width as usize).saturating_sub(line_num_width + 1 + MARGIN as usize);
        let lines = self.session.visible_highlighted_raw_lines(visible_height);

        for (i, highlighted_line) in lines.iter().enumerate() {
            let y = area.y + i as u16;
            if y >= area.y + area.height {
                break;
            }

            let line_num = self.session.scroll_offset + i + 1;
            let mut spans = Vec::new();

            if self.session.should_show_line_numbers() {
                let num_str = format!(
                    "{:>width$} │ ",
                    line_num,
                    width = line_num_width.saturating_sub(3)
                );
                spans.push(Span::styled(num_str, Style::default().fg(text_muted())));
            }

            if content_width > 0 {
                spans.extend(truncate_spans_with_ellipsis(
                    &highlighted_line.spans,
                    content_width,
                ));
            }

            let line = Line::from(spans);
            let line_area = Rect {
                x: area.x + MARGIN,
                y,
                width: area.width.saturating_sub(MARGIN + 1),
                height: 1,
            };
            Paragraph::new(line).render(line_area, buf);
        }
    }

    fn render_rendered(&mut self, area: Rect, buf: &mut Buffer, visible_height: usize) {
        const MARGIN: u16 = 2;

        let content_width = area.width.saturating_sub(MARGIN + 1) as usize;
        self.session.ensure_render_cache(content_width);

        let lines = self.session.visible_rendered_lines(visible_height);
        for (i, line) in lines.iter().enumerate() {
            let y = area.y + i as u16;
            if y >= area.y + area.height {
                break;
            }

            let line_area = Rect {
                x: area.x + MARGIN,
                y,
                width: area.width.saturating_sub(MARGIN + 1),
                height: 1,
            };
            Paragraph::new(line.clone()).render(line_area, buf);
        }
    }
}
