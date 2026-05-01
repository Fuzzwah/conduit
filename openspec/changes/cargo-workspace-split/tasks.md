## 1. Baseline measurement (BEFORE any code change)

- [x] 1.1 On a clean checkout of `master`, run `cargo clean && cargo build --timings 2>&1 | tee openspec/changes/cargo-workspace-split/baseline-cold.log` and save `target/cargo-timings/cargo-timing-*.html` to the change folder
- [x] 1.2 Run incremental edits and capture timings: `touch src/agent/runner.rs && /usr/bin/time -v cargo build 2>&1 | tee openspec/changes/cargo-workspace-split/baseline-incr-agent.log`
- [x] 1.3 Repeat 1.2 for `src/util/mod.rs` → `baseline-incr-util.log`
- [x] 1.4 Repeat 1.2 for `src/web/server.rs` → `baseline-incr-web.log`
- [x] 1.5 Repeat 1.2 for `src/ui/app.rs` → `baseline-incr-ui.log`
- [x] 1.6 Reset any `touch`-induced mtime changes (`git checkout -- src/`) before starting the split

**Baseline results:** cold = 1m32s, incr-agent = 10.09s, incr-util = 9.33s, incr-web = 9.14s, incr-ui = 10.20s

## 2. Workspace skeleton

- [ ] 2.1 Create `crates/` directory at repo root
- [ ] 2.2 Convert root `Cargo.toml` to a virtual workspace manifest: remove `[package]`, `[lib]`, `[[bin]]`, `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, `[target.*]`; add `[workspace] resolver = "2", members = ["crates/*", "."]` (root stays a member temporarily so existing `src/` keeps building during migration)
- [ ] 2.3 Add `[workspace.package] version = "0.5.0", edition = "2021", rust-version = "1.87"` and other shared package metadata
- [ ] 2.4 Move every existing `[dependencies]` entry into `[workspace.dependencies]` preserving versions and features (notably `tokio = { version = "1.42", features = ["full", ...] }`, `agent-client-protocol = { ..., features = ["unstable"] }`, `serde = { ..., features = ["derive"] }`, `codex-protocol = { git = "...", tag = "rust-v0.81.0" }`, `codex-app-server-protocol = { git = "...", tag = "rust-v0.81.0" }`)
- [ ] 2.5 Keep a temporary root package manifest fragment in a new `crates/conduit-legacy/Cargo.toml` (or keep root as package one more commit) so existing `src/` still compiles before extraction tiers begin; OR — alternative — keep root `Cargo.toml` as a hybrid (`[workspace]` + `[package]`) until the last extraction tier
- [ ] 2.6 Add the four `[profile.dev]` settings (`split-debuginfo = "unpacked"`, `debug = "line-tables-only"`, `codegen-units = 256`, `incremental = true`) and `[profile.dev.package."*"] opt-level = 0` to root `Cargo.toml`
- [ ] 2.7 Verify `cargo check --workspace` succeeds with the skeleton in place (no source moves yet)
- [ ] 2.8 Commit "chore: convert root to virtual cargo workspace skeleton"

## 3. Extract `conduit-util` (leaf, tier 1) ✓

- [x] 3.1 Create `crates/conduit-util/Cargo.toml` with `name = "conduit-util"`, inherited workspace package metadata, and `[dependencies]` (actual: `dirs`, `rand`, `serde`, `tokio`, `tracing`, `uuid`, `which`, plus cfg-unix `libc`)
- [x] 3.2 `git mv src/util crates/conduit-util/src` then rename `mod.rs` → `lib.rs`
- [x] 3.3 PRE-FIX: `src/util/title_generator.rs` depended on `crate::agent::*`, breaking util's leaf status. Moved to `src/agent/title_generator.rs` instead and updated 2 callers (`src/ui/app.rs`, `src/web/ws/handler.rs`)
- [x] 3.4 Replace `crate::util::process::pid_start_time` → `crate::process::pid_start_time` inside the moved process.rs
- [x] 3.5 In root `src/lib.rs`, replaced `pub mod util;` with `pub use conduit_util as util;`
- [x] 3.6 Added `conduit-util = { path = "crates/conduit-util" }` to root `[dependencies]`
- [x] 3.7 `cargo check --workspace` ✓
- [x] 3.8 Committed "refactor: extract conduit-util crate (tier 1)"

## 4. Extract `conduit-types` (NEW, tier 2 — performs Fix A + part of Fix B + part of Fix C)

- [x] 4.1 Create `crates/conduit-types/Cargo.toml` with `[dependencies]` `serde`, `sha2` (PathBuf is in std; no chrono/crossterm/regex needed for this tier — moved types are pure data)
- [x] 4.2 Strip dead `TurnSummary::render()` and `shorten_filename()` (with their ratatui imports) from `turn_summary.rs` so the type can move into a ratatui-free crate
- [x] 4.3 Move `chat_message.rs`, `turn_summary.rs` into `crates/conduit-types/src/`; rewrite `super::TurnSummary` → `crate::TurnSummary`
- [x] 4.4 Move `app_prompt.rs` into `crates/conduit-types/src/`; rewrite `crate::ui::components::{ChatMessage, MessageRole, TurnSummary}` → `crate::{ChatMessage, MessageRole, TurnSummary}`
- [x] 4.5 Move `src/ui/action.rs` into `crates/conduit-types/src/action.rs` (clean — only `PathBuf`, `serde` deps)
- [x] 4.6 Extract `InputMode` and `ViewMode` enums from `src/ui/events.rs` into `crates/conduit-types/src/input_mode.rs`; leave `AppEvent` and its result structs behind in `src/ui/events.rs` (they reference agent/git types and move with later tiers)
- [x] 4.7 Create `crates/conduit-types/src/lib.rs` with module decls and flat re-exports (`Action`, `ChatMessage`, `MessageRole`, `InputMode`, `ViewMode`, `FileChange`, `TurnSummary`)
- [x] 4.8 Replace `src/ui/action.rs` and `src/ui/app_prompt.rs` with re-export shims (`pub use conduit_types::action::*;` etc.)
- [x] 4.9 Replace InputMode/ViewMode definitions in `src/ui/events.rs` with `pub use conduit_types::{InputMode, ViewMode};`
- [x] 4.10 In `src/ui/components/mod.rs`, drop `mod chat_message;` / `mod turn_summary;` and re-export from conduit-types: `pub use conduit_types::{ChatMessage, MessageRole};` and `pub use conduit_types::{FileChange, TurnSummary};`
- [x] 4.11 Add `conduit-types` to workspace `members` and `[workspace.dependencies]`; add `conduit-types = { path = "crates/conduit-types" }` to root `[dependencies]`
- [x] 4.12 Add `tempfile` as a dev-dependency for `conduit-util` (test in `project_folders.rs` needs it now that the test target is per-crate)
- [x] 4.13 Fix A and partial Fix B/C: existing `crate::ui::components::{ChatMessage,...}` and `crate::ui::action::Action` imports still resolve via the shims; full direct rewrites happen when their containing modules become crates
- [x] 4.14 Verify: `cargo check`, `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` all pass

## 5. Extract `conduit-git` (tier 3) ✓

- [x] 5.1 Create `crates/conduit-git/Cargo.toml` with `[dependencies]` `serde`, `serde_json`, `thiserror`, `tracing` (no `tokio`/`anyhow`/`chrono` needed — git uses sync `std::process::Command`); `tempfile` as dev-dep. No `conduit-util` dep — git is a true leaf
- [x] 5.2 `git mv src/git crates/conduit-git/src` then `mv mod.rs → lib.rs`
- [x] 5.3 Rewrite `crate::git::worktree::*` → `crate::worktree::*` and `crate::git::{...}` → `crate::{...}` in `workspace_repo.rs` (only file with internal cross-module refs)
- [x] 5.4 In root `src/lib.rs`, replace `pub mod git;` with `pub use conduit_git as git;`
- [x] 5.5 Add `conduit-git = { path = "crates/conduit-git" }` to root `[dependencies]` and to workspace `members`
- [x] 5.6 Verify CI gate: `cargo check`, `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` all pass

## 6. Extract `conduit-agent` (tier 4 — completes Fix A path) ✓

- [x] 6.1 Create `crates/conduit-agent/Cargo.toml` with `[dependencies]`: `tokio`, `tokio-util`, `futures`, `async-trait`, `serde`, `serde_json`, `agent-client-protocol`, `codex-protocol`, `codex-app-server-protocol`, `chrono`, `uuid`, `anyhow`, `thiserror`, `tracing`, `reqwest`, `reqwest-eventsource`, `dirs`, `image`, `parking_lot`, `which`, `libc` cfg(unix), plus `conduit-util`, `conduit-types`. Dev-dep: `tempfile`
- [x] 6.2 `git mv src/agent crates/conduit-agent/src` then rename `mod.rs` → `lib.rs`
- [x] 6.3 Replace `crate::agent::X` with `crate::X`, `crate::util::X` with `conduit_util::X`, `crate::command_resolver::SkillReference` with `conduit_types::SkillReference` (extracted to break agent ↔ resolver cycle) — no `crate::ui::` remains
- [x] 6.4 In root `src/lib.rs`, replace `pub mod agent;` with `pub use conduit_agent as agent;` (and ensure all current `pub use agent::{...}` lines still resolve)
- [x] 6.5 Add `conduit-agent = { path = "crates/conduit-agent" }` to temporary root `[dependencies]`
- [x] 6.6 Fix doctest paths in `mock.rs` to use `conduit_agent::*` instead of `conduit::agent::*`
- [x] 6.7 Verify CI gate: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` all pass

## 7. Extract `conduit-resolver`, `conduit-session`, `conduit-data` (tier 5, parallel-safe)

- [ ] 7.1 Create `crates/conduit-resolver/Cargo.toml` with deps `serde`, `serde_json`, `regex`, `anyhow`, `conduit-util`, `conduit-agent`
- [ ] 7.2 `git mv src/command_resolver.rs crates/conduit-resolver/src/lib.rs`. Rewrite `crate::agent::X` → `conduit_agent::X`. Rename internal modules if needed
- [ ] 7.3 In root `src/lib.rs`, replace `pub mod command_resolver;` with `pub use conduit_resolver as command_resolver;`
- [ ] 7.4 Add path dep, verify `cargo check --workspace`
- [ ] 7.5 Create `crates/conduit-session/Cargo.toml` with deps `serde`, `serde_json`, `chrono`, `tracing`, `dirs`, `conduit-util`, `conduit-agent`
- [ ] 7.6 `git mv src/session crates/conduit-session/src` then rename `mod.rs` → `lib.rs`. Rewrite cross-module imports
- [ ] 7.7 Update root `src/lib.rs` re-export, add path dep, verify `cargo check --workspace`
- [ ] 7.8 Create `crates/conduit-data/Cargo.toml` with deps `rusqlite` (with `bundled` feature), `serde`, `serde_json`, `chrono`, `uuid`, `anyhow`, `tracing`, `conduit-util`, `conduit-agent`, `conduit-git`
- [ ] 7.9 `git mv src/data crates/conduit-data/src` then rename `mod.rs` → `lib.rs`. Rewrite cross-module imports
- [ ] 7.10 Update root `src/lib.rs` re-export, add path dep, verify `cargo check --workspace`
- [ ] 7.11 Commit "refactor: extract conduit-resolver, conduit-session, conduit-data crates"

## 8. Move `Action`/`events` into `conduit-types` and extract `conduit-config` (tier 6 — completes Fix B)

- [ ] 8.1 In `crates/conduit-types/Cargo.toml`, add `conduit-agent = { path = "../conduit-agent" }` and `conduit-git = { path = "../conduit-git" }` to `[dependencies]`
- [ ] 8.2 Move `src/ui/action.rs` into `crates/conduit-types/src/action.rs` and `src/ui/events.rs` into `crates/conduit-types/src/events.rs`. Move `src/ui/git_tracker.rs`'s `GitTrackerUpdate` enum (only the enum, not the rest of the module if larger) into `crates/conduit-types/src/git_tracker_update.rs`
- [ ] 8.3 In `crates/conduit-types/src/lib.rs` add `pub mod action; pub mod events; pub mod git_tracker_update;` plus flat re-exports if convenient (`pub use action::Action;` etc.)
- [ ] 8.4 Update imports inside the moved files: `crate::agent::AgentEvent` → `conduit_agent::AgentEvent`, `crate::git::GithubIssue` → `conduit_git::GithubIssue`, etc.
- [ ] 8.5 In `src/ui/mod.rs`, add re-export shims so existing `ui::action::Action`, `ui::events::InputMode`, `ui::git_tracker::GitTrackerUpdate` all still resolve: `pub mod action { pub use conduit_types::action::*; }` (or equivalent)
- [ ] 8.6 Update `src/config/keys.rs:13,190,704,750,758`, `src/config/default_keys.rs:11`, `src/config/settings.rs:10` to import from `conduit_types` instead of `crate::ui` (Fix B)
- [ ] 8.7 Create `crates/conduit-config/Cargo.toml` with deps `serde`, `toml`, `toml_edit`, `json5`, `dirs`, `crossterm`, `anyhow`, `conduit-util`, `conduit-types`, `conduit-agent`, `conduit-git`
- [ ] 8.8 `git mv src/config crates/conduit-config/src` then rename `mod.rs` → `lib.rs`. Rewrite remaining `crate::*` imports
- [ ] 8.9 In root `src/lib.rs`, replace `pub mod config;` with `pub use conduit_config as config;`
- [ ] 8.10 Add path dep, verify `cargo check --workspace`
- [ ] 8.11 Commit "refactor: extract conduit-config crate (Fix B complete)"

## 9. Extract `conduit-theme` (tier 7 — completes Fix C)

- [ ] 9.1 Create `crates/conduit-theme/Cargo.toml` with deps `serde`, `serde_json`, `toml`, `toml_edit`, `dirs`, `anyhow`, `parking_lot`, `regex`, `tracing`, `conduit-util`, `conduit-types` (NOT `ratatui`, NOT `syntect`)
- [ ] 9.2 `git mv src/ui/components/theme crates/conduit-theme/src` then rename `mod.rs` → `lib.rs`. (If theme code references `ratatui::style::Color` or similar, keep that as a `ratatui` dep — but verify whether it's just for color conversion that could move elsewhere; the goal is to keep `conduit-theme` light enough that `web` doesn't transitively pull `ratatui`)
- [ ] 9.3 Rewrite imports inside moved files (`crate::ui::components::*` → split appropriately; rusqlite-backed persistence may need access to conduit-data later — for now keep theme persistence functions self-contained and pass `&Database` as a generic parameter or callback if needed)
- [ ] 9.4 In `src/ui/components/mod.rs`, add `pub use conduit_theme as theme;` so all in-tree `ui::components::theme::Theme` callers keep working
- [ ] 9.5 Update `src/web/handlers/themes.rs:7,361` and `src/web/handlers/settings.rs:9` to import from `conduit_theme` (completes Fix C)
- [ ] 9.6 Verify `cargo tree -p conduit-theme` does NOT include `ratatui`. If it does, identify the offending import and either move it to `conduit-ui` or create a separate `conduit-theme-ratatui` adapter crate
- [ ] 9.7 Commit "refactor: extract conduit-theme crate (Fix C complete)"

## 10. Extract `conduit-core` (tier 8)

- [ ] 10.1 Create `crates/conduit-core/Cargo.toml` with deps `tokio`, `parking_lot`, `anyhow`, `tracing`, `conduit-util`, `conduit-agent`, `conduit-config`, `conduit-data`, `conduit-git`
- [ ] 10.2 `git mv src/core crates/conduit-core/src` then rename `mod.rs` → `lib.rs`. Rewrite `crate::*` imports to per-crate paths
- [ ] 10.3 In root `src/lib.rs`, replace `pub mod core;` with `pub use conduit_core as core;`
- [ ] 10.4 Add path dep, verify `cargo check --workspace`
- [ ] 10.5 Commit "refactor: extract conduit-core crate"

## 11. Extract `conduit-web` (tier 9 — verifies Fix C)

- [ ] 11.1 Create `crates/conduit-web/Cargo.toml` with deps `axum`, `axum-extra`, `tower`, `tower-http`, `rust-embed`, `mime_guess`, `reqwest`, `serde`, `serde_json`, `tokio`, `tracing`, `chrono`, `anyhow`, plus internal deps `conduit-util`, `conduit-types`, `conduit-agent`, `conduit-config`, `conduit-data`, `conduit-git`, `conduit-session`, `conduit-resolver`, `conduit-core`, `conduit-theme`. Also add `[build-dependencies] which = { workspace = true }`
- [ ] 11.2 `git mv src/web crates/conduit-web/src` then rename `mod.rs` → `lib.rs`. Rewrite `crate::*` imports
- [ ] 11.3 `git mv web crates/conduit-web/web` (the React frontend directory)
- [ ] 11.4 `git mv build.rs crates/conduit-web/build.rs`. Verify the `cargo::rerun-if-changed=web/src` directives still resolve relative to the new crate root
- [ ] 11.5 Verify `crates/conduit-web/src/routes/static_files.rs:13` still has `#[folder = "web/dist"]` and that the path resolves correctly
- [ ] 11.6 Verify `cargo build -p conduit-web` triggers the npm build and embeds assets correctly
- [ ] 11.7 Verify `cargo tree -p conduit-web` does NOT include `ratatui`, `syntect`, `tui-markdown`, or `arboard`
- [ ] 11.8 In root `src/lib.rs`, replace `pub mod web;` with `pub use conduit_web as web;`
- [ ] 11.9 Add path dep, verify `cargo check --workspace`
- [ ] 11.10 Commit "refactor: extract conduit-web crate, move web/ and build.rs"

## 12. Extract `conduit-ui` (tier 10)

- [ ] 12.1 Create `crates/conduit-ui/Cargo.toml` with deps `ratatui`, `crossterm`, `tui-markdown`, `pulldown-cmark`, `syntect`, `two-face`, `unicode-width`, `ansi-to-tui`, `arboard`, `image`, `tempfile`, `base64`, `tokio`, `serde_json`, `regex`, `tracing`, plus internal deps `conduit-util`, `conduit-types`, `conduit-agent`, `conduit-config`, `conduit-data`, `conduit-git`, `conduit-session`, `conduit-resolver`, `conduit-core`, `conduit-theme`
- [ ] 12.2 `git mv src/ui crates/conduit-ui/src` then rename `mod.rs` → `lib.rs`. Rewrite all `crate::*` imports to per-crate paths
- [ ] 12.3 Verify the re-export shims for `action`, `events`, `app_prompt`, `components::theme`, `components::{ChatMessage, ...}` are present at the right paths in `crates/conduit-ui/src/`
- [ ] 12.4 Verify `cargo check -p conduit-ui && cargo check --workspace`
- [ ] 12.5 Commit "refactor: extract conduit-ui crate"

## 13. Create `conduit` umbrella crate and move `tests/`

- [ ] 13.1 Create `crates/conduit/Cargo.toml` (lib only, name `conduit`) with internal deps on every workspace member except `conduit-bin`
- [ ] 13.2 Create `crates/conduit/src/lib.rs` containing only `pub use` re-exports mirroring the pre-split `src/lib.rs:12-35` (the `pub use agent::{...}`, `pub use config::Config`, `pub use ui::App`, etc.). Use full paths (`pub use conduit_agent::{AgentError, AgentEvent, ...}`)
- [ ] 13.3 `git mv tests crates/conduit/tests`. Verify integration tests under `crates/conduit/tests/integration/*.rs` still compile against the umbrella's re-exports
- [ ] 13.4 Move `tests/fixtures/`, `tests/common/`, `tests/e2e/` along with the integration tests
- [ ] 13.5 Verify `cargo test -p conduit` runs the integration tests and they pass
- [ ] 13.6 Verify `bash crates/conduit/tests/e2e/run_all.sh` (path may need adjusting) still works against `target/debug/conduit`
- [ ] 13.7 Commit "refactor: add conduit umbrella crate, relocate tests"

## 14. Extract `conduit-bin` and finalise root manifest

- [ ] 14.1 Create `crates/conduit-bin/Cargo.toml` with `[[bin]] name = "conduit"`, deps on `conduit` umbrella + `clap`, `tokio`, `anyhow`, `tracing-subscriber`, `crossterm`, `ratatui` (anything `main.rs` imports directly)
- [ ] 14.2 `git mv src/main.rs crates/conduit-bin/src/main.rs`. Update its imports (`use conduit::{...}` should mostly Just Work via the umbrella)
- [ ] 14.3 Delete the now-empty `src/` directory and any leftover root `src/lib.rs`
- [ ] 14.4 Remove the temporary root `[package]` / `[lib]` / `[[bin]]` / `[dependencies]` blocks from root `Cargo.toml`. Update `[workspace] members` to `["crates/*"]` only (drop the `"."` entry)
- [ ] 14.5 Verify `cargo build --workspace` produces `target/debug/conduit`
- [ ] 14.6 Verify `./target/debug/conduit --help` matches pre-split help output (subcommands and flags)
- [ ] 14.7 Verify `cargo install --path crates/conduit-bin --locked --force` installs successfully
- [ ] 14.8 Commit "refactor: extract conduit-bin, remove legacy root src/"

## 15. Linker config and dependency hygiene

- [ ] 15.1 Create `.cargo/config.toml` with the `[target.x86_64-unknown-linux-gnu]` block (`linker = "clang"`, `rustflags = ["-C", "link-arg=-fuse-ld=mold"]`)
- [ ] 15.2 Document `mold` + `clang` install command in `crates/conduit-bin/README.md` or top-level `AGENTS.md` (`apt install mold clang`)
- [ ] 15.3 Run `cargo tree --duplicates --workspace`. Resolve any duplicate-version warnings by aligning versions in `[workspace.dependencies]`
- [ ] 15.4 Run `cargo tree -p conduit-web` and `cargo tree -p conduit-ui`; verify `codex-protocol`, `codex-app-server-protocol`, `agent-client-protocol` appear in NEITHER (they should only appear under `conduit-agent`)
- [ ] 15.5 Run `cargo tree -p conduit-web`; verify `ratatui`, `syntect`, `two-face`, `tui-markdown`, `arboard` appear in NONE of its dependencies
- [ ] 15.6 Commit "build: add mold linker config and verify dep tree hygiene"

## 16. CI workflow updates

- [ ] 16.1 Search `.github/workflows/*.yml` (or equivalent) for `cargo clippy`, `cargo test`, `cargo build` invocations
- [ ] 16.2 Update each to its workspace form: `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --workspace`
- [ ] 16.3 Verify the workflow file is syntactically valid (push to a draft PR if necessary to confirm CI runs)
- [ ] 16.4 Commit "ci: switch to --workspace cargo invocations"

## 17. Post-split measurement and verification

- [ ] 17.1 Run `cargo clean && cargo build --timings 2>&1 | tee openspec/changes/cargo-workspace-split/split-cold.log`
- [ ] 17.2 Run incremental edits and capture: `touch crates/conduit-agent/src/runner.rs && /usr/bin/time -v cargo build 2>&1 | tee openspec/changes/cargo-workspace-split/split-incr-agent.log`
- [ ] 17.3 Repeat 17.2 for `crates/conduit-util/src/lib.rs`, `crates/conduit-web/src/server.rs`, `crates/conduit-ui/src/app.rs`
- [ ] 17.4 Reset any `touch`-induced changes (`git checkout -- crates/`)
- [ ] 17.5 Create `openspec/changes/cargo-workspace-split/measurements.md` summarising baseline vs post-split wall-clock times and noting the expected `conduit-ui` skip behaviour for non-`ui` edits (verify with `cargo build -v 2>&1 | grep "Compiling conduit-ui"` returning empty)
- [ ] 17.6 Run the full CI gate: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
- [ ] 17.7 Run `bash crates/conduit/tests/e2e/run_all.sh` and confirm it passes
- [ ] 17.8 Run `cargo run -- demo --clean` and confirm the TUI launches cleanly
- [ ] 17.9 Run `cargo run -- serve` and confirm `http://127.0.0.1:3000` serves the embedded React UI without 404s
- [ ] 17.10 If any insta snapshots changed due to module-path differences, run `cargo insta review` and accept if the diffs are metadata-only
- [ ] 17.11 Commit "docs: add baseline vs post-split build measurements"

## 18. PR preparation

- [ ] 18.1 Squash or rebase the staged commits into a logical sequence preserving the per-tier checkpoints (or leave individual commits if reviewers prefer bisectability)
- [ ] 18.2 Push the branch and open a PR against `Fuzzwah/conduit:master` per AGENTS.md (`gh pr create --repo Fuzzwah/conduit --base master --head "$(git branch --show-current)"`) using a body file (not inline) for the PR description
- [ ] 18.3 Include the measurements summary in the PR body so reviewers see the win
- [ ] 18.4 Wait for CI to pass; address any clippy / test failures
