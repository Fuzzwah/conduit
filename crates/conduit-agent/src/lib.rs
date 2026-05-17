pub mod claude;
pub mod codex;
pub mod copilot;
pub mod deepseek_tui;
pub mod dirac;
pub mod display;
pub mod error;
pub mod events;
pub mod gemini;
pub mod history;
pub mod mock;
pub mod models;
pub mod opencode;
pub mod orchestration;
pub mod pi;
pub mod runner;
pub mod session;
pub mod stream;
pub mod title_generator;

pub use claude::ClaudeCodeRunner;
pub use codex::CodexCliRunner;
pub use copilot::CopilotRunner;
pub use deepseek_tui::DeepseekTuiRunner;
pub use dirac::DiracRunner;
pub use display::MessageDisplay;
pub use error::AgentError;
pub use events::*;
pub use gemini::GeminiCliRunner;
pub use history::{
    load_claude_history_with_debug, load_codex_history_with_debug,
    load_opencode_history_for_dir_with_debug, load_opencode_history_with_debug,
    load_pi_history_with_debug, HistoryDebugEntry, HistoryError,
};
pub use mock::{MockAgentRunner, MockConfig, MockEventBuilder, MockStartError};
pub use models::{ModelInfo, ModelRegistry};
pub use opencode::OpencodeRunner;
pub use pi::PiRunner;
pub use runner::{
    AgentHandle, AgentInput, AgentMode, AgentRunner, AgentStartConfig, AgentType, ReasoningEffort,
};
pub use session::{SessionId, SessionMetadata, SessionStatus};
pub use title_generator::{generate_title_and_branch, sanitize_branch_suffix, GeneratedMetadata};
