use std::fs::File;
use std::io::Write;

use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Widget,
    Frame,
};
use serde_json::json;
use unicode_width::UnicodeWidthStr;

use crate::app::App;
use crate::app_queue;
use crate::components::{
    AddRepoDialog, AgentSelector, BaseDirDialog, CommandPalette, ConfirmationDialog, ErrorDialog,
    GlobalFooter, HelpDialog, KeybindingsEditor, McpDialog, MessageRole, MissingToolDialog,
    ModelSelector, OrchestrationSelector, ProjectPicker, ProviderSelector, ReasoningSelector,
    RenameProjectDialog, SessionHeader, SessionImportPicker, SettingsMenu, Sidebar, SlashMenu,
    ThemePicker, WorkspaceDefaultsDialog,
};
use crate::events::{InputMode, ViewMode};
use crate::session::AgentSession;

impl App {
    pub(super) fn draw(&mut self, f: &mut Frame) {
        let size = f.area();
        {
            use ratatui::style::Style;
            use ratatui::widgets::{Block, Widget};

            let background =
                Block::default().style(Style::default().bg(crate::components::bg_base()));
            background.render(size, f.buffer_mut());
        }

        // Calculate sidebar width
        let sidebar_width = if self.state.sidebar_state.visible {
            36u16
        } else {
            0
        };

        // First, split horizontally for sidebar
        let (sidebar_area, right_area) = if sidebar_width > 0 {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(sidebar_width), Constraint::Min(20)])
                .split(size);
            (chunks[0], chunks[1])
        } else {
            // No sidebar - use full width
            (Rect::default(), size)
        };

        // Split right area vertically to reserve bottom row for footer
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),    // Content area (chat + status bar + gap)
                Constraint::Length(1), // Footer (only in content area)
            ])
            .split(right_area);

        let content_area = right_chunks[0];
        let footer_area = right_chunks[1];

        // Store sidebar area for mouse hit-testing
        self.state.sidebar_area = if self.state.sidebar_state.visible {
            Some(sidebar_area)
        } else {
            None
        };

        // Render sidebar if visible
        if self.state.sidebar_state.visible {
            let sidebar =
                Sidebar::new(&self.state.sidebar_data).with_spinner_frame(self.state.spinner_frame);
            ratatui::widgets::StatefulWidget::render(
                sidebar,
                sidebar_area,
                f.buffer_mut(),
                &mut self.state.sidebar_state,
            );
        }

        // Check if active tab is a file viewer - render it separately
        if self.state.tab_manager.active_is_file() {
            self.render_file_viewer_tab(content_area, footer_area, f);
        } else {
            match self.state.view_mode {
                ViewMode::Chat => {
                    // Handle empty state - no tabs open
                    if self.state.tab_manager.is_empty() {
                        use crate::components::{text_muted, FooterContext};
                        use ratatui::style::Style;
                        use ratatui::text::{Line, Span};
                        use ratatui::widgets::{Paragraph, Widget};

                        // Layout with tab bar + content (footer is rendered in reserved footer_area)
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(1), // Tab bar
                                Constraint::Min(5),    // Content area
                            ])
                            .split(content_area);

                        // Store areas for mouse hit-testing
                        self.state.tab_bar_area = Some(chunks[0]);
                        self.state.chat_area = None;
                        self.state.file_viewer_area = None;
                        self.state.raw_events_area = None;
                        self.state.input_area = None;
                        self.state.status_bar_area = None;
                        self.state.footer_area = Some(footer_area);

                        // Render tab bar
                        let tabs_focused = self.state.input_mode != InputMode::SidebarNavigation;
                        self.ensure_tab_bar_scroll(chunks[0].width, tabs_focused);
                        let tab_bar = self.build_tab_bar(tabs_focused);
                        tab_bar.render(chunks[0], f.buffer_mut());

                        // Empty state message - different for first-time users vs returning users
                        let is_first_time = self.state.show_first_time_splash;

                        // Render animated logo with shine effect
                        let mut lines = self.state.logo_shine.render_logo_lines();
                        lines.push(Line::from(""));
                        lines.push(Line::from(""));
                        lines.push(Line::from(""));

                        if is_first_time {
                            // First-time user - simpler message
                            lines.push(Line::from(Span::styled(
                                "Add your first project with Ctrl+N",
                                Style::default().fg(text_muted()),
                            )));
                        } else {
                            // Returning user - full message
                            lines.push(Line::from(Span::styled(
                                "Add a new project with Ctrl+N",
                                Style::default().fg(text_muted()),
                            )));
                            lines.push(Line::from(""));
                            lines.push(Line::from(Span::styled(
                                "- or -",
                                Style::default().fg(text_muted()),
                            )));
                            lines.push(Line::from(""));
                            lines.push(Line::from(Span::styled(
                                "Select a project from the sidebar",
                                Style::default().fg(text_muted()),
                            )));
                        }

                        let paragraph =
                            Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center);

                        // Center vertically in the content area (chunks[1])
                        let message_area = chunks[1];
                        // First-time: 7 logo + 3 blank + 1 message = 11 lines
                        // Returning: 7 logo + 3 blank + 5 message = 15 lines
                        let text_height = if is_first_time { 11u16 } else { 15u16 };
                        let vertical_offset = message_area.height.saturating_sub(text_height) / 2;
                        let centered_area = Rect {
                            x: message_area.x,
                            y: message_area.y + vertical_offset,
                            width: message_area.width,
                            height: text_height,
                        };

                        paragraph.render(centered_area, f.buffer_mut());

                        // Render dialogs over empty state
                        if self.state.base_dir_dialog_state.is_visible() {
                            let dialog = BaseDirDialog::new();
                            dialog.render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.base_dir_dialog_state,
                            );
                        } else if self.state.provider_selector_state.is_visible() {
                            let selector = ProviderSelector::new();
                            selector.render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.provider_selector_state.dialog,
                            );
                        } else if self.state.project_picker_state.is_visible() {
                            let picker = ProjectPicker::new();
                            picker.render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.project_picker_state,
                            );
                        } else if self.state.add_repo_dialog_state.is_visible() {
                            let dialog = AddRepoDialog::new();
                            dialog.render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.add_repo_dialog_state,
                            );
                        } else if self.state.session_import_state.is_visible() {
                            let picker = SessionImportPicker::new();
                            picker.render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.session_import_state,
                            );
                        } else if self.state.model_selector_state.is_visible() {
                            self.state.model_selector_state.update_viewport(right_area);
                            let selector = ModelSelector::new();
                            selector.render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.model_selector_state,
                            );
                        } else if self.state.reasoning_selector_state.is_visible() {
                            let selector = ReasoningSelector::new();
                            selector.render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.reasoning_selector_state,
                            );
                        } else if self.state.orchestration_selector_state.is_visible() {
                            let selector = OrchestrationSelector::new();
                            selector.render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.orchestration_selector_state,
                            );
                        } else if self.state.theme_picker_state.is_visible() {
                            self.render_theme_picker(right_area, f.buffer_mut());
                        }

                        // Draw agent selector dialog if needed
                        if self.state.agent_selector_state.is_visible() {
                            let selector = AgentSelector::new();
                            selector.render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.agent_selector_state,
                            );
                        }

                        // Draw confirmation dialog if open
                        if self.state.confirmation_dialog_state.visible {
                            use ratatui::widgets::Widget;
                            let dialog =
                                ConfirmationDialog::new(&self.state.confirmation_dialog_state);
                            dialog.render(right_area, f.buffer_mut());
                        }

                        // Draw error dialog if open
                        if self.state.error_dialog_state.visible {
                            use ratatui::widgets::Widget;
                            let dialog = ErrorDialog::new(&self.state.error_dialog_state);
                            dialog.render(right_area, f.buffer_mut());
                        }

                        // Draw missing tool dialog if open
                        if self.state.missing_tool_dialog_state.is_visible() {
                            use ratatui::widgets::Widget;
                            let dialog =
                                MissingToolDialog::new(&self.state.missing_tool_dialog_state);
                            dialog.render(right_area, f.buffer_mut());
                        }

                        // Draw help dialog if open
                        if self.state.help_dialog_state.is_visible() {
                            HelpDialog::new().render(
                                right_area,
                                f.buffer_mut(),
                                &mut self.state.help_dialog_state,
                            );
                        }

                        if self.state.input_mode == InputMode::SettingsMenu
                            || self.state.settings_menu_state.is_visible()
                        {
                            SettingsMenu::new().render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.settings_menu_state,
                            );
                        }

                        // Draw command palette (on top of everything)
                        if self.state.command_palette_state.is_visible() {
                            CommandPalette::new().render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.command_palette_state,
                            );
                        }

                        if self.state.input_mode == InputMode::WorkspaceDefaults
                            || self.state.workspace_defaults_dialog_state.is_visible()
                        {
                            WorkspaceDefaultsDialog::new().render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.workspace_defaults_dialog_state,
                            );
                        }

                        if self.state.input_mode == InputMode::RenamingProject
                            || self.state.rename_project_dialog_state.is_visible()
                        {
                            RenameProjectDialog::new().render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.rename_project_dialog_state,
                            );
                        }

                        if self.state.input_mode == InputMode::ManageMcp
                            || self.state.mcp_dialog_state.is_visible()
                        {
                            McpDialog::new().render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.mcp_dialog_state,
                            );
                        }

                        // Draw workspace-creation dialogs (remote sync, issue/spec
                        // pickers, progress) — these can appear over the splash
                        // screen when the user starts a new workspace with no
                        // sessions open.
                        if self.state.issue_picker_state.visible {
                            use crate::components::IssuePicker;
                            IssuePicker::new().render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.issue_picker_state,
                            );
                        }

                        if self.state.spec_picker_state.visible {
                            use crate::components::SpecPicker;
                            SpecPicker::new().render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.spec_picker_state,
                            );
                        }

                        if self.state.specify_picker_state.visible {
                            use crate::components::SpecifyPicker;
                            SpecifyPicker::new().render(
                                right_area,
                                f.buffer_mut(),
                                &self.state.specify_picker_state,
                            );
                        }

                        if self.state.workspace_progress_dialog_state.visible {
                            use crate::components::WorkspaceProgressDialog;
                            use ratatui::widgets::Widget;
                            WorkspaceProgressDialog::new(
                                &self.state.workspace_progress_dialog_state,
                            )
                            .render(right_area, f.buffer_mut());
                        }

                        if self.state.remote_sync_dialog_state.visible {
                            use crate::components::RemoteSyncDialog;
                            use ratatui::widgets::Widget;
                            RemoteSyncDialog::new(&self.state.remote_sync_dialog_state)
                                .render(right_area, f.buffer_mut());
                        }

                        // Draw footer for empty state (sidebar-aware)
                        let footer_context =
                            if self.state.input_mode == InputMode::SidebarNavigation {
                                FooterContext::Sidebar
                            } else {
                                FooterContext::Empty
                            };
                        let footer = GlobalFooter::for_context_with_config(
                            footer_context,
                            &self.config().keybindings.clone(),
                        )
                        .with_spinner(self.state.footer_spinner.as_ref())
                        .with_message(self.state.footer_message.as_deref());
                        footer.render(footer_area, f.buffer_mut());

                        return;
                    }

                    // Margins for input area (constants to avoid duplication)
                    const INPUT_MARGIN_LEFT: u16 = 1;
                    const INPUT_MARGIN_RIGHT: u16 = 1;
                    let input_total_margin = INPUT_MARGIN_LEFT + INPUT_MARGIN_RIGHT;

                    // Calculate dynamic input height (max 30% of screen)
                    // When inline prompt is active, set to 0 so chat area expands
                    let max_input_height = (content_area.height as f32 * 0.30).ceil() as u16;
                    let input_width = content_area.width.saturating_sub(input_total_margin);
                    let has_inline_prompt = self
                        .state
                        .tab_manager
                        .active_session()
                        .map(|s| s.inline_prompt.is_some())
                        .unwrap_or(false);

                    let input_height = if has_inline_prompt {
                        0 // No input box when inline prompt is active
                    } else if let Some(session) = self.state.tab_manager.active_session() {
                        session
                            .input_box
                            .desired_height(max_input_height, input_width)
                    } else {
                        3 // Minimum height
                    };

                    // When inline prompt is active, hide status bar and gap too
                    let status_bar_height = if has_inline_prompt { 0 } else { 1 };
                    let gap_height = if has_inline_prompt { 0 } else { 1 };

                    // Chat layout with session header, input box, status bar, and gap
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(1),                 // Tab bar
                            Constraint::Length(1),                 // Session header
                            Constraint::Min(5),                    // Chat view
                            Constraint::Length(input_height),      // Input box (dynamic)
                            Constraint::Length(status_bar_height), // Status bar (hidden during inline prompt)
                            Constraint::Length(gap_height),        // Gap row before footer
                        ])
                        .split(content_area);

                    // Extract named areas to avoid brittle numeric indices
                    let tab_bar_chunk = chunks[0];
                    let header_chunk = chunks[1];
                    let chat_chunk = chunks[2];
                    let input_chunk = chunks[3];
                    let status_bar_chunk = chunks[4];
                    let gap_chunk = chunks[5];

                    // Create margin-adjusted areas for input, status bar, and gap rows
                    let input_area_inner = Rect {
                        x: input_chunk.x + INPUT_MARGIN_LEFT,
                        y: input_chunk.y,
                        width: input_chunk.width.saturating_sub(input_total_margin),
                        height: input_chunk.height,
                    };
                    let status_bar_area_inner = Rect {
                        x: status_bar_chunk.x + INPUT_MARGIN_LEFT,
                        y: status_bar_chunk.y,
                        width: status_bar_chunk.width.saturating_sub(input_total_margin),
                        height: status_bar_chunk.height,
                    };
                    let gap_area_inner = Rect {
                        x: gap_chunk.x + INPUT_MARGIN_LEFT,
                        y: gap_chunk.y,
                        width: gap_chunk.width.saturating_sub(input_total_margin),
                        height: gap_chunk.height,
                    };

                    // Fill margin areas so they match the app background.
                    let buf = f.buffer_mut();
                    let fill_margins = |buf: &mut ratatui::buffer::Buffer, row_area: Rect, bg| {
                        let style = ratatui::style::Style::default().bg(bg);
                        let left_width = INPUT_MARGIN_LEFT.min(row_area.width);
                        if left_width > 0 {
                            buf.set_style(
                                Rect {
                                    x: row_area.x,
                                    y: row_area.y,
                                    width: left_width,
                                    height: row_area.height,
                                },
                                style,
                            );
                        }
                        let right_width =
                            INPUT_MARGIN_RIGHT.min(row_area.width.saturating_sub(left_width));
                        if right_width > 0 {
                            let right_start =
                                row_area.x + row_area.width.saturating_sub(right_width);
                            buf.set_style(
                                Rect {
                                    x: right_start,
                                    y: row_area.y,
                                    width: right_width,
                                    height: row_area.height,
                                },
                                style,
                            );
                        }
                    };

                    use crate::components::bg_base;
                    let margin_bg = bg_base();
                    fill_margins(buf, input_chunk, margin_bg);
                    fill_margins(buf, status_bar_chunk, margin_bg);
                    fill_margins(buf, gap_chunk, margin_bg);

                    // Draw separator line in the gap row (▀ characters)
                    // Foreground = status bar bg, background = base bg (creates rounded bottom edge)
                    // Skip when inline prompt is active (gap row is hidden)
                    if !has_inline_prompt {
                        use crate::components::status_bar_bg;
                        for x in gap_area_inner.x..gap_area_inner.x + gap_area_inner.width {
                            buf[(x, gap_area_inner.y)]
                                .set_char('▀')
                                .set_fg(status_bar_bg());
                        }
                    }

                    // Store layout areas for mouse hit-testing
                    // Set hidden areas to None when inline prompt is active to avoid hit-testing confusion
                    self.state.tab_bar_area = Some(tab_bar_chunk);
                    self.state.chat_area = Some(chat_chunk);
                    self.state.file_viewer_area = None;
                    self.state.raw_events_area = None;
                    self.state.input_area = if has_inline_prompt {
                        None
                    } else {
                        Some(input_area_inner)
                    };
                    self.state.status_bar_area = if has_inline_prompt {
                        None
                    } else {
                        Some(status_bar_area_inner)
                    };
                    self.state.footer_area = Some(footer_area);

                    // Draw tab bar (unfocused when sidebar is focused)
                    let tabs_focused = self.state.input_mode != InputMode::SidebarNavigation;
                    self.ensure_tab_bar_scroll(tab_bar_chunk.width, tabs_focused);
                    let tab_bar = self.build_tab_bar(tabs_focused);
                    tab_bar.render(tab_bar_chunk, f.buffer_mut());

                    // Draw session header (below tab bar)
                    let session_title = self
                        .state
                        .tab_manager
                        .active_session()
                        .and_then(|s| s.title.as_deref());
                    SessionHeader::new(session_title).render(header_chunk, f.buffer_mut());

                    // Draw active session components
                    let is_command_mode = self.state.input_mode == InputMode::Command;
                    let show_chat_scrollbar = self.config().ui.show_chat_scrollbar;
                    let thinking_indicator_shimmer = self.config().ui.thinking_indicator_shimmer;
                    let thinking_indicator_spinner = self.config().ui.thinking_indicator_spinner;
                    let thinking_indicator_label =
                        self.config().ui.thinking_indicator_label.clone();
                    if let Some(session) = self.state.tab_manager.active_session_mut() {
                        // Use full chat area - prompt is now rendered as part of scrollable content
                        let chat_area = chat_chunk;

                        self.state.chat_area = if chat_area.height == 0 {
                            None
                        } else {
                            Some(chat_area)
                        };

                        // Render chat with thinking indicator if processing (but not during inline prompt)
                        let thinking_line =
                            if session.is_processing && session.inline_prompt.is_none() {
                                Some(session.thinking_indicator.render(
                                    thinking_indicator_shimmer,
                                    thinking_indicator_spinner,
                                    &thinking_indicator_label,
                                ))
                            } else {
                                None
                            };
                        let input_mode = self.state.input_mode;
                        let queue_lines =
                            app_queue::build_queue_lines(session, chat_area.width, input_mode);

                        // Build prompt lines from inline_prompt (renders as part of scrollable chat).
                        // Use the content area width (after left margin / scrollbar) so wrap points
                        // match the actual visible columns. Mirrors ChatView::content_area logic.
                        let prompt_render_width = if show_chat_scrollbar {
                            chat_area.width.saturating_sub(10)
                        } else {
                            chat_area.width.saturating_sub(8)
                        } as usize;
                        let prompt_lines = session
                            .inline_prompt
                            .as_ref()
                            .map(|p| p.render_as_lines(prompt_render_width));

                        session.chat_view.render_with_indicator(
                            chat_area,
                            f.buffer_mut(),
                            thinking_line,
                            queue_lines,
                            prompt_lines,
                            show_chat_scrollbar,
                        );

                        // Check if inline prompt is active
                        let has_inline_prompt = session.inline_prompt.is_some();

                        // Render input box (not in command mode, not when inline prompt active)
                        if !is_command_mode && !has_inline_prompt {
                            session.input_box.render(input_area_inner, f.buffer_mut());
                        }
                        // Update and render status bar (skip when inline prompt is active)
                        if !has_inline_prompt {
                            session.status_bar.set_metrics(
                                self.state.show_metrics,
                                self.state.metrics.draw_time,
                                self.state.metrics.event_time,
                                self.state.metrics.fps,
                                self.state.metrics.scroll_latency,
                                self.state.metrics.scroll_latency_avg,
                                self.state.metrics.scroll_lines_per_sec,
                                self.state.metrics.scroll_events_per_sec,
                                self.state.metrics.scroll_active,
                            );
                            session
                                .status_bar
                                .set_spinner_frame(self.state.spinner_frame);
                            session
                                .status_bar
                                .render(status_bar_area_inner, f.buffer_mut());
                        }

                        // Set cursor position (accounting for scroll)
                        if self.state.input_mode == InputMode::Normal {
                            // Inline prompt uses visual cursor (reversed style) in the rendered lines,
                            // so no cursor positioning needed. Only set cursor for normal input box.
                            if !has_inline_prompt {
                                let scroll_offset = session.input_box.scroll_offset();
                                let (cx, cy) = session
                                    .input_box
                                    .cursor_position(input_area_inner, scroll_offset);
                                f.set_cursor_position((cx, cy));
                            }
                        }
                    }

                    // Render command prompt if in command mode (outside session borrow)
                    if is_command_mode {
                        self.render_command_prompt(input_area_inner, f.buffer_mut());
                        // Cursor at end of command buffer (after prompt in padded area)
                        let prompt = format!("  cmd › {}", self.state.command_buffer);
                        let prompt_width = prompt.width() as u16;
                        let max_x = input_area_inner.x + input_area_inner.width.saturating_sub(1);
                        let cx = (input_area_inner.x + prompt_width).min(max_x);
                        let cy = input_area_inner.y + 1; // top padding
                        f.set_cursor_position((cx, cy));
                    }

                    if self.state.slash_menu_state.is_visible() && !has_inline_prompt {
                        self.render_slash_menu(chat_chunk, input_area_inner, f.buffer_mut());
                    }

                    if self.state.file_mention_state.is_visible() && !has_inline_prompt {
                        self.render_file_mention_menu(chat_chunk, input_area_inner, f.buffer_mut());
                    }

                    // Draw footer (full width) - context-aware based on input mode
                    let footer = GlobalFooter::from_state_with_config(
                        self.state.view_mode,
                        self.state.input_mode,
                        !self.state.tab_manager.is_empty(),
                        &self.config().keybindings.clone(),
                    )
                    .with_spinner(self.state.footer_spinner.as_ref())
                    .with_message(self.state.footer_message.as_deref());
                    footer.render(footer_area, f.buffer_mut());
                }
                ViewMode::RawEvents => {
                    // Raw events layout - no input box, full height for events
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(1), // Tab bar
                            Constraint::Length(1), // Session header
                            Constraint::Min(5),    // Raw events view (full height)
                        ])
                        .split(content_area);

                    // Extract named areas to avoid brittle numeric indices
                    let tab_bar_chunk = chunks[0];
                    let header_chunk = chunks[1];
                    let raw_events_chunk = chunks[2];

                    // Store layout areas for mouse hit-testing (no input/status in this mode)
                    self.state.tab_bar_area = Some(tab_bar_chunk);
                    self.state.chat_area = None;
                    self.state.file_viewer_area = None;
                    self.state.raw_events_area = Some(raw_events_chunk);
                    self.state.input_area = None;
                    self.state.status_bar_area = None;
                    self.state.footer_area = Some(footer_area);

                    // Draw tab bar (unfocused when sidebar is focused)
                    let tabs_focused = self.state.input_mode != InputMode::SidebarNavigation;
                    self.ensure_tab_bar_scroll(tab_bar_chunk.width, tabs_focused);
                    let tab_bar = self.build_tab_bar(tabs_focused);
                    tab_bar.render(tab_bar_chunk, f.buffer_mut());

                    // Draw session header (below tab bar) - consistent with Chat view
                    let session_title = self
                        .state
                        .tab_manager
                        .active_session()
                        .and_then(|s| s.title.as_deref());
                    SessionHeader::new(session_title).render(header_chunk, f.buffer_mut());

                    // Draw raw events view
                    if let Some(session) = self.state.tab_manager.active_session_mut() {
                        session
                            .raw_events_view
                            .render(raw_events_chunk, f.buffer_mut());
                    }

                    // Draw footer (full width) - context-aware based on input mode
                    let footer = GlobalFooter::from_state_with_config(
                        self.state.view_mode,
                        self.state.input_mode,
                        !self.state.tab_manager.is_empty(),
                        &self.config().keybindings.clone(),
                    )
                    .with_spinner(self.state.footer_spinner.as_ref())
                    .with_message(self.state.footer_message.as_deref());
                    footer.render(footer_area, f.buffer_mut());
                }
            }
        } // end of else block for agent tab rendering

        // Draw agent selector dialog if needed
        if self.state.agent_selector_state.is_visible() {
            let selector = AgentSelector::new();
            selector.render(right_area, f.buffer_mut(), &self.state.agent_selector_state);
        }

        // Draw add repository dialog if open
        if self.state.add_repo_dialog_state.is_visible() {
            let dialog = AddRepoDialog::new();
            dialog.render(
                right_area,
                f.buffer_mut(),
                &self.state.add_repo_dialog_state,
            );
        }

        if self.state.reasoning_selector_state.is_visible() {
            let selector = ReasoningSelector::new();
            selector.render(
                right_area,
                f.buffer_mut(),
                &self.state.reasoning_selector_state,
            );
        }

        if self.state.orchestration_selector_state.is_visible() {
            let selector = OrchestrationSelector::new();
            selector.render(
                right_area,
                f.buffer_mut(),
                &self.state.orchestration_selector_state,
            );
        }

        // Draw theme picker dialog if open
        self.render_theme_picker(right_area, f.buffer_mut());

        // Draw base directory dialog if open
        if self.state.base_dir_dialog_state.is_visible() {
            let dialog = BaseDirDialog::new();
            dialog.render(
                right_area,
                f.buffer_mut(),
                &self.state.base_dir_dialog_state,
            );
        }

        // Draw project picker if open
        if self.state.project_picker_state.is_visible() {
            let picker = ProjectPicker::new();
            picker.render(right_area, f.buffer_mut(), &self.state.project_picker_state);
        }

        // Draw session import picker if open
        if self.state.session_import_state.is_visible() {
            let picker = SessionImportPicker::new();
            picker.render(right_area, f.buffer_mut(), &self.state.session_import_state);
        }

        // Draw confirmation dialog if open
        if self.state.confirmation_dialog_state.visible {
            use ratatui::widgets::Widget;
            let dialog = ConfirmationDialog::new(&self.state.confirmation_dialog_state);
            dialog.render(right_area, f.buffer_mut());
        }

        // Draw error dialog (on top of everything except spinner)
        if self.state.error_dialog_state.visible {
            use ratatui::widgets::Widget;
            let dialog = ErrorDialog::new(&self.state.error_dialog_state);
            dialog.render(right_area, f.buffer_mut());
        }

        // Draw missing tool dialog (on top of everything except spinner)
        if self.state.missing_tool_dialog_state.is_visible() {
            use ratatui::widgets::Widget;
            let dialog = MissingToolDialog::new(&self.state.missing_tool_dialog_state);
            dialog.render(right_area, f.buffer_mut());
        }

        // Draw help dialog (on top of everything)
        if self.state.help_dialog_state.is_visible() {
            HelpDialog::new().render(
                right_area,
                f.buffer_mut(),
                &mut self.state.help_dialog_state,
            );
        }

        if self.state.input_mode == InputMode::SettingsMenu
            || self.state.settings_menu_state.is_visible()
        {
            SettingsMenu::new().render(right_area, f.buffer_mut(), &self.state.settings_menu_state);
        }

        if self.state.keybindings_editor_state.is_visible() {
            KeybindingsEditor::new().render(
                right_area,
                f.buffer_mut(),
                &self.state.keybindings_editor_state,
            );
        }

        // Draw command palette (on top of everything)
        if self.state.command_palette_state.is_visible() {
            CommandPalette::new().render(
                right_area,
                f.buffer_mut(),
                &self.state.command_palette_state,
            );
        }

        if self.state.input_mode == InputMode::WorkspaceDefaults
            || self.state.workspace_defaults_dialog_state.is_visible()
        {
            WorkspaceDefaultsDialog::new().render(
                right_area,
                f.buffer_mut(),
                &self.state.workspace_defaults_dialog_state,
            );
        }

        if self.state.input_mode == InputMode::RenamingProject
            || self.state.rename_project_dialog_state.is_visible()
        {
            RenameProjectDialog::new().render(
                right_area,
                f.buffer_mut(),
                &self.state.rename_project_dialog_state,
            );
        }

        if self.state.input_mode == InputMode::ManageMcp || self.state.mcp_dialog_state.is_visible()
        {
            McpDialog::new().render(right_area, f.buffer_mut(), &self.state.mcp_dialog_state);
        }

        if self.state.file_picker_dialog_state.is_visible() {
            use crate::components::FilePickerDialog;
            FilePickerDialog::new().render(
                right_area,
                f.buffer_mut(),
                &self.state.file_picker_dialog_state,
            );
        }

        if self.state.scp_command_dialog_state.visible {
            use crate::components::ScpCommandDialog;
            ScpCommandDialog::new().render(
                right_area,
                f.buffer_mut(),
                &self.state.scp_command_dialog_state,
            );
        }

        if self.state.issue_picker_state.visible {
            use crate::components::IssuePicker;
            IssuePicker::new().render(right_area, f.buffer_mut(), &self.state.issue_picker_state);
        }

        if self.state.spec_picker_state.visible {
            use crate::components::SpecPicker;
            SpecPicker::new().render(right_area, f.buffer_mut(), &self.state.spec_picker_state);
        }

        if self.state.specify_picker_state.visible {
            use crate::components::SpecifyPicker;
            SpecifyPicker::new().render(
                right_area,
                f.buffer_mut(),
                &self.state.specify_picker_state,
            );
        }

        // Draw cloning repository spinner overlay
        if self.state.input_mode == InputMode::CloningRepository {
            use crate::components::{accent_primary, DialogFrame, Spinner};
            use ratatui::layout::Alignment;
            use ratatui::text::Line;
            use ratatui::widgets::{Paragraph, Widget};

            let content_area =
                DialogFrame::new("Cloning Repository", 38, 4).render(right_area, f.buffer_mut());

            let spinner = Spinner::dots();
            let line = Line::from(vec![
                spinner.span(accent_primary()),
                ratatui::text::Span::raw(" Cloning repository..."),
            ]);

            Paragraph::new(line)
                .alignment(Alignment::Center)
                .render(content_area, f.buffer_mut());
        }

        // Draw removing project spinner overlay
        if self.state.input_mode == InputMode::RemovingProject {
            use crate::components::{accent_primary, DialogFrame, Spinner};
            use ratatui::layout::Alignment;
            use ratatui::text::Line;
            use ratatui::widgets::{Paragraph, Widget};

            let content_area =
                DialogFrame::new("Removing Project", 36, 4).render(right_area, f.buffer_mut());

            let spinner = Spinner::dots();
            let line = Line::from(vec![
                spinner.span(accent_primary()),
                ratatui::text::Span::raw(" Removing project..."),
            ]);

            Paragraph::new(line)
                .alignment(Alignment::Center)
                .render(content_area, f.buffer_mut());
        }

        // Draw workspace creation progress dialog
        if self.state.workspace_progress_dialog_state.visible {
            use crate::components::WorkspaceProgressDialog;
            use ratatui::widgets::Widget;
            WorkspaceProgressDialog::new(&self.state.workspace_progress_dialog_state)
                .render(right_area, f.buffer_mut());
        }

        // Draw provider/model selectors after workspace progress dialog so they
        // appear on top when opened from the workspace ready config panel.
        if self.state.provider_selector_state.is_visible() {
            let selector = ProviderSelector::new();
            selector.render(
                right_area,
                f.buffer_mut(),
                &self.state.provider_selector_state.dialog,
            );
        }

        if self.state.model_selector_state.is_visible() {
            self.state.model_selector_state.update_viewport(right_area);
            let model_selector = ModelSelector::new();
            model_selector.render(right_area, f.buffer_mut(), &self.state.model_selector_state);
        }

        // Draw remote-sync dialog (shown during the SyncingRemote phase of
        // workspace creation, before the issue picker appears).
        if self.state.remote_sync_dialog_state.visible {
            use crate::components::RemoteSyncDialog;
            use ratatui::widgets::Widget;
            RemoteSyncDialog::new(&self.state.remote_sync_dialog_state)
                .render(right_area, f.buffer_mut());
        }

        // Draw Work Complete dialog
        if let Some(ref session) = self.state.work_complete_session {
            use crate::components::WorkCompleteDialog;
            use ratatui::widgets::Widget;
            WorkCompleteDialog::new(session, self.state.spinner_frame)
                .render(right_area, f.buffer_mut());
        }
    }

    /// Render file viewer tab content
    pub(super) fn render_file_viewer_tab(
        &mut self,
        content_area: Rect,
        footer_area: Rect,
        f: &mut ratatui::Frame<'_>,
    ) {
        use crate::components::{
            bg_base, text_muted, text_primary, FileViewerView, FooterContext, GlobalFooter,
        };
        use ratatui::style::Style;
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Paragraph, Widget};
        use unicode_width::UnicodeWidthStr;

        let is_command_mode = self.state.input_mode == InputMode::Command;

        // Layout: tab bar, file header, content (+ optional command prompt)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(if is_command_mode {
                vec![
                    Constraint::Length(1), // Tab bar
                    Constraint::Length(1), // File header
                    Constraint::Min(3),    // File content
                    Constraint::Length(3), // Command prompt
                ]
            } else {
                vec![
                    Constraint::Length(1), // Tab bar
                    Constraint::Length(1), // File header (path + line count)
                    Constraint::Min(5),    // File content
                ]
            })
            .split(content_area);

        let tab_bar_chunk = chunks[0];
        let header_chunk = chunks[1];
        let content_chunk = chunks[2];
        let command_chunk = if is_command_mode {
            Some(chunks[3])
        } else {
            None
        };

        // Store areas for mouse hit-testing
        self.state.tab_bar_area = Some(tab_bar_chunk);
        self.state.chat_area = None;
        self.state.file_viewer_area = Some(content_chunk);
        self.state.raw_events_area = None;
        self.state.input_area = command_chunk;
        self.state.status_bar_area = None;
        self.state.footer_area = Some(footer_area);

        // Render tab bar
        let tabs_focused = self.state.input_mode != InputMode::SidebarNavigation;
        self.ensure_tab_bar_scroll(tab_bar_chunk.width, tabs_focused);
        let tab_bar = self.build_tab_bar(tabs_focused);
        tab_bar.render(tab_bar_chunk, f.buffer_mut());

        // Render file header and content
        if let Some(file_session) = self.state.tab_manager.active_file_viewer_mut() {
            let markdown_width = content_chunk.width.saturating_sub(1) as usize;
            file_session.ensure_render_cache(markdown_width);

            // Render file header with path and line count
            let path_str = file_session.file_path.display().to_string();
            let mode_label = format!(
                "{} • {}",
                file_session.file_kind_label(),
                file_session.view_mode_label()
            );
            let line_info = format!(
                " ({} lines • {})",
                file_session.effective_total_lines(),
                mode_label
            );

            // Truncate path if it doesn't fit in the header width (UTF-8 safe, width-aware)
            let available_width = header_chunk.width.saturating_sub(2) as usize; // 1 leading space + 1 safety
            let line_info_width = UnicodeWidthStr::width(line_info.as_str());
            let max_path_width = available_width.saturating_sub(line_info_width);

            let truncated_path = if UnicodeWidthStr::width(path_str.as_str()) > max_path_width {
                if max_path_width <= 3 {
                    // Not enough room for "..." + content
                    "...".chars().take(max_path_width).collect::<String>()
                } else {
                    // Build tail from right, respecting character boundaries and widths
                    let mut tail = String::new();
                    let mut width = 0usize;
                    let target = max_path_width.saturating_sub(3); // reserve for "..."
                    for ch in path_str.chars().rev() {
                        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
                        if width + w > target {
                            break;
                        }
                        width += w;
                        tail.insert(0, ch);
                    }
                    format!("...{}", tail)
                }
            } else {
                path_str
            };

            let header_line = Line::from(vec![
                Span::styled(" ", Style::default().bg(bg_base())),
                Span::styled(
                    truncated_path,
                    Style::default().fg(text_primary()).bg(bg_base()),
                ),
                Span::styled(line_info, Style::default().fg(text_muted()).bg(bg_base())),
            ]);

            let header_para = Paragraph::new(header_line).style(Style::default().bg(bg_base()));
            header_para.render(header_chunk, f.buffer_mut());

            // Render file content with line numbers and scrollbar
            FileViewerView::new(file_session).render(content_chunk, f.buffer_mut());
        }

        // Render command prompt if in command mode
        if let Some(cmd_area) = command_chunk {
            self.render_command_prompt(cmd_area, f.buffer_mut());
            // Set cursor position for command input
            let prompt = format!("  cmd › {}", self.state.command_buffer);
            let prompt_width = UnicodeWidthStr::width(prompt.as_str()) as u16;
            let max_x = cmd_area.x + cmd_area.width.saturating_sub(1);
            let cx = (cmd_area.x + prompt_width).min(max_x);
            let cy = cmd_area.y + 1;
            f.set_cursor_position((cx, cy));
        }

        // Render footer (sidebar-aware)
        let footer_context = if self.state.input_mode == InputMode::SidebarNavigation {
            FooterContext::Sidebar
        } else {
            FooterContext::FileViewer
        };
        let footer = GlobalFooter::for_context_with_config(
            footer_context,
            &self.config().keybindings.clone(),
        )
        .with_spinner(self.state.footer_spinner.as_ref())
        .with_message(self.state.footer_message.as_deref());
        footer.render(footer_area, f.buffer_mut());
    }

    pub(super) fn render_theme_picker(&mut self, size: Rect, buf: &mut ratatui::buffer::Buffer) {
        if !self.state.theme_picker_state.is_visible() {
            return;
        }
        use ratatui::widgets::Widget;
        self.state.theme_picker_state.update_viewport(size);
        let picker = ThemePicker::new(&self.state.theme_picker_state);
        picker.render(size, buf);
    }

    /// Render command mode prompt
    pub(super) fn render_command_prompt(&self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        use ratatui::style::Style;
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Clear, Paragraph, Widget};
        use unicode_width::UnicodeWidthStr;

        Clear.render(area, buf);
        buf.set_style(area, Style::default().bg(crate::components::input_bg()));

        if area.height < 3 || area.width == 0 {
            return;
        }

        let padding_top: u16 = 1;
        let padding_bottom: u16 = 1;
        let content_height = area.height.saturating_sub(padding_top + padding_bottom);
        if content_height == 0 {
            return;
        }

        let prefix = "  cmd › ";
        let prefix_width = UnicodeWidthStr::width(prefix) as u16;
        let buffer_width = UnicodeWidthStr::width(self.state.command_buffer.as_str()) as u16;
        let total_width = prefix_width + buffer_width;
        let content_width = area.width;

        let line = if total_width > content_width {
            // Truncate from the left, showing most recent input
            let mut truncated = String::new();
            let mut width = 0usize;
            for ch in self.state.command_buffer.chars().rev() {
                let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
                if width + w > content_width.saturating_sub(prefix_width + 1) as usize {
                    break;
                }
                width += w;
                truncated.insert(0, ch);
            }
            Line::from(vec![
                Span::styled(prefix, Style::default().fg(crate::components::text_muted())),
                Span::raw("…"),
                Span::styled(
                    truncated,
                    Style::default().fg(crate::components::text_primary()),
                ),
            ])
        } else {
            Line::from(vec![
                Span::styled(prefix, Style::default().fg(crate::components::text_muted())),
                Span::styled(
                    &self.state.command_buffer,
                    Style::default().fg(crate::components::text_primary()),
                ),
            ])
        };

        let para = Paragraph::new(line).style(Style::default().bg(crate::components::input_bg()));
        para.render(
            Rect {
                x: area.x,
                y: area.y + padding_top,
                width: content_width,
                height: content_height,
            },
            buf,
        );
    }

    pub(super) fn render_slash_menu(
        &mut self,
        chat_area: Rect,
        input_area: Rect,
        buf: &mut ratatui::buffer::Buffer,
    ) {
        if !self.state.slash_menu_state.is_visible() {
            return;
        }

        let available_height = input_area.y.saturating_sub(chat_area.y);
        let list_height_max = available_height.saturating_sub(4);
        if list_height_max == 0 {
            return;
        }

        let list_len = self.state.slash_menu_state.filtered_len().max(1);
        let list_height = list_len.min(list_height_max as usize).max(1) as u16;
        self.state
            .slash_menu_state
            .set_max_visible(list_height as usize);

        let menu_height = list_height.saturating_add(4);
        let menu_y = input_area.y.saturating_sub(menu_height);
        let menu_area = Rect {
            x: input_area.x,
            y: menu_y,
            width: input_area.width,
            height: menu_height,
        };

        SlashMenu::new().render(menu_area, buf, &self.state.slash_menu_state);
    }

    pub(super) fn render_file_mention_menu(
        &mut self,
        chat_area: Rect,
        input_area: Rect,
        buf: &mut ratatui::buffer::Buffer,
    ) {
        if !self.state.file_mention_state.is_visible() {
            return;
        }

        let available_height = input_area.y.saturating_sub(chat_area.y);
        let list_height_max = available_height.saturating_sub(4);
        if list_height_max == 0 {
            return;
        }

        let list_len = self.state.file_mention_state.filtered_len().max(1);
        let list_height = list_len.min(list_height_max as usize).max(1) as u16;
        self.state
            .file_mention_state
            .set_max_visible(list_height as usize);

        let menu_height = list_height.saturating_add(4);
        let menu_y = input_area.y.saturating_sub(menu_height);
        let menu_area = Rect {
            x: input_area.x,
            y: menu_y,
            width: input_area.width,
            height: menu_height,
        };

        SlashMenu::new().render(menu_area, buf, &self.state.file_mention_state);
    }

    pub(super) fn find_latest_plan_file(session: &AgentSession) -> Option<std::path::PathBuf> {
        let mut candidates = Vec::new();
        if let Some(home_dir) = dirs::home_dir() {
            candidates.push(home_dir.join(".claude").join("plans"));
        }
        if let Some(ref working_dir) = session.working_dir {
            candidates.push(working_dir.join(".claude").join("plans"));
        }

        let mut newest: Option<(std::path::PathBuf, std::time::SystemTime)> = None;
        for plans_dir in candidates {
            if !plans_dir.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(&plans_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "md") {
                        if let Ok(metadata) = path.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                if newest.as_ref().is_none_or(|(_, t)| modified > *t) {
                                    newest = Some((path, modified));
                                }
                            }
                        }
                    }
                }
            }
        }
        newest.map(|(path, _)| path)
    }

    /// Find the most recent plan file path for the session (for ExitPlanMode display)
    pub(super) fn read_plan_file_path_for_session(session: &AgentSession) -> Option<String> {
        Self::find_latest_plan_file(session).map(|path| path.display().to_string())
    }

    /// Read the plan file for the current session (for ExitPlanMode display)
    pub(super) fn read_plan_file_for_session(session: &AgentSession) -> (String, String) {
        if let Some(path) = Self::find_latest_plan_file(session) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                return (content, path.display().to_string());
            }
        }
        // Fallback if no plan file found
        (
            "(Plan content not found)".to_string(),
            ".claude/plans/plan.md".to_string(),
        )
    }

    /// Extract a filename from tool result text
    pub(super) fn extract_filename(text: &str) -> Option<String> {
        // Look for common file path patterns
        for line in text.lines() {
            let line = line.trim();
            // Look for paths like /path/to/file.rs or file.rs
            if line.contains('/') || line.contains('.') {
                // Try to find a file path
                for word in line.split_whitespace() {
                    let word = word.trim_matches(|c: char| {
                        !c.is_alphanumeric() && c != '/' && c != '.' && c != '_' && c != '-'
                    });
                    if word.contains('.') && !word.starts_with('.') {
                        // Looks like a filename
                        return Some(word.to_string());
                    }
                }
            }
        }
        None
    }

    /// Dump complete app state to a JSON file for debugging.
    pub(super) fn dump_debug_state(&self) -> Result<String, String> {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");

        // Save to ~/.conduit/debug/ directory
        let debug_dir = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".conduit")
            .join("debug");

        // Create directory if it doesn't exist
        std::fs::create_dir_all(&debug_dir)
            .map_err(|e| format!("Could not create debug directory: {}", e))?;

        let filepath = debug_dir.join(format!("conduit_debug_{}.json", timestamp));

        let mut sessions_data = Vec::new();

        for (idx, session) in self.state.tab_manager.sessions().iter().enumerate() {
            // Collect chat messages
            let messages: Vec<_> = session
                .chat_view
                .messages()
                .iter()
                .map(|msg| {
                    let summary_data = msg.summary.as_ref().map(|s| {
                        json!({
                            "duration_secs": s.duration_secs,
                            "input_tokens": s.input_tokens,
                            "output_tokens": s.output_tokens,
                            "files_changed": s.files_changed.iter().map(|f| json!({
                                "filename": f.filename,
                                "additions": f.additions,
                                "deletions": f.deletions,
                            })).collect::<Vec<_>>(),
                        })
                    });

                    json!({
                        "role": format!("{:?}", msg.role),
                        "content": msg.content,
                        "content_length": msg.content.len(),
                        "tool_name": msg.tool_name,
                        "tool_args": msg.tool_args,
                        "is_streaming": msg.is_streaming,
                        "has_summary": msg.summary.is_some(),
                        "summary": summary_data,
                    })
                })
                .collect();

            // Collect raw events
            let raw_events: Vec<_> = session
                .raw_events_view
                .events()
                .iter()
                .map(|evt| {
                    let elapsed = evt.timestamp.duration_since(evt.session_start);
                    json!({
                        "timestamp_ms": elapsed.as_millis(),
                        "direction": format!("{:?}", evt.direction),
                        "event_type": evt.event_type,
                        "raw_json": evt.raw_json,
                    })
                })
                .collect();

            // Current turn summary
            let turn_summary = json!({
                "duration_secs": session.current_turn_summary.duration_secs,
                "input_tokens": session.current_turn_summary.input_tokens,
                "output_tokens": session.current_turn_summary.output_tokens,
                "files_changed": session.current_turn_summary.files_changed.iter().map(|f| json!({
                    "filename": f.filename,
                    "additions": f.additions,
                    "deletions": f.deletions,
                })).collect::<Vec<_>>(),
            });

            sessions_data.push(json!({
                "index": idx,
                "id": session.id.to_string(),
                "agent_type": format!("{:?}", session.agent_type),
                "agent_session_id": session.agent_session_id.as_ref().map(|s| s.as_str().to_string()),
                "is_processing": session.is_processing,
                "turn_count": session.turn_count,
                "total_usage": {
                    "input_tokens": session.total_usage.input_tokens,
                    "output_tokens": session.total_usage.output_tokens,
                    "cached_tokens": session.total_usage.cached_tokens,
                    "total_tokens": session.total_usage.total_tokens,
                },
                "current_turn_summary": turn_summary,
                "chat_messages": messages,
                "chat_message_count": session.chat_view.len(),
                "streaming_buffer": session.chat_view.streaming_buffer(),
                "streaming_reasoning": session.chat_view.streaming_message_for(MessageRole::Reasoning),
                "raw_events": raw_events,
                "raw_event_count": session.raw_events_view.len(),
                "input_box_content": session.input_box.input(),
            }));
        }

        let dump = json!({
            "timestamp": Local::now().to_rfc3339(),
            "view_mode": format!("{:?}", self.state.view_mode),
            "input_mode": format!("{:?}", self.state.input_mode),
            "active_tab_index": self.state.tab_manager.active_index(),
            "tab_count": self.state.tab_manager.len(),
            "sessions": sessions_data,
        });

        let full_path = filepath.display().to_string();
        let mut file =
            File::create(&filepath).map_err(|e| format!("Could not create file: {}", e))?;
        let json_str = serde_json::to_string_pretty(&dump)
            .map_err(|e| format!("Could not serialize debug data: {}", e))?;
        file.write_all(json_str.as_bytes())
            .map_err(|e| format!("Could not write to file: {}", e))?;

        Ok(full_path)
    }
}
