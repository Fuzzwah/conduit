//! Dialog shown during the `SyncingRemote` phase of workspace creation.
//!
//! This is a lightweight, auto-dismissing dialog: it appears when `git fetch`
//! starts on the base repo, streams each line of git output, and is hidden by
//! `app.rs` when the sync completes. Unlike `WorkspaceProgressDialog`, this
//! never shows a Close button — it simply disappears once the next phase of
//! the workspace-creation flow takes over the screen.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use super::dialog::DialogFrame;
use super::theme::{accent_primary, text_muted, text_primary};

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const DIALOG_WIDTH: u16 = 68;
const LOG_LINES: usize = 10;

/// State for the remote-sync progress dialog.
#[derive(Debug, Clone, Default)]
pub struct RemoteSyncDialogState {
    pub visible: bool,
    /// Streaming lines from `git fetch`.
    pub messages: Vec<String>,
    /// Spinner animation frame, advanced on each Tick while visible.
    pub spinner_frame: usize,
}

impl RemoteSyncDialogState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.messages.clear();
        self.spinner_frame = 0;
    }

    pub fn push(&mut self, message: impl Into<String>) {
        self.messages.push(message.into());
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.messages.clear();
    }

    pub fn tick(&mut self) {
        if self.visible {
            self.spinner_frame = self.spinner_frame.wrapping_add(1);
        }
    }
}

/// Widget that renders the remote-sync progress dialog.
pub struct RemoteSyncDialog<'a> {
    state: &'a RemoteSyncDialogState,
}

impl<'a> RemoteSyncDialog<'a> {
    pub fn new(state: &'a RemoteSyncDialogState) -> Self {
        Self { state }
    }

    fn dialog_height(&self) -> u16 {
        // borders(2) + top_padding(1) + log_lines(10) + gap(1) + status(1) = 15
        15
    }
}

impl Widget for RemoteSyncDialog<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.state.visible {
            return;
        }

        let inner = DialogFrame::new("Syncing with Remote", DIALOG_WIDTH, self.dialog_height())
            .border_color(accent_primary())
            .render(area, buf);

        if inner.height == 0 {
            return;
        }

        let visible: Vec<&str> = {
            let msgs = &self.state.messages;
            if msgs.len() > LOG_LINES {
                msgs[msgs.len() - LOG_LINES..]
                    .iter()
                    .map(String::as_str)
                    .collect()
            } else {
                msgs.iter().map(String::as_str).collect()
            }
        };

        for (i, msg) in visible.iter().enumerate() {
            let y = inner.y + i as u16;
            if y >= inner.y + inner.height {
                break;
            }
            Paragraph::new(Line::from(Span::styled(
                *msg,
                Style::default().fg(text_primary()),
            )))
            .render(
                Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                },
                buf,
            );
        }

        let status_y = inner.y + LOG_LINES as u16 + 1;
        if status_y < inner.y + inner.height {
            let frame = SPINNER_FRAMES[self.state.spinner_frame % SPINNER_FRAMES.len()];
            let line = Line::from(vec![
                Span::styled(format!("{} ", frame), Style::default().fg(accent_primary())),
                Span::styled("Fetching from remote...", Style::default().fg(text_muted())),
            ]);
            Paragraph::new(line).alignment(Alignment::Left).render(
                Rect {
                    x: inner.x,
                    y: status_y,
                    width: inner.width,
                    height: 1,
                },
                buf,
            );
        }
    }
}
