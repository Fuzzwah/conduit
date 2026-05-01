use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

use crate::agent::{AgentMode, AgentType, MessageDisplay};
use crate::config::{remove_keybinding, save_keybinding, Config, KeyCombo, KeyContext};
use crate::ui::action::Action;
use crate::ui::app::App;
use crate::ui::components::{
    build_keybinding_items, ConflictPending, KeybindingItem, SIDEBAR_HEADER_ROWS,
};
use crate::ui::effect::Effect;
use crate::ui::events::{InputMode, ViewMode};
use crate::ui::terminal_guard::TerminalGuard;

impl App {
    pub(super) async fn handle_input_event(
        &mut self,
        input: Event,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        guard: &mut TerminalGuard,
    ) -> anyhow::Result<Vec<Effect>> {
        match input {
            Event::Key(key) => self.handle_key_event(key, terminal, guard).await,
            Event::Mouse(mouse) => self.handle_mouse_event(mouse, terminal, guard).await,
            Event::Paste(text) => {
                self.handle_paste_input(text);
                Ok(Vec::new())
            }
            Event::Resize(_, _) => {
                terminal.autoresize()?;
                Ok(Vec::new())
            }
            Event::FocusGained | Event::FocusLost => Ok(Vec::new()),
        }
    }

    pub(super) async fn handle_key_event(
        &mut self,
        key: KeyEvent,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        guard: &mut TerminalGuard,
    ) -> anyhow::Result<Vec<Effect>> {
        let mut key = key;

        // If we're buffering a suspected split mouse sequence and a non-character key
        // arrives, flush the buffer so the characters aren't lost.
        if self.state.suspect_mouse_buf.is_some() && !matches!(key.code, KeyCode::Char(_)) {
            let chars: Vec<char> = self.state.suspect_mouse_buf.take().unwrap_or_default();
            if !chars.is_empty() {
                self.flush_suspect_mouse_buf(chars);
            }
        }

        // Some terminals can emit CR/LF as plain chars instead of KeyCode::Enter.
        // Normalize these for consistent keybinding behavior across environments.
        if key.modifiers.is_empty() && matches!(key.code, KeyCode::Char('\r') | KeyCode::Char('\n'))
        {
            key.code = KeyCode::Enter;
        } else if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('m') {
            key.code = KeyCode::Enter;
            key.modifiers = KeyModifiers::NONE;
        }

        // Special handling for modes that bypass normal key processing
        if self.state.input_mode == InputMode::RemovingProject {
            // Ignore all input while removing project
            return Ok(Vec::new());
        }

        if self.state.input_mode == InputMode::CloningRepository {
            // Ignore all input while cloning repository
            return Ok(Vec::new());
        }

        if self.state.input_mode == InputMode::CreatingWorkspace {
            if self.state.workspace_progress_dialog_state.complete
                && matches!(key.code, KeyCode::Enter | KeyCode::Esc)
            {
                return Ok(self.close_workspace_progress_dialog());
            }
            return Ok(Vec::new());
        }

        // Defensive normalization: when an overlay is visible, force the matching
        // input mode so key context lookup targets the active modal.
        if self.state.model_selector_state.is_visible() {
            self.state.input_mode = InputMode::SelectingModel;
        } else if self.state.reasoning_selector_state.is_visible() {
            self.state.input_mode = InputMode::SelectingReasoning;
        } else if self.state.provider_selector_state.is_visible() {
            self.state.input_mode = InputMode::SelectingProviders;
        } else if self.state.settings_menu_state.is_visible() {
            self.state.input_mode = InputMode::SettingsMenu;
        } else if self.state.project_picker_state.is_visible() {
            self.state.input_mode = InputMode::PickingProject;
        } else if self.state.workspace_defaults_dialog_state.is_visible() {
            self.state.input_mode = InputMode::WorkspaceDefaults;
        } else if self.state.rename_project_dialog_state.is_visible() {
            self.state.input_mode = InputMode::RenamingProject;
        } else if self.state.issue_picker_state.visible {
            self.state.input_mode = InputMode::SelectingIssue;
        } else if self.state.scp_command_dialog_state.visible {
            self.state.input_mode = InputMode::ScpCommand;
        } else if self.state.file_picker_dialog_state.is_visible() {
            use crate::ui::components::FilePickerMode;
            self.state.input_mode = match self.state.file_picker_dialog_state.mode {
                FilePickerMode::SelectFile => InputMode::FilePickerSource,
                FilePickerMode::SelectDirectory => InputMode::FilePickerDest,
            };
        } else if self.state.base_dir_dialog_state.path.is_visible() {
            self.state.input_mode = InputMode::SettingBaseDir;
        } else if self.state.add_repo_dialog_state.path.is_visible() {
            self.state.input_mode = InputMode::AddingRepository;
        } else if self.state.keybindings_editor_state.is_visible() {
            // Preserve KeybindingsEditorCapture if already set; otherwise use KeybindingsEditor.
            if self.state.input_mode != InputMode::KeybindingsEditorCapture {
                self.state.input_mode = InputMode::KeybindingsEditor;
            }
        }
        self.sync_input_mode_for_active_tab();

        // If the file mention menu is open but something changed input_mode away from
        // FileMention (e.g. Ctrl+T sidebar toggle), dismiss the menu so it doesn't
        // stay rendered in an unresponsive state.
        if self.state.file_mention_state.is_visible()
            && self.state.input_mode != InputMode::FileMention
        {
            self.state.file_mention_state.hide();
        }

        // Handle issue picker navigation
        if self.state.input_mode == InputMode::SelectingIssue {
            return self.handle_issue_picker_key(key);
        }

        // Handle spec picker navigation
        if self.state.input_mode == InputMode::SelectingSpec {
            return self.handle_spec_picker_key(key);
        }

        // Handle specify (spec-kit) picker navigation
        if self.state.input_mode == InputMode::SelectingSpecifySpec {
            return self.handle_specify_picker_key(key);
        }

        // Handle SCP command dialog (Esc only)
        if self.state.input_mode == InputMode::ScpCommand {
            return self.handle_scp_command_key(key);
        }

        // Handle file picker navigation (intercepted before action dispatch)
        if matches!(
            self.state.input_mode,
            InputMode::FilePickerSource | InputMode::FilePickerDest
        ) {
            return self.handle_file_picker_key(key);
        }

        // Capture mode: bypass all action dispatch and feed the raw key directly.
        if self.state.input_mode == InputMode::KeybindingsEditorCapture {
            return self.handle_keybinding_capture(key).await;
        }

        // Handle Ctrl+C with double-press detection (global)
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            tracing::debug!("Ctrl+C detected, calling handle_ctrl_c_press");
            let effects = self.handle_ctrl_c_press();
            return Ok(effects);
        }

        // Handle inline prompt input (AskUserQuestion, ExitPlanMode)
        if let Some(session) = self.state.tab_manager.active_session_mut() {
            if let Some(ref mut prompt) = session.inline_prompt {
                use crate::ui::components::{PromptAction, PromptResponse};

                match prompt.handle_key(key) {
                    PromptAction::Submit(response) => {
                        let tool_id = prompt.tool_id.clone();
                        let response_clone = response.clone();
                        let prompt_snapshot = prompt.clone();
                        let pending_request_id = session.pending_tool_permissions.remove(&tool_id);
                        let agent_type = session.agent_type;

                        // Clear the inline prompt
                        session.inline_prompt = None;

                        // Handle the response - format as natural language for the model
                        let effects = if let (AgentType::Claude, true, Some(request_id)) = (
                            agent_type,
                            session.agent_input_tx.is_some(),
                            pending_request_id.as_ref(),
                        ) {
                            match response_clone {
                                PromptResponse::AskUserAnswers { answers } => {
                                    let updated_input = Self::build_ask_user_updated_input(
                                        &prompt_snapshot,
                                        &answers,
                                    );
                                    let response_payload = Self::build_permission_allow_response(
                                        updated_input,
                                        Some(&tool_id),
                                    );
                                    self.send_control_response(request_id, response_payload)
                                }
                                PromptResponse::ExitPlanApprove => {
                                    // Switch to Build mode
                                    session.agent_mode = AgentMode::Build;
                                    session.update_status();
                                    let updated_input =
                                        Self::build_exit_plan_updated_input(&prompt_snapshot);
                                    let response_payload = Self::build_permission_allow_response(
                                        updated_input,
                                        Some(&tool_id),
                                    );
                                    self.send_control_response(request_id, response_payload)
                                }
                                PromptResponse::ExitPlanFeedback(feedback) => {
                                    let response_payload = Self::build_permission_deny_response(
                                        format!("User feedback on plan: {}", feedback),
                                        Some(&tool_id),
                                    );
                                    self.send_control_response(request_id, response_payload)
                                }
                            }
                        } else if agent_type == AgentType::Claude
                            && session.agent_input_tx.is_some()
                        {
                            let response_payload = match response_clone {
                                PromptResponse::AskUserAnswers { answers } => {
                                    let updated_input = Self::build_ask_user_updated_input(
                                        &prompt_snapshot,
                                        &answers,
                                    );
                                    Self::build_permission_allow_response(
                                        updated_input,
                                        Some(&tool_id),
                                    )
                                }
                                PromptResponse::ExitPlanApprove => {
                                    // Switch to Build mode
                                    session.agent_mode = AgentMode::Build;
                                    session.update_status();
                                    let updated_input =
                                        Self::build_exit_plan_updated_input(&prompt_snapshot);
                                    Self::build_permission_allow_response(
                                        updated_input,
                                        Some(&tool_id),
                                    )
                                }
                                PromptResponse::ExitPlanFeedback(feedback) => {
                                    Self::build_permission_deny_response(
                                        format!("User feedback on plan: {}", feedback),
                                        Some(&tool_id),
                                    )
                                }
                            };
                            session
                                .pending_tool_permission_responses
                                .insert(tool_id.clone(), response_payload);
                            Vec::new()
                        } else if agent_type == AgentType::Opencode {
                            match response_clone {
                                PromptResponse::AskUserAnswers { answers } => {
                                    let answers_payload = Self::build_opencode_question_answers(
                                        &prompt_snapshot,
                                        &answers,
                                    );
                                    self.send_opencode_question_response(
                                        &tool_id,
                                        Some(answers_payload),
                                    )
                                }
                                PromptResponse::ExitPlanApprove => {
                                    session.agent_mode = AgentMode::Build;
                                    session.update_status();
                                    let (content, tool_use_result) =
                                        Self::build_exit_plan_tool_result(
                                            &prompt_snapshot,
                                            true,
                                            None,
                                        );
                                    self.send_tool_result(&tool_id, content, tool_use_result)
                                }
                                PromptResponse::ExitPlanFeedback(feedback) => {
                                    let (content, tool_use_result) =
                                        Self::build_exit_plan_tool_result(
                                            &prompt_snapshot,
                                            false,
                                            Some(feedback),
                                        );
                                    self.send_tool_result(&tool_id, content, tool_use_result)
                                }
                            }
                        } else {
                            match response_clone {
                                PromptResponse::AskUserAnswers { answers } => {
                                    let (content, tool_use_result) =
                                        Self::build_ask_user_tool_result(
                                            &prompt_snapshot,
                                            &answers,
                                        );
                                    self.send_tool_result(&tool_id, content, tool_use_result)
                                }
                                PromptResponse::ExitPlanApprove => {
                                    // Switch to Build mode
                                    session.agent_mode = AgentMode::Build;
                                    session.update_status();
                                    let (content, tool_use_result) =
                                        Self::build_exit_plan_tool_result(
                                            &prompt_snapshot,
                                            true,
                                            None,
                                        );
                                    self.send_tool_result(&tool_id, content, tool_use_result)
                                }
                                PromptResponse::ExitPlanFeedback(feedback) => {
                                    let (content, tool_use_result) =
                                        Self::build_exit_plan_tool_result(
                                            &prompt_snapshot,
                                            false,
                                            Some(feedback),
                                        );
                                    self.send_tool_result(&tool_id, content, tool_use_result)
                                }
                            }
                        };
                        return Ok(effects);
                    }
                    PromptAction::Cancel => {
                        let tool_id = prompt.tool_id.clone();
                        let pending_request_id = session.pending_tool_permissions.remove(&tool_id);
                        let agent_type = session.agent_type;
                        session.inline_prompt = None;
                        // Send cancellation as clear message
                        let effects = if let (AgentType::Claude, true, Some(request_id)) = (
                            agent_type,
                            session.agent_input_tx.is_some(),
                            pending_request_id.as_ref(),
                        ) {
                            let response_payload = Self::build_permission_deny_response(
                                "User cancelled the prompt.".to_string(),
                                Some(&tool_id),
                            );
                            self.send_control_response(request_id, response_payload)
                        } else if agent_type == AgentType::Claude
                            && session.agent_input_tx.is_some()
                        {
                            let response_payload = Self::build_permission_deny_response(
                                "User cancelled the prompt.".to_string(),
                                Some(&tool_id),
                            );
                            session
                                .pending_tool_permission_responses
                                .insert(tool_id.clone(), response_payload);
                            Vec::new()
                        } else if agent_type == AgentType::Opencode {
                            self.send_opencode_question_response(&tool_id, None)
                        } else {
                            self.send_tool_result(
                                &tool_id,
                                "User cancelled the prompt.".to_string(),
                                None,
                            )
                        };
                        return Ok(effects);
                    }
                    PromptAction::Consumed => {
                        // Key was handled but no action yet
                        return Ok(Vec::new());
                    }
                    PromptAction::NotHandled => {
                        // Fall through to normal handling
                    }
                }
            }
        }

        // Esc exits shell mode back to normal input
        if key.code == KeyCode::Esc
            && !self.has_active_dialog()
            && matches!(
                self.state.input_mode,
                InputMode::Normal | InputMode::Scrolling
            )
        {
            if let Some(session) = self.state.tab_manager.active_session_mut() {
                if session.input_box.is_shell_mode() {
                    session.input_box.set_shell_mode(false);
                    session.update_status();
                    self.state.last_esc_press = None;
                    return Ok(Vec::new());
                }
            }
        }

        // Handle Esc with double-press detection (only when no dialog active and in normal mode)
        if key.code == KeyCode::Esc
            && !self.has_active_dialog()
            && !self.state.show_first_time_splash
            && matches!(
                self.state.input_mode,
                InputMode::Normal | InputMode::Scrolling
            )
        {
            let was_first_press = self.state.last_esc_press.is_none();
            self.handle_esc_press();
            // After a first Esc press with no second press following within the timeout,
            // the next characters might be the tail of a split SGR mouse escape sequence.
            // Start buffering to detect and discard the pattern.
            if was_first_press && self.state.last_esc_press.is_some() {
                self.state.suspect_mouse_buf = Some(Vec::new());
            }
            return Ok(Vec::new());
        }

        // First-time splash screen shortcuts are only active in regular chat/scroll modes.
        // This prevents Enter from being hijacked while an onboarding modal is open.
        if Self::should_handle_first_time_splash_shortcuts(
            self.state.show_first_time_splash,
            self.state.input_mode,
            self.has_active_dialog(),
        ) {
            // Handle Ctrl+P to open command palette
            let is_ctrl_p = (key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('p') | KeyCode::Char('P')))
                || matches!(key.code, KeyCode::Char('\x10')); // ASCII 16 = Ctrl+P
            if is_ctrl_p {
                return self
                    .execute_action(Action::OpenCommandPalette, terminal, guard)
                    .await;
            }
            // Handle Ctrl+N to add new project
            let is_ctrl_n = (key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('n') | KeyCode::Char('N')))
                || matches!(key.code, KeyCode::Char('\x0e'));
            if is_ctrl_n || (key.modifiers.is_empty() && key.code == KeyCode::Enter) {
                return self
                    .execute_action(Action::NewProject, terminal, guard)
                    .await;
            }
            if key.modifiers.is_empty() {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        return self.execute_action(Action::Quit, terminal, guard).await;
                    }
                    _ => {}
                }
            }
        }

        // Handle Ctrl+N and Ctrl+P when tabs are empty (works from any input mode)
        if self.state.tab_manager.is_empty() && !self.state.command_palette_state.is_visible() {
            let is_ctrl_n = (key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('n') | KeyCode::Char('N')))
                || matches!(key.code, KeyCode::Char('\x0e')); // ASCII 14 = Ctrl+N

            if is_ctrl_n {
                return self
                    .execute_action(Action::NewProject, terminal, guard)
                    .await;
            }

            // Handle Ctrl+P for command palette
            let is_ctrl_p = (key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('p') | KeyCode::Char('P')))
                || matches!(key.code, KeyCode::Char('\x10')); // ASCII 16 = Ctrl+P

            if is_ctrl_p {
                return self
                    .execute_action(Action::OpenCommandPalette, terminal, guard)
                    .await;
            }

            // Allow ? to open help from the splash screen
            if key.code == KeyCode::Char('?') && key.modifiers.is_empty() {
                self.state.close_overlays();
                let keybindings = self.config().keybindings.clone();
                self.state.help_dialog_state.show(&keybindings);
                self.state.input_mode = InputMode::ShowingHelp;
                return Ok(vec![]);
            }
        }

        // Image paste: Ctrl+V (Linux/Windows) or Alt+V (macOS terminals report Cmd as Alt)
        // Match either modifier independently (Cmd often maps to Alt in terminal emulators)
        if self.state.input_mode == InputMode::Normal
            && (key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::ALT))
            && matches!(key.code, KeyCode::Char(c) if c.eq_ignore_ascii_case(&'v'))
        {
            if let Some(session) = self.state.tab_manager.active_session_mut() {
                match crate::ui::clipboard_paste::paste_image_to_temp_png() {
                    Ok((path, info)) => {
                        session
                            .input_box
                            .attach_image(path, info.width, info.height);
                    }
                    Err(err) => {
                        let display = MessageDisplay::Error {
                            content: format!("Failed to paste image: {err}"),
                        };
                        session.chat_view.push(display.to_chat_message());
                    }
                }
            }
            return Ok(Vec::new());
        }

        // Global command mode trigger - ':' from most modes enters command mode
        // Only trigger when input box is empty (so pasting "hello:world" doesn't activate command mode)
        // Also skip when inline prompt is active (user should respond to prompt first)
        let active_session = self.state.tab_manager.active_session();
        let has_active_session = active_session.is_some();
        let has_inline_prompt = active_session.is_some_and(|s| s.inline_prompt.is_some());

        // Only enter command mode if the input box is empty and not in shell mode
        let (input_is_empty, shell_mode) = active_session
            .map(|s| (s.input_box.input().is_empty(), s.input_box.is_shell_mode()))
            .unwrap_or((true, false));

        if Self::should_trigger_command_mode(
            key.code,
            key.modifiers,
            self.state.input_mode,
            input_is_empty,
            shell_mode,
            has_inline_prompt,
        ) {
            self.state.command_buffer.clear();
            self.state.input_mode = InputMode::Command;
            return Ok(Vec::new());
        }

        if Self::should_trigger_slash_menu(
            key.code,
            key.modifiers,
            self.state.input_mode,
            input_is_empty,
            shell_mode,
            has_inline_prompt,
            has_active_session,
        ) {
            let trigger = match key.code {
                KeyCode::Char('$') => '$',
                _ => '/',
            };
            self.open_resolver_menu(trigger);
            return Ok(Vec::new());
        }

        if self.state.input_mode == InputMode::SelectingProviders
            && key.modifiers.is_empty()
            && key.code == KeyCode::Char(' ')
        {
            self.state.provider_selector_state.toggle_selected();
            return Ok(Vec::new());
        }

        // Tab toggles Global/Project scope in the theme picker
        if self.state.input_mode == InputMode::SelectingTheme
            && key.modifiers.is_empty()
            && key.code == KeyCode::Tab
        {
            self.state.theme_picker_state.toggle_scope();
            return Ok(Vec::new());
        }

        // Ctrl+D clears the project theme override from within the theme picker
        if self.state.input_mode == InputMode::SelectingTheme
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && key.code == KeyCode::Char('d')
            && self.state.theme_picker_state.can_clear_project_theme()
        {
            self.clear_project_theme();
            return Ok(Vec::new());
        }

        // Get the current context from input mode and active tab type
        let context = self.key_context_for_active_tab();

        // Text input (typing characters) handled specially
        if self.should_handle_as_text_input(&key, context) {
            self.handle_text_input(key);
            return Ok(Vec::new());
        }

        // Convert key event to KeyCombo for lookup
        let key_combo = KeyCombo::from_key_event(&key);

        // Look up action in config (context-specific first, then global)
        if let Some(action) = self.config().keybindings.get_action(&key_combo, context) {
            return self.execute_action(action.clone(), terminal, guard).await;
        }

        Ok(Vec::new())
    }

    pub(super) fn should_handle_first_time_splash_shortcuts(
        show_first_time_splash: bool,
        input_mode: InputMode,
        has_active_dialog: bool,
    ) -> bool {
        show_first_time_splash
            && !has_active_dialog
            && matches!(input_mode, InputMode::Normal | InputMode::Scrolling)
    }

    /// Check if a key event should be handled as text input
    /// Returns true if the key is a printable character without Control/Alt modifiers
    /// and we're in a text-input context
    pub(super) fn should_handle_as_text_input(&self, key: &KeyEvent, context: KeyContext) -> bool {
        // Only handle plain characters (no Ctrl or Alt)
        let has_modifier = key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::ALT);

        if has_modifier {
            return false;
        }

        // Check if this is a character key
        let is_char = matches!(key.code, KeyCode::Char(_));

        if !is_char {
            return false;
        }

        if matches!(context, KeyContext::FileViewer) {
            return false;
        }

        // Only treat as text input in appropriate contexts
        matches!(
            context,
            KeyContext::Chat
                | KeyContext::AddRepository
                | KeyContext::BaseDir
                | KeyContext::ProjectPicker
                | KeyContext::Command
                | KeyContext::HelpDialog
                | KeyContext::SessionImport
                | KeyContext::CommandPalette
                | KeyContext::ThemePicker
                | KeyContext::ModelSelector
        )
    }

    /// Handle text input for text-input contexts
    /// Flush accumulated mouse-sequence suspect buffer into the current input target.
    fn flush_suspect_mouse_buf(&mut self, chars: Vec<char>) {
        // Insert the characters based on the current input mode.
        // This mirrors the per-mode insertion in handle_text_input.
        match self.state.input_mode {
            InputMode::Normal => {
                let mut trigger_file_mention = false;
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    for ch in chars {
                        if ch == '@' && !session.input_box.is_shell_mode() {
                            session.input_box.insert_char('@');
                            trigger_file_mention = true;
                        } else {
                            session.input_box.insert_char(ch);
                        }
                    }
                }
                if trigger_file_mention {
                    self.open_file_mention_menu();
                }
            }
            InputMode::Command => {
                for ch in chars {
                    self.state.command_buffer.push(ch);
                }
            }
            InputMode::ShowingHelp => {
                for ch in chars {
                    self.state.help_dialog_state.insert_char(ch);
                }
            }
            InputMode::AddingRepository => {
                for ch in chars {
                    self.state.add_repo_dialog_state.insert_char(ch);
                }
            }
            InputMode::SettingBaseDir => {
                for ch in chars {
                    self.state.base_dir_dialog_state.insert_char(ch);
                }
            }
            InputMode::PickingProject => {
                for ch in chars {
                    self.state.project_picker_state.insert_char(ch);
                }
            }
            InputMode::ImportingSession => {
                for ch in chars {
                    self.state.session_import_state.insert_char(ch);
                }
            }
            InputMode::CommandPalette => {
                for ch in chars {
                    self.state.command_palette_state.insert_char(ch);
                }
            }
            InputMode::SettingsMenu => {
                for ch in chars {
                    self.state.settings_menu_state.insert_char(ch);
                }
            }
            InputMode::SlashMenu => {
                for ch in chars {
                    self.state.slash_menu_state.insert_char(ch);
                }
            }
            InputMode::FileMention => {
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    for ch in chars {
                        self.state.file_mention_state.insert_char(ch);
                        session.input_box.insert_char(ch);
                    }
                }
            }
            InputMode::MissingTool => {
                for ch in chars {
                    self.state.missing_tool_dialog_state.insert_char(ch);
                }
            }
            InputMode::SelectingTheme => {
                self.state
                    .theme_picker_state
                    .insert_str(&chars.into_iter().collect::<String>());
            }
            InputMode::SelectingModel => {
                self.state
                    .model_selector_state
                    .insert_str(&chars.into_iter().collect::<String>());
            }
            InputMode::SelectingReasoning => {
                self.state
                    .reasoning_selector_state
                    .insert_str(&chars.into_iter().collect::<String>());
            }
            InputMode::SelectingProviders => {
                self.state
                    .provider_selector_state
                    .insert_str(&chars.into_iter().collect::<String>());
            }
            InputMode::RenamingProject => {
                for ch in chars {
                    self.state.rename_project_dialog_state.insert_char(ch);
                }
            }
            InputMode::KeybindingsEditor | InputMode::KeybindingsEditorCapture => {
                for ch in chars {
                    self.state.keybindings_editor_state.insert_filter_char(ch);
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_text_input(&mut self, key: KeyEvent) {
        let KeyCode::Char(c) = key.code else {
            return;
        };

        // Detect and discard split SGR mouse escape sequences.
        // When crossterm's EventStream splits `\x1b[<N;X;YM` across reads, the `\x1b`
        // arrives as an Esc key (handled above), and the remaining characters arrive as
        // individual text input. We buffer the trailing chars and check if they form
        // a valid SGR mouse sequence. If so, discard them entirely.
        if let Some(ref mut buf) = self.state.suspect_mouse_buf {
            if buf.is_empty() && c == '[' {
                // First char of a potential mouse sequence — keep buffering
                buf.push(c);
                return;
            } else if buf.is_empty() {
                // First char is not '[' — not a mouse sequence, stop suspecting
                self.state.suspect_mouse_buf = None;
                // Fall through to normal handling
            } else if buf.len() == 1 && c == '<' {
                // Second char matches the mouse sequence pattern — keep buffering
                buf.push(c);
                return;
            } else if buf.len() == 1 {
                // Had '[' but next char isn't '<' — flush and stop suspecting
                let saved: Vec<char> = std::mem::take(buf);
                self.state.suspect_mouse_buf = None;
                self.flush_suspect_mouse_buf(saved);
                // Fall through for the current char
            } else {
                // Third+ characters after `[<`
                buf.push(c);

                // Check if this completes a valid SGR mouse sequence
                if (c == 'M' || c == 'm') && buf.len() >= 5 {
                    let inner = &buf[2..buf.len() - 1]; // between `[<` and `M/m`
                    let is_valid = !inner.is_empty()
                        && inner.iter().all(|&ch| ch.is_ascii_digit() || ch == ';')
                        && inner.contains(&';');
                    if is_valid {
                        // Complete match — discard the entire sequence
                        self.state.suspect_mouse_buf = None;
                        return;
                    }
                }

                // Buffer too long or invalid character for this position
                let max_len: usize = 20;
                let invalid_pos =
                    buf.len() >= 3 && c != 'M' && c != 'm' && !c.is_ascii_digit() && c != ';';
                if buf.len() > max_len || invalid_pos {
                    // Doesn't match — flush buffered chars as text
                    let saved: Vec<char> = std::mem::take(buf);
                    self.state.suspect_mouse_buf = None;
                    self.flush_suspect_mouse_buf(saved);
                    return; // Current char already included in saved
                }

                // Still looks like it could be a mouse sequence — keep buffering
                return;
            }
        }

        match self.state.input_mode {
            InputMode::Normal => {
                let mut trigger_file_mention = false;
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    // Note: ':' is handled globally in handle_key_event
                    // Trigger shell mode with leading '!'
                    if c == '!'
                        && session.input_box.input().is_empty()
                        && !session.input_box.is_shell_mode()
                    {
                        session.input_box.set_shell_mode(true);
                        session.update_status();
                        return;
                    }

                    // Check for help trigger (? on empty input)
                    if c == '?'
                        && session.input_box.input().is_empty()
                        && !session.input_box.is_shell_mode()
                    {
                        self.state.close_overlays();
                        let keybindings = self.config().keybindings.clone();
                        self.state.help_dialog_state.show(&keybindings);
                        self.state.input_mode = InputMode::ShowingHelp;
                        return;
                    }

                    // Trigger file mention autocomplete with '@'
                    if c == '@' && !session.input_box.is_shell_mode() {
                        session.input_box.insert_char('@');
                        trigger_file_mention = true;
                    } else {
                        session.input_box.insert_char(c);
                    }
                }
                if trigger_file_mention {
                    self.open_file_mention_menu();
                }
            }
            InputMode::FileMention => {
                self.state.file_mention_state.insert_char(c);
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    session.input_box.insert_char(c);
                }
            }
            InputMode::FileViewer => {}
            InputMode::Command => {
                self.state.command_buffer.push(c);
            }
            InputMode::ShowingHelp => {
                self.state.help_dialog_state.insert_char(c);
            }
            InputMode::AddingRepository => {
                self.state.add_repo_dialog_state.insert_char(c);
            }
            InputMode::RenamingProject => {
                self.state.rename_project_dialog_state.insert_char(c);
            }
            InputMode::SettingBaseDir => {
                self.state.base_dir_dialog_state.insert_char(c);
            }
            InputMode::PickingProject => {
                self.state.project_picker_state.insert_char(c);
            }
            InputMode::ImportingSession => {
                self.state.session_import_state.insert_char(c);
            }
            InputMode::CommandPalette => {
                self.state.command_palette_state.insert_char(c);
            }
            InputMode::SettingsMenu => {
                self.state.settings_menu_state.insert_char(c);
            }
            InputMode::KeybindingsEditor => {
                self.state.keybindings_editor_state.insert_filter_char(c);
            }
            InputMode::SlashMenu => {
                self.state.slash_menu_state.insert_char(c);
            }
            InputMode::MissingTool => {
                self.state.missing_tool_dialog_state.insert_char(c);
            }
            InputMode::SelectingTheme => {
                self.state.theme_picker_state.insert_char(c);
            }
            InputMode::SelectingModel => {
                self.state.model_selector_state.insert_char(c);
            }
            InputMode::SelectingReasoning => {
                self.state.reasoning_selector_state.insert_char(c);
            }
            InputMode::SelectingProviders => {
                self.state.provider_selector_state.insert_char(c);
            }
            _ => {}
        }
    }

    pub(super) fn handle_paste_input(&mut self, pasted: String) {
        // Normalize line endings: CRLF → LF, then lone CR → LF
        let pasted = pasted.replace("\r\n", "\n").replace('\r', "\n");
        match self.state.input_mode {
            InputMode::Normal => {
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    // If an inline prompt is in text-input mode, paste into it instead.
                    if session.inline_prompt.as_ref().is_some_and(|p| p.input_mode) {
                        let sanitized = pasted.replace('\n', " ");
                        if let Some(ref mut prompt) = session.inline_prompt {
                            for ch in sanitized.chars() {
                                prompt.text_input.insert_char(ch);
                            }
                        }
                        return;
                    }
                    let mut sanitized = pasted;
                    if session.input_box.input().is_empty()
                        && !session.input_box.is_shell_mode()
                        && sanitized.starts_with('!')
                    {
                        session.input_box.set_shell_mode(true);
                        session.update_status();
                        if let Some(stripped) = sanitized.strip_prefix('!') {
                            sanitized = stripped.to_string();
                        }
                        if sanitized.is_empty() {
                            return;
                        }
                    }
                    session.input_box.handle_paste(sanitized);
                }
            }
            InputMode::FileViewer => {}
            InputMode::Command => {
                let sanitized = pasted.replace('\n', " ");
                self.state.command_buffer.push_str(&sanitized);
            }
            InputMode::ShowingHelp => {
                let sanitized = pasted.replace('\n', " ");
                for ch in sanitized.chars() {
                    self.state.help_dialog_state.insert_char(ch);
                }
            }
            InputMode::AddingRepository => {
                let sanitized = pasted.replace('\n', " ");
                for ch in sanitized.chars() {
                    self.state.add_repo_dialog_state.insert_char(ch);
                }
            }
            InputMode::SettingBaseDir => {
                let sanitized = pasted.replace('\n', " ");
                for ch in sanitized.chars() {
                    self.state.base_dir_dialog_state.insert_char(ch);
                }
            }
            InputMode::PickingProject => {
                let sanitized = pasted.replace('\n', " ");
                for ch in sanitized.chars() {
                    self.state.project_picker_state.insert_char(ch);
                }
            }
            InputMode::ImportingSession => {
                let sanitized = pasted.replace('\n', " ");
                for ch in sanitized.chars() {
                    self.state.session_import_state.insert_char(ch);
                }
            }
            InputMode::CommandPalette => {
                let sanitized = pasted.replace('\n', " ");
                for ch in sanitized.chars() {
                    self.state.command_palette_state.insert_char(ch);
                }
            }
            InputMode::SettingsMenu => {
                let sanitized = pasted.replace('\n', " ");
                for ch in sanitized.chars() {
                    self.state.settings_menu_state.insert_char(ch);
                }
            }
            InputMode::KeybindingsEditor => {
                let sanitized = pasted.replace('\n', " ");
                for ch in sanitized.chars() {
                    self.state.keybindings_editor_state.insert_filter_char(ch);
                }
            }
            InputMode::SlashMenu => {
                let sanitized = pasted.replace('\n', " ");
                for ch in sanitized.chars() {
                    self.state.slash_menu_state.insert_char(ch);
                }
            }
            InputMode::FileMention => {
                let sanitized = pasted.replace('\n', " ");
                for ch in sanitized.chars() {
                    self.state.file_mention_state.insert_char(ch);
                    if let Some(session) = self.state.tab_manager.active_session_mut() {
                        session.input_box.insert_char(ch);
                    }
                }
            }
            InputMode::MissingTool => {
                let sanitized = pasted.replace('\n', " ");
                for ch in sanitized.chars() {
                    self.state.missing_tool_dialog_state.insert_char(ch);
                }
            }
            InputMode::SelectingTheme => {
                let sanitized = pasted.replace('\n', " ");
                self.state.theme_picker_state.insert_str(&sanitized);
            }
            InputMode::SelectingModel => {
                let sanitized = pasted.replace('\n', " ");
                self.state.model_selector_state.insert_str(&sanitized);
            }
            InputMode::SelectingReasoning => {
                let sanitized = pasted.replace('\n', " ");
                self.state.reasoning_selector_state.insert_str(&sanitized);
            }
            InputMode::SelectingProviders => {
                let sanitized = pasted.replace('\n', " ");
                self.state.provider_selector_state.insert_str(&sanitized);
            }
            _ => {}
        }
    }

    pub(super) async fn handle_mouse_event(
        &mut self,
        mouse: MouseEvent,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        guard: &mut TerminalGuard,
    ) -> anyhow::Result<Vec<Effect>> {
        let x = mouse.column;
        let y = mouse.row;

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                // Route scroll to appropriate component based on mode
                if self.state.input_mode == InputMode::ShowingHelp {
                    self.state.help_dialog_state.scroll_up(3);
                } else if self.state.input_mode == InputMode::PickingProject
                    && self.state.project_picker_state.is_visible()
                {
                    self.state.project_picker_state.select_prev();
                } else if self.state.input_mode == InputMode::ImportingSession
                    && self.state.session_import_state.is_visible()
                {
                    self.state.session_import_state.select_prev();
                } else if self.state.input_mode == InputMode::SettingsMenu
                    && self.state.settings_menu_state.is_visible()
                {
                    self.state.settings_menu_state.select_prev();
                } else if self.state.input_mode == InputMode::SelectingTheme
                    && self.state.theme_picker_state.is_visible()
                {
                    self.state.theme_picker_state.select_prev();
                } else if self.state.input_mode == InputMode::SelectingReasoning
                    && self.state.reasoning_selector_state.is_visible()
                {
                    self.state.reasoning_selector_state.select_previous();
                } else if self.state.input_mode == InputMode::SelectingProviders
                    && self.state.provider_selector_state.is_visible()
                {
                    self.state.provider_selector_state.select_previous();
                } else if self.handle_tab_bar_wheel(x, y, true) {
                    return Ok(Vec::new());
                } else if let Some(file_session) = self.state.tab_manager.active_file_viewer_mut() {
                    file_session.scroll_up(1);
                    self.record_scroll(1);
                } else if self.state.view_mode == ViewMode::RawEvents {
                    if let Some(session) = self.state.tab_manager.active_session_mut() {
                        if session.raw_events_view.is_detail_visible() {
                            session.raw_events_view.event_detail.scroll_up(3);
                        } else {
                            session.raw_events_view.scroll_up(3);
                        }
                    }
                    self.record_scroll(3);
                } else if let Some(session) = self.state.tab_manager.active_session_mut() {
                    session.chat_view.scroll_up(1);
                    self.record_scroll(1);
                }
                Ok(Vec::new())
            }
            MouseEventKind::ScrollDown => {
                // Route scroll to appropriate component based on mode
                if self.state.input_mode == InputMode::ShowingHelp {
                    self.state.help_dialog_state.scroll_down(3);
                } else if self.state.input_mode == InputMode::PickingProject
                    && self.state.project_picker_state.is_visible()
                {
                    self.state.project_picker_state.select_next();
                } else if self.state.input_mode == InputMode::ImportingSession
                    && self.state.session_import_state.is_visible()
                {
                    self.state.session_import_state.select_next();
                } else if self.state.input_mode == InputMode::SettingsMenu
                    && self.state.settings_menu_state.is_visible()
                {
                    self.state.settings_menu_state.select_next();
                } else if self.state.input_mode == InputMode::SelectingTheme
                    && self.state.theme_picker_state.is_visible()
                {
                    self.state.theme_picker_state.select_next();
                } else if self.state.input_mode == InputMode::SelectingReasoning
                    && self.state.reasoning_selector_state.is_visible()
                {
                    self.state.reasoning_selector_state.select_next();
                } else if self.state.input_mode == InputMode::SelectingProviders
                    && self.state.provider_selector_state.is_visible()
                {
                    self.state.provider_selector_state.select_next();
                } else if self.handle_tab_bar_wheel(x, y, false) {
                    return Ok(Vec::new());
                } else if let Some(file_session) = self.state.tab_manager.active_file_viewer_mut() {
                    file_session.scroll_down(1);
                    self.record_scroll(1);
                } else if self.state.view_mode == ViewMode::RawEvents {
                    let list_height = self.raw_events_list_visible_height();
                    let detail_height = self.raw_events_detail_visible_height();
                    if let Some(session) = self.state.tab_manager.active_session_mut() {
                        if session.raw_events_view.is_detail_visible() {
                            let content_height = session.raw_events_view.detail_content_height();
                            session.raw_events_view.event_detail.scroll_down(
                                3,
                                content_height,
                                detail_height,
                            );
                        } else {
                            session.raw_events_view.scroll_down(3, list_height);
                        }
                    }
                    self.record_scroll(3);
                } else if let Some(session) = self.state.tab_manager.active_session_mut() {
                    session.chat_view.scroll_down(1);
                    self.record_scroll(1);
                }
                Ok(Vec::new())
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if self.handle_scrollbar_press(x, y) {
                    return Ok(Vec::new());
                }
                if self.handle_selection_start(x, y) {
                    return Ok(Vec::new());
                }
                // Handle left clicks based on position
                self.handle_mouse_click(x, y, terminal, guard).await
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.handle_scrollbar_drag(y) {
                    return Ok(Vec::new());
                }
                if self.handle_selection_drag(x, y) {
                    return Ok(Vec::new());
                }
                Ok(Vec::new())
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.state.scroll_drag = None;
                if let Some(mut effects) = self.handle_selection_end() {
                    // If no selection was made (simple click), check for clickable file paths
                    if effects.is_empty() {
                        if let Some(path_effects) = self.handle_file_path_click(x, y) {
                            effects.extend(path_effects);
                        }
                    }
                    return Ok(effects);
                }
                // No selection was active - still check for file path clicks
                // (e.g., clicking in prompt areas where selection isn't started)
                if let Some(path_effects) = self.handle_file_path_click(x, y) {
                    return Ok(path_effects);
                }
                Ok(Vec::new())
            }
            MouseEventKind::Moved => {
                // Update hover state for sidebar workspace name expansion
                if let Some(sidebar_area) = self.state.sidebar_area {
                    // Tree view starts after header (uses centralized constant for consistency)
                    let tree_start_y = sidebar_area.y.saturating_add(SIDEBAR_HEADER_ROWS);
                    // Sidebar has no borders - tree renders directly in content area
                    let inner_x = sidebar_area.x;
                    let inner_width = sidebar_area.width as usize;

                    if Self::point_in_rect(x, y, sidebar_area) && y >= tree_start_y {
                        // Calculate visual row within the tree view
                        let visual_row = (y - tree_start_y) as usize;
                        // Calculate x position within the tree inner area
                        let x_in_tree = x.saturating_sub(inner_x) as usize;
                        let scroll_offset = self.state.sidebar_state.tree_state.offset;

                        // Check if hovering over a workspace name (not git stats or PR)
                        if let Some(workspace_id) = self.state.sidebar_data.workspace_at_name_line(
                            visual_row,
                            x_in_tree,
                            scroll_offset,
                            inner_width,
                        ) {
                            self.state.sidebar_state.tree_state.set_hover(workspace_id);
                        } else {
                            self.state.sidebar_state.tree_state.clear_hover();
                        }
                    } else {
                        // Mouse left sidebar, clear hover
                        self.state.sidebar_state.tree_state.clear_hover();
                    }
                }

                // Update hover state for raw events session ID
                if self.state.view_mode == ViewMode::RawEvents {
                    if let Some(raw_events_area) = self.state.raw_events_area {
                        if let Some(session) = self.state.tab_manager.active_session_mut() {
                            let hover_changed = session.raw_events_view.update_session_id_hover(
                                x,
                                y,
                                raw_events_area,
                            );

                            if hover_changed {
                                // Check if now hovering (need to re-check since we mutated)
                                let is_hovered = session.raw_events_view.is_session_id_hovered();
                                if is_hovered {
                                    self.state.set_footer_message(Some(
                                        "Click session ID to copy".to_string(),
                                    ));
                                } else {
                                    // Clear the hint message when no longer hovering
                                    self.state.set_footer_message(None);
                                }
                            }
                        }
                    }
                }

                // Update hover state for clickable file paths in chat view
                if self.state.view_mode == ViewMode::Chat {
                    if let Some(chat_area) = self.state.chat_area {
                        let show_chat_scrollbar = self.config().ui.show_chat_scrollbar;
                        if let Some(session) = self.state.tab_manager.active_session_mut() {
                            let hover_changed = session.chat_view.update_file_path_hover(
                                x,
                                y,
                                chat_area,
                                show_chat_scrollbar,
                            );

                            if hover_changed {
                                let is_hovered = session.chat_view.is_file_path_hovered();
                                if is_hovered {
                                    self.state.set_footer_message(Some(
                                        "Click to open file in new tab".to_string(),
                                    ));
                                } else {
                                    // Clear the hint message when no longer hovering
                                    self.state.set_footer_message(None);
                                }
                            }
                        }
                    }
                }
                Ok(Vec::new())
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Handle click on a file path in the chat area
    /// Returns Some(effects) if a file path was clicked and opened
    fn handle_file_path_click(&mut self, x: u16, y: u16) -> Option<Vec<Effect>> {
        // Only handle in chat view mode
        if self.state.view_mode != ViewMode::Chat {
            return None;
        }

        // Get the chat area
        let chat_area = self.state.chat_area?;

        // Check if click is in chat area
        if !Self::point_in_rect(x, y, chat_area) {
            return None;
        }

        // Get the active session and check for file path at click position
        let show_chat_scrollbar = self.config().ui.show_chat_scrollbar;
        let session = self.state.tab_manager.active_session_mut()?;
        let path = session
            .chat_view
            .file_path_at_position(x, y, chat_area, show_chat_scrollbar)?;

        // Try to open the file in a new tab
        let path_buf = std::path::PathBuf::from(&path);
        match self.state.tab_manager.open_file(path_buf.clone()) {
            Ok(_) => {
                self.state.set_timed_footer_message(
                    format!("Opened: {}", path),
                    std::time::Duration::from_secs(2),
                );
                self.sync_input_mode_for_active_tab();
                Some(vec![Effect::SaveSessionState])
            }
            Err(e) => {
                self.state.set_timed_footer_message(
                    format!("Failed to open {}: {}", path, e),
                    std::time::Duration::from_secs(3),
                );
                None
            }
        }
    }

    pub(super) fn confirm_file_picker_copy(&mut self) -> anyhow::Result<Vec<Effect>> {
        // In upload mode, show the SCP command instead of copying a local file
        if self.state.file_picker_dialog_state.upload_mode {
            return self.show_scp_command();
        }

        let source = match self.state.file_picker_dialog_state.source_file.clone() {
            Some(p) => p,
            None => return Ok(Vec::new()),
        };
        let dest_dir = self.state.file_picker_dialog_state.dest_dir().clone();
        let file_name = match source.file_name() {
            Some(n) => n.to_owned(),
            None => return Ok(Vec::new()),
        };
        let dest_path = dest_dir.join(&file_name);

        match std::fs::copy(&source, &dest_path) {
            Ok(_) => {
                self.state.file_picker_dialog_state.hide();
                self.state.input_mode = InputMode::Normal;
                let relative = dest_path
                    .strip_prefix(
                        self.state
                            .file_picker_dialog_state
                            .repo_root
                            .as_deref()
                            .unwrap_or(&dest_dir),
                    )
                    .unwrap_or(&dest_path)
                    .to_string_lossy()
                    .to_string();
                self.state.set_timed_footer_message(
                    format!("Copied {} → {}", file_name.to_string_lossy(), relative),
                    std::time::Duration::from_secs(4),
                );
            }
            Err(err) => {
                self.show_error(
                    "Copy Failed",
                    &format!(
                        "Failed to copy {} to {}: {}",
                        file_name.to_string_lossy(),
                        dest_dir.display(),
                        err
                    ),
                );
            }
        }
        Ok(Vec::new())
    }

    pub(super) fn show_scp_command(&mut self) -> anyhow::Result<Vec<Effect>> {
        let dest_dir = self.state.file_picker_dialog_state.dest_dir().clone();
        let username = self.state.file_picker_dialog_state.upload_username.clone();
        let hostname = self.state.file_picker_dialog_state.upload_hostname.clone();

        // Build relative path for display
        let dest_display = self
            .state
            .file_picker_dialog_state
            .repo_root
            .as_deref()
            .and_then(|root| dest_dir.strip_prefix(root).ok())
            .map(|rel| rel.to_string_lossy().to_string())
            .unwrap_or_else(|| dest_dir.to_string_lossy().to_string());

        let dest_abs = dest_dir.to_string_lossy();
        let scp_command = format!("scp yourfile {}@{}:{}/", username, hostname, dest_abs);

        self.state.file_picker_dialog_state.hide();
        self.state
            .scp_command_dialog_state
            .show(scp_command.clone(), dest_display, dest_dir);
        self.state.input_mode = InputMode::ScpCommand;
        Ok(vec![Effect::CopyToClipboard(scp_command)])
    }

    pub(super) fn handle_scp_command_key(&mut self, key: KeyEvent) -> anyhow::Result<Vec<Effect>> {
        use crate::ui::components::ScpCommandPhase;
        match key.code {
            KeyCode::Enter
                if self.state.scp_command_dialog_state.phase == ScpCommandPhase::ShowCommand =>
            {
                self.state.scp_command_dialog_state.confirm_upload();
            }
            KeyCode::Esc => {
                self.state.scp_command_dialog_state.hide();
                self.state.input_mode = InputMode::Normal;
            }
            _ => {}
        }
        Ok(Vec::new())
    }

    pub(super) fn handle_file_picker_key(&mut self, key: KeyEvent) -> anyhow::Result<Vec<Effect>> {
        use crate::ui::components::{FilePickerEntry, FilePickerMode};

        let is_source = self.state.input_mode == InputMode::FilePickerSource;

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.file_picker_dialog_state.move_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.file_picker_dialog_state.move_down();
            }
            KeyCode::Left | KeyCode::Esc => {
                if is_source {
                    let ascended = self.state.file_picker_dialog_state.ascend();
                    if !ascended {
                        // Already at FS root or couldn't ascend — cancel
                        self.state.file_picker_dialog_state.hide();
                        self.state.input_mode = InputMode::Normal;
                    }
                } else {
                    // In dest mode: Left ascends (clamped at repo root), Esc cancels
                    if key.code == KeyCode::Esc {
                        self.state.file_picker_dialog_state.hide();
                        self.state.input_mode = InputMode::Normal;
                    } else {
                        self.state.file_picker_dialog_state.ascend();
                    }
                }
            }
            KeyCode::Enter => {
                let mode = self.state.file_picker_dialog_state.mode;
                match mode {
                    FilePickerMode::SelectFile => {
                        let selected_path = self.state.file_picker_dialog_state.selected_path();
                        if let Some(path) = selected_path {
                            if path.is_dir() {
                                self.state.file_picker_dialog_state.descend();
                            } else if path.is_file() {
                                // Store source and transition to dest picker
                                self.state.file_picker_dialog_state.source_file = Some(path);
                                self.state.file_picker_dialog_state.show_dest_picker();
                                self.state.input_mode = InputMode::FilePickerDest;
                            }
                        }
                    }
                    FilePickerMode::SelectDirectory => {
                        // Enter descends into the selected subdirectory.
                        // Use 'c' to copy/upload into the current directory.
                        let selected_entry = self
                            .state
                            .file_picker_dialog_state
                            .entries
                            .get(self.state.file_picker_dialog_state.selected)
                            .cloned();
                        match selected_entry {
                            None => {
                                return self.confirm_file_picker_copy();
                            }
                            Some(FilePickerEntry::Dir(_)) => {
                                self.state.file_picker_dialog_state.descend();
                            }
                            Some(FilePickerEntry::File(_, _)) => {
                                // files are shown for context only; use 'c' to select current dir
                            }
                        }
                    }
                }
            }
            // Allow 'c' as a shortcut to confirm copy in dest mode
            KeyCode::Char('c') if !is_source && key.modifiers.is_empty() => {
                return self.confirm_file_picker_copy();
            }
            _ => {}
        }
        Ok(Vec::new())
    }

    pub(super) fn handle_issue_picker_key(&mut self, key: KeyEvent) -> anyhow::Result<Vec<Effect>> {
        // Ignore all input while syncing remote or fetching issues.
        if self.state.issue_picker_state.syncing || self.state.issue_picker_state.loading {
            return Ok(Vec::new());
        }

        let repo_id = self.state.issue_picker_state.repo_id;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.issue_picker_state.select_prev();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.issue_picker_state.select_next();
            }
            KeyCode::Enter => {
                let issue = self.state.issue_picker_state.selected_issue().cloned();
                self.state.issue_picker_state.hide();
                self.state.input_mode = InputMode::SidebarNavigation;
                return Ok(vec![Effect::ShowSpecPicker { repo_id, issue }]);
            }
            KeyCode::Esc => {
                // Skip issue selection — proceed to spec picker with no issue
                self.state.issue_picker_state.hide();
                self.state.input_mode = InputMode::SidebarNavigation;
                return Ok(vec![Effect::ShowSpecPicker {
                    repo_id,
                    issue: None,
                }]);
            }
            _ => {}
        }
        Ok(Vec::new())
    }

    pub(super) fn handle_spec_picker_key(&mut self, key: KeyEvent) -> anyhow::Result<Vec<Effect>> {
        // Ignore all input while specs are still loading to avoid races.
        if self.state.spec_picker_state.loading {
            return Ok(Vec::new());
        }

        let repo_id = self.state.spec_picker_state.repo_id;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.spec_picker_state.select_prev();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.spec_picker_state.select_next();
            }
            KeyCode::Char('s') => {
                self.state.spec_picker_state.cycle_sort();
            }
            KeyCode::Enter => {
                let spec = self.state.spec_picker_state.selected_spec().cloned();
                let issue = self.state.spec_picker_state.issue.clone();
                self.state.spec_picker_state.hide();
                self.state.input_mode = InputMode::SidebarNavigation;
                return Ok(vec![Effect::CreateWorkspace {
                    repo_id,
                    issue,
                    spec,
                    specify_spec: None,
                }]);
            }
            KeyCode::Esc => {
                let issue = self.state.spec_picker_state.issue.clone();
                self.state.spec_picker_state.hide();
                self.state.input_mode = InputMode::SidebarNavigation;
                return Ok(vec![Effect::CreateWorkspace {
                    repo_id,
                    issue,
                    spec: None,
                    specify_spec: None,
                }]);
            }
            _ => {}
        }
        Ok(Vec::new())
    }

    pub(super) fn handle_specify_picker_key(
        &mut self,
        key: KeyEvent,
    ) -> anyhow::Result<Vec<Effect>> {
        if self.state.specify_picker_state.loading {
            return Ok(Vec::new());
        }

        let repo_id = self.state.specify_picker_state.repo_id;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.specify_picker_state.select_prev();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.state.specify_picker_state.select_next();
            }
            KeyCode::Char('s') => {
                self.state.specify_picker_state.cycle_sort();
            }
            KeyCode::Enter => {
                let specify_spec = self.state.specify_picker_state.selected_spec().cloned();
                let issue = self.state.specify_picker_state.issue.clone();
                self.state.specify_picker_state.hide();
                self.state.input_mode = InputMode::SidebarNavigation;
                return Ok(vec![Effect::CreateWorkspace {
                    repo_id,
                    issue,
                    spec: None,
                    specify_spec,
                }]);
            }
            KeyCode::Esc => {
                let issue = self.state.specify_picker_state.issue.clone();
                self.state.specify_picker_state.hide();
                self.state.input_mode = InputMode::SidebarNavigation;
                return Ok(vec![Effect::CreateWorkspace {
                    repo_id,
                    issue,
                    spec: None,
                    specify_spec: None,
                }]);
            }
            _ => {}
        }
        Ok(Vec::new())
    }

    pub(super) async fn handle_keybinding_capture(
        &mut self,
        key: KeyEvent,
    ) -> anyhow::Result<Vec<Effect>> {
        let is_modifier_only = matches!(
            key.code,
            KeyCode::Modifier(_) | KeyCode::CapsLock | KeyCode::ScrollLock | KeyCode::NumLock
        ) || key.code == KeyCode::Null;

        // Esc: if a conflict is pending, dismiss it and stay in capture; otherwise cancel capture.
        if key.code == KeyCode::Esc || is_modifier_only {
            if self
                .state
                .keybindings_editor_state
                .conflict_pending
                .is_some()
            {
                self.state.keybindings_editor_state.conflict_pending = None;
            } else {
                self.state.keybindings_editor_state.cancel_capture();
                self.state.input_mode = InputMode::KeybindingsEditor;
            }
            return Ok(vec![]);
        }

        let Some(item_idx) = self.state.keybindings_editor_state.capture_item_idx else {
            self.state.keybindings_editor_state.cancel_capture();
            self.state.input_mode = InputMode::KeybindingsEditor;
            return Ok(vec![]);
        };

        let item = self.state.keybindings_editor_state.items[item_idx].clone();

        // Enter while conflict is pending: steal the key from the conflicting action.
        if key.code == KeyCode::Enter {
            if let Some(conflict) = self.state.keybindings_editor_state.conflict_pending.take() {
                return self
                    .save_keybinding_and_finish(item, conflict.key_str)
                    .await;
            }
            // Enter with no conflict pending is a no-op (not a valid key to bind).
            return Ok(vec![]);
        }

        // Backspace or Delete: clear the user's custom binding (revert to default).
        // Neither key is bindable as a hotkey.
        let is_clear_key = matches!(key.code, KeyCode::Delete | KeyCode::Backspace);
        if is_clear_key {
            self.state.keybindings_editor_state.conflict_pending = None;
            if item.is_user_override {
                if let Err(e) = remove_keybinding(item.context, item.action_name) {
                    self.state
                        .keybindings_editor_state
                        .set_status(format!("Error resetting: {e}"));
                    self.state.keybindings_editor_state.cancel_capture();
                    self.state.input_mode = InputMode::KeybindingsEditor;
                    return Ok(vec![]);
                }
                self.config_mut().keybindings = Config::load().keybindings;
                let new_items = build_keybinding_items(&self.config().keybindings);
                self.state
                    .keybindings_editor_state
                    .set_status(format!("Reset to default: {}", item.default_key));
                self.state.keybindings_editor_state.refresh_items(new_items);
                self.state.keybindings_editor_state.cancel_capture();
                self.state.input_mode = InputMode::KeybindingsEditor;
            }
            return Ok(vec![]);
        }

        // Any other key while conflict is pending: clear the conflict and treat the
        // new key as a fresh capture attempt (fall through).
        self.state.keybindings_editor_state.conflict_pending = None;

        let combo = KeyCombo::from_key_event(&key);

        // Conflict check: look for another action already bound to this combo.
        let conflict = if let Some(ctx) = item.context {
            self.config()
                .keybindings
                .context
                .get(&ctx)
                .and_then(|m| m.get(&combo))
                .cloned()
        } else {
            self.config().keybindings.global.get(&combo).cloned()
        };

        if let Some(conflicting_action) = conflict {
            if conflicting_action != item.action {
                let conflicting_label = self
                    .state
                    .keybindings_editor_state
                    .items
                    .iter()
                    .find(|i| i.action == conflicting_action)
                    .map(|i| i.action_label.clone())
                    .unwrap_or_else(|| {
                        crate::config::action_to_name(&conflicting_action)
                            .unwrap_or("another action")
                            .to_string()
                    });
                self.state.keybindings_editor_state.conflict_pending = Some(ConflictPending {
                    key_str: combo.to_string(),
                    conflicting_label,
                });
                return Ok(vec![]);
            }
        }

        self.save_keybinding_and_finish(item, combo.to_string())
            .await
    }

    async fn save_keybinding_and_finish(
        &mut self,
        item: KeybindingItem,
        key_str: String,
    ) -> anyhow::Result<Vec<Effect>> {
        if let Err(e) = save_keybinding(item.context, item.action_name, &key_str) {
            self.state
                .keybindings_editor_state
                .set_status(format!("Error saving: {e}"));
            self.state.keybindings_editor_state.cancel_capture();
            self.state.input_mode = InputMode::KeybindingsEditor;
            return Ok(vec![]);
        }
        self.config_mut().keybindings = Config::load().keybindings;
        let new_items = build_keybinding_items(&self.config().keybindings);
        self.state
            .keybindings_editor_state
            .set_status(format!("Saved: {key_str}"));
        self.state.keybindings_editor_state.refresh_items(new_items);
        self.state.keybindings_editor_state.cancel_capture();
        self.state.input_mode = InputMode::KeybindingsEditor;
        Ok(vec![])
    }
}
