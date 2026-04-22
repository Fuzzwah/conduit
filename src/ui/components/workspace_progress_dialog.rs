//! Dialog that streams git progress while a workspace is being created.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use super::dialog::DialogFrame;
use super::theme::{accent_primary, text_muted, text_primary};

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

const DIALOG_WIDTH: u16 = 68;
const LOG_LINES: usize = 10;

/// State for the workspace creation progress dialog.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceProgressDialogState {
    pub visible: bool,
    /// All progress messages received so far.
    pub messages: Vec<String>,
    /// Whether the creation has finished (success or failure).
    pub complete: bool,
    /// Set to the error string if creation failed.
    pub error: Option<String>,
    /// Spinner animation frame (advanced on each Tick while not complete).
    pub spinner_frame: usize,
}

impl WorkspaceProgressDialogState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.complete = false;
        self.error = None;
        self.messages.clear();
        self.spinner_frame = 0;
    }

    pub fn push(&mut self, message: impl Into<String>) {
        self.messages.push(message.into());
    }

    pub fn finish(&mut self) {
        self.complete = true;
    }

    pub fn finish_with_error(&mut self, error: impl Into<String>) {
        self.complete = true;
        self.error = Some(error.into());
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn tick(&mut self) {
        if !self.complete {
            self.spinner_frame = self.spinner_frame.wrapping_add(1);
        }
    }

    pub fn failed(&self) -> bool {
        self.error.is_some()
    }
}

/// Widget that renders the workspace creation progress dialog.
pub struct WorkspaceProgressDialog<'a> {
    state: &'a WorkspaceProgressDialogState,
}

impl<'a> WorkspaceProgressDialog<'a> {
    pub fn new(state: &'a WorkspaceProgressDialogState) -> Self {
        Self { state }
    }

    fn dialog_height(&self) -> u16 {
        // borders(2) + top_padding(1) + log_lines(10) + gap(1) + status(1) + gap(1) + button(1)
        // = 17 when complete, 15 when running (no button row + gap)
        if self.state.complete {
            17
        } else {
            15
        }
    }
}

impl Widget for WorkspaceProgressDialog<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.state.visible {
            return;
        }

        let instructions = if self.state.complete {
            vec![("Enter", "Close"), ("Esc", "Close")]
        } else {
            vec![]
        };

        let border_color = if self.state.failed() {
            ratatui::style::Color::Red
        } else if self.state.complete {
            ratatui::style::Color::Green
        } else {
            accent_primary()
        };

        let inner = DialogFrame::new("Creating Workspace", DIALOG_WIDTH, self.dialog_height())
            .border_color(border_color)
            .instructions(instructions)
            .render(area, buf);

        if inner.height == 0 {
            return;
        }

        // Render the last LOG_LINES messages, filling from top
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
            let line = Line::from(Span::styled(*msg, Style::default().fg(text_primary())));
            Paragraph::new(line).render(
                Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                },
                buf,
            );
        }

        // Status line: spinner while running, summary when done
        let status_y = inner.y + LOG_LINES as u16 + 1;
        if status_y < inner.y + inner.height {
            let status_line = if self.state.complete {
                if let Some(ref err) = self.state.error {
                    let truncated = if err.len() > DIALOG_WIDTH as usize - 4 {
                        format!("{}...", &err[..DIALOG_WIDTH as usize - 7])
                    } else {
                        err.clone()
                    };
                    Line::from(Span::styled(
                        format!("✗ {}", truncated),
                        Style::default().fg(ratatui::style::Color::Red),
                    ))
                } else {
                    Line::from(Span::styled(
                        "✓ Workspace created",
                        Style::default().fg(ratatui::style::Color::Green),
                    ))
                }
            } else {
                let frame = SPINNER_FRAMES[self.state.spinner_frame % SPINNER_FRAMES.len()];
                Line::from(vec![
                    Span::styled(format!("{} ", frame), Style::default().fg(accent_primary())),
                    Span::styled("Working...", Style::default().fg(text_muted())),
                ])
            };
            Paragraph::new(status_line).render(
                Rect {
                    x: inner.x,
                    y: status_y,
                    width: inner.width,
                    height: 1,
                },
                buf,
            );
        }

        // Close button — only when complete
        if self.state.complete {
            let button_y = status_y + 2;
            if button_y < inner.y + inner.height {
                let button = Span::styled(
                    " Close ",
                    Style::default()
                        .fg(ratatui::style::Color::Black)
                        .bg(if self.state.failed() {
                            ratatui::style::Color::Red
                        } else {
                            ratatui::style::Color::Green
                        })
                        .add_modifier(Modifier::BOLD),
                );
                Paragraph::new(Line::from(button))
                    .alignment(Alignment::Center)
                    .render(
                        Rect {
                            x: inner.x,
                            y: button_y,
                            width: inner.width,
                            height: 1,
                        },
                        buf,
                    );
            }
        }
    }
}
