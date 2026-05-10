#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub workspaces: Vec<Workspace>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: u64,
    pub name: String,
    pub branch: String,
    pub status: WorkspaceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceStatus {
    pub ahead: u16,
    pub behind: u16,
    pub pr_state: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderProfile {
    pub id: &'static str,
    pub default_model: &'static str,
    pub supports_plan_mode: bool,
}

pub trait WorkspaceCatalog {
    fn repositories(&self) -> Vec<Repository>;
}

pub fn demo_repositories() -> Vec<Repository> {
    vec![Repository {
        id: 1,
        name: "conduit".to_string(),
        path: "~/code/conduit".to_string(),
        workspaces: vec![
            Workspace {
                id: 11,
                name: "main".to_string(),
                branch: "master".to_string(),
                status: WorkspaceStatus {
                    ahead: 0,
                    behind: 0,
                    pr_state: "merged",
                },
            },
            Workspace {
                id: 12,
                name: "clean-room".to_string(),
                branch: "fuz/clean-room".to_string(),
                status: WorkspaceStatus {
                    ahead: 2,
                    behind: 0,
                    pr_state: "open",
                },
            },
        ],
    }]
}

pub fn default_provider() -> ProviderProfile {
    ProviderProfile {
        id: "codex",
        default_model: "gpt-5.4",
        supports_plan_mode: true,
    }
}
