# Fork Changes

This documents the changes made in [Fuzzwah/conduit](https://github.com/Fuzzwah/conduit) relative to the upstream [conduit-cli/conduit](https://github.com/conduit-cli/conduit).

> **Screenshots:** Place image files in `docs/screenshots/` and they will render inline below.

## At A Glance

![Conduit TUI workspace view](docs/screenshots/workspace.png)

![Clean start screen on first launch](docs/screenshots/clean-start.png)

![Built-in help screen](docs/screenshots/help-screen.png)

---

## 1. Workspace Setup Script

After a workspace is created, Conduit now looks for a `workspace_setup.sh` script in the repository root and runs it automatically. This is useful for installing dependencies, setting up environment files, or running any other per-workspace initialisation steps without manual intervention.

This applies to workspaces created from both the TUI and the web UI.

---

## 2. Paste Images into Web UI Chat

The web UI chat input now supports pasting images directly from the clipboard (`Ctrl+V` / `Cmd+V`). Pasted images are embedded in the message and sent to the agent alongside the text prompt.

The TUI also received a companion fix: bracketed paste mode is now properly enabled, so multi-line text pasted into the input is handled correctly instead of being submitted prematurely.

![Pasting an image into the web UI chat input](docs/screenshots/web-image-paste.png)

---

## 3. Alt+Tab No Longer Cycles to the Sidebar

Previously, `Alt+Tab` / `Alt+Shift+Tab` would include the sidebar in the workspace cycle, which made keyboard navigation feel inconsistent. The sidebar is now excluded so the shortcut moves only between agent tabs.

---

## 4. Plan Mode Feedback Input Wraps Long Text

Long text typed into the plan mode feedback input now wraps within the input box rather than overflowing off-screen. This makes it easier to write detailed instructions before submitting.

---

## 5. TUI Project List Auto-Refreshes

When a new project is added via the web UI, the TUI sidebar project list now updates automatically without requiring a restart or manual refresh. The TUI polls for changes and merges them in the background.

---

## 6. Copy Code Blocks to Clipboard

**`Alt+y`** copies the nearest visible code block to the clipboard regardless of current focus. Pressing `Alt+y` repeatedly cycles through all code blocks in the current output.

---

## 7. Reorder Projects in the Sidebar

Projects in the sidebar can now be reordered via drag-and-drop in the web UI, or with dedicated move-up / move-down actions in the TUI. The order is persisted and reflected across both interfaces.

---

## 8. `@filename` Autocomplete in Chat Input

Typing `@` in the TUI chat input triggers an autocomplete menu that lists files in the current workspace. Selecting a file inserts its path as a mention, making it easy to reference specific files in prompts without typing paths by hand.

![The @filename autocomplete menu in the TUI chat input](docs/screenshots/file-mention-autocomplete.png)

---

## 9. 30 Built-in Themes

The theme picker now ships with 30 built-in themes, including a full set of iTerm2-compatible colour palettes. Themes can be switched live from the command palette (`Ctrl+P` → `theme`) without restarting.

![The theme picker showing the 30 built-in themes](docs/screenshots/builtin-themes.png)

![Switching the active theme live from the theme selector](docs/screenshots/theme-selection.png)

---

## 10. Archive Workspace from Inside a Tab

**`Alt+Shift+X`** archives the current workspace from anywhere — both from within a workspace tab and from the sidebar. Previously archiving required navigating to the sidebar and pressing `x`; both contexts now use the same hotkey.

The archive hint appears in the chat footer, and the action is available in the command palette.

![Archiving a workspace from inside a tab](docs/screenshots/workspace-archive.png)

---

## 11. Sidebar Selection Tracks Active Tab When Hidden

When switching or closing tabs with the sidebar hidden, the sidebar selection now stays in sync with the active workspace. Previously, opening the sidebar after a tab change would still show the previously-selected workspace highlighted.

---

## 12. Squash-Merge Detection in Archive Preflight

The archive preflight check now distinguishes between genuinely unmerged branches and branches that were squash-merged. When a branch has commits not in main's ancestry but the diff against main is empty, the dialog shows "Squash-merged (N commits ahead, diff already in main)" at informational severity rather than the alarming "Branch not merged" warning.

---

## 13. Always-Visible Sidebar Mode

A new `always_show_sidebar` config option keeps the sidebar permanently on screen:

```toml
[ui]
always_show_sidebar = true
```

When enabled, `Ctrl+T` toggles **focus** to the sidebar rather than hiding it, and opening or creating a workspace no longer closes it. Press `Escape` to return focus to the chat input while keeping the sidebar visible.

---

## 14. Ahead/Behind Counts in the Sidebar

Each workspace in the sidebar now shows `↑N` (yellow) and `↓N` (red) indicators when its branch has commits ahead of or behind `origin/main`. Both indicators are suppressed when zero, keeping the display clean. Counts are refreshed every 30 seconds without a network fetch.

```
  ▼ feature/my-branch
     my-workspace     +3 -1 ↑2 ↓1 #42 ✓
```

![Ahead/behind indicators in the sidebar](docs/screenshots/tui-ahead-behind.png)

---

## 15. Full Plan Content in Chat

The plan review step no longer caps plan content at 15 lines. The full plan is now rendered inline as part of the scrollable chat history, so long plans can be read in their entirety by scrolling back.

---

## 16. GitHub Copilot CLI as a 5th Agent

The standalone `copilot` CLI is now supported as a 5th agent alongside Claude, Codex, Gemini, and OpenCode. This provides access to GitHub Copilot's model lineup — including the Codex models — for users with a GitHub Copilot subscription but no direct OpenAI API key.

**Invocation:** `copilot -p "PROMPT" -s --allow-all [--model=MODEL]` (single-shot programmatic mode)

**Available models:** `gpt-5.3-codex` (default), `gpt-5.3-codex-spark`, `gpt-5.4`, `claude-sonnet-4-5`, `gpt-4o`, `o3-mini`

**Known limitation:** The `copilot` CLI has no session resumption flag, so each turn within a Conduit session starts a fresh process with no conversation history.

Copilot appears in the agent selector, model selector (`Ctrl+O`), provider selector, and session import picker. It must be enabled via the provider selector before it appears in the model list.

![Provider selector with Copilot enabled alongside the other agents](docs/screenshots/provider-select.png)

![Model selector showing Copilot's model lineup](docs/screenshots/model-select.png)

---

## 17. Companion tmux Configuration (`~/.tmux.conf`)

A tmux status bar tuned to complement Conduit's Night Owl colour scheme. Key settings:

```tmux
set -g set-clipboard on

# Match the Night Owl background
set -g status-style "bg=#011627,fg=#5f7e97"

# Align everything to the left
set -g status-justify left

# Left side: teal icon + bold current command name
set -g status-left "#[fg=#7fdbca]   #[fg=#d6deeb,bold]#{pane_current_command}  "
set -g status-left-length 50

# Hide the default window list (command name in status-left is sufficient)
setw -g window-status-current-format ""
setw -g window-status-format ""

# Right side: minimalist time with dim blue pill
set -g status-right "#[fg=#1d3b53]#[fg=#d6deeb,bg=#1d3b53] %H:%M #[fg=#1d3b53,bg=default]"

set -g status-left-style default
```

- Background `#011627` matches Night Owl's editor background.
- Left segment shows `#{pane_current_command}` (e.g. `conduit`) in off-white bold, prefixed by a teal icon.
- The window list is suppressed entirely — the active command name is context enough.
- Right segment shows `HH:MM` inside a dim blue `#1d3b53` pill with powerline-style end caps.
- `set-clipboard on` allows tmux to sync with the system clipboard, which pairs well with Conduit's `Alt+y` copy shortcut.
