# Conduit

Conduit is a keyboard-first interface for running coding agents inside git-backed workspaces. It ships as a Ratatui TUI and an optional local web UI, with support for Codex CLI, Claude Code, Gemini CLI, and OpenCode.

> This is a personal fork. See [FORK_CHANGES.md](FORK_CHANGES.md) for a summary of changes relative to the upstream project.

## What You Get

- Multi-agent sessions with up to 10 concurrent tabs
- Git-backed workspaces using either `worktree` or full `checkout` mode
- Session persistence, history restore, and external session import
- Provider and model selection with Codex `gpt-5.4` as the default
- Build/Plan mode for Claude, Codex, and Gemini
- Raw events view for inspecting agent traffic
- Token usage, context tracking, and cost estimates in the status bar
- Built-in themes plus VS Code theme migration
- A local web app served by the same binary

## Install

### Requirements

- `git` on your `PATH`
- At least one supported agent CLI on your `PATH`:
  - [Codex CLI](https://github.com/openai/codex) as `codex`
  - [Claude Code](https://docs.anthropic.com/en/docs/claude-code) as `claude`
  - [Gemini CLI](https://github.com/google-gemini/gemini-cli) as `gemini`
  - [OpenCode](https://opencode.ai/) as `opencode`

On first launch, Conduit checks for `git` and at least one agent binary, then prompts for tool paths if they are missing.

### Quick Install

```bash
curl -fsSL https://getconduit.sh/install | sh
```

### Homebrew

```bash
brew install conduit-cli/tap/conduit
```

### Build From Source

```bash
git clone https://github.com/conduit-cli/conduit.git
cd conduit
cargo build --release
```

The binary will be at `target/release/conduit`.

## Usage

### Start The TUI

```bash
conduit
```

### Start The Web UI

```bash
conduit serve --host 127.0.0.1 --port 3000
```

The local web app is served from the same binary and defaults to `http://127.0.0.1:3000`.

### Utility Commands

```bash
# Inspect how your terminal reports keys
conduit debug-keys

# Convert a VS Code theme into Conduit's TOML theme format
conduit migrate-theme path/to/theme.json --palette
```

## Common Workflows

- Add a repository, then create a workspace from the sidebar or project picker
- Open multiple tabs against different workspaces or providers
- Fork or hand off the current session into a new workspace and tab
- Resume saved sessions or import external sessions from Claude, Codex, and OpenCode
- Inspect chat output in the normal view or switch to Raw Events for protocol-level debugging

## Providers

| Provider | Default Model | Notes |
| --- | --- | --- |
| Codex CLI | `gpt-5.4` | Default provider for new sessions |
| Claude Code | `opus` | Supports Build/Plan mode |
| Gemini CLI | `gemini-2.5-pro` | Supports Build/Plan mode |
| OpenCode | `default` | Dynamic model list, no Plan-mode toggle |

Session import currently discovers Claude, Codex, and OpenCode sessions. Gemini session discovery is not implemented yet.

## Workspaces And Sessions

Conduit is built around git-backed workspaces. By default it creates git worktrees, but repositories can also use full checkout workspaces. Workspace defaults such as mode, branch cleanup, and remote-delete prompts are configurable in `~/.conduit/config.toml`.

Sessions are stored locally and can be resumed later. The UI also exposes queue editing, model switching, provider filtering, workspace archiving, PR actions, and file viewing from chat output.

## Common Shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl+N` | New project |
| `Ctrl+P` | Command palette |
| `Ctrl+O` | Model selector |
| `Ctrl+G` | Toggle Chat / Raw Events |
| `Alt+1..9` | Jump to tab |
| `Alt+Tab` / `Alt+Shift+Tab` | Next / previous tab |
| `Alt+Shift+F` | Fork session |
| `Alt+Shift+H` | Handoff session |
| `Alt+I` | Import session |
| `Ctrl+4` | Toggle Build / Plan mode |
| `Enter` | Submit prompt |
| `Shift+Enter` or `Alt+Enter` | Insert newline |
| `Ctrl+Q` | Quit |

Many terminals report `Ctrl+\` as `Ctrl+4`. Use `conduit debug-keys` if a shortcut does not behave as expected.

## Configuration

Conduit loads configuration from `~/.conduit/config.toml`. The bundled example covers:

- default provider and model
- enabled providers
- tool paths
- queue and steering behavior
- workspace defaults
- theme selection
- web status refresh settings
- keybinding overrides

The current default configuration starts new sessions on Codex, enables token and cost display, uses `worktree` mode for workspaces, and keeps provider selection open to all installed agents.

## UI Notes

The status bar tracks token usage, context usage, and a cost estimate. That estimate is currently based on configurable Claude pricing defaults, so treat it as an approximation rather than provider-specific billing data.

Conduit also includes a Raw Events view for session debugging and a local web app with repository, workspace, session, queue, onboarding, model, theme, and UI state endpoints behind the `serve` command.

## Repository Layout

- `src/` contains the TUI, agent runners, workspace logic, persistence, and the local web server
- `src/web/` contains the Axum-backed web UI and API
- `website/` contains the separate Astro marketing site for `getconduit.sh`
- `docs/` contains the mdBook documentation

## License

MIT
