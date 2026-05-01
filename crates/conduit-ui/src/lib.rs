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

/// Enter the terminal, animate the startup splash while `App` initializes, then run the app.
///
/// Runs `App::new_with_progress` on a blocking thread so the logo shine animation can play
/// on the current async task. Progress labels are shown below the logo as each phase completes.
pub async fn run_startup_with_splash(
    config: conduit_config::Config,
    tools: conduit_util::ToolAvailability,
) -> anyhow::Result<()> {
    use crossterm::{
        event::{EnableBracketedPaste, EnableMouseCapture},
        execute,
        terminal::{enable_raw_mode, EnterAlternateScreen},
    };
    use ratatui::{backend::CrosstermBackend, Terminal};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    enable_raw_mode()?;
    let mut guard = terminal_guard::TerminalGuard::new(false);
    let mut stdout = std::io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Shared status message updated by App::new_with_progress on the blocking thread
    let status = Arc::new(Mutex::new("Starting...".to_string()));
    let status_for_init = status.clone();

    // Run App::new() on a blocking thread so the animation can play on this task
    let init_handle = tokio::task::spawn_blocking(move || {
        App::new_with_progress(config, tools, move |label| {
            if let Ok(mut s) = status_for_init.lock() {
                *s = label.to_string();
            }
        })
    });

    let mut anim = components::LogoShineAnimation::new();
    let mut tick = 0u32;
    loop {
        let status_text = status.lock().unwrap().clone();
        terminal.draw(|f| components::draw_startup_splash_animated(f, &anim, &status_text))?;
        tick += 1;
        if tick.is_multiple_of(3) {
            anim.tick();
        }
        tokio::time::sleep(Duration::from_millis(16)).await;
        if init_handle.is_finished() {
            break;
        }
    }

    let mut app = init_handle.await?;
    app.run_with_prepared_terminal(&mut terminal, &mut guard)
        .await
}
