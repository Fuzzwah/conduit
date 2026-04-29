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

## 12. Squash-Merge Detection and GitHub PR Status in Archive Preflight

The archive preflight check now distinguishes between genuinely unmerged branches and branches that were squash-merged. When a branch has commits not in main's ancestry but the diff against main is empty, the dialog shows "Squash-merged (N commits ahead, diff already in main)" at informational severity rather than the alarming "Branch not merged" warning.

Optionally, the preflight can also query the GitHub CLI for the actual PR state. When `workspaces.use_gh_cli_merge_status = true` is set in the config, the archive dialog shows the live PR state: "PR merged (via GitHub)", "PR is open", "PR is a draft", or "PR closed without merging" (as a warning). The git-based detection remains active as a fallback when `gh` is unavailable or the PR is not found.

```toml
[workspaces]
use_gh_cli_merge_status = true
```

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

**Available models:** `gpt-5.3-codex` (default), `gpt-5.3-codex-spark`, `gpt-5.4`, `gpt-5.4-mini`, `gpt-5.4-nano`, `gpt-5.2`, `gpt-5.2-codex`, `gpt-5-mini`, `gpt-4.1`, `gpt-4o`, `claude-haiku-4.5`, `claude-sonnet-4`, `claude-sonnet-4.5`, `claude-sonnet-4.6`, `gemini-2.5-pro`, `gemini-3-flash`, `gemini-3.1-pro`, `grok-code-fast-1`, `raptor-mini`, `goldeneye`

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

---

## 18. Quit Confirmation Dialog (`Ctrl+Q`)

Instead of requiring two rapid keypresses to quit, `Ctrl+Q` now opens a proper confirmation dialog with **Quit** and **Cancel** buttons. The **Quit** option is pre-selected, so pressing Enter immediately after `Ctrl+Q` quits without further navigation. `Esc` dismisses the dialog; pressing `Ctrl+Q` a second time while the dialog is open also confirms quit.

---

## 19. Copy Local File into Workspace (`M-a`)

**`Alt+a`** opens a two-step file browser for copying a local file into the current workspace repository:

1. **Step 1 — source:** Browse the local filesystem (starting from the home directory) to select a file.
2. **Step 2 — destination:** Browse the repository's directory tree; press `c` to confirm the copy destination.

The file is copied via `fs::copy`. A footer message confirms the destination on success; `Esc` cancels at any point.

---

## 20. Upload File to Workspace via SCP (`M-u`)

**`Alt+u`** opens a destination browser within the current workspace repository. After selecting a directory and pressing `c`, Conduit displays an SCP command with the absolute destination path — automatically copied to the clipboard via OSC 52 (works over SSH/tmux). Run the SCP command from your workstation, then press Enter inside Conduit to scan the destination for newly arrived (or updated) files and see their names reported in the dialog.

---

## 21. Alt+Shift+Tab Cycles to Previous Tab

**`Alt+Shift+Tab`** (`M-S-Tab`) now cycles backward through workspace tabs. The existing `Alt+Tab` forward cycle is unchanged. The footer hint has been updated to show both shortcuts: `M-tab/M-S-tab: next/prev tab`.

---

## 22. TUI Text Input Improvements

Several input-handling fixes land across the TUI:

- **Word navigation:** `Alt+b` / `Alt+f` move the cursor backward / forward by word in the plan-mode feedback input (mirrors Emacs-style navigation, handles terminals that send these sequences for `Option+Left/Right`).
- **AskUserQuestion text wrap:** Long answers in the "type something" input now wrap within the box rather than overflowing off-screen.
- **Continuation-row indent:** The wrapped text input applies a consistent wrap point across all rows, fixing a 2-character misalignment on continuation rows.

---

## 23. Scroll Position Preserved During Streaming

Scrolling up while the agent is generating output now keeps the viewport pinned to your scroll position. New content growing below no longer yanks the view down. Scrolling back to the bottom restores auto-follow behaviour automatically.

---

## 24. Context-Window Percentage (`ctx%`) Shows Current Usage

The `ctx%` counter now reflects the current context-window state based on per-call token usage from each assistant message (`input + cache_creation + cache_read`). Previously it accumulated cumulative totals from the final result event, causing readings above 100% (seen as high as 908%) during long sessions. After compaction, `ctx%` naturally drops to match the post-compaction assistant message.

---

## 25. Web Chat Auto-Follow in Collapsed Code Blocks

While an agent streams output inside a collapsed markdown code block, the web chat view now keeps the collapsed block pinned to the bottom of the viewport. Previously, output continued inside the block but the main view appeared to stall.

---

## 26. GitHub Issue Picker Integration Test

The GitHub issue picker is now verified end-to-end: selecting an issue when creating a workspace correctly links the workspace to that issue and displays the issue title in the workspace header.

---

## 27. Pi Coding Agent as a 6th Agent

Conduit now supports **Pi** as an additional agent alongside Claude, Codex, Gemini, OpenCode, and Copilot.

- **Invocation:** `pi --mode rpc`
- **Sessions:** Pi sessions can be resumed, and Conduit preserves the live Pi process across turns so follow-up prompts stay in the same session.
- **History import:** Conduit can discover Pi sessions from `~/.pi/agent/sessions/` and import their visible chat history from Pi's JSONL session files.
- **Events/tools:** Pi's structured RPC event stream is mapped into Conduit's chat/debug views, including assistant text, reasoning, and tool execution events.

Pi appears in the agent selector, provider selector, and model selector. Common Pi model presets are included, with `claude-sonnet-4.6` as the default.

---

## 28. `/btw` Command — Queue a Note Without Interrupting

The `/btw` command, available natively in the Claude Code CLI, is now supported in the Conduit TUI.

- **`/btw <note>`** — immediately queues the note as a follow-up message without interrupting the agent. Works whether the agent is idle or actively running.
- **`/btw`** (no args) — opens the queue editor.
- The command appears in the `/` autocomplete menu with description "Queue a note without interrupting".

### Claude Code command tracking

Claude Code's built-in slash commands are compiled into the CLI binary and not discoverable programmatically. The table below tracks which commands conduit implements and how they map:

Verified against Claude Code v2.1.119 (`strings` on the binary).

| Claude Code command | Conduit equivalent | Status |
|---|---|---|
| `/btw <note>` | `/btw` → queues as `FollowUp` message | ✅ Implemented (this change) |
| `/clear` | `/new` → starts a new session | ✅ Equivalent |
| `/config` | `Ctrl+P` → Settings | ✅ Partial equivalent |
| `/effort <level>` | `/reasoning` | ✅ Equivalent |
| `/fast` | — | ❌ Not implemented |
| `/feedback` | — | ❌ Not implemented |
| `/help` | `?` on empty input | ✅ Equivalent |
| `/hooks` | — | ❌ Not implemented |
| `/init` | — | ❌ Not implemented |
| `/login` / `/logout` | — | ❌ Not implemented |
| `/mcp` | — | ❌ Not implemented |
| `/memory` | — | ❌ Not implemented |
| `/model` | `/model` | ✅ Implemented |
| `/quit` | `Ctrl+Q` | ✅ Equivalent |
| `/resume` | session import picker | ✅ Partial equivalent |
| `/rewind` | `/rewind` → removes last turn from display + truncates Claude session file | ✅ Implemented (Claude only) |
| `/status` | `/status` → shows agent/model/session/ctx%/turns/dir as a chat message | ✅ Implemented (all agents) |

To add a new command: add a variant to `ConduitCommand` in `src/command_resolver.rs`, handle it in `handle_submit_action` in `src/ui/app.rs`, update `slash_command_action` and `execute_resolved_conduit_command`, and add a row to this table.
