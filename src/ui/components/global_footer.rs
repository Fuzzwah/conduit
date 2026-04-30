use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
};

use super::KnightRiderSpinner;
use crate::config::{default_keybindings, KeybindingConfig};
use crate::ui::action::Action;
use crate::ui::components::{render_key_hints_responsive, text_muted, KeyHintBarStyle};
use crate::ui::events::{InputMode, ViewMode};

/// Context for determining which footer hints to show
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FooterContext {
    /// Empty state - no tabs open
    Empty,
    /// Normal chat mode with tabs
    #[default]
    Chat,
    /// Sidebar navigation mode
    Sidebar,
    /// Raw events view mode
    RawEvents,
    /// File viewer mode
    FileViewer,
}

impl FooterContext {
    /// Determine footer context from view mode, input mode, and whether tabs exist
    pub fn from_state(view_mode: ViewMode, input_mode: InputMode, has_tabs: bool) -> Self {
        if !has_tabs {
            return FooterContext::Empty;
        }

        match view_mode {
            ViewMode::RawEvents => FooterContext::RawEvents,
            ViewMode::Chat => {
                if input_mode == InputMode::SidebarNavigation {
                    FooterContext::Sidebar
                } else if input_mode == InputMode::FileViewer {
                    FooterContext::FileViewer
                } else {
                    FooterContext::Chat
                }
            }
        }
    }
}

/// Reverse-lookup: find the key string bound to `action` in the global bindings of `config`.
fn lookup_global_key(config: &KeybindingConfig, action: &Action) -> Option<String> {
    config
        .global
        .iter()
        .find(|(_, a)| *a == action)
        .map(|(k, _)| k.to_string())
}

/// Return the key string for `action`, falling back to the compiled-in default if not found.
fn key_for(config: &KeybindingConfig, action: &Action, fallback: &'static str) -> String {
    lookup_global_key(config, action).unwrap_or_else(|| fallback.to_string())
}

/// Global footer showing keyboard shortcuts in minimal style
/// Layout: [Spinner][Message]                    [Key Hints (right-aligned)]
pub struct GlobalFooter<'a> {
    hints: Vec<(String, &'static str)>,
    spinner: Option<&'a KnightRiderSpinner>,
    message: Option<&'a str>,
}

impl<'a> GlobalFooter<'a> {
    pub fn new() -> Self {
        Self {
            hints: Self::chat_hints(),
            spinner: None,
            message: None,
        }
    }

    /// Create footer for a specific context using default keybindings.
    pub fn for_context(context: FooterContext) -> Self {
        let defaults = default_keybindings();
        Self::for_context_with_config(context, &defaults)
    }

    /// Create footer for a specific context using the live keybinding config.
    pub fn for_context_with_config(context: FooterContext, config: &KeybindingConfig) -> Self {
        Self {
            hints: match context {
                FooterContext::Empty => Self::empty_hints_with_config(config),
                FooterContext::Chat => Self::chat_hints_with_config(config),
                FooterContext::Sidebar => Self::sidebar_hints(),
                FooterContext::RawEvents => Self::raw_events_hints(),
                FooterContext::FileViewer => Self::file_viewer_hints(),
            },
            spinner: None,
            message: None,
        }
    }

    /// Create footer for file viewer context
    pub fn file_viewer_context() -> Self {
        Self::for_context(FooterContext::FileViewer)
    }

    /// Create footer from app state using default keybindings.
    pub fn from_state(view_mode: ViewMode, input_mode: InputMode, has_tabs: bool) -> Self {
        let context = FooterContext::from_state(view_mode, input_mode, has_tabs);
        Self::for_context(context)
    }

    /// Create footer from app state using the live keybinding config.
    pub fn from_state_with_config(
        view_mode: ViewMode,
        input_mode: InputMode,
        has_tabs: bool,
        config: &KeybindingConfig,
    ) -> Self {
        let context = FooterContext::from_state(view_mode, input_mode, has_tabs);
        Self::for_context_with_config(context, config)
    }

    /// Set spinner for left side of footer
    pub fn with_spinner(mut self, spinner: Option<&'a KnightRiderSpinner>) -> Self {
        self.spinner = spinner;
        self
    }

    /// Set message for left side of footer (after spinner)
    pub fn with_message(mut self, message: Option<&'a str>) -> Self {
        self.message = message;
        self
    }

    /// Hints for empty state using default keybindings.
    pub fn empty_hints() -> Vec<(String, &'static str)> {
        Self::empty_hints_with_config(&default_keybindings())
    }

    /// Return the hints this footer would display (for click-handling).
    pub fn hints(&self) -> &[(String, &'static str)] {
        &self.hints
    }

    fn empty_hints_with_config(config: &KeybindingConfig) -> Vec<(String, &'static str)> {
        vec![
            (key_for(config, &Action::NewProject, "C-n"), "new project"),
            (key_for(config, &Action::ToggleSidebar, "C-t"), "sidebar"),
            (
                key_for(config, &Action::ImportSession, "M-i"),
                "import session",
            ),
            (key_for(config, &Action::Quit, "C-q"), "quit"),
        ]
    }

    /// Hints for chat mode using default keybindings.
    pub fn chat_hints() -> Vec<(String, &'static str)> {
        Self::chat_hints_with_config(&default_keybindings())
    }

    fn chat_hints_with_config(config: &KeybindingConfig) -> Vec<(String, &'static str)> {
        let next = key_for(config, &Action::NextTab, "M-tab");
        let prev = key_for(config, &Action::PrevTab, "M-S-tab");
        let tab_hint = format!("{next}/{prev}");
        vec![
            (tab_hint, "next/prev tab"),
            (key_for(config, &Action::ShowModelSelector, "C-o"), "model"),
            (key_for(config, &Action::ToggleSidebar, "C-t"), "sidebar"),
            (key_for(config, &Action::NewProject, "C-n"), "new project"),
            (key_for(config, &Action::CloseTab, "M-S-w"), "close"),
            (
                key_for(config, &Action::ArchiveCurrentWorkspace, "M-S-x"),
                "archive",
            ),
            (key_for(config, &Action::InterruptAgent, "C-c"), "stop"),
            (key_for(config, &Action::Quit, "C-q"), "quit"),
        ]
    }

    /// Get hints for sidebar navigation mode (not configurable via global bindings)
    pub fn sidebar_hints() -> Vec<(String, &'static str)> {
        vec![
            ("↑↓".to_string(), "navigate"),
            ("enter".to_string(), "select"),
            ("h/l".to_string(), "collapse/expand"),
            ("M-S-x".to_string(), "archive"),
            ("C-n".to_string(), "new project"),
            ("esc".to_string(), "exit"),
        ]
    }

    /// Get hints for raw events view mode
    pub fn raw_events_hints() -> Vec<(String, &'static str)> {
        vec![
            ("j/k".to_string(), "nav"),
            ("e".to_string(), "detail"),
            ("C-j/k".to_string(), "panel"),
            ("c".to_string(), "copy"),
            ("C-g".to_string(), "chat"),
        ]
    }

    /// Get hints for file viewer mode
    pub fn file_viewer_hints() -> Vec<(String, &'static str)> {
        vec![
            ("j/k".to_string(), "scroll"),
            ("g/G".to_string(), "top/bottom"),
            ("C-d/u".to_string(), "page"),
            ("M-tab/M-S-tab".to_string(), "next/prev tab"),
            ("q".to_string(), "close"),
            ("esc".to_string(), "close"),
        ]
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        // Build left side content (spinner + message)
        let mut left_spans: Vec<Span> = Vec::new();

        // Add spinner if present
        if let Some(spinner) = self.spinner {
            left_spans.push(Span::raw("  "));
            left_spans.extend(spinner.render());
        }

        // Add message if present
        if let Some(message) = self.message {
            if !left_spans.is_empty() {
                left_spans.push(Span::raw("  ")); // Gap between spinner and message
            } else {
                left_spans.push(Span::raw("  ")); // Leading space
            }
            left_spans.push(Span::styled(message, Style::default().fg(text_muted())));
        }

        // Calculate left side width
        let left_width: u16 = left_spans.iter().map(|s| s.width() as u16).sum();

        // Render left side if present
        if !left_spans.is_empty() {
            let left_line = Line::from(left_spans);
            buf.set_line(area.x, area.y, &left_line, left_width);
        }

        // Reserve space for spinner/message, key hints get the rest (right-aligned)
        let reserved_left = if left_width > 0 { left_width + 2 } else { 0 }; // +2 for gap
        let max_hints_width = area.width.saturating_sub(reserved_left);

        // Coerce to &[(&str, &str)] for the render function
        let hint_refs: Vec<(&str, &str)> =
            self.hints.iter().map(|(k, v)| (k.as_str(), *v)).collect();

        // Render key hints responsively (right-aligned, removes from left when too wide)
        render_key_hints_responsive(
            area,
            buf,
            &hint_refs,
            KeyHintBarStyle::minimal_footer(),
            Some(max_hints_width),
        );
    }
}

impl Default for GlobalFooter<'_> {
    fn default() -> Self {
        Self::new()
    }
}
