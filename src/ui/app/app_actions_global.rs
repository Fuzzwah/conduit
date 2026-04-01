use crate::ui::action::Action;
use crate::ui::app::App;
use crate::ui::app_state::{ModelPickerContext, SelectionDragTarget};
use crate::ui::effect::Effect;
use crate::ui::events::{InputMode, ViewMode};
use std::time::Duration;

impl App {
    pub(super) fn handle_global_action(&mut self, action: Action, effects: &mut Vec<Effect>) {
        match action {
            Action::Quit => {
                self.state.should_quit = true;
                effects.push(Effect::SaveSessionState);
            }
            Action::NewProject => {
                self.open_project_picker_or_base_dir();
            }
            Action::NewWorkspaceUnderCursor => {
                use crate::ui::components::{ActionType, NodeType};

                let sidebar_focused = self.state.sidebar_state.focused;
                let repo_id_from_sidebar = if sidebar_focused {
                    let selected = self.state.sidebar_state.tree_state.selected;
                    self.state
                        .sidebar_data
                        .get_at(selected)
                        .and_then(|node| match node.node_type {
                            NodeType::Repository => Some(node.id),
                            NodeType::Workspace => node.parent_id,
                            NodeType::Action(ActionType::NewWorkspace) => node.parent_id,
                        })
                } else {
                    None
                };

                let repo_id_from_tab = if sidebar_focused {
                    None
                } else {
                    let session = self.state.tab_manager.active_session();
                    let workspace_id = session.and_then(|s| s.workspace_id);
                    match (workspace_id, self.workspace_dao().as_ref()) {
                        (Some(workspace_id), Some(workspace_dao)) => {
                            match workspace_dao.get_by_id(workspace_id) {
                                Ok(Some(workspace)) => Some(workspace.repository_id),
                                Ok(None) => {
                                    tracing::error!(
                                        workspace_id = %workspace_id,
                                        "Workspace not found for active tab"
                                    );
                                    None
                                }
                                Err(err) => {
                                    tracing::error!(
                                        workspace_id = %workspace_id,
                                        error = %err,
                                        "Failed to load workspace for active tab"
                                    );
                                    None
                                }
                            }
                        }
                        _ => None,
                    }
                };

                let repo_id = if sidebar_focused {
                    repo_id_from_sidebar
                } else {
                    repo_id_from_tab
                };

                if let Some(repo_id) = repo_id {
                    if let Some(effect) = self.start_workspace_creation(repo_id) {
                        effects.push(effect);
                    }
                } else {
                    self.state.set_timed_footer_message(
                        "No project selected to create a workspace".to_string(),
                        Duration::from_secs(5),
                    );
                }
            }
            Action::ForkSession => {
                self.initiate_fork_session();
            }
            Action::HandoffSession => {
                self.initiate_handoff_session();
            }
            Action::InterruptAgent => {
                self.interrupt_agent();
            }
            Action::ToggleViewMode => {
                self.state.view_mode = match self.state.view_mode {
                    ViewMode::Chat => ViewMode::RawEvents,
                    ViewMode::RawEvents => ViewMode::Chat,
                };
            }
            Action::ShowModelSelector => {
                if let Some(session) = self.state.tab_manager.active_session() {
                    let model = session.model.clone();
                    let mut allowed = self.config().effective_enabled_providers(self.tools());
                    if !allowed.contains(&session.agent_type) {
                        let tool = Self::required_tool(session.agent_type);
                        if self.tools().is_available(tool) {
                            allowed.push(session.agent_type);
                        }
                    }
                    if allowed.is_empty() {
                        self.state.set_timed_footer_message(
                            "No enabled providers available. Use /providers.".to_string(),
                            Duration::from_secs(4),
                        );
                        return;
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
            Action::ShowReasoningSelector => {
                if let Some(session) = self.state.tab_manager.active_session() {
                    let agent_type = session.agent_type;
                    let reasoning_effort = session.reasoning_effort;
                    if !Self::reasoning_supported(agent_type) {
                        self.state.set_timed_footer_message(
                            "Reasoning effort is not supported for this agent".to_string(),
                            Duration::from_secs(3),
                        );
                    } else {
                        self.state.close_overlays();
                        self.state
                            .reasoning_selector_state
                            .show(agent_type, reasoning_effort);
                        self.state.input_mode = InputMode::SelectingReasoning;
                    }
                }
            }
            Action::ShowThemePicker => {
                self.state.close_overlays();
                let theme_path = self.config().theme_path.clone();
                self.state.theme_picker_state.show(theme_path.as_deref());
                self.state.input_mode = InputMode::SelectingTheme;
            }
            Action::ShowProvidersSelector => {
                self.state.close_overlays();
                self.state.pending_new_project_target = None;
                self.state.provider_selector_state =
                    crate::ui::components::ProviderSelectorState::configure_for(
                        self.config(),
                        self.tools(),
                    );
                self.state.provider_selector_state.show();
                self.state.input_mode = InputMode::SelectingProviders;
            }
            Action::OpenSessionImport => {
                self.state.close_overlays();
                self.state.session_import_state.show();
                self.state.input_mode = InputMode::ImportingSession;
                // Trigger session discovery
                effects.push(Effect::DiscoverSessions);
            }
            Action::ImportSession => {
                if self.state.input_mode == InputMode::ImportingSession {
                    if let Some(session) =
                        self.state.session_import_state.selected_session().cloned()
                    {
                        self.state.session_import_state.hide();
                        self.state.input_mode = InputMode::Normal;
                        effects.push(Effect::ImportSession(session));
                    }
                }
            }
            Action::CycleImportFilter => {
                if self.state.input_mode == InputMode::ImportingSession {
                    self.state.session_import_state.cycle_filter();
                }
            }
            Action::ToggleMetrics => {
                self.state.show_metrics = !self.state.show_metrics;
                // Uncomment to test spinner animation smoothness with Alt+P:
                // if self.state.show_metrics {
                //     self.state
                //         .start_footer_spinner(Some("Testing spinner...".to_string()));
                // } else {
                //     self.state.stop_footer_spinner();
                // }
            }
            Action::ToggleAgentMode => {
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    // Only toggle when agent supports plan mode
                    if session.capabilities.supports_plan_mode {
                        session.agent_mode = session.agent_mode.toggle();
                        session.update_status();
                    }
                }
            }
            Action::DumpDebugState => {
                effects.push(Effect::DumpDebugState);
            }
            Action::CopyWorkspacePath => {
                if let Some(session) = self.state.tab_manager.active_session() {
                    if let Some(working_dir) = &session.working_dir {
                        let path_str = working_dir.display().to_string();
                        effects.push(Effect::CopyToClipboard(path_str.clone()));
                        self.state.set_timed_footer_message(
                            format!("Copied: {}", path_str),
                            Duration::from_secs(10),
                        );
                    }
                }
            }
            Action::CopySelection => {
                let mut copied = false;
                let clear_after_copy = self.config().selection.clear_selection_after_copy;
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    if session.input_box.has_selection() {
                        if let Some(text) = session.input_box.selected_text() {
                            copied = true;
                            effects.push(Effect::CopyToClipboard(text));
                            if clear_after_copy {
                                Self::clear_selection_for_target(
                                    session,
                                    SelectionDragTarget::Input,
                                );
                            }
                        }
                    } else if session.chat_view.has_selection() {
                        if let Some(text) = session.chat_view.copy_selection() {
                            copied = true;
                            effects.push(Effect::CopyToClipboard(text));
                            if clear_after_copy {
                                Self::clear_selection_for_target(
                                    session,
                                    SelectionDragTarget::Chat,
                                );
                            }
                        }
                    }
                }

                if copied {
                    self.state.set_timed_footer_message(
                        "Copied selection".to_string(),
                        Duration::from_secs(5),
                    );
                } else {
                    self.state.set_timed_footer_message(
                        "No selection to copy".to_string(),
                        Duration::from_secs(3),
                    );
                }
            }
            Action::CopyCodeBlock => {
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    match session.chat_view.nearest_code_block_content() {
                        Some((content, idx, total)) => {
                            effects.push(Effect::CopyToClipboard(content));
                            let msg = if total > 1 {
                                format!(
                                    "Copied code block ({} of {}) — scroll to copy others",
                                    idx, total
                                )
                            } else {
                                "Copied code block".to_string()
                            };
                            self.state
                                .set_timed_footer_message(msg, Duration::from_secs(3));
                        }
                        None => {
                            self.state.set_timed_footer_message(
                                "No code block in view".to_string(),
                                Duration::from_secs(2),
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
