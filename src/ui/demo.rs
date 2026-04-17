//! Demo mode: populates app state with hardcoded fake data for screenshots.

use uuid::Uuid;

use crate::agent::{AgentMode, AgentType};
use crate::git::PrStatus;
use crate::ui::components::{ChatMessage, SidebarData};
use crate::ui::session::AgentSession;

pub struct DemoWorkspace {
    pub id: Uuid,
    pub name: String,
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub pr_status: Option<PrStatus>,
}

pub struct DemoRepo {
    pub id: Uuid,
    pub name: String,
    pub workspaces: Vec<DemoWorkspace>,
}

pub struct DemoState {
    pub repos: Vec<DemoRepo>,
    pub session: AgentSession,
    pub active_workspace_id: Uuid,
}

pub fn build_demo() -> DemoState {
    let conduit_repo_id = Uuid::new_v4();
    let slow_fern_id = Uuid::new_v4();
    let main_ws_id = Uuid::new_v4();

    let my_api_repo_id = Uuid::new_v4();
    let feature_parser_id = Uuid::new_v4();
    let my_api_main_id = Uuid::new_v4();

    let repos = vec![
        DemoRepo {
            id: conduit_repo_id,
            name: "conduit".to_string(),
            workspaces: vec![
                DemoWorkspace {
                    id: slow_fern_id,
                    name: "slow-fern".to_string(),
                    branch: "fuz/mock-ui-screenshots".to_string(),
                    ahead: 2,
                    behind: 0,
                    pr_status: Some(PrStatus {
                        exists: true,
                        number: Some(61),
                        checks: crate::git::CheckStatus {
                            total: 1,
                            passed: 1,
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                },
                DemoWorkspace {
                    id: main_ws_id,
                    name: "main".to_string(),
                    branch: "master".to_string(),
                    ahead: 0,
                    behind: 0,
                    pr_status: None,
                },
            ],
        },
        DemoRepo {
            id: my_api_repo_id,
            name: "my-api".to_string(),
            workspaces: vec![
                DemoWorkspace {
                    id: feature_parser_id,
                    name: "feature/parser".to_string(),
                    branch: "feature/parser".to_string(),
                    ahead: 3,
                    behind: 0,
                    pr_status: None,
                },
                DemoWorkspace {
                    id: my_api_main_id,
                    name: "main".to_string(),
                    branch: "main".to_string(),
                    ahead: 0,
                    behind: 2,
                    pr_status: None,
                },
            ],
        },
    ];

    let mut session = AgentSession::new(AgentType::Claude);
    session.workspace_id = Some(slow_fern_id);
    session.project_name = Some("conduit".to_string());
    session.workspace_name = Some("slow-fern".to_string());
    session.model = Some("claude-sonnet-4-6".to_string());
    session.agent_mode = AgentMode::Build;

    session.chat_view.push(ChatMessage::user(
        "Add error handling to the parser module.",
    ));

    session.chat_view.push(ChatMessage::tool(
        "Bash",
        "cargo test -- parser_tests",
        "running 4 tests\n\
         test parse_empty           ... ok\n\
         test parse_nested          ... ok\n\
         test parse_error           ... ok\n\
         test parse_unicode         ... ok\n\
         \n\
         test result: ok. 4 passed; 0 failed; 0 ignored",
    ));

    session.chat_view.push(ChatMessage::assistant(
        "All 4 tests pass. The error handler is in place.\n\
         The module now returns `ParseError::UnexpectedToken` on malformed input.",
    ));

    session.update_status();

    DemoState {
        repos,
        session,
        active_workspace_id: slow_fern_id,
    }
}

/// Populate a SidebarData from demo repos and return any workspace IDs that need
/// ahead/behind and PR status updates after the sidebar is built.
pub fn populate_sidebar(sidebar: &mut SidebarData, repos: &[DemoRepo]) {
    for repo in repos {
        let workspaces: Vec<(Uuid, String, String)> = repo
            .workspaces
            .iter()
            .map(|ws| (ws.id, ws.name.clone(), ws.branch.clone()))
            .collect();
        sidebar.add_repository(repo.id, &repo.name, workspaces);
        sidebar.expand_repo(repo.id);
    }

    for repo in repos {
        for ws in &repo.workspaces {
            if ws.ahead > 0 || ws.behind > 0 {
                sidebar.update_workspace_ahead_behind(ws.id, ws.ahead, ws.behind);
            }
            if let Some(ref status) = ws.pr_status {
                sidebar.update_workspace_pr_status(ws.id, Some(status.clone()));
            }
        }
    }
}
