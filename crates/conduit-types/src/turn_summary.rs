/// Represents a file change with diff stats
#[derive(Debug, Clone)]
pub struct FileChange {
    pub filename: String,
    pub additions: usize,
    pub deletions: usize,
}

/// Summary of a completed turn
#[derive(Debug, Clone, Default)]
pub struct TurnSummary {
    /// Duration in seconds
    pub duration_secs: u64,
    /// Input tokens used
    pub input_tokens: u64,
    /// Output tokens generated
    pub output_tokens: u64,
    /// Files that were modified
    pub files_changed: Vec<FileChange>,
}

impl TurnSummary {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set duration from seconds
    pub fn with_duration(mut self, secs: u64) -> Self {
        self.duration_secs = secs;
        self
    }

    /// Set token usage
    pub fn with_tokens(mut self, input: u64, output: u64) -> Self {
        self.input_tokens = input;
        self.output_tokens = output;
        self
    }

    /// Add a file change
    pub fn add_file(&mut self, filename: impl Into<String>, additions: usize, deletions: usize) {
        self.files_changed.push(FileChange {
            filename: filename.into(),
            additions,
            deletions,
        });
    }

    /// Format duration as human-readable string
    pub fn format_duration(&self) -> String {
        let secs = self.duration_secs;
        if secs >= 60 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}s", secs)
        }
    }

    /// Format token count (abbreviate if large)
    pub fn format_tokens(count: u64) -> String {
        if count >= 1000 {
            format!("{:.1}k", count as f64 / 1000.0)
        } else {
            count.to_string()
        }
    }
}
