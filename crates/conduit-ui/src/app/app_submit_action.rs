use std::env;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use chrono::Utc;
use crossterm::{
    event::{EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tempfile::Builder;
use uuid::Uuid;

use crate::app::App;
use crate::app_prompt;
use crate::app_state::{ModelPickerContext, PendingForkRequest, PendingHandoffRequest};
use crate::components::ConfirmationContext;
use crate::effect::Effect;
use crate::events::{ForkSessionDialogPreflightResult, InputMode};
use crate::session::AgentSession;
use conduit_agent::{AgentMode, AgentType, MessageDisplay, ModelRegistry};
use conduit_core::{resolve_repo_workspace_settings, services::ContextWindowService};
use conduit_data::{ForkSeed, QueuedImageAttachment, QueuedMessage, QueuedMessageMode};

use super::PLAN_MODE_INLINE_REMINDER_ENV;

impl App {
    pub(super) fn handle_submit_action(
        &mut self,
        mode: QueuedMessageMode,
    ) -> anyhow::Result<Vec<Effect>> {
        let mut effects = Vec::new();
        let mut immediate_submit: Option<(String, Vec<PathBuf>, Vec<String>)> = None;
        let mut interrupt_before_submit = false;
        let mut prompt_fallback_id: Option<Uuid> = None;
        let mut footer_message: Option<String> = None;
        let mut shell_command: Option<(Uuid, usize, String, Option<PathBuf>)> = None;
        let mut shell_error: Option<String> = None;
        let mut queued_handled = false;
        let mut open_queue_editor_after = false;
        let mut rewind_kill_process = false;

        // Extract config values before the mutable borrow
        let steer_behavior = self.config().steer.behavior;
        let steer_fallback = self.config().steer.fallback;

        {
            let Some(session) = self.state.tab_manager.active_session_mut() else {
                return Ok(effects);
            };

            if session.input_box.is_empty() {
                session.chat_view.scroll_to_bottom();
                return Ok(effects);
            }

            let submission = session.input_box.submit();
            if submission.text.trim().is_empty() && submission.image_paths.is_empty() {
                return Ok(effects);
            }

            let submission_text = submission.text;
            let submission_image_paths = submission.image_paths;
            let submission_image_placeholders = submission.image_placeholders;

            let handled_by_shell = session.input_box.is_shell_mode();
            if handled_by_shell {
                let command = submission_text.trim().to_string();
                if command.is_empty() {
                    shell_error = Some("Shell command is empty".to_string());
                } else {
                    let args = serde_json::json!({ "command": command }).to_string();
                    session
                        .chat_view
                        .push(crate::components::ChatMessage::tool_with_exit(
                            "Bash",
                            args,
                            "Running...".to_string(),
                            None,
                        ));
                    let message_index = session.chat_view.len().saturating_sub(1);
                    session.input_box.set_shell_mode(false);
                    session.update_status();
                    shell_command = Some((
                        session.id,
                        message_index,
                        command,
                        session.working_dir.clone(),
                    ));
                }
                queued_handled = true;
            }

            if !queued_handled {
                if let Some(note) = submission_text
                    .trim()
                    .strip_prefix("/btw")
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    let queued = QueuedMessage {
                        id: Uuid::new_v4(),
                        mode: QueuedMessageMode::FollowUp,
                        text: note.to_string(),
                        images: Vec::new(),
                        created_at: Utc::now(),
                    };
                    session.queue_message(queued);
                    footer_message = Some("Note queued".to_string());
                    queued_handled = true;
                } else if submission_text.trim() == "/btw" {
                    open_queue_editor_after = true;
                    queued_handled = true;
                } else if submission_text.trim() == "/status" {
                    let msg = Self::format_session_status(session);
                    session
                        .chat_view
                        .push(crate::components::ChatMessage::system(msg));
                    queued_handled = true;
                } else if submission_text.trim() == "/rewind" {
                    if session.is_processing {
                        footer_message =
                            Some("Cannot rewind while the agent is running".to_string());
                    } else if session.agent_type != AgentType::Claude {
                        footer_message = Some(format!(
                            "/rewind is only supported for Claude sessions (current: {})",
                            session.agent_type
                        ));
                    } else if session.agent_session_id.is_none() {
                        footer_message = Some("No session to rewind".to_string());
                    } else if !session.chat_view.pop_last_turn() {
                        footer_message = Some("Nothing to rewind".to_string());
                    } else {
                        if let (Some(session_id), Some(working_dir)) =
                            (&session.agent_session_id, &session.working_dir)
                        {
                            if let Err(e) = Self::truncate_claude_session(session_id, working_dir) {
                                tracing::warn!(error = %e, "Failed to truncate Claude session file during rewind");
                            }
                        }
                        rewind_kill_process = true;
                        footer_message = Some("Rewound 1 turn".to_string());
                    }
                    queued_handled = true;
                }
            }

            if !queued_handled {
                let effective_mode = if mode == QueuedMessageMode::Steer
                    && steer_behavior == conduit_config::SteerBehavior::Soft
                {
                    QueuedMessageMode::FollowUp
                } else {
                    mode
                };

                if session.is_processing {
                    let images = submission_image_paths
                        .iter()
                        .cloned()
                        .zip(submission_image_placeholders.iter().cloned())
                        .map(|(path, placeholder)| QueuedImageAttachment { path, placeholder })
                        .collect::<Vec<_>>();
                    let queued = QueuedMessage {
                        id: Uuid::new_v4(),
                        mode: effective_mode,
                        text: submission_text.clone(),
                        images,
                        created_at: Utc::now(),
                    };

                    if mode == QueuedMessageMode::Steer
                        && effective_mode == QueuedMessageMode::Steer
                    {
                        match steer_fallback {
                            conduit_config::SteerFallback::Interrupt => {
                                let (text, image_paths, image_placeholders) =
                                    crate::app_queue::queued_to_submission(&queued);
                                immediate_submit = Some((text, image_paths, image_placeholders));
                                interrupt_before_submit = true;
                                queued_handled = true;
                            }
                            conduit_config::SteerFallback::Prompt => {
                                session.queue_message(queued.clone());
                                prompt_fallback_id = Some(queued.id);
                                footer_message = Some(
                                    "Steering queued · press Enter to confirm interrupt"
                                        .to_string(),
                                );
                                queued_handled = true;
                            }
                            conduit_config::SteerFallback::Queue => {
                                session.queue_message(queued);
                                footer_message = Some("Steering queued".to_string());
                                queued_handled = true;
                            }
                        }
                    } else {
                        session.queue_message(queued);
                        footer_message = Some(if mode == QueuedMessageMode::Steer {
                            "Steering queued (soft mode)".to_string()
                        } else {
                            "Message queued".to_string()
                        });
                        queued_handled = true;
                    }
                }

                if !queued_handled {
                    immediate_submit = Some((
                        submission_text,
                        submission_image_paths,
                        submission_image_placeholders,
                    ));
                }
            }
        }

        if let Some(message) = shell_error {
            self.state
                .set_timed_footer_message(message, Duration::from_secs(3));
            return Ok(effects);
        }

        if let Some((session_id, message_index, command, working_dir)) = shell_command {
            effects.push(Effect::RunShellCommand {
                session_id,
                message_index,
                command,
                working_dir,
            });
            return Ok(effects);
        }

        if open_queue_editor_after {
            self.open_queue_editor();
            return Ok(effects);
        }

        if rewind_kill_process {
            self.interrupt_agent();
        }

        if let Some(message) = footer_message {
            self.state
                .set_timed_footer_message(message, Duration::from_secs(3));
        }

        if let Some(message_id) = prompt_fallback_id {
            self.show_steer_fallback_prompt(message_id);
            return Ok(effects);
        }

        if let Some((text, images, placeholders)) = immediate_submit {
            if interrupt_before_submit {
                self.interrupt_agent();
            }
            effects.extend(self.submit_prompt(text, images, placeholders)?);
        }

        Ok(effects)
    }

    pub(super) fn strip_image_placeholders(prompt: String, placeholders: &[String]) -> String {
        if placeholders.is_empty() {
            return prompt;
        }

        let mut cleaned = prompt;
        for placeholder in placeholders {
            cleaned = cleaned.replace(placeholder, "");
        }

        cleaned.trim().to_string()
    }

    pub(super) fn plan_prompt_inline_enabled() -> bool {
        env::var(PLAN_MODE_INLINE_REMINDER_ENV)
            .ok()
            .map(|value| {
                matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false)
    }

    pub(super) fn plan_file_prompt_info(session: &AgentSession) -> (String, bool) {
        if let Some(path) = Self::read_plan_file_path_for_session(session) {
            return (path, true);
        }

        let path = if let Some(ref working_dir) = session.working_dir {
            working_dir.join(".claude").join("plans").join("plan.md")
        } else if let Some(home_dir) = dirs::home_dir() {
            home_dir.join(".claude").join("plans").join("plan.md")
        } else {
            PathBuf::from(".claude").join("plans").join("plan.md")
        };

        (path.display().to_string(), false)
    }

    pub(super) fn take_mode_prompt(
        session: &mut AgentSession,
        use_inline_plan_prompt: bool,
    ) -> Option<String> {
        if !session.capabilities.supports_plan_mode {
            return None;
        }

        match session.agent_mode {
            AgentMode::Plan => {
                if session.last_mode_prompt == Some(AgentMode::Plan) {
                    return None;
                }
                let prompt = if use_inline_plan_prompt {
                    let (plan_path, exists) = Self::plan_file_prompt_info(session);
                    app_prompt::build_plan_mode_prompt_inline(&plan_path, exists)
                } else {
                    app_prompt::plan_mode_prompt_default().to_string()
                };
                session.last_mode_prompt = Some(AgentMode::Plan);
                Some(prompt)
            }
            AgentMode::Build => {
                if session.last_mode_prompt == Some(AgentMode::Plan) {
                    session.last_mode_prompt = Some(AgentMode::Build);
                    Some(app_prompt::build_switch_prompt().to_string())
                } else {
                    None
                }
            }
        }
    }

    pub(super) fn prepend_mode_prompt(mode_prompt: &str, prompt: &str) -> String {
        if prompt.trim().is_empty() {
            mode_prompt.to_string()
        } else {
            format!("{mode_prompt}\n\n{prompt}")
        }
    }

    pub(super) fn resolve_external_editor(&self) -> Option<Vec<String>> {
        let editor = env::var("VISUAL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                env::var("EDITOR")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })?;

        let parts: Vec<String> = editor
            .split_whitespace()
            .map(|part| part.to_string())
            .collect();

        if parts.is_empty() {
            None
        } else {
            Some(parts)
        }
    }

    pub(super) fn reinitialize_terminal(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> anyhow::Result<()> {
        enable_raw_mode()?;
        if !self.demo_mode {
            let mut stdout = io::stdout();
            execute!(
                stdout,
                crossterm::terminal::EnterAlternateScreen,
                EnableMouseCapture,
                EnableBracketedPaste
            )?;
        }
        terminal.clear()?;
        Ok(())
    }

    pub(super) fn edit_prompt_external(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        guard: &mut crate::terminal_guard::TerminalGuard,
    ) -> anyhow::Result<()> {
        if self.state.input_mode != InputMode::Normal {
            self.state.set_timed_footer_message(
                "External editor only works in chat input".to_string(),
                Duration::from_secs(3),
            );
            return Ok(());
        }

        let editor_parts = match self.resolve_external_editor() {
            Some(parts) => parts,
            None => {
                self.state.set_timed_footer_message(
                    "Set $VISUAL or $EDITOR to use external editor".to_string(),
                    Duration::from_secs(3),
                );
                return Ok(());
            }
        };

        let (expanded_input, attachments) = {
            let Some(session) = self.state.tab_manager.active_session_mut() else {
                return Ok(());
            };
            (
                session.input_box.expanded_input(),
                session.input_box.attachments_snapshot(),
            )
        };

        let temp = Builder::new()
            .prefix("conduit-prompt-")
            .suffix(".txt")
            .tempfile()?;
        std::fs::write(temp.path(), expanded_input)?;

        guard.cleanup_for_suspend()?;

        let status = {
            let mut parts = editor_parts.into_iter();
            let command = match parts.next() {
                Some(cmd) => cmd,
                None => {
                    self.reinitialize_terminal(terminal)?;
                    self.state.set_timed_footer_message(
                        "External editor is not configured".to_string(),
                        Duration::from_secs(3),
                    );
                    return Ok(());
                }
            };
            let args: Vec<String> = parts.collect();
            Command::new(command).args(args).arg(temp.path()).status()
        };

        self.reinitialize_terminal(terminal)?;

        let status = status?;

        if !status.success() {
            self.state.set_timed_footer_message(
                "External editor cancelled".to_string(),
                Duration::from_secs(3),
            );
            return Ok(());
        }

        let edited = std::fs::read_to_string(temp.path())?;
        if let Some(session) = self.state.tab_manager.active_session_mut() {
            session
                .input_box
                .set_input_with_attachments(edited, attachments);
            session.input_box.move_end();
        }

        Ok(())
    }

    #[cfg(unix)]
    pub(super) fn suspend_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        guard: &mut crate::terminal_guard::TerminalGuard,
    ) -> anyhow::Result<()> {
        guard.cleanup_for_suspend()?;
        let result = unsafe { libc::raise(libc::SIGTSTP) };
        if result == -1 {
            let err = io::Error::last_os_error();
            self.reinitialize_terminal(terminal)?;
            return Err(anyhow!("SIGTSTP failed: {}", err));
        }
        self.reinitialize_terminal(terminal)?;
        Ok(())
    }

    #[cfg(not(unix))]
    pub(super) fn suspend_app(
        &mut self,
        _terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        _guard: &mut crate::terminal_guard::TerminalGuard,
    ) -> anyhow::Result<()> {
        self.state.set_timed_footer_message(
            "Suspend is not supported on this platform".to_string(),
            Duration::from_secs(3),
        );
        Ok(())
    }

    /// Initiate handoff flow - capture source context and open model selection.
    pub(super) fn initiate_handoff_session(&mut self) {
        let (
            source_agent_type,
            source_agent_mode,
            source_reasoning_effort,
            workspace_id,
            working_dir,
            project_name,
            workspace_name,
            branch_name,
            pr_number,
            handoff_prompt,
        ) = {
            let Some(session) = self.state.tab_manager.active_session() else {
                self.state.set_timed_footer_message(
                    "No active session to hand off".to_string(),
                    Duration::from_secs(3),
                );
                return;
            };

            if session.is_processing {
                self.show_error("Cannot Handoff", "Wait for the current response to finish.");
                return;
            }

            (
                session.agent_type,
                session.agent_mode,
                session.reasoning_effort,
                session.workspace_id,
                session.working_dir.clone(),
                session.project_name.clone(),
                session.workspace_name.clone(),
                session.branch_name.clone(),
                session.pr_number,
                app_prompt::build_handoff_prompt(session.chat_view.messages()),
            )
        };

        self.state.close_overlays();
        self.state.pending_handoff_request = Some(PendingHandoffRequest {
            source_agent_type,
            agent_mode: source_agent_mode,
            reasoning_effort: source_reasoning_effort,
            workspace_id,
            working_dir,
            project_name,
            workspace_name,
            branch_name,
            pr_number,
            handoff_prompt: Arc::from(handoff_prompt),
        });

        let mut allowed = self.config().effective_enabled_providers(self.tools());
        if !allowed.contains(&source_agent_type) {
            let tool = Self::required_tool(source_agent_type);
            if self.tools().is_available(tool) {
                allowed.push(source_agent_type);
            }
        }

        if allowed.is_empty() {
            self.state.pending_handoff_request = None;
            self.state.set_timed_footer_message(
                "No enabled providers available. Use /providers.".to_string(),
                Duration::from_secs(4),
            );
            return;
        }

        let target_provider = self.preferred_provider_for_handoff(source_agent_type);
        let target_model = self.config().default_model_for(target_provider);
        let defaults = self.model_selector_defaults();
        self.state
            .model_selector_state
            .set_allowed_providers(Some(allowed));
        self.state.model_selector_state.show_with_title(
            Some(target_model),
            defaults,
            "Handoff Target Model".to_string(),
        );
        self.state.model_picker_context = ModelPickerContext::HandoffSelection;
        self.state.input_mode = InputMode::SelectingModel;
    }

    /// Execute handoff after selecting target provider.
    pub(super) fn execute_handoff_session(
        &mut self,
        target_agent: AgentType,
        target_model: String,
    ) -> anyhow::Result<Vec<Effect>> {
        let Some(pending) = self.state.pending_handoff_request.clone() else {
            return Err(anyhow!("No pending handoff request."));
        };

        let target_provider = if self
            .config()
            .is_provider_enabled_effective(target_agent, self.tools())
        {
            target_agent
        } else {
            self.preferred_provider_for_new_sessions()
                .unwrap_or(target_agent)
        };
        let target_model = if target_provider == target_agent {
            target_model
        } else {
            self.config().default_model_for(target_provider)
        };

        // Keep track of where we came from so we can recover cleanly on failure.
        let prev_index = self.state.tab_manager.active_index();
        let prev_sidebar_visible = self.state.sidebar_state.visible;
        let prev_sidebar_focused = self.state.sidebar_state.focused;
        let prev_input_mode = self.state.input_mode;
        let prev_tree_selected = self.state.sidebar_state.tree_state.selected;

        let mut session = if let Some(dir) = pending.working_dir.clone() {
            AgentSession::with_working_dir(target_provider, dir)
        } else {
            AgentSession::new(target_provider)
        };
        session.workspace_id = pending.workspace_id;
        session.project_name = pending.project_name.clone();
        session.workspace_name = pending.workspace_name.clone();
        session.branch_name = pending.branch_name.clone();
        session.pr_number = pending.pr_number;
        session.model = Some(target_model);
        session.init_context_for_model();
        session.model_invalid = false;
        session.agent_mode = if session.capabilities.supports_plan_mode {
            pending.agent_mode
        } else {
            AgentMode::Build
        };
        session.reasoning_effort = if Self::reasoning_supported(target_provider) {
            pending.reasoning_effort
        } else {
            None
        };
        session.suppress_next_assistant_reply = true;
        session.suppress_next_turn_summary = true;
        session.update_status();

        let Some(new_index) = self.state.tab_manager.add_session(session) else {
            self.state.pending_handoff_request = None;
            return Err(anyhow!("Maximum number of tabs reached."));
        };

        self.state.tab_manager.switch_to(new_index);
        self.sync_footer_spinner();
        if !self.config().ui.always_show_sidebar {
            self.state.sidebar_state.hide();
        }
        self.state.sidebar_state.set_focused(false);
        self.state.input_mode = InputMode::Normal;

        let rollback = |app: &mut Self| {
            app.close_tab_at_index(new_index);
            let fallback = prev_index.min(app.state.tab_manager.len().saturating_sub(1));
            app.state.tab_manager.switch_to(fallback);
            if prev_sidebar_visible {
                app.state.sidebar_state.show();
            } else {
                app.state.sidebar_state.hide();
            }
            app.state.sidebar_state.set_focused(prev_sidebar_focused);
            app.state.input_mode = prev_input_mode;
            app.state.sidebar_state.tree_state.selected = prev_tree_selected;
            app.sync_footer_spinner();
        };

        let effects =
            match self.submit_prompt_hidden(pending.handoff_prompt.to_string(), vec![], vec![]) {
                Ok(effects) if effects.is_empty() => {
                    rollback(self);
                    self.state.pending_handoff_request = None;
                    return Err(anyhow!(
                        "Failed to start handoff session: no start-agent effect produced."
                    ));
                }
                Ok(effects) => effects,
                Err(err) => {
                    rollback(self);
                    self.state.pending_handoff_request = None;
                    return Err(err);
                }
            };

        if let Some(session) = self.state.tab_manager.session_mut(new_index) {
            let display = MessageDisplay::System {
                content: "Handoff context was injected. Waiting for your next prompt.".to_string(),
            };
            session.chat_view.push(display.to_chat_message());
        }

        self.state.pending_handoff_request = None;

        Ok(effects)
    }

    /// Initiate fork session flow - validate and show confirmation dialog
    pub(super) fn initiate_fork_session(&mut self) {
        let Some(session) = self.state.tab_manager.active_session() else {
            return;
        };

        if session.is_processing {
            self.show_error("Cannot Fork", "Wait for the current response to finish.");
            return;
        }

        let parent_workspace_id = match session.workspace_id {
            Some(id) => id,
            None => {
                self.show_error(
                    "Cannot Fork",
                    "This session is not attached to a workspace.",
                );
                return;
            }
        };

        if self.fork_seed_dao().is_none() {
            self.show_error("Cannot Fork", "Fork metadata store unavailable.");
            return;
        }

        let seed_prompt = app_prompt::build_fork_seed_prompt(session.chat_view.messages());
        let model_id = session
            .model
            .clone()
            .unwrap_or_else(|| ModelRegistry::default_model(session.agent_type));
        let context_window =
            ContextWindowService::resolve(&self.core, session.agent_type, &model_id).tokens;
        let heuristic_tokens = Self::estimate_tokens(&seed_prompt);
        let observed_tokens = session.context_state.current_tokens.max(0);
        let token_estimate = heuristic_tokens.max(observed_tokens);

        self.state.pending_fork_request = Some(PendingForkRequest {
            agent_type: session.agent_type,
            agent_mode: session.agent_mode,
            model: session.model.clone(),
            reasoning_effort: session.reasoning_effort,
            parent_session_id: session
                .agent_session_id
                .as_ref()
                .map(|s| s.as_str().to_string()),
            parent_workspace_id,
            seed_prompt: Arc::from(seed_prompt),
            token_estimate,
            context_window,
            fork_seed_id: None,
        });

        self.show_blocking_confirmation_loading(
            "Fork Session",
            "Analyzing workspace state...",
            ConfirmationContext::ForkSessionPreflightInProgress {
                parent_workspace_id,
            },
        );

        let workspace_dao = self.workspace_dao_clone();
        let worktree_manager = self.worktree_manager().clone();
        self.spawn_blocking_preflight(
            move || {
                let workspace_dao =
                    workspace_dao.ok_or_else(|| "Workspace database unavailable".to_string())?;

                let workspace = workspace_dao
                    .get_by_id(parent_workspace_id)
                    .map_err(|e| format!("Failed to load workspace: {}", e))?
                    .ok_or_else(|| "Workspace not found.".to_string())?;

                let base_branch = worktree_manager
                    .get_current_branch(&workspace.path)
                    .unwrap_or_else(|_| workspace.branch.clone());

                let dirty_warning = match worktree_manager.get_branch_status(&workspace.path) {
                    Ok(status) if status.is_dirty => Some(
                        status
                            .dirty_description
                            .unwrap_or_else(|| "Uncommitted changes detected".to_string()),
                    ),
                    _ => None,
                };

                Ok(ForkSessionDialogPreflightResult {
                    base_branch,
                    dirty_warning,
                })
            },
            move |result| crate::events::AppEvent::ForkSessionDialogPreflightCompleted {
                parent_workspace_id,
                result,
            },
            "fork_session_dialog_preflight_completed",
        );
    }

    /// Execute fork session after confirmation
    pub(super) fn execute_fork_session(
        &mut self,
        parent_workspace_id: uuid::Uuid,
        base_branch: String,
    ) -> Option<Effect> {
        let Some(mut pending) = self.state.pending_fork_request.clone() else {
            self.show_error("Fork Failed", "No pending fork request.");
            return None;
        };

        if pending.parent_workspace_id != parent_workspace_id {
            self.show_error("Fork Failed", "Fork request does not match workspace.");
            self.state.pending_fork_request = None;
            return None;
        }

        let fork_seed_dao = match self.fork_seed_dao() {
            Some(dao) => dao,
            None => {
                self.show_error("Fork Failed", "Fork metadata store unavailable.");
                self.state.pending_fork_request = None;
                return None;
            }
        };

        let seed_prompt_hash = app_prompt::compute_seed_prompt_hash(&pending.seed_prompt);
        let fork_seed = ForkSeed::new(
            pending.agent_type,
            pending.parent_session_id.clone(),
            Some(pending.parent_workspace_id),
            seed_prompt_hash,
            None,
            pending.token_estimate,
            pending.context_window,
        );

        if let Err(e) = fork_seed_dao.create(&fork_seed) {
            self.show_error(
                "Fork Failed",
                &format!("Failed to save fork metadata: {}", e),
            );
            self.state.pending_fork_request = None;
            return None;
        }

        pending.fork_seed_id = Some(fork_seed.id);
        self.state.pending_fork_request = Some(pending);

        self.mark_workspace_busy(parent_workspace_id);
        Some(Effect::ForkWorkspace {
            parent_workspace_id,
            base_branch,
        })
    }

    pub(super) fn finish_fork_session(
        &mut self,
        workspace_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<Effect>> {
        let Some(pending) = self.state.pending_fork_request.clone() else {
            return Err(anyhow!("No pending fork data."));
        };

        let fork_seed_id = match pending.fork_seed_id {
            Some(id) => id,
            None => return Err(anyhow!("Fork metadata was not saved.")),
        };

        let workspace_dao = self
            .workspace_dao()
            .ok_or_else(|| anyhow!("Workspace database unavailable."))?;

        let repo_dao = self
            .repo_dao()
            .ok_or_else(|| anyhow!("Repository database unavailable."))?;

        let workspace = workspace_dao
            .get_by_id(workspace_id)
            .map_err(|e| anyhow!("Failed to load workspace: {}", e))?
            .ok_or_else(|| anyhow!("Workspace not found."))?;

        let project_name = repo_dao
            .get_by_id(workspace.repository_id)
            .ok()
            .flatten()
            .map(|repo| repo.name);

        // Keep track of where we came from so we can recover cleanly on failure
        let prev_index = self.state.tab_manager.active_index();
        let prev_sidebar_visible = self.state.sidebar_state.visible;
        let prev_input_mode = self.state.input_mode;
        let prev_tree_selected = self.state.sidebar_state.tree_state.selected;

        let mut session =
            AgentSession::with_working_dir(pending.agent_type, workspace.path.clone());
        session.workspace_id = Some(workspace_id);
        session.project_name = project_name;
        session.workspace_name = Some(workspace.name.clone());
        session.branch_name = Some(workspace.branch.clone());
        session.model = pending.model.clone();
        session.reasoning_effort = pending.reasoning_effort;
        session.model_invalid = false;
        session.agent_mode = pending.agent_mode;
        session.fork_seed_id = Some(fork_seed_id);
        session.suppress_next_assistant_reply = true;
        session.suppress_next_turn_summary = true;
        session.update_status();

        let new_index = self
            .state
            .tab_manager
            .add_session(session)
            .ok_or_else(|| anyhow!("Maximum number of tabs reached."))?;

        self.state.tab_manager.switch_to(new_index);
        self.sync_footer_spinner();

        if let Some(ref tracker) = self.git_tracker {
            tracker.track_workspace(workspace_id, workspace.path.clone());
        }

        if !self.config().ui.always_show_sidebar {
            self.state.sidebar_state.hide();
        }
        self.state.sidebar_state.set_focused(false);
        self.state.input_mode = InputMode::Normal;

        // Note: suppress flags already set on session before add_session, no need to set again

        // Use submit_prompt_hidden - don't add 500KB seed to chat transcript
        let effects =
            match self.submit_prompt_hidden(pending.seed_prompt.to_string(), vec![], vec![]) {
                Ok(effects) if effects.is_empty() => {
                    // Remove the broken tab and untrack workspace
                    if let Some(ref tracker) = self.git_tracker {
                        tracker.untrack_workspace(workspace_id);
                    }
                    self.close_tab_at_index(new_index);
                    let fallback = prev_index.min(self.state.tab_manager.len().saturating_sub(1));
                    self.state.tab_manager.switch_to(fallback);
                    // Restore pre-fork UI state
                    if prev_sidebar_visible {
                        self.state.sidebar_state.show();
                    }
                    self.state.input_mode = prev_input_mode;
                    self.state.sidebar_state.tree_state.selected = prev_tree_selected;
                    return Err(anyhow!(
                        "Failed to start forked agent: no start-agent effect produced."
                    ));
                }
                Ok(effects) => effects,
                Err(e) => {
                    // Remove the broken tab and untrack workspace
                    if let Some(ref tracker) = self.git_tracker {
                        tracker.untrack_workspace(workspace_id);
                    }
                    self.close_tab_at_index(new_index);
                    let fallback = prev_index.min(self.state.tab_manager.len().saturating_sub(1));
                    self.state.tab_manager.switch_to(fallback);
                    // Restore pre-fork UI state
                    if prev_sidebar_visible {
                        self.state.sidebar_state.show();
                    }
                    self.state.input_mode = prev_input_mode;
                    self.state.sidebar_state.tree_state.selected = prev_tree_selected;
                    return Err(e);
                }
            };

        self.state.pending_fork_request = None;

        Ok(effects)
    }

    /// Attempt to clean up a fork workspace after finish_fork_session fails.
    /// Returns Some(error_message) if cleanup failed or partial cleanup occurred,
    /// None only if all cleanup operations succeeded.
    pub(super) fn cleanup_fork_workspace(
        &mut self,
        workspace_id: uuid::Uuid,
        repo_id: uuid::Uuid,
    ) -> Option<String> {
        // Untrack workspace from git tracker first (must happen even on early returns)
        if let Some(ref tracker) = self.git_tracker {
            tracker.untrack_workspace(workspace_id);
        }

        let workspace_dao = self.workspace_dao()?;
        let repo_dao = self.repo_dao()?;

        // Safety: only allow deletion of paths under the managed workspaces directory
        let managed_root = conduit_util::workspaces_dir();

        // Get workspace and repo info for worktree cleanup
        let workspace = match workspace_dao.get_by_id(workspace_id) {
            Ok(Some(ws)) => ws,
            Ok(None) => return None, // Already gone
            Err(e) => return Some(format!("Failed to load workspace: {}", e)),
        };

        // Check if workspace path is under managed root using canonicalization (security guard)
        // This prevents path traversal attacks like /managed/root/../../../etc
        let path_is_managed = match (
            std::fs::canonicalize(&managed_root),
            std::fs::canonicalize(&workspace.path),
        ) {
            (Ok(canonical_root), Ok(canonical_path)) => canonical_path.starts_with(&canonical_root),
            (Err(e), _) => {
                tracing::warn!(
                    error = %e,
                    managed_root = %managed_root.display(),
                    "Cannot canonicalize managed root; refusing removal for safety"
                );
                false
            }
            (_, Err(e)) => {
                // Path doesn't exist or can't be canonicalized - may already be deleted
                // Log but don't treat as managed (safe default)
                tracing::debug!(
                    error = %e,
                    path = %workspace.path.display(),
                    "Cannot canonicalize workspace path; may already be deleted"
                );
                // Try to prune stale worktree metadata since the path may have been deleted
                if let Ok(Some(repo)) = repo_dao.get_by_id(workspace.repository_id) {
                    if let Some(base_path) = &repo.base_path {
                        let repo_settings = resolve_repo_workspace_settings(self.config(), &repo);
                        if let Err(prune_err) = self
                            .worktree_manager()
                            .prune_workspaces(repo_settings.mode, base_path)
                        {
                            tracing::debug!(
                                error = %prune_err,
                                "Failed to prune stale worktrees"
                            );
                        }
                    }
                }
                false
            }
        };

        let repo = match repo_dao.get_by_id(repo_id) {
            Ok(Some(r)) => r,
            Ok(None) => {
                // Repo not found; try best-effort directory removal then delete from DB
                if path_is_managed {
                    if let Err(e) = std::fs::remove_dir_all(&workspace.path) {
                        tracing::warn!(
                            error = %e,
                            workspace_id = %workspace_id,
                            "Best-effort workspace directory removal failed (repo not found)"
                        );
                    }
                } else {
                    tracing::warn!(
                        workspace_id = %workspace_id,
                        path = %workspace.path.display(),
                        managed_root = %managed_root.display(),
                        "Refusing to remove non-managed workspace path (repo not found)"
                    );
                }
                if let Err(e) = workspace_dao.delete(workspace_id) {
                    return Some(format!("Failed to delete workspace from database: {}", e));
                }
                self.refresh_sidebar_data();
                return None;
            }
            Err(e) => {
                // Repo load failed; try best-effort directory removal then delete from DB
                if path_is_managed {
                    if let Err(fs_err) = std::fs::remove_dir_all(&workspace.path) {
                        tracing::warn!(
                            error = %fs_err,
                            workspace_id = %workspace_id,
                            "Best-effort workspace directory removal failed (repo load error)"
                        );
                    }
                } else {
                    tracing::warn!(
                        workspace_id = %workspace_id,
                        path = %workspace.path.display(),
                        managed_root = %managed_root.display(),
                        "Refusing to remove non-managed workspace path (repo load error)"
                    );
                }
                if let Err(db_err) = workspace_dao.delete(workspace_id) {
                    return Some(format!(
                        "Failed to load repository: {}; also failed to delete workspace from database: {}",
                        e, db_err
                    ));
                }
                self.refresh_sidebar_data();
                return Some(format!(
                    "Failed to load repository: {} (workspace deleted from DB)",
                    e
                ));
            }
        };
        let settings = resolve_repo_workspace_settings(self.config(), &repo);

        // Collect cleanup warnings for resources that may need manual cleanup
        let mut cleanup_warnings: Vec<String> = Vec::new();

        // Try to remove the worktree first (only if path is under managed root)
        if let Some(base_path) = &repo.base_path {
            if !path_is_managed {
                tracing::warn!(
                    workspace_id = %workspace_id,
                    path = %workspace.path.display(),
                    managed_root = %managed_root.display(),
                    "Refusing to remove worktree: workspace path is outside managed directory"
                );
                cleanup_warnings.push(format!(
                    "Worktree at {} may need manual removal (outside managed directory)",
                    workspace.path.display()
                ));
            } else if let Err(e) =
                self.worktree_manager()
                    .remove_workspace(settings.mode, base_path, &workspace.path)
            {
                tracing::warn!(
                    error = %e,
                    workspace_id = %workspace_id,
                    "Failed to remove worktree during fork cleanup"
                );
                cleanup_warnings.push(format!(
                    "Worktree at {} may need manual removal",
                    workspace.path.display()
                ));
            }

            // Also try to delete the branch (only if we successfully managed the worktree path)
            if path_is_managed {
                if let Err(e) = self.worktree_manager().delete_branch(
                    settings.mode,
                    base_path,
                    &workspace.path,
                    &workspace.branch,
                ) {
                    tracing::warn!(
                        error = %e,
                        workspace_id = %workspace_id,
                        branch = %workspace.branch,
                        "Failed to delete branch during fork cleanup"
                    );
                    cleanup_warnings.push(format!(
                        "Branch '{}' may need manual deletion",
                        workspace.branch
                    ));
                }
            } else {
                cleanup_warnings.push(format!(
                    "Branch '{}' not auto-deleted (workspace path outside managed directory)",
                    workspace.branch
                ));
            }
        } else {
            // No base_path available; try best-effort directory removal
            if path_is_managed {
                if let Err(e) = std::fs::remove_dir_all(&workspace.path) {
                    tracing::warn!(
                        error = %e,
                        workspace_id = %workspace_id,
                        "Best-effort workspace directory removal failed (no base_path)"
                    );
                    cleanup_warnings.push(format!(
                        "Workspace at {} may need manual removal",
                        workspace.path.display()
                    ));
                }
            } else {
                tracing::warn!(
                    workspace_id = %workspace_id,
                    path = %workspace.path.display(),
                    managed_root = %managed_root.display(),
                    "Refusing to remove non-managed workspace path (no base_path)"
                );
                cleanup_warnings.push(format!(
                    "Workspace at {} may need manual removal (outside managed directory)",
                    workspace.path.display()
                ));
            }
            // Note: Can't delete branch without base_path
            cleanup_warnings.push(format!(
                "Branch '{}' may need manual deletion (no repo base path)",
                workspace.branch
            ));
        }

        // Delete workspace from database
        if let Err(e) = workspace_dao.delete(workspace_id) {
            return Some(format!("Failed to delete workspace from database: {}", e));
        }

        self.refresh_sidebar_data();

        // Return cleanup warnings if any resources may need manual cleanup
        if cleanup_warnings.is_empty() {
            None
        } else {
            Some(format!("Partial cleanup: {}", cleanup_warnings.join("; ")))
        }
    }
}
