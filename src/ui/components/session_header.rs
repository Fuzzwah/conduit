//! Session header component displaying the AI-generated session title
//!
//! This component renders a fixed header below the tab bar showing
//! the session title/description. Shows "New session" in muted text
//! when no title has been generated yet.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

use super::{bg_elevated, text_bright, text_muted};

/// Session header component
pub struct SessionHeader<'a> {
    /// The session title (None = new session)
    title: Option<&'a str>,
}

impl<'a> SessionHeader<'a> {
    /// Create a new session header
    pub fn new(title: Option<&'a str>) -> Self {
        Self { title }
    }
}

impl Widget for SessionHeader<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        // Fill entire background first (all rows in area)
        let bg_style = Style::default().bg(bg_elevated());
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf[(x, y)].set_style(bg_style).set_symbol(" ");
            }
        }

        // Display text
        let text = self.title.unwrap_or("New session");
        // Reserve 2 chars for side padding + 1 for ellipsis safety
        let max_display_chars = area.width.saturating_sub(4) as usize;

        // UTF-8 safe truncation: count by characters, not bytes
        let display = truncate_utf8(text, max_display_chars);

        // Style: bright color if we have a title, muted if placeholder
        let text_color = if self.title.is_some() {
            text_bright()
        } else {
            text_muted()
        };

        // Center the title within the available width
        let display_width = display.chars().count() as u16;
        let x_offset = area.x + area.width.saturating_sub(display_width) / 2;

        let span = Span::styled(display, bg_style.fg(text_color));
        let line = Line::from(vec![span]);

        buf.set_line(
            x_offset,
            area.y,
            &line,
            area.width.saturating_sub(x_offset - area.x),
        );
    }
}

/// UTF-8 safe string truncation that respects character boundaries
fn truncate_utf8(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        // Take max_chars - 1 to leave room for ellipsis
        let truncated: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}
