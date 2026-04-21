use crate::ui::action::Action;
use crate::ui::app::App;
use crate::ui::app_state::ModelPickerContext;
use crate::ui::events::InputMode;

impl App {
    pub(super) fn handle_dialog_action(&mut self, action: Action) {
        match action {
            Action::Cancel => match self.state.input_mode {
                InputMode::SidebarNavigation => {
                    self.state.input_mode = InputMode::Normal;
                    self.state.sidebar_state.set_focused(false);
                }
                InputMode::SelectingModel => {
                    self.state.model_selector_state.hide();
                    if self.state.model_picker_context
                        == ModelPickerContext::OnboardingDefaultSelection
                    {
                        self.state.pending_new_project_target = None;
                    } else if self.state.model_picker_context
                        == ModelPickerContext::SettingsDefaultSelection
                    {
                        self.state.model_picker_context = ModelPickerContext::SessionSelection;
                        self.reopen_settings_menu();
                        return;
                    } else if self.state.model_picker_context
                        == ModelPickerContext::HandoffSelection
                    {
                        self.state.pending_handoff_request = None;
                    }
                    self.state.model_picker_context = ModelPickerContext::SessionSelection;
                    self.state.input_mode = InputMode::Normal;
                }
                InputMode::SelectingReasoning => {
                    self.state.reasoning_selector_state.hide();
                    self.state.input_mode = InputMode::Normal;
                }
                InputMode::SelectingTheme => {
                    self.state.theme_picker_state.hide(true); // Cancelled - restore original
                    if !self.return_to_settings_menu_if_needed() {
                        self.state.input_mode = InputMode::Normal;
                    }
                }
                InputMode::SelectingAgent => {
                    self.state.agent_selector_state.hide();
                    self.state.input_mode = InputMode::Normal;
                }
                InputMode::SelectingProviders => {
                    self.state.provider_selector_state.hide();
                    self.state.pending_new_project_target = None;
                    if !self.return_to_settings_menu_if_needed() {
                        self.state.input_mode = InputMode::Normal;
                    }
                }
                InputMode::PickingProject => {
                    self.state.project_picker_state.hide();
                    self.state.input_mode = InputMode::Normal;
                }
                InputMode::AddingRepository => {
                    self.state.add_repo_dialog_state.hide();
                    self.state.input_mode = InputMode::Normal;
                }
                InputMode::SettingBaseDir => {
                    self.state.base_dir_dialog_state.hide();
                    self.state.base_dir_dialog_context =
                        crate::ui::app_state::BaseDirDialogContext::ProjectDiscovery;
                    if !self.return_to_settings_menu_if_needed() {
                        self.state.input_mode = InputMode::Normal;
                    }
                }
                InputMode::SettingsMenu => {
                    self.state.settings_menu_state.hide();
                    self.state.input_mode = InputMode::Normal;
                }
                InputMode::Confirming => {
                    if self.is_blocking_confirmation_loading_dialog() {
                        return;
                    }
                    self.state.input_mode = self.dismiss_confirmation_dialog();
                }
                InputMode::ShowingError => {
                    self.state.error_dialog_state.hide();
                    self.state.input_mode = InputMode::Normal;
                }
                InputMode::MissingTool => {
                    self.state.missing_tool_dialog_state.hide();
                    self.state.input_mode = InputMode::Normal;
                }
                InputMode::Scrolling => {
                    self.state.input_mode = InputMode::Normal;
                }
                InputMode::Command => {
                    self.state.command_buffer.clear();
                    self.state.input_mode = InputMode::Normal;
                }
                InputMode::ShowingHelp => {
                    self.state.help_dialog_state.hide();
                    self.state.input_mode = InputMode::Normal;
                }
                InputMode::ImportingSession => {
                    self.state.session_import_state.hide();
                    self.state.input_mode = InputMode::Normal;
                }
                InputMode::CommandPalette => {
                    self.state.command_palette_state.hide();
                    self.state.input_mode = InputMode::Normal;
                }
                InputMode::WorkspaceDefaults => {
                    self.state.workspace_defaults_dialog_state.hide();
                    if !self.return_to_settings_menu_if_needed() {
                        self.state.input_mode = InputMode::Normal;
                    }
                }
                InputMode::RenamingProject => {
                    self.state.rename_project_dialog_state.hide();
                    self.state.input_mode = InputMode::SidebarNavigation;
                }
                InputMode::SlashMenu => {
                    self.state.slash_menu_state.hide();
                    self.state.input_mode = InputMode::Normal;
                }
                InputMode::FileMention => {
                    // ESC closes the menu but keeps '@<filter>' in the input
                    self.state.file_mention_state.hide();
                    self.state.input_mode = InputMode::Normal;
                }
                InputMode::QueueEditing => {
                    self.close_queue_editor();
                }
                _ => {}
            },
            Action::AddRepository => match self.state.input_mode {
                InputMode::SidebarNavigation => {
                    self.state.close_overlays();
                    self.state.add_repo_dialog_state.show();
                    self.state.input_mode = InputMode::AddingRepository;
                }
                InputMode::PickingProject => {
                    self.state.project_picker_state.hide();
                    self.state.close_overlays();
                    self.state.add_repo_dialog_state.show();
                    self.state.input_mode = InputMode::AddingRepository;
                }
                _ => {}
            },
            Action::OpenSettings => {
                if matches!(
                    self.state.input_mode,
                    InputMode::Normal | InputMode::SidebarNavigation | InputMode::Scrolling
                ) {
                    self.open_settings_menu();
                }
            }
            Action::AddFileToProject => {
                if matches!(
                    self.state.input_mode,
                    InputMode::Normal | InputMode::Scrolling
                ) {
                    let workspace_id = self
                        .state
                        .tab_manager
                        .active_session()
                        .and_then(|s| s.workspace_id);
                    if let Some(ws_id) = workspace_id {
                        let repo_info = self.workspace_dao_clone().and_then(|ws_dao| {
                            ws_dao.get_by_id(ws_id).ok().flatten().and_then(|ws| {
                                let repo_id = ws.repository_id;
                                self.repo_dao_clone().and_then(|repo_dao| {
                                    repo_dao
                                        .get_by_id(repo_id)
                                        .ok()
                                        .flatten()
                                        .map(|repo| (repo_id, repo.base_path))
                                })
                            })
                        });
                        if let Some((repo_id, repo_base_path)) = repo_info {
                            let start_dir = dirs::home_dir()
                                .or_else(|| repo_base_path.clone())
                                .unwrap_or_else(|| std::path::PathBuf::from("/"));
                            self.state.file_picker_dialog_state.show_source_picker(
                                repo_id,
                                repo_base_path,
                                start_dir,
                            );
                            self.state.input_mode = InputMode::FilePickerSource;
                        }
                    }
                }
            }
            Action::UploadFileToProject => {
                if matches!(
                    self.state.input_mode,
                    InputMode::Normal | InputMode::Scrolling
                ) {
                    let workspace_id = self
                        .state
                        .tab_manager
                        .active_session()
                        .and_then(|s| s.workspace_id);
                    if let Some(ws_id) = workspace_id {
                        let repo_info = self.workspace_dao_clone().and_then(|ws_dao| {
                            ws_dao.get_by_id(ws_id).ok().flatten().and_then(|ws| {
                                let repo_id = ws.repository_id;
                                self.repo_dao_clone().and_then(|repo_dao| {
                                    repo_dao
                                        .get_by_id(repo_id)
                                        .ok()
                                        .flatten()
                                        .map(|repo| (repo_id, repo.base_path))
                                })
                            })
                        });
                        if let Some((repo_id, repo_base_path)) = repo_info {
                            let username = std::env::var("USER").unwrap_or_default();
                            let hostname = crate::util::conduit_hostname();
                            self.state.file_picker_dialog_state.show_upload_dest_picker(
                                repo_id,
                                repo_base_path,
                                username,
                                hostname,
                            );
                            self.state.input_mode = InputMode::FilePickerDest;
                        }
                    }
                }
            }
            Action::RenameProject => {
                if self.state.input_mode == InputMode::SidebarNavigation {
                    let selected = self.state.sidebar_state.tree_state.selected;
                    if let Some(node) = self.state.sidebar_data.get_at(selected) {
                        use crate::ui::components::NodeType;
                        if node.node_type == NodeType::Repository {
                            let repo_id = node.id;
                            if let Some(dao) = self.repo_dao() {
                                if let Ok(Some(repo)) = dao.get_by_id(repo_id) {
                                    self.state
                                        .rename_project_dialog_state
                                        .show(repo_id, &repo.name);
                                    self.state.input_mode = InputMode::RenamingProject;
                                }
                            }
                        }
                    }
                }
            }
            Action::ArchiveOrRemove => {
                if self.state.input_mode == InputMode::SidebarNavigation {
                    let selected = self.state.sidebar_state.tree_state.selected;
                    if let Some(node) = self.state.sidebar_data.get_at(selected) {
                        use crate::ui::components::NodeType;
                        match node.node_type {
                            NodeType::Workspace => {
                                self.initiate_archive_workspace(node.id);
                            }
                            NodeType::Repository => {
                                self.initiate_remove_project(node.id);
                            }
                            _ => {}
                        }
                    }
                }
            }
            Action::ArchiveCurrentWorkspace => {
                if matches!(
                    self.state.input_mode,
                    InputMode::Normal | InputMode::Scrolling
                ) {
                    if let Some(workspace_id) = self
                        .state
                        .tab_manager
                        .active_session()
                        .and_then(|s| s.workspace_id)
                    {
                        self.initiate_archive_workspace(workspace_id);
                    }
                }
            }
            _ => {}
        }
    }
}
