# Installing the Fuzzwah/conduit Fork

This is a personal fork of [conduit-cli/conduit](https://github.com/conduit-cli/conduit) with additional features. See [FORK_CHANGES.md](FORK_CHANGES.md) for the full list of changes. There are no pre-built binaries — you need to build from source.

## Prerequisites

- **Git** — Required for workspace and worktree management
- **Rust** (latest stable) — Install via [rustup](https://rustup.rs/): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
  Check your version with `cargo --version` and keep it current with `rustup update stable`. Cargo 1.82 and older are known to fail resolving the codex dependencies.
- **Node.js** (v18+) and **npm** — Required to build the web UI frontend
- **At least one AI agent CLI on your PATH:**
  - [Claude Code](https://docs.anthropic.com/en/docs/claude-code) — `npm install -g @anthropic-ai/claude-code`
  - [Codex CLI](https://github.com/openai/codex) — `npm install -g @openai/codex`
  - [Gemini CLI](https://github.com/google-gemini/gemini-cli) — `npm install -g @google/gemini-cli`
  - [OpenCode](https://opencode.ai/) — see their install docs

## Build from Source

```bash
git clone https://github.com/Fuzzwah/conduit.git
cd conduit
cargo build --release
```

The first build compiles the Rust code and automatically builds the web UI frontend (`npm install && npm run build` runs in `web/` as part of the build script). Expect it to take a few minutes.

The binary will be at `./target/release/conduit`.

## Install the Binary

Copy or symlink the binary to a directory on your `PATH`:

```bash
# Copy
cp ./target/release/conduit ~/.local/bin/

# Or symlink (stays up to date when you rebuild)
ln -s "$(pwd)/target/release/conduit" ~/.local/bin/conduit
```

If `~/.local/bin` is not on your `PATH`, add this to your shell config (`~/.bashrc`, `~/.zshrc`, or `~/.config/fish/config.fish`):

```bash
# bash / zsh
export PATH="$HOME/.local/bin:$PATH"

# fish
fish_add_path ~/.local/bin
```

## Verify

```bash
conduit --version
```

## First Run

```bash
conduit
```

On first launch, Conduit will:

1. **Detect Git** — Shows an error dialog if Git is not found
2. **Detect agents** — Searches for `claude`, `codex`, `gemini`, and `opencode` on your `PATH`
3. **Create config directory** — Creates `~/.conduit/` for settings, sessions, and workspaces

If no agents are found you'll be prompted to configure tool paths in the settings.

## Directory Structure

```
~/.conduit/
├── config.toml      # Configuration file
├── conduit.db       # SQLite database (sessions, workspaces)
├── logs/            # Application logs
├── workspaces/      # Git worktrees managed by Conduit
└── themes/          # Custom theme files
```

## Updating

Pull the latest changes and rebuild:

```bash
cd /path/to/conduit
git pull
cargo build --release
```

If you used a symlink during install the updated binary is available immediately. If you copied the binary, re-run the `cp` command.

## What's Different in This Fork

See [FORK_CHANGES.md](FORK_CHANGES.md) for the full list of 35 changes. Highlights:

**Agents**
- GitHub Copilot CLI as a 5th agent (access Codex models via a Copilot subscription)
- Pi as a 6th agent with session resumption and history import from `~/.pi/agent/sessions/`
- Dirac as a 7th agent with resumable sessions via `--taskId`
- Provider/agent selectors re-detect installed tools on open — no restart needed

**TUI**
- `@filename` autocomplete in the chat input
- `Alt+y` copies the nearest code block to clipboard (cycles through blocks on repeat)
- `Alt+Shift+X` archives the current workspace from inside a tab
- `Alt+a` opens a file browser to copy a local file into the workspace
- `Alt+u` generates an SCP command (copied via OSC 52) for uploading a file to the workspace
- `Alt+Tab` / `Alt+Shift+Tab` cycle between agent tabs only (sidebar excluded)
- `Ctrl+Q` opens a quit confirmation dialog instead of requiring two rapid keypresses
- Ahead/behind git counts (`↑N` / `↓N`) in the sidebar
- Always-visible sidebar mode (`ui.always_show_sidebar = true` in config)
- Scroll position preserved while the agent streams — viewport stays pinned where you scrolled
- Latest agent status message pinned to the top of the viewport during tool output
- Full plan content rendered in chat (no 15-line cap)
- `ctx%` shows current context usage per call, not a broken cumulative total
- `/btw <note>` queues a note without interrupting the agent

**Web UI**
- Paste images directly into the web UI chat input (`Ctrl+V`)
- Project list auto-refreshes in the TUI when projects are added via the web UI
- Drag-and-drop project reordering in the web UI (move-up/down in the TUI)
- Web chat auto-follows collapsed code blocks during streaming

**Themes & Config**
- 30 built-in themes including Night Owl and iTerm2-compatible palettes
- Companion tmux configuration in `~/.tmux.conf` tuned for Night Owl

**Workspace**
- `workspace_setup.sh` auto-executed after workspace creation
- Squash-merge detection in archive preflight (no false "branch not merged" warnings)
- GitHub PR status in archive preflight (`workspaces.use_gh_cli_merge_status = true`)
- Error dialog when a git URL maps to an already-existing directory
