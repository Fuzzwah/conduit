//! In-TUI keybindings editor component.

use std::collections::HashMap;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::config::action_to_name;
use crate::config::default_keys::default_keybindings;
use crate::config::keys::{KeyCombo, KeyContext, KeybindingConfig};
use crate::ui::action::Action;

use super::{
    accent_error, accent_primary, bg_highlight, dialog_bg, ensure_contrast_bg, ensure_contrast_fg,
    render_minimal_scrollbar, text_muted, text_primary, text_secondary, truncate_to_width,
    DialogFrame,
};

const DIALOG_WIDTH: u16 = 76;
const DIALOG_HEIGHT: u16 = 24;

/// A single bindable (context, key_combo, action) triple from the default keybindings.
#[derive(Debug, Clone)]
pub struct KeybindingItem {
    pub context: Option<KeyContext>,
    pub context_label: String,
    /// Canonical TOML name for this action (e.g. "quit")
    pub action_name: &'static str,
    pub action: Action,
    /// Human-readable description (from `Action::description()`)
    pub action_label: String,
    /// Currently active key (user override if set, otherwise same as default_key)
    pub current_key: String,
    /// Default key notation string (e.g. "C-q")
    pub default_key: String,
    /// Whether the user has a custom TOML binding for this action+context
    pub is_user_override: bool,
}

/// A row in the rendered list — either a context section header or an item.
#[derive(Debug, Clone)]
enum DisplayRow {
    Header(String),
    Item(usize),
}

/// A key conflict that needs user confirmation before reassigning.
#[derive(Debug, Clone)]
pub struct ConflictPending {
    /// String form of the combo (e.g. "C-q"), used to save once confirmed.
    pub key_str: String,
    /// Human-readable label of the action that currently owns this key.
    pub conflicting_label: String,
}

/// State for the keybindings editor dialog.
#[derive(Debug, Clone)]
pub struct KeybindingsEditorState {
    pub visible: bool,
    /// All bindable items (populated when dialog opens)
    pub items: Vec<KeybindingItem>,
    /// Current filter text
    pub filter: String,
    /// Cursor position in filter input
    pub filter_cursor: usize,
    /// Rendered display rows (headers + filtered items)
    display_rows: Vec<DisplayRow>,
    /// Index of the selected row in `display_rows`
    selected_row: usize,
    /// Scroll offset for the display list
    scroll_offset: usize,
    /// Whether we are waiting for a keypress to capture
    pub capture_mode: bool,
    /// Index into `items` of the binding being remapped
    pub capture_item_idx: Option<usize>,
    /// Set when the pressed key conflicts with another binding; awaits confirmation.
    pub conflict_pending: Option<ConflictPending>,
    /// Timed status message shown at the bottom of the dialog
    pub status_message: Option<String>,
}

impl Default for KeybindingsEditorState {
    fn default() -> Self {
        Self::new()
    }
}

impl KeybindingsEditorState {
    pub fn new() -> Self {
        Self {
            visible: false,
            items: Vec::new(),
            filter: String::new(),
            filter_cursor: 0,
            display_rows: Vec::new(),
            selected_row: 0,
            scroll_offset: 0,
            capture_mode: false,
            capture_item_idx: None,
            conflict_pending: None,
            status_message: None,
        }
    }

    pub fn show(&mut self, items: Vec<KeybindingItem>) {
        self.visible = true;
        self.items = items;
        self.filter.clear();
        self.filter_cursor = 0;
        self.selected_row = 0;
        self.scroll_offset = 0;
        self.capture_mode = false;
        self.capture_item_idx = None;
        self.conflict_pending = None;
        self.status_message = None;
        self.rebuild_display_rows();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.capture_mode = false;
        self.capture_item_idx = None;
        self.conflict_pending = None;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn enter_capture_mode(&mut self) {
        if let Some(item_idx) = self.selected_item_idx() {
            if self.items[item_idx].is_editable() {
                self.capture_mode = true;
                self.capture_item_idx = Some(item_idx);
                self.status_message = None;
            } else {
                self.status_message = Some("This binding cannot be edited via the TUI".to_string());
            }
        }
    }

    pub fn cancel_capture(&mut self) {
        self.capture_mode = false;
        self.capture_item_idx = None;
        self.conflict_pending = None;
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Insert a character into the filter and rebuild display.
    pub fn insert_filter_char(&mut self, c: char) {
        self.filter.insert(self.filter_cursor, c);
        self.filter_cursor += c.len_utf8();
        self.selected_row = 0;
        self.scroll_offset = 0;
        self.rebuild_display_rows();
    }

    /// Delete character before cursor in filter.
    pub fn delete_filter_char(&mut self) {
        if self.filter_cursor > 0 {
            let ch_start = self.filter[..self.filter_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.filter.remove(ch_start);
            self.filter_cursor = ch_start;
            self.selected_row = 0;
            self.scroll_offset = 0;
            self.rebuild_display_rows();
        }
    }

    pub fn select_next(&mut self) {
        let selectable: Vec<usize> = self
            .display_rows
            .iter()
            .enumerate()
            .filter(|(_, r)| matches!(r, DisplayRow::Item(_)))
            .map(|(i, _)| i)
            .collect();

        if let Some(pos) = selectable.iter().position(|&i| i == self.selected_row) {
            if pos + 1 < selectable.len() {
                self.selected_row = selectable[pos + 1];
                self.scroll_to_selected();
            }
        } else if let Some(&first) = selectable.first() {
            self.selected_row = first;
            self.scroll_to_selected();
        }
    }

    pub fn select_prev(&mut self) {
        let selectable: Vec<usize> = self
            .display_rows
            .iter()
            .enumerate()
            .filter(|(_, r)| matches!(r, DisplayRow::Item(_)))
            .map(|(i, _)| i)
            .collect();

        if let Some(pos) = selectable.iter().position(|&i| i == self.selected_row) {
            if pos > 0 {
                self.selected_row = selectable[pos - 1];
                self.scroll_to_selected();
            }
        } else if let Some(&first) = selectable.first() {
            self.selected_row = first;
            self.scroll_to_selected();
        }
    }

    pub fn select_page_down(&mut self, page_size: usize) {
        for _ in 0..page_size {
            self.select_next();
        }
    }

    pub fn select_page_up(&mut self, page_size: usize) {
        for _ in 0..page_size {
            self.select_prev();
        }
    }

    /// Returns the `items` index of the currently selected item, if any.
    pub fn selected_item_idx(&self) -> Option<usize> {
        match self.display_rows.get(self.selected_row) {
            Some(DisplayRow::Item(idx)) => Some(*idx),
            _ => {
                // Fall back to first selectable item
                self.display_rows.iter().find_map(|r| {
                    if let DisplayRow::Item(idx) = r {
                        Some(*idx)
                    } else {
                        None
                    }
                })
            }
        }
    }

    /// Refresh item list after a remap or reset, preserving selection.
    pub fn refresh_items(&mut self, new_items: Vec<KeybindingItem>) {
        self.items = new_items;
        self.rebuild_display_rows();
    }

    fn rebuild_display_rows(&mut self) {
        let query = self.filter.to_lowercase();
        let mut rows = Vec::new();
        let mut current_context_label: Option<String> = None;
        let mut context_header_row: Option<usize> = None;

        for (idx, item) in self.items.iter().enumerate() {
            let matches = query.is_empty()
                || item.action_label.to_lowercase().contains(&query)
                || item.action_name.contains(&query)
                || item.current_key.to_lowercase().contains(&query)
                || item.default_key.to_lowercase().contains(&query)
                || item.context_label.to_lowercase().contains(&query);

            if !matches {
                continue;
            }

            // Emit a header when context changes
            if current_context_label.as_deref() != Some(&item.context_label) {
                current_context_label = Some(item.context_label.clone());
                context_header_row = Some(rows.len());
                rows.push(DisplayRow::Header(item.context_label.clone()));
            }
            let _ = context_header_row;
            rows.push(DisplayRow::Item(idx));
        }

        self.display_rows = rows;

        // Re-clamp selection to a valid selectable row
        let has_selected_item = matches!(
            self.display_rows.get(self.selected_row),
            Some(DisplayRow::Item(_))
        );
        if !has_selected_item {
            self.selected_row = self
                .display_rows
                .iter()
                .enumerate()
                .find_map(|(i, r)| matches!(r, DisplayRow::Item(_)).then_some(i))
                .unwrap_or(0);
        }
        self.clamp_scroll();
    }

    fn scroll_to_selected(&mut self) {
        self.clamp_scroll();
        let max_visible = visible_rows_count(DIALOG_HEIGHT);
        if self.selected_row < self.scroll_offset {
            self.scroll_offset = self.selected_row;
        } else if self.selected_row >= self.scroll_offset + max_visible {
            self.scroll_offset = self.selected_row + 1 - max_visible;
        }
    }

    fn clamp_scroll(&mut self) {
        let max_visible = visible_rows_count(DIALOG_HEIGHT);
        let total = self.display_rows.len();
        if total <= max_visible {
            self.scroll_offset = 0;
        } else {
            self.scroll_offset = self.scroll_offset.min(total - max_visible);
        }
    }
}

impl KeybindingItem {
    /// Whether this item can be remapped via the TUI editor.
    /// Items in contexts without a TOML section are read-only.
    pub fn is_editable(&self) -> bool {
        match self.context {
            None => true, // global
            Some(ctx) => ctx.toml_section_name().is_some(),
        }
    }
}

/// Calculate how many rows are visible in the list area.
fn visible_rows_count(dialog_height: u16) -> usize {
    // Header(1) + separator(1) + filter(1) + separator(1) + footer(1) = 5 chrome lines
    // Plus DialogFrame border = 2 top + 2 bottom = 4
    // Actually: inner area height = dialog_height - 4 (frame borders + instructions)
    // Then: 1 filter, 1 sep, 1 footer = 3 more lines
    let chrome = 4u16 + 3; // frame(4) + filter+sep+footer(3)
    (dialog_height.saturating_sub(chrome)) as usize
}

/// Build the full list of keybinding items from `default_keybindings()`,
/// cross-referencing the live config to detect user overrides.
pub fn build_keybinding_items(live_config: &KeybindingConfig) -> Vec<KeybindingItem> {
    let defaults = default_keybindings();

    // Build sets of (context, key_combo) that are in defaults per action
    // so we can detect user additions.
    // Format: action → Vec<(context, combo_str)>
    let mut default_combos_by_action: HashMap<String, Vec<(Option<KeyContext>, String)>> =
        HashMap::new();

    // Global defaults
    for (combo, action) in &defaults.global {
        if let Some(name) = action_to_name(action) {
            default_combos_by_action
                .entry(name.to_string())
                .or_default()
                .push((None, combo.to_string()));
        }
    }
    // Context defaults
    for (ctx, bindings) in &defaults.context {
        for (combo, action) in bindings {
            if let Some(name) = action_to_name(action) {
                default_combos_by_action
                    .entry(name.to_string())
                    .or_default()
                    .push((Some(*ctx), combo.to_string()));
            }
        }
    }

    let mut items: Vec<KeybindingItem> = Vec::new();

    // --- Global bindings ---
    let mut global_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Collect and sort for stable ordering
    let mut global_entries: Vec<(&KeyCombo, &Action)> = defaults.global.iter().collect();
    global_entries.sort_by_key(|(_, a)| action_to_name(a).unwrap_or(""));

    for (combo, action) in global_entries {
        let Some(action_name) = action_to_name(action) else {
            continue;
        };
        if !global_seen.insert(action_name.to_string()) {
            continue; // deduplicate per action in global
        }

        let is_user_override = user_has_override(None, action, action_name, live_config, &defaults);
        let default_key = combo.to_string();
        let current_key = if is_user_override {
            live_override_key(None, action, live_config, &defaults)
                .unwrap_or_else(|| default_key.clone())
        } else {
            default_key.clone()
        };

        items.push(KeybindingItem {
            context: None,
            context_label: "Global".to_string(),
            action_name,
            action: action.clone(),
            action_label: action.description().to_string(),
            current_key,
            default_key,
            is_user_override,
        });
    }

    // Sort global items by action_label
    items.sort_by(|a, b| a.action_label.cmp(&b.action_label));

    // --- Context-specific bindings ---
    let mut context_list: Vec<KeyContext> = KeyContext::all_contexts().to_vec();
    context_list.sort_by_key(|c| context_display_name(*c));

    for ctx in context_list {
        let Some(bindings) = defaults.context.get(&ctx) else {
            continue;
        };

        let ctx_label = context_display_name(ctx).to_string();
        let mut ctx_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut ctx_entries: Vec<(&KeyCombo, &Action)> = bindings.iter().collect();
        ctx_entries.sort_by_key(|(_, a)| action_to_name(a).unwrap_or(""));

        let mut ctx_items: Vec<KeybindingItem> = Vec::new();

        for (combo, action) in ctx_entries {
            let Some(action_name) = action_to_name(action) else {
                continue;
            };
            if !ctx_seen.insert(action_name.to_string()) {
                continue;
            }

            let is_user_override =
                user_has_override(Some(ctx), action, action_name, live_config, &defaults);
            let default_key = combo.to_string();
            let current_key = if is_user_override {
                live_override_key(Some(ctx), action, live_config, &defaults)
                    .unwrap_or_else(|| default_key.clone())
            } else {
                default_key.clone()
            };

            ctx_items.push(KeybindingItem {
                context: Some(ctx),
                context_label: ctx_label.clone(),
                action_name,
                action: action.clone(),
                action_label: action.description().to_string(),
                current_key,
                default_key,
                is_user_override,
            });
        }

        ctx_items.sort_by(|a, b| a.action_label.cmp(&b.action_label));
        items.extend(ctx_items);
    }

    items
}

/// Returns true if the live config has a binding for `action` in this context
/// that is NOT present in the defaults (i.e. user added it via TOML).
fn user_has_override(
    context: Option<KeyContext>,
    action: &Action,
    _action_name: &str,
    live: &KeybindingConfig,
    defaults: &KeybindingConfig,
) -> bool {
    let live_combos: Vec<&KeyCombo> = match context {
        None => live
            .global
            .iter()
            .filter(|(_, a)| *a == action)
            .map(|(k, _)| k)
            .collect(),
        Some(ctx) => live
            .context
            .get(&ctx)
            .map(|m| {
                m.iter()
                    .filter(|(_, a)| *a == action)
                    .map(|(k, _)| k)
                    .collect()
            })
            .unwrap_or_default(),
    };

    let default_combos: std::collections::HashSet<&KeyCombo> = match context {
        None => defaults
            .global
            .iter()
            .filter(|(_, a)| *a == action)
            .map(|(k, _)| k)
            .collect(),
        Some(ctx) => defaults
            .context
            .get(&ctx)
            .map(|m| {
                m.iter()
                    .filter(|(_, a)| *a == action)
                    .map(|(k, _)| k)
                    .collect()
            })
            .unwrap_or_default(),
    };

    live_combos.iter().any(|k| !default_combos.contains(k))
}

/// Returns the user's override key string for an action in a context, if any.
fn live_override_key(
    context: Option<KeyContext>,
    action: &Action,
    live: &KeybindingConfig,
    defaults: &KeybindingConfig,
) -> Option<String> {
    let default_combos: std::collections::HashSet<&KeyCombo> = match context {
        None => defaults
            .global
            .iter()
            .filter(|(_, a)| *a == action)
            .map(|(k, _)| k)
            .collect(),
        Some(ctx) => defaults
            .context
            .get(&ctx)
            .map(|m| {
                m.iter()
                    .filter(|(_, a)| *a == action)
                    .map(|(k, _)| k)
                    .collect()
            })
            .unwrap_or_default(),
    };

    let live_iter: Box<dyn Iterator<Item = (&KeyCombo, &Action)>> = match context {
        None => Box::new(live.global.iter()),
        Some(ctx) => live
            .context
            .get(&ctx)
            .map(|m| -> Box<dyn Iterator<Item = _>> { Box::new(m.iter()) })
            .unwrap_or_else(|| Box::new(std::iter::empty())),
    };

    live_iter
        .filter(|(_, a)| *a == action)
        .find(|(k, _)| !default_combos.contains(k))
        .map(|(k, _)| k.to_string())
}

/// Human-readable display name for a context.
fn context_display_name(ctx: KeyContext) -> &'static str {
    match ctx {
        KeyContext::Global => "Global",
        KeyContext::Chat => "Chat",
        KeyContext::FileViewer => "File Viewer",
        KeyContext::Scrolling => "Scrolling",
        KeyContext::Sidebar => "Sidebar",
        KeyContext::Dialog => "Dialog",
        KeyContext::ProjectPicker => "Project Picker",
        KeyContext::ModelSelector => "Model Selector",
        KeyContext::AddRepository => "Add Repository",
        KeyContext::BaseDir => "Base Directory",
        KeyContext::RawEvents => "Raw Events",
        KeyContext::Command => "Command Mode",
        KeyContext::HelpDialog => "Help Dialog",
        KeyContext::SessionImport => "Session Import",
        KeyContext::CommandPalette => "Command Palette",
        KeyContext::ThemePicker => "Theme Picker",
        KeyContext::QueueEditing => "Queue Editor",
    }
}

/// The renderer widget.
pub struct KeybindingsEditor;

impl KeybindingsEditor {
    pub fn new() -> Self {
        Self
    }

    pub fn dialog_area(area: Rect) -> Rect {
        let w = DIALOG_WIDTH.min(area.width.saturating_sub(4));
        let h = DIALOG_HEIGHT.min(area.height.saturating_sub(2));
        Rect {
            x: area.x + (area.width.saturating_sub(w)) / 2,
            y: area.y + (area.height.saturating_sub(h)) / 2,
            width: w,
            height: h,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, state: &KeybindingsEditorState) {
        if !state.visible {
            return;
        }

        let title = if state.capture_mode {
            " Keybindings — Press new key "
        } else {
            " Keybindings "
        };

        let capture_item = state.capture_item_idx.and_then(|i| state.items.get(i));
        let instructions = if state.capture_mode {
            if state.conflict_pending.is_some() {
                vec![("Enter", "reassign"), ("Esc", "different key")]
            } else if capture_item.map(|i| i.is_user_override).unwrap_or(false) {
                vec![("Esc", "cancel"), ("⌫", "reset to default")]
            } else {
                vec![("Esc", "cancel")]
            }
        } else {
            vec![("↑↓", "navigate"), ("Enter", "remap"), ("Esc", "close")]
        };

        let frame = DialogFrame::new(title, DIALOG_WIDTH, DIALOG_HEIGHT).instructions(instructions);
        let inner = frame.render(area, buf);

        let chunks = Layout::vertical([
            Constraint::Length(1), // filter input
            Constraint::Length(1), // separator
            Constraint::Min(1),    // list
            Constraint::Length(1), // status/footer
        ])
        .split(inner);

        self.render_filter(chunks[0], buf, state);

        Paragraph::new("─".repeat(chunks[1].width as usize))
            .style(Style::default().fg(text_muted()))
            .render(chunks[1], buf);

        if state.capture_mode {
            self.render_capture_overlay(chunks[2], buf, state);
        } else {
            self.render_list(chunks[2], buf, state);
        }

        self.render_footer(chunks[3], buf, state);
    }

    fn render_filter(&self, area: Rect, buf: &mut Buffer, state: &KeybindingsEditorState) {
        let prefix = "Filter: ";
        let text = &state.filter;

        let mut spans = vec![Span::styled(prefix, Style::default().fg(text_muted()))];
        if text.is_empty() {
            spans.push(Span::styled(
                "type to filter",
                Style::default().fg(text_muted()),
            ));
        } else {
            spans.push(Span::styled(
                text.clone(),
                Style::default().fg(text_primary()),
            ));
        }
        Paragraph::new(Line::from(spans)).render(area, buf);

        // Show cursor
        if state.visible && !state.capture_mode {
            let cursor_x = area
                .x
                .saturating_add(prefix.len() as u16)
                .saturating_add(state.filter_cursor as u16);
            if cursor_x < area.x + area.width {
                buf[(cursor_x, area.y)]
                    .set_fg(text_primary())
                    .set_bg(bg_highlight());
            }
        }
    }

    fn render_capture_overlay(&self, area: Rect, buf: &mut Buffer, state: &KeybindingsEditorState) {
        // Clear area
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf[(x, y)].set_bg(dialog_bg());
            }
        }

        if area.height == 0 {
            return;
        }

        let item = state.capture_item_idx.and_then(|i| state.items.get(i));
        let y = area.y + area.height / 2;

        if let Some(conflict) = &state.conflict_pending {
            let target_label = item
                .map(|i| i.action_label.as_str())
                .unwrap_or("this action");

            let line1 = format!(
                "\"{}\" is already used by \"{}\"",
                conflict.key_str, conflict.conflicting_label
            );
            Paragraph::new(Span::styled(
                truncate_to_width(&line1, area.width as usize),
                Style::default()
                    .fg(accent_error())
                    .add_modifier(Modifier::BOLD),
            ))
            .render(
                Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: 1,
                },
                buf,
            );

            if y + 2 < area.y + area.height {
                let line2 = format!(
                    "Enter to reassign to \"{target_label}\", or Esc to choose a different key"
                );
                Paragraph::new(Span::styled(
                    truncate_to_width(&line2, area.width as usize),
                    Style::default().fg(text_primary()),
                ))
                .render(
                    Rect {
                        x: area.x,
                        y: y + 2,
                        width: area.width,
                        height: 1,
                    },
                    buf,
                );
            }
        } else {
            let prompt = if let Some(item) = item {
                format!(
                    "Press the new key for: {}  ({})",
                    item.action_label, item.context_label
                )
            } else {
                "Press the new key combination...".to_string()
            };

            Paragraph::new(Span::styled(
                truncate_to_width(&prompt, area.width as usize),
                Style::default()
                    .fg(text_primary())
                    .add_modifier(Modifier::BOLD),
            ))
            .render(
                Rect {
                    x: area.x,
                    y,
                    width: area.width,
                    height: 1,
                },
                buf,
            );

            let has_override = item.map(|i| i.is_user_override).unwrap_or(false);
            let hint = if has_override {
                "Esc to cancel  •  ⌫ to reset to default"
            } else {
                "Esc to cancel"
            };
            if y + 1 < area.y + area.height {
                Paragraph::new(Span::styled(hint, Style::default().fg(text_muted()))).render(
                    Rect {
                        x: area.x,
                        y: y + 1,
                        width: area.width,
                        height: 1,
                    },
                    buf,
                );
            }
        }
    }

    fn render_list(&self, area: Rect, buf: &mut Buffer, state: &KeybindingsEditorState) {
        if area.height == 0 {
            return;
        }

        // Clear
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf[(x, y)].set_bg(dialog_bg());
            }
        }

        if state.display_rows.is_empty() {
            Paragraph::new("No bindings match your filter.")
                .style(Style::default().fg(text_muted()))
                .render(area, buf);
            return;
        }

        let visible = area.height as usize;
        let total_rows = state.display_rows.len();
        let has_scrollbar = total_rows > visible;
        let content_width = if has_scrollbar {
            area.width.saturating_sub(1) as usize
        } else {
            area.width as usize
        };

        let start = state.scroll_offset;
        let end = (start + visible).min(total_rows);

        let selected_bg = ensure_contrast_bg(bg_highlight(), dialog_bg(), 2.0);
        let selected_fg = ensure_contrast_fg(text_primary(), selected_bg, 4.5);
        let selected_muted = ensure_contrast_fg(text_muted(), selected_bg, 2.5);
        let selected_secondary = ensure_contrast_fg(text_secondary(), selected_bg, 3.0);

        for (row_offset, row_idx) in (start..end).enumerate() {
            let y = area.y + row_offset as u16;
            if y >= area.y + area.height {
                break;
            }

            match &state.display_rows[row_idx] {
                DisplayRow::Header(label) => {
                    // Section header — muted, bold, full width
                    for x in area.x..area.x + area.width {
                        buf[(x, y)].set_bg(dialog_bg());
                    }
                    let header_text = truncate_to_width(label, content_width);
                    Paragraph::new(Span::styled(
                        header_text,
                        Style::default()
                            .fg(text_muted())
                            .add_modifier(Modifier::BOLD),
                    ))
                    .render(
                        Rect {
                            x: area.x,
                            y,
                            width: area.width,
                            height: 1,
                        },
                        buf,
                    );
                }
                DisplayRow::Item(item_idx) => {
                    let Some(item) = state.items.get(*item_idx) else {
                        continue;
                    };
                    let is_selected = row_idx == state.selected_row;
                    let bg = if is_selected {
                        selected_bg
                    } else {
                        dialog_bg()
                    };

                    for x in area.x..area.x + area.width {
                        buf[(x, y)].set_bg(bg);
                    }

                    // Layout: [prefix(3)] [override_marker(1)] [action_label] [gap] [key]
                    let prefix_str = if is_selected { " \u{25b8} " } else { "   " };
                    let prefix_fg = if is_selected { accent_primary() } else { bg };

                    let override_marker = if item.is_user_override { "*" } else { " " };
                    let override_fg = if item.is_user_override {
                        accent_primary()
                    } else {
                        bg
                    };

                    // Build key column: current key, and if overridden, show default dimmed
                    let current = truncate_to_width(&item.current_key, 18);
                    let default_hint = if item.is_user_override {
                        let hint = format!(" ({})", item.default_key);
                        truncate_to_width(&hint, 14)
                    } else {
                        String::new()
                    };
                    let key_col_len = current.len() + default_hint.len();

                    let used = 3 + 1 + key_col_len + 2; // prefix + marker + key + gap
                    let label_width = content_width.saturating_sub(used);
                    let label = truncate_to_width(&item.action_label, label_width);
                    let label_len = label.len();
                    let gap = content_width
                        .saturating_sub(3 + 1 + label_len + key_col_len)
                        .max(1);

                    let action_fg = if is_selected {
                        selected_fg
                    } else if item.is_editable() {
                        text_primary()
                    } else {
                        text_muted()
                    };
                    let key_fg = if is_selected {
                        selected_secondary
                    } else {
                        text_secondary()
                    };
                    let default_hint_fg = if is_selected {
                        selected_muted
                    } else {
                        text_muted()
                    };

                    let mut spans = vec![
                        Span::styled(prefix_str, Style::default().fg(prefix_fg).bg(bg)),
                        Span::styled(override_marker, Style::default().fg(override_fg).bg(bg)),
                        Span::styled(label, Style::default().fg(action_fg).bg(bg)),
                        Span::styled(" ".repeat(gap), Style::default().bg(bg)),
                        Span::styled(
                            current,
                            Style::default()
                                .fg(key_fg)
                                .bg(bg)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ];
                    if !default_hint.is_empty() {
                        spans.push(Span::styled(
                            default_hint,
                            Style::default().fg(default_hint_fg).bg(bg),
                        ));
                    }
                    let line = Line::from(spans);

                    Paragraph::new(line).render(
                        Rect {
                            x: area.x,
                            y,
                            width: area.width,
                            height: 1,
                        },
                        buf,
                    );

                    if is_selected && !item.is_editable() {
                        let note = " (read-only)";
                        let nx = area.x + area.width.saturating_sub(note.len() as u16);
                        Paragraph::new(Span::styled(
                            note,
                            Style::default().fg(selected_muted).bg(bg),
                        ))
                        .render(
                            Rect {
                                x: nx,
                                y,
                                width: note.len() as u16,
                                height: 1,
                            },
                            buf,
                        );
                    }
                }
            }
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
                total_rows,
                visible,
                state.scroll_offset,
            );
        }
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer, state: &KeybindingsEditorState) {
        // Clear
        for x in area.x..area.x + area.width {
            buf[(x, area.y)].set_bg(dialog_bg());
        }

        if let Some(msg) = &state.status_message {
            let text = truncate_to_width(msg, area.width as usize);
            Paragraph::new(Span::styled(text, Style::default().fg(accent_primary())))
                .render(area, buf);
        } else {
            // Show item count
            let count = state
                .display_rows
                .iter()
                .filter(|r| matches!(r, DisplayRow::Item(_)))
                .count();
            let total = state.items.len();
            let text = if state.filter.is_empty() {
                format!("{} bindings", total)
            } else {
                format!("{}/{} bindings", count, total)
            };
            Paragraph::new(Span::styled(text, Style::default().fg(text_muted()))).render(area, buf);
        }
    }
}

impl Default for KeybindingsEditor {
    fn default() -> Self {
        Self::new()
    }
}
