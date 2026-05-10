use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeEvent {
    SessionStarted { title: String },
    AssistantMessageDelta { chunk: String },
    ReasoningDelta { chunk: String },
    ToolStarted {
        tool_id: u64,
        tool_name: String,
        summary: String,
    },
    ToolCompleted {
        tool_id: u64,
        tool_name: String,
        summary: String,
        output: String,
    },
    TokenUsage {
        prompt_tokens: u32,
        completion_tokens: u32,
    },
    ContextWarning {
        used: u32,
        total: u32,
        message: String,
    },
    Error { message: String },
    SessionEnded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEnvelope {
    pub session_id: u64,
    pub event: RuntimeEvent,
}

pub trait SessionTransport {
    fn submit_prompt(&mut self, session_id: u64, prompt: &str, now_tick: u64);
    fn interrupt(&mut self, session_id: u64, now_tick: u64);
    fn drain_ready(&mut self, now_tick: u64) -> Vec<RuntimeEnvelope>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScheduledEvent {
    due_tick: u64,
    envelope: RuntimeEnvelope,
}

#[derive(Debug, Default)]
pub struct MockTransport {
    scheduled: VecDeque<ScheduledEvent>,
    next_tool_id: u64,
}

impl MockTransport {
    pub fn new() -> Self {
        Self {
            scheduled: VecDeque::new(),
            next_tool_id: 1,
        }
    }

    fn push(&mut self, due_tick: u64, session_id: u64, event: RuntimeEvent) {
        self.scheduled.push_back(ScheduledEvent {
            due_tick,
            envelope: RuntimeEnvelope { session_id, event },
        });
    }
}

impl SessionTransport for MockTransport {
    fn submit_prompt(&mut self, session_id: u64, prompt: &str, now_tick: u64) {
        let tool_id = self.next_tool_id;
        self.next_tool_id += 1;

        self.push(
            now_tick,
            session_id,
            RuntimeEvent::SessionStarted {
                title: format!("Session #{session_id}"),
            },
        );
        self.push(
            now_tick + 1,
            session_id,
            RuntimeEvent::ReasoningDelta {
                chunk: "Inspecting the prompt and selecting a workflow...".to_string(),
            },
        );
        self.push(
            now_tick + 2,
            session_id,
            RuntimeEvent::ToolStarted {
                tool_id,
                tool_name: "search_code".to_string(),
                summary: "Looking for related TUI primitives".to_string(),
            },
        );
        self.push(
            now_tick + 3,
            session_id,
            RuntimeEvent::ToolCompleted {
                tool_id,
                tool_name: "search_code".to_string(),
                summary: "Found matching app, state, and session patterns".to_string(),
                output: "app.rs, app_state.rs, session.rs, tab_manager.rs".to_string(),
            },
        );
        self.push(
            now_tick + 4,
            session_id,
            RuntimeEvent::AssistantMessageDelta {
                chunk: format!(
                    "Transport-neutral response for `{prompt}`: update the app state, render the shared surfaces, and keep runtime events normalized."
                ),
            },
        );
        self.push(
            now_tick + 5,
            session_id,
            RuntimeEvent::TokenUsage {
                prompt_tokens: 96,
                completion_tokens: 148,
            },
        );
        if prompt.len() > 32 {
            self.push(
                now_tick + 6,
                session_id,
                RuntimeEvent::ContextWarning {
                    used: 18_000,
                    total: 32_000,
                    message: "Large prompt loaded into the clean-room scaffold".to_string(),
                },
            );
        }
        self.push(now_tick + 7, session_id, RuntimeEvent::SessionEnded);
    }

    fn interrupt(&mut self, session_id: u64, now_tick: u64) {
        self.push(
            now_tick,
            session_id,
            RuntimeEvent::Error {
                message: "Mock transport interrupted".to_string(),
            },
        );
        self.push(now_tick + 1, session_id, RuntimeEvent::SessionEnded);
    }

    fn drain_ready(&mut self, now_tick: u64) -> Vec<RuntimeEnvelope> {
        let mut ready = Vec::new();
        while self
            .scheduled
            .front()
            .is_some_and(|event| event.due_tick <= now_tick)
        {
            if let Some(event) = self.scheduled.pop_front() {
                ready.push(event.envelope);
            }
        }
        ready
    }
}

#[cfg(test)]
mod tests {
    use super::{MockTransport, RuntimeEvent, SessionTransport};

    #[test]
    fn mock_transport_emits_normalized_sequence() {
        let mut transport = MockTransport::new();
        transport.submit_prompt(7, "hello world", 10);

        let events = transport.drain_ready(20);
        assert!(matches!(events.first().map(|e| &e.event), Some(RuntimeEvent::SessionStarted { .. })));
        assert!(matches!(events.last().map(|e| &e.event), Some(RuntimeEvent::SessionEnded)));
        assert!(events.iter().any(|event| matches!(event.event, RuntimeEvent::ToolStarted { .. })));
        assert!(events.iter().any(|event| matches!(event.event, RuntimeEvent::ToolCompleted { .. })));
    }
}
