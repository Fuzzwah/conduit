//! Dialog for viewing and updating MCP server configuration at project or workspace scope.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use uuid::Uuid;

use conduit_types::InputMode;

use super::{
    bg_highlight, dialog_bg, ensure_contrast_bg, ensure_contrast_fg, text_muted, text_primary,
    DialogFrame,
};

const DIALOG_WIDTH: u16 = 66;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum McpScope {
    #[default]
    Project,
    Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpSource {
    Codex,
    McpJson,
}

impl McpSource {
    pub fn label(self) -> &'static str {
        match self {
            McpSource::Codex => ".codex",
            McpSource::McpJson => ".mcp.json",
        }
    }
}

#[derive(Debug, Clone)]
pub struct McpServer {
    pub name: String,
    pub source: McpSource,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct McpDialogState {
    pub visible: bool,
    pub scope: McpScope,
    pub repo_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub project_name: String,
    pub workspace_name: Option<String>,
    /// true when workspace has no saved MCP config yet (pre-populated from project)
    pub workspace_config_is_inherited: bool,
    pub project_servers: Vec<McpServer>,
    pub workspace_servers: Vec<McpServer>,
    /// Cursor position: 0..servers.len() for server rows, servers.len() for Save
    pub selected: usize,
    /// Mode to restore on cancel/save
    pub return_to_input_mode: InputMode,
}

impl Default for McpDialogState {
    fn default() -> Self {
        Self {
            visible: false,
            scope: McpScope::Project,
            repo_id: None,
            workspace_id: None,
            project_name: String::new(),
            workspace_name: None,
            workspace_config_is_inherited: false,
            project_servers: Vec::new(),
            workspace_servers: Vec::new(),
            selected: 0,
            return_to_input_mode: InputMode::Normal,
        }
    }
}

/// Parameters for opening the MCP configuration dialog.
pub struct McpDialogParams {
    pub scope: McpScope,
    pub repo_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub project_name: String,
    pub workspace_name: Option<String>,
    pub project_servers: Vec<McpServer>,
    pub workspace_servers: Vec<McpServer>,
    pub workspace_config_is_inherited: bool,
    pub return_to_input_mode: InputMode,
}

impl McpDialogState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, params: McpDialogParams) {
        self.visible = true;
        self.scope = params.scope;
        self.repo_id = Some(params.repo_id);
        self.workspace_id = params.workspace_id;
        self.project_name = params.project_name;
        self.workspace_name = params.workspace_name;
        self.workspace_config_is_inherited = params.workspace_config_is_inherited;
        self.project_servers = params.project_servers;
        self.workspace_servers = params.workspace_servers;
        self.selected = 0;
        self.return_to_input_mode = params.return_to_input_mode;
    }

    pub fn hide(&mut self) {
        *self = Self::default();
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn current_servers(&self) -> &[McpServer] {
        match self.scope {
            McpScope::Project => &self.project_servers,
            McpScope::Workspace => &self.workspace_servers,
        }
    }

    pub fn current_servers_mut(&mut self) -> &mut Vec<McpServer> {
        match self.scope {
            McpScope::Project => &mut self.project_servers,
            McpScope::Workspace => &mut self.workspace_servers,
        }
    }

    pub fn toggle_scope(&mut self) {
        self.scope = match self.scope {
            McpScope::Project => McpScope::Workspace,
            McpScope::Workspace => McpScope::Project,
        };
        self.selected = 0;
    }

    pub fn select_next(&mut self) {
        let max = self.current_servers().len(); // len() = Save row index
        if self.selected < max {
            self.selected += 1;
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Activate the currently selected row.
    /// Returns true if the Save row was activated (caller should save and close).
    pub fn activate_selected(&mut self) -> bool {
        let server_count = self.current_servers().len();
        if self.selected == server_count {
            return true; // Save
        }
        let selected = self.selected;
        let servers = self.current_servers_mut();
        if let Some(server) = servers.get_mut(selected) {
            server.enabled = !server.enabled;
        }
        false
    }

    pub fn disabled_servers_for_current_scope(&self) -> Vec<String> {
        self.current_servers()
            .iter()
            .filter(|s| !s.enabled)
            .map(|s| s.name.clone())
            .collect()
    }
}

pub struct McpDialog;

impl McpDialog {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, state: &McpDialogState) {
        let server_count = state.current_servers().len() as u16;
        // 2 header lines + 1 separator + 2 tab lines + 1 separator + servers + 1 save + instructions
        let inner_height = 2 + 1 + 2 + 1 + server_count.max(1) + 1;
        let dialog_height = inner_height + 4; // border (2) + instruction line (1) + padding (1)

        let frame = DialogFrame::new(" MCP Configuration ", DIALOG_WIDTH, dialog_height)
            .instructions(vec![
                ("↑↓", "navigate"),
                ("Enter", "toggle/save"),
                ("←→/Tab", "switch scope"),
                ("Esc", "cancel"),
            ]);
        let inner = frame.render(area, buf);

        let chunks = Layout::vertical([
            Constraint::Length(1), // project/workspace name
            Constraint::Length(1), // separator
            Constraint::Length(1), // scope tab row
            Constraint::Length(1), // scope subtitle (inherited hint or empty)
            Constraint::Length(1), // separator
            Constraint::Min(1),    // server list + save
        ])
        .split(inner);

        // Header: project name / workspace name
        let header = match &state.workspace_name {
            Some(ws) => format!("{} / {}", state.project_name, ws),
            None => state.project_name.clone(),
        };
        Paragraph::new(header)
            .style(Style::default().fg(text_muted()))
            .render(chunks[0], buf);

        // Separator
        Paragraph::new("─".repeat(chunks[1].width as usize))
            .style(Style::default().fg(text_muted()))
            .render(chunks[1], buf);

        // Scope tabs
        self.render_scope_tabs(chunks[2], buf, state);

        // Scope subtitle
        let subtitle = if state.scope == McpScope::Workspace && state.workspace_config_is_inherited
        {
            "  (using project defaults — save to create workspace override)"
        } else {
            ""
        };
        Paragraph::new(subtitle)
            .style(Style::default().fg(text_muted()))
            .render(chunks[3], buf);

        // Separator
        Paragraph::new("─".repeat(chunks[4].width as usize))
            .style(Style::default().fg(text_muted()))
            .render(chunks[4], buf);

        // Server list + Save
        self.render_server_rows(chunks[5], buf, state);
    }

    fn render_scope_tabs(&self, area: Rect, buf: &mut Buffer, state: &McpDialogState) {
        let project_active = state.scope == McpScope::Project;

        let project_style = if project_active {
            Style::default()
                .fg(ensure_contrast_fg(text_primary(), bg_highlight(), 4.5))
                .bg(bg_highlight())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_muted())
        };
        let workspace_style = if !project_active {
            Style::default()
                .fg(ensure_contrast_fg(text_primary(), bg_highlight(), 4.5))
                .bg(bg_highlight())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(text_muted())
        };

        let line = Line::from(vec![
            Span::styled(" Project ", project_style),
            Span::styled("  ", Style::default().fg(text_muted())),
            Span::styled(" Workspace ", workspace_style),
        ]);
        Paragraph::new(line).render(area, buf);
    }

    fn render_server_rows(&self, area: Rect, buf: &mut Buffer, state: &McpDialogState) {
        let servers = state.current_servers();
        let save_row = servers.len();

        let mut row = 0u16;
        if servers.is_empty() {
            if row < area.height {
                let line_area = Rect {
                    x: area.x,
                    y: area.y + row,
                    width: area.width,
                    height: 1,
                };
                Paragraph::new("  No MCP servers detected.")
                    .style(Style::default().fg(text_muted()))
                    .render(line_area, buf);
                row += 1;
            }
        } else {
            let source_col_width: u16 = 10;
            for (i, server) in servers.iter().enumerate() {
                if row >= area.height {
                    break;
                }
                let line_area = Rect {
                    x: area.x,
                    y: area.y + row,
                    width: area.width,
                    height: 1,
                };
                let is_selected = i == state.selected;
                self.render_row_bg(line_area, buf, is_selected);
                let bg = if is_selected {
                    ensure_contrast_bg(bg_highlight(), dialog_bg(), 2.0)
                } else {
                    dialog_bg()
                };
                let fg = if is_selected {
                    ensure_contrast_fg(text_primary(), bg, 4.5)
                } else {
                    text_primary()
                };

                let check = if server.enabled { "[✓]" } else { "[✗]" };
                let name_width = area
                    .width
                    .saturating_sub(source_col_width)
                    .saturating_sub(6) as usize;
                let name_padded = format!("{:<width$}", server.name, width = name_width);
                let source_label = format!(
                    "{:>width$}",
                    server.source.label(),
                    width = source_col_width as usize
                );

                let line = Line::from(vec![
                    Span::styled(format!("  {} ", check), Style::default().fg(fg).bg(bg)),
                    Span::styled(name_padded, Style::default().fg(fg).bg(bg)),
                    Span::styled(source_label, Style::default().fg(text_muted()).bg(bg)),
                ]);
                Paragraph::new(line).render(line_area, buf);
                row += 1;
            }
        }

        // Save row
        if row < area.height {
            let line_area = Rect {
                x: area.x,
                y: area.y + row,
                width: area.width,
                height: 1,
            };
            let is_selected = state.selected == save_row;
            self.render_row_bg(line_area, buf, is_selected);
            let bg = if is_selected {
                ensure_contrast_bg(bg_highlight(), dialog_bg(), 2.0)
            } else {
                dialog_bg()
            };
            let fg = if is_selected {
                ensure_contrast_fg(text_primary(), bg, 4.5)
            } else {
                text_primary()
            };
            Paragraph::new(Line::from(vec![Span::styled(
                "  Save changes",
                Style::default().fg(fg).bg(bg),
            )]))
            .render(line_area, buf);
        }
    }

    fn render_row_bg(&self, area: Rect, buf: &mut Buffer, is_selected: bool) {
        let bg = if is_selected {
            ensure_contrast_bg(bg_highlight(), dialog_bg(), 2.0)
        } else {
            dialog_bg()
        };
        for x in area.x..area.x.saturating_add(area.width) {
            buf[(x, area.y)].set_bg(bg);
        }
    }
}

impl Default for McpDialog {
    fn default() -> Self {
        Self::new()
    }
}
