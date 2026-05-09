use std::time::{Duration, Instant};

use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use std::io;

use crate::action::Action;
use crate::app::App;
use crate::app_state::ModelPickerContext;
use crate::components::{
    dialog_content_area, GlobalFooter, ProviderSelector, SettingsMenu, TabBarHitTarget,
    WorkspaceDefaultsDialog, SIDEBAR_HEADER_ROWS,
};
use crate::effect::Effect;
use crate::events::{InputMode, ViewMode};
use conduit_agent::{AgentType, MessageDisplay};
use conduit_config::{parse_key_notation, KeyContext};

impl App {
    /// Handle a mouse click at the given position.
    pub(super) async fn handle_mouse_click(
        &mut self,
        x: u16,
        y: u16,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        guard: &mut crate::terminal_guard::TerminalGuard,
    ) -> anyhow::Result<Vec<Effect>> {
        let mut effects = Vec::new();

        // Handle confirmation dialog - close on any click outside
        // Use same context-aware logic as Cancel action for consistent UX
        if self.state.input_mode == InputMode::Confirming
            && self.state.confirmation_dialog_state.visible
        {
            if self.is_blocking_confirmation_loading_dialog() {
                return Ok(effects);
            }
            self.state.input_mode = self.dismiss_confirmation_dialog();
            return Ok(effects);
        }

        // Handle model selector clicks first (it's a modal dialog)
        if self.state.input_mode == InputMode::SelectingModel
            && self.state.model_selector_state.is_visible()
        {
            effects.extend(self.handle_model_selector_click(x, y));
            return Ok(effects);
        }

        if self.state.input_mode == InputMode::SelectingReasoning
            && self.state.reasoning_selector_state.is_visible()
        {
            self.handle_reasoning_selector_click(x, y);
            return Ok(effects);
        }

        if self.state.input_mode == InputMode::SelectingOrchestration
            && self.state.orchestration_selector_state.is_visible()
        {
            self.handle_orchestration_selector_click(x, y);
            return Ok(effects);
        }

        if self.state.input_mode == InputMode::SelectingProviders
            && self.state.provider_selector_state.is_visible()
        {
            self.handle_provider_selector_click(x, y);
            return Ok(effects);
        }

        if self.state.input_mode == InputMode::SettingsMenu
            || self.state.settings_menu_state.is_visible()
        {
            self.handle_settings_menu_click(x, y);
            return Ok(effects);
        }

        if self.state.input_mode == InputMode::WorkspaceDefaults
            || self.state.workspace_defaults_dialog_state.is_visible()
        {
            self.handle_workspace_defaults_click(x, y);
            return Ok(effects);
        }

        // Handle project picker clicks first (it's a modal dialog)
        if self.state.input_mode == InputMode::PickingProject
            && self.state.project_picker_state.is_visible()
        {
            self.handle_project_picker_click(x, y);
            return Ok(effects);
        }

        // Check sidebar first (if visible)
        if let Some(sidebar_area) = self.state.sidebar_area {
            if Self::point_in_rect(x, y, sidebar_area) {
                effects.extend(self.handle_sidebar_click(x, y, sidebar_area));
                return Ok(effects);
            }
        }

        // Check tab bar
        if let Some(tab_bar_area) = self.state.tab_bar_area {
            if Self::point_in_rect(x, y, tab_bar_area) {
                self.handle_tab_bar_click(x, y, tab_bar_area);
                return Ok(effects);
            }
        }

        // Check input area
        if let Some(input_area) = self.state.input_area {
            if Self::point_in_rect(x, y, input_area) {
                self.handle_input_click(x, y, input_area);
                return Ok(effects);
            }
        }

        // Check status bar
        if let Some(status_bar_area) = self.state.status_bar_area {
            if Self::point_in_rect(x, y, status_bar_area) {
                if let Some(effect) = self.handle_status_bar_click(x, y, status_bar_area) {
                    effects.push(effect);
                }
                return Ok(effects);
            }
        }

        // Check footer
        if let Some(footer_area) = self.state.footer_area {
            if Self::point_in_rect(x, y, footer_area) {
                if let Some(action) = self.handle_footer_click(x, y, footer_area) {
                    effects.extend(self.execute_action(action, terminal, guard).await?);
                }
                return Ok(effects);
            }
        }

        // Check raw events area (debug view)
        if self.state.view_mode == ViewMode::RawEvents {
            if let Some(raw_events_area) = self.state.raw_events_area {
                if Self::point_in_rect(x, y, raw_events_area) {
                    if let Some(session) = self.state.tab_manager.active_session_mut() {
                        if let Some(click) =
                            session.raw_events_view.handle_click(x, y, raw_events_area)
                        {
                            match click {
                                crate::components::RawEventsClick::SessionId => {
                                    if let Some(session_id) = session.raw_events_view.session_id() {
                                        let id_str = session_id.to_string();
                                        effects.push(Effect::CopyToClipboard(id_str.clone()));
                                        self.state.set_timed_footer_message(
                                            format!("Copied session ID: {}", id_str),
                                            Duration::from_secs(3),
                                        );
                                    }
                                    self.state.last_raw_events_click = None;
                                }
                                crate::components::RawEventsClick::Event(clicked_index) => {
                                    // Check for double-click (same index within 500ms)
                                    let now = Instant::now();
                                    let is_double_click = if let Some((last_time, last_index)) =
                                        self.state.last_raw_events_click
                                    {
                                        last_index == clicked_index
                                            && now.duration_since(last_time)
                                                < Duration::from_millis(500)
                                    } else {
                                        false
                                    };

                                    if is_double_click {
                                        // Double-click: toggle detail panel
                                        session.raw_events_view.toggle_detail();
                                        self.state.last_raw_events_click = None;
                                    } else {
                                        // Single click: just select (already done in handle_click)
                                        self.state.last_raw_events_click =
                                            Some((now, clicked_index));
                                    }
                                }
                            }
                        }
                    }
                    return Ok(effects);
                }
            }
        }

        // Click in chat area - selection handled earlier in the mouse pipeline.
        // Clicking in chat area while in sidebar mode returns to normal.
        if self.state.input_mode == InputMode::SidebarNavigation {
            self.state.input_mode = InputMode::Normal;
            self.state.sidebar_state.set_focused(false);
        }

        Ok(effects)
    }

    /// Check if a point is within a rectangle
    pub(super) fn point_in_rect(x: u16, y: u16, rect: Rect) -> bool {
        x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
    }

    /// Handle click in sidebar area
    pub(super) fn handle_sidebar_click(
        &mut self,
        x: u16,
        y: u16,
        sidebar_area: Rect,
    ) -> Vec<Effect> {
        // Use centralized constant for header height (same as hover hit-testing)
        let tree_start_y = sidebar_area.y.saturating_add(SIDEBAR_HEADER_ROWS);
        if y < tree_start_y {
            return Vec::new(); // Clicked on title or separator
        }

        // Check if clicking on "Add Project" button (when sidebar is empty)
        if let Some(button_area) = self.state.sidebar_state.add_project_button_area {
            if Self::point_in_rect(x, y, button_area) {
                // Trigger new project dialog (same logic as Action::NewProject)
                self.open_project_picker_or_base_dir();
                return Vec::new();
            }
        }

        // Always focus sidebar when clicking on it
        self.state.sidebar_state.set_focused(true);
        self.state.input_mode = InputMode::SidebarNavigation;

        let visual_row = (y - tree_start_y) as usize;
        let scroll_offset = self.state.sidebar_state.tree_state.offset;
        let Some(clicked_index) = self
            .state
            .sidebar_data
            .index_from_visual_row(visual_row, scroll_offset)
        else {
            return Vec::new();
        };

        // Detect double-click (same index within 500ms)
        let now = Instant::now();
        let is_double_click = if let Some((last_time, last_index)) = self.state.last_sidebar_click {
            last_index == clicked_index
                && now.duration_since(last_time) < Duration::from_millis(500)
        } else {
            false
        };

        // Update last click tracking
        self.state.last_sidebar_click = Some((now, clicked_index));

        // Get the node at this index
        if let Some(node) = self.state.sidebar_data.get_at(clicked_index) {
            use crate::components::{ActionType, NodeType};

            // Update selection
            self.state.sidebar_state.tree_state.selected = clicked_index;

            // Handle based on node type
            match node.node_type {
                NodeType::Repository => {
                    // Toggle expand/collapse
                    self.state.sidebar_data.toggle_at(clicked_index);
                }
                NodeType::Workspace => {
                    // Single click: open workspace but keep sidebar open
                    // Double click: open workspace and close sidebar
                    self.open_workspace_with_options(node.id, is_double_click);
                }
                NodeType::Action(ActionType::NewWorkspace) => {
                    // Create new workspace
                    if let Some(parent_id) = node.parent_id {
                        return self.start_workspace_creation(parent_id);
                    }
                }
            }
        }

        Vec::new()
    }

    pub(super) fn build_tab_bar(&self, focused: bool) -> crate::components::TabBar {
        let sessions = self.state.tab_manager.sessions();
        let mut pr_numbers = Vec::with_capacity(sessions.len());
        let mut processing_flags = Vec::with_capacity(sessions.len());
        let mut attention_flags = Vec::with_capacity(sessions.len());
        let mut awaiting_response_flags = Vec::with_capacity(sessions.len());
        for session in sessions {
            pr_numbers.push(session.pr_number);
            // Don't show processing spinner if awaiting response (inline prompt active)
            let has_inline_prompt = session.inline_prompt.is_some();
            processing_flags.push(session.is_processing && !has_inline_prompt);
            attention_flags.push(session.needs_attention);
            awaiting_response_flags.push(has_inline_prompt);
        }

        crate::components::TabBar::new(
            self.state.tab_manager.tab_names(),
            self.state.tab_manager.active_index(),
        )
        .focused(focused)
        .with_tab_states(
            pr_numbers,
            processing_flags,
            attention_flags,
            awaiting_response_flags,
        )
        .with_spinner_frame(self.state.spinner_frame)
        .with_scroll_offset(self.state.tab_bar_scroll)
    }

    pub(super) fn ensure_tab_bar_scroll(&mut self, area_width: u16, focused: bool) {
        if self.state.tab_manager.is_empty() {
            self.state.tab_bar_scroll = 0;
            self.state.tab_bar_last_active = None;
            return;
        }

        let tab_bar = self.build_tab_bar(focused);
        let max_scroll = tab_bar.max_scroll(area_width);
        if self.state.tab_bar_scroll > max_scroll {
            self.state.tab_bar_scroll = max_scroll;
        }

        let active = self.state.tab_manager.active_index();
        if self.state.tab_bar_last_active != Some(active) {
            self.state.tab_bar_scroll = tab_bar.adjust_scroll_to_active(area_width).min(max_scroll);
            self.state.tab_bar_last_active = Some(active);
            self.state.tab_bar_active_end = tab_bar.active_tab_end();
        } else {
            // Active tab unchanged — check if it grew (e.g., spinner appeared) and snap right.
            let active_end = tab_bar.active_tab_end();
            if active_end > self.state.tab_bar_active_end {
                let adjusted = tab_bar.adjust_scroll_to_active(area_width).min(max_scroll);
                if adjusted > self.state.tab_bar_scroll {
                    self.state.tab_bar_scroll = adjusted;
                }
            }
            self.state.tab_bar_active_end = active_end;
        }
    }

    pub(super) fn scroll_tab_bar(
        &mut self,
        area_width: u16,
        focused: bool,
        scroll_left: bool,
    ) -> bool {
        let tab_bar = self.build_tab_bar(focused);
        let new_offset = if scroll_left {
            tab_bar.scroll_left(area_width)
        } else {
            tab_bar.scroll_right(area_width)
        };

        if new_offset != self.state.tab_bar_scroll {
            self.state.tab_bar_scroll = new_offset;
            return true;
        }

        false
    }

    pub(super) fn handle_tab_bar_wheel(&mut self, x: u16, y: u16, scroll_left: bool) -> bool {
        let Some(tab_bar_area) = self.state.tab_bar_area else {
            return false;
        };
        if !Self::point_in_rect(x, y, tab_bar_area) {
            return false;
        }

        let tabs_focused = self.state.input_mode != InputMode::SidebarNavigation;
        self.scroll_tab_bar(tab_bar_area.width, tabs_focused, scroll_left);
        true
    }

    /// Handle click in tab bar area
    pub(super) fn handle_tab_bar_click(&mut self, x: u16, _y: u16, tab_bar_area: Rect) {
        if self.state.input_mode == InputMode::SidebarNavigation {
            self.state.input_mode = InputMode::Normal;
            self.state.sidebar_state.set_focused(false);
        }

        let tabs_focused = self.state.input_mode != InputMode::SidebarNavigation;
        let tab_bar = self.build_tab_bar(tabs_focused);

        match tab_bar.hit_test(tab_bar_area, x) {
            TabBarHitTarget::Tab(index) => {
                self.state.tab_manager.switch_to(index);
                self.sync_input_mode_for_active_tab();
                self.ensure_tab_bar_scroll(tab_bar_area.width, tabs_focused);
                self.sync_sidebar_to_active_tab();
                self.sync_footer_spinner();
                self.sync_theme_to_active_tab();
            }
            TabBarHitTarget::ScrollLeft => {
                self.scroll_tab_bar(tab_bar_area.width, tabs_focused, true);
            }
            TabBarHitTarget::ScrollRight => {
                self.scroll_tab_bar(tab_bar_area.width, tabs_focused, false);
            }
            TabBarHitTarget::None => {
                if self.state.tab_manager.can_add_tab() {
                    self.state.close_overlays();
                    self.redetect_tools();
                    let default_provider = self
                        .preferred_provider_for_new_sessions()
                        .unwrap_or(self.config().default_agent);
                    self.state
                        .agent_selector_state
                        .show_with_default(default_provider);
                    self.state.input_mode = InputMode::SelectingAgent;
                }
            }
        }
    }

    /// Handle click in input area
    pub(super) fn handle_input_click(&mut self, x: u16, y: u16, input_area: Rect) {
        // Switch to normal mode if we were in sidebar navigation
        if self.state.input_mode == InputMode::SidebarNavigation {
            self.state.input_mode = InputMode::Normal;
            self.state.sidebar_state.set_focused(false);
        }

        // Position cursor based on click
        if let Some(session) = self.state.tab_manager.active_session_mut() {
            session.input_box.set_cursor_from_click(x, y, input_area);
        }
    }

    /// Handle click in status bar area
    pub(super) fn handle_status_bar_click(
        &mut self,
        x: u16,
        _y: u16,
        status_bar_area: Rect,
    ) -> Option<Effect> {
        // Status bar format (with plan mode): "  Build  ModelName Agent"
        // Status bar format (without plan mode): "  ModelName Agent"
        //
        // Layout with positions:
        // - 2 chars: leading spaces
        // - For plan mode: 5 chars ("Build") or 4 chars ("Plan") + 2 chars separator
        // - Model name (variable length)
        // - 1 char space + Agent name

        let relative_x = x.saturating_sub(status_bar_area.x) as usize;

        // Extract info from session in a limited scope
        let (show_mode, mode_width, model_width, agent_width, model, agent_type, shell_mode) = {
            let session = self.state.tab_manager.active_session()?;

            let show_mode = session.capabilities.supports_plan_mode;
            let mode_width = if show_mode {
                session.agent_mode.display_name().len()
            } else {
                0
            };

            // Calculate model display name
            let shell_mode = session.input_box.is_shell_mode();
            let model_display = if shell_mode {
                "Shell".to_string()
            } else {
                let model_id = session.model.clone().unwrap_or_else(|| {
                    conduit_agent::ModelRegistry::default_model(session.agent_type)
                });
                conduit_agent::ModelRegistry::find_model(session.agent_type, &model_id)
                    .map(|m| m.display_name.to_string())
                    .unwrap_or(model_id)
            };
            let model_width = model_display.len();

            let agent_display = session.agent_type.display_name();
            let agent_width = agent_display.len();
            let model = session.model.clone();
            let agent_type = session.agent_type;

            (
                show_mode,
                mode_width,
                model_width,
                agent_width,
                model,
                agent_type,
                shell_mode,
            )
        };

        if shell_mode {
            return self.check_pr_badge_click(x, status_bar_area);
        }

        // Calculate positions with 1 char padding on each side
        // Leading spaces: 2 chars
        let leading: usize = 2;

        if show_mode {
            // Mode area: leading + mode_width (with 1 char padding each side)
            let mode_start = leading.saturating_sub(1); // 1 char before
            let mode_end = leading + mode_width + 1; // 1 char after

            // Model/Agent area starts after mode + 2 char separator
            let model_start = leading + mode_width + 2 - 1; // 1 char before model
            let model_end = leading + mode_width + 2 + model_width + 1 + agent_width + 1; // 1 char after agent

            if relative_x >= mode_start && relative_x < mode_end {
                // Click on mode area - toggle mode
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    if session.capabilities.supports_plan_mode {
                        session.agent_mode = session.agent_mode.toggle();
                        session.update_status();
                    }
                }
            } else if relative_x >= model_start && relative_x < model_end && !shell_mode {
                let should_block_model_switch = self
                    .state
                    .tab_manager
                    .active_session()
                    .is_some_and(|session| {
                        session.is_processing
                            || session.tools_in_flight > 0
                            || session.pending_user_message.is_some()
                            || session.inline_prompt.is_some()
                    });
                if should_block_model_switch {
                    self.state.set_timed_footer_message(
                        "Finish the current response before switching models".to_string(),
                        Duration::from_secs(3),
                    );
                } else {
                    // Click on model/agent area - open model selector
                    let mut allowed = self.config().effective_enabled_providers(self.tools());
                    if !allowed.contains(&agent_type) {
                        let tool = Self::required_tool(agent_type);
                        if self.tools().is_available(tool) {
                            allowed.push(agent_type);
                        }
                    }
                    if allowed.is_empty() {
                        self.state.set_timed_footer_message(
                            "No enabled providers available. Use /providers.".to_string(),
                            Duration::from_secs(4),
                        );
                        return self.check_pr_badge_click(x, status_bar_area);
                    }
                    self.state.close_overlays();
                    let defaults = self.model_selector_defaults();
                    self.state
                        .model_selector_state
                        .set_allowed_providers(Some(allowed));
                    self.state.model_selector_state.show(model, defaults);
                    self.state.model_picker_context = ModelPickerContext::SessionSelection;
                    self.state.input_mode = InputMode::SelectingModel;
                }
            }
        } else {
            // No mode area, just model/agent
            let model_start = leading.saturating_sub(1); // 1 char before model
            let model_end = leading + model_width + 1 + agent_width + 1; // 1 char after agent

            if relative_x >= model_start && relative_x < model_end && !shell_mode {
                let should_block_model_switch = self
                    .state
                    .tab_manager
                    .active_session()
                    .is_some_and(|session| {
                        session.is_processing
                            || session.tools_in_flight > 0
                            || session.pending_user_message.is_some()
                            || session.inline_prompt.is_some()
                    });
                if should_block_model_switch {
                    self.state.set_timed_footer_message(
                        "Finish the current response before switching models".to_string(),
                        Duration::from_secs(3),
                    );
                } else {
                    let mut allowed = self.config().effective_enabled_providers(self.tools());
                    if !allowed.contains(&agent_type) {
                        let tool = Self::required_tool(agent_type);
                        if self.tools().is_available(tool) {
                            allowed.push(agent_type);
                        }
                    }
                    if allowed.is_empty() {
                        self.state.set_timed_footer_message(
                            "No enabled providers available. Use /providers.".to_string(),
                            Duration::from_secs(4),
                        );
                        return self.check_pr_badge_click(x, status_bar_area);
                    }
                    self.state.close_overlays();
                    let defaults = self.model_selector_defaults();
                    self.state
                        .model_selector_state
                        .set_allowed_providers(Some(allowed));
                    self.state.model_selector_state.show(model, defaults);
                    self.state.model_picker_context = ModelPickerContext::SessionSelection;
                    self.state.input_mode = InputMode::SelectingModel;
                }
            }
        }

        // Check for PR badge click on the right side
        self.check_pr_badge_click(x, status_bar_area)
    }

    /// Check if click is on the PR badge and return an effect to open PR in browser
    pub(super) fn check_pr_badge_click(&self, x: u16, status_bar_area: Rect) -> Option<Effect> {
        // Get PR info and calculate right content width from current session
        let session = self.state.tab_manager.active_session()?;

        let working_dir = session.working_dir.clone()?;

        // If no PR, nothing to click
        let num = session.pr_number?;

        // Calculate PR badge width: " PR #N " = 5 + digits + 1
        let pr_badge_width = 5 + num.to_string().len() + 1;

        // Calculate total right content width to find where it starts
        // Format: [PR badge] [· +N -M] [· branch] [  ]
        let mut right_content_width = pr_badge_width;

        // Git stats (if any)
        let stats = session.status_bar.git_diff_stats();
        if stats.has_changes() {
            right_content_width += 3; // " · "
            if stats.additions > 0 {
                right_content_width += 1 + stats.additions.to_string().len(); // "+N"
            }
            if stats.additions > 0 && stats.deletions > 0 {
                right_content_width += 1; // " "
            }
            if stats.deletions > 0 {
                right_content_width += 1 + stats.deletions.to_string().len(); // "-N"
            }
        }

        // Branch name
        if let Some(branch) = session.status_bar.branch_name() {
            right_content_width += 3; // " · "
            right_content_width += branch.len();
        }

        // Trailing padding
        right_content_width += 2;

        // Calculate where right content starts
        let status_width = status_bar_area.width as usize;
        if right_content_width > status_width {
            return None; // Content doesn't fit
        }

        let right_start_x = status_bar_area.x + (status_width - right_content_width) as u16;
        let pr_badge_end_x = right_start_x + pr_badge_width as u16;

        // Check if click is within PR badge
        if x >= right_start_x && x < pr_badge_end_x {
            Some(Effect::OpenPrInBrowser { working_dir })
        } else {
            None
        }
    }

    /// Handle click in model selector dialog
    pub(super) fn handle_model_selector_click(&mut self, x: u16, y: u16) -> Vec<Effect> {
        let mut effects = Vec::new();
        const DIALOG_WIDTH: u16 = 60;
        const DIALOG_HEIGHT: u16 = 18;

        let terminal_size = crossterm::terminal::size().unwrap_or((80, 24));
        let screen = Rect::new(0, 0, terminal_size.0, terminal_size.1);

        let dialog_width = DIALOG_WIDTH.min(screen.width.saturating_sub(4));
        let dialog_height = DIALOG_HEIGHT.min(screen.height.saturating_sub(2));
        let dialog_x = (screen.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = (screen.height.saturating_sub(dialog_height)) / 2;

        let dialog_area = Rect {
            x: dialog_x,
            y: dialog_y,
            width: dialog_width,
            height: dialog_height,
        };

        if x < dialog_area.x
            || x >= dialog_area.x + dialog_area.width
            || y < dialog_area.y
            || y >= dialog_area.y + dialog_area.height
        {
            self.state.model_selector_state.hide();
            if self.state.model_picker_context == ModelPickerContext::OnboardingDefaultSelection {
                self.state.pending_new_project_target = None;
            } else if self.state.model_picker_context
                == ModelPickerContext::SettingsDefaultSelection
            {
                self.state.model_picker_context = ModelPickerContext::SessionSelection;
                self.reopen_settings_menu();
                return effects;
            } else if self.state.model_picker_context == ModelPickerContext::WorkspaceReadyConfig
                || self.state.model_picker_context
                    == ModelPickerContext::WorkspaceReadyAdversarialConfig
            {
                self.state.model_picker_context = ModelPickerContext::SessionSelection;
                self.state.input_mode = InputMode::CreatingWorkspace;
                return effects;
            }
            self.state.model_picker_context = ModelPickerContext::SessionSelection;
            self.state.input_mode = InputMode::Normal;
            return effects;
        }

        let inner = dialog_content_area(dialog_area);

        if inner.height < 4 {
            return effects;
        }

        // Layout: search, separator, list, instructions
        let list_y = inner.y + 2;
        let list_height = inner.height.saturating_sub(3);

        if y >= list_y && y < list_y + list_height {
            let clicked_row = (y - list_y) as usize;
            if self.state.model_selector_state.select_at_row(clicked_row) {
                if let Some(model) = self.state.model_selector_state.selected_model().cloned() {
                    let required_tool = Self::required_tool(model.agent_type);
                    if !self.tools().is_available(required_tool) {
                        self.show_missing_tool(
                            required_tool,
                            format!(
                                "{} is required to use this model.",
                                required_tool.display_name()
                            ),
                        );
                        return effects;
                    }

                    if self.state.model_picker_context
                        == ModelPickerContext::OnboardingDefaultSelection
                    {
                        if self.persist_default_model_selection(&model) {
                            self.state.model_selector_state.hide();
                            self.state.model_picker_context = ModelPickerContext::SessionSelection;
                            self.continue_new_project_flow();
                        }
                        return effects;
                    }

                    if self.state.model_picker_context
                        == ModelPickerContext::SettingsDefaultSelection
                    {
                        if self.persist_default_model_selection(&model) {
                            self.state.model_selector_state.hide();
                            self.state.model_picker_context = ModelPickerContext::SessionSelection;
                            self.reopen_settings_menu();
                        }
                        return effects;
                    }

                    if self.state.model_picker_context == ModelPickerContext::HandoffSelection {
                        self.state.model_selector_state.hide();
                        self.state.model_picker_context = ModelPickerContext::SessionSelection;
                        self.state.input_mode = InputMode::Normal;
                        match self.execute_handoff_session(model.agent_type, model.id.clone()) {
                            Ok(new_effects) => effects.extend(new_effects),
                            Err(err) => self.show_error("Handoff Failed", &err.to_string()),
                        }
                        return effects;
                    }

                    if self.state.model_picker_context == ModelPickerContext::WorkspaceReadyConfig {
                        self.state
                            .workspace_progress_dialog_state
                            .update_model(model.id.clone());
                        self.state.model_selector_state.hide();
                        self.state.model_picker_context = ModelPickerContext::SessionSelection;
                        self.state.input_mode = InputMode::CreatingWorkspace;
                        return effects;
                    }

                    if self.state.model_picker_context
                        == ModelPickerContext::WorkspaceReadyAdversarialConfig
                    {
                        self.state
                            .workspace_progress_dialog_state
                            .update_adversarial_model(model.id.clone());
                        self.state.model_selector_state.hide();
                        self.state.model_picker_context = ModelPickerContext::SessionSelection;
                        self.state.input_mode = InputMode::CreatingWorkspace;
                        return effects;
                    }

                    if let Some(session) = self.state.tab_manager.active_session_mut() {
                        if Self::reject_cross_agent_switch(session, model.agent_type) {
                            return effects;
                        }
                        let agent_changed =
                            session.set_agent_and_model(model.agent_type, Some(model.id.clone()));

                        let msg = if agent_changed {
                            format!(
                                "Switched to {} with model: {}",
                                model.agent_type, model.display_name
                            )
                        } else {
                            format!("Model changed to: {}", model.display_name)
                        };
                        let display = MessageDisplay::System { content: msg };
                        session.chat_view.push(display.to_chat_message());
                    }
                }
                self.state.model_selector_state.hide();
                self.state.model_picker_context = ModelPickerContext::SessionSelection;
                self.state.input_mode = InputMode::Normal;
            }
        }

        effects
    }

    pub(super) fn handle_reasoning_selector_click(&mut self, x: u16, y: u16) {
        const DIALOG_WIDTH: u16 = 58;
        const DIALOG_HEIGHT: u16 = 14;

        let terminal_size = crossterm::terminal::size().unwrap_or((80, 24));
        let screen = Rect::new(0, 0, terminal_size.0, terminal_size.1);

        let dialog_width = DIALOG_WIDTH.min(screen.width.saturating_sub(4));
        let dialog_height = DIALOG_HEIGHT.min(screen.height.saturating_sub(2));
        let dialog_x = (screen.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = (screen.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect {
            x: dialog_x,
            y: dialog_y,
            width: dialog_width,
            height: dialog_height,
        };

        if !Self::point_in_rect(x, y, dialog_area) {
            self.state.reasoning_selector_state.hide();
            self.state.input_mode = InputMode::Normal;
            return;
        }

        let inner = dialog_content_area(dialog_area);
        if inner.height < 4 {
            return;
        }

        // Layout: search, separator, list, hint
        let list_y = inner.y + 2;
        let list_height = inner.height.saturating_sub(3);
        self.state
            .reasoning_selector_state
            .set_max_visible(list_height.saturating_sub(1) as usize);

        if y >= list_y && y < list_y + list_height {
            let clicked_row = (y - list_y) as usize;
            if self
                .state
                .reasoning_selector_state
                .select_at_row(clicked_row)
            {
                if let Some(option) = self.state.reasoning_selector_state.selected_option() {
                    if let Some(session) = self.state.tab_manager.active_session_mut() {
                        if Self::session_started(session) {
                            let display = MessageDisplay::Error {
                                content: "Changing reasoning effort after a session has started is not supported. Start a new session/tab."
                                    .to_string(),
                            };
                            session.chat_view.push(display.to_chat_message());
                            return;
                        }
                        session.set_reasoning_effort(option.effort);
                        let msg = match option.effort {
                            Some(effort) => {
                                format!("Reasoning effort set to: {}", effort.display_name())
                            }
                            None => "Reasoning effort set to: Auto".to_string(),
                        };
                        let display = MessageDisplay::System { content: msg };
                        session.chat_view.push(display.to_chat_message());
                    }
                }
                self.state.reasoning_selector_state.hide();
                self.state.input_mode = InputMode::Normal;
            }
        }
    }

    pub(super) fn handle_orchestration_selector_click(&mut self, x: u16, y: u16) {
        const DIALOG_WIDTH: u16 = 58;
        const DIALOG_HEIGHT: u16 = 9;

        let terminal_size = crossterm::terminal::size().unwrap_or((80, 24));
        let screen = Rect::new(0, 0, terminal_size.0, terminal_size.1);
        let dialog_width = DIALOG_WIDTH.min(screen.width);
        let dialog_height = DIALOG_HEIGHT.min(screen.height);
        let dialog_x = (screen.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = (screen.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect {
            x: dialog_x,
            y: dialog_y,
            width: dialog_width,
            height: dialog_height,
        };

        if !Self::point_in_rect(x, y, dialog_area) {
            self.state.orchestration_selector_state.hide();
            self.state.input_mode = InputMode::Normal;
            return;
        }

        let inner = dialog_content_area(dialog_area);
        if inner.height < 3 {
            return;
        }

        // Layout: list, hint — list occupies all but the last row
        let list_y = inner.y;
        let list_height = inner.height.saturating_sub(1);

        if y >= list_y && y < list_y + list_height {
            let clicked_row = (y - list_y) as usize;
            if self
                .state
                .orchestration_selector_state
                .select_at_row(clicked_row)
            {
                let enabled = self
                    .state
                    .orchestration_selector_state
                    .selected_option()
                    .enabled;
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    session.orchestration_enabled = enabled;
                    let msg = if enabled {
                        "Orchestration mode enabled — sub-agents will be used for exploration and review"
                    } else {
                        "Orchestration mode disabled"
                    };
                    let display = MessageDisplay::System {
                        content: msg.to_string(),
                    };
                    session.chat_view.push(display.to_chat_message());
                    session.update_status();
                }
                self.state.orchestration_selector_state.hide();
                self.state.input_mode = InputMode::Normal;
            }
        }
    }

    pub(super) fn handle_provider_selector_click(&mut self, x: u16, y: u16) {
        let terminal_size = crossterm::terminal::size().unwrap_or((80, 24));
        let screen = Rect::new(0, 0, terminal_size.0, terminal_size.1);
        let dialog_area = ProviderSelector::dialog_area(screen);

        if !Self::point_in_rect(x, y, dialog_area) {
            self.state.provider_selector_state.hide();
            if self.state.model_picker_context == ModelPickerContext::WorkspaceReadyConfig {
                self.state.model_picker_context = ModelPickerContext::SessionSelection;
                self.state.input_mode = InputMode::CreatingWorkspace;
            } else {
                self.state.pending_new_project_target = None;
                if !self.return_to_settings_menu_if_needed() {
                    self.state.input_mode = InputMode::Normal;
                }
            }
            return;
        }

        let list_area = ProviderSelector::list_area(screen);
        if y >= list_area.y && y < list_area.y + list_area.height {
            let clicked_row = (y - list_area.y) as usize;
            if self
                .state
                .provider_selector_state
                .select_at_row(clicked_row)
            {
                if self.state.model_picker_context == ModelPickerContext::WorkspaceReadyConfig {
                    // Single-click confirms in workspace-ready context.
                    if let Some(item) = self.state.provider_selector_state.dialog.selected_item() {
                        let provider = AgentType::parse(&item.id.clone());
                        let default_model = self.config().default_model_for(provider);
                        self.state
                            .workspace_progress_dialog_state
                            .update_provider(provider, default_model);
                    }
                    self.state.provider_selector_state.hide();
                    self.state.model_picker_context = ModelPickerContext::SessionSelection;
                    self.state.input_mode = InputMode::CreatingWorkspace;
                } else {
                    self.state.provider_selector_state.toggle_selected();
                }
            }
        }
    }

    pub(super) fn handle_settings_menu_click(&mut self, x: u16, y: u16) {
        let terminal_size = crossterm::terminal::size().unwrap_or((80, 24));
        let screen = Rect::new(0, 0, terminal_size.0, terminal_size.1);
        let dialog_area = SettingsMenu::dialog_area(screen);

        if !Self::point_in_rect(x, y, dialog_area) {
            self.state.settings_menu_state.hide();
            self.state.input_mode = InputMode::Normal;
            return;
        }

        let list_area = SettingsMenu::list_area(screen);
        if y >= list_area.y && y < list_area.y + list_area.height {
            let clicked_row = (y - list_area.y) as usize;
            if self.state.settings_menu_state.select_at_row(clicked_row) {
                self.open_selected_setting();
            }
        }
    }

    pub(super) fn handle_workspace_defaults_click(&mut self, x: u16, y: u16) {
        let terminal_size = crossterm::terminal::size().unwrap_or((80, 24));
        let screen = Rect::new(0, 0, terminal_size.0, terminal_size.1);
        let dialog_area = WorkspaceDefaultsDialog::dialog_area(screen);

        if !Self::point_in_rect(x, y, dialog_area) {
            self.state.workspace_defaults_dialog_state.hide();
            if !self.return_to_settings_menu_if_needed() {
                self.state.input_mode = InputMode::Normal;
            }
            return;
        }

        let list_area = WorkspaceDefaultsDialog::list_area(screen);
        if y >= list_area.y && y < list_area.y + list_area.height {
            let clicked_row = (y - list_area.y) as usize;
            self.state
                .workspace_defaults_dialog_state
                .select_at_row(clicked_row);
        }
    }

    /// Handle click in project picker dialog
    pub(super) fn handle_project_picker_click(&mut self, x: u16, y: u16) {
        // Calculate dialog position based on terminal size
        // The dialog is 60 wide and centered, height is 7 + list_height
        let terminal_size = crossterm::terminal::size().unwrap_or((80, 24));
        let screen_width = terminal_size.0;
        let screen_height = terminal_size.1;

        let dialog_width: u16 = 60;
        let list_height = self.state.project_picker_state.list.visible_len() as u16;
        let dialog_height = 6 + list_height;

        // Calculate dialog position (centered)
        let dialog_x = screen_width.saturating_sub(dialog_width) / 2;
        let dialog_y = screen_height.saturating_sub(dialog_height) / 2;

        let dialog_area = Rect {
            x: dialog_x,
            y: dialog_y,
            width: dialog_width,
            height: dialog_height,
        };

        let inner = dialog_content_area(dialog_area);

        // List area starts at row 2 within inner area (after search_label, separator)
        // Layout: [0] search label, [1] separator, [2..] list
        let list_y = inner.y + 2;
        let list_height_actual = inner.height.saturating_sub(3);

        // Check if click is in the list area
        if x >= inner.x
            && x < inner.x + inner.width
            && y >= list_y
            && y < list_y + list_height_actual
        {
            // Calculate which row was clicked
            let clicked_row = (y - list_y) as usize;

            // Select the item and trigger double-click detection
            if self.state.project_picker_state.select_at_row(clicked_row) {
                // Check for double-click (would need timing - for now just select)
                // Could add double-click to open in future
            }
        }
    }

    /// Handle click in footer area
    /// Returns an action to execute if a valid hint was clicked
    pub(super) fn handle_footer_click(
        &mut self,
        x: u16,
        _y: u16,
        footer_area: Rect,
    ) -> Option<Action> {
        // Use the same hints as GlobalFooter to stay in sync
        // Sidebar focus takes precedence over file viewer / view_mode
        // Build the footer the same way the renderer does so click positions stay in sync
        use crate::components::FooterContext;
        let context = if self.state.input_mode == InputMode::SidebarNavigation {
            FooterContext::Sidebar
        } else if self.state.tab_manager.active_is_file() {
            FooterContext::FileViewer
        } else {
            match self.state.view_mode {
                ViewMode::Chat => FooterContext::Chat,
                ViewMode::RawEvents => FooterContext::RawEvents,
            }
        };
        let footer =
            GlobalFooter::for_context_with_config(context, &self.config().keybindings.clone());

        // Calculate click position relative to footer
        let relative_x = x.saturating_sub(footer_area.x) as usize;

        // Match the layout from GlobalFooter::render:
        // " [key] action   [key] action ..."
        // Leading space = 1, key has " key " (len+2), action has " action" (len+1), spacing = 3
        let mut current_x: usize = 1; // Leading space

        for (key, action_name) in footer.hints() {
            // Format: " key " (key.len + 2) + " action" (action_name.len + 1) + spacing (3)
            let key_width = key.len() + 2;
            let action_width = action_name.len() + 1;
            let hint_width = key_width + action_width + 3;

            if relative_x >= current_x && relative_x < current_x + hint_width {
                // Clicked on this hint - look up action from keybinding config
                return self.lookup_footer_action(key);
            }
            current_x += hint_width;
        }
        None
    }

    /// Look up the action for a footer key hint using the keybinding config
    pub(super) fn lookup_footer_action(&self, key: &str) -> Option<Action> {
        // Handle compound keys like "j/k" by taking the first one
        let primary_key = key.split('/').next().unwrap_or(key);

        // Special case for "CR" which should be "<CR>"
        let key_notation = if primary_key == "CR" {
            "<CR>".to_string()
        } else {
            primary_key.to_string()
        };

        // Parse the key notation
        let key_combo = parse_key_notation(&key_notation).ok()?;

        // Determine context from current mode and active tab type.
        let context = self.key_context_for_active_tab();

        // Look up action in keybinding config
        self.config()
            .keybindings
            .get_action(&key_combo, context)
            .cloned()
    }

    pub(super) fn key_context_for_active_tab(&self) -> KeyContext {
        match self.state.input_mode {
            InputMode::Normal | InputMode::Scrolling | InputMode::FileViewer => {
                if self.state.tab_manager.active_is_file() {
                    KeyContext::FileViewer
                } else {
                    KeyContext::from_input_mode(self.state.input_mode, self.state.view_mode)
                }
            }
            _ => KeyContext::from_input_mode(self.state.input_mode, self.state.view_mode),
        }
    }
}
