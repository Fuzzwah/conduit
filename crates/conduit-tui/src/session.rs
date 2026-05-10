use crate::domain::ProviderProfile;
use crate::runtime::RuntimeEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMode {
    Build,
    Plan,
}

impl AgentMode {
    pub fn toggle(self) -> Self {
        match self {
            Self::Build => Self::Plan,
            Self::Plan => Self::Build,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Build => "Build",
            Self::Plan => "Plan",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    Reasoning,
    Tool,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRecord {
    pub label: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextWindowState {
    pub used: u32,
    pub total: u32,
    pub message: Option<String>,
}

impl Default for ContextWindowState {
    fn default() -> Self {
        Self {
            used: 0,
            total: 32_000,
            message: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedPrompt {
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposerState {
    pub buffer: String,
    pub shell_mode: bool,
}

impl Default for ComposerState {
    fn default() -> Self {
        Self {
            buffer: String::new(),
            shell_mode: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSession {
    pub id: u64,
    pub title: String,
    pub provider: ProviderProfile,
    pub model: String,
    pub mode: AgentMode,
    pub branch_name: String,
    pub pr_state: String,
    pub processing: bool,
    pub messages: Vec<ChatMessage>,
    pub raw_events: Vec<RuntimeRecord>,
    pub token_usage: TokenUsage,
    pub context: ContextWindowState,
    pub queue: Vec<QueuedPrompt>,
    pub composer: ComposerState,
}

impl AgentSession {
    pub fn demo(id: u64, title: impl Into<String>, provider: ProviderProfile) -> Self {
        let model = provider.default_model.to_string();
        Self {
            id,
            title: title.into(),
            provider,
            model,
            mode: AgentMode::Build,
            branch_name: "fuz/clean-room".to_string(),
            pr_state: "#42 open".to_string(),
            processing: false,
            messages: vec![ChatMessage {
                role: MessageRole::Assistant,
                text: "Welcome to the clean-room Ratatui scaffold.".to_string(),
            }],
            raw_events: vec![RuntimeRecord {
                label: "bootstrap".to_string(),
                detail: "Session created from transport-neutral demo data".to_string(),
            }],
            token_usage: TokenUsage::default(),
            context: ContextWindowState::default(),
            queue: vec![QueuedPrompt {
                label: "Recreate sidebar workflow".to_string(),
            }],
            composer: ComposerState::default(),
        }
    }

    pub fn push_user_prompt(&mut self, prompt: String) {
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            text: prompt,
        });
        self.processing = true;
    }

    pub fn apply_runtime_event(&mut self, event: RuntimeEvent) {
        self.raw_events.push(RuntimeRecord {
            label: runtime_label(&event).to_string(),
            detail: runtime_detail(&event),
        });

        match event {
            RuntimeEvent::SessionStarted { title } => self.title = title,
            RuntimeEvent::AssistantMessageDelta { chunk } => {
                if let Some(last) = self.messages.last_mut() {
                    if last.role == MessageRole::Assistant {
                        if !last.text.is_empty() {
                            last.text.push(' ');
                        }
                        last.text.push_str(&chunk);
                    } else {
                        self.messages.push(ChatMessage {
                            role: MessageRole::Assistant,
                            text: chunk,
                        });
                    }
                } else {
                    self.messages.push(ChatMessage {
                        role: MessageRole::Assistant,
                        text: chunk,
                    });
                }
            }
            RuntimeEvent::ReasoningDelta { chunk } => self.messages.push(ChatMessage {
                role: MessageRole::Reasoning,
                text: chunk,
            }),
            RuntimeEvent::ToolStarted { summary, .. } => self.messages.push(ChatMessage {
                role: MessageRole::Tool,
                text: format!("Started tool: {summary}"),
            }),
            RuntimeEvent::ToolCompleted {
                summary, output, ..
            } => self.messages.push(ChatMessage {
                role: MessageRole::Tool,
                text: format!("{summary}\n{output}"),
            }),
            RuntimeEvent::TokenUsage {
                prompt_tokens,
                completion_tokens,
            } => {
                self.token_usage.prompt_tokens += prompt_tokens;
                self.token_usage.completion_tokens += completion_tokens;
            }
            RuntimeEvent::ContextWarning {
                used,
                total,
                message,
            } => {
                self.context.used = used;
                self.context.total = total;
                self.context.message = Some(message);
            }
            RuntimeEvent::Error { message } => self.messages.push(ChatMessage {
                role: MessageRole::Error,
                text: message,
            }),
            RuntimeEvent::SessionEnded => self.processing = false,
        }
    }
}

fn runtime_label(event: &RuntimeEvent) -> &'static str {
    match event {
        RuntimeEvent::SessionStarted { .. } => "session.started",
        RuntimeEvent::AssistantMessageDelta { .. } => "assistant.delta",
        RuntimeEvent::ReasoningDelta { .. } => "reasoning.delta",
        RuntimeEvent::ToolStarted { .. } => "tool.started",
        RuntimeEvent::ToolCompleted { .. } => "tool.completed",
        RuntimeEvent::TokenUsage { .. } => "token.usage",
        RuntimeEvent::ContextWarning { .. } => "context.warning",
        RuntimeEvent::Error { .. } => "error",
        RuntimeEvent::SessionEnded => "session.ended",
    }
}

fn runtime_detail(event: &RuntimeEvent) -> String {
    match event {
        RuntimeEvent::SessionStarted { title } => title.clone(),
        RuntimeEvent::AssistantMessageDelta { chunk } => chunk.clone(),
        RuntimeEvent::ReasoningDelta { chunk } => chunk.clone(),
        RuntimeEvent::ToolStarted {
            tool_name, summary, ..
        } => format!("{tool_name}: {summary}"),
        RuntimeEvent::ToolCompleted {
            tool_name, output, ..
        } => format!("{tool_name}: {output}"),
        RuntimeEvent::TokenUsage {
            prompt_tokens,
            completion_tokens,
        } => format!("prompt={prompt_tokens}, completion={completion_tokens}"),
        RuntimeEvent::ContextWarning { message, .. } => message.clone(),
        RuntimeEvent::Error { message } => message.clone(),
        RuntimeEvent::SessionEnded => "complete".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::default_provider;
    use crate::runtime::RuntimeEvent;

    use super::{AgentSession, MessageRole};

    #[test]
    fn assistant_deltas_are_coalesced() {
        let mut session = AgentSession::demo(1, "Demo", default_provider());
        session.apply_runtime_event(RuntimeEvent::AssistantMessageDelta {
            chunk: "one".to_string(),
        });
        session.apply_runtime_event(RuntimeEvent::AssistantMessageDelta {
            chunk: "two".to_string(),
        });

        let last = session.messages.last().expect("message exists");
        assert_eq!(last.role, MessageRole::Assistant);
        assert_eq!(last.text, "Welcome to the clean-room Ratatui scaffold. one two");
    }

    #[test]
    fn token_usage_accumulates() {
        let mut session = AgentSession::demo(1, "Demo", default_provider());
        session.apply_runtime_event(RuntimeEvent::TokenUsage {
            prompt_tokens: 10,
            completion_tokens: 20,
        });
        assert_eq!(session.token_usage.prompt_tokens, 10);
        assert_eq!(session.token_usage.completion_tokens, 20);
    }
}
