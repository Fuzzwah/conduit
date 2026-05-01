## Context

Today the conduit repo is a single Cargo package: root `Cargo.toml` declares `name = "conduit-cli"` with `[lib] name = "conduit"` and `[[bin]] name = "conduit"`, both compiled from `src/`. Source totals ~95k LOC across nine top-level modules:

| Module | LOC | Notes |
|---|---|---|
| `ui` | 58,175 | Dominant. Includes `app.rs` (15k), `chat_view.rs` (3.8k), `theme/builtin.rs` (2.7k). |
| `agent` | 14,672 | Pulls `agent-client-protocol`, `codex-protocol` (git tag), `codex-app-server-protocol`. |
| `web` | 7,306 | Pulls `axum`, `tower`, `tower-http`, `rust-embed`, `reqwest`. Embeds `web/dist`. |
| `git` | 3,121 | Largely a leaf. |
| `config` | 3,096 | Imports `ui::action` + `ui::events` (cycle). |
| `data` | 2,448 | SQLite via rusqlite; depends on `agent` + `git`. |
| `core` | 1,524 | Service orchestrator. |
| `session` | 1,517 | Discovery of external sessions. |
| `util` | 1,431 | Leaf. |
| `command_resolver.rs` | 987 | Single file; depends only on `agent`. |

Verified cross-module dependencies (via grep of `use crate::<module>` in each subdirectory):

- `agent → ui::components` at `src/agent/history.rs:18` and `src/agent/display.rs:6` (uses `ChatMessage`, `MessageRole`, `TurnSummary`).
- `config → ui::action`/`ui::events` at `src/config/keys.rs:13,190,704,750,758`, `src/config/default_keys.rs:11`, `src/config/settings.rs:10`.
- `web → ui::*` at `src/web/ws/handler.rs:21` (`app_prompt`), `src/web/handlers/sessions.rs:22-23` (`app_prompt`, `ChatMessage`, `MessageRole`), `src/web/handlers/themes.rs:7,361` (`theme::{Theme, set_theme, theme_test_lock_async, ...}`), `src/web/handlers/settings.rs:9` (`theme::current_theme_name`).

`build.rs` runs `npm install && npm run build` against `web/`, then `rust-embed` derives `#[folder = "web/dist"]` in `src/web/routes/static_files.rs:13`.

Integration tests under `tests/integration/*.rs` consume only the public lib re-exports (`use conduit::X`).

The user has chosen an aggressive split (each top-level module becomes its own crate, `ui` left whole) targeted at incremental dev builds, with baseline measurements taken before and after.

## Goals / Non-Goals

**Goals:**
- After editing a single file in `agent`/`util`/`web`/`config`/`data`/`git`/`session`/`core`/`resolver`/`theme`, `cargo build` recompiles only that crate plus the binary, leaving `conduit-ui` and the heavy third-party deps (`ratatui`, `axum`, `syntect`, `codex-protocol`, `reqwest`) cached.
- Preserve the public API surface: `conduit::App`, `conduit::Config`, `conduit::Database`, `conduit::ConduitCore`, etc., remain importable from a single `conduit` crate so external consumers and existing `tests/` keep compiling.
- Keep one `conduit` binary, same name, same CLI behavior.
- Pass the existing CI gate unchanged in spirit (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`) — adapted to `--workspace`.
- Provide reproducible baseline + post-split measurements so the user can verify the win.

**Non-Goals:**
- Splitting `ui` itself into sub-crates. Touching `ui/app.rs` (15k LOC) will still rebuild all of `conduit-ui`. This is the trade the user accepted.
- Replacing or upgrading any third-party dep.
- Trimming `tokio`'s `"full"` feature set. Workspace feature unification will pull "full" anyway, and slimming requires per-call audits out of scope here.
- Adding `sccache` as a hard requirement. Mention as opt-in; default config does not assume it.
- Publishing any of the new crates to crates.io. They are private workspace members.
- Changing test framework, snapshot tool, or e2e harness.

## Decisions

### Target crate layout

Twelve crates under `crates/`, plus a thin re-export umbrella:

| Crate | Purpose | Internal deps |
|---|---|---|
| `conduit-util` | Existing `src/util/`. | — |
| `conduit-types` | NEW. Houses cross-module shared types: `Action`, `KeyBinding`, `InputMode`, `ViewMode`, `ChatMessage`, `MessageRole`, `TurnSummary`, `FileChange`, `app_prompt::*`. Also `GitTrackerUpdate` and a few small enums currently in `ui::git_tracker` to avoid forcing `conduit-types → conduit-ui`. | `conduit-util`, `conduit-agent`, `conduit-git` (for re-referenced types like `AgentEvent`, `GithubIssue`) |
| `conduit-git` | Existing `src/git/`. | `conduit-util` |
| `conduit-agent` | Existing `src/agent/`, after Fix A removes the `ui::components` references. | `conduit-util`, `conduit-types` |
| `conduit-resolver` | Single file `src/command_resolver.rs`. | `conduit-util`, `conduit-agent` |
| `conduit-data` | Existing `src/data/`. | `conduit-util`, `conduit-agent`, `conduit-git` |
| `conduit-session` | Existing `src/session/`. | `conduit-util`, `conduit-agent` |
| `conduit-config` | Existing `src/config/`, after Fix B removes `ui::action`/`ui::events` references. | `conduit-util`, `conduit-agent`, `conduit-git`, `conduit-types` |
| `conduit-theme` | NEW. Extracts `src/ui/components/theme/` (~6.5k LOC across 8 files). Required to break Fix C cleanly so `web` does not pull `ratatui`. | `conduit-util`, `conduit-types` |
| `conduit-core` | Existing `src/core/`. | `conduit-util`, `conduit-agent`, `conduit-config`, `conduit-data`, `conduit-git` |
| `conduit-ui` | Remaining `src/ui/` (after `action.rs`, `events.rs`, `app_prompt.rs`, `components/chat_message.rs`, `components/turn_summary.rs`, `components/theme/*` are extracted). Largest crate; not split internally. | All non-web/non-bin crates above |
| `conduit-web` | Existing `src/web/` plus the moved `web/` frontend dir and `build.rs`. | `conduit-util`, `conduit-types`, `conduit-agent`, `conduit-config`, `conduit-data`, `conduit-git`, `conduit-session`, `conduit-resolver`, `conduit-core`, `conduit-theme` |
| `conduit` (umbrella) | Re-exports only. `pub use conduit_agent::*; pub use conduit_ui::App; ...` mirrors today's `src/lib.rs:12-35`. Hosts `tests/`. | All of the above except `conduit-bin` |
| `conduit-bin` | `src/main.rs`. `[[bin]] name = "conduit"`. | `conduit` umbrella + `clap`, `tokio`, `anyhow`, `tracing-subscriber` |

**Rationale (vs. alternatives considered):**

- *Why an umbrella `conduit` crate at all?* Without it, every `tests/integration/*.rs` `use conduit::X` line would need rewriting to per-crate paths (~hundreds of edits) and any external consumer of the lib breaks. The umbrella is `pub use`-only, so it codegen costs nothing measurable.
- *Why a `conduit-types` crate instead of moving the shared types directly into one of the existing leaf crates?* The shared types span agent/ui/web territory (e.g. `Action` references `agent::AgentEvent` and `git::GithubIssue`; `ChatMessage` is consumed by `agent`, `ui`, `web`). A neutral lightweight types crate (no `ratatui`, no `axum`) breaks all three cycles with one new compilation unit instead of forcing artificial dep direction reversals.
- *Why a separate `conduit-theme` crate instead of feature-gating `theme` inside `conduit-ui`?* `web` only needs the persistence/migration layer of the theme module (`Theme` struct, `current_theme_name`, `set_theme`). If `web` depended on `conduit-ui` even via a feature flag, it would still pull `ratatui` and `syntect` as transitive deps. A standalone `conduit-theme` crate (which `conduit-ui` re-exports) keeps `web`'s dep tree free of TUI deps. The 6.5k LOC theme subtree is also rarely co-edited with the rest of `ui`, so isolating it gives the user a second incremental win.
- *Why move `tests/` into the umbrella and not keep them at the root?* Cargo only discovers `tests/` adjacent to a package's `Cargo.toml`. Keeping them at the root would require the root to remain a package, breaking the virtual-manifest design.
- *Why move `build.rs` into `conduit-web`?* The build script's whole job is preparing `web/dist` for the `rust-embed` derive in `web::routes::static_files`. Tying it to that crate means it only re-runs when `conduit-web` recompiles — currently every `cargo build` re-checks the npm tree.

### Three forced inversion fixes

**Fix A (`agent → ui::components`):** Move `src/ui/components/chat_message.rs` and `src/ui/components/turn_summary.rs` content into `conduit-types`. Re-export from `conduit-ui::components` (`pub use conduit_types::{ChatMessage, MessageRole, TurnSummary, FileChange};`) so existing UI callers compile unchanged.

**Fix B (`config → ui::action`/`ui::events`):** Move `src/ui/action.rs` and `src/ui/events.rs` into `conduit-types`. The `Action` enum references `AgentEvent` (from `conduit-agent`), `GithubIssue`/`OpenSpec`/`PrPreflightResult`/`SpecifySpec` (from `conduit-git`), and `GitTrackerUpdate` (small enum currently in `ui::git_tracker`). The first two are addressable by adding `conduit-agent` and `conduit-git` to `conduit-types`'s deps; `GitTrackerUpdate` moves into `conduit-types` to keep the dep arrow correct. Re-export from `conduit-ui::action`/`conduit-ui::events` for in-tree callers.

**Fix C (`web → ui::app_prompt` + `ui::components::theme` + `ui::components::{ChatMessage, MessageRole}`):** Move `src/ui/app_prompt.rs` into `conduit-types` (it only needs `ChatMessage`, which already moved in Fix A). Extract `src/ui/components/theme/` into the new `conduit-theme` crate. Re-export both from their original `conduit-ui` paths for source compatibility.

### Build profile additions

Set on the workspace, not per-crate:

```toml
[profile.dev]
split-debuginfo = "unpacked"
debug = "line-tables-only"
codegen-units = 256
incremental = true

[profile.dev.package."*"]
opt-level = 0
```

`split-debuginfo = "unpacked"` shaves seconds off relink. `debug = "line-tables-only"` keeps backtraces but drops type info from `.o` files (≈3× smaller, faster link). `codegen-units = 256` lets rustc parallelise compilation of the still-large `conduit-ui` crate. `incremental = true` is the default but stated explicitly.

### Linker config (Linux dev)

`.cargo/config.toml`:
```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

Requires `mold` and `clang`; if absent, Cargo errors and the user can either install them (`apt install mold clang`) or delete the file. Bundled with the workspace split because relinking is the second-largest cost after `ui` rebuilds, and the change is one file.

### `[workspace.dependencies]`

Every third-party dep currently in root `Cargo.toml` moves into `[workspace.dependencies]` with its current version + features. Per-crate `[dependencies]` declare `tokio = { workspace = true }` etc., so feature unions stay consistent (especially `tokio = "full"`, `agent-client-protocol` features = ["unstable"], `serde` features = ["derive"]). Git deps (`codex-protocol`, `codex-app-server-protocol`) are declared once in `[workspace.dependencies]` and only `conduit-agent` lists them in its `[dependencies]`, so they compile exactly once.

### Migration order (one PR, staged commits)

Each tier compiles cleanly before the next is started. This makes bisect useful and lets reviewers verify each step.

1. Workspace skeleton: convert root to virtual manifest; `crates/` dir created; old root package kept temporarily as a workspace member.
2. `conduit-util` (leaf).
3. `conduit-types` (with Fix A + B + part of C content extracted in).
4. `conduit-git`.
5. `conduit-agent` (Fix A done, now compiles).
6. `conduit-resolver`, `conduit-session`, `conduit-data` (parallel; each depends on subset above).
7. `conduit-config` (Fix B done, now compiles).
8. `conduit-theme` (completes Fix C).
9. `conduit-core`.
10. `conduit-web` (Fix C done, now compiles).
11. `conduit-ui`.
12. `conduit` umbrella (re-exports + `tests/` moved in).
13. `conduit-bin` (`cargo run` works); old root `src/` deleted; old root `Cargo.toml` becomes pure `[workspace]` virtual manifest.

## Risks / Trade-offs

- **Cycle introduction during inversions** → Each inversion fix is local to one or two files; verify with `cargo check -p <crate>` after each move. The dependency-tier order surfaces cycles at the earliest tier where they appear.
- **`tokio` feature fragmentation** → Declare `tokio` once in `[workspace.dependencies]` with full features; every crate inherits via `tokio = { workspace = true }`. Resist trimming.
- **`codex-protocol` recompiled per crate** → Declare in `[workspace.dependencies]` but only list under `conduit-agent`'s `[dependencies]`. Verify with `cargo tree -p conduit-web` that they're absent there.
- **`rust-embed` path drift** → `#[folder = "web/dist"]` is relative to the crate root containing the deriving struct. After `web/` moves under `crates/conduit-web/`, the literal stays correct — but verify with a full `cargo build` after the move.
- **insta snapshot path drift** → snapshot files embed test module paths (e.g. `crates/conduit-ui/src/components/__snapshots__/...`). When `git mv` relocates test files, snapshots move with them. Run `INSTA_UPDATE=no cargo test --workspace` first to confirm only metadata diffs; accept with `cargo insta review` if needed.
- **e2e shell tests reference `target/debug/conduit`** → The bin name is unchanged, so no edits required. Verify `bash tests/e2e/run_all.sh` passes after the split.
- **`build.rs` rerun-if-changed paths** → After moving to `crates/conduit-web/build.rs`, the `cargo::rerun-if-changed=web/src` directives are now relative to the crate root, which still resolves correctly because `web/` moves with `build.rs`. Verify by touching a frontend source file and confirming a rebuild triggers.
- **CI workflow assumes single-package** → Update `cargo clippy -- -D warnings` to `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test` to `cargo test --workspace` in CI configs (verify by grepping `.github/workflows/*.yml` if present).
- **External tooling assumes root `Cargo.toml` defines a package** → Some tools (`cargo install --path .`, `cargo doc --open`, IDE configs) may need adjustment. Mitigate by keeping `crates/conduit-bin` installable via `cargo install --path crates/conduit-bin`.
- **`mold` not available on macOS / Windows** → `.cargo/config.toml` targets only `x86_64-unknown-linux-gnu`. Other platforms inherit defaults. Document this in the change notes.
- **The biggest cost — `ui` rebuilds — is unchanged** → User accepted this trade explicitly. Splitting `ui/` internals is documented as a future opportunity but out of scope.

## Migration Plan

Single PR, staged commits per tier (see Decisions §"Migration order"). Each commit must pass `cargo check --workspace`. Final commit must pass the full CI gate (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `bash tests/e2e/run_all.sh`).

**Baseline measurement (BEFORE any change, on clean `master`):**
```bash
cargo clean
cargo build --timings 2>&1 | tee baseline-cold.log
touch src/agent/runner.rs && /usr/bin/time -v cargo build 2>&1 | tee baseline-incr-agent.log
touch src/util/mod.rs    && /usr/bin/time -v cargo build 2>&1 | tee baseline-incr-util.log
touch src/web/server.rs  && /usr/bin/time -v cargo build 2>&1 | tee baseline-incr-web.log
touch src/ui/app.rs      && /usr/bin/time -v cargo build 2>&1 | tee baseline-incr-ui.log
```

**Post-split measurement:**
```bash
cargo clean
cargo build --timings 2>&1 | tee split-cold.log
touch crates/conduit-agent/src/runner.rs && /usr/bin/time -v cargo build 2>&1 | tee split-incr-agent.log
touch crates/conduit-util/src/lib.rs     && /usr/bin/time -v cargo build 2>&1 | tee split-incr-util.log
touch crates/conduit-web/src/server.rs   && /usr/bin/time -v cargo build 2>&1 | tee split-incr-web.log
touch crates/conduit-ui/src/app.rs       && /usr/bin/time -v cargo build 2>&1 | tee split-incr-ui.log
```

Compare wall-clock + Cargo timing chart. Expected: cold build similar (or slightly slower from extra crate setup); incremental edits to non-`ui` crates rebuild only that crate plus binary linking, with `conduit-ui` skipped.

**Rollback:** if the PR ships and incremental gains don't materialize, revert the merge commit. Because the public API and binary name are preserved, no consumer or downstream depends on the new crate paths.

## Open Questions

- Should `conduit-types` depend on `conduit-agent`/`conduit-git` (simpler, accepts a small dep direction) or should `Action` be split into a tiny enum in `conduit-types` plus a richer enum in `conduit-ui`/`conduit-config` (more pure, but doubles the type)? Default chosen: the simpler option.
- Whether to publish `conduit-core` to crates.io as a stable embedding interface for third-party tools. Out of scope for this change but enabled by the split.
- Whether to add `sccache` to `.cargo/config.toml` (opt-in via env var). Default: not added; user can enable per-machine.
