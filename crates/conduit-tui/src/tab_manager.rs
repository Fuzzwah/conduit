use crate::domain::ProviderProfile;
use crate::session::AgentSession;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileViewerTab {
    pub title: String,
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum Tab {
    Session(AgentSession),
    File(FileViewerTab),
}

impl Tab {
    pub fn title(&self) -> &str {
        match self {
            Self::Session(session) => &session.title,
            Self::File(file) => &file.title,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabManager {
    tabs: Vec<Tab>,
    active: usize,
    next_session_id: u64,
}

impl TabManager {
    pub fn new(provider: ProviderProfile) -> Self {
        Self {
            tabs: vec![Tab::Session(AgentSession::demo(
                1,
                "Workspace: clean-room",
                provider,
            ))],
            active: 0,
            next_session_id: 2,
        }
    }

    pub fn titles(&self) -> Vec<&str> {
        self.tabs.iter().map(Tab::title).collect()
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn active(&self) -> usize {
        self.active
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active)
    }

    pub fn active_session_mut(&mut self) -> Option<&mut AgentSession> {
        match self.active_tab_mut() {
            Some(Tab::Session(session)) => Some(session),
            _ => None,
        }
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active = (self.active + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active = if self.active == 0 {
                self.tabs.len() - 1
            } else {
                self.active - 1
            };
        }
    }

    pub fn open_session(&mut self, provider: ProviderProfile) -> u64 {
        let id = self.next_session_id;
        self.next_session_id += 1;
        self.tabs.push(Tab::Session(AgentSession::demo(
            id,
            format!("Workspace: demo-{id}"),
            provider,
        )));
        self.active = self.tabs.len() - 1;
        id
    }

    pub fn open_file(&mut self, path: impl Into<String>, content: impl Into<String>) {
        let path = path.into();
        let title = path.rsplit('/').next().unwrap_or(&path).to_string();
        self.tabs.push(Tab::File(FileViewerTab {
            title,
            path,
            content: content.into(),
        }));
        self.active = self.tabs.len() - 1;
    }

    pub fn close_active(&mut self) {
        if self.tabs.len() <= 1 {
            return;
        }
        self.tabs.remove(self.active);
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::default_provider;

    use super::TabManager;

    #[test]
    fn cycles_tabs() {
        let mut tabs = TabManager::new(default_provider());
        tabs.open_file("README.md", "demo");
        tabs.prev_tab();
        assert_eq!(tabs.active(), 0);
        tabs.next_tab();
        assert_eq!(tabs.active(), 1);
    }

    #[test]
    fn keeps_one_tab_open() {
        let mut tabs = TabManager::new(default_provider());
        tabs.close_active();
        assert_eq!(tabs.len(), 1);
    }
}
