import { useEffect, useRef, useState, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import {
  AlertTriangle,
  CheckCircle2,
  GitBranch,
  GitPullRequest,
  BookOpen,
  CircleDot,
  Loader2,
  X,
  ChevronRight,
  Check,
} from 'lucide-react';
import { cn } from '../lib/cn';
import {
  useWorkCompletePreflight,
  useWorkCompleteCommit,
  useWorkCompletePush,
  useWorkCompleteOpenPr,
  useWorkCompleteMergePr,
  useWorkCompleteCloseIssue,
  useWorkCompleteArchiveSpec,
  useWorkCompleteArchive,
  queryKeys,
} from '../hooks/useApi';
import { useWebSocket } from '../hooks/useWebSocket';
import type { Workspace, Session } from '../types';
import type {
  SuggestedAction,
  WorkCompleteScenario,
  WorkCompletePreflight,
} from '../types/api';

type Phase =
  | { kind: 'loading' }
  | { kind: 'reviewing' }
  | { kind: 'force_confirm'; action: SuggestedAction; reason: string }
  | { kind: 'commit_message' }
  | { kind: 'executing'; action: SuggestedAction }
  | { kind: 'admin_merge_confirm' };

interface WorkCompleteDialogProps {
  isOpen: boolean;
  workspace: Workspace | null;
  session: Session | null;
  onClose: () => void;
  onArchived: (workspace: Workspace) => void;
}

function suggestCommitMessage(preflight: WorkCompletePreflight): string {
  const parts: string[] = [];
  const branch = preflight.branch_name.replace(/^[^/]+\//, '');
  if (preflight.spec) {
    parts.push(`Implement ${preflight.spec.change_id}`);
  } else {
    parts.push(branch);
  }
  if (preflight.issue) {
    parts.push(`Fix #${preflight.issue.number}`);
  }
  for (const file of preflight.dirty_files.slice(0, 2)) {
    parts.push(file.path);
  }
  return parts.join('; ');
}

function getActionLabel(action: SuggestedAction): { label: string; description: string } {
  switch (action) {
    case 'commit':
      return { label: 'Commit', description: 'Stage and commit changes' };
    case 'push':
      return { label: 'Push', description: 'Push commits to remote' };
    case 'open_pr':
      return { label: 'Open PR', description: 'Create a pull request' };
    case 'merge_pr':
      return { label: 'Merge PR', description: 'Merge the open pull request' };
    case 'close_issue':
      return { label: 'Close Issue', description: 'Close the linked GitHub issue' };
    case 'archive_spec':
      return { label: 'Archive Spec', description: 'Mark spec as done' };
    case 'archive':
      return { label: 'Archive Workspace', description: 'Close and archive this workspace' };
    case 'show_remaining_tasks':
      return { label: 'Show Remaining Tasks', description: 'Ask agent to list incomplete tasks' };
  }
}

function getExecutingLabel(action: SuggestedAction): string {
  switch (action) {
    case 'commit':
      return 'Committing…';
    case 'push':
      return 'Pushing…';
    case 'open_pr':
      return 'Opening PR…';
    case 'merge_pr':
      return 'Merging PR…';
    case 'close_issue':
      return 'Closing issue…';
    case 'archive_spec':
      return 'Archiving spec…';
    case 'archive':
      return 'Archiving workspace…';
    case 'show_remaining_tasks':
      return 'Working…';
  }
}

function getScenarioLabel(scenario: WorkCompleteScenario): string {
  switch (scenario) {
    case 'clean_ready':
      return 'Clean — ready to archive';
    case 'edits_no_link':
      return 'Uncommitted edits';
    case 'spec_complete':
      return 'Spec complete';
    case 'spec_incomplete':
      return 'Spec in progress';
    case 'issue_open':
      return 'Issue open';
    case 'issue_closed':
      return 'Issue closed';
  }
}

function isSuccessScenario(scenario: WorkCompleteScenario): boolean {
  return scenario === 'clean_ready' || scenario === 'spec_complete' || scenario === 'issue_closed';
}

function getMergeReadinessLabel(readiness: string): string {
  switch (readiness) {
    case 'ready':
      return 'Ready to merge';
    case 'blocked':
      return 'Checks failing or review pending';
    case 'has_conflicts':
      return 'Has merge conflicts';
    default:
      return 'Mergeability unknown';
  }
}

export function WorkCompleteDialog({
  isOpen,
  workspace,
  session,
  onClose,
  onArchived,
}: WorkCompleteDialogProps) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const adminDialogRef = useRef<HTMLDialogElement>(null);
  const queryClient = useQueryClient();
  const { sendPrompt } = useWebSocket();

  const [phase, setPhase] = useState<Phase>({ kind: 'loading' });
  const [actionLog, setActionLog] = useState<string[]>([]);
  const [commitMessage, setCommitMessage] = useState('');
  const [selectedActionIndex, setSelectedActionIndex] = useState(0);

  const workspaceId = workspace?.id ?? null;

  const preflight = useWorkCompletePreflight(isOpen ? workspaceId : null, { staleTime: 0 });

  const commitMutation = useWorkCompleteCommit();
  const pushMutation = useWorkCompletePush();
  const openPrMutation = useWorkCompleteOpenPr();
  const mergePrMutation = useWorkCompleteMergePr();
  const closeIssueMutation = useWorkCompleteCloseIssue();
  const archiveSpecMutation = useWorkCompleteArchiveSpec();
  const archiveMutation = useWorkCompleteArchive();

  const anyPending =
    commitMutation.isPending ||
    pushMutation.isPending ||
    openPrMutation.isPending ||
    mergePrMutation.isPending ||
    closeIssueMutation.isPending ||
    archiveSpecMutation.isPending ||
    archiveMutation.isPending;

  // Open/close main dialog
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (isOpen) {
      if (!dialog.open) dialog.showModal();
    } else {
      if (dialog.open) dialog.close();
    }
  }, [isOpen]);

  // Open/close admin merge dialog
  useEffect(() => {
    const dialog = adminDialogRef.current;
    if (!dialog) return;
    if (phase.kind === 'admin_merge_confirm') {
      if (!dialog.open) dialog.showModal();
    } else {
      if (dialog.open) dialog.close();
    }
  }, [phase.kind]);

  // Block Escape while executing
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    const handleCancel = (e: Event) => {
      e.preventDefault();
      if (!anyPending && phase.kind !== 'executing') {
        onClose();
      }
    };
    dialog.addEventListener('cancel', handleCancel);
    return () => dialog.removeEventListener('cancel', handleCancel);
  }, [anyPending, onClose, phase.kind]);

  // Reset when dialog opens
  useEffect(() => {
    if (isOpen && workspaceId) {
      setPhase({ kind: 'loading' });
      setActionLog([]);
      setSelectedActionIndex(0);
      queryClient.invalidateQueries({ queryKey: queryKeys.workCompletePreflight(workspaceId) });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen]);

  // Transition loading → reviewing when preflight arrives
  useEffect(() => {
    if (phase.kind === 'loading' && preflight.data && !preflight.isFetching) {
      setPhase({ kind: 'reviewing' });
      setSelectedActionIndex(0);
    }
  }, [phase.kind, preflight.data, preflight.isFetching]);

  const appendLog = useCallback((lines: string[]) => {
    setActionLog((prev) => [...prev, ...lines]);
  }, []);

  const afterAction = useCallback(
    (lines: string[]) => {
      appendLog(lines);
      setPhase({ kind: 'loading' });
      if (workspaceId) {
        queryClient.invalidateQueries({
          queryKey: queryKeys.workCompletePreflight(workspaceId),
        });
      }
    },
    [appendLog, queryClient, workspaceId]
  );

  const executeAction = useCallback(
    (action: SuggestedAction, adminMerge = false) => {
      if (!workspaceId) return;
      setPhase({ kind: 'executing', action });

      const handleError = (e: Error) => {
        appendLog([`Error: ${e.message}`]);
        setPhase({ kind: 'reviewing' });
      };

      switch (action) {
        case 'push':
          pushMutation.mutate(workspaceId, {
            onSuccess: (res) => afterAction(res.log_lines),
            onError: handleError,
          });
          break;
        case 'open_pr':
          openPrMutation.mutate(
            { id: workspaceId },
            {
              onSuccess: (res) =>
                afterAction([...res.log_lines, `PR created: ${res.url}`]),
              onError: handleError,
            }
          );
          break;
        case 'merge_pr':
          mergePrMutation.mutate(
            { id: workspaceId, admin: adminMerge },
            {
              onSuccess: (res) => afterAction(res.log_lines),
              onError: handleError,
            }
          );
          break;
        case 'close_issue':
          closeIssueMutation.mutate(workspaceId, {
            onSuccess: (res) => afterAction(res.log_lines),
            onError: handleError,
          });
          break;
        case 'archive_spec':
          archiveSpecMutation.mutate(
            { id: workspaceId, changeId: preflight.data?.spec?.change_id ?? '' },
            {
              onSuccess: (res) => afterAction([...res.log_lines, ...res.warnings]),
              onError: handleError,
            }
          );
          break;
        case 'archive':
          archiveMutation.mutate(workspaceId, {
            onSuccess: (res) => {
              appendLog(res.log_lines);
              if (workspace) onArchived(workspace);
              onClose();
            },
            onError: handleError,
          });
          break;
        default:
          setPhase({ kind: 'reviewing' });
          break;
      }
    },
    [
      workspaceId,
      workspace,
      preflight.data,
      pushMutation,
      openPrMutation,
      mergePrMutation,
      closeIssueMutation,
      archiveSpecMutation,
      archiveMutation,
      afterAction,
      appendLog,
      onArchived,
      onClose,
    ]
  );

  const handleActionClick = useCallback(
    (action: SuggestedAction, index: number) => {
      if (!preflight.data) return;
      setSelectedActionIndex(index);

      if (action === 'show_remaining_tasks') {
        const changeId = preflight.data.spec?.change_id ?? '';
        if (session) {
          sendPrompt(
            session.id,
            `show incomplete tasks in ${changeId}`,
            workspace?.path ?? '',
            session.model ?? undefined
          );
        }
        onClose();
        return;
      }

      if (action === 'commit') {
        setCommitMessage(suggestCommitMessage(preflight.data));
        setPhase({ kind: 'commit_message' });
        return;
      }

      if (action === 'archive') {
        const scenario = preflight.data.scenario;
        if (scenario === 'spec_incomplete') {
          setPhase({
            kind: 'force_confirm',
            action,
            reason: 'The linked spec still has incomplete tasks.',
          });
          return;
        }
        if (scenario === 'issue_open') {
          setPhase({
            kind: 'force_confirm',
            action,
            reason: 'The linked issue is still open.',
          });
          return;
        }
      }

      if (action === 'merge_pr' && preflight.data.pr?.merge_readiness !== 'ready') {
        setPhase({ kind: 'admin_merge_confirm' });
        return;
      }

      executeAction(action);
    },
    [preflight.data, session, workspace, sendPrompt, onClose, executeAction]
  );

  const handleCommitSubmit = useCallback(() => {
    if (!workspaceId || !commitMessage.trim()) return;
    setPhase({ kind: 'executing', action: 'commit' });
    commitMutation.mutate(
      { id: workspaceId, message: commitMessage },
      {
        onSuccess: (res) => afterAction([...res.log_lines, `Committed: ${res.sha}`]),
        onError: (e) => {
          appendLog([`Error: ${e.message}`]);
          setPhase({ kind: 'reviewing' });
        },
      }
    );
  }, [workspaceId, commitMessage, commitMutation, afterAction, appendLog]);

  const handleBackdropClick = (e: React.MouseEvent<HTMLDialogElement>) => {
    if (e.target === dialogRef.current && !anyPending && phase.kind !== 'executing') {
      onClose();
    }
  };

  const data = preflight.data;

  return (
    <>
      <dialog
        ref={dialogRef}
        onClick={handleBackdropClick}
        className="m-auto w-full max-w-xl rounded-xl border border-border bg-surface p-0 shadow-xl backdrop:bg-black/50"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-border px-6 py-4">
          <h2 className="text-lg font-semibold text-text">Work Complete</h2>
          <button
            onClick={onClose}
            disabled={anyPending || phase.kind === 'executing'}
            className="rounded-md p-1 text-text-muted transition-colors hover:bg-surface-elevated hover:text-text disabled:opacity-50"
            aria-label="Close dialog"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {/* Body */}
        <div className="space-y-4 px-6 py-5">
          {phase.kind === 'loading' && (
            <div className="flex items-center gap-3 py-2 text-text-muted">
              <Loader2 className="h-4 w-4 animate-spin text-accent" />
              <span>Analyzing workspace…</span>
            </div>
          )}

          {(phase.kind === 'reviewing' ||
            phase.kind === 'force_confirm' ||
            phase.kind === 'commit_message') &&
            data && (
              <div className="space-y-1.5 text-sm">
                {/* Scenario badge */}
                <div
                  className={cn(
                    'flex items-center gap-2 font-medium',
                    isSuccessScenario(data.scenario) ? 'text-emerald-400' : 'text-amber-400'
                  )}
                >
                  {isSuccessScenario(data.scenario) ? (
                    <CheckCircle2 className="h-4 w-4" />
                  ) : (
                    <AlertTriangle className="h-4 w-4" />
                  )}
                  <span>{getScenarioLabel(data.scenario)}</span>
                </div>

                {/* Branch */}
                <div className="flex items-center gap-2 text-text-muted">
                  <GitBranch className="h-3.5 w-3.5 shrink-0" />
                  <span className="text-text-secondary">{data.branch_name}</span>
                  {(data.commits_ahead > 0 || data.commits_behind > 0) && (
                    <span className="text-xs">
                      ↑{data.commits_ahead} ↓{data.commits_behind}
                    </span>
                  )}
                  {data.is_dirty && (
                    <span className="text-amber-400">{data.dirty_files.length} modified</span>
                  )}
                </div>

                {/* PR */}
                {data.pr && (
                  <div
                    className={cn(
                      'flex items-center gap-2 text-sm',
                      data.pr.is_merged
                        ? 'text-purple-400'
                        : data.pr.is_open
                          ? 'text-emerald-400'
                          : 'text-text-muted'
                    )}
                  >
                    <GitPullRequest className="h-3.5 w-3.5 shrink-0" />
                    <span>PR #{data.pr.number}</span>
                    <span className="text-text-muted">
                      ({data.pr.is_merged ? 'merged' : data.pr.is_open ? 'open' : 'closed'})
                    </span>
                    {data.pr.url && (
                      <a
                        href={data.pr.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="truncate text-text-muted hover:underline"
                      >
                        {data.pr.url}
                      </a>
                    )}
                    {data.pr.title && (
                      <span className="truncate text-text-muted">{data.pr.title}</span>
                    )}
                    {data.pr.is_open && data.pr.merge_readiness !== 'ready' && (
                      <span className="ml-auto shrink-0 text-xs text-amber-400">
                        {getMergeReadinessLabel(data.pr.merge_readiness)}
                      </span>
                    )}
                  </div>
                )}

                {/* Spec */}
                {data.spec && (
                  <div className="flex items-center gap-2 text-sm">
                    <BookOpen className="h-3.5 w-3.5 shrink-0 text-text-muted" />
                    <span className="text-text-muted">spec:</span>
                    <span className="text-text-secondary">{data.spec.change_id}</span>
                    <span className="text-xs text-text-muted">({data.spec.source})</span>
                    <span
                      className={cn(
                        'ml-auto shrink-0 text-xs',
                        data.spec.completed >= data.spec.total
                          ? 'text-emerald-400'
                          : 'text-amber-400'
                      )}
                    >
                      {data.spec.completed}/{data.spec.total} tasks
                    </span>
                  </div>
                )}

                {/* Issue */}
                {data.issue && (
                  <div
                    className={cn(
                      'flex items-center gap-2 text-sm',
                      data.issue.is_open ? 'text-amber-400' : 'text-emerald-400'
                    )}
                  >
                    <CircleDot className="h-3.5 w-3.5 shrink-0" />
                    <span>Issue #{data.issue.number}</span>
                    <span className="text-text-muted">
                      ({data.issue.is_open ? 'open' : 'closed'})
                    </span>
                    {data.issue.title && (
                      <span className="truncate text-text-muted">{data.issue.title}</span>
                    )}
                  </div>
                )}
              </div>
            )}

          {/* Action list */}
          {phase.kind === 'reviewing' && data && (
            <div className="border-t border-border pt-3">
              <div className="space-y-0.5">
                {data.suggested_actions.map((action, i) => {
                  const { label, description } = getActionLabel(action);
                  const isSelected = i === selectedActionIndex;
                  const mergeBlocked =
                    action === 'merge_pr' && data.pr?.merge_readiness !== 'ready';

                  return (
                    <button
                      key={action}
                      disabled={mergeBlocked}
                      onClick={() => handleActionClick(action, i)}
                      title={
                        mergeBlocked
                          ? getMergeReadinessLabel(data.pr?.merge_readiness ?? 'unknown')
                          : undefined
                      }
                      className={cn(
                        'flex w-full items-start gap-2 rounded-lg px-3 py-2 text-left text-sm transition-colors',
                        isSelected ? 'bg-accent/10' : 'hover:bg-surface-elevated',
                        mergeBlocked && 'cursor-not-allowed opacity-40'
                      )}
                    >
                      <ChevronRight
                        className={cn(
                          'mt-0.5 h-3.5 w-3.5 shrink-0 text-accent',
                          isSelected ? 'opacity-100' : 'opacity-0'
                        )}
                      />
                      <div>
                        <span
                          className={cn(
                            'font-medium',
                            isSelected ? 'text-accent' : 'text-text'
                          )}
                        >
                          {label}
                        </span>
                        <span className="ml-2 text-text-muted">{description}</span>
                      </div>
                    </button>
                  );
                })}

                {/* Admin merge option when merge is blocked */}
                {data.pr?.is_open && data.pr.merge_readiness !== 'ready' && (
                  <button
                    onClick={() => setPhase({ kind: 'admin_merge_confirm' })}
                    className="flex w-full items-start gap-2 rounded-lg px-3 py-2 text-left text-sm text-amber-400 transition-colors hover:bg-surface-elevated"
                  >
                    <ChevronRight className="mt-0.5 h-3.5 w-3.5 shrink-0 opacity-0" />
                    <div>
                      <span className="font-medium">Merge PR (admin)</span>
                      <span className="ml-2 text-text-muted">force merge, bypassing checks</span>
                    </div>
                  </button>
                )}
              </div>
            </div>
          )}

          {/* Force confirm */}
          {phase.kind === 'force_confirm' && (
            <div className="space-y-4">
              <div className="flex items-start gap-3 rounded-lg border border-amber-500/30 bg-amber-500/10 px-4 py-3">
                <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-amber-400" />
                <div className="space-y-1">
                  <p className="font-medium text-amber-200">Warning</p>
                  <p className="text-sm text-amber-100">{phase.reason}</p>
                  <p className="text-sm text-amber-100">Are you sure you want to proceed?</p>
                </div>
              </div>
              <div className="flex justify-end gap-3">
                <button
                  onClick={() => setPhase({ kind: 'reviewing' })}
                  className="rounded-lg px-4 py-2 text-sm font-medium text-text-muted transition-colors hover:bg-surface-elevated"
                >
                  Cancel
                </button>
                <button
                  onClick={() => executeAction(phase.action)}
                  className="rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-amber-400"
                >
                  Proceed anyway
                </button>
              </div>
            </div>
          )}

          {/* Commit message input */}
          {phase.kind === 'commit_message' && (
            <div className="space-y-3">
              <div className="border-t border-border pt-3">
                <label className="mb-2 block text-sm font-medium text-text-secondary">
                  Commit message
                </label>
                <textarea
                  value={commitMessage}
                  onChange={(e) => setCommitMessage(e.target.value)}
                  className="w-full resize-none rounded-lg border border-border bg-surface-elevated px-3 py-2 text-sm text-text focus:outline-none focus:ring-1 focus:ring-accent"
                  rows={3}
                  autoFocus
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) handleCommitSubmit();
                    if (e.key === 'Escape') setPhase({ kind: 'reviewing' });
                  }}
                />
                <div className="mt-3 flex justify-end gap-3">
                  <button
                    onClick={() => setPhase({ kind: 'reviewing' })}
                    className="rounded-lg px-4 py-2 text-sm font-medium text-text-muted transition-colors hover:bg-surface-elevated"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={handleCommitSubmit}
                    disabled={!commitMessage.trim()}
                    className="flex items-center gap-2 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover disabled:opacity-50"
                  >
                    <Check className="h-4 w-4" />
                    Commit
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* Executing spinner */}
          {phase.kind === 'executing' && (
            <div className="flex items-center gap-3 py-2 text-text-muted">
              <Loader2 className="h-4 w-4 animate-spin text-accent" />
              <span>{getExecutingLabel(phase.action)}</span>
            </div>
          )}

          {/* Action log */}
          {actionLog.length > 0 && (
            <div className="max-h-32 overflow-y-auto rounded-lg bg-surface-elevated px-3 py-2 font-mono text-xs text-text-muted">
              {actionLog.map((line, i) => (
                <div key={i}>{line}</div>
              ))}
            </div>
          )}
        </div>
      </dialog>

      {/* Admin merge confirm — separate dialog (task 5.8) */}
      <dialog
        ref={adminDialogRef}
        className="m-auto w-full max-w-md rounded-xl border border-border bg-surface p-0 shadow-xl backdrop:bg-black/50"
      >
        <div className="flex items-center justify-between border-b border-border px-6 py-4">
          <h2 className="text-lg font-semibold text-text">Force Merge</h2>
          <button
            onClick={() => setPhase({ kind: 'reviewing' })}
            className="rounded-md p-1 text-text-muted transition-colors hover:bg-surface-elevated"
          >
            <X className="h-5 w-5" />
          </button>
        </div>
        <div className="space-y-4 px-6 py-5">
          <div className="flex items-start gap-3 rounded-lg border border-amber-500/30 bg-amber-500/10 px-4 py-3">
            <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-amber-400" />
            <div className="space-y-1">
              <p className="font-medium text-amber-200">PR is not ready to merge</p>
              <p className="text-sm text-amber-100">
                {data?.pr ? getMergeReadinessLabel(data.pr.merge_readiness) : 'Unknown issue'}.
                Using --admin will bypass all branch protection rules and cannot be undone.
              </p>
            </div>
          </div>
          <div className="flex justify-end gap-3">
            <button
              onClick={() => setPhase({ kind: 'reviewing' })}
              className="rounded-lg px-4 py-2 text-sm font-medium text-text-muted transition-colors hover:bg-surface-elevated"
            >
              Cancel
            </button>
            <button
              onClick={() => {
                setPhase({ kind: 'reviewing' });
                executeAction('merge_pr', true);
              }}
              disabled={mergePrMutation.isPending}
              className="flex items-center gap-2 rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-black transition-colors hover:bg-amber-400 disabled:opacity-50"
            >
              {mergePrMutation.isPending && <Loader2 className="h-4 w-4 animate-spin" />}
              Force merge (admin)
            </button>
          </div>
        </div>
      </dialog>
    </>
  );
}
