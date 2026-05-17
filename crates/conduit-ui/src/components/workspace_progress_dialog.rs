//! Dialog that streams git progress while a workspace is being created,
//! then presents a configuration panel before opening the workspace.

use conduit_agent::{AgentMode, AgentType};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use super::dialog::DialogFrame;
use super::theme::{accent_primary, text_muted, text_primary};

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

const DIALOG_WIDTH: u16 = 68;
const MAX_VISIBLE_PICKER_ITEMS: usize = 10;

/// An inline picker rendered within the workspace config panel.
#[derive(Debug, Clone)]
pub struct InlinePickerState {
    pub title: String,
    pub items: Vec<(String, String)>,
    pub selected: usize,
    pub scroll_offset: usize,
}

impl InlinePickerState {
    pub fn new(
        title: impl Into<String>,
        items: Vec<(String, String)>,
        current_id: Option<&str>,
    ) -> Self {
        let selected = current_id
            .and_then(|id| items.iter().position(|(item_id, _)| item_id == id))
            .unwrap_or(0);
        Self {
            title: title.into(),
            items,
            selected,
            scroll_offset: 0,
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
            if self.selected >= self.scroll_offset + MAX_VISIBLE_PICKER_ITEMS {
                self.scroll_offset = self.selected + 1 - MAX_VISIBLE_PICKER_ITEMS;
            }
        }
    }

    pub fn selected_item(&self) -> Option<&(String, String)> {
        self.items.get(self.selected)
    }
}

/// Which config field an inline picker is editing.
#[derive(Debug, Clone, PartialEq)]
pub enum InlinePickerTarget {
    AgentCli,
    Model,
    Mode,
    Orchestration,
    AdversarialReview,
}

const LOG_LINES: usize = 10;

// Config panel row indices
const ROW_PROVIDER: usize = 0;
const ROW_MODEL: usize = 1;
const ROW_MODE: usize = 2;
const ROW_ORCHESTRATION: usize = 3;
const ROW_ADVERSARIAL_REVIEW: usize = 4;
const ROW_ADVERSARIAL_MODEL: usize = 5;
const ROW_SAVE_DEFAULT: usize = 6;
const ROW_CONTINUE: usize = 7;
const ROW_COUNT: usize = 8;

/// Inline configuration shown after successful workspace creation.
#[derive(Debug, Clone)]
pub struct WorkspaceReadyConfigState {
    pub focused_row: usize,
    pub provider: AgentType,
    pub model_id: String,
    pub mode: AgentMode,
    pub orchestration_enabled: bool,
    pub adversarial_review_enabled: bool,
    pub adversarial_review_model: String,
    pub save_as_project_default: bool,
    pub active_picker: Option<(InlinePickerTarget, InlinePickerState)>,
}

impl WorkspaceReadyConfigState {
    pub fn new(
        provider: AgentType,
        model_id: String,
        orchestration_enabled: bool,
        adversarial_review_enabled: bool,
        adversarial_review_model: String,
    ) -> Self {
        Self {
            focused_row: ROW_CONTINUE,
            provider,
            model_id,
            mode: AgentMode::Build,
            orchestration_enabled,
            adversarial_review_enabled,
            adversarial_review_model,
            save_as_project_default: false,
            active_picker: None,
        }
    }

    pub fn is_orchestration_applicable(&self) -> bool {
        self.provider == AgentType::Claude
    }

    pub fn is_plan_mode_applicable(&self) -> bool {
        self.provider.supports_plan_mode()
    }
}

/// State for the workspace creation progress dialog.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceProgressDialogState {
    pub visible: bool,
    /// All progress messages received so far.
    pub messages: Vec<String>,
    /// Whether the creation has finished (success or failure).
    pub complete: bool,
    /// Set to the error string if creation failed.
    pub error: Option<String>,
    /// Spinner animation frame (advanced on each Tick while not complete).
    pub spinner_frame: usize,
    /// Config panel shown after successful creation.
    pub config: Option<WorkspaceReadyConfigState>,
}

impl WorkspaceProgressDialogState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.complete = false;
        self.error = None;
        self.messages.clear();
        self.spinner_frame = 0;
        self.config = None;
    }

    pub fn push(&mut self, message: impl Into<String>) {
        self.messages.push(message.into());
    }

    /// Transition to the success state and show the config panel.
    pub fn finish(
        &mut self,
        provider: AgentType,
        model_id: String,
        orchestration_enabled: bool,
        adversarial_review_enabled: bool,
        adversarial_review_model: String,
    ) {
        self.complete = true;
        self.config = Some(WorkspaceReadyConfigState::new(
            provider,
            model_id,
            orchestration_enabled,
            adversarial_review_enabled,
            adversarial_review_model,
        ));
    }

    pub fn finish_with_error(&mut self, error: impl Into<String>) {
        self.complete = true;
        self.error = Some(error.into());
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.config = None;
    }

    pub fn tick(&mut self) {
        if !self.complete {
            self.spinner_frame = self.spinner_frame.wrapping_add(1);
        }
    }

    pub fn failed(&self) -> bool {
        self.error.is_some()
    }

    // ── Config panel navigation helpers ──────────────────────────────────────

    pub fn move_focus_up(&mut self) {
        if let Some(cfg) = &mut self.config {
            cfg.focused_row = if cfg.focused_row == 0 {
                ROW_COUNT - 1
            } else {
                cfg.focused_row - 1
            };
        }
    }

    pub fn move_focus_down(&mut self) {
        if let Some(cfg) = &mut self.config {
            cfg.focused_row = (cfg.focused_row + 1) % ROW_COUNT;
        }
    }

    pub fn focused_row(&self) -> usize {
        self.config
            .as_ref()
            .map(|c| c.focused_row)
            .unwrap_or(ROW_CONTINUE)
    }

    pub fn toggle_mode(&mut self) {
        if let Some(cfg) = &mut self.config {
            if cfg.is_plan_mode_applicable() {
                cfg.mode = match cfg.mode {
                    AgentMode::Build => AgentMode::Plan,
                    AgentMode::Plan => AgentMode::Build,
                };
            }
        }
    }

    pub fn toggle_orchestration(&mut self) {
        if let Some(cfg) = &mut self.config {
            if cfg.is_orchestration_applicable() {
                cfg.orchestration_enabled = !cfg.orchestration_enabled;
            }
        }
    }

    pub fn toggle_adversarial_review(&mut self) {
        if let Some(cfg) = &mut self.config {
            if cfg.is_orchestration_applicable() {
                cfg.adversarial_review_enabled = !cfg.adversarial_review_enabled;
            }
        }
    }

    pub fn update_adversarial_model(&mut self, model_id: String) {
        if let Some(cfg) = &mut self.config {
            cfg.adversarial_review_model = model_id;
        }
    }

    pub fn toggle_save_default(&mut self) {
        if let Some(cfg) = &mut self.config {
            cfg.save_as_project_default = !cfg.save_as_project_default;
        }
    }

    pub fn update_provider(&mut self, provider: AgentType, default_model: String) {
        if let Some(cfg) = &mut self.config {
            cfg.provider = provider;
            cfg.model_id = default_model;
            if !cfg.is_orchestration_applicable() {
                cfg.orchestration_enabled = false;
                cfg.adversarial_review_enabled = false;
            }
            if !cfg.is_plan_mode_applicable() {
                cfg.mode = AgentMode::Build;
            }
        }
    }

    pub fn update_model(&mut self, model_id: String) {
        if let Some(cfg) = &mut self.config {
            cfg.model_id = model_id;
        }
    }

    pub fn open_provider_picker(&mut self, items: Vec<(String, String)>) {
        if let Some(cfg) = &mut self.config {
            let current_id = cfg.provider.as_str().to_string();
            cfg.active_picker = Some((
                InlinePickerTarget::AgentCli,
                InlinePickerState::new("Select Agent CLI", items, Some(&current_id)),
            ));
        }
    }

    pub fn open_model_picker(&mut self, items: Vec<(String, String)>) {
        if let Some(cfg) = &mut self.config {
            let current_id = cfg.model_id.clone();
            cfg.active_picker = Some((
                InlinePickerTarget::Model,
                InlinePickerState::new("Select Model", items, Some(&current_id)),
            ));
        }
    }

    pub fn open_mode_picker(&mut self) {
        if let Some(cfg) = &mut self.config {
            if !cfg.is_plan_mode_applicable() {
                return;
            }
            let current = cfg.mode.as_str().to_string();
            let items = vec![
                ("build".to_string(), "Build".to_string()),
                ("plan".to_string(), "Plan".to_string()),
            ];
            cfg.active_picker = Some((
                InlinePickerTarget::Mode,
                InlinePickerState::new("Select Mode", items, Some(&current)),
            ));
        }
    }

    pub fn open_orchestration_picker(&mut self) {
        if let Some(cfg) = &mut self.config {
            if !cfg.is_orchestration_applicable() {
                return;
            }
            let current = if cfg.orchestration_enabled {
                "on"
            } else {
                "off"
            };
            let items = vec![
                ("off".to_string(), "Off".to_string()),
                ("on".to_string(), "On".to_string()),
            ];
            cfg.active_picker = Some((
                InlinePickerTarget::Orchestration,
                InlinePickerState::new("Orchestration", items, Some(current)),
            ));
        }
    }

    pub fn open_adversarial_review_picker(&mut self) {
        if let Some(cfg) = &mut self.config {
            if !cfg.is_orchestration_applicable() {
                return;
            }
            let current = if cfg.adversarial_review_enabled {
                "on"
            } else {
                "off"
            };
            let items = vec![
                ("off".to_string(), "Off".to_string()),
                ("on".to_string(), "On".to_string()),
            ];
            cfg.active_picker = Some((
                InlinePickerTarget::AdversarialReview,
                InlinePickerState::new("Adversarial Review", items, Some(current)),
            ));
        }
    }

    pub fn has_active_picker(&self) -> bool {
        self.config
            .as_ref()
            .is_some_and(|c| c.active_picker.is_some())
    }

    pub fn picker_move_up(&mut self) {
        if let Some(cfg) = &mut self.config {
            if let Some((_, picker)) = &mut cfg.active_picker {
                picker.move_up();
            }
        }
    }

    pub fn picker_move_down(&mut self) {
        if let Some(cfg) = &mut self.config {
            if let Some((_, picker)) = &mut cfg.active_picker {
                picker.move_down();
            }
        }
    }

    pub fn close_active_picker(&mut self) {
        if let Some(cfg) = &mut self.config {
            cfg.active_picker = None;
        }
    }

    /// Consume the active picker and return its target + selected id.
    pub fn take_picker_selection(&mut self) -> Option<(InlinePickerTarget, String)> {
        if let Some(cfg) = &mut self.config {
            if let Some((target, picker)) = cfg.active_picker.take() {
                let id = picker.selected_item().map(|(id, _)| id.clone())?;
                return Some((target, id));
            }
        }
        None
    }

    pub fn update_mode(&mut self, mode: AgentMode) {
        if let Some(cfg) = &mut self.config {
            cfg.mode = mode;
        }
    }

    pub fn update_orchestration_enabled(&mut self, enabled: bool) {
        if let Some(cfg) = &mut self.config {
            cfg.orchestration_enabled = enabled;
        }
    }

    pub fn update_adversarial_review_enabled(&mut self, enabled: bool) {
        if let Some(cfg) = &mut self.config {
            cfg.adversarial_review_enabled = enabled;
        }
    }
}

/// Widget that renders the workspace creation progress dialog.
pub struct WorkspaceProgressDialog<'a> {
    state: &'a WorkspaceProgressDialogState,
}

impl<'a> WorkspaceProgressDialog<'a> {
    pub fn new(state: &'a WorkspaceProgressDialogState) -> Self {
        Self { state }
    }

    fn dialog_height(&self) -> u16 {
        if self.state.config.is_some() {
            // borders(2) + top_padding(1) + log_lines(10) + gap(1) + status(1)
            // + separator(1) + gap(1) + 7 config rows + gap(1) + button(1) + gap(1)
            // = 28
            28
        } else if self.state.complete {
            // borders(2) + top_padding(1) + log_lines(10) + gap(1) + status(1) + gap(1) + button(1)
            17
        } else {
            // no button row
            15
        }
    }
}

impl Widget for WorkspaceProgressDialog<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.state.visible {
            return;
        }

        let instructions = if self.state.config.is_some() {
            if self.state.has_active_picker() {
                vec![("↑↓", "Navigate"), ("Enter", "Select"), ("Esc", "Cancel")]
            } else {
                vec![
                    ("↑↓", "Navigate"),
                    ("Enter", "Continue"),
                    ("Esc", "Continue"),
                ]
            }
        } else if self.state.complete {
            vec![("Enter", "Continue"), ("Esc", "Continue")]
        } else {
            vec![]
        };

        let border_color = if self.state.failed() {
            Color::Red
        } else if self.state.complete {
            Color::Green
        } else {
            accent_primary()
        };

        let inner = DialogFrame::new("Creating Workspace", DIALOG_WIDTH, self.dialog_height())
            .border_color(border_color)
            .instructions(instructions)
            .render(area, buf);

        if inner.height == 0 {
            return;
        }

        // ── Progress log ──────────────────────────────────────────────────────
        let visible: Vec<&str> = {
            let msgs = &self.state.messages;
            if msgs.len() > LOG_LINES {
                msgs[msgs.len() - LOG_LINES..]
                    .iter()
                    .map(String::as_str)
                    .collect()
            } else {
                msgs.iter().map(String::as_str).collect()
            }
        };

        for (i, msg) in visible.iter().enumerate() {
            let y = inner.y + i as u16;
            if y >= inner.y + inner.height {
                break;
            }
            let line = Line::from(Span::styled(*msg, Style::default().fg(text_primary())));
            Paragraph::new(line).render(
                Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                },
                buf,
            );
        }

        // ── Status line ───────────────────────────────────────────────────────
        let status_y = inner.y + LOG_LINES as u16 + 1;
        if status_y < inner.y + inner.height {
            let status_line = if self.state.complete {
                if let Some(ref err) = self.state.error {
                    let truncated = if err.len() > DIALOG_WIDTH as usize - 4 {
                        format!("{}...", &err[..DIALOG_WIDTH as usize - 7])
                    } else {
                        err.clone()
                    };
                    Line::from(Span::styled(
                        format!("✗ {}", truncated),
                        Style::default().fg(Color::Red),
                    ))
                } else {
                    Line::from(Span::styled(
                        "✓ Workspace created",
                        Style::default().fg(Color::Green),
                    ))
                }
            } else {
                let frame = SPINNER_FRAMES[self.state.spinner_frame % SPINNER_FRAMES.len()];
                Line::from(vec![
                    Span::styled(format!("{} ", frame), Style::default().fg(accent_primary())),
                    Span::styled("Working...", Style::default().fg(text_muted())),
                ])
            };
            Paragraph::new(status_line).render(
                Rect {
                    x: inner.x,
                    y: status_y,
                    width: inner.width,
                    height: 1,
                },
                buf,
            );
        }

        // ── Config panel (success only) ───────────────────────────────────────
        if let Some(cfg) = &self.state.config {
            let sep_y = status_y + 1;
            if sep_y < inner.y + inner.height {
                // Separator line
                let sep = "─".repeat(inner.width as usize);
                Paragraph::new(Line::from(Span::styled(
                    sep,
                    Style::default().fg(text_muted()),
                )))
                .render(
                    Rect {
                        x: inner.x,
                        y: sep_y,
                        width: inner.width,
                        height: 1,
                    },
                    buf,
                );

                let rows_start_y = sep_y + 1;

                // ── Inline picker ─────────────────────────────────────────────
                if let Some((_, picker)) = cfg.active_picker.as_ref() {
                    // Title
                    let title_y = rows_start_y;
                    if title_y < inner.y + inner.height {
                        Paragraph::new(Line::from(Span::styled(
                            picker.title.as_str(),
                            Style::default()
                                .fg(accent_primary())
                                .add_modifier(Modifier::BOLD),
                        )))
                        .render(
                            Rect {
                                x: inner.x,
                                y: title_y,
                                width: inner.width,
                                height: 1,
                            },
                            buf,
                        );
                    }

                    // Items
                    let items_start_y = rows_start_y + 1;
                    let visible_end =
                        (picker.scroll_offset + MAX_VISIBLE_PICKER_ITEMS).min(picker.items.len());
                    for (i, (_, display_name)) in picker.items[picker.scroll_offset..visible_end]
                        .iter()
                        .enumerate()
                    {
                        let actual_idx = picker.scroll_offset + i;
                        let is_selected = actual_idx == picker.selected;
                        let y = items_start_y + i as u16;
                        if y >= inner.y + inner.height {
                            break;
                        }
                        let (prefix, style) = if is_selected {
                            (
                                "› ",
                                Style::default()
                                    .fg(accent_primary())
                                    .add_modifier(Modifier::BOLD),
                            )
                        } else {
                            ("  ", Style::default().fg(text_primary()))
                        };
                        Paragraph::new(Line::from(Span::styled(
                            format!("{}{}", prefix, display_name),
                            style,
                        )))
                        .render(
                            Rect {
                                x: inner.x,
                                y,
                                width: inner.width,
                                height: 1,
                            },
                            buf,
                        );
                    }

                    // "↓ more" indicator when items are hidden below
                    if visible_end < picker.items.len() {
                        let indicator_y = items_start_y + MAX_VISIBLE_PICKER_ITEMS as u16;
                        if indicator_y < inner.y + inner.height {
                            Paragraph::new(Line::from(Span::styled(
                                "  ↓ more",
                                Style::default().fg(text_muted()),
                            )))
                            .render(
                                Rect {
                                    x: inner.x,
                                    y: indicator_y,
                                    width: inner.width,
                                    height: 1,
                                },
                                buf,
                            );
                        }
                    }
                } else {
                    // ── Config rows ───────────────────────────────────────────────
                    const LABEL_WIDTH: u16 = 20;

                    // Renders one config row: label left-aligned, value right-aligned.
                    let render_row = |buf: &mut Buffer,
                                      row_idx: usize,
                                      y: u16,
                                      label: &str,
                                      value_spans: Vec<Span>| {
                        if y >= inner.y + inner.height {
                            return;
                        }
                        let is_focused = cfg.focused_row == row_idx;
                        let label_style = if is_focused {
                            Style::default()
                                .fg(Color::Black)
                                .bg(accent_primary())
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(text_primary())
                        };
                        let padded = format!("{:<width$}", label, width = LABEL_WIDTH as usize);
                        Paragraph::new(Line::from(Span::styled(padded, label_style))).render(
                            Rect {
                                x: inner.x,
                                y,
                                width: LABEL_WIDTH.min(inner.width),
                                height: 1,
                            },
                            buf,
                        );
                        const RIGHT_MARGIN: u16 = 6;
                        let value_area_x = inner.x + LABEL_WIDTH;
                        let value_area_w = inner
                            .width
                            .saturating_sub(LABEL_WIDTH)
                            .saturating_sub(RIGHT_MARGIN);
                        if value_area_w > 0 {
                            Paragraph::new(Line::from(value_spans))
                                .alignment(Alignment::Right)
                                .render(
                                    Rect {
                                        x: value_area_x,
                                        y,
                                        width: value_area_w,
                                        height: 1,
                                    },
                                    buf,
                                );
                        }
                    };

                    // ROW 0: Agent CLI
                    {
                        let provider_name = format!("{:?}", cfg.provider);
                        let value_spans = vec![Span::styled(
                            provider_name,
                            Style::default().fg(text_primary()),
                        )];
                        render_row(buf, ROW_PROVIDER, rows_start_y, "Agent CLI", value_spans);
                    }

                    // ROW 1: Model
                    {
                        let max_model_len =
                            (inner.width as usize).saturating_sub(LABEL_WIDTH as usize + 1);
                        let model_display = if cfg.model_id.len() > max_model_len {
                            format!("{}…", &cfg.model_id[..max_model_len.saturating_sub(1)])
                        } else {
                            cfg.model_id.clone()
                        };
                        let value_spans = vec![Span::styled(
                            model_display,
                            Style::default().fg(text_primary()),
                        )];
                        render_row(buf, ROW_MODEL, rows_start_y + 1, "Model", value_spans);
                    }

                    // ROW 2: Mode
                    {
                        let plan_applicable = cfg.is_plan_mode_applicable();
                        let (build_style, plan_style) = match cfg.mode {
                            AgentMode::Build => (
                                Style::default()
                                    .fg(Color::Black)
                                    .bg(Color::Green)
                                    .add_modifier(Modifier::BOLD),
                                if plan_applicable {
                                    Style::default().fg(text_muted())
                                } else {
                                    Style::default()
                                        .fg(text_muted())
                                        .add_modifier(Modifier::DIM)
                                },
                            ),
                            AgentMode::Plan => (
                                Style::default().fg(text_muted()),
                                Style::default()
                                    .fg(Color::Black)
                                    .bg(Color::Green)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        };
                        let value_spans = vec![
                            Span::styled("[ Build ]", build_style),
                            Span::raw("  "),
                            Span::styled("[ Plan ]", plan_style),
                        ];
                        render_row(buf, ROW_MODE, rows_start_y + 2, "Mode", value_spans);
                    }

                    // ROW 3: Orchestration
                    {
                        let orch_applicable = cfg.is_orchestration_applicable();
                        let row_color = if orch_applicable {
                            text_primary()
                        } else {
                            text_muted()
                        };
                        let (off_style, on_style) = if orch_applicable {
                            match cfg.orchestration_enabled {
                                false => (
                                    Style::default()
                                        .fg(Color::Black)
                                        .bg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD),
                                    Style::default().fg(row_color),
                                ),
                                true => (
                                    Style::default().fg(row_color),
                                    Style::default()
                                        .fg(Color::Black)
                                        .bg(Color::Green)
                                        .add_modifier(Modifier::BOLD),
                                ),
                            }
                        } else {
                            let dim = Style::default()
                                .fg(text_muted())
                                .add_modifier(Modifier::DIM);
                            (dim, dim)
                        };
                        let value_spans = vec![
                            Span::styled("[ Off ]", off_style),
                            Span::raw("  "),
                            Span::styled("[ On ]", on_style),
                        ];
                        render_row(
                            buf,
                            ROW_ORCHESTRATION,
                            rows_start_y + 3,
                            "Orchestration",
                            value_spans,
                        );
                    }

                    // ROW 4: Adversarial Review toggle
                    {
                        let ar_applicable = cfg.is_orchestration_applicable();
                        let row_color = if ar_applicable {
                            text_primary()
                        } else {
                            text_muted()
                        };
                        let (off_style, on_style) = if ar_applicable {
                            match cfg.adversarial_review_enabled {
                                false => (
                                    Style::default()
                                        .fg(Color::Black)
                                        .bg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD),
                                    Style::default().fg(row_color),
                                ),
                                true => (
                                    Style::default().fg(row_color),
                                    Style::default()
                                        .fg(Color::Black)
                                        .bg(Color::Green)
                                        .add_modifier(Modifier::BOLD),
                                ),
                            }
                        } else {
                            let dim = Style::default()
                                .fg(text_muted())
                                .add_modifier(Modifier::DIM);
                            (dim, dim)
                        };
                        let value_spans = vec![
                            Span::styled("[ Off ]", off_style),
                            Span::raw("  "),
                            Span::styled("[ On ]", on_style),
                        ];
                        render_row(
                            buf,
                            ROW_ADVERSARIAL_REVIEW,
                            rows_start_y + 4,
                            "Adversarial Review",
                            value_spans,
                        );
                    }

                    // ROW 5: Adversarial Review model
                    {
                        let ar_applicable =
                            cfg.is_orchestration_applicable() && cfg.adversarial_review_enabled;
                        let max_model_len =
                            (inner.width as usize).saturating_sub(LABEL_WIDTH as usize + 1);
                        let model_display = if cfg.adversarial_review_model.len() > max_model_len {
                            format!(
                                "{}…",
                                &cfg.adversarial_review_model[..max_model_len.saturating_sub(1)]
                            )
                        } else {
                            cfg.adversarial_review_model.clone()
                        };
                        let style = if ar_applicable {
                            Style::default().fg(text_primary())
                        } else {
                            Style::default()
                                .fg(text_muted())
                                .add_modifier(Modifier::DIM)
                        };
                        let value_spans = vec![Span::styled(model_display, style)];
                        render_row(
                            buf,
                            ROW_ADVERSARIAL_MODEL,
                            rows_start_y + 5,
                            "  Review Model",
                            value_spans,
                        );
                    }

                    // ROW 6: Save as project default
                    {
                        let checkbox = if cfg.save_as_project_default {
                            "[x]"
                        } else {
                            "[ ]"
                        };
                        let is_focused = cfg.focused_row == ROW_SAVE_DEFAULT;
                        let style = if is_focused {
                            Style::default().fg(accent_primary())
                        } else {
                            Style::default().fg(text_primary())
                        };
                        let y = rows_start_y + 6;
                        if y < inner.y + inner.height {
                            Paragraph::new(Line::from(vec![
                                Span::styled(checkbox, style),
                                Span::styled(" Set as project default", style),
                            ]))
                            .render(
                                Rect {
                                    x: inner.x,
                                    y,
                                    width: inner.width,
                                    height: 1,
                                },
                                buf,
                            );
                        }
                    }

                    // Continue button
                    let button_y = rows_start_y + 8;
                    if button_y < inner.y + inner.height {
                        let continue_focused = cfg.focused_row == ROW_CONTINUE;
                        let button_style = if continue_focused {
                            Style::default()
                                .fg(Color::Black)
                                .bg(accent_primary())
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Green)
                                .add_modifier(Modifier::BOLD)
                        };
                        let button = Span::styled(" Continue ", button_style);
                        Paragraph::new(Line::from(button))
                            .alignment(Alignment::Center)
                            .render(
                                Rect {
                                    x: inner.x,
                                    y: button_y,
                                    width: inner.width,
                                    height: 1,
                                },
                                buf,
                            );
                    }
                } // end else (config rows)
            }
        } else if self.state.complete {
            // Error path: bare Continue button
            let button_y = status_y + 2;
            if button_y < inner.y + inner.height {
                let button = Span::styled(
                    " Continue ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                );
                Paragraph::new(Line::from(button))
                    .alignment(Alignment::Center)
                    .render(
                        Rect {
                            x: inner.x,
                            y: button_y,
                            width: inner.width,
                            height: 1,
                        },
                        buf,
                    );
            }
        }
    }
}
