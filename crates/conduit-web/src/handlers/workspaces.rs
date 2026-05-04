//! Workspace handlers for the Conduit web API.

use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    Json,
};
use futures::stream;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::error::WebError;
use crate::handlers::sessions::SessionResponse;
use crate::state::WebAppState;
use crate::status_types::{PrStatusResponse, WorkspaceStatusResponse};
use conduit_core::resolve_repo_workspace_settings;
use conduit_core::services::{ServiceError, SessionService};
use conduit_data::Workspace;
use conduit_git::PrManager;
use conduit_util::names::{generate_branch_name, generate_workspace_name, get_git_username};
use conduit_util::workspace_setup::run_workspace_setup_script;

/// Response for a single workspace.
#[derive(Debug, Serialize)]
pub struct WorkspaceResponse {
    pub id: Uuid,
    pub repository_id: Uuid,
    pub name: String,
    pub branch: String,
    pub path: String,
    pub created_at: String,
    pub last_accessed: String,
    pub is_default: bool,
    pub archived_at: Option<String>,
}

impl From<Workspace> for WorkspaceResponse {
    fn from(ws: Workspace) -> Self {
        Self {
            id: ws.id,
            repository_id: ws.repository_id,
            name: ws.name,
            branch: ws.branch,
            path: ws.path.to_string_lossy().to_string(),
            created_at: ws.created_at.to_rfc3339(),
            last_accessed: ws.last_accessed.to_rfc3339(),
            is_default: ws.is_default,
            archived_at: ws.archived_at.map(|d| d.to_rfc3339()),
        }
    }
}

/// Response for listing workspaces.
#[derive(Debug, Serialize)]
pub struct ListWorkspacesResponse {
    pub workspaces: Vec<WorkspaceResponse>,
}

/// PR preflight response for a workspace.
#[derive(Debug, Serialize)]
pub struct PrPreflightResponse {
    pub gh_installed: bool,
    pub gh_authenticated: bool,
    pub on_main_branch: bool,
    pub branch_name: String,
    pub target_branch: String,
    pub uncommitted_count: usize,
    pub has_upstream: bool,
    pub existing_pr: Option<PrStatusResponse>,
}

/// PR create response returns prompt to send to agent.
#[derive(Debug, Serialize)]
pub struct PrCreateResponse {
    pub preflight: PrPreflightResponse,
    pub prompt: String,
}

/// Request to create a new workspace.
#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub branch: String,
    pub path: String,
    #[serde(default)]
    pub is_default: bool,
}

/// List all workspaces.
pub async fn list_workspaces(
    State(state): State<WebAppState>,
) -> Result<Json<ListWorkspacesResponse>, WebError> {
    let core = state.core().await;
    let store = core
        .workspace_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;

    let workspaces = store
        .get_all()
        .map_err(|e| WebError::Internal(format!("Failed to list workspaces: {}", e)))?;

    Ok(Json(ListWorkspacesResponse {
        workspaces: workspaces
            .into_iter()
            .map(WorkspaceResponse::from)
            .collect(),
    }))
}

/// List workspaces for a specific repository.
pub async fn list_repository_workspaces(
    State(state): State<WebAppState>,
    Path(repository_id): Path<Uuid>,
) -> Result<Json<ListWorkspacesResponse>, WebError> {
    let core = state.core().await;

    // First check if repository exists
    let repo_store = core
        .repo_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;

    let repo = repo_store
        .get_by_id(repository_id)
        .map_err(|e| WebError::Internal(format!("Failed to get repository: {}", e)))?
        .ok_or_else(|| WebError::NotFound(format!("Repository {} not found", repository_id)))?;

    let workspace_store = core
        .workspace_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;

    if repo.workspace_mode.is_none() {
        let total = workspace_store
            .count_all_by_repository(repository_id)
            .map_err(|e| WebError::Internal(format!("Failed to check workspaces: {}", e)))?;
        if total == 0 {
            return Err(WebError::Conflict("workspace_mode_required".to_string()));
        }
    }

    let workspaces = workspace_store
        .get_by_repository(repository_id)
        .map_err(|e| WebError::Internal(format!("Failed to list workspaces: {}", e)))?;

    Ok(Json(ListWorkspacesResponse {
        workspaces: workspaces
            .into_iter()
            .map(WorkspaceResponse::from)
            .collect(),
    }))
}

/// Get a single workspace by ID.
pub async fn get_workspace(
    State(state): State<WebAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkspaceResponse>, WebError> {
    let core = state.core().await;
    let store = core
        .workspace_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;

    let workspace = store
        .get_by_id(id)
        .map_err(|e| WebError::Internal(format!("Failed to get workspace: {}", e)))?
        .ok_or_else(|| WebError::NotFound(format!("Workspace {} not found", id)))?;

    Ok(Json(WorkspaceResponse::from(workspace)))
}

/// Create a new workspace for a repository.
pub async fn create_workspace(
    State(state): State<WebAppState>,
    Path(repository_id): Path<Uuid>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Result<(StatusCode, Json<WorkspaceResponse>), WebError> {
    // Validate request
    if req.name.is_empty() {
        return Err(WebError::BadRequest(
            "Workspace name is required".to_string(),
        ));
    }

    if req.branch.is_empty() {
        return Err(WebError::BadRequest("Branch is required".to_string()));
    }

    if req.path.is_empty() {
        return Err(WebError::BadRequest("Path is required".to_string()));
    }

    let core = state.core().await;

    // Check if repository exists
    let repo_store = core
        .repo_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;
    let workspace_store = core
        .workspace_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;

    let _repo = repo_store
        .get_by_id(repository_id)
        .map_err(|e| WebError::Internal(format!("Failed to get repository: {}", e)))?
        .ok_or_else(|| WebError::NotFound(format!("Repository {} not found", repository_id)))?;

    // Create workspace model
    let workspace = if req.is_default {
        Workspace::new_default(
            repository_id,
            &req.name,
            &req.branch,
            PathBuf::from(&req.path),
        )
    } else {
        Workspace::new(
            repository_id,
            &req.name,
            &req.branch,
            PathBuf::from(&req.path),
        )
    };

    // Save to database
    workspace_store
        .create(&workspace)
        .map_err(|e| WebError::Internal(format!("Failed to create workspace: {}", e)))?;

    let response = WorkspaceResponse::from(workspace.clone());
    state
        .status_manager()
        .register_workspace(workspace.id, workspace.path.clone());
    state.status_manager().refresh_workspace(workspace.id);

    Ok((StatusCode::CREATED, Json(response)))
}

/// Delete a workspace.
pub async fn delete_workspace(
    State(state): State<WebAppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, WebError> {
    let core = state.core().await;
    let store = core
        .workspace_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;

    // Check if workspace exists
    let _workspace = store
        .get_by_id(id)
        .map_err(|e| WebError::Internal(format!("Failed to get workspace: {}", e)))?
        .ok_or_else(|| WebError::NotFound(format!("Workspace {} not found", id)))?;

    // Delete workspace
    store
        .delete(id)
        .map_err(|e| WebError::Internal(format!("Failed to delete workspace: {}", e)))?;

    state.status_manager().remove_workspace(id);

    Ok(StatusCode::NO_CONTENT)
}

/// Auto-create a workspace with generated name/branch.
///
/// This endpoint mirrors the TUI's workspace creation flow:
/// 1. Generates a unique workspace name (adjective-noun)
/// 2. Generates a branch name (username/workspace-name)
/// 3. Creates a git worktree
/// 4. Saves the workspace to the database
pub async fn auto_create_workspace(
    State(state): State<WebAppState>,
    Path(repository_id): Path<Uuid>,
) -> Result<(StatusCode, Json<WorkspaceResponse>), WebError> {
    // Get write access to core for worktree operations
    let core = state.core_mut().await;

    // Load repository
    let repo_store = core
        .repo_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;

    let repo = repo_store
        .get_by_id(repository_id)
        .map_err(|e| WebError::Internal(format!("Failed to get repository: {}", e)))?
        .ok_or_else(|| WebError::NotFound(format!("Repository {} not found", repository_id)))?;

    // Get existing workspace names (including archived) to avoid conflicts
    let workspace_store = core
        .workspace_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;

    let existing_names = workspace_store
        .get_all_names_by_repository(repository_id)
        .map_err(|e| WebError::Internal(format!("Failed to get workspace names: {}", e)))?;

    if repo.workspace_mode.is_none() && existing_names.is_empty() {
        return Err(WebError::Conflict("workspace_mode_required".to_string()));
    }

    let settings = resolve_repo_workspace_settings(core.config(), &repo);

    // Generate unique workspace name
    let workspace_name = generate_workspace_name(&existing_names);

    // Generate branch name (username/workspace-name)
    let username = get_git_username();
    let branch_name = generate_branch_name(&username, &workspace_name);

    // Get repository path
    let repo_path = repo
        .base_path
        .as_ref()
        .map(PathBuf::from)
        .ok_or_else(|| WebError::BadRequest("Repository has no base path".to_string()))?;

    // Sync the base repo's remote-tracking refs first so the workspace branches
    // from the freshest origin/<default>.
    conduit_git::sync_remote(&repo_path);

    // Create workspace checkout or worktree
    let worktree_manager = core.worktree_manager();
    let worktree_path = worktree_manager
        .create_workspace(
            settings.mode,
            &repo_path,
            &branch_name,
            &workspace_name,
            |_| {},
        )
        .map_err(|e| WebError::Internal(format!("Failed to create workspace: {}", e)))?;

    // Create workspace model
    let workspace = Workspace::new(repository_id, &workspace_name, &branch_name, worktree_path);

    // Save to database
    workspace_store.create(&workspace).map_err(|e| {
        // If database save fails, try to clean up the worktree
        if let Err(err) =
            core.worktree_manager()
                .remove_workspace(settings.mode, &repo_path, &workspace.path)
        {
            tracing::warn!(
                error = %err,
                repo_path = %repo_path.display(),
                workspace_path = %workspace.path.display(),
                "Failed to remove workspace after workspace save failure"
            );
        }
        WebError::Internal(format!("Failed to save workspace: {}", e))
    })?;

    run_workspace_setup_script(&repo_path, &workspace.path, || {});

    let response = WorkspaceResponse::from(workspace.clone());
    state
        .status_manager()
        .register_workspace(workspace.id, workspace.path.clone());
    state.status_manager().refresh_workspace(workspace.id);

    Ok((StatusCode::CREATED, Json(response)))
}

/// NDJSON event emitted by `auto_create_workspace_stream`.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WorkspaceCreationEvent {
    Progress { message: String },
    Done { workspace: WorkspaceResponse },
    Error { message: String },
}

impl WorkspaceCreationEvent {
    fn to_ndjson_line(&self) -> String {
        let mut s = serde_json::to_string(self).unwrap_or_default();
        s.push('\n');
        s
    }
}

/// Auto-create a workspace and stream progress as NDJSON.
///
/// Emits `{"type":"progress","message":"..."}` lines while working, then a final
/// `{"type":"done","workspace":{...}}` or `{"type":"error","message":"..."}` line.
pub async fn auto_create_workspace_stream(
    State(state): State<WebAppState>,
    Path(repository_id): Path<Uuid>,
) -> Result<Response, WebError> {
    // Phase 1: fast DB lookups — do everything before releasing the lock.
    let (worktree_manager, settings, repo_path, workspace_name, branch_name) = {
        let core = state.core_mut().await;

        let repo_store = core
            .repo_store()
            .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;
        let workspace_store = core
            .workspace_store()
            .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;

        let repo = repo_store
            .get_by_id(repository_id)
            .map_err(|e| WebError::Internal(format!("Failed to get repository: {}", e)))?
            .ok_or_else(|| WebError::NotFound(format!("Repository {} not found", repository_id)))?;

        let existing_names = workspace_store
            .get_all_names_by_repository(repository_id)
            .map_err(|e| WebError::Internal(format!("Failed to get workspace names: {}", e)))?;

        if repo.workspace_mode.is_none() && existing_names.is_empty() {
            return Err(WebError::Conflict("workspace_mode_required".to_string()));
        }

        let settings = resolve_repo_workspace_settings(core.config(), &repo);
        let repo_path = repo
            .base_path
            .as_ref()
            .map(PathBuf::from)
            .ok_or_else(|| WebError::BadRequest("Repository has no base path".to_string()))?;

        let worktree_manager = core.worktree_manager().clone();
        let workspace_name = generate_workspace_name(&existing_names);
        let username = get_git_username();
        let branch_name = generate_branch_name(&username, &workspace_name);

        (
            worktree_manager,
            settings,
            repo_path,
            workspace_name,
            branch_name,
        )
    }; // Core lock released here.

    let (tx, rx) = mpsc::channel::<String>(64);

    tokio::spawn({
        let tx = tx.clone();
        let state = state.clone();
        let repo_path = repo_path.clone();
        let workspace_name = workspace_name.clone();
        let branch_name = branch_name.clone();

        async move {
            let mode = settings.mode;
            let wm_cleanup = worktree_manager.clone();

            // Phase 2: blocking git operations.
            let creation_result = tokio::task::spawn_blocking({
                let tx = tx.clone();
                let repo_path = repo_path.clone();
                let workspace_name = workspace_name.clone();
                let branch_name = branch_name.clone();

                move || {
                    let progress = move |msg: &str| {
                        let event = WorkspaceCreationEvent::Progress {
                            message: msg.to_string(),
                        };
                        let _ = tx.blocking_send(event.to_ndjson_line());
                    };
                    progress("Syncing with remote...");
                    conduit_git::sync_remote(&repo_path);
                    worktree_manager.create_workspace(
                        mode,
                        &repo_path,
                        &branch_name,
                        &workspace_name,
                        progress,
                    )
                }
            })
            .await;

            match creation_result {
                Ok(Ok(worktree_path)) => {
                    let _ = tx
                        .send(
                            WorkspaceCreationEvent::Progress {
                                message: "Running workspace setup...".to_string(),
                            }
                            .to_ndjson_line(),
                        )
                        .await;

                    let _ = tokio::task::spawn_blocking({
                        let repo_path = repo_path.clone();
                        let worktree_path = worktree_path.clone();
                        move || run_workspace_setup_script(&repo_path, &worktree_path, || {})
                    })
                    .await;

                    let workspace = conduit_data::Workspace::new(
                        repository_id,
                        &workspace_name,
                        &branch_name,
                        worktree_path.clone(),
                    );

                    let save_result = {
                        let core = state.core_mut().await;
                        core.workspace_store()
                            .ok_or_else(|| "Database not available".to_string())
                            .and_then(|store| store.create(&workspace).map_err(|e| e.to_string()))
                    }; // Lock released.

                    match save_result {
                        Ok(()) => {
                            state
                                .status_manager()
                                .register_workspace(workspace.id, workspace.path.clone());
                            state.status_manager().refresh_workspace(workspace.id);

                            let event = WorkspaceCreationEvent::Done {
                                workspace: WorkspaceResponse::from(workspace),
                            };
                            let _ = tx.send(event.to_ndjson_line()).await;
                        }
                        Err(e) => {
                            let core = state.core_mut().await;
                            if let Err(err) =
                                wm_cleanup.remove_workspace(mode, &repo_path, &worktree_path)
                            {
                                tracing::warn!(
                                    error = %err,
                                    "Failed to clean up worktree after DB save failure"
                                );
                            }
                            drop(core);
                            let _ = tx
                                .send(
                                    WorkspaceCreationEvent::Error {
                                        message: format!("Failed to save workspace: {}", e),
                                    }
                                    .to_ndjson_line(),
                                )
                                .await;
                        }
                    }
                }
                Ok(Err(e)) => {
                    let _ = tx
                        .send(
                            WorkspaceCreationEvent::Error {
                                message: e.to_string(),
                            }
                            .to_ndjson_line(),
                        )
                        .await;
                }
                Err(e) => {
                    let _ = tx
                        .send(
                            WorkspaceCreationEvent::Error {
                                message: e.to_string(),
                            }
                            .to_ndjson_line(),
                        )
                        .await;
                }
            }
        }
    });

    let byte_stream = stream::unfold(rx, |mut rx| async move {
        rx.recv()
            .await
            .map(|line| (Ok::<Bytes, std::io::Error>(Bytes::from(line)), rx))
    });

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-ndjson")
        .header("Cache-Control", "no-cache")
        .header("X-Accel-Buffering", "no")
        .body(Body::from_stream(byte_stream))
        .unwrap())
}

/// Get workspace git status and PR info.
pub async fn get_workspace_status(
    State(state): State<WebAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkspaceStatusResponse>, WebError> {
    let core = state.core().await;
    let store = core
        .workspace_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;

    // Get the workspace
    let workspace = store
        .get_by_id(id)
        .map_err(|e| WebError::Internal(format!("Failed to get workspace: {}", e)))?
        .ok_or_else(|| WebError::NotFound(format!("Workspace {} not found", id)))?;

    state
        .status_manager()
        .register_workspace(workspace.id, workspace.path.clone());

    Ok(Json(
        state
            .status_manager()
            .get_status(workspace.id)
            .unwrap_or_default(),
    ))
}

/// Run PR preflight checks for a workspace.
pub async fn get_workspace_pr_preflight(
    State(state): State<WebAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<PrPreflightResponse>, WebError> {
    let core = state.core().await;
    let store = core
        .workspace_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;

    let workspace = store
        .get_by_id(id)
        .map_err(|e| WebError::Internal(format!("Failed to get workspace: {}", e)))?
        .ok_or_else(|| WebError::NotFound(format!("Workspace {} not found", id)))?;

    let preflight = PrManager::preflight_check(&workspace.path);
    Ok(Json(build_pr_preflight_response(preflight)))
}

/// Create a PR prompt for a workspace after preflight checks.
pub async fn create_workspace_pr(
    State(state): State<WebAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<PrCreateResponse>, WebError> {
    let core = state.core().await;
    let store = core
        .workspace_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;

    let workspace = store
        .get_by_id(id)
        .map_err(|e| WebError::Internal(format!("Failed to get workspace: {}", e)))?
        .ok_or_else(|| WebError::NotFound(format!("Workspace {} not found", id)))?;

    let preflight = PrManager::preflight_check(&workspace.path);
    let prompt = PrManager::generate_pr_prompt(&preflight);

    Ok(Json(PrCreateResponse {
        preflight: build_pr_preflight_response(preflight),
        prompt,
    }))
}

/// Get or create a session for a workspace.
///
/// This endpoint returns the existing session for a workspace if one exists,
/// or creates a new session with the default agent (Claude) if none exists.
/// This mirrors the TUI behavior where opening a workspace automatically
/// creates/restores a session.
pub async fn get_or_create_session(
    State(state): State<WebAppState>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<SessionResponse>, WebError> {
    let core = state.core().await;
    let session = SessionService::get_or_create_session_for_workspace(&core, workspace_id)
        .map_err(map_service_error)?;

    Ok(Json(SessionResponse::from(session)))
}

fn map_service_error(error: ServiceError) -> WebError {
    match error {
        ServiceError::InvalidInput(message) => WebError::BadRequest(message),
        ServiceError::NotFound(message) => WebError::NotFound(message),
        ServiceError::Internal(message) => WebError::Internal(message),
    }
}

fn build_pr_preflight_response(preflight: conduit_git::PrPreflightResult) -> PrPreflightResponse {
    PrPreflightResponse {
        gh_installed: preflight.gh_installed,
        gh_authenticated: preflight.gh_authenticated,
        on_main_branch: preflight.on_main_branch,
        branch_name: preflight.branch_name,
        target_branch: preflight.target_branch,
        uncommitted_count: preflight.uncommitted_count,
        has_upstream: preflight.has_upstream,
        existing_pr: preflight
            .existing_pr
            .as_ref()
            .and_then(PrStatusResponse::from_pr_status),
    }
}

/// Request to read a file within a workspace.
#[derive(Debug, Deserialize)]
pub struct ReadFileRequest {
    pub path: String,
}

/// Response for reading a file.
#[derive(Debug, Serialize)]
pub struct ReadFileResponse {
    pub content: String,
    pub encoding: String,
    pub size: u64,
    pub media_type: String,
    pub exists: bool,
}

/// Read a file from a workspace.
///
/// Security: Only files within the workspace directory are allowed.
pub async fn read_workspace_file(
    State(state): State<WebAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<ReadFileRequest>,
) -> Result<Json<ReadFileResponse>, WebError> {
    let core = state.core().await;
    let store = core
        .workspace_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;

    let workspace = store
        .get_by_id(id)
        .map_err(|e| WebError::Internal(format!("Failed to get workspace: {}", e)))?
        .ok_or_else(|| WebError::NotFound(format!("Workspace {} not found", id)))?;

    let requested_path = PathBuf::from(&req.path);
    let file_path = if requested_path.is_absolute() {
        requested_path
    } else {
        workspace.path.join(&req.path)
    };

    // Security: Ensure the requested path is within the workspace directory
    let workspace_canonical = workspace
        .path
        .canonicalize()
        .map_err(|e| WebError::Internal(format!("Failed to resolve workspace path: {}", e)))?;

    let file_canonical = file_path.canonicalize().map_err(|_| {
        // File doesn't exist - return exists: false
        WebError::NotFound("File not found".to_string())
    });

    let file_canonical = match file_canonical {
        Ok(path) => path,
        Err(_) => {
            return Ok(Json(ReadFileResponse {
                content: String::new(),
                encoding: "utf-8".to_string(),
                size: 0,
                media_type: "text/plain".to_string(),
                exists: false,
            }));
        }
    };

    // Verify file is within workspace
    if !file_canonical.starts_with(&workspace_canonical) {
        return Err(WebError::BadRequest(
            "File path must be within workspace directory".to_string(),
        ));
    }

    // Read file metadata
    let metadata = std::fs::metadata(&file_canonical)
        .map_err(|e| WebError::Internal(format!("Failed to read file metadata: {}", e)))?;

    let size = metadata.len();

    // Determine media type from extension
    let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let media_type = match extension.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        "pdf" => "application/pdf",
        "json" => "application/json",
        "xml" => "application/xml",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "text/javascript",
        "ts" | "tsx" => "text/typescript",
        "md" | "markdown" => "text/markdown",
        "rs" => "text/x-rust",
        "py" => "text/x-python",
        "go" => "text/x-go",
        "yaml" | "yml" => "text/yaml",
        "toml" => "text/toml",
        _ => "text/plain",
    }
    .to_string();

    // Check if binary file
    let is_binary = matches!(
        extension.to_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "bmp" | "pdf"
    );

    let (content, encoding) = if is_binary {
        // Read as base64
        let bytes = std::fs::read(&file_canonical)
            .map_err(|e| WebError::Internal(format!("Failed to read file: {}", e)))?;
        (
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes),
            "base64".to_string(),
        )
    } else {
        // Read as UTF-8
        let text = std::fs::read_to_string(&file_canonical)
            .map_err(|e| WebError::Internal(format!("Failed to read file: {}", e)))?;
        (text, "utf-8".to_string())
    };

    Ok(Json(ReadFileResponse {
        content,
        encoding,
        size,
        media_type,
        exists: true,
    }))
}
