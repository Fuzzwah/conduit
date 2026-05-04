use crate::action::Action;
use crate::app::App;
use crate::components::ConfirmationContext;
use crate::effect::Effect;
use crate::events::InputMode;

impl App {
    pub(super) fn handle_confirmation_action(
        &mut self,
        action: Action,
        effects: &mut Vec<Effect>,
    ) -> anyhow::Result<()> {
        match action {
            Action::ConfirmYes if self.state.input_mode == InputMode::Confirming => {
                if self.is_blocking_confirmation_loading_dialog() {
                    return Ok(());
                }
                if let Some(context) = self.state.confirmation_dialog_state.context.take() {
                    match context {
                        ConfirmationContext::SelectWorkspaceMode { repo_id } => {
                            match self.apply_repo_workspace_mode(
                                repo_id,
                                conduit_git::WorkspaceMode::Worktree,
                            ) {
                                Ok(()) => {
                                    self.state.confirmation_dialog_state.hide();
                                    self.state.input_mode = InputMode::SidebarNavigation;
                                    effects.extend(self.start_workspace_creation(repo_id));
                                }
                                Err(err) => {
                                    self.state.confirmation_dialog_state.hide();
                                    self.show_error("Unable to Set Workspace Mode", &err);
                                }
                            }
                        }
                        ConfirmationContext::RemoveProject(id) => {
                            effects.push(self.execute_remove_project(id));
                            self.state.confirmation_dialog_state.hide();
                            self.state.input_mode = InputMode::SidebarNavigation;
                        }
                        ConfirmationContext::RemoveProjectPreflightInProgress { .. } => {
                            return Ok(());
                        }
                        ConfirmationContext::CreatePullRequest {
                            tab_index,
                            working_dir,
                            preflight,
                        } => {
                            self.state.confirmation_dialog_state.hide();
                            self.state.input_mode = InputMode::Normal;
                            effects.extend(self.submit_pr_workflow(
                                tab_index,
                                working_dir,
                                preflight,
                            )?);
                        }
                        ConfirmationContext::OpenExistingPr { working_dir, .. } => {
                            self.state.confirmation_dialog_state.hide();
                            self.state.input_mode = InputMode::Normal;
                            effects.push(Effect::OpenPrInBrowser { working_dir });
                        }
                        ConfirmationContext::SteerFallback { message_id } => {
                            self.state.confirmation_dialog_state.hide();
                            self.state.input_mode = InputMode::Normal;
                            effects.extend(self.confirm_steer_fallback(message_id)?);
                        }
                        ConfirmationContext::ForkSession {
                            parent_workspace_id,
                            base_branch,
                        } => {
                            self.state.confirmation_dialog_state.hide();
                            self.state.input_mode = InputMode::Normal;
                            if let Some(effect) =
                                self.execute_fork_session(parent_workspace_id, base_branch)
                            {
                                effects.push(effect);
                            }
                        }
                        ConfirmationContext::ForkSessionPreflightInProgress { .. } => {
                            return Ok(());
                        }
                        ConfirmationContext::Quit => {
                            self.state.confirmation_dialog_state.hide();
                            self.state.input_mode = InputMode::Normal;
                            self.state.should_quit = true;
                            effects.push(Effect::SaveSessionState);
                        }
                    }
                }
            }
            Action::ConfirmNo if self.state.input_mode == InputMode::Confirming => {
                if self.is_blocking_confirmation_loading_dialog() {
                    return Ok(());
                }
                if let Some(context) = self.state.confirmation_dialog_state.context.take() {
                    match context {
                        ConfirmationContext::SelectWorkspaceMode { repo_id } => {
                            match self.apply_repo_workspace_mode(
                                repo_id,
                                conduit_git::WorkspaceMode::Checkout,
                            ) {
                                Ok(()) => {
                                    self.state.confirmation_dialog_state.hide();
                                    self.state.input_mode = InputMode::SidebarNavigation;
                                    effects.extend(self.start_workspace_creation(repo_id));
                                }
                                Err(err) => {
                                    self.state.confirmation_dialog_state.hide();
                                    self.show_error("Unable to Set Workspace Mode", &err);
                                }
                            }
                        }
                        ConfirmationContext::RemoveProjectPreflightInProgress { .. } => {
                            return Ok(());
                        }
                        ConfirmationContext::ForkSessionPreflightInProgress { .. } => {
                            return Ok(());
                        }
                        _ => {
                            self.state.input_mode = self.dismiss_confirmation_dialog();
                        }
                    }
                } else {
                    self.state.input_mode = self.dismiss_confirmation_dialog();
                }
            }
            Action::ConfirmToggle if self.state.input_mode == InputMode::Confirming => {
                if self.is_blocking_confirmation_loading_dialog() {
                    return Ok(());
                }
                self.state.confirmation_dialog_state.toggle_selection();
            }
            _ => {}
        }

        Ok(())
    }
}
