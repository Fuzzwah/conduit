# Installing the Fuzzwah/conduit Fork

This is a personal fork of [conduit-cli/conduit](https://github.com/conduit-cli/conduit) with additional features. See [FORK_CHANGES.md](FORK_CHANGES.md) for the full list of changes. There are no pre-built binaries — you need to build from source.

## Prerequisites

- **Git** — Required for workspace and worktree management
- **Rust** (stable) — Install via [rustup](https://rustup.rs/): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
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

See [FORK_CHANGES.md](FORK_CHANGES.md) for the full list. Highlights:

- 30 built-in themes (including Night Owl and iTerm2-compatible palettes)
- `@filename` autocomplete in the TUI chat input
- Ahead/behind git counts in the sidebar
- `Alt+y` to copy code blocks to clipboard
- `Alt+Shift+X` to archive a workspace from inside a tab
- `workspace_setup.sh` auto-execution after workspace creation
- Squash-merge detection in archive preflight
- Full plan content in chat (no 15-line cap)
- Companion tmux configuration in `~/.tmux.conf.conduit`
