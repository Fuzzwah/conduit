---
description: Build and test a conduit branch from its git worktree under ~/.conduit/workspaces/conduit/
argument-hint: "[branch-name]"
---

Build and run a conduit branch from its conduit-managed git worktree.

Conduit manages its own git worktrees under `~/.conduit/workspaces/conduit/`. When a branch is active in conduit it is **already checked out there** — running `git checkout <branch>` in `~/code/conduit` will fail with "already used by worktree". Build and run from the worktree directly instead.

**Input**: Optionally specify a branch name (e.g., `/branch-test add-auth`). If omitted, ask the user which branch to test.

**Provided arguments**: $@

## Steps

1. **Resolve the worktree path**

   List available worktrees:
   ```bash
   ls ~/.conduit/workspaces/conduit/
   git -C ~/code/conduit worktree list
   ```

   Conduit's worktree directory name is usually the branch name with `/` replaced by `-`, but confirm by listing. If the branch name was provided as an argument, find the matching worktree directory. If no match is found or the name was omitted, ask the user to clarify.

2. **Build from the worktree**

   Change to the worktree directory and build with the debug profile (does **not** overwrite the installed release binary):
   ```bash
   cd ~/.conduit/workspaces/conduit/<worktree-name>
   cargo build
   ```

   Web UI changes: `cargo build` automatically triggers `npm install && npm run build` in `web/` when assets are stale.

   If the build fails, report the error and stop.

3. **Report the binary location**

   The debug binary is at:
   ```
   ./target/debug/conduit
   ```

   Note which branch was built and the binary path so the user can run it.

## Usage Examples

```bash
# Run the TUI
./target/debug/conduit

# Run the web UI (default: 127.0.0.1:3000)
./target/debug/conduit serve

# Pass extra flags
./target/debug/conduit --model sonnet
```
