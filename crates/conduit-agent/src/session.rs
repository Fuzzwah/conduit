use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Session identifier (compatible with both Claude and Codex)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::now_v7().to_string())
    }

    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Session metadata stored for resume capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: SessionId,
    pub agent_type: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_active: chrono::DateTime<chrono::Utc>,
    pub working_dir: std::path::PathBuf,
    pub total_tokens_used: i64,
    pub turn_count: u32,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Active,
    Completed,
    Failed,
    Abandoned,
}

impl SessionMetadata {
    pub fn new(agent_type: impl Into<String>, working_dir: std::path::PathBuf) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: SessionId::new(),
            agent_type: agent_type.into(),
            created_at: now,
            last_active: now,
            working_dir,
            total_tokens_used: 0,
            turn_count: 0,
            status: SessionStatus::Active,
        }
    }

    pub fn update_activity(&mut self) {
        self.last_active = chrono::Utc::now();
    }

    pub fn add_tokens(&mut self, tokens: i64) {
        self.total_tokens_used += tokens;
    }

    pub fn increment_turn(&mut self) {
        self.turn_count += 1;
    }
}
