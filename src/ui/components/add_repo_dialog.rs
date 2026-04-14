//! Add repository dialog component

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    symbols::border,
    widgets::{Block, Borders, Paragraph, Widget},
};
use std::path::PathBuf;

use super::{DialogFrame, PathInputState, StatusLine};

/// Whether the current input is a local path or a remote git URL
#[derive(Debug, Clone, PartialEq, Default)]
pub enum RepoInputKind {
    /// A local filesystem path
    #[default]
    LocalPath,
    /// A remote git URL (SSH or HTTPS)
    GitUrl { url: String },
}

/// State for the add repository dialog
#[derive(Debug, Clone)]
pub struct AddRepoDialogState {
    /// Shared path input state (includes visibility and validation)
    pub path: PathInputState,
    /// Extracted repository name
    pub repo_name: Option<String>,
    /// Whether the current input is a local path or a remote git URL
    pub input_kind: RepoInputKind,
}

impl Default for AddRepoDialogState {
    fn default() -> Self {
        Self::new()
    }
}

impl AddRepoDialogState {
    pub fn new() -> Self {
        Self {
            path: PathInputState::new(),
            repo_name: None,
            input_kind: RepoInputKind::LocalPath,
        }
    }

    /// Show the dialog
    pub fn show(&mut self) {
        self.path.show();
        self.path.text.clear();
        self.repo_name = None;
        self.input_kind = RepoInputKind::LocalPath;
    }

    /// Hide the dialog
    pub fn hide(&mut self) {
        self.path.hide();
    }

    /// Get the current input value
    pub fn input(&self) -> &str {
        self.path.input()
    }

    /// Whether the current input is a git URL
    pub fn is_url(&self) -> bool {
        matches!(self.input_kind, RepoInputKind::GitUrl { .. })
    }

    // Delegate text input methods with validation
    pub fn insert_char(&mut self, c: char) {
        self.path.insert_char(c);
        self.validate();
    }

    pub fn delete_char(&mut self) {
        self.path.delete_char();
        self.validate();
    }

    pub fn delete_forward(&mut self) {
        self.path.delete_forward();
        self.validate();
    }

    pub fn move_left(&mut self) {
        self.path.move_left();
    }

    pub fn move_right(&mut self) {
        self.path.move_right();
    }

    pub fn move_start(&mut self) {
        self.path.move_start();
    }

    pub fn move_end(&mut self) {
        self.path.move_end();
    }

    /// Validate the current input — handles both local paths and remote git URLs
    pub fn validate(&mut self) {
        let input = self.path.input().to_string();

        if input.is_empty() {
            self.path.set_invalid();
            self.repo_name = None;
            self.input_kind = RepoInputKind::LocalPath;
            return;
        }

        if let Some(name) = detect_git_url_repo_name(&input) {
            self.repo_name = Some(name);
            self.input_kind = RepoInputKind::GitUrl { url: input };
            self.path.set_valid();
            return;
        }

        // Local path validation
        self.input_kind = RepoInputKind::LocalPath;

        let expanded_path = self.path.expanded_path();

        if !expanded_path.exists() {
            self.path.set_error("Path does not exist");
            self.repo_name = None;
            return;
        }

        if !expanded_path.is_dir() {
            self.path.set_error("Path is not a directory");
            self.repo_name = None;
            return;
        }

        let git_dir = expanded_path.join(".git");
        if !git_dir.exists() {
            self.path
                .set_error("Not a git repository (no .git directory)");
            self.repo_name = None;
            return;
        }

        self.repo_name = expanded_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        self.path.set_valid();
    }

    /// Get the expanded path (only meaningful for local path inputs)
    pub fn expanded_path(&self) -> PathBuf {
        self.path.expanded_path()
    }

    /// Check if dialog is visible
    pub fn is_visible(&self) -> bool {
        self.path.is_visible()
    }

    /// Validation error message
    pub fn error(&self) -> Option<&str> {
        self.path.error.as_deref()
    }

    /// Whether the path/URL is valid
    pub fn is_valid(&self) -> bool {
        self.path.is_valid
    }
}

/// Detect if the input string looks like a git URL and return the repository name.
///
/// Supported formats:
/// - SSH: `git@github.com:user/repo.git` or `git@github.com:user/repo`
/// - HTTPS: `https://github.com/user/repo.git` or `https://github.com/user/repo`
fn detect_git_url_repo_name(input: &str) -> Option<String> {
    // SSH: git@host:path/to/repo[.git]
    if input.starts_with("git@") {
        if let Some(colon_pos) = input.find(':') {
            let path_part = &input[colon_pos + 1..];
            return extract_repo_name_from_path(path_part);
        }
    }

    // HTTPS: https://host/path/to/repo[.git]
    if input.starts_with("https://") || input.starts_with("http://") {
        // Strip scheme
        let after_scheme = if let Some(s) = input.strip_prefix("https://") {
            s
        } else if let Some(s) = input.strip_prefix("http://") {
            s
        } else {
            return None;
        };
        // Find the path after the host
        if let Some(slash_pos) = after_scheme.find('/') {
            let path_part = &after_scheme[slash_pos + 1..];
            if !path_part.is_empty() {
                return extract_repo_name_from_path(path_part);
            }
        }
    }

    None
}

/// Extract the repo name (last segment) from a URL path, stripping `.git` suffix.
fn extract_repo_name_from_path(path: &str) -> Option<String> {
    let last = path.trim_end_matches('/').split('/').next_back()?;
    if last.is_empty() {
        return None;
    }
    let name = last.strip_suffix(".git").unwrap_or(last);
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Add repository dialog widget
pub struct AddRepoDialog;

impl AddRepoDialog {
    pub fn new() -> Self {
        Self
    }

    /// Render the dialog
    pub fn render(&self, area: Rect, buf: &mut Buffer, state: &AddRepoDialogState) {
        if !state.is_visible() {
            return;
        }

        // Render dialog frame (instructions render on bottom border)
        let frame = DialogFrame::new("Add Custom Project", 60, 11)
            .instructions(vec![("Enter", "add"), ("Esc", "cancel")]);
        let inner = frame.render(area, buf);

        // Layout inside dialog
        let chunks = Layout::vertical([
            Constraint::Length(1), // Label
            Constraint::Length(1), // Spacing
            Constraint::Length(3), // Input field (with border)
            Constraint::Length(1), // Status/error
            Constraint::Min(0),    // Remaining space
        ])
        .split(inner);

        // Render label
        let label =
            Paragraph::new("Enter local path or git URL:").style(Style::default().fg(Color::White));
        label.render(chunks[0], buf);

        // Render input field
        let input_style = if state.is_valid() {
            Style::default().fg(Color::Green)
        } else if state.error().is_some() {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::White)
        };

        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .border_style(input_style);

        let input_inner = input_block.inner(chunks[2]);
        input_block.render(chunks[2], buf);

        // Render input text with cursor and placeholder
        state.path.text.render_with_placeholder(
            input_inner,
            buf,
            Style::default().fg(Color::White),
            "~/path/to/repo or git@github.com:user/repo.git",
            Style::default().fg(Color::DarkGray),
        );

        // Render status/error using StatusLine component
        let success_msg = if state.is_url() {
            format!(
                "Will clone: {}",
                state.repo_name.as_deref().unwrap_or("repository")
            )
        } else {
            format!(
                "Valid repository: {}",
                state.repo_name.as_deref().unwrap_or("repository")
            )
        };
        let status = StatusLine::from_result(state.error(), state.is_valid(), &success_msg);
        status.render(chunks[3], buf);
    }
}

impl Default for AddRepoDialog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_ssh_url_with_git_suffix() {
        let name = detect_git_url_repo_name("git@github.com:Fuzzwah/recipes.git");
        assert_eq!(name, Some("recipes".to_string()));
    }

    #[test]
    fn test_detect_ssh_url_without_git_suffix() {
        let name = detect_git_url_repo_name("git@github.com:user/myrepo");
        assert_eq!(name, Some("myrepo".to_string()));
    }

    #[test]
    fn test_detect_https_url_with_git_suffix() {
        let name = detect_git_url_repo_name("https://github.com/user/myrepo.git");
        assert_eq!(name, Some("myrepo".to_string()));
    }

    #[test]
    fn test_detect_https_url_without_git_suffix() {
        let name = detect_git_url_repo_name("https://github.com/user/myrepo");
        assert_eq!(name, Some("myrepo".to_string()));
    }

    #[test]
    fn test_local_path_not_detected_as_url() {
        assert!(detect_git_url_repo_name("~/code/myrepo").is_none());
        assert!(detect_git_url_repo_name("/home/user/code/myrepo").is_none());
    }

    #[test]
    fn test_partial_input_not_detected_as_url() {
        assert!(detect_git_url_repo_name("git@").is_none());
        assert!(detect_git_url_repo_name("https://").is_none());
    }
}
