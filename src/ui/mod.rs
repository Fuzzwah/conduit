pub mod action;
pub mod app;
pub mod app_prompt;
pub mod app_queue;
pub mod app_state;
pub mod capabilities;
pub mod clipboard_paste;
pub mod components;
pub mod demo;
pub mod effect;
pub mod events;
pub mod file_viewer;
pub mod git_tracker;
pub mod session;
pub mod tab;
pub mod tab_manager;
pub mod terminal_guard;

pub use action::Action;
pub use app::App;
pub use app_state::{AppState, PerformanceMetrics};
pub use capabilities::AgentCapabilities;
pub use effect::Effect;
pub use events::{AppEvent, InputMode};
pub use file_viewer::FileViewerSession;
pub use git_tracker::{GitTrackerHandle, GitTrackerUpdate};
pub use session::AgentSession;
pub use tab::Tab;
pub use tab_manager::TabManager;

/// Enter the terminal and draw the startup splash screen, returning the prepared terminal and guard.
///
/// Call this before constructing `App` so the splash is visible during initialization.
/// Pass the returned values to `App::run_with_prepared_terminal`.
pub fn prepare_and_show_splash() -> anyhow::Result<(
    ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    terminal_guard::TerminalGuard,
)> {
    use crossterm::{
        event::{EnableBracketedPaste, EnableMouseCapture},
        execute,
        terminal::{enable_raw_mode, EnterAlternateScreen},
    };
    use ratatui::{backend::CrosstermBackend, Terminal};

    enable_raw_mode()?;
    let guard = terminal_guard::TerminalGuard::new(false);
    let mut stdout = std::io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(components::draw_startup_splash)?;
    Ok((terminal, guard))
}
