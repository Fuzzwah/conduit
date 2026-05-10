use ratatui::layout::Rect;

use crate::domain::{default_provider, demo_repositories, Repository};
use crate::tab_manager::TabManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusArea {
    Sidebar,
    Composer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Chat,
    RawEvents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteState {
    pub visible: bool,
    pub query: String,
    pub selected: usize,
    pub commands: Vec<&'static str>,
}

impl Default for CommandPaletteState {
    fn default() -> Self {
        Self {
            visible: false,
            query: String::new(),
            selected: 0,
            commands: vec![
                "New session",
                "Open file tab",
                "Toggle raw events",
                "Toggle sidebar focus",
                "Toggle build/plan",
                "Help",
                "Quit",
            ],
        }
    }
}

impl CommandPaletteState {
    pub fn filtered_commands(&self) -> Vec<&'static str> {
        self.commands
            .iter()
            .copied()
            .filter(|command| {
                self.query.is_empty()
                    || command
                        .to_ascii_lowercase()
                        .contains(&self.query.to_ascii_lowercase())
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModalState {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LayoutRegions {
    pub sidebar: Rect,
    pub tabs: Rect,
    pub body: Rect,
    pub composer: Rect,
    pub status: Rect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    pub should_quit: bool,
    pub tick: u64,
    pub repositories: Vec<Repository>,
    pub selected_sidebar: usize,
    pub sidebar_visible: bool,
    pub focus: FocusArea,
    pub view_mode: ViewMode,
    pub command_palette: CommandPaletteState,
    pub modal: Option<ModalState>,
    pub layout: LayoutRegions,
    pub tabs: TabManager,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let provider = default_provider();
        Self {
            should_quit: false,
            tick: 0,
            repositories: demo_repositories(),
            selected_sidebar: 0,
            sidebar_visible: true,
            focus: FocusArea::Composer,
            view_mode: ViewMode::Chat,
            command_palette: CommandPaletteState::default(),
            modal: None,
            layout: LayoutRegions::default(),
            tabs: TabManager::new(provider),
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            FocusArea::Sidebar => FocusArea::Composer,
            FocusArea::Composer => FocusArea::Sidebar,
        };
    }

    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Chat => ViewMode::RawEvents,
            ViewMode::RawEvents => ViewMode::Chat,
        };
    }
}
