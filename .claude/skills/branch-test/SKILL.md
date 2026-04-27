---
name: branch-test
description: Build and run a conduit branch from its conduit-managed git worktree under ~/.conduit/workspaces/conduit/. Use when the user asks to manually test, try, or run a branch.
---

# branch-test

Conduit manages its own git worktrees under `~/.conduit/workspaces/conduit/`. When a branch is active in conduit it is **already checked out there** — running `git checkout <branch>` in `~/code/conduit` will fail with "already used by worktree". Build and run from the worktree directly instead.

## Steps

1. Resolve the worktree path. Conduit's worktree directory name is usually the branch name with `/` replaced by `-`, but list to be sure:

   ```bash
   ls ~/.conduit/workspaces/conduit/
   git -C ~/code/conduit worktree list
   ```

2. Build from the worktree (debug profile — does **not** overwrite the installed release binary):

   ```bash
   cd ~/.conduit/workspaces/conduit/<worktree-name>
   cargo build
   ```

3. Run the resulting debug binary:

   ```bash
   ./target/debug/conduit            # TUI
   ./target/debug/conduit serve      # web UI on 127.0.0.1:3000
   ```

## Notes

- Use `cargo build` (debug), not `cargo build --release`, unless the user asks. Debug builds land in `target/debug/` and leave the user's installed release binary alone.
- Do not `git checkout` the branch in `~/code/conduit` — the worktree owns it.
- Web UI changes: `cargo build` triggers `npm install && npm run build` in `web/` automatically when assets are stale (per `build.rs`).
