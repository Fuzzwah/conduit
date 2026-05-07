use ansi_to_tui::IntoText;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use std::borrow::Cow;
use std::path::Path;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::{
    render_minimal_scrollbar,
    source_highlighter::highlight_file_for_tool,
    theme::{
        accent_error, accent_primary, accent_success, bg_base, bg_highlight, diff_add, diff_remove,
        markdown_code_bg, text_muted, theme_revision, tool_block_bg, tool_command, tool_comment,
        tool_output,
    },
    ChatMessage, MarkdownRenderer, MessageRole, ScrollbarMetrics, TurnSummary,
};

mod chat_view_cache;

// =============================================================================
// Tool Block Builder - Opencode-style tool rendering
// =============================================================================

/// Helper for building Opencode-style tool blocks with consistent styling.
/// Creates lines with ┃ prefix and full-width background.
struct ToolBlockBuilder {
    width: usize,
    block_style: Style,
    bg_style: Style,
}

impl ToolBlockBuilder {
    fn new(width: usize) -> Self {
        Self {
            width,
            // Use conversation background color as foreground so ┃ blends with surrounding area
            block_style: Style::default().fg(bg_base()).bg(tool_block_bg()),
            bg_style: Style::default().bg(tool_block_bg()),
        }
    }

    /// Create a line with ┃ prefix and full-width background
    fn line(&self, spans: Vec<Span<'static>>) -> Line<'static> {
        // Note: "┃" is a box-drawing character with ambiguous width.
        // We treat it as width 1, plus 2 spaces = 3 total prefix width.
        let prefix_width = 3; // "┃" (1) + "  " (2)

        let content_width: usize = spans
            .iter()
            .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
            .sum();

        let total_used = prefix_width + content_width;
        // Add 1 extra character of padding to prevent background color from stopping
        // 1 character short at certain terminal widths due to unicode width calculation
        // differences between ratatui and actual terminal rendering
        let padding_needed = self.width.saturating_sub(total_used).saturating_add(1);

        let mut line_spans = vec![
            Span::styled("┃", self.block_style),
            Span::styled("  ", self.bg_style),
        ];
        line_spans.extend(spans);

        line_spans.push(Span::styled(" ".repeat(padding_needed), self.bg_style));

        Line::from(line_spans)
    }

    /// Create an empty line for padding (fills entire width)
    fn empty_line(&self) -> Line<'static> {
        let prefix_width = 3; // "┃" (1) + "  " (2)
                              // Add 1 extra character of padding to prevent background color from stopping
                              // 1 character short at certain terminal widths due to unicode width calculation
                              // differences between ratatui and actual terminal rendering
        let padding = self.width.saturating_sub(prefix_width).saturating_add(1);
        Line::from(vec![
            Span::styled("┃", self.block_style),
            Span::styled("  ", self.bg_style),
            Span::styled(" ".repeat(padding), self.bg_style),
        ])
    }

    /// Create a comment line (# prefix, muted color)
    fn comment(&self, text: &str) -> Line<'static> {
        self.line(vec![Span::styled(
            format!("# {}", text),
            Style::default().fg(tool_comment()).bg(tool_block_bg()),
        )])
    }

    /// Create a command line ($ prefix, bright color)
    fn command(&self, text: &str) -> Line<'static> {
        self.line(vec![Span::styled(
            format!("$ {}", text),
            Style::default().fg(tool_command()).bg(tool_block_bg()),
        )])
    }

    /// Create an output line (normal color)
    fn output(&self, text: &str) -> Line<'static> {
        self.line(vec![Span::styled(
            text.to_string(),
            Style::default().fg(tool_output()).bg(tool_block_bg()),
        )])
    }

    /// Create a colored output line
    fn output_colored(&self, text: &str, color: Color) -> Line<'static> {
        self.line(vec![Span::styled(
            text.to_string(),
            Style::default().fg(color).bg(tool_block_bg()),
        )])
    }

    /// Create a line with custom spans
    fn custom(&self, spans: Vec<Span<'static>>) -> Line<'static> {
        self.line(spans)
    }

    /// Get the background style for use in custom spans
    fn bg_style(&self) -> Style {
        self.bg_style
    }

    /// Get the content width (total width minus prefix)
    fn content_width(&self) -> usize {
        let prefix_width = 3; // "┃  "
        self.width.saturating_sub(prefix_width).max(1)
    }

    /// Wrap text and return multiple lines with the given color
    fn wrapped_output_colored(&self, text: &str, color: Color) -> Vec<Line<'static>> {
        let content_width = self.content_width();
        let style = Style::default().fg(color).bg(tool_block_bg());
        let spans = vec![Span::styled(text.to_string(), style)];
        let wrapped = wrap_spans(spans, content_width);

        wrapped
            .into_iter()
            .map(|line_spans| self.line(line_spans))
            .collect()
    }

    /// Wrap custom spans and return multiple lines
    fn wrapped_custom(&self, spans: Vec<Span<'static>>) -> Vec<Line<'static>> {
        let content_width = self.content_width();
        let wrapped = wrap_spans(spans, content_width);

        wrapped
            .into_iter()
            .map(|line_spans| self.line(line_spans))
            .collect()
    }
}

// =============================================================================
// User Message Block Builder - Accent stripe with base background
// =============================================================================

/// Helper for building user message blocks with an accent stripe and base background.
/// Uses the same ┃ glyph as tool blocks, but keeps the base background.
struct UserMessageBlockBuilder {
    width: usize,
    stripe_style: Style,
    bg_style: Style,
}

impl UserMessageBlockBuilder {
    const PREFIX_WIDTH: usize = 3; // "┃" (1) + "  " (2)
    const RIGHT_PADDING: usize = 2;

    fn new(width: usize) -> Self {
        Self {
            width,
            stripe_style: Style::default().fg(accent_primary()).bg(bg_base()),
            bg_style: Style::default().bg(tool_block_bg()),
        }
    }

    /// Create a line with ┃ prefix, left padding, content, and right padding.
    fn line(&self, spans: Vec<Span<'static>>) -> Line<'static> {
        let content_width: usize = spans
            .iter()
            .map(|s| UnicodeWidthStr::width(s.content.as_ref()))
            .sum();
        let total_used = Self::PREFIX_WIDTH + Self::RIGHT_PADDING + content_width;
        let padding_needed = self.width.saturating_sub(total_used);

        let mut line_spans = vec![
            Span::styled("┃", self.stripe_style),
            Span::styled("  ", self.bg_style),
        ];
        line_spans.extend(spans);
        line_spans.push(Span::styled(" ".repeat(Self::RIGHT_PADDING), self.bg_style));
        if padding_needed > 0 {
            line_spans.push(Span::styled(" ".repeat(padding_needed), self.bg_style));
        }

        Line::from(line_spans)
    }

    /// Create an empty line for vertical padding (fills the line width).
    fn empty_line(&self) -> Line<'static> {
        self.line(Vec::new())
    }

    /// Get the content width (total width minus prefix and right padding).
    fn content_width(&self) -> usize {
        self.width
            .saturating_sub(Self::PREFIX_WIDTH + Self::RIGHT_PADDING)
            .max(1)
    }
}

use self::chat_view_cache::LineCache;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelectionPoint {
    line_index: usize,
    column: u16,
}

use super::truncate_to_width;

/// Truncate a string to fit within a maximum display width (no ellipsis).
/// Uses unicode display width to handle multi-byte and wide characters correctly.
fn truncate_to_width_exact(s: &str, max_width: usize) -> String {
    let current_width = UnicodeWidthStr::width(s);
    if current_width <= max_width {
        return s.to_string();
    }

    let mut width = 0;
    let mut result = String::new();

    for c in s.chars() {
        let char_width = UnicodeWidthChar::width(c).unwrap_or(0);
        if width + char_width > max_width {
            break;
        }
        result.push(c);
        width += char_width;
    }

    result
}

/// Normalize carriage returns and backspaces while preserving ANSI sequences.
/// Strip the `cat -n` style line-number prefix ("   N\t") that the Read tool prepends.
fn strip_line_number_prefix(line: &str) -> &str {
    // Format is optional leading spaces, one or more digits, then a tab character.
    let trimmed = line.trim_start_matches(' ');
    let digits_end = trimmed
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(trimmed.len());
    if digits_end > 0 && trimmed.as_bytes().get(digits_end) == Some(&b'\t') {
        &trimmed[digits_end + 1..]
    } else {
        line
    }
}

/// Extract the `file_path` field from JSON tool args (used by the Read tool).
fn extract_file_path_from_args(tool_args: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(tool_args).ok()?;
    json.get("file_path")
        .or_else(|| json.get("filePath"))
        .or_else(|| json.get("path"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

fn normalize_tool_output_line(line: &str) -> Cow<'_, str> {
    let bytes = line.as_bytes();
    let has_cr = bytes.contains(&b'\r');
    let has_bs = bytes.contains(&b'\x08');

    if !has_cr && !has_bs {
        return Cow::Borrowed(line);
    }

    let mut current: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut visible_stack: Vec<usize> = Vec::new();
    let mut sgr_state: Vec<u8> = Vec::new();
    let mut prefix_ansi: Vec<u8> = Vec::new();
    let mut saw_cr = false;
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\x1b' {
            if i + 1 >= bytes.len() {
                break;
            }

            let start = i;
            let next = bytes[i + 1];
            match next {
                b'[' => {
                    let mut j = i + 2;
                    while j < bytes.len() {
                        let b = bytes[j];
                        if (0x40..=0x7E).contains(&b) {
                            j += 1;
                            break;
                        }
                        j += 1;
                    }
                    let seq = &bytes[start..j];
                    current.extend_from_slice(seq);
                    if seq.last() == Some(&b'm') {
                        sgr_state.extend_from_slice(seq);
                    }
                    i = j;
                    continue;
                }
                b']' => {
                    let mut j = i + 2;
                    while j < bytes.len() {
                        if bytes[j] == b'\x07' {
                            j += 1;
                            break;
                        }
                        if bytes[j] == b'\x1b' && j + 1 < bytes.len() && bytes[j + 1] == b'\\' {
                            j += 2;
                            break;
                        }
                        j += 1;
                    }
                    current.extend_from_slice(&bytes[start..j]);
                    i = j;
                    continue;
                }
                _ => {
                    let end = (i + 2).min(bytes.len());
                    current.extend_from_slice(&bytes[start..end]);
                    i = end;
                    continue;
                }
            }
        }

        if bytes[i] == b'\r' {
            saw_cr = true;
            prefix_ansi = sgr_state.clone();
            current.clear();
            visible_stack.clear();
            i += 1;
            continue;
        }

        if bytes[i] == b'\x08' {
            if let Some(len) = visible_stack.pop() {
                let new_len = current.len().saturating_sub(len);
                current.truncate(new_len);
            }
            i += 1;
            continue;
        }

        if bytes[i].is_ascii_control() && bytes[i] != b'\t' {
            i += 1;
            continue;
        }

        let b = bytes[i];
        let char_len = if b < 0x80 {
            1
        } else if (b & 0xE0) == 0xC0 {
            2
        } else if (b & 0xF0) == 0xE0 {
            3
        } else if (b & 0xF8) == 0xF0 {
            4
        } else {
            1
        };
        let end = (i + char_len).min(bytes.len());
        current.extend_from_slice(&bytes[i..end]);
        visible_stack.push(end - i);
        i = end;
    }

    if !saw_cr {
        return Cow::Owned(String::from_utf8_lossy(&current).into_owned());
    }

    let mut out = prefix_ansi;
    out.extend_from_slice(&current);
    Cow::Owned(String::from_utf8_lossy(&out).into_owned())
}

/// Strip ANSI escape sequences that `ansi_to_tui` doesn't consume (e.g. ESC ( B).
fn sanitize_tool_output_line(line: &str) -> Cow<'_, str> {
    if !line.as_bytes().contains(&b'\x1b') {
        return Cow::Borrowed(line);
    }

    let bytes = line.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] != b'\x1b' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }

        if i + 1 >= bytes.len() {
            break;
        }

        match bytes[i + 1] {
            // Charset designation (SCS): ESC ( B or ESC ) 0
            b'(' | b')' => {
                i += 2;
                if i < bytes.len() {
                    i += 1;
                }
            }
            // DCS/APC/PM/SOS: ESC P ... ESC \
            b'P' | b'_' | b'^' | b'X' => {
                i += 2;
                while i < bytes.len() {
                    if bytes[i] == b'\x1b' && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            }
            // ESC % @ / ESC % G
            b'%' => {
                i += 2;
                if i < bytes.len() {
                    i += 1;
                }
            }
            // Keep CSI/OSC for ansi_to_tui; drop other single-char escapes.
            b'[' | b']' => {
                out.push(b'\x1b');
                i += 1;
            }
            _ => {
                i += 2;
            }
        }
    }

    Cow::Owned(String::from_utf8_lossy(&out).into_owned())
}

/// Chat view component displaying message history
#[derive(Debug, Clone)]
struct StreamingMessage {
    role: MessageRole,
    content: String,
}

pub struct ChatView {
    /// All messages in the chat
    messages: Vec<ChatMessage>,
    /// Scroll offset (0 = bottom, increases upward)
    scroll_offset: usize,
    /// Currently streaming messages (in arrival order)
    streaming_messages: Vec<StreamingMessage>,
    /// Cached rendered lines per message
    line_cache: LineCache,
    /// Width the cache was built for (invalidate on change)
    cache_width: Option<u16>,
    /// Flattened cache of all message lines
    flat_cache: Vec<Line<'static>>,
    /// Width the flattened cache was built for
    flat_cache_width: Option<u16>,
    /// Whether the flattened cache needs rebuilding
    flat_cache_dirty: bool,
    /// Cached lines for current streaming message
    streaming_cache: Option<Vec<Line<'static>>>,
    /// Selection anchor (content space)
    selection_anchor: Option<SelectionPoint>,
    /// Selection head (content space)
    selection_head: Option<SelectionPoint>,
    /// Joiner string to insert before each wrapped line
    joiner_before: Vec<Option<String>>,
    /// Joiners for streaming cache (aligned to streaming_cache)
    streaming_joiner_before: Option<Vec<Option<String>>>,
    /// Selection scroll lock (offset from top)
    selection_scroll_lock: Option<usize>,
    /// User scroll pin: absolute line-from-top when user has scrolled up, cleared at bottom
    pinned_scroll_top: Option<usize>,
    /// Theme revision used for cached lines (invalidate on change)
    theme_revision: u64,
    /// Extra lines appended in the last render (thinking/queue/prompt + spacing)
    last_render_extra_lines: usize,
    /// Currently hovered file path (for underline highlighting)
    hovered_file_path: Option<HoveredFilePath>,
    /// Extra lines from last render (prompts, indicators) for hover detection
    last_extra_lines: Vec<Line<'static>>,
    /// Starting line index for extra lines (cached_len + streaming_len)
    last_extra_lines_start: usize,
    /// Label for the agent (e.g. "Claude", "Codex") shown above assistant messages
    agent_label: String,
    /// Flat cache line span (start, end) for each line_cache entry, parallel to line_cache.entries
    flat_cache_entry_spans: Vec<(usize, usize)>,
    /// Viewport height from the last render pass (used by nearest_code_block_content)
    last_visible_height: usize,
    /// Currently selected code block index (newest-first order). None = nothing selected yet.
    code_block_cycle_idx: Option<usize>,
    /// Total code block count seen on the last Alt+y press; resets cycle when it grows
    code_block_last_total: usize,
    /// Which code block is currently highlighted after a copy: (entry_idx, block_within_entry) in forward order
    highlighted_code_block: Option<(usize, usize)>,
    /// Flat-cache line spans for each code block: (entry_idx, block_within, flat_start, flat_end)
    flat_code_block_spans: Vec<(usize, usize, usize, usize)>,
    /// Height (in rows) occupied by the pinned agent message header during the last render
    last_pin_height: usize,
}

/// Information about a hovered file path for rendering
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoveredFilePath {
    /// The line index in the flat cache where the path is located
    pub line_index: usize,
    /// The start column (display position) of the path
    pub start_col: usize,
    /// The end column (display position) of the path
    pub end_col: usize,
    /// The actual file path string
    pub path: String,
}

impl ChatView {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
            streaming_messages: Vec::new(),
            line_cache: LineCache::default(),
            cache_width: None,
            flat_cache: Vec::new(),
            flat_cache_width: None,
            flat_cache_dirty: true,
            streaming_cache: None,
            selection_anchor: None,
            selection_head: None,
            joiner_before: Vec::new(),
            streaming_joiner_before: None,
            selection_scroll_lock: None,
            pinned_scroll_top: None,
            theme_revision: theme_revision(),
            last_render_extra_lines: 0,
            hovered_file_path: None,
            last_extra_lines: Vec::new(),
            last_extra_lines_start: 0,
            agent_label: "Claude".to_string(),
            flat_cache_entry_spans: Vec::new(),
            last_visible_height: 0,
            last_pin_height: 0,
            code_block_cycle_idx: None,
            code_block_last_total: 0,
            highlighted_code_block: None,
            flat_code_block_spans: Vec::new(),
        }
    }

    /// Set the agent label displayed above assistant messages
    pub fn set_agent_label(&mut self, label: String) {
        if self.agent_label != label {
            self.agent_label = label;
            // Invalidate caches since agent label affects rendered output
            self.line_cache = LineCache::default();
            self.flat_cache.clear();
            self.flat_cache_width = None;
            self.flat_cache_dirty = true;
            self.streaming_cache = None;
            self.streaming_joiner_before = None;
            self.cache_width = None;
        }
    }

    /// Return the dedented content of the bottommost visible code block.
    ///
    /// Returns `Some((content, block_index, total_blocks))` where `block_index` is
    /// 1-based within visible blocks, or `None` if no code blocks are visible.
    /// Return a code block by cycling backwards through all blocks (newest first).
    ///
    /// Each call advances the cycle index so repeated presses of the hotkey walk
    /// from the most recent code block toward older ones, wrapping around.
    /// Returns `Some((content, 1-based index, total))` or `None` if no blocks exist.
    pub fn nearest_code_block_content(&mut self) -> Option<(String, usize, usize)> {
        // Collect all code blocks newest-first: reverse over messages, reverse within each message.
        let all_blocks: Vec<String> = self
            .line_cache
            .entries
            .iter()
            .rev()
            .filter_map(|e| e.as_ref())
            .flat_map(|e| e.code_blocks.iter().rev().cloned())
            .collect();

        let total = all_blocks.len();
        if total == 0 {
            self.code_block_cycle_idx = None;
            self.code_block_last_total = 0;
            return None;
        }

        if total > self.code_block_last_total {
            self.code_block_cycle_idx = None;
        }
        self.code_block_last_total = total;

        let idx = match self.code_block_cycle_idx {
            None => 0,
            Some(i) => (i + 1) % total,
        };
        self.code_block_cycle_idx = Some(idx);

        // Find (entry_idx, block_within_entry) for idx in the newest-first ordering.
        // Entries are visited in reverse (newest first), blocks within each entry in reverse.
        let mut remaining = idx;
        let mut found: Option<(usize, usize)> = None;
        'find: for (fwd_entry_idx, entry_opt) in self.line_cache.entries.iter().enumerate().rev() {
            if let Some(entry) = entry_opt {
                let n = entry.code_blocks.len();
                if remaining < n {
                    found = Some((fwd_entry_idx, n - 1 - remaining));
                    break 'find;
                }
                remaining = remaining.saturating_sub(n);
            }
        }
        self.highlighted_code_block = found;
        self.flat_cache_dirty = true;
        self.scroll_to_show_highlighted_code_block();

        Some((Self::dedent(&all_blocks[idx]), idx + 1, total))
    }

    /// Like `nearest_code_block_content` but cycles in reverse (toward newer blocks).
    pub fn prev_code_block_content(&mut self) -> Option<(String, usize, usize)> {
        let all_blocks: Vec<String> = self
            .line_cache
            .entries
            .iter()
            .rev()
            .filter_map(|e| e.as_ref())
            .flat_map(|e| e.code_blocks.iter().rev().cloned())
            .collect();

        let total = all_blocks.len();
        if total == 0 {
            self.code_block_cycle_idx = None;
            self.code_block_last_total = 0;
            return None;
        }

        if total > self.code_block_last_total {
            self.code_block_cycle_idx = None;
        }
        self.code_block_last_total = total;

        let idx = match self.code_block_cycle_idx {
            None | Some(0) => total - 1,
            Some(i) => i - 1,
        };
        self.code_block_cycle_idx = Some(idx);

        let mut remaining = idx;
        let mut found: Option<(usize, usize)> = None;
        'find: for (fwd_entry_idx, entry_opt) in self.line_cache.entries.iter().enumerate().rev() {
            if let Some(entry) = entry_opt {
                let n = entry.code_blocks.len();
                if remaining < n {
                    found = Some((fwd_entry_idx, n - 1 - remaining));
                    break 'find;
                }
                remaining = remaining.saturating_sub(n);
            }
        }
        self.highlighted_code_block = found;
        self.flat_cache_dirty = true;
        self.scroll_to_show_highlighted_code_block();

        Some((Self::dedent(&all_blocks[idx]), idx + 1, total))
    }

    /// Scroll so the currently highlighted code block is visible.
    /// Must be called after `highlighted_code_block` and `flat_cache_dirty` are set.
    fn scroll_to_show_highlighted_code_block(&mut self) {
        if self.last_visible_height == 0 {
            return;
        }
        let Some((entry_idx, block_within)) = self.highlighted_code_block else {
            return;
        };

        // Rebuild flat cache so spans reflect the current state.
        self.ensure_flat_cache();

        let span = self
            .flat_code_block_spans
            .iter()
            .find(|(ei, bi, _, _)| *ei == entry_idx && *bi == block_within)
            .copied();
        let Some((_, _, flat_start, flat_end)) = span else {
            return;
        };

        let total = self.flat_cache.len();
        // Subtract pinned header height so the scroll target doesn't land under the pin.
        let visible = self
            .last_visible_height
            .saturating_sub(self.last_pin_height);
        let max_scroll = total.saturating_sub(visible);

        // Compute the current visible range [view_top, view_top + visible).
        let current_offset = self.scroll_offset.min(max_scroll);
        let view_top = total.saturating_sub(current_offset + visible);

        // Check if the entire block is already on screen.
        if flat_start >= view_top && flat_end <= view_top + visible {
            return;
        }

        // Choose a new view_top that brings the block into view.
        let new_view_top = if flat_start < view_top {
            // Block is above the viewport — scroll up, leaving a small margin.
            flat_start.saturating_sub(2)
        } else {
            // Block is below the viewport — scroll down to show its end, with margin.
            let block_height = flat_end.saturating_sub(flat_start);
            if block_height >= visible {
                flat_start
            } else {
                flat_end.saturating_sub(visible.saturating_sub(2))
            }
        };

        self.scroll_offset = max_scroll.saturating_sub(new_view_top.min(max_scroll));
        // Pin the position so it stays fixed while new content arrives.
        self.pinned_scroll_top = if self.scroll_offset > 0 {
            Some(new_view_top)
        } else {
            None
        };
    }

    fn dedent(content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let min_indent = lines
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start_matches(' ').len())
            .min()
            .unwrap_or(0);
        if min_indent == 0 {
            return content.to_string();
        }
        lines
            .iter()
            .map(|l| l.get(min_indent..).unwrap_or(l))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Calculate content area with padding for left margin and optional scrollbar.
    fn content_area(area: Rect, show_scrollbar: bool) -> Option<Rect> {
        let width = if show_scrollbar {
            area.width.saturating_sub(4) // 1 left margin + 1 scrollbar + 1 gap + 1 right margin
        } else {
            area.width.saturating_sub(2) // 1 left margin + 1 right margin
        };
        let content = Rect {
            x: area.x.saturating_add(1),
            y: area.y,
            width,
            height: area.height,
        };
        if content.width < 3 || content.height < 1 {
            return None;
        }
        Some(content)
    }

    pub(crate) fn content_area_for(area: Rect, show_scrollbar: bool) -> Option<Rect> {
        Self::content_area(area, show_scrollbar)
    }

    /// Add a message to the chat
    pub fn push(&mut self, message: ChatMessage) {
        // If we were streaming, finalize it
        if !self.streaming_messages.is_empty() {
            self.finalize_streaming();
        }

        // Update previous message's spacing if needed (it may have changed)
        if !self.messages.is_empty() {
            if let Some(width) = self.cache_width {
                let prev_idx = self.messages.len() - 1;
                self.invalidate_cache_entry(prev_idx);
                self.update_cache_entry(prev_idx, width);
            }
        }

        self.messages.push(message);

        // Add cache entry for new message if cache is active
        if let Some(width) = self.cache_width {
            let idx = self.messages.len() - 1;
            self.update_cache_entry(idx, width);
        }

        // Auto-scroll to bottom only if user is already at bottom
        // When scroll_offset > 0, user has scrolled up - preserve their position
    }

    /// Update the last tool message with new content and exit code.
    /// Returns true if update was successful, false if no matching tool message was found.
    pub fn update_last_tool(&mut self, content: String, exit_code: Option<i32>) -> bool {
        if let Some(idx) = self
            .messages
            .iter()
            .rposition(|m| m.role == MessageRole::Tool)
        {
            return self.update_tool_at(idx, content, exit_code);
        }

        false
    }

    /// Update a tool message at a specific index.
    /// Returns true if update was successful, false if no matching tool message was found.
    pub fn update_tool_at(
        &mut self,
        index: usize,
        content: String,
        exit_code: Option<i32>,
    ) -> bool {
        let Some(msg) = self.messages.get_mut(index) else {
            return false;
        };

        if msg.role != MessageRole::Tool {
            return false;
        }

        msg.content = content;
        msg.exit_code = exit_code;

        // For Read tool on images, cache file size now (while file still exists)
        if msg.file_size.is_none() {
            if let Some(ref tool_name) = msg.tool_name {
                if tool_name == "Read" {
                    if let Some(ref tool_args) = msg.tool_args {
                        if Self::is_image_file(tool_args) {
                            msg.file_size = Self::get_file_size_from_args_as_u64(tool_args);
                        }
                    }
                }
            }
        }

        // Invalidate cache for this message
        if let Some(width) = self.cache_width {
            self.invalidate_cache_entry(index);
            self.update_cache_entry(index, width);
        }

        true
    }

    /// Start or append to streaming message
    pub fn stream_append(&mut self, text: &str) {
        self.stream_append_role(MessageRole::Assistant, text);
    }

    /// Start or append to streaming message for a specific role
    pub fn stream_append_role(&mut self, role: MessageRole, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(idx) = self
            .streaming_messages
            .iter()
            .rposition(|message| message.role == role)
        {
            self.streaming_messages[idx].content.push_str(text);
        } else {
            self.streaming_messages.push(StreamingMessage {
                role,
                content: text.to_string(),
            });
        }
        // Invalidate streaming cache so it gets rebuilt on next render
        self.streaming_cache = None;
        self.streaming_joiner_before = None;
    }

    /// Finalize streaming messages and add to history
    pub fn finalize_streaming(&mut self) {
        if self.streaming_messages.is_empty() {
            return;
        }

        let streaming_messages = std::mem::take(&mut self.streaming_messages);
        // Clear streaming cache
        self.streaming_cache = None;
        self.streaming_joiner_before = None;

        for message in streaming_messages {
            let chat_message = match message.role {
                MessageRole::Assistant => ChatMessage::assistant(message.content),
                MessageRole::Reasoning => ChatMessage::reasoning(message.content),
                MessageRole::System => ChatMessage::system(message.content),
                MessageRole::Error => ChatMessage::error(message.content),
                MessageRole::User => ChatMessage::user(message.content),
                MessageRole::Tool | MessageRole::Summary => ChatMessage::assistant(message.content),
            };
            self.push(chat_message);
        }
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
        self.streaming_messages.clear();
        self.scroll_offset = 0;
        self.pinned_scroll_top = None;
        self.clear_selection();
        self.last_render_extra_lines = 0;
        // Clear all caches
        self.line_cache = LineCache::default();
        self.flat_cache.clear();
        self.flat_cache_width = self.cache_width;
        self.flat_cache_dirty = false;
        self.streaming_cache = None;
        self.joiner_before.clear();
        self.streaming_joiner_before = None;
        self.code_block_cycle_idx = None;
        // Keep cache_width so we don't have to recalculate on next render
    }

    /// Remove the last user message and all subsequent messages.
    /// Returns true if a turn was removed, false if there were no user messages.
    pub fn pop_last_turn(&mut self) -> bool {
        let Some(last_user_idx) = self
            .messages
            .iter()
            .rposition(|m| m.role == MessageRole::User)
        else {
            return false;
        };
        self.messages.truncate(last_user_idx);
        self.streaming_messages.clear();
        self.scroll_offset = 0;
        self.pinned_scroll_top = None;
        self.clear_selection();
        self.last_render_extra_lines = 0;
        self.line_cache = LineCache::default();
        self.flat_cache.clear();
        self.flat_cache_width = self.cache_width;
        self.flat_cache_dirty = false;
        self.streaming_cache = None;
        self.joiner_before.clear();
        self.streaming_joiner_before = None;
        true
    }

    /// Scroll up by n lines
    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(n);
        self.pinned_scroll_top = None; // render will re-establish pin at new position
    }

    /// Scroll down by n lines
    pub fn scroll_down(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
        self.pinned_scroll_top = None; // render will re-establish pin or clear at bottom
    }

    /// Scroll to top
    pub fn scroll_to_top(&mut self) {
        // Will be clamped during render
        self.scroll_offset = usize::MAX;
        self.pinned_scroll_top = None;
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.pinned_scroll_top = None;
    }

    /// Jump to previous user message (returns true if moved).
    pub fn scroll_to_prev_user_message(
        &mut self,
        width: u16,
        visible_height: usize,
        extra_lines: usize,
    ) -> bool {
        self.scroll_to_user_message(width, visible_height, extra_lines, true)
    }

    /// Jump to next user message (returns true if moved).
    pub fn scroll_to_next_user_message(
        &mut self,
        width: u16,
        visible_height: usize,
        extra_lines: usize,
    ) -> bool {
        self.scroll_to_user_message(width, visible_height, extra_lines, false)
    }

    fn scroll_to_user_message(
        &mut self,
        width: u16,
        visible_height: usize,
        extra_lines: usize,
        prev: bool,
    ) -> bool {
        if visible_height == 0 {
            return false;
        }

        self.ensure_cache(width);
        self.ensure_flat_cache();
        self.ensure_streaming_cache(width);

        let user_lines = self.user_message_line_indices();
        if user_lines.is_empty() {
            return false;
        }

        let cached_len = self.flat_cache.len();
        let streaming_len = self
            .streaming_cache
            .as_ref()
            .map(|lines| lines.len())
            .unwrap_or(0);
        let total_lines = cached_len + streaming_len + extra_lines;
        let max_scroll = total_lines.saturating_sub(visible_height);
        let scroll_from_top = max_scroll.saturating_sub(self.scroll_offset.min(max_scroll));

        let target = if prev {
            user_lines.iter().rev().find(|&&idx| idx < scroll_from_top)
        } else {
            user_lines.iter().find(|&&idx| idx > scroll_from_top)
        };

        let Some(&target_line) = target else {
            return false;
        };

        let clamped_target = target_line.min(max_scroll);
        self.scroll_offset = max_scroll.saturating_sub(clamped_target);
        self.pinned_scroll_top = None;
        true
    }

    fn user_message_line_indices(&self) -> Vec<usize> {
        let mut indices = Vec::new();
        let mut flat_index = 0usize;
        let mut last_is_blank = false;

        for (msg_idx, msg) in self.messages.iter().enumerate() {
            let Some(Some(cached)) = self.line_cache.entries.get(msg_idx) else {
                continue;
            };
            let mut first_included: Option<usize> = None;
            for line in &cached.lines {
                let is_blank = chat_view_cache::is_blank_line(line);
                if is_blank && last_is_blank {
                    continue;
                }
                if msg.role == MessageRole::User && first_included.is_none() {
                    first_included = Some(flat_index);
                }
                flat_index = flat_index.saturating_add(1);
                last_is_blank = is_blank;
            }
            if msg.role == MessageRole::User {
                if let Some(idx) = first_included {
                    indices.push(idx);
                }
            }
        }

        indices
    }

    pub fn set_scroll_from_top(&mut self, offset_from_top: usize, total: usize, visible: usize) {
        let max_scroll = total.saturating_sub(visible);
        self.scroll_offset = max_scroll.saturating_sub(offset_from_top.min(max_scroll));
        self.pinned_scroll_top = None;
    }

    fn ensure_streaming_cache(&mut self, width: u16) {
        if self.streaming_messages.is_empty() {
            self.streaming_cache = None;
            self.streaming_joiner_before = None;
            return;
        }

        if self.streaming_cache.is_none() {
            let mut streaming_lines = Vec::new();
            let mut streaming_joiners = Vec::new();
            for message in &self.streaming_messages {
                let msg = ChatMessage::streaming_with_role(message.role, message.content.clone());
                self.format_message_with_joiners(
                    &msg,
                    width as usize,
                    &mut streaming_lines,
                    &mut streaming_joiners,
                );
            }
            self.streaming_cache = Some(streaming_lines);
            self.streaming_joiner_before = Some(streaming_joiners);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection_anchor = None;
        self.selection_head = None;
        self.selection_scroll_lock = None;
    }

    pub fn has_selection(&self) -> bool {
        self.selection_anchor.is_some()
            && self.selection_head.is_some()
            && self.selection_anchor != self.selection_head
    }

    pub fn begin_selection(
        &mut self,
        click_x: u16,
        click_y: u16,
        area: Rect,
        show_scrollbar: bool,
    ) -> bool {
        let Some(point) = self.selection_point_from_mouse(click_x, click_y, area, show_scrollbar)
        else {
            return false;
        };
        self.selection_anchor = Some(point);
        self.selection_head = None;
        self.selection_scroll_lock = None;
        true
    }

    pub fn update_selection(
        &mut self,
        click_x: u16,
        click_y: u16,
        area: Rect,
        is_streaming: bool,
        show_scrollbar: bool,
    ) -> bool {
        if self.selection_anchor.is_none() {
            return false;
        }
        let Some(point) = self.selection_point_from_mouse(click_x, click_y, area, show_scrollbar)
        else {
            return false;
        };
        self.selection_head = Some(point);

        // Lock scroll position during streaming to prevent auto-scroll from
        // disrupting the active selection.
        if is_streaming && self.selection_scroll_lock.is_none() {
            let Some(content) = Self::content_area(area, show_scrollbar) else {
                return true;
            };
            let cached_len = self.flat_cache.len();
            let streaming_len = self
                .streaming_cache
                .as_ref()
                .map(|lines| lines.len())
                .unwrap_or(0);
            let total_lines = cached_len + streaming_len + self.last_render_extra_lines;
            let visible_height = content.height as usize;
            let max_scroll = total_lines.saturating_sub(visible_height);
            if self.scroll_offset == 0 && total_lines > visible_height {
                let scroll_from_top = max_scroll.saturating_sub(self.scroll_offset.min(max_scroll));
                self.selection_scroll_lock = Some(scroll_from_top);
            }
        }

        true
    }

    pub fn finalize_selection(&mut self) -> bool {
        if self.selection_head.is_none() || self.selection_anchor == self.selection_head {
            self.clear_selection();
            return false;
        }
        self.selection_scroll_lock = None;
        true
    }

    pub fn copy_selection(&mut self) -> Option<String> {
        let (start, end) = self.selection_ordered()?;
        let width = self.cache_width?;
        self.ensure_cache(width);
        self.ensure_flat_cache();
        self.ensure_streaming_cache(width);

        let streaming_len = self.streaming_cache.as_ref().map(|s| s.len()).unwrap_or(0);
        let total_len = self.flat_cache.len() + streaming_len;
        let mut lines = Vec::with_capacity(total_len);
        lines.extend(self.flat_cache.iter().cloned());
        let mut joiner_before = Vec::with_capacity(total_len);
        joiner_before.extend(self.joiner_before.iter().cloned());

        if let Some(ref streaming) = self.streaming_cache {
            lines.extend(streaming.iter().cloned());
            if let Some(ref streaming_joiners) = self.streaming_joiner_before {
                joiner_before.extend(streaming_joiners.iter().cloned());
            } else {
                #[allow(clippy::manual_repeat_n)]
                joiner_before.extend(std::iter::repeat(None).take(streaming.len()));
            }
        }

        if lines.len() != joiner_before.len() {
            return None;
        }

        selection_to_copy_text(&lines, &joiner_before, start, end, width)
    }

    fn selection_ordered(&self) -> Option<(SelectionPoint, SelectionPoint)> {
        let (anchor, head) = self.selection_anchor.zip(self.selection_head)?;
        if anchor == head {
            return None;
        }
        Some(order_points(anchor, head))
    }

    fn selection_point_from_mouse(
        &mut self,
        click_x: u16,
        click_y: u16,
        area: Rect,
        show_scrollbar: bool,
    ) -> Option<SelectionPoint> {
        let content = Self::content_area(area, show_scrollbar)?;
        if click_x < content.x
            || click_y < content.y
            || click_x >= content.x + content.width
            || click_y >= content.y + content.height
        {
            return None;
        }

        let rel_x = click_x.saturating_sub(content.x);
        let rel_y = click_y.saturating_sub(content.y) as usize;

        self.ensure_cache(content.width);
        self.ensure_flat_cache();
        self.ensure_streaming_cache(content.width);

        let cached_len = self.flat_cache.len();
        let streaming_len = self
            .streaming_cache
            .as_ref()
            .map(|lines| lines.len())
            .unwrap_or(0);
        let total_lines = cached_len + streaming_len + self.last_render_extra_lines;
        if total_lines == 0 {
            return None;
        }

        let visible_height = content.height as usize;
        let max_scroll = total_lines.saturating_sub(visible_height);
        let scroll_from_top = max_scroll.saturating_sub(self.scroll_offset.min(max_scroll));
        let top_offset = visible_height.saturating_sub(total_lines);
        if rel_y < top_offset {
            return None;
        }
        let line_index = scroll_from_top.saturating_add(rel_y.saturating_sub(top_offset));
        if line_index >= cached_len + streaming_len {
            return None;
        }

        let line = if line_index < cached_len {
            self.flat_cache.get(line_index)?
        } else {
            let idx = line_index.saturating_sub(cached_len);
            self.streaming_cache.as_ref()?.get(idx)?
        };

        let base_x = line_gutter_cols(line);
        let max_x = content.width.saturating_sub(1);
        if base_x > max_x {
            return Some(SelectionPoint {
                line_index,
                column: 0,
            });
        }
        let content_width = max_x.saturating_sub(base_x);
        let column = rel_x.saturating_sub(base_x).min(content_width);

        Some(SelectionPoint { line_index, column })
    }

    /// Find a clickable file path at the given mouse position.
    /// Returns the file path if one exists at that position and it exists on disk.
    pub fn file_path_at_position(
        &mut self,
        click_x: u16,
        click_y: u16,
        area: Rect,
        show_scrollbar: bool,
    ) -> Option<String> {
        use super::file_path_detector::{detect_existing_paths, expand_tilde};

        let content = Self::content_area(area, show_scrollbar)?;
        if click_x < content.x
            || click_y < content.y
            || click_x >= content.x + content.width
            || click_y >= content.y + content.height
        {
            return None;
        }

        let rel_x = click_x.saturating_sub(content.x) as usize;
        let rel_y = click_y.saturating_sub(content.y) as usize;

        self.ensure_cache(content.width);
        self.ensure_flat_cache();
        self.ensure_streaming_cache(content.width);

        let cached_len = self.flat_cache.len();
        let streaming_len = self
            .streaming_cache
            .as_ref()
            .map(|lines| lines.len())
            .unwrap_or(0);
        let extra_len = self.last_extra_lines.len();
        let total_lines = cached_len + streaming_len + extra_len;
        if total_lines == 0 {
            return None;
        }

        let visible_height = content.height as usize;
        let max_scroll = total_lines.saturating_sub(visible_height);
        let scroll_from_top = max_scroll.saturating_sub(self.scroll_offset.min(max_scroll));
        let line_index = scroll_from_top.saturating_add(rel_y);
        if line_index >= total_lines {
            return None;
        }

        // Get the line text - check flat_cache, streaming_cache, then extra_lines
        let line = if line_index < cached_len {
            self.flat_cache.get(line_index)?
        } else if line_index < cached_len + streaming_len {
            let idx = line_index.saturating_sub(cached_len);
            self.streaming_cache.as_ref()?.get(idx)?
        } else {
            // Check extra lines (prompts, indicators)
            let idx = line_index.saturating_sub(cached_len + streaming_len);
            self.last_extra_lines.get(idx)?
        };

        // Extract text content from the line spans
        let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

        // Detect existing file paths in the line
        let paths = detect_existing_paths(&line_text);

        // Check if click position is within any detected path
        for path_match in paths {
            // Calculate the display column for the path's start/end positions
            // We need to convert byte positions to display columns
            let prefix_text = &line_text[..path_match.start];
            let start_col = UnicodeWidthStr::width(prefix_text);
            let end_col = start_col + UnicodeWidthStr::width(path_match.path.as_str());

            // Check if click is within this path's region
            if rel_x >= start_col && rel_x < end_col {
                // Return the expanded path
                return Some(expand_tilde(&path_match.path));
            }
        }

        None
    }

    /// Update hover state for file paths at the given mouse position.
    /// Returns true if the hover state changed.
    pub fn update_file_path_hover(
        &mut self,
        mouse_x: u16,
        mouse_y: u16,
        area: Rect,
        show_scrollbar: bool,
    ) -> bool {
        use super::file_path_detector::{detect_existing_paths, expand_tilde};

        let content = match Self::content_area(area, show_scrollbar) {
            Some(c) => c,
            None => {
                let changed = self.hovered_file_path.is_some();
                self.hovered_file_path = None;
                return changed;
            }
        };

        // Check if mouse is within content area
        if mouse_x < content.x
            || mouse_y < content.y
            || mouse_x >= content.x + content.width
            || mouse_y >= content.y + content.height
        {
            let changed = self.hovered_file_path.is_some();
            self.hovered_file_path = None;
            return changed;
        }

        let rel_x = mouse_x.saturating_sub(content.x) as usize;
        let rel_y = mouse_y.saturating_sub(content.y) as usize;

        self.ensure_cache(content.width);
        self.ensure_flat_cache();
        self.ensure_streaming_cache(content.width);

        let cached_len = self.flat_cache.len();
        let streaming_len = self
            .streaming_cache
            .as_ref()
            .map(|lines| lines.len())
            .unwrap_or(0);
        let extra_len = self.last_extra_lines.len();
        let total_lines = cached_len + streaming_len + extra_len;
        if total_lines == 0 {
            let changed = self.hovered_file_path.is_some();
            self.hovered_file_path = None;
            return changed;
        }

        let visible_height = content.height as usize;
        let max_scroll = total_lines.saturating_sub(visible_height);
        let scroll_from_top = max_scroll.saturating_sub(self.scroll_offset.min(max_scroll));
        let line_index = scroll_from_top.saturating_add(rel_y);
        if line_index >= total_lines {
            let changed = self.hovered_file_path.is_some();
            self.hovered_file_path = None;
            return changed;
        }

        // Get the line text - check flat_cache, streaming_cache, then extra_lines
        let line = if line_index < cached_len {
            match self.flat_cache.get(line_index) {
                Some(l) => l,
                None => {
                    let changed = self.hovered_file_path.is_some();
                    self.hovered_file_path = None;
                    return changed;
                }
            }
        } else if line_index < cached_len + streaming_len {
            let idx = line_index.saturating_sub(cached_len);
            match self.streaming_cache.as_ref().and_then(|c| c.get(idx)) {
                Some(l) => l,
                None => {
                    let changed = self.hovered_file_path.is_some();
                    self.hovered_file_path = None;
                    return changed;
                }
            }
        } else {
            // Check extra lines (prompts, indicators)
            let idx = line_index.saturating_sub(cached_len + streaming_len);
            match self.last_extra_lines.get(idx) {
                Some(l) => l,
                None => {
                    let changed = self.hovered_file_path.is_some();
                    self.hovered_file_path = None;
                    return changed;
                }
            }
        };

        // Extract text content from the line spans
        let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();

        // Detect existing file paths in the line
        let paths = detect_existing_paths(&line_text);

        // Check if mouse position is within any detected path
        for path_match in paths {
            let prefix_text = &line_text[..path_match.start];
            let start_col = UnicodeWidthStr::width(prefix_text);
            let end_col = start_col + UnicodeWidthStr::width(path_match.path.as_str());

            if rel_x >= start_col && rel_x < end_col {
                let new_hover = HoveredFilePath {
                    line_index,
                    start_col,
                    end_col,
                    path: expand_tilde(&path_match.path),
                };
                let changed = self.hovered_file_path.as_ref() != Some(&new_hover);
                self.hovered_file_path = Some(new_hover);
                return changed;
            }
        }

        // No path under cursor
        let changed = self.hovered_file_path.is_some();
        self.hovered_file_path = None;
        changed
    }

    /// Clear the file path hover state
    pub fn clear_file_path_hover(&mut self) {
        self.hovered_file_path = None;
    }

    /// Check if there's a file path being hovered
    pub fn is_file_path_hovered(&self) -> bool {
        self.hovered_file_path.is_some()
    }

    /// Get the currently hovered file path
    pub fn hovered_file_path(&self) -> Option<&HoveredFilePath> {
        self.hovered_file_path.as_ref()
    }

    fn apply_selection_highlight(
        &self,
        visible_lines: Vec<(Line<'static>, Option<usize>)>,
        width: u16,
    ) -> Vec<Line<'static>> {
        let selection = self.selection_ordered();

        let highlighted_flat_range =
            self.highlighted_code_block
                .and_then(|(entry_idx, block_within)| {
                    self.flat_code_block_spans
                        .iter()
                        .find(|&&(ei, bi, _, _)| ei == entry_idx && bi == block_within)
                        .map(|&(_, _, flat_start, flat_end)| (flat_start, flat_end))
                });

        let mut out = Vec::with_capacity(visible_lines.len());
        for (line, line_index) in visible_lines {
            let mut result_line = line.clone();

            // Apply copied code block background highlight
            if let (Some((hl_start, hl_end)), Some(idx)) = (highlighted_flat_range, line_index) {
                if idx >= hl_start && idx < hl_end {
                    result_line = highlight_code_block_line(&result_line);
                }
            }

            // Apply selection highlight if applicable
            if let (Some((start, end)), Some(idx)) = (selection, line_index) {
                if idx >= start.line_index && idx <= end.line_index {
                    if let Some((start_col, end_col)) =
                        self.selection_bounds_for_line(idx, &result_line, start, end, width)
                    {
                        result_line = highlight_line_by_cols(&result_line, start_col, end_col);
                    }
                }
            }

            // Apply hover underline if this line contains the hovered file path
            if let (Some(hover), Some(idx)) = (&self.hovered_file_path, line_index) {
                if idx == hover.line_index {
                    result_line = underline_line_by_cols(
                        &result_line,
                        hover.start_col as u16,
                        hover.end_col as u16,
                    );
                }
            }

            out.push(result_line);
        }
        out
    }

    fn selection_bounds_for_line(
        &self,
        line_index: usize,
        line: &Line<'static>,
        start: SelectionPoint,
        end: SelectionPoint,
        width: u16,
    ) -> Option<(u16, u16)> {
        let base_x = line_gutter_cols(line);
        let max_x = width.saturating_sub(1);
        if base_x > max_x {
            return None;
        }
        let content_width = max_x.saturating_sub(base_x);

        let line_start_col = if line_index == start.line_index {
            start.column
        } else {
            0
        };
        let line_end_col = if line_index == end.line_index {
            end.column
        } else {
            content_width
        };

        let abs_start = base_x.saturating_add(line_start_col.min(content_width));
        let abs_end = base_x.saturating_add(line_end_col.min(content_width));
        if abs_start > abs_end {
            return None;
        }
        Some((abs_start, abs_end))
    }

    pub fn scrollbar_metrics(
        &mut self,
        area: Rect,
        show_thinking_line: bool,
        queue_lines_len: usize,
        show_scrollbar: bool,
    ) -> Option<ScrollbarMetrics> {
        if !show_scrollbar {
            return None;
        }

        let content = Self::content_area(area, true)?;

        self.ensure_cache(content.width);
        self.ensure_flat_cache();
        self.ensure_streaming_cache(content.width);

        let cached_len = self.flat_cache.len();
        let streaming_len = self
            .streaming_cache
            .as_ref()
            .map(|lines| lines.len())
            .unwrap_or(0);
        let mut extra_len = 0;
        if show_thinking_line {
            extra_len += 1;
        }
        if queue_lines_len > 0 {
            extra_len += queue_lines_len;
        }
        if extra_len > 0 {
            extra_len += 1; // spacing line
        }

        let total_lines = cached_len + streaming_len + extra_len;
        let visible_height = content.height as usize;
        if total_lines <= visible_height {
            return None;
        }

        Some(ScrollbarMetrics {
            area: Rect {
                x: area.x + area.width.saturating_sub(1),
                y: area.y,
                width: 1,
                height: area.height,
            },
            total: total_lines,
            visible: visible_height,
        })
    }

    /// Get message count
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty() && self.streaming_messages.is_empty()
    }

    /// Get all messages (for debug dump)
    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// Get streaming message content for a specific role (for debug dump)
    pub fn streaming_message_for(&self, role: MessageRole) -> Option<&str> {
        self.streaming_messages
            .iter()
            .rposition(|message| message.role == role)
            .and_then(|idx| self.streaming_messages.get(idx))
            .map(|message| message.content.as_str())
    }

    /// Get assistant streaming buffer (for debug dump)
    pub fn streaming_buffer(&self) -> Option<&str> {
        self.streaming_message_for(MessageRole::Assistant)
    }

    /// Toggle collapsed state for a tool message at the given index
    pub fn toggle_tool_at(&mut self, index: usize) {
        if let Some(msg) = self.messages.get_mut(index) {
            if msg.role == MessageRole::Tool {
                msg.is_collapsed = !msg.is_collapsed;
                // Invalidate and update cache for this message
                if let Some(width) = self.cache_width {
                    self.invalidate_cache_entry(index);
                    self.update_cache_entry(index, width);
                }
            }
        }
    }

    /// Collapse all tool messages
    pub fn collapse_all_tools(&mut self) {
        let mut changed_indices = Vec::new();
        for (i, msg) in self.messages.iter_mut().enumerate() {
            if msg.role == MessageRole::Tool && !msg.is_collapsed {
                msg.is_collapsed = true;
                changed_indices.push(i);
            }
        }
        // Update cache for changed messages
        if let Some(width) = self.cache_width {
            for idx in changed_indices {
                self.invalidate_cache_entry(idx);
                self.update_cache_entry(idx, width);
            }
        }
    }

    /// Expand all tool messages
    pub fn expand_all_tools(&mut self) {
        let mut changed_indices = Vec::new();
        for (i, msg) in self.messages.iter_mut().enumerate() {
            if msg.role == MessageRole::Tool && msg.is_collapsed {
                msg.is_collapsed = false;
                changed_indices.push(i);
            }
        }
        // Update cache for changed messages
        if let Some(width) = self.cache_width {
            for idx in changed_indices {
                self.invalidate_cache_entry(idx);
                self.update_cache_entry(idx, width);
            }
        }
    }

    /// Get indices of all tool messages
    pub fn tool_message_indices(&self) -> Vec<usize> {
        self.messages
            .iter()
            .enumerate()
            .filter_map(|(i, msg)| {
                if msg.role == MessageRole::Tool {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    fn format_message_with_joiners(
        &self,
        msg: &ChatMessage,
        width: usize,
        lines: &mut Vec<Line<'static>>,
        joiner_before: &mut Vec<Option<String>>,
    ) -> (Vec<String>, Vec<(usize, usize)>) {
        match msg.role {
            MessageRole::Tool => {
                self.format_tool_message(msg, width, lines, joiner_before);
                (Vec::new(), Vec::new())
            }
            MessageRole::User => {
                self.format_user_message(msg, width, lines, joiner_before);
                (Vec::new(), Vec::new())
            }
            MessageRole::Assistant => {
                self.format_assistant_message(msg, width, lines, joiner_before)
            }
            MessageRole::Reasoning => {
                self.format_reasoning_message(msg, width, lines, joiner_before);
                (Vec::new(), Vec::new())
            }
            MessageRole::System => {
                self.format_system_message(msg, width, lines, joiner_before);
                (Vec::new(), Vec::new())
            }
            MessageRole::Error => {
                self.format_error_message(msg, width, lines, joiner_before);
                (Vec::new(), Vec::new())
            }
            MessageRole::Summary => {
                self.format_summary_message(msg, width, lines, joiner_before);
                (Vec::new(), Vec::new())
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn format_wrapped_lines(
        &self,
        lines: &mut Vec<Line<'static>>,
        joiner_before: &mut Vec<Option<String>>,
        content_spans: Vec<Span<'static>>,
        first_prefix: Vec<Span<'static>>,
        cont_prefix: Vec<Span<'static>>,
        first_prefix_width: usize,
        cont_prefix_width: usize,
        width: usize,
    ) {
        let content_width = width
            .saturating_sub(first_prefix_width.max(cont_prefix_width))
            .max(1);
        let (wrapped, wrapped_joiners) = wrap_spans_with_joiners(content_spans, content_width);
        for (idx, (wrapped_spans, joiner)) in wrapped.into_iter().zip(wrapped_joiners).enumerate() {
            let prefix = if idx == 0 {
                first_prefix.clone()
            } else {
                cont_prefix.clone()
            };
            let mut line_spans = prefix;
            line_spans.extend(wrapped_spans);
            lines.push(Line::from(line_spans));
            joiner_before.push(joiner);
        }
    }

    /// Format user messages with accent stripe and padding
    fn format_user_message(
        &self,
        msg: &ChatMessage,
        width: usize,
        lines: &mut Vec<Line<'static>>,
        joiner_before: &mut Vec<Option<String>>,
    ) {
        if msg.content.is_empty() {
            return;
        }

        let builder = UserMessageBlockBuilder::new(width);
        let text_style = Style::default().fg(Color::White).bg(tool_block_bg());

        // Top padding
        lines.push(builder.empty_line());
        joiner_before.push(None);

        // Role label
        let label_spans = vec![Span::styled(
            "You".to_string(),
            Style::default().fg(text_muted()).bg(tool_block_bg()),
        )];
        lines.push(builder.line(label_spans));
        joiner_before.push(None);

        for line in msg.content.lines() {
            let content_spans = vec![Span::styled(line.to_string(), text_style)];
            let (wrapped, wrapped_joiners) =
                wrap_spans_with_joiners(content_spans, builder.content_width());

            for (wrapped_spans, joiner) in wrapped.into_iter().zip(wrapped_joiners) {
                lines.push(builder.line(wrapped_spans));
                joiner_before.push(joiner);
            }
        }

        // Bottom padding
        lines.push(builder.empty_line());
        joiner_before.push(None);
    }

    /// Format assistant messages - flowing text with markdown
    fn format_assistant_message(
        &self,
        msg: &ChatMessage,
        width: usize,
        lines: &mut Vec<Line<'static>>,
        joiner_before: &mut Vec<Option<String>>,
    ) -> (Vec<String>, Vec<(usize, usize)>) {
        if msg.content.is_empty() {
            return (Vec::new(), Vec::new());
        }

        // Vertical breathing room
        lines.push(Line::from(""));
        joiner_before.push(None);

        // Agent role label
        lines.push(Line::from(Span::styled(
            format!("  {}", self.agent_label),
            Style::default().fg(text_muted()),
        )));
        joiner_before.push(None);

        // Parse markdown with custom renderer
        let mut renderer = MarkdownRenderer::new();
        let md_text = renderer.render(&msg.content);
        let code_blocks = std::mem::take(&mut renderer.code_blocks);
        let md_code_block_ranges = std::mem::take(&mut renderer.code_block_line_ranges);

        let bullet_prefix = vec![Span::raw("• ")];
        let continuation_prefix = vec![Span::raw("  ")];
        let bullet_width = UnicodeWidthStr::width("• ");
        let continuation_width = UnicodeWidthStr::width("  ");

        let mut first_content_line = true;
        let mut md_line_output_starts: Vec<usize> = Vec::with_capacity(md_text.lines.len());
        let mut md_line_output_ends: Vec<usize> = Vec::with_capacity(md_text.lines.len());

        for line in md_text.lines {
            md_line_output_starts.push(lines.len());

            if line.spans.is_empty() {
                lines.push(Line::from(""));
                joiner_before.push(None);
            } else {
                let content_spans: Vec<Span<'static>> = line
                    .spans
                    .into_iter()
                    .map(|s| {
                        // Apply a slightly dimmer style for assistant text
                        let mut style = s.style;
                        if style.fg.is_none() {
                            style = style.fg(Color::Rgb(220, 220, 220)); // Slightly dimmer white
                        }
                        Span::styled(s.content.into_owned(), style)
                    })
                    .collect();

                let line_text: String = content_spans.iter().map(|s| s.content.as_ref()).collect();
                let trimmed = line_text.trim_start();
                let is_list_item = trimmed.starts_with("• ")
                    || trimmed.starts_with("- ")
                    || trimmed
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
                        && trimmed.get(1..2) == Some(".")
                        && trimmed.get(2..3) == Some(" ");

                let (first_prefix, first_prefix_width) = if first_content_line && !is_list_item {
                    (bullet_prefix.clone(), bullet_width)
                } else {
                    (continuation_prefix.clone(), continuation_width)
                };

                self.format_wrapped_lines(
                    lines,
                    joiner_before,
                    content_spans,
                    first_prefix,
                    continuation_prefix.clone(),
                    first_prefix_width,
                    continuation_width,
                    width,
                );

                if first_content_line {
                    first_content_line = false;
                }
            }

            md_line_output_ends.push(lines.len());
        }

        // Add streaming indicator if still streaming
        if msg.is_streaming {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "…",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
            ]));
            joiner_before.push(None);
        }

        // Map markdown-relative code block ranges to output line indices
        let code_block_line_ranges: Vec<(usize, usize)> = md_code_block_ranges
            .iter()
            .map(|&(md_start, md_end)| {
                let out_start = md_line_output_starts.get(md_start).copied().unwrap_or(0);
                let out_end = if md_end > 0 {
                    md_line_output_ends
                        .get(md_end - 1)
                        .copied()
                        .unwrap_or(lines.len())
                } else {
                    out_start
                };
                (out_start, out_end)
            })
            .collect();

        (code_blocks, code_block_line_ranges)
    }

    /// Format reasoning messages with subdued styling
    fn format_reasoning_message(
        &self,
        msg: &ChatMessage,
        width: usize,
        lines: &mut Vec<Line<'static>>,
        joiner_before: &mut Vec<Option<String>>,
    ) {
        if msg.content.is_empty() {
            return;
        }

        let content_lines: Vec<&str> = msg.content.lines().collect();
        let text_style = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC);
        let prefix_first = vec![Span::styled("... ", text_style)];
        let prefix_next = vec![Span::raw("    ")];
        let prefix_first_width = UnicodeWidthStr::width("... ");
        let prefix_next_width = UnicodeWidthStr::width("    ");

        for (i, line) in content_lines.iter().enumerate() {
            let content_spans = vec![Span::styled(line.to_string(), text_style)];
            let (first_prefix, first_prefix_width) = if i == 0 {
                (prefix_first.clone(), prefix_first_width)
            } else {
                (prefix_next.clone(), prefix_next_width)
            };
            self.format_wrapped_lines(
                lines,
                joiner_before,
                content_spans,
                first_prefix,
                prefix_next.clone(),
                first_prefix_width,
                prefix_next_width,
                width,
            );
        }

        if msg.is_streaming {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    "...",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
            ]));
            joiner_before.push(None);
        }
    }

    /// Format system messages with info symbol
    fn format_system_message(
        &self,
        msg: &ChatMessage,
        width: usize,
        lines: &mut Vec<Line<'static>>,
        joiner_before: &mut Vec<Option<String>>,
    ) {
        let content_lines: Vec<&str> = msg.content.lines().collect();
        let prefix_first = vec![Span::styled("ℹ ", Style::default().fg(Color::Blue))];
        let prefix_next = vec![Span::raw("  ")];
        let prefix_first_width = UnicodeWidthStr::width("ℹ ");
        let prefix_next_width = UnicodeWidthStr::width("  ");
        let text_style = Style::default().fg(Color::Blue);

        for (i, line) in content_lines.iter().enumerate() {
            let content_spans = vec![Span::styled(line.to_string(), text_style)];
            let (first_prefix, first_prefix_width) = if i == 0 {
                (prefix_first.clone(), prefix_first_width)
            } else {
                (prefix_next.clone(), prefix_next_width)
            };
            self.format_wrapped_lines(
                lines,
                joiner_before,
                content_spans,
                first_prefix,
                prefix_next.clone(),
                first_prefix_width,
                prefix_next_width,
                width,
            );
        }
    }

    /// Format error messages with X symbol
    fn format_error_message(
        &self,
        msg: &ChatMessage,
        width: usize,
        lines: &mut Vec<Line<'static>>,
        joiner_before: &mut Vec<Option<String>>,
    ) {
        let content_lines: Vec<&str> = msg.content.lines().collect();
        let prefix_first = vec![Span::styled("✗ ", Style::default().fg(Color::Red))];
        let prefix_next = vec![Span::raw("  ")];
        let prefix_first_width = UnicodeWidthStr::width("✗ ");
        let prefix_next_width = UnicodeWidthStr::width("  ");
        let text_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);

        for (i, line) in content_lines.iter().enumerate() {
            let content_spans = vec![Span::styled(line.to_string(), text_style)];
            let (first_prefix, first_prefix_width) = if i == 0 {
                (prefix_first.clone(), prefix_first_width)
            } else {
                (prefix_next.clone(), prefix_next_width)
            };
            self.format_wrapped_lines(
                lines,
                joiner_before,
                content_spans,
                first_prefix,
                prefix_next.clone(),
                first_prefix_width,
                prefix_next_width,
                width,
            );
        }
    }

    /// Format tool messages using Opencode-style blocks
    fn format_tool_message(
        &self,
        msg: &ChatMessage,
        width: usize,
        lines: &mut Vec<Line<'static>>,
        joiner_before: &mut Vec<Option<String>>,
    ) {
        let tool_name = msg.tool_name.as_deref().unwrap_or("Tool");

        // Special formatting for TodoWrite
        if tool_name == "TodoWrite" {
            return self.format_todowrite(msg, width, lines, joiner_before);
        }

        let tool_args = msg.tool_args.as_deref().unwrap_or("");
        let content_lines: Vec<&str> = msg.content.lines().collect();
        let line_count = content_lines.len();

        // Check if this is an image file read
        let is_image = if tool_name == "Read" {
            Self::is_image_file(tool_args)
        } else {
            false
        };

        // Determine if error
        let is_error = if let Some(code) = msg.exit_code {
            code != 0
        } else {
            msg.content.starts_with("Error:")
        };

        let builder = ToolBlockBuilder::new(width);

        // === Top padding ===
        lines.push(builder.empty_line());
        joiner_before.push(None);

        // === Description line (# comment) ===
        let description = self.get_tool_description(tool_name, tool_args);
        lines.push(builder.comment(&description));
        joiner_before.push(None);

        // === Blank line after description ===
        lines.push(builder.empty_line());
        joiner_before.push(None);

        // === Command line ($ command) ===
        let command_display = self.get_tool_command(tool_name, tool_args, width.saturating_sub(6));
        lines.push(builder.command(&command_display));
        joiner_before.push(None);

        // === Blank line after title section ===
        lines.push(builder.empty_line());
        joiner_before.push(None);

        // === Output content ===
        let line_word = if line_count == 1 { "line" } else { "lines" };
        if msg.is_collapsed {
            // Collapsed: show summary
            let summary = if line_count > 0 {
                format!("▶ {} {} (click to expand)", line_count, line_word)
            } else {
                "▶ No output".to_string()
            };
            lines.push(builder.output(&summary));
            joiner_before.push(None);
        } else {
            // Expanded: show output lines
            let max_display_lines = 50;
            let truncated = line_count > max_display_lines;
            let display_lines = if truncated {
                &content_lines[..max_display_lines]
            } else {
                &content_lines[..]
            };

            if tool_name == "Read" && !is_image {
                // Syntax-highlight file content.
                // The Read tool prefixes each line with cat-n style numbers ("   N\t");
                // strip them before passing to the highlighter.
                let stripped: Vec<String> = display_lines
                    .iter()
                    .map(|l| strip_line_number_prefix(l).to_string())
                    .collect();

                let file_path = extract_file_path_from_args(tool_args);
                let path_ref = file_path.as_deref().unwrap_or("");
                let highlighted = highlight_file_for_tool(Path::new(path_ref), &stripped);

                for hl_line in highlighted {
                    let spans: Vec<Span<'static>> = if hl_line.spans.is_empty() {
                        vec![Span::styled("", builder.bg_style())]
                    } else {
                        hl_line
                            .spans
                            .into_iter()
                            .map(|s| Span::styled(s.content, s.style.bg(tool_block_bg())))
                            .collect()
                    };
                    for wrapped_line in builder.wrapped_custom(spans) {
                        lines.push(wrapped_line);
                        joiner_before.push(None);
                    }
                }
            } else {
                for line in display_lines {
                    let normalized = normalize_tool_output_line(line);
                    let sanitized = sanitize_tool_output_line(normalized.as_ref());
                    let display_line = sanitized.as_ref();
                    // Check for diff-style lines
                    let (line_color, line_text) = if display_line.starts_with('+')
                        && !display_line.starts_with("+++")
                    {
                        (diff_add(), sanitized.to_string())
                    } else if display_line.starts_with('-') && !display_line.starts_with("---") {
                        (diff_remove(), sanitized.to_string())
                    } else if display_line.starts_with("Error:") || display_line.contains("error:")
                    {
                        (accent_error(), sanitized.to_string())
                    } else {
                        // Parse ANSI escape codes
                        let parsed = sanitized.as_ref().as_bytes().into_text();
                        match parsed {
                            Ok(text) => {
                                // If ANSI parsed successfully, add spans with background
                                let mut spans: Vec<Span<'static>> = text
                                    .lines
                                    .into_iter()
                                    .flat_map(|l| l.spans)
                                    .map(|s| {
                                        Span::styled(
                                            s.content.into_owned(),
                                            s.style.bg(tool_block_bg()),
                                        )
                                    })
                                    .collect();
                                if spans.is_empty() {
                                    spans.push(Span::styled("", builder.bg_style()));
                                }
                                // Wrap ANSI-parsed lines
                                for line in builder.wrapped_custom(spans) {
                                    lines.push(line);
                                    joiner_before.push(None);
                                }
                                continue;
                            }
                            Err(_) => (tool_output(), sanitized.to_string()),
                        }
                    };

                    // Wrap long lines
                    for line in builder.wrapped_output_colored(&line_text, line_color) {
                        lines.push(line);
                        joiner_before.push(None);
                    }
                }
            }

            // Truncation notice
            if truncated {
                let remaining = line_count - max_display_lines;
                let more_word = if remaining == 1 { "line" } else { "lines" };
                lines.push(builder.output_colored(
                    &format!("... ({} more {})", remaining, more_word),
                    tool_comment(),
                ));
                joiner_before.push(None);
            }
        }

        // === Blank line before status ===
        lines.push(builder.empty_line());
        joiner_before.push(None);

        // === Status line ===
        let status_text = if is_error {
            if let Some(code) = msg.exit_code {
                format!("✗ Failed (exit: {})", code)
            } else {
                "✗ Failed".to_string()
            }
        } else if let Some(code) = msg.exit_code {
            format!("✓ Completed (exit: {})", code)
        } else if is_image {
            // For images, show file size instead of line count
            // Use cached file_size if available, otherwise try fs lookup
            let size_str = if let Some(size) = msg.file_size {
                Self::format_file_size(size)
            } else {
                Self::get_file_size_from_args(tool_args)
            };
            format!("✓ Read image ({})", size_str)
        } else {
            format!("✓ {} {}", line_count, line_word)
        };

        let status_color = if is_error {
            accent_error()
        } else {
            accent_success()
        };
        lines.push(builder.output_colored(&status_text, status_color));
        joiner_before.push(None);

        // === Bottom padding ===
        lines.push(builder.empty_line());
        joiner_before.push(None);
    }

    /// Get a human-readable description for a tool invocation
    fn get_tool_description(&self, tool_name: &str, tool_args: &str) -> String {
        // Try to extract description from JSON args if present
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(tool_args) {
            if let Some(desc) = json.get("description").and_then(|d| d.as_str()) {
                return desc.to_string();
            }
        }

        // Default descriptions by tool type
        match tool_name {
            "Bash" => "Run command".to_string(),
            "Read" => "Read file".to_string(),
            "Write" => "Write file".to_string(),
            "Edit" => "Edit file".to_string(),
            "Glob" => "Find files".to_string(),
            "Grep" => "Search for pattern".to_string(),
            "LS" => "List directory".to_string(),
            "Task" => "Run agent".to_string(),
            _ => tool_name.to_string(),
        }
    }

    /// Get the command/path to display for a tool invocation
    fn get_tool_command(&self, tool_name: &str, tool_args: &str, max_width: usize) -> String {
        // Try to parse as JSON first
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(tool_args) {
            let command = match tool_name {
                "Bash" | "exec_command" | "shell" | "local_shell_call" | "command_execution" => {
                    json.get("command")
                        .or_else(|| json.get("cmd"))
                        .and_then(|c| c.as_str())
                        .map(String::from)
                }
                "Read" | "read_file" => {
                    let path = json
                        .get("file_path")
                        .or_else(|| json.get("filePath"))
                        .or_else(|| json.get("path"))
                        .or_else(|| json.get("file"))
                        .and_then(|p| p.as_str())
                        .unwrap_or("");
                    let offset = json.get("offset").and_then(|o| o.as_i64());
                    let limit = json.get("limit").and_then(|l| l.as_i64());
                    if let (Some(off), Some(lim)) = (offset, limit) {
                        // Display as 1-indexed line numbers (Read tool uses 0-indexed offset internally)
                        Some(format!("{} (lines {}-{})", path, off + 1, off + lim))
                    } else {
                        Some(path.to_string())
                    }
                }
                "Write" | "write_file" | "Edit" => json
                    .get("file_path")
                    .or_else(|| json.get("filePath"))
                    .or_else(|| json.get("path"))
                    .or_else(|| json.get("file"))
                    .and_then(|p| p.as_str())
                    .map(String::from),
                "Glob" => {
                    let pattern = json.get("pattern").and_then(|p| p.as_str()).unwrap_or("");
                    let path = json.get("path").and_then(|p| p.as_str());
                    if let Some(p) = path {
                        Some(format!("{} in {}", pattern, p))
                    } else {
                        Some(pattern.to_string())
                    }
                }
                "Grep" => {
                    let pattern = json.get("pattern").and_then(|p| p.as_str()).unwrap_or("");
                    let path = json.get("path").and_then(|p| p.as_str()).unwrap_or(".");
                    Some(format!("\"{}\" in {}", pattern, path))
                }
                "Task" => {
                    let prompt = json.get("prompt").and_then(|p| p.as_str()).unwrap_or("");
                    let agent_type = json
                        .get("subagent_type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("agent");
                    Some(format!(
                        "[{}] {}",
                        agent_type,
                        truncate_to_width(prompt, max_width.saturating_sub(15))
                    ))
                }
                _ => None,
            };

            if let Some(cmd) = command {
                return truncate_to_width(&cmd, max_width);
            }
        }

        // Fallback: use raw args (truncated), but skip unhelpful empty JSON
        let fallback = truncate_to_width(tool_args, max_width);
        if matches!(fallback.trim(), "{}" | "null" | "") {
            String::new()
        } else {
            fallback
        }
    }

    /// Check if tool_args refers to an image file
    fn is_image_file(tool_args: &str) -> bool {
        let image_extensions = [
            ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp", ".svg", ".ico", ".tiff", ".tif",
        ];

        // Try to extract file_path from JSON args
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(tool_args) {
            if let Some(path) = json.get("file_path").and_then(|p| p.as_str()) {
                let path_lower = path.to_lowercase();
                return image_extensions.iter().any(|ext| path_lower.ends_with(ext));
            }
        }

        // Fallback: check if raw args look like an image path
        let args_lower = tool_args.to_lowercase();
        image_extensions.iter().any(|ext| args_lower.contains(ext))
    }

    /// Get file size from tool_args for display (returns formatted string)
    fn get_file_size_from_args(tool_args: &str) -> String {
        if let Some(size) = Self::get_file_size_from_args_as_u64(tool_args) {
            Self::format_file_size(size)
        } else {
            "unknown size".to_string()
        }
    }

    /// Get file size from tool_args as u64 (returns None if file doesn't exist)
    fn get_file_size_from_args_as_u64(tool_args: &str) -> Option<u64> {
        // Try to extract file_path and get its size
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(tool_args) {
            if let Some(path) = json.get("file_path").and_then(|p| p.as_str()) {
                if let Ok(metadata) = std::fs::metadata(path) {
                    return Some(metadata.len());
                }
            }
        }
        None
    }

    /// Format file size in human-readable form
    fn format_file_size(size: u64) -> String {
        if size < 1024 {
            format!("{}B", size)
        } else if size < 1024 * 1024 {
            format!("{:.1}KB", size as f64 / 1024.0)
        } else if size < 1024 * 1024 * 1024 {
            format!("{:.1}MB", size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.1}GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }

    /// Format TodoWrite tool message
    fn format_todowrite(
        &self,
        msg: &ChatMessage,
        width: usize,
        lines: &mut Vec<Line<'static>>,
        joiner_before: &mut Vec<Option<String>>,
    ) {
        let tool_args = msg.tool_args.as_deref().unwrap_or("{}");

        // Parse todos
        let todos: Vec<(String, String)> =
            match serde_json::from_str::<serde_json::Value>(tool_args) {
                Ok(json) => {
                    if let Some(todos_array) = json.get("todos").and_then(|t| t.as_array()) {
                        todos_array
                            .iter()
                            .filter_map(|todo| {
                                let content = todo.get("content").and_then(|c| c.as_str())?;
                                let status = todo
                                    .get("status")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("pending");
                                Some((content.to_string(), status.to_string()))
                            })
                            .collect()
                    } else {
                        Vec::new()
                    }
                }
                Err(_) => Vec::new(),
            };

        let total = todos.len();
        let completed = todos.iter().filter(|(_, s)| s == "completed").count();
        let in_progress = todos.iter().filter(|(_, s)| s == "in_progress").count();

        let builder = ToolBlockBuilder::new(width);

        // Top padding
        lines.push(builder.empty_line());
        joiner_before.push(None);

        // Description
        lines.push(builder.comment("Update todo list"));
        joiner_before.push(None);

        // Blank line
        lines.push(builder.empty_line());
        joiner_before.push(None);

        if msg.is_collapsed {
            // Collapsed view
            let summary = format!(
                "▶ {} tasks: {} completed, {} in progress, {} pending",
                total,
                completed,
                in_progress,
                total.saturating_sub(completed).saturating_sub(in_progress)
            );
            lines.push(builder.output(&summary));
            joiner_before.push(None);
        } else {
            // Expanded view - show todo items
            let max_display = 15;
            let display_todos = if todos.len() > max_display {
                &todos[..max_display]
            } else {
                &todos[..]
            };

            for (content, status) in display_todos {
                let (icon, text_color) = match status.as_str() {
                    "completed" => ("✅", tool_comment()),
                    "in_progress" => ("🔄", tool_command()),
                    _ => ("⬜", tool_output()),
                };

                let display_content = truncate_to_width(content, 70);
                lines.push(builder.custom(vec![
                    Span::styled(format!("{} ", icon), builder.bg_style()),
                    Span::styled(
                        display_content,
                        Style::default().fg(text_color).bg(tool_block_bg()),
                    ),
                ]));
                joiner_before.push(None);
            }

            if todos.len() > max_display {
                let remaining = todos.len() - max_display;
                lines.push(
                    builder.output_colored(&format!("... (+{} more)", remaining), tool_comment()),
                );
                joiner_before.push(None);
            }
        }

        // Status line
        let status_text = format!("{}/{} completed", completed, total);
        let status_color = if completed == total && total > 0 {
            accent_success()
        } else if in_progress > 0 {
            Color::Yellow
        } else {
            // Pending items - use neutral muted color (not success green)
            tool_comment()
        };

        lines.push(builder.empty_line());
        joiner_before.push(None);
        lines.push(builder.output_colored(&status_text, status_color));
        joiner_before.push(None);

        // Bottom padding
        lines.push(builder.empty_line());
        joiner_before.push(None);
    }

    /// Format turn summary message
    fn format_summary_message(
        &self,
        msg: &ChatMessage,
        width: usize,
        lines: &mut Vec<Line<'static>>,
        joiner_before: &mut Vec<Option<String>>,
    ) {
        if let Some(ref summary) = msg.summary {
            lines.push(Line::from(Span::raw("")));
            joiner_before.push(None);
            lines.push(self.render_summary_divider(summary, width));
            joiner_before.push(None);
            lines.push(Line::from(Span::raw("")));
            joiner_before.push(None);
        }
    }

    fn render_summary_divider(&self, summary: &TurnSummary, width: usize) -> Line<'static> {
        let duration = summary.format_duration();
        let input_tokens = TurnSummary::format_tokens(summary.input_tokens);
        let output_tokens = TurnSummary::format_tokens(summary.output_tokens);
        let mut text = format!("─ ⏱ {duration} │ ↓{input_tokens} ↑{output_tokens} ");
        let target_width = width.max(1);
        let current_width = UnicodeWidthStr::width(text.as_str());
        if current_width < target_width {
            text.push_str(&"─".repeat(target_width - current_width));
        } else if current_width > target_width {
            // Use display-width-aware truncation for proper UTF-8/wide char handling
            text = truncate_to_width_exact(&text, target_width);
        }
        Line::from(Span::styled(text, Style::default().fg(Color::DarkGray)))
    }

    /// Render the chat view
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.render_with_indicator(area, buf, None, None, None, false);
    }

    /// Render the chat view with optional indicators and prompt lines.
    ///
    /// # Arguments
    /// * `thinking_line` - Optional thinking indicator line
    /// * `queue_lines` - Optional queued message preview lines
    /// * `prompt_lines` - Optional inline prompt lines (AskUserQuestion, ExitPlanMode)
    pub fn render_with_indicator(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        thinking_line: Option<Line<'static>>,
        queue_lines: Option<Vec<Line<'static>>>,
        prompt_lines: Option<Vec<Line<'static>>>,
        show_scrollbar: bool,
    ) {
        let Some(content) = Self::content_area(area, show_scrollbar) else {
            return;
        };

        self.invalidate_theme_cache_if_needed();

        // Ensure cache is valid for current width
        self.ensure_cache(content.width);
        self.ensure_flat_cache();

        self.ensure_streaming_cache(content.width);

        let cached_len = self.flat_cache.len();
        let streaming_len = self
            .streaming_cache
            .as_ref()
            .map(|lines| lines.len())
            .unwrap_or(0);

        let mut extra_lines = Vec::new();
        if let Some(indicator) = thinking_line {
            extra_lines.push(indicator);
        }
        if let Some(mut queue) = queue_lines {
            extra_lines.append(&mut queue);
        }
        // Add inline prompt lines (AskUserQuestion, ExitPlanMode)
        // No extra spacing needed before - prompt has its own separator line
        if let Some(mut prompt) = prompt_lines {
            extra_lines.append(&mut prompt);
            // Add trailing empty line for better separation from footer
            extra_lines.push(Line::from(""));
        } else if !extra_lines.is_empty() {
            // Only add trailing empty line when NOT showing inline prompt
            extra_lines.push(Line::from(""));
        }

        let extra_len = extra_lines.len();
        self.last_render_extra_lines = extra_len;
        // Store extra lines for file path hover detection in prompts
        self.last_extra_lines = extra_lines.clone();
        self.last_extra_lines_start = cached_len + streaming_len;
        let visible_height = content.height as usize;
        self.last_visible_height = visible_height;

        // Pinned header: keep the latest assistant message at the top once it reaches
        // the top of the viewport (pushed there by accumulating tool output below it).
        const MAX_PIN_LINES: usize = 10;
        const MIN_SCROLLABLE_ROWS: usize = 4;

        // Probe the first visible line using full content dimensions (before any pin
        // adjustment) to break circularity. The pin activates when the message's first
        // line is at or above this probe value — i.e. it has naturally reached the top.
        let total_lines_full = cached_len + streaming_len + extra_len;
        let probe_start = total_lines_full
            .saturating_sub(self.scroll_offset)
            .saturating_sub(visible_height);

        let pinned_span = self
            .messages
            .iter()
            .rposition(|m| matches!(m.role, MessageRole::Assistant) && !m.content.is_empty())
            .and_then(|i| self.flat_cache_entry_spans.get(i).copied())
            .filter(|&(ps, pe)| {
                pe > ps
                    && ps <= probe_start
                    // Only pin when there's tool output or active processing after the
                    // message. Streaming assistant text alone does not qualify — that case
                    // is the agent typing its reply, which should scroll normally.
                    && (pe < cached_len || extra_len > 0)
            });

        let (pin_start, pin_end) = pinned_span.unwrap_or((cached_len, cached_len));
        let pin_content_lines = pin_end - pin_start;
        let pin_content_height = pin_content_lines.min(MAX_PIN_LINES);
        // +1 for separator line drawn below the pinned block
        let pin_total_height = if pin_content_height > 0
            && visible_height > pin_content_height + 1 + MIN_SCROLLABLE_ROWS
        {
            pin_content_height + 1
        } else {
            0
        };
        self.last_pin_height = pin_total_height;

        // Scroll-space dimensions: exclude only the lines actually displayed in the pin
        // header from the cached line count. Lines beyond pin_content_height remain in
        // the scrollable area so the user can scroll up to read a long pinned message.
        let s_cached_len = cached_len
            - if pin_total_height > 0 {
                pin_content_height
            } else {
                0
            };
        let s_visible = visible_height - pin_total_height;
        let s_total = s_cached_len + streaming_len + extra_len;

        // Map a scrollable flat-cache index to an actual flat_cache index, skipping only
        // the lines rendered in the pin header.  When the pin is inactive the mapping is
        // identity.
        let to_actual = |si: usize| -> usize {
            if pin_total_height > 0 && si >= pin_start {
                si + pin_content_height
            } else {
                si
            }
        };

        // Clamp scroll offset (respect locks if active)
        let max_scroll = s_total.saturating_sub(s_visible);
        let scroll_from_top = if let Some(lock) = self.selection_scroll_lock {
            // Selection drag in progress — hold absolute position
            let locked = lock.min(max_scroll);
            self.scroll_offset = max_scroll.saturating_sub(locked);
            locked
        } else if let Some(pinned) = self.pinned_scroll_top {
            // User has scrolled up — maintain absolute line position as content grows
            let locked = pinned.min(max_scroll);
            self.scroll_offset = max_scroll.saturating_sub(locked);
            self.pinned_scroll_top = Some(locked);
            locked
        } else {
            self.scroll_offset = self.scroll_offset.min(max_scroll);
            max_scroll.saturating_sub(self.scroll_offset)
        };

        // Track user scroll position so it stays fixed when new content arrives
        if self.selection_scroll_lock.is_none() {
            if self.scroll_offset > 0 {
                self.pinned_scroll_top = Some(scroll_from_top);
            } else {
                self.pinned_scroll_top = None;
            }
        }

        let start_line = s_total.saturating_sub(self.scroll_offset + s_visible);
        let end_line = s_total.saturating_sub(self.scroll_offset);
        let mut visible_lines: Vec<(Line<'static>, Option<usize>)> = Vec::with_capacity(s_visible);

        // Cached lines range (scrollable index space; pinned span is excluded).
        // line_index values are stored as actual flat_cache indices so that hover
        // detection and selection (which compare against flat_cache_entry_spans)
        // remain correct.
        let s_cached_end = s_cached_len;
        if start_line < s_cached_end {
            let slice_end = end_line.min(s_cached_end);
            for si in start_line..slice_end {
                let ai = to_actual(si);
                visible_lines.push((self.flat_cache[ai].clone(), Some(ai)));
            }
        }

        // Streaming lines range
        let streaming_start = s_cached_end;
        let streaming_end = s_cached_end + streaming_len;
        if streaming_len > 0 && end_line > streaming_start && start_line < streaming_end {
            if let Some(ref cached_streaming) = self.streaming_cache {
                let range_start = start_line.max(streaming_start) - streaming_start;
                let range_end = end_line.min(streaming_end) - streaming_start;
                for (idx, line) in cached_streaming[range_start..range_end]
                    .iter()
                    .cloned()
                    .enumerate()
                {
                    // Actual index: streaming content lives at [cached_len, cached_len+streaming_len)
                    let line_index = cached_len + range_start + idx;
                    visible_lines.push((line, Some(line_index)));
                }
            }
        }

        // Extra lines (thinking indicator, queued messages, prompts, spacing)
        if extra_len > 0 {
            let extra_start = streaming_end;
            let extra_end = streaming_end + extra_len;
            if start_line < extra_end && end_line > extra_start {
                let range_start = start_line.max(extra_start) - extra_start;
                let range_end = end_line.min(extra_end) - extra_start;
                for (idx, line) in extra_lines[range_start..range_end]
                    .iter()
                    .cloned()
                    .enumerate()
                {
                    // Actual index for hover detection on file paths in prompts
                    let line_index = cached_len + streaming_len + range_start + idx;
                    visible_lines.push((line, Some(line_index)));
                }
            }
        }

        let highlighted = self.apply_selection_highlight(visible_lines, content.width);

        // Scrollable content renders below the pinned header (or at full area when no pin)
        let scrollable_rect = Rect {
            x: content.x,
            y: content.y + pin_total_height as u16,
            width: content.width,
            height: content.height.saturating_sub(pin_total_height as u16),
        };
        let actual_lines = highlighted.len();
        let render_area = if actual_lines < s_visible {
            // Push short content to the bottom of the scrollable area
            let top_offset = (s_visible - actual_lines) as u16;
            Rect {
                x: scrollable_rect.x,
                y: scrollable_rect.y + top_offset,
                width: scrollable_rect.width,
                height: actual_lines as u16,
            }
        } else {
            scrollable_rect
        };
        Paragraph::new(highlighted).render(render_area, buf);

        // Render pinned header and separator
        if pin_total_height > 0 {
            let pin_lines: Vec<Line<'static>> =
                self.flat_cache[pin_start..(pin_start + pin_content_height)].to_vec();
            Paragraph::new(pin_lines).render(
                Rect {
                    x: content.x,
                    y: content.y,
                    width: content.width,
                    height: pin_content_height as u16,
                },
                buf,
            );
            let sep_line = Line::from(Span::styled(
                "─".repeat(content.width as usize),
                Style::default().fg(text_muted()),
            ));
            Paragraph::new(vec![sep_line]).render(
                Rect {
                    x: content.x,
                    y: content.y + pin_content_height as u16,
                    width: content.width,
                    height: 1,
                },
                buf,
            );
        }

        if show_scrollbar {
            render_minimal_scrollbar(
                Rect {
                    x: area.x + area.width.saturating_sub(1),
                    y: area.y,
                    width: 1,
                    height: area.height,
                },
                buf,
                s_total,
                s_visible,
                scroll_from_top,
            );
        }
    }

    fn invalidate_theme_cache_if_needed(&mut self) {
        let current = theme_revision();
        if self.theme_revision == current {
            return;
        }

        self.theme_revision = current;
        self.line_cache = LineCache::default();
        self.flat_cache.clear();
        self.flat_cache_width = None;
        self.flat_cache_dirty = true;
        self.streaming_cache = None;
        self.streaming_joiner_before = None;
        self.cache_width = None;
        tracing::debug!(
            theme_revision = current,
            "Chat view cache invalidated for theme change"
        );
    }
}

fn wrap_spans(spans: Vec<Span<'static>>, max_width: usize) -> Vec<Vec<Span<'static>>> {
    let (lines, _joiners) = wrap_spans_with_joiners(spans, max_width);
    lines
}

fn wrap_spans_with_joiners(
    spans: Vec<Span<'static>>,
    max_width: usize,
) -> (Vec<Vec<Span<'static>>>, Vec<Option<String>>) {
    if spans.is_empty() {
        return (vec![Vec::new()], vec![None]);
    }

    if max_width == 0 {
        return (vec![Vec::new()], vec![None]);
    }

    let mut chars: Vec<(char, Style)> = Vec::new();
    let mut col: usize = 0;
    for span in spans {
        let style = span.style;
        for ch in span.content.chars() {
            if ch == '\t' {
                let spaces = 8 - (col % 8);
                for _ in 0..spaces {
                    chars.push((' ', style));
                    col += 1;
                }
                continue;
            }
            if ch.is_control() {
                continue;
            }
            let w = UnicodeWidthChar::width(ch).unwrap_or(0);
            chars.push((ch, style));
            col += w;
        }
    }

    if chars.is_empty() {
        return (vec![Vec::new()], vec![None]);
    }

    let mut lines: Vec<Vec<(char, Style)>> = Vec::new();
    let mut joiners: Vec<Option<String>> = Vec::new();
    let mut current: Vec<(char, Style)> = Vec::new();
    let mut line_width = 0usize;
    let mut last_break: Option<(usize, usize)> = None;
    // Joiners preserve whitespace between wrapped lines so copy can reconstruct original text.
    let mut pending_joiner: Option<String> = None;

    let trailing_whitespace = |line: &[(char, Style)]| -> String {
        // Capture whitespace that would be lost when we split a line; this becomes the joiner.
        let mut rev = String::new();
        for (ch, _) in line.iter().rev() {
            if ch.is_whitespace() {
                rev.push(*ch);
            } else {
                break;
            }
        }
        rev.chars().rev().collect()
    };

    for (ch, style) in chars {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);

        if line_width + ch_width > max_width && !current.is_empty() {
            if let Some((break_idx, break_width)) = last_break {
                // Word-boundary wrap: keep whitespace at the break as a joiner for copy reconstruction.
                let joiner = if break_idx == 0 {
                    String::new()
                } else {
                    trailing_whitespace(&current[..break_idx.min(current.len())])
                };
                let next_line = current.split_off(break_idx);
                lines.push(current);
                joiners.push(pending_joiner.take());
                current = next_line;
                pending_joiner = Some(joiner);
                line_width = line_width.saturating_sub(break_width);
                last_break = None;

                let mut width = 0usize;
                for (idx, (c, _)) in current.iter().enumerate() {
                    let w = UnicodeWidthChar::width(*c).unwrap_or(0);
                    width += w;
                    if c.is_whitespace() {
                        last_break = Some((idx + 1, width));
                    }
                }
            } else {
                // Mid-word wrap: no whitespace to preserve, so use an empty joiner.
                lines.push(current);
                joiners.push(pending_joiner.take());
                current = Vec::new();
                line_width = 0;
                last_break = None;
                pending_joiner = Some(String::new());
            }
        }

        current.push((ch, style));
        line_width += ch_width;
        if ch.is_whitespace() {
            last_break = Some((current.len(), line_width));
        }
    }

    lines.push(current);
    joiners.push(pending_joiner.take());

    let out_lines = lines
        .into_iter()
        .map(|line_chars| chars_to_spans(line_chars))
        .collect();
    (out_lines, joiners)
}

fn line_gutter_cols(line: &Line<'_>) -> u16 {
    const TOOL_BLOCK_PREFIX: &str = "┃  ";
    const CONTENT_PREFIX_WIDTH: u16 = 2; // "❯ ", "• ", "ℹ ", "✗ ", "  "
    const CONTENT_PREFIXES: [&str; 5] = ["❯ ", "• ", "ℹ ", "✗ ", "  "];

    let flat = line_to_flat(line);
    if flat.starts_with(TOOL_BLOCK_PREFIX) {
        3
    } else if CONTENT_PREFIXES
        .iter()
        .any(|prefix| flat.starts_with(prefix))
    {
        CONTENT_PREFIX_WIDTH
    } else {
        0
    }
}

fn highlight_code_block_line(line: &Line<'static>) -> Line<'static> {
    let bg = accent_success();
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(line.spans.len() + 1);
    spans.push(Span::styled(" ", Style::default().bg(bg)));
    let mut first_span = true;
    for span in &line.spans {
        if first_span && span.content.chars().all(|c| c == ' ' || c == '\t') {
            // Replace leading whitespace span with a single gap space, preserving original width minus the bar cell
            let gap = span.content.len().saturating_sub(1);
            if gap > 0 {
                spans.push(Span::raw(" ".repeat(gap)));
            }
            first_span = false;
        } else {
            first_span = false;
            spans.push(span.clone());
        }
    }
    Line::from(spans).style(line.style)
}

fn highlight_line_by_cols(line: &Line<'static>, start_col: u16, end_col: u16) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buffer = String::new();
    let mut current_style: Option<Style> = None;
    let mut col: u16 = 0;

    for span in &line.spans {
        let base_style = span.style;
        for ch in span.content.chars() {
            let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
            let end = col.saturating_add(w.saturating_sub(1));
            let in_selection = end >= start_col && col <= end_col;
            let style = if in_selection {
                base_style.bg(bg_highlight())
            } else {
                base_style
            };

            if current_style.map(|s| s == style).unwrap_or(false) {
                buffer.push(ch);
            } else {
                if !buffer.is_empty() {
                    spans.push(Span::styled(
                        std::mem::take(&mut buffer),
                        current_style.unwrap_or_default(),
                    ));
                }
                current_style = Some(style);
                buffer.push(ch);
            }

            col = col.saturating_add(w);
        }
    }

    if !buffer.is_empty() {
        spans.push(Span::styled(buffer, current_style.unwrap_or_default()));
    }

    Line::from(spans).style(line.style)
}

/// Apply underline styling to characters in the specified column range (for hover highlighting)
fn underline_line_by_cols(line: &Line<'static>, start_col: u16, end_col: u16) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buffer = String::new();
    let mut current_style: Option<Style> = None;
    let mut col: u16 = 0;

    for span in &line.spans {
        let base_style = span.style;
        for ch in span.content.chars() {
            let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
            let end = col.saturating_add(w.saturating_sub(1));
            let in_range = end >= start_col && col < end_col;
            let style = if in_range {
                // Add underline and accent color for hovered file paths
                base_style
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(accent_primary())
            } else {
                base_style
            };

            if current_style.map(|s| s == style).unwrap_or(false) {
                buffer.push(ch);
            } else {
                if !buffer.is_empty() {
                    spans.push(Span::styled(
                        std::mem::take(&mut buffer),
                        current_style.unwrap_or_default(),
                    ));
                }
                current_style = Some(style);
                buffer.push(ch);
            }

            col = col.saturating_add(w);
        }
    }

    if !buffer.is_empty() {
        spans.push(Span::styled(buffer, current_style.unwrap_or_default()));
    }

    Line::from(spans).style(line.style)
}

fn selection_to_copy_text(
    lines: &[Line<'static>],
    joiner_before: &[Option<String>],
    start: SelectionPoint,
    end: SelectionPoint,
    width: u16,
) -> Option<String> {
    if width == 0 {
        return None;
    }

    let (start, end) = order_points(start, end);
    if start == end {
        return None;
    }

    let max_x = width.saturating_sub(1);
    let mut out = String::new();
    let mut prev_selected_line: Option<usize> = None;
    let mut wrote_any = false;

    for line_index in start.line_index..=end.line_index {
        let line = lines.get(line_index)?;
        let base_x = line_gutter_cols(line);
        if base_x > max_x {
            continue;
        }
        let content_width = max_x.saturating_sub(base_x);

        let line_start_col = if line_index == start.line_index {
            start.column
        } else {
            0
        };
        let line_end_col = if line_index == end.line_index {
            end.column
        } else {
            content_width
        };

        let row_sel_start = base_x
            .saturating_add(line_start_col.min(content_width))
            .min(max_x);
        let mut row_sel_end = base_x
            .saturating_add(line_end_col.min(content_width))
            .min(max_x);
        if row_sel_start > row_sel_end {
            continue;
        }

        let is_code_block_line = is_code_block_line(line);
        if is_code_block_line && line_end_col >= content_width {
            // For code blocks, allow selection to extend beyond visible width
            // to capture the full line content when copying.
            row_sel_end = u16::MAX;
        }

        let flat = line_to_flat(line);
        let text_end = if is_code_block_line {
            last_non_space_col(flat.as_str())
        } else {
            last_non_space_col(flat.as_str()).map(|c| c.min(max_x))
        };

        let selected_line = if let Some(text_end) = text_end {
            let from_col = row_sel_start.max(base_x);
            let to_col = row_sel_end.min(text_end);
            if from_col > to_col {
                Line::default().style(line.style)
            } else {
                slice_line_by_cols(line, from_col, to_col)
            }
        } else {
            Line::default().style(line.style)
        };

        let line_text = line_to_markdown(&selected_line, is_code_block_line);

        if wrote_any {
            let joiner = joiner_before.get(line_index).cloned().unwrap_or(None);
            if prev_selected_line == Some(line_index.saturating_sub(1)) {
                if let Some(joiner) = joiner {
                    out.push_str(joiner.as_str());
                } else {
                    out.push('\n');
                }
            } else {
                out.push('\n');
            }
        }

        out.push_str(line_text.as_str());
        prev_selected_line = Some(line_index);
        wrote_any = true;
    }

    (!out.is_empty()).then_some(out)
}

fn order_points(a: SelectionPoint, b: SelectionPoint) -> (SelectionPoint, SelectionPoint) {
    if (b.line_index < a.line_index) || (b.line_index == a.line_index && b.column < a.column) {
        (b, a)
    } else {
        (a, b)
    }
}

fn line_to_flat(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect::<String>()
}

fn is_code_block_line(line: &Line<'_>) -> bool {
    line.style.bg == Some(tool_block_bg())
        || line
            .spans
            .iter()
            .any(|span| span.style.bg == Some(tool_block_bg()))
        || line
            .spans
            .iter()
            .any(|span| span.style.bg == Some(markdown_code_bg()))
}

fn last_non_space_col(flat: &str) -> Option<u16> {
    let mut col: u16 = 0;
    let mut last: Option<u16> = None;
    for ch in flat.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        if ch != ' ' {
            let end = col.saturating_add(w.saturating_sub(1));
            last = Some(end);
        }
        col = col.saturating_add(w);
    }
    last
}

fn byte_range_for_cols(flat: &str, start_col: u16, end_col: u16) -> Option<std::ops::Range<usize>> {
    let mut col: u16 = 0;
    let mut start_byte: Option<usize> = None;
    let mut end_byte: Option<usize> = None;

    for (idx, ch) in flat.char_indices() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        let end = col.saturating_add(w.saturating_sub(1));

        if start_byte.is_none() && end >= start_col {
            start_byte = Some(idx);
        }

        if col <= end_col {
            end_byte = Some(idx + ch.len_utf8());
        }

        col = col.saturating_add(w);
        if col > end_col {
            break;
        }
    }

    let start = start_byte?;
    let end = end_byte?;
    if start > end {
        None
    } else {
        Some(start..end)
    }
}

fn slice_line_by_cols(line: &Line<'static>, start_col: u16, end_col: u16) -> Line<'static> {
    let flat = line_to_flat(line);
    let Some(range) = byte_range_for_cols(flat.as_str(), start_col, end_col) else {
        return Line::default().style(line.style);
    };

    let mut out_spans: Vec<Span<'static>> = Vec::new();
    let mut offset = 0usize;
    for span in &line.spans {
        let span_len = span.content.len();
        let span_start = offset;
        let span_end = offset + span_len;

        if range.end <= span_start || range.start >= span_end {
            offset = span_end;
            continue;
        }

        let local_start = range.start.saturating_sub(span_start);
        let local_end = range.end.min(span_end).saturating_sub(span_start);
        if local_start < local_end {
            let slice = span.content[local_start..local_end].to_string();
            out_spans.push(Span::styled(slice, span.style));
        }
        offset = span_end;
    }

    Line::from(out_spans).style(line.style)
}

fn line_to_markdown(line: &Line<'static>, is_code_block: bool) -> String {
    let _ = is_code_block;
    line_to_flat(line)
}

fn chars_to_spans(chars: Vec<(char, Style)>) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut buffer = String::new();
    let mut current_style: Option<Style> = None;

    for (ch, style) in chars {
        if current_style.map(|s| s == style).unwrap_or(false) {
            buffer.push(ch);
        } else {
            if !buffer.is_empty() {
                spans.push(Span::styled(
                    buffer.clone(),
                    current_style.unwrap_or_default(),
                ));
                buffer.clear();
            }
            current_style = Some(style);
            buffer.push(ch);
        }
    }

    if !buffer.is_empty() {
        spans.push(Span::styled(buffer, current_style.unwrap_or_default()));
    }

    spans
}

impl Default for ChatView {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str) -> Line<'static> {
        Line::from(Span::raw(text.to_string()))
    }

    fn code_line(text: &str) -> Line<'static> {
        Line::from(Span::raw(text.to_string()))
    }

    #[test]
    fn test_push_scrolls_to_bottom_when_already_at_bottom() {
        let mut view = ChatView::new();
        view.push(ChatMessage::user("First message"));
        assert_eq!(view.scroll_offset, 0);

        view.push(ChatMessage::assistant("Response"));
        assert_eq!(
            view.scroll_offset, 0,
            "Should stay at bottom when already at bottom"
        );
    }

    #[test]
    fn test_push_preserves_scroll_when_user_scrolled_up() {
        let mut view = ChatView::new();
        view.push(ChatMessage::user("Message 1"));
        view.push(ChatMessage::assistant("Response 1"));

        // User scrolls up
        view.scroll_up(5);
        assert_eq!(view.scroll_offset, 5);

        // New message arrives
        view.push(ChatMessage::assistant("Response 2"));

        // Scroll position should be preserved (not reset to 0)
        assert!(
            view.scroll_offset > 0,
            "Scroll position should be preserved when user has scrolled up, got {}",
            view.scroll_offset
        );
    }

    #[test]
    fn test_finalize_streaming_preserves_scroll_when_user_scrolled_up() {
        let mut view = ChatView::new();
        view.push(ChatMessage::user("Question"));

        // Start streaming
        view.stream_append("Streaming content...");

        // User scrolls up during streaming
        view.scroll_up(3);
        assert_eq!(view.scroll_offset, 3);

        // Finalize streaming
        view.finalize_streaming();

        // Scroll should be preserved
        assert!(
            view.scroll_offset > 0,
            "Scroll position should be preserved after finalize_streaming, got {}",
            view.scroll_offset
        );
    }

    #[test]
    fn test_streaming_reasoning_finalizes_in_order() {
        let mut view = ChatView::new();
        view.stream_append_role(MessageRole::Reasoning, "thinking...");
        view.stream_append("answer");

        view.finalize_streaming();

        assert_eq!(view.messages.len(), 2);
        assert_eq!(view.messages[0].role, MessageRole::Reasoning);
        assert_eq!(view.messages[0].content, "thinking...");
        assert_eq!(view.messages[1].role, MessageRole::Assistant);
        assert_eq!(view.messages[1].content, "answer");
    }

    #[test]
    fn test_finalize_streaming_stays_at_bottom_when_at_bottom() {
        let mut view = ChatView::new();
        view.stream_append("Streaming...");
        assert_eq!(view.scroll_offset, 0);

        view.finalize_streaming();
        assert_eq!(
            view.scroll_offset, 0,
            "Should stay at bottom when already at bottom"
        );
    }

    #[test]
    fn test_tool_message_block_style() {
        let mut view = ChatView::new();

        // Add a Bash tool message
        let tool_msg = ChatMessage::tool_with_exit(
            "Bash",
            r#"{"command": "ls -la", "description": "List files"}"#,
            "total 0\ndrwxr-xr-x  2 user staff 64 Jan  1 00:00 .\ndrwxr-xr-x 10 user staff 320 Jan  1 00:00 ..",
            Some(0),
        );
        view.push(tool_msg);

        // The view should have the tool message
        assert_eq!(view.messages.len(), 1);
        assert_eq!(view.messages[0].role, MessageRole::Tool);
    }

    #[test]
    fn test_tool_command_parsing_bash() {
        let view = ChatView::new();
        let result = view.get_tool_command(
            "Bash",
            r#"{"command": "cargo test", "description": "Run tests"}"#,
            100,
        );
        assert_eq!(result, "cargo test");
    }

    #[test]
    fn test_tool_command_parsing_read() {
        let view = ChatView::new();
        let result = view.get_tool_command(
            "Read",
            r#"{"file_path": "/path/to/file.rs", "offset": 10, "limit": 50}"#,
            100,
        );
        // 1-indexed display: offset 10 + 1 = line 11, through offset 10 + limit 50 = line 60
        assert_eq!(result, "/path/to/file.rs (lines 11-60)");
    }

    #[test]
    fn test_tool_command_parsing_grep() {
        let view = ChatView::new();
        let result =
            view.get_tool_command("Grep", r#"{"pattern": "fn main", "path": "src/"}"#, 100);
        assert_eq!(result, "\"fn main\" in src/");
    }

    #[test]
    fn test_tool_description_with_custom() {
        let view = ChatView::new();
        let result = view.get_tool_description(
            "Bash",
            r#"{"command": "ls", "description": "List directory contents"}"#,
        );
        assert_eq!(result, "List directory contents");
    }

    #[test]
    fn test_tool_description_default() {
        let view = ChatView::new();
        let result = view.get_tool_description("Read", r#"{"file_path": "/path/to/file"}"#);
        assert_eq!(result, "Read file");
    }

    #[test]
    fn test_update_last_tool_no_tool_message() {
        let mut view = ChatView::new();
        // Add only non-tool messages
        view.push(ChatMessage::user("Hello"));
        view.push(ChatMessage::assistant("Hi there"));

        // update_last_tool should return false when no tool message exists
        let result = view.update_last_tool("new content".to_string(), Some(0));
        assert!(!result, "Should return false when no tool message exists");

        // Original messages should be unchanged
        assert_eq!(view.messages.len(), 2);
        assert_eq!(view.messages[0].content, "Hello");
        assert_eq!(view.messages[1].content, "Hi there");
    }

    #[test]
    fn test_update_last_tool_empty_view() {
        let mut view = ChatView::new();

        // update_last_tool on empty view should return false
        let result = view.update_last_tool("content".to_string(), Some(0));
        assert!(!result, "Should return false on empty view");
    }

    #[test]
    fn test_tool_message_collapsed_state() {
        let mut view = ChatView::new();

        // Create a tool message and set it to collapsed
        let mut tool_msg = ChatMessage::tool(
            "Bash",
            r#"{"command": "ls"}"#,
            "file1.txt\nfile2.txt\nfile3.txt",
        );
        tool_msg.is_collapsed = true;
        view.push(tool_msg);

        assert!(view.messages[0].is_collapsed, "Message should be collapsed");

        // Toggle to expanded
        view.messages[0].is_collapsed = false;
        assert!(!view.messages[0].is_collapsed, "Message should be expanded");
    }

    #[test]
    fn test_tool_message_error_exit_code() {
        let mut view = ChatView::new();

        // Add a tool message with error exit code
        let tool_msg = ChatMessage::tool_with_exit(
            "Bash",
            r#"{"command": "false"}"#,
            "Command failed",
            Some(1),
        );
        view.push(tool_msg);

        assert_eq!(view.messages[0].exit_code, Some(1));

        // Test updating exit code via update_last_tool
        view.update_last_tool("Updated output".to_string(), Some(127));
        assert_eq!(view.messages[0].exit_code, Some(127));
        assert_eq!(view.messages[0].content, "Updated output");
    }

    #[test]
    fn test_tool_message_success_exit_code() {
        let mut view = ChatView::new();

        let tool_msg =
            ChatMessage::tool_with_exit("Bash", r#"{"command": "true"}"#, "Success", Some(0));
        view.push(tool_msg);

        assert_eq!(view.messages[0].exit_code, Some(0));
    }

    #[test]
    fn test_selection_to_copy_text_flattens_code_blocks() {
        let lines = vec![
            line("before"),
            code_line("code1"),
            code_line("code2"),
            line("after"),
        ];
        let joiners = vec![None, None, None, None];
        let start = SelectionPoint {
            line_index: 0,
            column: 0,
        };
        let end = SelectionPoint {
            line_index: 3,
            column: 10,
        };
        let out = selection_to_copy_text(&lines, &joiners, start, end, 80).unwrap();
        assert_eq!(out, "before\ncode1\ncode2\nafter");
    }

    #[test]
    fn test_selection_to_copy_text_uses_joiner_whitespace() {
        let lines = vec![line("hello"), line("world")];
        let joiners = vec![None, Some(" ".to_string())];
        let start = SelectionPoint {
            line_index: 0,
            column: 0,
        };
        let end = SelectionPoint {
            line_index: 1,
            column: 10,
        };
        let out = selection_to_copy_text(&lines, &joiners, start, end, 80).unwrap();
        assert_eq!(out, "hello world");
    }

    #[test]
    fn test_selection_to_copy_text_uses_empty_joiner_for_mid_word_wrap() {
        let lines = vec![line("hello"), line("world")];
        let joiners = vec![None, Some(String::new())];
        let start = SelectionPoint {
            line_index: 0,
            column: 0,
        };
        let end = SelectionPoint {
            line_index: 1,
            column: 10,
        };
        let out = selection_to_copy_text(&lines, &joiners, start, end, 80).unwrap();
        assert_eq!(out, "helloworld");
    }

    #[test]
    fn test_wrap_spans_joiner_uses_break_whitespace() {
        let spans = vec![Span::raw("ab cd ef".to_string())];
        let (_lines, joiners) = wrap_spans_with_joiners(spans, 5);

        assert!(joiners.len() > 1);
        assert_eq!(joiners[0], None);
        assert_eq!(joiners[1], Some(" ".to_string()));
    }

    #[test]
    fn test_selection_to_copy_text_preserves_empty_lines() {
        let lines = vec![line("first"), line(""), line("third")];
        let joiners = vec![None, None, None];
        let start = SelectionPoint {
            line_index: 0,
            column: 0,
        };
        let end = SelectionPoint {
            line_index: 2,
            column: 10,
        };
        let out = selection_to_copy_text(&lines, &joiners, start, end, 80).unwrap();
        assert_eq!(out, "first\n\nthird");
    }

    #[test]
    fn test_selection_to_copy_text_mid_line_code_block() {
        let lines = vec![code_line("abcde")];
        let joiners = vec![None];
        let start = SelectionPoint {
            line_index: 0,
            column: 1,
        };
        let end = SelectionPoint {
            line_index: 0,
            column: 3,
        };
        let out = selection_to_copy_text(&lines, &joiners, start, end, 80).unwrap();
        assert_eq!(out, "bcd");
    }

    #[test]
    fn test_selection_to_copy_text_mixed_code_and_text_paragraphs() {
        let lines = vec![
            line("para1"),
            line(""),
            code_line("code"),
            line(""),
            line("para2"),
        ];
        let joiners = vec![None, None, None, None, None];
        let start = SelectionPoint {
            line_index: 0,
            column: 0,
        };
        let end = SelectionPoint {
            line_index: 4,
            column: 10,
        };
        let out = selection_to_copy_text(&lines, &joiners, start, end, 80).unwrap();
        assert_eq!(out, "para1\n\ncode\n\npara2");
    }
}
