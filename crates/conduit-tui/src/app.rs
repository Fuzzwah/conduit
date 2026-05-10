use std::{
    io::{self, stdout},
    time::Duration,
};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::{
    app_state::{AppState, FocusArea, ModalState},
    domain::default_provider,
    runtime::{MockTransport, SessionTransport},
    tab_manager::Tab,
    ui,
};

struct TerminalSession;

impl TerminalSession {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
    }
}

pub fn run() -> Result<()> {
    let _session = TerminalSession::enter()?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new();
    app.run(&mut terminal)
}

pub struct App {
    state: AppState,
    transport: MockTransport,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: AppState::new(),
            transport: MockTransport::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
        while !self.state.should_quit {
            terminal.draw(|frame| ui::render(frame, &mut self.state))?;
            self.poll_runtime();
            if event::poll(Duration::from_millis(80))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key);
                }
            }
            self.state.tick += 1;
        }
        Ok(())
    }

    fn poll_runtime(&mut self) {
        for envelope in self.transport.drain_ready(self.state.tick) {
            if let Some(session) = self.state.tabs.active_session_mut() {
                if session.id == envelope.session_id {
                    session.apply_runtime_event(envelope.event);
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.state.modal.is_some() {
            if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
                self.state.modal = None;
            }
            return;
        }

        if self.state.command_palette.visible {
            self.handle_command_palette_key(key);
            return;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => self.state.should_quit = true,
            (KeyCode::Char('p'), KeyModifiers::CONTROL) => self.state.command_palette.visible = true,
            (KeyCode::Char('g'), KeyModifiers::CONTROL) => self.state.toggle_view_mode(),
            (KeyCode::Char('b'), KeyModifiers::CONTROL) => self.state.toggle_focus(),
            (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                self.state.tabs.open_session(default_provider());
            }
            (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                self.state.tabs.open_file(
                    "/home/runner/work/conduit/conduit/crates/conduit-tui/README.md",
                    include_str!("../README.md"),
                );
            }
            (KeyCode::Char('4'), KeyModifiers::CONTROL) => {
                if let Some(session) = self.state.tabs.active_session_mut() {
                    session.mode = session.mode.toggle();
                }
            }
            (KeyCode::Tab, _) => self.state.tabs.next_tab(),
            (KeyCode::BackTab, _) => self.state.tabs.prev_tab(),
            (KeyCode::F(1), _) => self.open_help(),
            (KeyCode::Esc, _) => self.state.focus = FocusArea::Composer,
            _ => self.handle_context_key(key),
        }
    }

    fn handle_command_palette_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.state.command_palette.visible = false,
            KeyCode::Up => {
                self.state.command_palette.selected = self.state.command_palette.selected.saturating_sub(1)
            }
            KeyCode::Down => {
                let max = self
                    .state
                    .command_palette
                    .filtered_commands()
                    .len()
                    .saturating_sub(1);
                self.state.command_palette.selected = self.state.command_palette.selected.min(max).saturating_add(1).min(max);
            }
            KeyCode::Backspace => {
                self.state.command_palette.query.pop();
                self.state.command_palette.selected = 0;
            }
            KeyCode::Enter => {
                let selected = self
                    .state
                    .command_palette
                    .filtered_commands()
                    .get(self.state.command_palette.selected)
                    .copied();
                self.state.command_palette.visible = false;
                if let Some(command) = selected {
                    self.execute_palette_command(command);
                }
            }
            KeyCode::Char(c) if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
                self.state.command_palette.query.push(c);
                self.state.command_palette.selected = 0;
            }
            _ => {}
        }
    }

    fn execute_palette_command(&mut self, command: &str) {
        match command {
            "New session" => {
                self.state.tabs.open_session(default_provider());
            }
            "Open file tab" => {
                self.state.tabs.open_file(
                    "/home/runner/work/conduit/conduit/crates/conduit-tui/README.md",
                    include_str!("../README.md"),
                );
            }
            "Toggle raw events" => self.state.toggle_view_mode(),
            "Toggle sidebar focus" => self.state.toggle_focus(),
            "Toggle build/plan" => {
                if let Some(session) = self.state.tabs.active_session_mut() {
                    session.mode = session.mode.toggle();
                }
            }
            "Help" => self.open_help(),
            "Quit" => self.state.should_quit = true,
            _ => {}
        }
    }

    fn handle_context_key(&mut self, key: KeyEvent) {
        match self.state.focus {
            FocusArea::Sidebar => match key.code {
                KeyCode::Down => self.state.selected_sidebar = self.state.selected_sidebar.saturating_add(1),
                KeyCode::Up => self.state.selected_sidebar = self.state.selected_sidebar.saturating_sub(1),
                KeyCode::Enter => self.open_sidebar_selection(),
                _ => {}
            },
            FocusArea::Composer => {
                if let Some(Tab::Session(session)) = self.state.tabs.active_tab_mut() {
                    match key.code {
                        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                            session.composer.buffer.push('\n');
                        }
                        KeyCode::Enter => {
                            let prompt = std::mem::take(&mut session.composer.buffer);
                            if !prompt.trim().is_empty() {
                                session.push_user_prompt(prompt.clone());
                                self.transport.submit_prompt(session.id, &prompt, self.state.tick);
                            }
                        }
                        KeyCode::Backspace => {
                            session.composer.buffer.pop();
                        }
                        KeyCode::Char(c)
                            if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
                        {
                            session.composer.buffer.push(c);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn open_help(&mut self) {
        self.state.modal = Some(ModalState {
            title: "Clean-room TUI scaffold".to_string(),
            body: "This crate implements the first-pass architecture from the TUI-only plan:\n\n- transport-neutral runtime events\n- sidebar, tabs, chat/raw events, composer, status bar\n- command palette and modal shell\n- file tab placeholder and demo domain data\n\nUse Ctrl+P to explore commands, Ctrl+G to toggle raw events, and Enter to stream mock events into the active session.".to_string(),
        });
    }

    fn open_sidebar_selection(&mut self) {
        let Some(repository) = self.state.repositories.first() else {
            return;
        };
        let workspace_count = repository.workspaces.len();
        if self.state.selected_sidebar == 0 || self.state.selected_sidebar > workspace_count {
            return;
        }
        let workspace = &repository.workspaces[self.state.selected_sidebar - 1];
        if let Some(session) = self.state.tabs.active_session_mut() {
            session.title = format!("Workspace: {}", workspace.name);
            session.branch_name = workspace.branch.clone();
            session.pr_state = workspace.status.pr_state.to_string();
        }
    }
}
