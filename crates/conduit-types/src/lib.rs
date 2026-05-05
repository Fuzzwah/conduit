//! Conduit shared data types.
//!
//! Houses cross-crate value types (chat messages, actions, input/view modes,
//! prompt builders) so that lower tiers (`agent`, `config`, `web`) do not
//! need to depend on the heavyweight `ui` crate.

pub mod action;
pub mod agent;
pub mod app_prompt;
pub mod chat_message;
pub mod input_mode;
pub mod skill;
pub mod turn_summary;

pub use action::Action;
pub use agent::{AgentMode, AgentType};
pub use chat_message::{ChatMessage, MessageRole};
pub use input_mode::{InputMode, ViewMode};
pub use skill::SkillReference;
pub use turn_summary::{FileChange, TurnSummary};
