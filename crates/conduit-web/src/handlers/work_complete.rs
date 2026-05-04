//! Work Complete flow handlers — preflight checks and inline action endpoints.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::WebError;
use crate::state::WebAppState;
use conduit_core::resolve_repo_workspace_settings;
use conduit_git::{
    archive_change, classify, close_issue, commit_all, fetch_change_detail, git_diff_files,
    infer_active_change, infer_active_issue, push_branch, view_issue, ArchiveError, ContextSource,
    GitState, IssueSnapshot, MergeMethod, MergeReadiness, PrCreateOpts, PrManager, PrSnapshot,
    PrState, Scenario, SpecSnapshot, SuggestedAction,
};

// ---------- Response types ----------
#[derive(Debug, Serialize)]
pub struct DirtyFileResponse {
    pub status: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct PrSnapshotResponse {
    pub number: u32,
    pub url: Option<String>,
    pub title: Option<String>,
    pub is_open: bool,
    pub is_merged: bool,
    pub merge_readiness: String,
}

#[derive(Debug, Serialize)]
pub struct SpecSnapshotResponse {
    pub change_id: String,
    pub total: usize,
    pub completed: usize,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct IssueSnapshotResponse {
    pub number: i32,
    pub title: Option<String>,
    pub url: Option<String>,
    pub is_open: bool,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct WorkCompletePreflightResponse {
    pub branch_name: String,
    pub base_branch: String,
    pub is_dirty: bool,
    pub dirty_files: Vec<DirtyFileResponse>,
    pub commits_ahead: usize,
    pub commits_behind: usize,
    pub is_merged: bool,
    pub has_upstream: bool,
    pub remote_branch_exists: bool,
    pub pr: Option<PrSnapshotResponse>,
    pub spec: Option<SpecSnapshotResponse>,
    pub issue: Option<IssueSnapshotResponse>,
    pub scenario: Scenario,
    pub suggested_actions: Vec<SuggestedAction>,
}

#[derive(Debug, Serialize)]
pub struct WorkCompleteActionResponse {
    pub status: String,
    pub log_lines: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CommitActionResponse {
    pub status: String,
    pub log_lines: Vec<String>,
    pub sha: String,
}

#[derive(Debug, Serialize)]
pub struct PrCreateActionResponse {
    pub status: String,
    pub log_lines: Vec<String>,
    pub url: String,
    pub number: u32,
}

#[derive(Debug, Serialize)]
pub struct SpecArchiveActionResponse {
    pub status: String,
    pub log_lines: Vec<String>,
    pub new_path: String,
    pub warnings: Vec<String>,
}

// ---------- Request types ----------

#[derive(Debug, Deserialize)]
pub struct CommitRequest {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct PrCreateRequest {
    pub title: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PrMergeRequest {
    #[serde(default)]
    pub method: MergeMethodRequest,
    #[serde(default)]
    pub admin: bool,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MergeMethodRequest {
    #[default]
    Squash,
    Merge,
    Rebase,
}

#[derive(Debug, Deserialize)]
pub struct SpecArchiveRequest {
    pub change_id: String,
}

#[derive(Debug, Deserialize)]
pub struct WorkCompleteArchiveRequest {
    pub delete_remote: Option<bool>,
}

// ---------- Preflight ----------

/// GET /workspaces/{id}/work-complete/preflight
pub async fn get_work_complete_preflight(
    State(state): State<WebAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkCompletePreflightResponse>, WebError> {
    let core = state.core().await;
    let store = core
        .workspace_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;

    let workspace = store
        .get_by_id(id)
        .map_err(|e| WebError::Internal(format!("Failed to get workspace: {}", e)))?
        .ok_or_else(|| WebError::NotFound(format!("Workspace {} not found", id)))?;

    let worktree_manager = core.worktree_manager();
    let use_gh_cli = core.config().workspaces.use_gh_cli_merge_status;

    // Branch status (dirty, merged, ahead/behind)
    let branch_status = worktree_manager
        .get_branch_status_with_gh_option(&workspace.path, use_gh_cli)
        .unwrap_or_default();

    // Dirty file list
    let dirty_files: Vec<DirtyFileResponse> = git_diff_files(&workspace.path)
        .into_iter()
        .map(|f| DirtyFileResponse {
            status: f.status,
            path: f.path,
        })
        .collect();

    // PR preflight — gives us has_upstream, branch_name, existing PR info
    let pr_preflight = PrManager::preflight_check(&workspace.path);

    // --- Spec resolution ---
    let (spec_change_id, spec_source) = if let Some(ref id) = workspace.active_change_id {
        (Some(id.clone()), ContextSource::Linked)
    } else {
        let inferred = infer_active_change(&workspace.path, &pr_preflight.target_branch);
        if let Some(ref cid) = inferred {
            let _ = store.update_active_links(workspace.id, Some(cid.clone()), None);
        }
        (inferred, ContextSource::Detected)
    };

    let spec_snapshot: Option<SpecSnapshotResponse> = spec_change_id.and_then(|change_id| {
        fetch_change_detail(&workspace.path, &change_id).map(|d| SpecSnapshotResponse {
            change_id: d.change_id,
            total: d.total,
            completed: d.completed,
            source: source_label(spec_source),
        })
    });

    // --- Issue resolution ---
    let (issue_number, issue_source) = if let Some(n) = workspace.active_issue_number {
        (Some(n), ContextSource::Linked)
    } else {
        let inferred = infer_active_issue(&workspace.branch);
        if let Some(n) = inferred {
            let _ = store.update_active_links(workspace.id, None, Some(n));
        }
        (inferred, ContextSource::Detected)
    };

    let issue_snapshot: Option<IssueSnapshotResponse> = issue_number.map(|n| {
        let view = view_issue(&workspace.path, n);
        let is_open = view
            .as_ref()
            .map(|v| v.state.to_uppercase() == "OPEN")
            .unwrap_or(true);
        IssueSnapshotResponse {
            number: n,
            title: view.as_ref().map(|v| v.title.clone()),
            url: view.as_ref().map(|v| v.url.clone()),
            is_open,
            source: source_label(issue_source),
        }
    });

    // --- PR snapshot ---
    let pr_snapshot: Option<PrSnapshotResponse> = pr_preflight
        .existing_pr
        .as_ref()
        .filter(|p| p.exists)
        .map(|pr| PrSnapshotResponse {
            number: pr.number.unwrap_or(0),
            url: pr.url.clone(),
            title: pr.title.clone(),
            is_open: pr.state == PrState::Open || pr.state == PrState::Draft,
            is_merged: pr.state == PrState::Merged,
            merge_readiness: merge_readiness_label(&pr.merge_readiness),
        });

    // --- Classifier ---
    let git_state = GitState {
        is_dirty: branch_status.is_dirty,
        commits_ahead: branch_status.commits_ahead as u32,
        commits_behind: branch_status.commits_behind as u32,
        is_merged: branch_status.is_merged,
        has_upstream: pr_preflight.has_upstream,
    };

    let pr_classify = pr_snapshot.as_ref().map(|pr| PrSnapshot {
        number: pr.number,
        is_open: pr.is_open,
        is_merged: pr.is_merged,
        merge_readiness: label_to_merge_readiness(&pr.merge_readiness),
    });

    let spec_classify = spec_snapshot.as_ref().map(|s| SpecSnapshot {
        change_id: s.change_id.clone(),
        total: s.total,
        completed: s.completed,
        source: label_to_context_source(&s.source),
    });

    let issue_classify = issue_snapshot.as_ref().map(|i| IssueSnapshot {
        number: i.number,
        is_open: i.is_open,
        source: label_to_context_source(&i.source),
    });

    let (scenario, suggested_actions) = classify(
        &git_state,
        pr_classify.as_ref(),
        spec_classify.as_ref(),
        issue_classify.as_ref(),
    );

    let remote_branch_exists = pr_preflight.has_upstream || {
        // Check local remote-tracking ref without a network call
        let refspec = format!("refs/remotes/origin/{}", pr_preflight.branch_name);
        std::process::Command::new("git")
            .args(["rev-parse", "--verify", &refspec])
            .current_dir(&workspace.path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };

    Ok(Json(WorkCompletePreflightResponse {
        branch_name: pr_preflight.branch_name,
        base_branch: pr_preflight.target_branch,
        is_dirty: branch_status.is_dirty,
        dirty_files,
        commits_ahead: branch_status.commits_ahead,
        commits_behind: branch_status.commits_behind,
        is_merged: branch_status.is_merged,
        has_upstream: pr_preflight.has_upstream,
        remote_branch_exists,
        pr: pr_snapshot,
        spec: spec_snapshot,
        issue: issue_snapshot,
        scenario,
        suggested_actions,
    }))
}

// ---------- Action: commit ----------

/// POST /workspaces/{id}/work-complete/commit
pub async fn post_work_complete_commit(
    State(state): State<WebAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<CommitRequest>,
) -> Result<Json<CommitActionResponse>, WebError> {
    if req.message.trim().is_empty() {
        return Err(WebError::BadRequest(
            "Commit message must not be empty".to_string(),
        ));
    }

    let path = workspace_path(&state, id).await?;
    let sha = commit_all(&path, &req.message)
        .map_err(|e| WebError::Internal(format!("git commit failed: {}", e)))?;

    Ok(Json(CommitActionResponse {
        status: "ok".to_string(),
        log_lines: vec![format!("Committed: {}", sha)],
        sha,
    }))
}

// ---------- Action: push ----------

/// POST /workspaces/{id}/work-complete/push
pub async fn post_work_complete_push(
    State(state): State<WebAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkCompleteActionResponse>, WebError> {
    let (path, branch, has_upstream) = workspace_push_info(&state, id).await?;
    push_branch(&path, &branch, !has_upstream)
        .map_err(|e| WebError::Internal(format!("git push failed: {}", e)))?;

    Ok(Json(WorkCompleteActionResponse {
        status: "ok".to_string(),
        log_lines: vec![format!("Pushed branch: {}", branch)],
    }))
}

// ---------- Action: open PR ----------

/// POST /workspaces/{id}/work-complete/pr
pub async fn post_work_complete_pr(
    State(state): State<WebAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<PrCreateRequest>,
) -> Result<Json<PrCreateActionResponse>, WebError> {
    let path = workspace_path(&state, id).await?;
    let preflight = PrManager::preflight_check(&path);
    let opts = PrCreateOpts {
        base_branch: preflight.target_branch,
        title: req.title,
        body: req.body,
    };
    let pr_info = PrManager::create(&path, &opts)
        .map_err(|e| WebError::Internal(format!("gh pr create failed: {}", e)))?;

    Ok(Json(PrCreateActionResponse {
        status: "ok".to_string(),
        log_lines: vec![format!("Created PR #{}: {}", pr_info.number, pr_info.url)],
        url: pr_info.url,
        number: pr_info.number,
    }))
}

// ---------- Action: merge PR ----------

/// POST /workspaces/{id}/work-complete/pr/merge
pub async fn post_work_complete_pr_merge(
    State(state): State<WebAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<PrMergeRequest>,
) -> Result<Json<WorkCompleteActionResponse>, WebError> {
    let path = workspace_path(&state, id).await?;

    // Check merge readiness unless admin override
    if !req.admin {
        let preflight = PrManager::preflight_check(&path);
        if let Some(pr) = &preflight.existing_pr {
            if !matches!(pr.merge_readiness, MergeReadiness::Ready) {
                return Err(WebError::Conflict(format!(
                    "PR is not ready to merge ({})",
                    merge_readiness_label(&pr.merge_readiness)
                )));
            }
        }
    }

    let method = match req.method {
        MergeMethodRequest::Squash => MergeMethod::Squash,
        MergeMethodRequest::Merge => MergeMethod::Merge,
        MergeMethodRequest::Rebase => MergeMethod::Rebase,
    };

    PrManager::merge(&path, method, req.admin)
        .map_err(|e| WebError::Internal(format!("gh pr merge failed: {}", e)))?;

    Ok(Json(WorkCompleteActionResponse {
        status: "ok".to_string(),
        log_lines: vec!["PR merged".to_string()],
    }))
}

// ---------- Action: close issue ----------

/// POST /workspaces/{id}/work-complete/issue/close
pub async fn post_work_complete_issue_close(
    State(state): State<WebAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<WorkCompleteActionResponse>, WebError> {
    let core = state.core().await;
    let store = core
        .workspace_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;
    let workspace = store
        .get_by_id(id)
        .map_err(|e| WebError::Internal(format!("Failed to get workspace: {}", e)))?
        .ok_or_else(|| WebError::NotFound(format!("Workspace {} not found", id)))?;

    let issue_number = workspace
        .active_issue_number
        .or_else(|| infer_active_issue(&workspace.branch));

    let number = issue_number.ok_or_else(|| {
        WebError::BadRequest("No linked issue found for this workspace".to_string())
    })?;

    close_issue(&workspace.path, number)
        .map_err(|e| WebError::Internal(format!("gh issue close failed: {}", e)))?;

    Ok(Json(WorkCompleteActionResponse {
        status: "ok".to_string(),
        log_lines: vec![format!("Closed issue #{}", number)],
    }))
}

// ---------- Action: archive spec ----------

/// POST /workspaces/{id}/work-complete/spec/archive
pub async fn post_work_complete_spec_archive(
    State(state): State<WebAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<SpecArchiveRequest>,
) -> Result<Json<SpecArchiveActionResponse>, WebError> {
    let path = workspace_path(&state, id).await?;

    let today: NaiveDate = chrono::Local::now().date_naive();
    let result = archive_change(&path, &req.change_id, today).map_err(|e| match e {
        ArchiveError::SourceNotFound(p) => {
            WebError::NotFound(format!("Change directory not found: {}", p.display()))
        }
        ArchiveError::TargetExists(p) => {
            WebError::Conflict(format!("Archive target already exists: {}", p.display()))
        }
        ArchiveError::Rename(io_err) => {
            WebError::Internal(format!("Failed to rename change directory: {}", io_err))
        }
    })?;

    let new_path = result.new_path.to_string_lossy().to_string();

    Ok(Json(SpecArchiveActionResponse {
        status: "ok".to_string(),
        log_lines: vec![format!("Archived spec to {}", new_path)],
        new_path,
        warnings: result.warnings,
    }))
}

// ---------- Action: archive workspace ----------

/// POST /workspaces/{id}/work-complete/archive
///
/// Equivalent to the legacy `POST /workspaces/{id}/archive` endpoint.
pub async fn post_work_complete_archive(
    State(state): State<WebAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<WorkCompleteArchiveRequest>,
) -> Result<StatusCode, WebError> {
    let core = state.core().await;
    let workspace_store = core
        .workspace_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;
    let repo_store = core
        .repo_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;
    let session_store = core
        .session_tab_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;

    let workspace = workspace_store
        .get_by_id(id)
        .map_err(|e| WebError::Internal(format!("Failed to get workspace: {}", e)))?
        .ok_or_else(|| WebError::NotFound(format!("Workspace {} not found", id)))?;

    let repo = repo_store
        .get_by_id(workspace.repository_id)
        .map_err(|e| WebError::Internal(format!("Failed to get repository: {}", e)))?
        .ok_or_else(|| {
            WebError::NotFound(format!("Repository {} not found", workspace.repository_id))
        })?;

    let settings = resolve_repo_workspace_settings(core.config(), &repo);
    let delete_remote = settings.archive_delete_branch && req.delete_remote.unwrap_or(false);
    let worktree_manager = core.worktree_manager();
    let mut warnings = Vec::new();
    let mut archived_commit_sha = None;

    if let Err(err) = state.session_manager().stop_workspace_sessions(id).await {
        warnings.push(format!("Failed to stop active sessions: {}", err));
    }

    if let Some(base_path) = repo.base_path {
        match worktree_manager.get_branch_sha(
            settings.mode,
            &base_path,
            &workspace.path,
            &workspace.branch,
        ) {
            Ok(sha) => archived_commit_sha = Some(sha),
            Err(err) => warnings.push(format!("Failed to read branch SHA: {}", err)),
        }

        if let Err(err) =
            worktree_manager.remove_workspace(settings.mode, &base_path, &workspace.path)
        {
            warnings.push(format!("Failed to remove worktree: {}", err));
        }

        if settings.archive_delete_branch {
            if let Err(err) = worktree_manager.delete_branch(
                settings.mode,
                &base_path,
                &workspace.path,
                &workspace.branch,
            ) {
                warnings.push(format!(
                    "Failed to delete branch '{}': {}",
                    workspace.branch, err
                ));
            }
        }

        if delete_remote {
            if let Err(err) = worktree_manager.delete_remote_branch(&base_path, &workspace.branch) {
                warnings.push(format!(
                    "Failed to delete remote branch '{}': {}",
                    workspace.branch, err
                ));
            }
        }
    } else {
        warnings.push("Repository has no base path; worktree not removed".to_string());
    }

    workspace_store
        .archive(id, archived_commit_sha)
        .map_err(|e| WebError::Internal(format!("Failed to archive workspace: {}", e)))?;

    if let Err(e) = session_store.set_open_by_workspace(id, false) {
        tracing::warn!(error = %e, "Failed to close sessions for archived workspace");
    }

    state.status_manager().remove_workspace(id);

    if !warnings.is_empty() {
        tracing::warn!(
            workspace_id = %id,
            warnings = ?warnings,
            "Work-complete archive finished with warnings"
        );
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---------- Helpers ----------

async fn workspace_path(state: &WebAppState, id: Uuid) -> Result<std::path::PathBuf, WebError> {
    let core = state.core().await;
    let store = core
        .workspace_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;
    let ws = store
        .get_by_id(id)
        .map_err(|e| WebError::Internal(format!("Failed to get workspace: {}", e)))?
        .ok_or_else(|| WebError::NotFound(format!("Workspace {} not found", id)))?;
    Ok(ws.path)
}

async fn workspace_push_info(
    state: &WebAppState,
    id: Uuid,
) -> Result<(std::path::PathBuf, String, bool), WebError> {
    let core = state.core().await;
    let store = core
        .workspace_store()
        .ok_or_else(|| WebError::Internal("Database not available".to_string()))?;
    let ws = store
        .get_by_id(id)
        .map_err(|e| WebError::Internal(format!("Failed to get workspace: {}", e)))?
        .ok_or_else(|| WebError::NotFound(format!("Workspace {} not found", id)))?;
    let preflight = PrManager::preflight_check(&ws.path);
    Ok((ws.path, ws.branch, preflight.has_upstream))
}

fn source_label(source: ContextSource) -> String {
    match source {
        ContextSource::Linked => "linked".to_string(),
        ContextSource::Detected => "detected".to_string(),
    }
}

fn merge_readiness_label(r: &MergeReadiness) -> String {
    match r {
        MergeReadiness::Ready => "ready".to_string(),
        MergeReadiness::Blocked => "blocked".to_string(),
        MergeReadiness::HasConflicts => "has_conflicts".to_string(),
        MergeReadiness::Unknown => "unknown".to_string(),
    }
}

fn label_to_merge_readiness(s: &str) -> MergeReadiness {
    match s {
        "ready" => MergeReadiness::Ready,
        "blocked" => MergeReadiness::Blocked,
        "has_conflicts" => MergeReadiness::HasConflicts,
        _ => MergeReadiness::Unknown,
    }
}

fn label_to_context_source(s: &str) -> ContextSource {
    match s {
        "linked" => ContextSource::Linked,
        _ => ContextSource::Detected,
    }
}
