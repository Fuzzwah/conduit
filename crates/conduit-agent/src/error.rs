use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Failed to spawn agent process")]
    ProcessSpawnFailed,

    #[error("Failed to capture stdout")]
    StdoutCaptureFailed,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Agent binary not found: {0}")]
    BinaryNotFound(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Operation not supported: {0}")]
    NotSupported(String),

    #[error("Agent crashed with exit code: {0}")]
    Crashed(i32),

    #[error("Agent timeout after {0}ms")]
    Timeout(u64),

    #[error("Channel closed unexpectedly")]
    ChannelClosed,

    #[error("Configuration error: {0}")]
    Config(String),
}
