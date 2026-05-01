use crate::action::Action;
use crate::app::App;
use crate::components::ChatView;
use crate::events::InputMode;

impl App {
    pub(super) fn handle_scroll_action(&mut self, action: Action) {
        // Handle file viewer scrolling if active tab is a file
        if self.state.tab_manager.active_is_file() {
            self.handle_file_viewer_scroll(&action);
            return;
        }

        match action {
            Action::ScrollUp(n) => {
                if self.state.input_mode == InputMode::ShowingHelp {
                    self.state.help_dialog_state.scroll_up(n as usize);
                } else {
                    if let Some(session) = self.state.tab_manager.active_session_mut() {
                        session.chat_view.scroll_up(n as usize);
                    }
                    self.record_scroll(n as usize);
                }
            }
            Action::ScrollDown(n) => {
                if self.state.input_mode == InputMode::ShowingHelp {
                    self.state.help_dialog_state.scroll_down(n as usize);
                } else {
                    if let Some(session) = self.state.tab_manager.active_session_mut() {
                        session.chat_view.scroll_down(n as usize);
                    }
                    self.record_scroll(n as usize);
                }
            }
            Action::ScrollPageUp => {
                if self.state.input_mode == InputMode::ShowingHelp {
                    self.state.help_dialog_state.page_up();
                } else {
                    if let Some(session) = self.state.tab_manager.active_session_mut() {
                        session.chat_view.scroll_up(10);
                    }
                    self.record_scroll(10);
                }
            }
            Action::ScrollPageDown => {
                if self.state.input_mode == InputMode::ShowingHelp {
                    self.state.help_dialog_state.page_down();
                } else {
                    if let Some(session) = self.state.tab_manager.active_session_mut() {
                        session.chat_view.scroll_down(10);
                    }
                    self.record_scroll(10);
                }
            }
            Action::ScrollToTop => {
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    session.chat_view.scroll_to_top();
                }
            }
            Action::ScrollToBottom => {
                if let Some(session) = self.state.tab_manager.active_session_mut() {
                    session.chat_view.scroll_to_bottom();
                }
            }
            Action::ScrollPrevUserMessage => {
                let show_chat_scrollbar = self.config().ui.show_chat_scrollbar;
                if let (Some(session), Some(chat_area)) = (
                    self.state.tab_manager.active_session_mut(),
                    self.state.chat_area,
                ) {
                    if let Some(content) =
                        ChatView::content_area_for(chat_area, show_chat_scrollbar)
                    {
                        let mut extra_len = 0usize;
                        if session.is_processing {
                            extra_len += 1;
                        }
                        if let Some(queue_lines) = crate::app_queue::build_queue_lines(
                            session,
                            chat_area.width,
                            self.state.input_mode,
                        ) {
                            extra_len += queue_lines.len();
                        }
                        if extra_len > 0 {
                            extra_len += 1; // spacing line after extras
                        }

                        session.chat_view.scroll_to_prev_user_message(
                            content.width,
                            content.height as usize,
                            extra_len,
                        );
                    }
                }
            }
            Action::ScrollNextUserMessage => {
                let show_chat_scrollbar = self.config().ui.show_chat_scrollbar;
                if let (Some(session), Some(chat_area)) = (
                    self.state.tab_manager.active_session_mut(),
                    self.state.chat_area,
                ) {
                    if let Some(content) =
                        ChatView::content_area_for(chat_area, show_chat_scrollbar)
                    {
                        let mut extra_len = 0usize;
                        if session.is_processing {
                            extra_len += 1;
                        }
                        if let Some(queue_lines) = crate::app_queue::build_queue_lines(
                            session,
                            chat_area.width,
                            self.state.input_mode,
                        ) {
                            extra_len += queue_lines.len();
                        }
                        if extra_len > 0 {
                            extra_len += 1; // spacing line after extras
                        }

                        session.chat_view.scroll_to_next_user_message(
                            content.width,
                            content.height as usize,
                            extra_len,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    /// Handle scroll actions for file viewer
    fn handle_file_viewer_scroll(&mut self, action: &Action) {
        let visible_height = self
            .state
            .file_viewer_area
            .map(|area| area.height as usize)
            .unwrap_or(20)
            .max(1);

        match action {
            Action::ScrollUp(n) => {
                if let Some(file_session) = self.state.tab_manager.active_file_viewer_mut() {
                    file_session.scroll_up(*n as usize);
                }
            }
            Action::ScrollDown(n) => {
                if let Some(file_session) = self.state.tab_manager.active_file_viewer_mut() {
                    file_session.scroll_down_clamped(*n as usize, visible_height);
                }
            }
            Action::ScrollPageUp => {
                if let Some(file_session) = self.state.tab_manager.active_file_viewer_mut() {
                    file_session.page_up_exact(visible_height);
                }
            }
            Action::ScrollPageDown => {
                if let Some(file_session) = self.state.tab_manager.active_file_viewer_mut() {
                    file_session.page_down_exact(visible_height);
                }
            }
            Action::ScrollToTop => {
                if let Some(file_session) = self.state.tab_manager.active_file_viewer_mut() {
                    file_session.scroll_to_top();
                }
            }
            Action::ScrollToBottom => {
                if let Some(file_session) = self.state.tab_manager.active_file_viewer_mut() {
                    file_session.scroll_to_bottom_exact(visible_height);
                }
            }
            // User message navigation doesn't apply to file viewer
            Action::ScrollPrevUserMessage | Action::ScrollNextUserMessage => {}
            _ => {}
        }
    }
}
