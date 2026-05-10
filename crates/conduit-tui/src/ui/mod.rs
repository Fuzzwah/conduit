use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::{
    app_state::{AppState, CommandPaletteState, FocusArea, ModalState, ViewMode},
    session::{AgentSession, MessageRole},
    tab_manager::Tab,
};

pub fn render(frame: &mut Frame<'_>, state: &mut AppState) {
    let root = frame.area();
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(8),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(root);

    state.layout.tabs = vertical[0];
    state.layout.body = vertical[1];
    state.layout.composer = vertical[2];
    state.layout.status = vertical[3];

    let body_chunks = if state.sidebar_visible {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(40)])
            .split(vertical[1])
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(0), Constraint::Min(40)])
            .split(vertical[1])
    };
    state.layout.sidebar = body_chunks[0];

    render_tab_bar(frame, state);
    if state.sidebar_visible {
        render_sidebar(frame, state, body_chunks[0]);
    }
    render_body(frame, state, body_chunks[1]);
    render_composer(frame, state, vertical[2]);
    render_status(frame, state, vertical[3]);

    if state.command_palette.visible {
        render_command_palette(frame, &state.command_palette);
    }
    if let Some(modal) = &state.modal {
        render_modal(frame, modal);
    }
}

fn render_tab_bar(frame: &mut Frame<'_>, state: &AppState) {
    let titles = state.tabs.titles();
    let spans = titles
        .iter()
        .enumerate()
        .flat_map(|(index, title)| {
            let style = if index == state.tabs.active() {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            [
                Span::styled(format!(" {} ", index + 1), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{title}  "), style),
            ]
        })
        .collect::<Vec<_>>();

    frame.render_widget(Paragraph::new(Line::from(spans)), state.layout.tabs);
}

fn render_sidebar(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let mut items = Vec::new();
    let mut selected = 0usize;
    let mut cursor = 0usize;
    for repo in &state.repositories {
        items.push(ListItem::new(Line::from(vec![Span::styled(
            format!("▸ {}", repo.name),
            Style::default().add_modifier(Modifier::BOLD),
        )])));
        if state.selected_sidebar == cursor {
            selected = cursor;
        }
        cursor += 1;
        for workspace in &repo.workspaces {
            let line = format!(
                "  {}  ↑{} ↓{}  {}",
                workspace.branch, workspace.status.ahead, workspace.status.behind, workspace.status.pr_state
            );
            items.push(ListItem::new(line));
            if state.selected_sidebar == cursor {
                selected = cursor;
            }
            cursor += 1;
        }
    }

    let block = Block::default()
        .title(if state.focus == FocusArea::Sidebar {
            " Workspaces [focused] "
        } else {
            " Workspaces "
        })
        .borders(Borders::ALL)
        .border_style(if state.focus == FocusArea::Sidebar {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("→ ");
    let mut list_state = ratatui::widgets::ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_body(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    match state.tabs.active_tab() {
        Some(Tab::Session(session)) => match state.view_mode {
            ViewMode::Chat => render_chat(frame, session, area),
            ViewMode::RawEvents => render_raw_events(frame, session, area),
        },
        Some(Tab::File(file)) => {
            let widget = Paragraph::new(file.content.as_str())
                .block(Block::default().title(file.path.as_str()).borders(Borders::ALL))
                .wrap(Wrap { trim: false });
            frame.render_widget(widget, area);
        }
        None => frame.render_widget(Paragraph::new("No tabs"), area),
    }
}

fn render_chat(frame: &mut Frame<'_>, session: &AgentSession, area: Rect) {
    let lines = session
        .messages
        .iter()
        .flat_map(|message| {
            let style = match message.role {
                MessageRole::User => Style::default().fg(Color::Yellow),
                MessageRole::Assistant => Style::default().fg(Color::White),
                MessageRole::Reasoning => Style::default().fg(Color::Magenta),
                MessageRole::Tool => Style::default().fg(Color::Cyan),
                MessageRole::Error => Style::default().fg(Color::Red),
            };
            [
                Line::from(Span::styled(role_label(message.role), style.add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(message.text.clone(), style)),
                Line::default(),
            ]
        })
        .collect::<Vec<_>>();

    let title = if session.processing {
        " Chat • streaming "
    } else {
        " Chat "
    };
    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn render_raw_events(frame: &mut Frame<'_>, session: &AgentSession, area: Rect) {
    let items = session
        .raw_events
        .iter()
        .map(|event| ListItem::new(format!("{} — {}", event.label, event.detail)))
        .collect::<Vec<_>>();
    let list = List::new(items).block(Block::default().title(" Raw events ").borders(Borders::ALL));
    frame.render_widget(list, area);
}

fn render_composer(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let text = match state.tabs.active_tab() {
        Some(Tab::Session(session)) if session.composer.buffer.is_empty() => {
            "Type a prompt for the transport-neutral scaffold...".to_string()
        }
        Some(Tab::Session(session)) => session.composer.buffer.clone(),
        Some(Tab::File(_)) => "File tabs are read-only in this scaffold.".to_string(),
        None => String::new(),
    };
    let block = Block::default()
        .title(match state.focus {
            FocusArea::Composer => " Composer [focused] ",
            FocusArea::Sidebar => " Composer ",
        })
        .borders(Borders::ALL)
        .border_style(if state.focus == FocusArea::Composer {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });
    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn render_status(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let summary = match state.tabs.active_tab() {
        Some(Tab::Session(session)) => format!(
            " {} • {} • {} • tokens {} / {} • context {}/{} • {} • {} ",
            session.provider.id,
            session.model,
            session.mode.label(),
            session.token_usage.prompt_tokens,
            session.token_usage.completion_tokens,
            session.context.used,
            session.context.total,
            session.branch_name,
            session.pr_state,
        ),
        Some(Tab::File(file)) => format!(" file viewer • {} ", file.path),
        None => " no active tab ".to_string(),
    };
    frame.render_widget(
        Paragraph::new(summary)
            .style(Style::default().fg(Color::Black).bg(Color::Gray))
            .alignment(Alignment::Left),
        area,
    );
}

fn render_command_palette(frame: &mut Frame<'_>, state: &CommandPaletteState) {
    let area = centered_rect(frame.area(), 60, 45);
    frame.render_widget(Clear, area);
    let filtered = state.filtered_commands();
    let items = filtered
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let prefix = if index == state.selected { "> " } else { "  " };
            ListItem::new(format!("{prefix}{item}"))
        })
        .collect::<Vec<_>>();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(4)])
        .margin(1)
        .split(area);
    frame.render_widget(
        Block::default().title(" Command palette ").borders(Borders::ALL),
        area,
    );
    frame.render_widget(
        Paragraph::new(state.query.as_str())
            .block(Block::default().title(" Search ").borders(Borders::ALL)),
        chunks[0],
    );
    frame.render_widget(List::new(items), chunks[1]);
}

fn render_modal(frame: &mut Frame<'_>, modal: &ModalState) {
    let area = centered_rect(frame.area(), 58, 38);
    frame.render_widget(Clear, area);
    let inner = area.inner(Margin {
        vertical: 1,
        horizontal: 2,
    });
    frame.render_widget(
        Block::default()
            .title(modal.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
        area,
    );
    frame.render_widget(
        Paragraph::new(modal.body.as_str()).wrap(Wrap { trim: false }),
        inner,
    );
}

fn centered_rect(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_percent) / 2),
            Constraint::Percentage(height_percent),
            Constraint::Percentage((100 - height_percent) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1])[1]
}

fn role_label(role: MessageRole) -> &'static str {
    match role {
        MessageRole::User => "You",
        MessageRole::Assistant => "Assistant",
        MessageRole::Reasoning => "Reasoning",
        MessageRole::Tool => "Tool",
        MessageRole::Error => "Error",
    }
}
