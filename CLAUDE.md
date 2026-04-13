# Conduit — Claude Code Instructions

This is a personal fork of [conduit-cli/conduit](https://github.com/conduit-cli/conduit). Fork-specific changes are documented in `FORK_CHANGES.md`.

## Tech Stack

- **TUI:** Rust (2021) + Ratatui + Crossterm
- **Web UI backend:** Axum (Rust)
- **Web UI frontend:** React/TypeScript (compiled and embedded into the binary)
- **Database:** SQLite via rusqlite

## Common Commands

```bash
# Build
cargo build                   # debug
cargo build --release         # release

# Run
cargo run                     # TUI
cargo run -- serve            # web UI (default: 127.0.0.1:3000)

# Test
cargo test
cargo test -- --nocapture

# Lint / format
cargo fmt --all
cargo clippy -- -D warnings
```

The build script automatically runs `npm install && npm run build` in `web/` when web assets are stale.

## Project Layout

```
src/          TUI, agent runners, workspace logic, web server
src/web/      Axum API handlers and WebSocket endpoints
web/          React/TypeScript frontend
website/      Astro marketing site (getconduit.sh)
docs/         mdBook documentation
tests/        Integration and E2E tests
```

## Testing Notes

- Snapshot tests use `insta` — run `cargo insta review` to accept updated snapshots.
- Property-based tests use `proptest` (JSONL parsing).
- E2E tests use `termwright`.

## Manual Testing a Branch

Conduit manages its own git worktrees under `~/.conduit/workspaces/`. When a branch is active in conduit, it is already checked out there — attempting `git checkout <branch>` in `~/code/conduit` will fail with "already used by worktree".

To manually test a branch, build and run from the worktree directly:

```bash
cd ~/.conduit/workspaces/conduit/<worktree-name>
cargo build
./target/debug/conduit
```

`cargo build` (without `--release`) outputs to `target/debug/` and does not overwrite any installed release binary.

## PRs

Always target `Fuzzwah/conduit` (not the upstream `conduit-cli/conduit`):

```bash
gh pr create --repo Fuzzwah/conduit
```

## CI Checks

PRs must pass: `cargo check` → `cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo test`.

**Before declaring any code change complete, always run all four commands:**

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

(`cargo clippy` implies `cargo check`, so running all three above covers the full CI gate.)

Do not substitute `cargo build` for these — it skips format, lint, and test verification.

---

## Rules of Engagement

1. Read relevant files before proposing or making changes.
2. Do not make speculative improvements outside the request.
3. Prefer small, local edits over broad refactors unless the task requires it.
4. Prefer dedicated tools for reading, editing, searching, and testing.
5. If multiple reads or searches are independent, run them in parallel.
6. Before reporting success, run `cargo fmt --check && cargo clippy -- -D warnings && cargo test` and confirm all pass.
7. Never claim tests passed unless output confirms it.
8. Ask before taking destructive or externally visible actions.

## Known Failure Patterns to Avoid

- Reading code and then claiming the behavior is verified.
- Cleaning up unrelated code while "already in there."
- Reporting a task complete without running the relevant check.
- Treating a likely fix as a confirmed fix.

## Response Style

- Before the first tool call, state your plan in one or two sentences.
- During execution, provide brief progress updates at natural milestones.
- In the final response, lead with outcome, then verification status, then any remaining risks.
