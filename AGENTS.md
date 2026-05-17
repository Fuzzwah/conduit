# Conduit — Claude Code Instructions

This is a personal fork of [conduit-cli/conduit](https://github.com/conduit-cli/conduit). Fork-specific changes are documented in `FORK_CHANGES.md`.

## Feature Development Workflow

New features use the OpenSpec workflow:

1. **Explore** — the user describes the feature or bug in plan mode; the agent builds a plan and asks clarifying questions as needed
2. **Propose** — once the plan is settled, use `/opsx:propose` to generate the spec and tasks
3. **Apply** — use `/opsx:apply` to implement the tasks

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
cargo clippy --workspace --all-targets -- -D warnings
```

The build script (in `crates/conduit-web/build.rs`) automatically runs `npm install && npm run build` in `crates/conduit-web/web/` when web assets are stale.

## Project Layout

This is a Cargo workspace. All Rust source lives under `crates/`; the root `Cargo.toml` is a virtual manifest.

```
crates/conduit-bin/      Binary entry point (`[[bin]] conduit`) — depends only on the umbrella
crates/conduit/          Umbrella library re-exporting the per-tier crates; holds the integration + E2E tests
crates/conduit-ui/       TUI built on Ratatui
crates/conduit-web/      Axum HTTP/WebSocket server; embeds the React frontend (web/ subdir + build.rs)
crates/conduit-core/     Glue between agent + data + git + web (ConduitCore facade)
crates/conduit-agent/    Agent runners (Claude, Codex, Gemini, OpenCode, Pi, …)
crates/conduit-data/     SQLite repositories (workspaces, sessions)
crates/conduit-git/      Worktree + PR management
crates/conduit-config/   User config + key bindings
crates/conduit-resolver/ Slash-command and menu-entry resolution
crates/conduit-session/  External-session discovery
crates/conduit-theme/    Theme types + parsing (depended on by both ui and web)
crates/conduit-types/    Shared types (Action, ChatMessage, app_prompt) — leaf, no internal deps
crates/conduit-util/     Pure utilities (data dirs, naming, git username)

website/                 Astro marketing site (getconduit.sh)
docs/                    mdBook documentation
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
gh pr create --repo Fuzzwah/conduit --base master --head "$(git branch --show-current)"
```

Avoid inline shell-quoted PR bodies; use a file so quotes/newlines are preserved:

```bash
tmp_body="$(mktemp)"
cat > "$tmp_body" <<'EOF'
## Summary
- ...

## Testing
- cargo fmt --check
- cargo clippy -- -D warnings
- cargo test
EOF

gh pr create \
  --repo Fuzzwah/conduit \
  --base master \
  --head "$(git branch --show-current)" \
  --title "..." \
  --body-file "$tmp_body"
rm -f "$tmp_body"
```

## Monitoring PR Checks

After creating a PR, monitor CI with:

```bash
gh pr checks <number> --repo Fuzzwah/conduit --watch
```

This polls until all checks complete and exits cleanly. Do **not** hand-roll an `until`/`sleep` loop — it will misfire.

## CI Checks

PRs must pass: `cargo check --workspace` → `cargo fmt --check` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo test --workspace`.

**Before declaring any code change complete, always run all four commands:**

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
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
6. Before reporting success, run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` and confirm all pass.
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
