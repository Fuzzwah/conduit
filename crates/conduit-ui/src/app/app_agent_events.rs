use std::path::PathBuf;

use uuid::Uuid;

use crate::app::App;
use crate::app_prompt;
use crate::components::{
    EventDirection, InlinePromptState, InlinePromptType, MessageRole, ProcessingState, PromptAnswer,
};
use crate::effect::Effect;
use crate::events::AppEvent;
use conduit_agent::{
    AgentEvent, AgentInput, AgentStartConfig, AgentType, MessageDisplay, ModelRegistry,
};
use conduit_core::services::ContextWindowService;
use conduit_git::PrStatus;
use conduit_resolver::{CommandResolver, ResolveResult, ResolvedPrompt};

use super::{AskUserQuestionWrapper, ExitPlanModeWrapper};

impl App {
    pub(super) async fn handle_agent_event(
        &mut self,
        session_id: uuid::Uuid,
        event: AgentEvent,
    ) -> anyhow::Result<()> {
        let Some(tab_index) = self.state.tab_manager.session_index_by_id(session_id) else {
            tracing::debug!(
                %session_id,
                "Agent event for unknown session; ignoring"
            );
            return Ok(());
        };

        // Route auto-approval control responses back to the agent's stdin.
        // Handled here rather than in the JSONL parser task so that the parser
        // never holds a clone of the stdin sender (which would keep stdin open
        // even after the user-facing input channel is dropped).
        if let AgentEvent::AutoControlResponse { payload } = event {
            if let Some(session) = self.state.tab_manager.session_mut(tab_index) {
                if let Some(ref input_tx) = session.agent_input_tx {
                    let input_tx = input_tx.clone();
                    tokio::spawn(async move {
                        if let Err(err) = input_tx
                            .send(conduit_agent::AgentInput::ClaudeJsonl(payload))
                            .await
                        {
                            tracing::debug!(
                                "AutoControlResponse dropped (agent shutting down): {}",
                                err
                            );
                        }
                    });
                }
            }
            return Ok(());
        }

        // Check if this is a non-active tab receiving content - mark as needing attention
        let is_active_tab = self.state.tab_manager.active_index() == tab_index;
        let is_content_event = matches!(
            &event,
            AgentEvent::AssistantMessage(_)
                | AgentEvent::AssistantReasoning(_)
                | AgentEvent::ToolStarted(_)
                | AgentEvent::ToolCompleted(_)
                | AgentEvent::CommandOutput(_)
                | AgentEvent::TurnCompleted(_)
                | AgentEvent::TurnFailed(_)
        );

        // Track whether we need to stop footer spinner (done after session borrow ends)
        let mut should_stop_footer_spinner = false;
        let mut should_start_footer_spinner = false;
        let mut pending_sidebar_pr_update: Option<(Uuid, PrStatus)> = None;
        let mut pending_model_invalidation = false;
        let mut should_drain_queue = false;
        let mut pending_observed_context_window: Option<(AgentType, String, i64)> = None;
        let repo_dao = self.repo_dao_clone();
        let workspace_dao = self.workspace_dao_clone();

        {
            let Some(session) = self.state.tab_manager.session_mut(tab_index) else {
                return Ok(());
            };

            // Mark non-active tabs as needing attention when content arrives
            // Exclude suppressed assistant messages (like fork seed ACKs)
            let is_suppressed_assistant = matches!(&event, AgentEvent::AssistantMessage(_))
                && session.suppress_next_assistant_reply;
            if !is_active_tab && is_content_event && !is_suppressed_assistant {
                session.needs_attention = true;
            }

            // Record raw event for debug view
            let (event_type, raw_json) = match &event {
                AgentEvent::Raw { data } => {
                    let event_type = data
                        .get("type")
                        .and_then(|value| value.as_str())
                        .unwrap_or("Raw");
                    (event_type.to_string(), data.clone())
                }
                _ => match serde_json::to_value(&event) {
                    Ok(raw_json) => (event.event_type_name().to_string(), raw_json),
                    Err(error) => {
                        let event_type = event.event_type_name();
                        tracing::warn!(
                            event_type,
                            error = %error,
                            "Failed to serialize agent event for raw events view"
                        );
                        let fallback = serde_json::json!({
                            "type": event_type,
                            "serialize_failed": true,
                            "error": error.to_string(),
                        });
                        (event_type.to_string(), fallback)
                    }
                },
            };
            session.record_raw_event(EventDirection::Received, event_type, raw_json);

            match event {
                AgentEvent::SessionInit(init) => {
                    session.agent_session_id = Some(init.session_id);
                    // Clear pending message - agent has confirmed receipt
                    session.pending_user_message = None;
                    session.update_status();
                }
                AgentEvent::TurnStarted => {
                    session.is_processing = true;
                    session.update_status();
                }
                AgentEvent::TurnCompleted(completed) => {
                    session.add_usage(completed.usage);
                    session.stop_processing();
                    if session.inline_prompt.is_none()
                        && !session.capabilities.supports_interactive_input
                    {
                        session.agent_input_tx = None;
                    }
                    if session.inline_prompt.is_none() && !session.queued_messages.is_empty() {
                        should_drain_queue = true;
                    }
                    // Safety net: avoid suppressing a future real assistant message
                    // (in case the final assistant message event never arrived)
                    session.suppress_next_assistant_reply = false;
                    // Only stop footer spinner if this is the active tab
                    if is_active_tab {
                        should_stop_footer_spinner = true;
                    }
                    session.chat_view.finalize_streaming();
                    // Add turn summary to chat
                    if session.suppress_next_turn_summary {
                        session.suppress_next_turn_summary = false;
                    } else {
                        if session.pending_turn_summary.is_some() {
                            Self::flush_pending_agent_output(session);
                        }
                        let summary = session.current_turn_summary.clone();
                        session.pending_turn_summary = Some(summary);
                        if session.chat_view.streaming_buffer().is_none() {
                            Self::flush_pending_agent_output(session);
                        }
                    }
                }
                AgentEvent::TurnFailed(failed) => {
                    session.stop_processing();
                    session.chat_view.finalize_streaming();
                    session.tools_in_flight = 0;
                    session.set_processing_state(ProcessingState::Thinking);
                    if !session.capabilities.supports_interactive_input {
                        session.agent_input_tx = None;
                    }
                    // Only stop footer spinner if this is the active tab
                    if is_active_tab {
                        should_stop_footer_spinner = true;
                    }
                    session.suppress_next_assistant_reply = false;
                    session.suppress_next_turn_summary = false;
                    let display = MessageDisplay::Error {
                        content: failed.error,
                    };
                    session.chat_view.push(display.to_chat_message());
                }
                AgentEvent::AssistantReasoning(reasoning) => {
                    let token_estimate = (reasoning.text.len() / 4).max(1);
                    session.add_streaming_tokens(token_estimate);
                    session
                        .chat_view
                        .stream_append_role(MessageRole::Reasoning, &reasoning.text);
                }
                AgentEvent::AssistantMessage(msg) => {
                    if session.suppress_next_assistant_reply {
                        if msg.is_final {
                            session.suppress_next_assistant_reply = false;
                        }
                        // Skip rendering the fork seed acknowledgement
                        return Ok(());
                    }
                    // Track streaming tokens (rough estimate: ~4 chars per token)
                    let token_estimate = (msg.text.len() / 4).max(1);
                    session.add_streaming_tokens(token_estimate);

                    // Check for PR URL in the message and capture PR number
                    if session.pr_number.is_none() {
                        if let Some(pr_num) = Self::extract_pr_number_from_text(&msg.text) {
                            pending_sidebar_pr_update =
                                Self::apply_pr_number_to_session(session, pr_num);
                        }
                    }

                    session.chat_view.stream_append(&msg.text);
                    if msg.is_final {
                        Self::flush_pending_agent_output(session);
                    }
                }
                AgentEvent::ToolStarted(tool) => {
                    // Check for special interactive tools that use inline prompts
                    let is_inline_prompt_tool = if tool.tool_name == "AskUserQuestion" {
                        // Parse the questions from the tool arguments
                        match serde_json::from_value::<AskUserQuestionWrapper>(
                            tool.arguments.clone(),
                        ) {
                            Ok(wrapper) => {
                                session.inline_prompt = Some(InlinePromptState::new_ask_user(
                                    tool.tool_id.clone(),
                                    wrapper.questions,
                                ));
                                // Scroll to bottom so prompt is visible
                                session.chat_view.scroll_to_bottom();
                                // Don't push to chat - the inline prompt will be rendered as extra lines
                                session.tools_in_flight = session.tools_in_flight.saturating_add(1);
                                // Stop footer spinner since we're now awaiting user response
                                should_stop_footer_spinner = true;
                                true
                            }
                            Err(e) => {
                                tracing::warn!(
                                    tool_id = %tool.tool_id,
                                    tool_name = %tool.tool_name,
                                    arguments = %serde_json::to_string(&tool.arguments).unwrap_or_default(),
                                    error = %e,
                                    "Failed to deserialize AskUserQuestion arguments"
                                );
                                // Surface error to user so they know why prompt didn't appear
                                let display = MessageDisplay::Error {
                                    content: format!("Failed to parse AskUserQuestion: {}", e),
                                };
                                session.chat_view.push(display.to_chat_message());
                                false
                            }
                        }
                    } else if tool.tool_name == "ExitPlanMode" {
                        // Use plan content from tool arguments when available
                        let (plan_content, plan_path) =
                            match serde_json::from_value::<ExitPlanModeWrapper>(
                                tool.arguments.clone(),
                            ) {
                                Ok(wrapper) => {
                                    let plan_path = Self::read_plan_file_path_for_session(session)
                                        .unwrap_or_else(|| ".claude/plans/plan.md".to_string());
                                    (wrapper.plan, plan_path)
                                }
                                Err(e) => {
                                    // Fall back to reading plan from file
                                    tracing::debug!(
                                        tool_id = %tool.tool_id,
                                        error = %e,
                                        "ExitPlanMode arguments missing plan, falling back to file"
                                    );
                                    Self::read_plan_file_for_session(session)
                                }
                            };

                        session.inline_prompt = Some(InlinePromptState::new_exit_plan(
                            tool.tool_id.clone(),
                            plan_content,
                            plan_path,
                        ));
                        // Scroll to bottom so prompt is visible
                        session.chat_view.scroll_to_bottom();
                        // Don't push to chat - the inline prompt will be rendered as extra lines
                        session.tools_in_flight = session.tools_in_flight.saturating_add(1);
                        // Stop footer spinner since we're now awaiting user response
                        should_stop_footer_spinner = true;
                        true
                    } else {
                        false
                    };

                    // Detect orchestration sub-agent delegation
                    if tool.tool_name == "Agent" && session.orchestration_enabled {
                        let subagent = tool.arguments.get("subagent_type").and_then(|v| v.as_str());
                        let delegation = match subagent {
                            Some("conduit-explore") => {
                                let model = fallback_subagent_model(
                                    session.agent_type,
                                    None,
                                    "claude-haiku-4-5",
                                );
                                Some(("Explore", model))
                            }
                            Some("conduit-review") => {
                                let model = fallback_subagent_model(
                                    session.agent_type,
                                    None,
                                    "claude-haiku-4-5",
                                );
                                Some(("Review", model))
                            }
                            Some("conduit-adversarial-review") => {
                                let model = fallback_subagent_model(
                                    session.agent_type,
                                    session.adversarial_review_model.as_deref(),
                                    "claude-sonnet-4-6",
                                );
                                Some(("Adversarial Review", model))
                            }
                            _ => None,
                        };
                        if let Some((label, model)) = delegation {
                            session.delegated_agent = Some(crate::session::DelegatedAgent {
                                tool_id: tool.tool_id.clone(),
                                display_label: label.to_string(),
                                model: model.to_string(),
                            });
                            session.update_status();
                        }
                    }

                    // Skip normal tool processing for inline prompt tools
                    if !is_inline_prompt_tool {
                        // Update processing state to show tool name
                        session
                            .set_processing_state(ProcessingState::ToolUse(tool.tool_name.clone()));
                        // ToolStarted pairs with ToolCompleted for non-shell tools or CommandOutput
                        // for shell tools; these events are mutually exclusive in agent runners.
                        session.tools_in_flight = session.tools_in_flight.saturating_add(1);

                        let args_str = if tool.arguments.is_null() {
                            String::new()
                        } else {
                            // Compact single-line for display
                            serde_json::to_string(&tool.arguments).unwrap_or_default()
                        };
                        let display = MessageDisplay::Tool {
                            name: MessageDisplay::tool_display_name_owned(&tool.tool_name),
                            args: args_str,
                            output: "Running...".to_string(),
                            exit_code: None,
                            file_size: None, // Only set for Read tool on images via update_last_tool
                        };
                        session.chat_view.push(display.to_chat_message());
                    }
                }
                AgentEvent::ControlRequest(request) => {
                    let mcp_server_disabled = if session.agent_type == AgentType::Claude
                        && Self::is_claude_mcp_tool_name(&request.tool_name)
                    {
                        if let Some(server_name) = Self::extract_mcp_server_name(&request.tool_name)
                        {
                            let repo = session.repository_id.and_then(|repo_id| {
                                repo_dao
                                    .as_ref()
                                    .and_then(|dao| dao.get_by_id(repo_id).ok().flatten())
                            });
                            let workspace = session.workspace_id.and_then(|ws_id| {
                                workspace_dao
                                    .as_ref()
                                    .and_then(|dao| dao.get_by_id(ws_id).ok().flatten())
                            });
                            repo.as_ref().is_some_and(|r| {
                                Self::resolve_disabled_servers(r, workspace.as_ref())
                                    .iter()
                                    .any(|s| s == server_name)
                            })
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if mcp_server_disabled {
                        let server_name =
                            Self::extract_mcp_server_name(&request.tool_name).unwrap_or("unknown");
                        let response_payload = Self::build_permission_deny_response(
                            format!("MCP server '{server_name}' is disabled for this workspace."),
                            request.tool_use_id.as_deref(),
                        );
                        if let Some(ref input_tx) = session.agent_input_tx {
                            if let Ok(jsonl) = Self::build_control_response_jsonl(
                                &request.request_id,
                                response_payload,
                            ) {
                                let input_tx = input_tx.clone();
                                tokio::spawn(async move {
                                    if let Err(err) =
                                        input_tx.send(AgentInput::ClaudeJsonl(jsonl)).await
                                    {
                                        tracing::warn!(
                                            "Failed to send automatic Claude MCP deny response: {}",
                                            err
                                        );
                                    }
                                });
                                session.start_processing();
                                session.set_processing_state(ProcessingState::Thinking);
                                if is_active_tab {
                                    should_start_footer_spinner = true;
                                }
                            }
                        }
                    } else if let Some(tool_use_id) = request.tool_use_id.clone() {
                        session
                            .pending_tool_permissions
                            .insert(tool_use_id.clone(), request.request_id.clone());

                        if let Some(response_payload) = session
                            .pending_tool_permission_responses
                            .remove(&tool_use_id)
                        {
                            if let Ok(jsonl) = Self::build_control_response_jsonl(
                                &request.request_id,
                                response_payload,
                            ) {
                                if let Some(ref input_tx) = session.agent_input_tx {
                                    let input_tx = input_tx.clone();
                                    tokio::spawn(async move {
                                        if let Err(err) =
                                            input_tx.send(AgentInput::ClaudeJsonl(jsonl)).await
                                        {
                                            tracing::warn!(
                                                "Failed to send deferred control response: {}",
                                                err
                                            );
                                        }
                                    });
                                    session.start_processing();
                                    session.set_processing_state(ProcessingState::Thinking);
                                    if is_active_tab {
                                        should_start_footer_spinner = true;
                                    }
                                }
                            }
                            session.pending_tool_permissions.remove(&tool_use_id);
                        }
                    } else {
                        tracing::warn!(
                            tool_name = request.tool_name,
                            "Control request missing tool_use_id"
                        );
                    }
                }
                AgentEvent::ToolCompleted(tool) => {
                    tracing::info!(
                        "ToolCompleted event: tool_id={}, success={}, result_len={}",
                        tool.tool_id,
                        tool.success,
                        tool.result.as_ref().map(|r| r.len()).unwrap_or(0)
                    );

                    // Clear sub-agent delegation if this tool result matches
                    if session
                        .delegated_agent
                        .as_ref()
                        .map(|d| d.tool_id == tool.tool_id)
                        .unwrap_or(false)
                    {
                        session.delegated_agent = None;
                        session.update_status();
                    }

                    // Return to thinking state
                    session.set_processing_state(ProcessingState::Thinking);
                    session.tools_in_flight = match session.tools_in_flight.checked_sub(1) {
                        Some(value) => value,
                        None => {
                            tracing::warn!("tools_in_flight underflow on ToolCompleted");
                            0
                        }
                    };

                    // Track file changes for write/edit tools
                    if tool.success {
                        let tool_name_lower = tool.tool_id.to_lowercase();
                        if tool_name_lower.contains("edit")
                            || tool_name_lower.contains("write")
                            || tool_name_lower.contains("multiedit")
                        {
                            // Try to extract filename from result or use generic name
                            if let Some(ref result) = tool.result {
                                // Simple heuristic: look for file paths in result
                                if let Some(filename) = Self::extract_filename(result) {
                                    // Rough estimate of changes (can be refined)
                                    session.record_file_change(filename, 5, 2);
                                }
                            }
                        }
                    }

                    let output = if tool.success {
                        tool.result.unwrap_or_else(|| "Completed".to_string())
                    } else {
                        format!("Error: {}", tool.error.unwrap_or_default())
                    };
                    // Update the existing "Running..." message instead of pushing a new one
                    if !session.chat_view.update_last_tool(output, None) {
                        tracing::warn!("ToolCompleted: no matching tool message found to update");
                    }
                }
                AgentEvent::CommandOutput(cmd) => {
                    // Check for PR URL in command output (e.g., from gh pr create)
                    if session.pr_number.is_none() {
                        if let Some(pr_num) = Self::extract_pr_number_from_text(&cmd.output) {
                            pending_sidebar_pr_update =
                                Self::apply_pr_number_to_session(session, pr_num);
                        }
                    }

                    // Update the existing "Running..." message instead of pushing a new one
                    if !session
                        .chat_view
                        .update_last_tool(cmd.output.clone(), cmd.exit_code)
                    {
                        tracing::warn!("CommandOutput: no matching tool message found to update");
                    }
                    if !cmd.is_streaming {
                        session.tools_in_flight = match session.tools_in_flight.checked_sub(1) {
                            Some(value) => value,
                            None => {
                                tracing::warn!(
                                    "tools_in_flight underflow on CommandOutput (non-streaming)"
                                );
                                0
                            }
                        };
                    }
                }
                AgentEvent::Error(err) => {
                    let display = MessageDisplay::Error {
                        content: err.message,
                    };
                    session.chat_view.push(display.to_chat_message());
                    if err.code.as_deref() == Some("model_not_found") {
                        session.model = None;
                        session.model_invalid = true;
                        session.update_status();
                        pending_model_invalidation = true;
                    }
                    if err.is_fatal {
                        session.stop_processing();
                        session.chat_view.finalize_streaming();
                        session.tools_in_flight = 0;
                        session.set_processing_state(ProcessingState::Thinking);
                        session.agent_input_tx = None;
                        // Only stop footer spinner if this is the active tab
                        if is_active_tab {
                            should_stop_footer_spinner = true;
                        }
                    }
                }
                AgentEvent::TokenUsage(usage_event) => {
                    session.update_context_usage(&usage_event);
                    if let Some(context_window) = usage_event.context_window {
                        if context_window > 0 {
                            let model_id = session.model.clone().unwrap_or_else(|| {
                                ModelRegistry::default_model(session.agent_type)
                            });
                            pending_observed_context_window =
                                Some((session.agent_type, model_id, context_window));
                        }
                    }

                    // Check if we need to show a warning notification
                    if let Some(warning) = session.pending_context_warning.take() {
                        use conduit_agent::events::ContextWarningLevel;
                        let display = match warning.level {
                            ContextWarningLevel::Critical => MessageDisplay::Error {
                                content: warning.message,
                            },
                            ContextWarningLevel::High | ContextWarningLevel::Medium => {
                                MessageDisplay::System {
                                    content: format!("⚠️ {}", warning.message),
                                }
                            }
                            ContextWarningLevel::Normal => MessageDisplay::System {
                                content: format!("ℹ️ {}", warning.message),
                            },
                        };
                        session.chat_view.push(display.to_chat_message());
                    }
                }
                AgentEvent::ContextCompaction(compaction_event) => {
                    use conduit_agent::events::ContextWindowState;
                    session.handle_compaction(compaction_event.clone());

                    // Always show compaction notification in chat
                    let display = MessageDisplay::System {
                        content: format!(
                            "🔄 Context compacted: {} → {} tokens (reason: {})",
                            ContextWindowState::format_tokens(compaction_event.tokens_before),
                            ContextWindowState::format_tokens(compaction_event.tokens_after),
                            compaction_event.reason
                        ),
                    };
                    session.chat_view.push(display.to_chat_message());

                    // Clear any pending warning since we just compacted
                    session.pending_context_warning = None;
                }
                _ => {}
            }
        } // End session borrow scope

        if let Some((workspace_id, status)) = pending_sidebar_pr_update {
            self.state
                .sidebar_data
                .update_workspace_pr_status(workspace_id, Some(status));
        }
        if let Some((agent_type, model_id, context_window)) = pending_observed_context_window {
            ContextWindowService::record_observed(
                &self.core,
                agent_type,
                &model_id,
                context_window,
            );
        }
        if pending_model_invalidation {
            if let Some(session_tab_dao) = self.session_tab_dao_clone() {
                if let Ok(Some(mut tab)) = session_tab_dao.get_by_id(session_id) {
                    tab.model = None;
                    tab.model_invalid = true;
                    if let Err(err) = session_tab_dao.update(&tab) {
                        tracing::warn!(
                            error = %err,
                            session_id = %session_id,
                            "Failed to persist model invalidation"
                        );
                    }
                } else {
                    tracing::warn!(
                        session_id = %session_id,
                        "Failed to load session for model invalidation"
                    );
                }
            }
        }

        if should_drain_queue {
            match self.drain_queue_for_tab(tab_index) {
                Ok(effects) if !effects.is_empty() => {
                    self.run_effects(effects).await?;
                }
                Ok(_) => {}
                Err(err) => {
                    tracing::warn!(error = %err, "Failed to drain queued messages");
                }
            }
        }

        // Stop footer spinner after session borrow is released
        if should_stop_footer_spinner {
            self.state.stop_footer_spinner();
        }
        if should_start_footer_spinner {
            self.state.start_footer_spinner(None);
        }

        Ok(())
    }

    pub(super) fn submit_prompt(
        &mut self,
        prompt: String,
        images: Vec<PathBuf>,
        image_placeholders: Vec<String>,
    ) -> anyhow::Result<Vec<Effect>> {
        let tab_index = self.state.tab_manager.active_index();
        self.submit_prompt_for_tab(tab_index, prompt, images, image_placeholders, false, None)
    }

    pub(super) fn submit_prompt_hidden(
        &mut self,
        prompt: String,
        images: Vec<PathBuf>,
        image_placeholders: Vec<String>,
    ) -> anyhow::Result<Vec<Effect>> {
        let tab_index = self.state.tab_manager.active_index();
        self.submit_prompt_for_tab(tab_index, prompt, images, image_placeholders, true, None)
    }

    pub(super) fn submit_prompt_hidden_jsonl(
        &mut self,
        payload: String,
    ) -> anyhow::Result<Vec<Effect>> {
        let tab_index = self.state.tab_manager.active_index();
        self.submit_prompt_for_tab(
            tab_index,
            String::new(),
            Vec::new(),
            Vec::new(),
            true,
            Some(payload),
        )
    }

    /// Send a tool result back to the agent by resuming the session with a hidden prompt.
    ///
    /// Claude Code CLI in headless mode accepts structured stdin input, so we resume the
    /// session with a tool_result payload over stream-json.
    ///
    /// For AskUserQuestion: The result contains the user's answers
    /// For ExitPlanMode: The result indicates approval or feedback
    pub(super) fn send_tool_result(
        &mut self,
        tool_id: &str,
        content: String,
        tool_use_result: Option<serde_json::Value>,
    ) -> Vec<Effect> {
        let payload = Self::build_tool_result_jsonl(tool_id, &content, tool_use_result);
        match payload {
            Ok(jsonl) => {
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    if session.agent_type == AgentType::Claude {
                        if let Some(ref input_tx) = session.agent_input_tx {
                            let input_tx = input_tx.clone();
                            let jsonl_to_send = jsonl.clone();
                            tokio::spawn(async move {
                                if let Err(err) =
                                    input_tx.send(AgentInput::ClaudeJsonl(jsonl_to_send)).await
                                {
                                    tracing::warn!(
                                        "Failed to send tool result via streaming input: {}",
                                        err
                                    );
                                }
                            });
                            let pending_tools = session.tools_in_flight;
                            session.start_processing();
                            session.tools_in_flight = pending_tools.saturating_sub(1);
                            session.set_processing_state(ProcessingState::Thinking);
                            self.state.start_footer_spinner(None);
                            return Vec::new();
                        }
                    }
                }

                match self.submit_prompt_hidden_jsonl(jsonl) {
                    Ok(effects) => effects,
                    Err(e) => {
                        tracing::error!("Failed to send tool result: {}", e);
                        Vec::new()
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to build tool result payload: {}", e);
                Vec::new()
            }
        }
    }

    pub(super) fn send_opencode_question_response(
        &mut self,
        request_id: &str,
        answers: Option<Vec<Vec<String>>>,
    ) -> Vec<Effect> {
        let (input_tx, session_id, should_start_footer_spinner, should_stop_footer_spinner, abort) = {
            let Some(session) = self.state.tab_manager.active_session_mut() else {
                return Vec::new();
            };
            if session.agent_type != AgentType::Opencode {
                return Vec::new();
            }
            let pending_tools = session.tools_in_flight;
            session.start_processing();
            session.tools_in_flight = pending_tools.saturating_add(1);
            session.set_processing_state(ProcessingState::Thinking);
            let mut should_start_footer_spinner = true;
            let mut should_stop_footer_spinner = false;
            let mut abort = false;

            if session.agent_input_tx.is_none() {
                session.chat_view.push(
                    MessageDisplay::Error {
                        content: "OpenCode question response failed: session not ready."
                            .to_string(),
                    }
                    .to_chat_message(),
                );
                session.tools_in_flight = session.tools_in_flight.saturating_sub(1);
                session.stop_processing();
                session.set_processing_state(ProcessingState::Thinking);
                should_start_footer_spinner = false;
                should_stop_footer_spinner = true;
                abort = true;
            }

            (
                session.agent_input_tx.clone(),
                Some(session.id),
                should_start_footer_spinner,
                should_stop_footer_spinner,
                abort,
            )
        };

        if should_start_footer_spinner {
            self.state.start_footer_spinner(None);
        }
        if should_stop_footer_spinner {
            self.state.stop_footer_spinner();
        }
        if abort {
            return Vec::new();
        }

        let Some(input_tx) = input_tx else {
            return Vec::new();
        };
        let session_id = match session_id {
            Some(session_id) => session_id,
            None => return Vec::new(),
        };

        let request_id = request_id.to_string();
        let event_tx = self.event_tx.clone();
        tokio::spawn(async move {
            let result = input_tx
                .send(AgentInput::OpencodeQuestion {
                    request_id,
                    answers,
                })
                .await
                .map_err(|err| err.to_string());
            let _ = super::send_app_event(
                &event_tx,
                AppEvent::OpencodeQuestionResponseCompleted { session_id, result },
                "opencode_question_response",
            );
        });
        Vec::new()
    }

    pub(super) fn send_control_response(
        &mut self,
        request_id: &str,
        response_payload: serde_json::Value,
    ) -> Vec<Effect> {
        let payload = Self::build_control_response_jsonl(request_id, response_payload);
        match payload {
            Ok(jsonl) => {
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    if session.agent_type == AgentType::Claude {
                        if let Some(ref input_tx) = session.agent_input_tx {
                            let input_tx = input_tx.clone();
                            let jsonl_to_send = jsonl.clone();
                            tokio::spawn(async move {
                                if let Err(err) =
                                    input_tx.send(AgentInput::ClaudeJsonl(jsonl_to_send)).await
                                {
                                    tracing::warn!(
                                        "Failed to send control response via streaming input: {}",
                                        err
                                    );
                                }
                            });
                            // Preserve tools_in_flight count, then decrement after starting processing
                            // (mirrors send_tool_result behavior for consistency)
                            let pending_tools = session.tools_in_flight;
                            session.start_processing();
                            session.tools_in_flight = pending_tools.saturating_sub(1);
                            session.set_processing_state(ProcessingState::Thinking);
                            self.state.start_footer_spinner(None);
                            return Vec::new();
                        }
                    }
                }

                tracing::warn!("Unable to send control response: missing Claude input channel");
                // Surface error to user and clean up state
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    session.stop_processing();
                    let display = MessageDisplay::Error {
                        content: "Cannot reply to prompt: missing streaming input channel. Try restarting the session.".to_string(),
                    };
                    session.chat_view.push(display.to_chat_message());
                }
                self.state.stop_footer_spinner();
                Vec::new()
            }
            Err(e) => {
                tracing::error!("Failed to build control response payload: {}", e);
                // Surface error to user
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    session.stop_processing();
                    let display = MessageDisplay::Error {
                        content: format!("Failed to send response: {}", e),
                    };
                    session.chat_view.push(display.to_chat_message());
                }
                self.state.stop_footer_spinner();
                Vec::new()
            }
        }
    }

    pub(super) fn build_tool_result_jsonl(
        tool_id: &str,
        content: &str,
        tool_use_result: Option<serde_json::Value>,
    ) -> anyhow::Result<String> {
        let mut payload = serde_json::json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": tool_id,
                    "content": content,
                    "is_error": false,
                }]
            }
        });

        if let Some(value) = tool_use_result {
            if let serde_json::Value::Object(obj) = &mut payload {
                obj.insert("toolUseResult".to_string(), value);
            }
        }

        let json = serde_json::to_string(&payload)?;
        Ok(format!("{json}\n"))
    }

    pub(super) fn build_control_response_jsonl(
        request_id: &str,
        response_payload: serde_json::Value,
    ) -> anyhow::Result<String> {
        let payload = serde_json::json!({
            "type": "control_response",
            "response": {
                "subtype": "success",
                "request_id": request_id,
                "response": response_payload,
            }
        });
        let json = serde_json::to_string(&payload)?;
        Ok(format!("{json}\n"))
    }

    /// Encode an image file to base64 and determine its media type
    pub(super) fn encode_image_to_base64(path: &PathBuf) -> anyhow::Result<(String, String)> {
        use anyhow::anyhow;
        use base64::{engine::general_purpose::STANDARD, Engine as _};

        let data = std::fs::read(path)
            .map_err(|e| anyhow!("Failed to read image file {}: {}", path.display(), e))?;

        let media_type = match path.extension().and_then(|e| e.to_str()) {
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("webp") => "image/webp",
            _ => "image/png", // Default to PNG for unknown extensions
        };

        let base64_data = STANDARD.encode(&data);
        Ok((base64_data, media_type.to_string()))
    }

    pub(super) fn build_user_prompt_jsonl(
        prompt: &str,
        images: &[PathBuf],
    ) -> anyhow::Result<String> {
        let mut content_blocks: Vec<serde_json::Value> = Vec::new();

        tracing::info!(
            "build_user_prompt_jsonl: building JSONL with {} images, prompt_len={}",
            images.len(),
            prompt.len()
        );

        // Add image content blocks first (Claude works best with images before text)
        for (i, path) in images.iter().enumerate() {
            tracing::info!(
                "build_user_prompt_jsonl: processing image {} at {:?}",
                i,
                path
            );
            match Self::encode_image_to_base64(path) {
                Ok((base64_data, media_type)) => {
                    tracing::info!(
                        "build_user_prompt_jsonl: encoded image {} successfully, media_type={}, base64_len={}",
                        i,
                        media_type,
                        base64_data.len()
                    );
                    // Add image label if multiple images
                    if images.len() > 1 {
                        content_blocks.push(serde_json::json!({
                            "type": "text",
                            "text": format!("Image {}:", i + 1),
                        }));
                    }
                    content_blocks.push(serde_json::json!({
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": media_type,
                            "data": base64_data,
                        }
                    }));
                }
                Err(e) => {
                    tracing::warn!("Failed to encode image {}: {}", path.display(), e);
                    // Fall back to mentioning the file path
                    content_blocks.push(serde_json::json!({
                        "type": "text",
                        "text": format!("[Failed to load image: {}]", path.display()),
                    }));
                }
            }
        }

        // Add text content block
        if !prompt.is_empty() {
            content_blocks.push(serde_json::json!({
                "type": "text",
                "text": prompt,
            }));
        }

        let payload = serde_json::json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": content_blocks,
            }
        });
        let json = serde_json::to_string(&payload)?;
        tracing::info!(
            "build_user_prompt_jsonl: final JSONL payload size={} bytes, content_blocks={}",
            json.len(),
            content_blocks.len()
        );
        Ok(format!("{json}\n"))
    }

    pub(super) fn build_permission_allow_response(
        updated_input: serde_json::Value,
        tool_use_id: Option<&str>,
    ) -> serde_json::Value {
        let mut response = serde_json::Map::new();
        response.insert(
            "behavior".to_string(),
            serde_json::Value::String("allow".to_string()),
        );
        response.insert("updatedInput".to_string(), updated_input);
        if let Some(tool_use_id) = tool_use_id {
            response.insert(
                "toolUseID".to_string(),
                serde_json::Value::String(tool_use_id.to_string()),
            );
        }
        serde_json::Value::Object(response)
    }

    pub(super) fn build_permission_deny_response(
        message: String,
        tool_use_id: Option<&str>,
    ) -> serde_json::Value {
        let mut response = serde_json::Map::new();
        response.insert(
            "behavior".to_string(),
            serde_json::Value::String("deny".to_string()),
        );
        response.insert("message".to_string(), serde_json::Value::String(message));
        if let Some(tool_use_id) = tool_use_id {
            response.insert(
                "toolUseID".to_string(),
                serde_json::Value::String(tool_use_id.to_string()),
            );
        }
        serde_json::Value::Object(response)
    }

    pub(super) fn build_ask_user_updated_input(
        prompt: &InlinePromptState,
        answers: &std::collections::HashMap<String, PromptAnswer>,
    ) -> serde_json::Value {
        let questions = match &prompt.prompt_type {
            InlinePromptType::AskUserQuestion { questions } => questions.clone(),
            _ => Vec::new(),
        };

        let mut answers_map = serde_json::Map::new();
        for (question, answer) in answers {
            let formatted = Self::format_prompt_answer(answer);
            answers_map.insert(question.clone(), serde_json::Value::String(formatted));
        }

        serde_json::json!({
            "questions": questions,
            "answers": serde_json::Value::Object(answers_map),
        })
    }

    pub(super) fn build_exit_plan_updated_input(prompt: &InlinePromptState) -> serde_json::Value {
        match &prompt.prompt_type {
            InlinePromptType::ExitPlanMode { plan_content, .. } => {
                serde_json::json!({ "plan": plan_content })
            }
            _ => serde_json::Value::Null,
        }
    }

    pub(super) fn build_ask_user_tool_result(
        prompt: &InlinePromptState,
        answers: &std::collections::HashMap<String, PromptAnswer>,
    ) -> (String, Option<serde_json::Value>) {
        let mut parts = Vec::new();
        for (question, answer) in answers {
            let formatted = Self::format_prompt_answer(answer);
            parts.push(format!("\"{}\"=\"{}\"", question, formatted));
        }

        let content = if parts.is_empty() {
            "User has answered your questions. You can now continue with the user's answers in mind."
                .to_string()
        } else {
            format!(
                "User has answered your questions: {}. You can now continue with the user's answers in mind.",
                parts.join(", ")
            )
        };

        let tool_use_result = match &prompt.prompt_type {
            InlinePromptType::AskUserQuestion { questions } => {
                let mut answers_map = serde_json::Map::new();
                for (question, answer) in answers {
                    let formatted = Self::format_prompt_answer(answer);
                    answers_map.insert(question.clone(), serde_json::Value::String(formatted));
                }
                Some(serde_json::json!({
                    "questions": questions,
                    "answers": serde_json::Value::Object(answers_map),
                }))
            }
            _ => None,
        };

        (content, tool_use_result)
    }

    pub(super) fn build_opencode_question_answers(
        prompt: &InlinePromptState,
        answers: &std::collections::HashMap<String, PromptAnswer>,
    ) -> Vec<Vec<String>> {
        let questions = match &prompt.prompt_type {
            InlinePromptType::AskUserQuestion { questions } => questions,
            _ => return Vec::new(),
        };

        questions
            .iter()
            .map(|question| {
                answers
                    .get(&question.question)
                    .map(|answer| match answer {
                        PromptAnswer::Single(text) => vec![text.clone()],
                        PromptAnswer::Multiple(items) => items.clone(),
                    })
                    .unwrap_or_default()
            })
            .collect()
    }

    pub(super) fn build_exit_plan_tool_result(
        prompt: &InlinePromptState,
        approved: bool,
        feedback: Option<String>,
    ) -> (String, Option<serde_json::Value>) {
        let (plan_content, plan_file_path) = match &prompt.prompt_type {
            InlinePromptType::ExitPlanMode {
                plan_content,
                plan_file_path,
            } => (plan_content.clone(), plan_file_path.clone()),
            _ => (String::new(), ".claude/plans/plan.md".to_string()),
        };

        let tool_use_result = Some(serde_json::json!({
            "plan": plan_content.clone(),
            "isAgent": false,
            "filePath": plan_file_path.clone(),
        }));

        let content = if approved {
            format!(
                "User has approved your plan. You can now start coding. Start with updating your todo list if applicable\n\nYour plan has been saved to: {}\nYou can refer back to it if needed during implementation.\n\n## Approved Plan:\n{}",
                plan_file_path,
                plan_content
            )
        } else if let Some(feedback) = feedback {
            format!("User feedback on plan: {}", feedback)
        } else {
            "User feedback on plan.".to_string()
        };

        (content, tool_use_result)
    }

    pub(super) fn format_prompt_answer(answer: &PromptAnswer) -> String {
        match answer {
            PromptAnswer::Single(text) => text.clone(),
            PromptAnswer::Multiple(items) => items.join(", "),
        }
    }

    pub(super) fn submit_prompt_for_tab(
        &mut self,
        tab_index: usize,
        prompt: String,
        images: Vec<PathBuf>,
        image_placeholders: Vec<String>,
        hidden: bool,
        stdin_payload: Option<String>,
    ) -> anyhow::Result<Vec<Effect>> {
        let mut effects = Vec::new();

        if let Some(session) = self.state.tab_manager.session_mut(tab_index) {
            Self::flush_pending_agent_output(session);
        }

        // Extract session info in a limited borrow scope
        // NOTE: We don't take() resume_session_id here because early returns below
        // (e.g., working_dir validation) would consume it incorrectly. We only
        // consume resume_session_id later when we're committed to spawning the agent.
        // Get default working dir before the mutable borrow
        let default_working_dir = self.config().working_dir.clone();

        let (
            agent_type,
            agent_mode,
            model,
            reasoning_effort,
            model_invalid,
            orchestration_enabled,
            adversarial_review_enabled,
            adversarial_review_model,
            session_id_to_use,
            working_dir,
            is_new_session_for_title,
            session_id,
        ) = {
            let Some(session) = self.state.tab_manager.session_mut(tab_index) else {
                return Ok(effects);
            };

            // "New session" for auto-title purposes == no visible user message has ever been shown.
            // This intentionally ignores hidden prompts (e.g., fork seeds), which don't push a
            // chat user message and shouldn't suppress auto-title on the first real user message.
            let has_visible_user_message = session
                .chat_view
                .messages()
                .iter()
                .any(|m| m.role == MessageRole::User);

            let agent_type = session.agent_type;
            let agent_mode = session.agent_mode;
            let model = session.model.clone();
            let reasoning_effort = session.reasoning_effort;
            let orchestration_enabled = session.orchestration_enabled;
            let adversarial_review_enabled = session.adversarial_review_enabled;
            let adversarial_review_model = session.adversarial_review_model.clone();
            let model_invalid = session.model_invalid;
            // Use agent_session_id if available (set by agent after first prompt)
            // Fall back to resume_session_id (clone, don't take - we consume it later)
            let session_id_to_use = if agent_type == AgentType::Codex
                && session.agent_input_tx.is_none()
                && session.agent_session_id.is_none()
            {
                None
            } else {
                session
                    .agent_session_id
                    .clone()
                    .or_else(|| session.resume_session_id.clone())
            };
            // Use session's working_dir if set, otherwise fall back to config
            let working_dir = session.working_dir.clone().unwrap_or(default_working_dir);
            let session_id = session.id;

            tracing::debug!(
                session_id = %session_id,
                agent = %agent_type,
                has_input_tx = session.agent_input_tx.is_some(),
                agent_session_id = session.agent_session_id.as_ref().map(|id| id.as_str()),
                resume_session_id = session.resume_session_id.as_ref().map(|id| id.as_str()),
                selected_session_id = session_id_to_use.as_ref().map(|id| id.as_str()),
                "submit_prompt_for_tab resolved session state"
            );

            (
                agent_type,
                agent_mode,
                model,
                reasoning_effort,
                model_invalid,
                orchestration_enabled,
                adversarial_review_enabled,
                adversarial_review_model,
                session_id_to_use,
                working_dir,
                !has_visible_user_message,
                session_id,
            )
        };

        let resolved_input = CommandResolver::resolve(&prompt, &working_dir, agent_type);
        match &resolved_input {
            ResolveResult::ConduitCommand { command, .. } => {
                return self.execute_resolved_conduit_command(tab_index, *command);
            }
            ResolveResult::ListRequest { trigger } => {
                self.open_resolver_menu(*trigger);
                return Ok(effects);
            }
            _ => {}
        }

        let display_prompt = prompt;
        let mut agent_prompt = display_prompt.clone();
        let mut codex_skill = None;
        let mut stdin_payload = stdin_payload;
        let use_inline_plan_prompt = Self::plan_prompt_inline_enabled();

        if let ResolveResult::ProviderPrompt(ResolvedPrompt {
            agent_text,
            codex_skill: resolved_skill,
            ..
        }) = resolved_input
        {
            agent_prompt = agent_text;
            codex_skill = resolved_skill;
        }

        // Validate working directory exists before showing user message
        if !working_dir.exists() {
            if let Some(session) = self.state.tab_manager.session_mut(tab_index) {
                let display = MessageDisplay::Error {
                    content: format!(
                        "Working directory does not exist: {}",
                        working_dir.display()
                    ),
                };
                session.chat_view.push(display.to_chat_message());
            }
            return Ok(effects);
        }

        if model_invalid || model.is_none() {
            if let Some(session) = self.state.tab_manager.session_mut(tab_index) {
                session.model_invalid = true;
                let display = MessageDisplay::Error {
                    content: "Select a model to continue.".to_string(),
                };
                session.chat_view.push(display.to_chat_message());
            }
            return Ok(effects);
        }

        // Capture original user message for title generation BEFORE agent-specific transformations
        // (e.g., Codex placeholder stripping, Claude image-path appends)
        let prompt_for_title = display_prompt.clone();
        let working_dir_for_title = working_dir.clone();

        // Add user message to chat and start processing (after validation passes)
        // For hidden prompts (like fork seeds), skip showing in chat and pending_user_message
        if let Some(session) = self.state.tab_manager.session_mut(tab_index) {
            if !hidden {
                let display = MessageDisplay::User {
                    content: display_prompt.clone(),
                };
                session.chat_view.push(display.to_chat_message());
                // Store pending message for persistence (cleared on agent confirmation)
                session.pending_user_message = Some(display_prompt.clone());
            }
            session.start_processing();
        }
        if self.state.tab_manager.active_index() == tab_index {
            self.state.start_footer_spinner(None);
        }

        // Start agent
        if matches!(
            agent_type,
            AgentType::Gemini
                | AgentType::DeepseekTui
                | AgentType::Opencode
                | AgentType::Copilot
                | AgentType::Pi
                | AgentType::Maki
        ) && !images.is_empty()
        {
            if let Some(session) = self.state.tab_manager.session_mut(tab_index) {
                session.stop_processing();
                session.pending_user_message = None;
                let display = MessageDisplay::Error {
                    content: match agent_type {
                        AgentType::Gemini => {
                            "Image attachments aren't supported for Gemini in Conduit yet."
                                .to_string()
                        }
                        AgentType::DeepseekTui => {
                            "Image attachments aren't supported for DeepSeek TUI in Conduit yet."
                                .to_string()
                        }
                        AgentType::Opencode => {
                            "Image attachments aren't supported for OpenCode in Conduit yet."
                                .to_string()
                        }
                        AgentType::Copilot => {
                            "Image attachments aren't supported for GitHub Copilot in Conduit yet."
                                .to_string()
                        }
                        AgentType::Pi => {
                            "Image attachments aren't supported for Pi in Conduit yet.".to_string()
                        }
                        AgentType::Maki => {
                            "Image attachments aren't supported for Maki in Conduit yet."
                                .to_string()
                        }
                        _ => "Image attachments aren't supported for this agent.".to_string(),
                    },
                };
                session.chat_view.push(display.to_chat_message());
            }
            if self.state.tab_manager.active_index() == tab_index {
                self.state.stop_footer_spinner();
            }
            return Ok(effects);
        }

        // Strip placeholders for agents that send images out-of-band.
        if matches!(
            agent_type,
            AgentType::Codex
                | AgentType::Claude
                | AgentType::Gemini
                | AgentType::DeepseekTui
                | AgentType::Opencode
                | AgentType::Copilot
                | AgentType::Pi
                | AgentType::Maki
        ) {
            agent_prompt = Self::strip_image_placeholders(agent_prompt, &image_placeholders);
        }

        if agent_prompt.trim().is_empty() && images.is_empty() && stdin_payload.is_none() {
            if let Some(session) = self.state.tab_manager.session_mut(tab_index) {
                session.stop_processing();
                let display = MessageDisplay::Error {
                    content: "Cannot submit: prompt is empty after processing".to_string(),
                };
                session.chat_view.push(display.to_chat_message());
            }
            if self.state.tab_manager.active_index() == tab_index {
                self.state.stop_footer_spinner();
            }
            return Ok(effects);
        }

        if !hidden {
            let mode_prompt = self
                .state
                .tab_manager
                .session_mut(tab_index)
                .and_then(|session| Self::take_mode_prompt(session, use_inline_plan_prompt));
            if let Some(mode_prompt) = mode_prompt {
                agent_prompt = Self::prepend_mode_prompt(&mode_prompt, &agent_prompt);
            }
        }

        // Record user input for debug view (post-processing)
        // For hidden prompts (like fork seeds), redact content to avoid storing ~500KB
        let mut debug_payload = serde_json::json!({
            "agent_type": agent_type.as_str(),
            "hidden": hidden,
        });
        if hidden {
            debug_payload["prompt_len"] = serde_json::json!(agent_prompt.len());
            debug_payload["prompt_hash"] =
                serde_json::json!(app_prompt::compute_seed_prompt_hash(&agent_prompt));
            if let Some(ref payload) = stdin_payload {
                debug_payload["stdin_payload_len"] = serde_json::json!(payload.len());
                debug_payload["stdin_payload_hash"] =
                    serde_json::json!(app_prompt::compute_seed_prompt_hash(payload));
            }
        } else {
            debug_payload["prompt"] = serde_json::json!(&agent_prompt);
        }
        if !images.is_empty() {
            let image_paths: Vec<String> = images
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect();
            debug_payload["images"] = serde_json::json!(image_paths);
        }
        if let Some(session) = self.state.tab_manager.session_mut(tab_index) {
            session.record_raw_event(EventDirection::Sent, "UserPrompt", debug_payload);
        }

        let mut use_stream_json = false;
        if agent_type == AgentType::Claude {
            use_stream_json = true;
            if stdin_payload.is_none() {
                tracing::info!(
                    "submit_prompt_for_tab: building JSONL for Claude with {} images",
                    images.len()
                );
                stdin_payload = Some(Self::build_user_prompt_jsonl(&agent_prompt, &images)?);
            }
        }

        if agent_type == AgentType::Claude {
            let is_active_tab = self.state.tab_manager.active_index() == tab_index;
            if let Some(session) = self.state.tab_manager.session_mut(tab_index) {
                if let Some(ref input_tx) = session.agent_input_tx {
                    if let Some(payload) = stdin_payload.clone() {
                        let input_tx = input_tx.clone();
                        tokio::spawn(async move {
                            if let Err(err) = input_tx.send(AgentInput::ClaudeJsonl(payload)).await
                            {
                                tracing::warn!("Failed to send streaming prompt: {}", err);
                            }
                        });

                        session.start_processing();
                        session.set_processing_state(ProcessingState::Thinking);
                        if is_active_tab {
                            self.state.start_footer_spinner(None);
                        }
                        return Ok(Vec::new());
                    }
                }
            }
        }

        if matches!(
            agent_type,
            AgentType::Codex | AgentType::Opencode | AgentType::Pi
        ) {
            let is_active_tab = self.state.tab_manager.active_index() == tab_index;
            if let Some(session) = self.state.tab_manager.session_mut(tab_index) {
                if let Some(ref input_tx) = session.agent_input_tx {
                    let input_tx = input_tx.clone();
                    let prompt_to_send = agent_prompt.clone();
                    let images_to_send = images.clone();
                    tokio::spawn(async move {
                        let input = AgentInput::CodexPrompt {
                            text: prompt_to_send,
                            images: images_to_send,
                            model: model.clone(),
                            skill: codex_skill.clone(),
                        };
                        if let Err(err) = input_tx.send(input).await {
                            tracing::warn!("Failed to send prompt: {}", err);
                        }
                    });

                    session.start_processing();
                    session.set_processing_state(ProcessingState::Thinking);
                    if is_active_tab {
                        self.state.start_footer_spinner(None);
                    }
                    return Ok(Vec::new());
                }
            }
        }

        let prompt_for_agent = if agent_type == AgentType::Claude {
            String::new()
        } else {
            agent_prompt.clone()
        };

        let disabled_codex_mcp_servers: Vec<String> = if agent_type == AgentType::Codex {
            let repo = self
                .state
                .tab_manager
                .session(tab_index)
                .and_then(|s| s.repository_id)
                .and_then(|repo_id| {
                    self.repo_dao()
                        .and_then(|dao| dao.get_by_id(repo_id).ok().flatten())
                });
            let workspace = self
                .state
                .tab_manager
                .session(tab_index)
                .and_then(|s| s.workspace_id)
                .and_then(|ws_id| {
                    self.workspace_dao()
                        .and_then(|dao| dao.get_by_id(ws_id).ok().flatten())
                });
            if let Some(ref repo) = repo {
                let effective_disabled = Self::resolve_disabled_servers(repo, workspace.as_ref());
                Self::detect_codex_project_mcp_servers(&working_dir)
                    .into_iter()
                    .filter_map(|(name, _)| {
                        if effective_disabled.contains(&name) {
                            Some(name)
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let mut config = AgentStartConfig::new(prompt_for_agent, working_dir)
            .with_tools(self.config().claude_allowed_tools.clone())
            .with_images(images)
            .with_agent_mode(agent_mode);

        for server_name in disabled_codex_mcp_servers {
            config = config.with_session_config_override(
                format!("mcp_servers.{server_name}.enabled"),
                serde_json::json!(false),
            );
        }

        if let Some(skill) = codex_skill {
            config = config.with_skill(skill);
        }

        // Add model if specified
        if let Some(model_id) = model {
            config = config.with_model(model_id);
        }
        if let Some(effort) = reasoning_effort {
            config = config.with_reasoning_effort(effort);
        }
        if orchestration_enabled {
            config = config.with_orchestration(true);
            if adversarial_review_enabled {
                let model = adversarial_review_model.unwrap_or_else(|| {
                    if agent_type == AgentType::Claude {
                        "claude-sonnet-4-6".to_string()
                    } else {
                        "gemini-2.5-flash".to_string()
                    }
                });
                config = config.with_adversarial_review(
                    conduit_agent::orchestration::AdversarialReviewConfig {
                        enabled: true,
                        model,
                    },
                );
            }
        }

        // Structured stdin payload (used for tool results / stream-json input)
        if let Some(payload) = stdin_payload {
            config = config
                .with_input_format("stream-json")
                .with_stdin_payload(payload);
        } else if use_stream_json {
            config = config.with_input_format("stream-json");
        }

        // Add session ID to continue existing conversation
        if let Some(session_id) = session_id_to_use {
            config = config.with_resume(session_id);
        }

        // Now that we're committed to spawning the agent, consume the resume_session_id
        // to prevent it from being used again on subsequent submits
        if let Some(session) = self.state.tab_manager.session_mut(tab_index) {
            session.resume_session_id.take();
        }

        effects.push(Effect::StartAgent {
            session_id,
            agent_type,
            config: Box::new(config),
        });

        // Generate title on first user message of a NEW session (no title yet, not already pending)
        // Skip for hidden prompts (e.g., fork seeds) - those are not "first user messages"
        // Use is_new_session_for_title (based on session ID presence) instead of turn_count
        // because restored sessions have turn_count == 0 but loaded history
        let should_generate_title = !hidden
            && is_new_session_for_title
            && self
                .state
                .tab_manager
                .session(tab_index)
                .is_some_and(|s| s.title.is_none() && !s.title_generation_pending);

        if should_generate_title {
            if let Some(session) = self.state.tab_manager.session_mut(tab_index) {
                let session_id = session.id;
                let workspace_id = session.workspace_id;

                // Get current branch from status_bar (most accurate source from git tracker)
                let current_branch = session
                    .status_bar
                    .branch_name()
                    .unwrap_or_default()
                    .to_string();

                // Mark as pending to prevent duplicate calls
                session.title_generation_pending = true;

                effects.push(Effect::GenerateTitleAndBranch {
                    session_id,
                    user_message: prompt_for_title.clone(),
                    working_dir: working_dir_for_title.clone(),
                    workspace_id,
                    current_branch,
                });
            }
        }

        Ok(effects)
    }
}

// ============================================================================
// Sub-agent model helpers
// ============================================================================

/// Determine the display model for a sub-agent delegation badge.
/// Uses the explicitly configured model when available, otherwise falls back
/// to a provider-appropriate default.
fn fallback_subagent_model(
    agent_type: AgentType,
    configured: Option<&str>,
    claude_default: &str,
) -> String {
    if let Some(model) = configured {
        if !model.is_empty() {
            return model.to_string();
        }
    }
    match agent_type {
        AgentType::Claude => claude_default.to_string(),
        AgentType::Pi => "gemini-2.5-flash".to_string(),
        _ => claude_default.to_string(),
    }
}
