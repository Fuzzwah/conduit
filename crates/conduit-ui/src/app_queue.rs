use std::path::PathBuf;

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::components::{bg_highlight, text_muted, text_primary};
use crate::events::InputMode;
use crate::session::AgentSession;
use conduit_data::QueuedMessage;

pub fn build_queue_lines(
    session: &AgentSession,
    width: u16,
    input_mode: InputMode,
) -> Option<Vec<Line<'static>>> {
    if session.queued_messages.is_empty() {
        return None;
    }

    let max_width = width.saturating_sub(2) as usize;
    let selected = if input_mode == InputMode::QueueEditing {
        session.queue_selection
    } else {
        None
    };

    let mut lines = Vec::new();
    for (idx, msg) in session.queued_messages.iter().enumerate() {
        let raw = msg.text.trim();
        let mut preview = raw.lines().next().unwrap_or("").trim().to_string();
        if preview.is_empty() {
            preview = "<empty>".to_string();
        }
        if raw.contains('\n') {
            preview.push_str(" ...");
        }
        let mut text = format!("{}: {}", msg.mode.label(), preview);
        if !msg.images.is_empty() {
            let count = msg.images.len();
            let suffix = if count == 1 { "image" } else { "images" };
            text.push_str(&format!(" [{} {}]", count, suffix));
        }

        if max_width > 0 {
            text = truncate_queue_line(&text, max_width);
        }

        let prefix = if selected == Some(idx) { "› " } else { "  " };
        let line_text = format!("{prefix}{text}");
        let style = if selected == Some(idx) {
            Style::default()
                .fg(text_primary())
                .bg(bg_highlight())
                .add_modifier(Modifier::ITALIC)
        } else {
            Style::default()
                .fg(text_muted())
                .add_modifier(Modifier::ITALIC)
        };
        lines.push(Line::from(Span::styled(line_text, style)));
    }

    Some(lines)
}

pub fn clamp_queue_selection(session: &mut AgentSession) {
    if session.queued_messages.is_empty() {
        session.queue_selection = None;
        return;
    }
    if let Some(idx) = session.queue_selection {
        let max = session.queued_messages.len().saturating_sub(1);
        if idx > max {
            session.queue_selection = Some(max);
        }
    }
}

pub fn truncate_queue_line(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let ellipsis = "...";
    let ellipsis_width = UnicodeWidthStr::width(ellipsis);
    let current_width = UnicodeWidthStr::width(text);
    if current_width <= max_width {
        return text.to_string();
    }
    if max_width <= ellipsis_width {
        return ellipsis[..max_width.min(ellipsis.len())].to_string();
    }
    let target = max_width - ellipsis_width;
    let mut width = 0;
    let mut result = String::new();
    for c in text.chars() {
        let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
        if width + char_width > target {
            break;
        }
        result.push(c);
        width += char_width;
    }
    result.push_str(ellipsis);
    result
}

pub fn queued_to_submission(message: &QueuedMessage) -> (String, Vec<PathBuf>, Vec<String>) {
    let image_paths = message.images.iter().map(|img| img.path.clone()).collect();
    let placeholders = message
        .images
        .iter()
        .map(|img| img.placeholder.clone())
        .collect();
    (message.text.clone(), image_paths, placeholders)
}

pub fn build_queued_submission(
    messages: &[QueuedMessage],
    delivery: conduit_config::QueueDelivery,
) -> (String, Vec<PathBuf>, Vec<String>) {
    let mut lines = Vec::new();
    if messages.len() == 1 || delivery == conduit_config::QueueDelivery::Concat {
        for msg in messages {
            lines.push(msg.text.trim().to_string());
        }
    } else {
        for (idx, msg) in messages.iter().enumerate() {
            lines.push(format!("[Queued {} of {}]", idx + 1, messages.len()));
            lines.push(msg.text.trim().to_string());
        }
    }

    let mut image_paths = Vec::new();
    let mut placeholders = Vec::new();
    for msg in messages {
        for img in &msg.images {
            image_paths.push(img.path.clone());
            placeholders.push(img.placeholder.clone());
        }
    }

    (lines.join("\n\n"), image_paths, placeholders)
}
