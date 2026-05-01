import { useEffect, useRef, useState, useMemo } from 'react';
import { X, Search, Loader2, ChevronRight, Sun, Moon, Check } from 'lucide-react';
import { useSettings } from '../hooks';
import { useTheme } from '../hooks';
import { cn } from '../lib/cn';
import type { SettingItem } from '../lib/api';

type SubEditor = 'providers' | 'workspace_defaults' | 'theme' | null;

interface SettingsDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onOpenBaseDirDialog: () => void;
  onOpenModelPicker: () => void;
}

export function SettingsDialog({
  isOpen,
  onClose,
  onOpenBaseDirDialog,
  onOpenModelPicker,
}: SettingsDialogProps) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [subEditor, setSubEditor] = useState<SubEditor>(null);
  const { data: settingsData, isLoading } = useSettings();

  // Filter settings based on search
  const filteredItems = useMemo(() => {
    if (!settingsData?.items) return [];
    const query = searchQuery.toLowerCase().trim();
    if (!query) return settingsData.items;
    return settingsData.items.filter(
      (item) =>
        item.title.toLowerCase().includes(query) ||
        item.description.toLowerCase().includes(query) ||
        item.value.toLowerCase().includes(query)
    );
  }, [settingsData, searchQuery]);

  // Handle dialog open/close
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    if (isOpen) {
      dialog.showModal();
      setSearchQuery('');
      setSelectedIndex(0);
      setSubEditor(null);
      setTimeout(() => searchInputRef.current?.focus(), 50);
    } else {
      dialog.close();
    }
  }, [isOpen]);

  // Handle escape/cancel
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    const handleCancel = (e: Event) => {
      e.preventDefault();
      if (subEditor) {
        setSubEditor(null);
      } else {
        onClose();
      }
    };

    dialog.addEventListener('cancel', handleCancel);
    return () => dialog.removeEventListener('cancel', handleCancel);
  }, [onClose, subEditor]);

  // Reset selection when search changes
  useEffect(() => {
    setSelectedIndex(0);
  }, [searchQuery]);

  // Handle item activation
  const handleActivateItem = (item: SettingItem) => {
    switch (item.id) {
      case 'projects_directory':
        onClose();
        onOpenBaseDirDialog();
        break;
      case 'default_model':
        onClose();
        onOpenModelPicker();
        break;
      case 'enabled_providers':
        setSubEditor('providers');
        break;
      case 'theme':
        setSubEditor('theme');
        break;
      case 'workspace_defaults':
        setSubEditor('workspace_defaults');
        break;
    }
  };

  // Keyboard navigation
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (subEditor) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((prev) => Math.min(prev + 1, filteredItems.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((prev) => Math.max(prev - 1, 0));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (filteredItems[selectedIndex]) {
        handleActivateItem(filteredItems[selectedIndex]);
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      if (subEditor) {
        setSubEditor(null);
      } else {
        onClose();
      }
    }
  };

  const handleBackdropClick = (e: React.MouseEvent<HTMLDialogElement>) => {
    if (e.target === dialogRef.current) {
      onClose();
    }
  };

  return (
    <dialog
      ref={dialogRef}
      onClick={handleBackdropClick}
      onKeyDown={handleKeyDown}
      className="m-auto max-h-[600px] w-full max-w-lg rounded-xl border border-border bg-surface p-0 shadow-xl backdrop:bg-black/50"
    >
      <div className="flex max-h-[600px] flex-col">
        {/* Header */}
        <div className="flex shrink-0 items-center justify-between border-b border-border px-6 py-4">
          <div className="flex items-center gap-2">
            {subEditor && (
              <button
                onClick={() => setSubEditor(null)}
                className="rounded-md p-1 text-text-muted transition-colors hover:bg-surface-elevated hover:text-text"
                aria-label="Back to settings"
              >
                <ChevronRight className="h-4 w-4 rotate-180" />
              </button>
            )}
            <h2 className="text-lg font-semibold text-text">
              {subEditor === 'providers'
                ? 'Enabled Providers'
                : subEditor === 'workspace_defaults'
                  ? 'Workspace Defaults'
                  : subEditor === 'theme'
                    ? 'Theme'
                    : 'Settings'}
            </h2>
          </div>
          <button
            onClick={onClose}
            className="rounded-md p-1 text-text-muted transition-colors hover:bg-surface-elevated hover:text-text"
            aria-label="Close dialog"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {/* Content area */}
        {subEditor === 'providers' ? (
          <ProvidersSubEditor />
        ) : subEditor === 'workspace_defaults' ? (
          <WorkspaceDefaultsSubEditor />
        ) : subEditor === 'theme' ? (
          <ThemeSubEditor />
        ) : (
          <>
            {/* Search input */}
            <div className="shrink-0 border-b border-border px-6 py-3">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-muted" />
                <input
                  ref={searchInputRef}
                  type="text"
                  placeholder="Search settings..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="w-full rounded-lg border border-border bg-surface-elevated py-2 pl-10 pr-4 text-sm text-text placeholder-text-muted focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent"
                />
              </div>
            </div>

            {/* Settings list */}
            <div className="min-h-0 flex-1 overflow-y-auto">
              {isLoading ? (
                <div className="flex items-center justify-center py-12">
                  <Loader2 className="h-6 w-6 animate-spin text-text-muted" />
                </div>
              ) : filteredItems.length === 0 ? (
                <div className="py-12 text-center text-text-muted">No settings found</div>
              ) : (
                <div className="py-2">
                  {filteredItems.map((item, index) => (
                    <button
                      key={item.id}
                      onClick={() => handleActivateItem(item)}
                      onMouseEnter={() => setSelectedIndex(index)}
                      className={cn(
                        'flex w-full items-center justify-between px-6 py-3 text-left transition-colors',
                        index === selectedIndex && 'bg-surface-elevated',
                        index !== selectedIndex && 'hover:bg-surface-elevated/50'
                      )}
                    >
                      <div className="flex min-w-0 flex-1 flex-col">
                        <div className="flex items-center gap-3">
                          <span className="font-medium text-text">{item.title}</span>
                          <span className="truncate text-xs text-text-muted">{item.value}</span>
                        </div>
                        <span className="text-xs text-text-muted">{item.description}</span>
                      </div>
                      <ChevronRight className="ml-2 h-4 w-4 shrink-0 text-text-muted" />
                    </button>
                  ))}
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </dialog>
  );
}

// --- Inline sub-editors ---

import { useProviders, useSetProviders } from '../hooks';

function ProvidersSubEditor() {
  const { data, isLoading } = useProviders();
  const setProviders = useSetProviders();
  const [localEnabled, setLocalEnabled] = useState<Set<string>>(new Set());
  const initialized = useRef(false);

  // Initialize local state from server data
  useEffect(() => {
    if (data && !initialized.current) {
      setLocalEnabled(new Set(data.providers.filter((p) => p.enabled).map((p) => p.id)));
      initialized.current = true;
    }
  }, [data]);

  // Reset initialized flag when component unmounts
  useEffect(() => {
    return () => {
      initialized.current = false;
    };
  }, []);

  const handleToggle = (id: string) => {
    setLocalEnabled((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        // Don't allow disabling all
        if (next.size <= 1) return prev;
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const handleSave = () => {
    setProviders.mutate([...localEnabled]);
  };

  const hasChanges =
    data &&
    initialized.current &&
    (() => {
      const serverEnabled = new Set(data.providers.filter((p) => p.enabled).map((p) => p.id));
      if (serverEnabled.size !== localEnabled.size) return true;
      for (const id of localEnabled) {
        if (!serverEnabled.has(id)) return true;
      }
      return false;
    })();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="h-6 w-6 animate-spin text-text-muted" />
      </div>
    );
  }

  return (
    <div className="flex flex-col">
      <div className="min-h-0 flex-1 overflow-y-auto py-2">
        {data?.providers.map((provider) => (
          <button
            key={provider.id}
            onClick={() => handleToggle(provider.id)}
            className="flex w-full items-center gap-3 px-6 py-3 text-left transition-colors hover:bg-surface-elevated"
          >
            <div
              className={cn(
                'flex h-5 w-5 shrink-0 items-center justify-center rounded border transition-colors',
                localEnabled.has(provider.id)
                  ? 'border-accent bg-accent text-white'
                  : 'border-border bg-surface-elevated'
              )}
            >
              {localEnabled.has(provider.id) && <Check className="h-3 w-3" />}
            </div>
            <div className="flex flex-col">
              <span className="font-medium text-text">{provider.display_name}</span>
              {!provider.installed && (
                <span className="text-xs text-warning">Not installed</span>
              )}
            </div>
          </button>
        ))}
      </div>
      {hasChanges && (
        <div className="flex justify-end border-t border-border px-6 py-3">
          <button
            onClick={handleSave}
            disabled={setProviders.isPending}
            className={cn(
              'flex items-center gap-2 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover disabled:opacity-70',
              setProviders.isPending && 'cursor-wait'
            )}
          >
            {setProviders.isPending && <Loader2 className="h-4 w-4 animate-spin" />}
            Save
          </button>
        </div>
      )}
    </div>
  );
}

import { useWorkspaceDefaults, useSetWorkspaceDefaults } from '../hooks';

function WorkspaceDefaultsSubEditor() {
  const { data, isLoading } = useWorkspaceDefaults();
  const setDefaults = useSetWorkspaceDefaults();
  const [mode, setMode] = useState('worktree');
  const [deleteBranch, setDeleteBranch] = useState(true);
  const [remotePrompt, setRemotePrompt] = useState(true);
  const initialized = useRef(false);

  // Initialize from server data
  useEffect(() => {
    if (data && !initialized.current) {
      setMode(data.mode);
      setDeleteBranch(data.archive_delete_branch);
      setRemotePrompt(data.archive_remote_prompt);
      initialized.current = true;
    }
  }, [data]);

  useEffect(() => {
    return () => {
      initialized.current = false;
    };
  }, []);

  const handleSave = () => {
    setDefaults.mutate({
      mode,
      archive_delete_branch: deleteBranch,
      archive_remote_prompt: remotePrompt,
    });
  };

  const hasChanges =
    data &&
    initialized.current &&
    (mode !== data.mode ||
      deleteBranch !== data.archive_delete_branch ||
      remotePrompt !== data.archive_remote_prompt);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="h-6 w-6 animate-spin text-text-muted" />
      </div>
    );
  }

  return (
    <div className="flex flex-col">
      <div className="space-y-6 px-6 py-4">
        {/* Mode selection */}
        <div>
          <div className="mb-2 text-sm font-medium text-text">Workspace Mode</div>
          <div className="space-y-2">
            <label className="flex cursor-pointer items-start gap-3 rounded-lg border border-border p-3 transition-colors hover:bg-surface-elevated">
              <input
                type="radio"
                name="mode"
                value="worktree"
                checked={mode === 'worktree'}
                onChange={() => setMode('worktree')}
                className="mt-0.5 accent-accent"
              />
              <div>
                <div className="text-sm font-medium text-text">Worktree</div>
                <div className="text-xs text-text-muted">
                  Lightweight, shares git metadata with the main repo
                </div>
              </div>
            </label>
            <label className="flex cursor-pointer items-start gap-3 rounded-lg border border-border p-3 transition-colors hover:bg-surface-elevated">
              <input
                type="radio"
                name="mode"
                value="checkout"
                checked={mode === 'checkout'}
                onChange={() => setMode('checkout')}
                className="mt-0.5 accent-accent"
              />
              <div>
                <div className="text-sm font-medium text-text">Checkout</div>
                <div className="text-xs text-text-muted">
                  Full clone for complete isolation
                </div>
              </div>
            </label>
          </div>
        </div>

        {/* Toggle switches */}
        <div className="space-y-3">
          <label className="flex cursor-pointer items-center justify-between">
            <div>
              <div className="text-sm font-medium text-text">Delete branch on archive</div>
              <div className="text-xs text-text-muted">
                Delete the local branch when archiving a workspace
              </div>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={deleteBranch}
              onClick={() => setDeleteBranch(!deleteBranch)}
              className={cn(
                'relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors',
                deleteBranch ? 'bg-accent' : 'bg-border'
              )}
            >
              <span
                className={cn(
                  'pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow ring-0 transition-transform',
                  deleteBranch ? 'translate-x-5' : 'translate-x-0'
                )}
              />
            </button>
          </label>

          <label className="flex cursor-pointer items-center justify-between">
            <div>
              <div className="text-sm font-medium text-text">Remote branch prompt</div>
              <div className="text-xs text-text-muted">
                Ask about deleting the remote branch on archive
              </div>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={remotePrompt}
              onClick={() => setRemotePrompt(!remotePrompt)}
              className={cn(
                'relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors',
                remotePrompt ? 'bg-accent' : 'bg-border'
              )}
            >
              <span
                className={cn(
                  'pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow ring-0 transition-transform',
                  remotePrompt ? 'translate-x-5' : 'translate-x-0'
                )}
              />
            </button>
          </label>
        </div>
      </div>

      {hasChanges && (
        <div className="flex justify-end border-t border-border px-6 py-3">
          <button
            onClick={handleSave}
            disabled={setDefaults.isPending}
            className={cn(
              'flex items-center gap-2 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent-hover disabled:opacity-70',
              setDefaults.isPending && 'cursor-wait'
            )}
          >
            {setDefaults.isPending && <Loader2 className="h-4 w-4 animate-spin" />}
            Save
          </button>
        </div>
      )}
    </div>
  );
}

function ThemeSubEditor() {
  const { themes, currentThemeName, currentTheme, setTheme: applyTheme } = useTheme();
  const [searchQuery, setSearchQuery] = useState('');
  const searchInputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const effectiveThemeName = currentThemeName ?? currentTheme?.name ?? null;

  useEffect(() => {
    setTimeout(() => searchInputRef.current?.focus(), 50);
  }, []);

  // Auto-scroll to current theme
  useEffect(() => {
    if (!effectiveThemeName) return;
    const list = listRef.current;
    if (!list) return;
    requestAnimationFrame(() => {
      const target = list.querySelector<HTMLElement>(
        `[data-theme-name="${CSS.escape(effectiveThemeName)}"]`
      );
      if (target) {
        target.scrollIntoView({ block: 'center' });
      }
    });
  }, [effectiveThemeName]);

  // Filter themes
  const filtered = useMemo(() => {
    const query = searchQuery.toLowerCase().trim();
    if (!query) return themes;
    return themes.filter(
      (t) =>
        t.name.toLowerCase().includes(query) ||
        t.displayName.toLowerCase().includes(query)
    );
  }, [themes, searchQuery]);

  // Group by source
  const builtinThemes = filtered.filter((t) => t.source === 'builtin');
  const vscodeThemes = filtered.filter((t) => t.source === 'vscode');
  const customThemes = filtered.filter((t) => t.source === 'toml' || t.source === 'custom');

  const lowerCurrent = effectiveThemeName?.toLowerCase() ?? null;
  const isSelected = (theme: { name: string; displayName: string }) =>
    lowerCurrent !== null &&
    (theme.name.toLowerCase() === lowerCurrent ||
      theme.displayName.toLowerCase() === lowerCurrent);

  const renderGroup = (label: string, groupThemes: typeof filtered) => {
    if (groupThemes.length === 0) return null;
    return (
      <div className="mb-2">
        <div className="px-6 py-2 text-xs font-medium uppercase tracking-wider text-text-muted">
          {label}
        </div>
        {groupThemes.map((theme) => (
          <button
            key={theme.name}
            data-theme-name={theme.name}
            onClick={() => applyTheme(theme.name)}
            className={cn(
              'flex w-full items-center gap-2 px-6 py-2 text-left text-sm transition-colors',
              isSelected(theme) ? 'bg-accent/10 text-accent' : 'text-text hover:bg-surface-elevated'
            )}
          >
            <span className="flex size-4 shrink-0 items-center justify-center">
              {theme.isLight ? (
                <Sun className="size-3 text-warning" />
              ) : (
                <Moon className="size-3 text-accent" />
              )}
            </span>
            <span className="flex-1 truncate">{theme.displayName}</span>
            {isSelected(theme) && <Check className="size-4 shrink-0 text-accent" />}
          </button>
        ))}
      </div>
    );
  };

  return (
    <>
      {/* Search */}
      <div className="shrink-0 border-b border-border px-6 py-3">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-muted" />
          <input
            ref={searchInputRef}
            type="text"
            placeholder="Search themes..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full rounded-lg border border-border bg-surface-elevated py-2 pl-10 pr-4 text-sm text-text placeholder-text-muted focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent"
          />
        </div>
      </div>

      {/* Theme list */}
      <div ref={listRef} className="min-h-0 flex-1 overflow-y-auto py-2">
        {filtered.length === 0 ? (
          <div className="py-12 text-center text-text-muted">No themes found</div>
        ) : (
          <>
            {renderGroup('Built-in', builtinThemes)}
            {renderGroup('VS Code', vscodeThemes)}
            {renderGroup('Custom', customThemes)}
          </>
        )}
      </div>
    </>
  );
}
