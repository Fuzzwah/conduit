//! Scenario classifier for the Work Complete flow.
//!
//! All logic here is pure: no I/O, no side effects. The HTTP handler collects
//! inputs from the various git/gh services and passes them in; unit tests do
//! the same from constructed values.

/// Source of spec/issue linkage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSource {
    /// Stored in the workspace row at creation time.
    Linked,
    /// Inferred at preflight time from git history or branch name.
    Detected,
}

/// Classified scenario — the dominant state of the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scenario {
    /// Worktree is clean, nothing pending — ready to archive.
    CleanReady,
    /// Uncommitted edits, no spec/issue linked.
    EditsNoLink,
    /// Commits ahead of remote but worktree is clean, no spec/issue linked.
    UnpushedCommits,
    /// Linked spec has all tasks checked.
    SpecComplete,
    /// Linked spec has at least one unchecked task.
    SpecIncomplete,
    /// Linked issue is still open.
    IssueOpen,
    /// Linked issue is already closed.
    IssueClosed,
}

/// Suggested action the dialog can execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestedAction {
    Commit,
    Push,
    OpenPr,
    MergePr,
    CloseIssue,
    ArchiveSpec,
    Archive,
    /// Open the spec's task list in the active agent session (TUI/web only; never executed server-side).
    ShowRemainingTasks,
    /// Trigger an adversarial code review in the active agent session (TUI/web only; never executed server-side).
    AdversarialReview,
}

/// Git state snapshot passed to the classifier.
#[derive(Debug, Clone)]
pub struct GitState {
    pub is_dirty: bool,
    pub commits_ahead: u32,
    pub commits_behind: u32,
    pub is_merged: bool,
    pub has_upstream: bool,
}

/// PR snapshot passed to the classifier.
#[derive(Debug, Clone)]
pub struct PrSnapshot {
    pub number: u32,
    /// True when the PR is open (not merged, not closed).
    pub is_open: bool,
    /// True when the PR has been merged.
    pub is_merged: bool,
    pub merge_readiness: crate::MergeReadiness,
}

/// OpenSpec change snapshot passed to the classifier.
#[derive(Debug, Clone)]
pub struct SpecSnapshot {
    pub change_id: String,
    pub total: usize,
    pub completed: usize,
    pub source: ContextSource,
}

impl SpecSnapshot {
    pub fn is_complete(&self) -> bool {
        self.total > 0 && self.completed == self.total
    }
}

/// GitHub issue snapshot passed to the classifier.
#[derive(Debug, Clone)]
pub struct IssueSnapshot {
    pub number: i32,
    pub is_open: bool,
    pub source: ContextSource,
}

/// Classify the workspace situation and return an ordered list of suggested actions.
pub fn classify(
    git: &GitState,
    pr: Option<&PrSnapshot>,
    spec: Option<&SpecSnapshot>,
    issue: Option<&IssueSnapshot>,
    adversarial_review_enabled: bool,
) -> (Scenario, Vec<SuggestedAction>) {
    let pr_open = pr.map(|p| p.is_open).unwrap_or(false);
    let pr_merged = pr.map(|p| p.is_merged).unwrap_or(false) || git.is_merged;

    // --- Scenario determination (priority order) ---

    let scenario = if let Some(spec) = spec {
        if !spec.is_complete() {
            Scenario::SpecIncomplete
        } else if let Some(issue) = issue {
            if issue.is_open {
                Scenario::IssueOpen
            } else {
                Scenario::SpecComplete
            }
        } else {
            Scenario::SpecComplete
        }
    } else if let Some(issue) = issue {
        if issue.is_open {
            Scenario::IssueOpen
        } else {
            Scenario::IssueClosed
        }
    } else if git.is_dirty {
        Scenario::EditsNoLink
    } else if git.commits_ahead > 0 && !pr_merged && !pr_open {
        // commits_ahead after a squash-merge are artifacts; treat as clean if PR is done.
        // When a PR is open, commits_ahead measures vs the default branch (not the tracking
        // ref), so it's always > 0 for a feature branch — don't call that "unpushed".
        Scenario::UnpushedCommits
    } else {
        Scenario::CleanReady
    };

    // --- Action list (ordered: commit → push → open_pr → merge → close_issue → archive_spec → archive) ---

    let mut actions: Vec<SuggestedAction> = Vec::new();

    if git.is_dirty {
        actions.push(SuggestedAction::Commit);
    }

    // Suppress Push when an open PR already has an upstream tracking branch and the
    // working tree is clean: commits_ahead measures vs the default branch, not vs the
    // tracking ref, so it stays > 0 for all feature branches even after pushing.
    if !pr_merged
        && (git.commits_ahead > 0 || (git.is_dirty && !git.has_upstream))
        && !(pr_open && git.has_upstream && !git.is_dirty)
    {
        actions.push(SuggestedAction::Push);
    }

    if !pr_open && !pr_merged && git.commits_ahead > 0 {
        actions.push(SuggestedAction::OpenPr);
    }

    if pr_open && !pr_merged {
        actions.push(SuggestedAction::MergePr);
    }

    // Suggest close_issue only when there's no open PR to auto-close it on merge.
    if let Some(issue) = issue {
        if issue.is_open && !pr_open {
            actions.push(SuggestedAction::CloseIssue);
        }
    }

    if let Some(spec) = spec {
        if spec.is_complete() {
            actions.push(SuggestedAction::ArchiveSpec);
        } else if spec.total > 0 {
            actions.push(SuggestedAction::ShowRemainingTasks);
        }
    }

    if adversarial_review_enabled && (git.is_dirty || git.commits_ahead > 0) {
        actions.push(SuggestedAction::AdversarialReview);
    }

    // Always offer archive at the end (except when there's still an open PR or uncommitted work
    // that hasn't been addressed — but we include it so the user can force-archive if needed).
    actions.push(SuggestedAction::Archive);

    (scenario, actions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MergeReadiness;

    fn git(is_dirty: bool, commits_ahead: u32, is_merged: bool, has_upstream: bool) -> GitState {
        GitState {
            is_dirty,
            commits_ahead,
            commits_behind: 0,
            is_merged,
            has_upstream,
        }
    }

    fn pr_open(number: u32) -> PrSnapshot {
        PrSnapshot {
            number,
            is_open: true,
            is_merged: false,
            merge_readiness: MergeReadiness::Ready,
        }
    }

    fn pr_merged() -> PrSnapshot {
        PrSnapshot {
            number: 1,
            is_open: false,
            is_merged: true,
            merge_readiness: MergeReadiness::Unknown,
        }
    }

    fn pr_closed_unmerged() -> PrSnapshot {
        PrSnapshot {
            number: 1,
            is_open: false,
            is_merged: false,
            merge_readiness: MergeReadiness::Unknown,
        }
    }

    fn spec_complete(id: &str) -> SpecSnapshot {
        SpecSnapshot {
            change_id: id.to_string(),
            total: 10,
            completed: 10,
            source: ContextSource::Linked,
        }
    }

    fn spec_incomplete(id: &str) -> SpecSnapshot {
        SpecSnapshot {
            change_id: id.to_string(),
            total: 10,
            completed: 7,
            source: ContextSource::Linked,
        }
    }

    fn issue_open(n: i32) -> IssueSnapshot {
        IssueSnapshot {
            number: n,
            is_open: true,
            source: ContextSource::Linked,
        }
    }

    fn issue_closed(n: i32) -> IssueSnapshot {
        IssueSnapshot {
            number: n,
            is_open: false,
            source: ContextSource::Linked,
        }
    }

    // --- Scenario classification ---

    #[test]
    fn clean_no_links_is_clean_ready() {
        let (scenario, actions) = classify(&git(false, 0, false, true), None, None, None, false);
        assert_eq!(scenario, Scenario::CleanReady);
        assert_eq!(actions, vec![SuggestedAction::Archive]);
    }

    #[test]
    fn dirty_no_links_is_edits_no_link() {
        let (scenario, _) = classify(&git(true, 0, false, true), None, None, None, false);
        assert_eq!(scenario, Scenario::EditsNoLink);
    }

    #[test]
    fn commits_ahead_no_links_is_unpushed_commits() {
        let (scenario, _) = classify(&git(false, 3, false, true), None, None, None, false);
        assert_eq!(scenario, Scenario::UnpushedCommits);
    }

    #[test]
    fn dirty_and_commits_ahead_is_edits_no_link() {
        let (scenario, _) = classify(&git(true, 3, false, true), None, None, None, false);
        assert_eq!(scenario, Scenario::EditsNoLink);
    }

    #[test]
    fn merged_pr_with_commits_ahead_is_clean_ready_and_suppresses_push() {
        let pr = PrSnapshot {
            number: 1,
            is_open: false,
            is_merged: true,
            merge_readiness: crate::MergeReadiness::Ready,
        };
        let (scenario, actions) =
            classify(&git(false, 2, false, true), Some(&pr), None, None, false);
        assert_eq!(scenario, Scenario::CleanReady);
        assert!(!actions.contains(&SuggestedAction::Push));
        assert!(actions.contains(&SuggestedAction::Archive));
    }

    #[test]
    fn spec_complete_scenario() {
        let s = spec_complete("feat");
        let (scenario, _) = classify(&git(false, 0, false, true), None, Some(&s), None, false);
        assert_eq!(scenario, Scenario::SpecComplete);
    }

    #[test]
    fn spec_incomplete_scenario() {
        let s = spec_incomplete("feat");
        let (scenario, _) = classify(&git(false, 0, false, true), None, Some(&s), None, false);
        assert_eq!(scenario, Scenario::SpecIncomplete);
    }

    #[test]
    fn issue_open_scenario() {
        let i = issue_open(42);
        let (scenario, _) = classify(&git(false, 0, false, true), None, None, Some(&i), false);
        assert_eq!(scenario, Scenario::IssueOpen);
    }

    #[test]
    fn issue_closed_scenario() {
        let i = issue_closed(42);
        let (scenario, _) = classify(&git(false, 0, false, true), None, None, Some(&i), false);
        assert_eq!(scenario, Scenario::IssueClosed);
    }

    // --- Edge cases ---

    #[test]
    fn pr_closed_unmerged_still_suggests_open_pr() {
        let pr = pr_closed_unmerged();
        let (_, actions) = classify(&git(false, 1, false, true), Some(&pr), None, None, false);
        assert!(actions.contains(&SuggestedAction::OpenPr));
    }

    #[test]
    fn commits_ahead_no_upstream_suggests_push() {
        let (_, actions) = classify(&git(false, 2, false, false), None, None, None, false);
        assert!(actions.contains(&SuggestedAction::Push));
    }

    #[test]
    fn open_pr_with_commits_ahead_is_not_unpushed_commits_scenario() {
        // commits_ahead measures vs the default branch, so it's always > 0 for a feature
        // branch. When a PR is open and tree is clean, the scenario should be CleanReady.
        let pr = pr_open(7);
        let (scenario, _) = classify(&git(false, 3, false, true), Some(&pr), None, None);
        assert_eq!(scenario, Scenario::CleanReady);
    }

    #[test]
    fn open_pr_with_upstream_and_clean_tree_suppresses_push() {
        // After OpenPr, branch is pushed and PR is open. commits_ahead > 0 because it
        // measures vs the default branch, not vs origin/feature-branch. Push should not
        // appear — the branch is already on the remote and the PR tracks it.
        let pr = pr_open(7);
        let (_, actions) = classify(&git(false, 3, false, true), Some(&pr), None, None);
        assert!(!actions.contains(&SuggestedAction::Push));
        assert!(actions.contains(&SuggestedAction::MergePr));
    }

    #[test]
    fn open_pr_with_dirty_tree_still_suggests_push() {
        // New uncommitted edits while PR is open still need push after commit.
        let pr = pr_open(7);
        let (_, actions) = classify(&git(true, 3, false, true), Some(&pr), None, None);
        assert!(actions.contains(&SuggestedAction::Push));
    }

    #[test]
    fn spec_complete_includes_archive_spec() {
        let s = spec_complete("my-change");
        let (_, actions) = classify(&git(false, 0, false, true), None, Some(&s), None, false);
        assert!(actions.contains(&SuggestedAction::ArchiveSpec));
    }

    #[test]
    fn spec_incomplete_does_not_suggest_archive_spec() {
        let s = spec_incomplete("my-change");
        let (_, actions) = classify(&git(false, 0, false, true), None, Some(&s), None, false);
        assert!(!actions.contains(&SuggestedAction::ArchiveSpec));
    }

    #[test]
    fn issue_open_no_pr_suggests_close_issue() {
        let i = issue_open(10);
        let (_, actions) = classify(&git(false, 0, false, true), None, None, Some(&i), false);
        assert!(actions.contains(&SuggestedAction::CloseIssue));
    }

    #[test]
    fn issue_open_with_open_pr_does_not_suggest_close_issue() {
        let pr = pr_open(5);
        let i = issue_open(10);
        let (_, actions) = classify(
            &git(false, 0, false, true),
            Some(&pr),
            None,
            Some(&i),
            false,
        );
        assert!(!actions.contains(&SuggestedAction::CloseIssue));
    }

    #[test]
    fn spec_and_issue_linked_simultaneously_spec_incomplete_wins() {
        let s = spec_incomplete("feat");
        let i = issue_open(99);
        let (scenario, _) = classify(&git(true, 0, false, true), None, Some(&s), Some(&i), false);
        assert_eq!(scenario, Scenario::SpecIncomplete);
    }

    #[test]
    fn spec_complete_with_open_issue_is_issue_open() {
        let s = spec_complete("feat");
        let i = issue_open(99);
        let (scenario, _) = classify(&git(false, 0, false, true), None, Some(&s), Some(&i), false);
        assert_eq!(scenario, Scenario::IssueOpen);
    }

    #[test]
    fn merged_branch_is_clean_ready() {
        let pr = pr_merged();
        let (scenario, _) = classify(&git(false, 0, true, true), Some(&pr), None, None, false);
        assert_eq!(scenario, Scenario::CleanReady);
    }

    #[test]
    fn no_number_in_branch_name_infers_none() {
        // This tests infer_active_issue indirectly by verifying issue-less classify
        let (scenario, _) = classify(&git(false, 0, false, true), None, None, None, false);
        assert_eq!(scenario, Scenario::CleanReady);
    }
}
