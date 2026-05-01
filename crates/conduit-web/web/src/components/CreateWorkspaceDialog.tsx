import { useEffect, useRef } from 'react';
import { X, Info, CheckCircle, AlertCircle, Loader2 } from 'lucide-react';
import { useAutoCreateWorkspaceStream } from '../hooks';
import type { Workspace } from '../types';
import { cn } from '../lib/cn';

const MAX_VISIBLE_LINES = 10;

interface CreateWorkspaceDialogProps {
  repositoryId: string;
  repositoryName: string;
  isOpen: boolean;
  onClose: () => void;
  onModeRequired: () => void;
  onSuccess: (workspace: Workspace) => void;
}

export function CreateWorkspaceDialog({
  repositoryId,
  repositoryName,
  isOpen,
  onClose,
  onModeRequired,
  onSuccess,
}: CreateWorkspaceDialogProps) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const logEndRef = useRef<HTMLDivElement>(null);
  const { status, messages, workspace, error, start, reset } = useAutoCreateWorkspaceStream();

  const isRunning = status === 'running';
  const isIdle = status === 'idle';

  // Handle dialog open/close
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (isOpen) {
      dialog.showModal();
    } else {
      dialog.close();
      reset();
    }
  }, [isOpen, reset]);

  // Scroll log to bottom as messages arrive
  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Call onModeRequired as a side-effect so the dialog can be closed first
  useEffect(() => {
    if (status === 'mode_required') {
      reset();
      onModeRequired();
    }
  }, [status, reset, onModeRequired]);

  // Prevent closing while running
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    const handleCancel = (e: Event) => {
      e.preventDefault();
      if (!isRunning) onClose();
    };
    dialog.addEventListener('cancel', handleCancel);
    return () => dialog.removeEventListener('cancel', handleCancel);
  }, [isRunning, onClose]);

  const handleCreate = () => {
    start(repositoryId);
  };

  const handleBackdropClick = (e: React.MouseEvent<HTMLDialogElement>) => {
    if (e.target === dialogRef.current && !isRunning) onClose();
  };

  const visibleMessages =
    messages.length > MAX_VISIBLE_LINES ? messages.slice(-MAX_VISIBLE_LINES) : messages;

  return (
    <dialog
      ref={dialogRef}
      onClick={handleBackdropClick}
      className="m-auto max-w-md rounded-xl border border-border bg-surface p-0 shadow-xl backdrop:bg-black/50"
    >
      <div className="flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-border px-6 py-4">
          <div className="flex items-center gap-2">
            {isRunning && <Loader2 className="h-4 w-4 animate-spin text-accent" />}
            <h2 className="text-lg font-semibold text-text">
              {isIdle ? 'Create New Workspace' : 'Creating Workspace'}
            </h2>
          </div>
          <button
            onClick={onClose}
            disabled={isRunning}
            className="rounded-md p-1 text-text-muted transition-colors hover:bg-surface-elevated hover:text-text disabled:opacity-50"
            aria-label="Close dialog"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {/* Content */}
        <div className="px-6 py-5">
          {isIdle ? (
            <>
              <p className="text-text">
                Create a new workspace in{' '}
                <span className="font-medium">"{repositoryName}"</span>?
              </p>
              <div className="mt-4 flex items-start gap-2 rounded-lg bg-accent/10 px-3 py-2.5 text-sm text-text-muted">
                <Info className="mt-0.5 h-4 w-4 shrink-0 text-accent" />
                <span>A unique name and branch will be generated automatically.</span>
              </div>
            </>
          ) : (
            <>
              {/* Progress log */}
              <div className="min-h-[8rem] rounded-lg bg-surface-elevated px-3 py-2.5 font-mono text-xs text-text-muted">
                {visibleMessages.length === 0 ? (
                  <span className="opacity-50">Starting…</span>
                ) : (
                  visibleMessages.map((msg, i) => (
                    <div key={i} className="leading-5">
                      {msg}
                    </div>
                  ))
                )}
                <div ref={logEndRef} />
              </div>

              {/* Status line */}
              <div className="mt-3 flex items-center gap-2 text-sm">
                {isRunning && (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin text-accent" />
                    <span className="text-text-muted">Working…</span>
                  </>
                )}
                {status === 'done' && workspace && (
                  <>
                    <CheckCircle className="h-4 w-4 text-green-500" />
                    <span className="text-green-500">Workspace created</span>
                  </>
                )}
                {status === 'error' && (
                  <>
                    <AlertCircle className="h-4 w-4 text-red-400" />
                    <span
                      className={cn(
                        'text-red-400',
                        error && error.length > 60 && 'line-clamp-2'
                      )}
                      title={error ?? undefined}
                    >
                      {error ?? 'Creation failed'}
                    </span>
                  </>
                )}
              </div>
            </>
          )}
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-3 border-t border-border px-6 py-4">
          {isIdle && (
            <>
              <button
                onClick={onClose}
                className="rounded-lg px-4 py-2 text-sm font-medium text-text-muted transition-colors hover:bg-surface-elevated hover:text-text"
              >
                Cancel
              </button>
              <button
                onClick={handleCreate}
                className="rounded-lg bg-accent px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover"
              >
                Create Workspace
              </button>
            </>
          )}
          {status === 'done' && workspace && (
            <button
              onClick={() => onSuccess(workspace)}
              className="rounded-lg bg-green-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-green-700"
            >
              Open Workspace
            </button>
          )}
          {status === 'error' && (
            <button
              onClick={onClose}
              className="rounded-lg bg-red-600/80 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-red-600"
            >
              Close
            </button>
          )}
        </div>
      </div>
    </dialog>
  );
}
