# Tool Paths

Configure paths to external tools.

## Default Detection

Conduit searches your `PATH` for:
- `git` — Required
- `gh` — GitHub CLI (optional)
- `codex` — Codex CLI agent
- `claude` — Claude Code agent
- `gemini` — Gemini CLI agent
- `cecli` or `aider-ce` — CE CLI agent

## Custom Paths

If tools aren't in your PATH, specify them:

```toml
[tools]
git = "/usr/bin/git"
gh = "/usr/local/bin/gh"
codex = "/home/user/.local/bin/codex"
claude = "/opt/homebrew/bin/claude"
gemini = "/home/user/.local/bin/gemini"
cecli = "/home/user/.local/bin/cecli"
```

## Verifying Paths

Check tool detection:

```bash
# Should show tool locations
which codex claude gemini cecli git gh
```

## Missing Tools

If a required tool is missing, Conduit shows a dialog on startup with options to:
- Configure the path manually
- Skip (if optional)
