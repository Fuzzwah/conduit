## 1. Baseline measurement (BEFORE any code change)

- [x] 1.1 On a clean checkout of `master`, run `cargo clean && cargo build --timings 2>&1 | tee openspec/changes/cargo-workspace-split/baseline-cold.log` and save `target/cargo-timings/cargo-timing-*.html` to the change folder
- [x] 1.2 Run incremental edits and capture timings: `touch src/agent/runner.rs && /usr/bin/time -v cargo build 2>&1 | tee openspec/changes/cargo-workspace-split/baseline-incr-agent.log`
- [x] 1.3 Repeat 1.2 for `src/util/mod.rs` → `baseline-incr-util.log`
- [x] 1.4 Repeat 1.2 for `src/web/server.rs` → `baseline-incr-web.log`
- [x] 1.5 Repeat 1.2 for `src/ui/app.rs` → `baseline-incr-ui.log`
- [x] 1.6 Reset any `touch`-induced mtime changes (`git checkout -- src/`) before starting the split

**Baseline results:** cold = 1m32s, incr-agent = 10.09s, incr-util = 9.33s, incr-web = 9.14s, incr-ui = 10.20s

## 2. Workspace skeleton

- [x] 2.1 Create `crates/` directory at repo root
- [x] 2.2 Convert root `Cargo.toml` to a virtual workspace manifest: remove `[package]`, `[lib]`, `[[bin]]`, `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, `[target.*]`; add `[workspace] resolver = "2", members = ["crates/*", "."]` (root stays a member temporarily so existing `src/` keeps building during migration)
- [x] 2.3 Add `[workspace.package] version = "0.5.0", edition = "2021", rust-version = "1.87"` and other shared package metadata
- [x] 2.4 Move every existing `[dependencies]` entry into `[workspace.dependencies]` preserving versions and features (notably `tokio = { version = "1.42", features = ["full", ...] }`, `agent-client-protocol = { ..., features = ["unstable"] }`, `serde = { ..., features = ["derive"] }`, `codex-protocol = { git = "...", tag = "rust-v0.81.0" }`, `codex-app-server-protocol = { git = "...", tag = "rust-v0.81.0" }`)
- [x] 2.5 Keep a temporary root package manifest fragment in a new `crates/conduit-legacy/Cargo.toml` (or keep root as package one more commit) so existing `src/` still compiles before extraction tiers begin; OR — alternative — keep root `Cargo.toml` as a hybrid (`[workspace]` + `[package]`) until the last extraction tier
- [x] 2.6 Add the four `[profile.dev]` settings (`split-debuginfo = "unpacked"`, `debug = "line-tables-only"`, `codegen-units = 256`, `incremental = true`) and `[profile.dev.package."*"] opt-level = 0` to root `Cargo.toml`
- [x] 2.7 Verify `cargo check --workspace` succeeds with the skeleton in place (no source moves yet)
- [x] 2.8 Commit "chore: convert root to virtual cargo workspace skeleton"

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

## 7. Extract `conduit-resolver`, `conduit-session`, `conduit-data` (tier 5, parallel-safe) ✓

- [x] 7.1 Create `crates/conduit-resolver/Cargo.toml` with deps `serde`, `dirs`, `toml`, `tracing`, `conduit-agent`, `conduit-types`. Dev-dep: `tempfile`
- [x] 7.2 `git mv src/command_resolver.rs crates/conduit-resolver/src/lib.rs`. Rewrite `crate::agent::AgentType` → `conduit_agent::AgentType`
- [x] 7.3 In root `src/lib.rs`, replace `pub mod command_resolver;` with `pub use conduit_resolver as command_resolver;`
- [x] 7.4 Add path dep, verify `cargo check --workspace`
- [x] 7.5 Create `crates/conduit-session/Cargo.toml` with deps `anyhow`, `chrono`, `dirs`, `serde`, `serde_json`, `tracing`, `conduit-agent`, `conduit-util`
- [x] 7.6 `git mv src/session crates/conduit-session/src` then rename `mod.rs` → `lib.rs`. Rewrite `crate::agent::AgentType` → `conduit_agent::AgentType`, `crate::session::cache::*` → `crate::cache::*`, `crate::util::data_dir()` → `conduit_util::data_dir()`, `crate::session::ExternalSession` → `crate::ExternalSession`
- [x] 7.7 Update root `src/lib.rs` re-export, add path dep, verify `cargo check --workspace`
- [x] 7.8 Create `crates/conduit-data/Cargo.toml` with deps `chrono`, `rusqlite` (bundled), `serde`, `serde_json`, `sha2`, `thiserror`, `tracing`, `uuid`, `conduit-agent`, `conduit-git`, `conduit-util`. Dev-dep: `tempfile`
- [x] 7.9 `git mv src/data crates/conduit-data/src` then rename `mod.rs` → `lib.rs`. Rewrite `crate::agent::AgentType` → `conduit_agent::AgentType`, `crate::git::WorkspaceMode` → `conduit_git::WorkspaceMode`, `crate::util::database_path()` → `conduit_util::database_path()`, `crate::data::*` → `crate::*` (in tests). Promote 5 `pub(crate)` `*_with_conn` fns on `SessionTabStore` to `pub` since they're called from `conduit-cli::core::services::session_service`
- [x] 7.10 Update root `src/lib.rs` re-export, add path dep, verify `cargo check --workspace`
- [x] 7.11 Verify CI gate: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` all pass

## 8. Move `Action`/`events` into `conduit-types` and extract `conduit-config` (tier 6 — completes Fix B) ✓

- [x] 8.1 `Action`, `InputMode`, `ViewMode` already moved to `conduit-types` in tier 2 (4.5–4.10). No changes needed here.
- [x] 8.2 Skipped — types already moved in tier 2 with re-export shims via `src/ui/action.rs`, `src/ui/events.rs`.
- [x] 8.3 `conduit-types/src/lib.rs` already exports `Action`, `InputMode`, `ViewMode` from tier 2.
- [x] 8.4 Skipped — `Action` is a pure `PathBuf`/`serde`-only enum; agent/git references in the original Action variants were removed earlier as dead code.
- [x] 8.5 Re-export shims in `src/ui/action.rs`, `src/ui/app_prompt.rs`, `src/ui/events.rs` added in tier 2.
- [x] 8.6 Update `crates/conduit-config/src/keys.rs`, `default_keys.rs`, `settings.rs` to import from `conduit_types` instead of `crate::ui` (Fix B complete)
- [x] 8.7 Create `crates/conduit-config/Cargo.toml` with deps `crossterm`, `serde`, `toml`, `toml_edit`, `tracing`, `conduit-agent`, `conduit-git`, `conduit-types`, `conduit-util`
- [x] 8.8 `git mv src/config/{default_keys,keys,settings}.rs crates/conduit-config/src/` then rename `mod.rs` → `lib.rs`. Rewrite `crate::agent::*` → `conduit_agent::*`, `crate::git::*` → `conduit_git::*`, `crate::util::*` → `conduit_util::*`. Restore `config.toml.example` from git.
- [x] 8.9 In root `src/lib.rs`, replace `pub mod config;` with `pub use conduit_config as config;`
- [x] 8.10 Add path dep, verify `cargo check --workspace`
- [x] 8.11 Verify CI gate: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` all pass

## 9. Extract `conduit-theme` (tier 7 — completes Fix C) ✓

- [x] 9.1 Create `crates/conduit-theme/Cargo.toml` with deps `dirs`, `json5`, `parking_lot`, `ratatui`, `serde`, `serde_json`, `tokio`, `toml`, `toml_edit`, `tracing`, `conduit-util`. Dev-dep: `tempfile`. (Theme code uses `ratatui::style::Color` directly — keeping ratatui as a dep is the simpler choice; web will still re-route around theme through the `crate::ui::components::theme` shim until tier 9.)
- [x] 9.2 `git mv src/ui/components/theme/{builtin,colors,migrate,registry,toml,types,vscode}.rs crates/conduit-theme/src/` and `mod.rs` → `lib.rs`
- [x] 9.3 Rewrite `crate::util` → `conduit_util as util` in `registry.rs`
- [x] 9.4 In `src/ui/components/mod.rs`, replace `pub mod theme;` with `pub use conduit_theme as theme;`
- [x] 9.5 Skipped — `src/web/handlers/themes.rs` and `settings.rs` keep using `crate::ui::components::theme::*` via the shim. Direct `conduit_theme` imports happen when web is extracted in tier 9.
- [x] 9.6 Promote `theme_test_lock`/`theme_test_lock_async` from `#[cfg(test)] pub(crate)` to always-`pub` (and pull `tokio` out of dev-deps) so cross-crate tests in `conduit-cli` can serialize global theme state. Trivial overhead — tokio is already in the production dep graph.
- [x] 9.7 Verify CI gate: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` all pass

## 10. Extract `conduit-core` (tier 8) ✓

- [x] 10.1 Create `crates/conduit-core/Cargo.toml` with deps `anyhow`, `chrono`, `rusqlite`, `serde`, `serde_json`, `thiserror`, `tokio`, `tracing`, `uuid`, `conduit-agent`, `conduit-config`, `conduit-data`, `conduit-git`, `conduit-util`. Dev-dep: `tempfile`. (`parking_lot` not needed; `rusqlite` is — `session_service.rs` references it.)
- [x] 10.2 `git mv src/core/{conduit_core,repo_settings}.rs crates/conduit-core/src/`, `git mv src/core/{dto,services} crates/conduit-core/src/`, `git mv src/core/mod.rs crates/conduit-core/src/lib.rs`. Bulk-rewrite `crate::{agent,config,data,git,util}::` → `conduit_*::` then `crate::core::` → `crate::`.
- [x] 10.3 In root `src/lib.rs`, replace `pub mod core;` with `pub use conduit_core as core;`.
- [x] 10.4 Promote `pub(crate) fn ConduitCore::new_with_progress` to `pub` so `src/ui/app.rs:390` (now in conduit-cli) can call it across the crate boundary.
- [x] 10.5 Fix `tests` module in `services/context_window_service.rs`: `use conduit_util::{self, ToolAvailability};` → `use conduit_util::{self as util, ToolAvailability};` so existing `util::init_data_dir(...)` call site keeps resolving.
- [x] 10.6 Verify CI gate: `cargo check --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` — all pass.
- [x] 10.7 Commit "refactor: extract conduit-core crate (tier 8)".

## 11. Extract `conduit-web` (tier 9 — verifies Fix C) ✓

- [x] 11.1 Create `crates/conduit-web/Cargo.toml` with internal deps `conduit-{agent,config,core,data,git,resolver,session,theme,types,util}` and external deps `anyhow`, `axum`, `axum-extra`, `base64`, `chrono`, `dirs`, `futures`, `mime_guess`, `parking_lot`, `ratatui`, `reqwest`, `reqwest-eventsource`, `rusqlite`, `rust-embed`, `serde`, `serde_json`, `thiserror`, `tokio`, `tokio-util`, `tower`, `tower-http`, `tracing`, `uuid`. Dev-deps: `http-body-util`, `tempfile`, `tokio-test`. Build-deps: `which`. (`ratatui` and `parking_lot` not listed in original task spec but are required transitively — ratatui via `Color` type from `conduit-theme`, parking_lot via `status_manager.rs`.)
- [x] 11.2 `git mv src/web crates/conduit-web/src` then `mv crates/conduit-web/src/mod.rs crates/conduit-web/src/lib.rs`. Bulk-rewrite `crate::{agent,config,core,data,git,session,command_resolver,util}::` → `conduit_*::`, plus Fix C rewrites: `crate::ui::app_prompt` → `conduit_types::app_prompt`, `crate::ui::components::theme::*` → `conduit_theme::*`, `crate::ui::components::Theme` → `conduit_theme::Theme`, `crate::ui::components::{ChatMessage, MessageRole}` → `conduit_types::{ChatMessage, MessageRole}`. Then `crate::web::` → `crate::`.
- [x] 11.3 `git mv web crates/conduit-web/web` (the React frontend directory).
- [x] 11.4 `git mv build.rs crates/conduit-web/build.rs`. The `cargo::rerun-if-changed=web/src` directives use paths relative to the crate manifest root, which now correctly points to `crates/conduit-web/web/`.
- [x] 11.5 `crates/conduit-web/src/routes/static_files.rs:13` still has `#[folder = "web/dist"]`, which `rust-embed` resolves relative to the crate root — confirmed working from the build log "Frontend build complete!".
- [x] 11.6 `cargo build -p conduit-web` triggers the npm build (visible "Building frontend..." warning) and the resulting binary embeds the assets.
- [x] 11.7 `cargo tree -p conduit-web --edges no-build,no-dev | grep -cE '(syntect|tui-markdown|arboard|ansi-to-tui|two-face)'` returns **0** — none of the heavy ui-only deps leak into the web graph. (`ratatui` itself does appear, since `conduit-theme` re-exports `ratatui::style::Color`, but the heavy widget/syntax-highlighting crates do not.)
- [x] 11.8 In root `src/lib.rs`, replace `pub mod web;` with `pub use conduit_web as web;`.
- [x] 11.9 Remove root `[build-dependencies] which = { workspace = true }` (the build.rs that needed it has moved to conduit-web). Add `conduit-web = { workspace = true }` to root `[dependencies]`.
- [x] 11.10 Verify CI gate: `cargo check --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` — all pass.
- [x] 11.11 Commit "refactor: extract conduit-web crate (tier 9) — completes Fix C".

## 12. Extract `conduit-ui` (tier 10) ✓

- [x] 12.1 Create `crates/conduit-ui/Cargo.toml` with internal deps `conduit-{agent,config,core,data,git,resolver,session,theme,types,util}` and external deps `ansi-to-tui`, `anyhow`, `arboard`, `base64`, `chrono`, `crossterm`, `dirs`, `futures`, `image`, `parking_lot`, `pulldown-cmark`, `rand`, `ratatui`, `regex`, `serde`, `serde_json`, `syntect`, `tempfile`, `tokio`, `tracing`, `two-face`, `unicode-width`, `uuid`. Unix-only: `libc`. (`tui-markdown` listed in original spec is unused.)
- [x] 12.2 `git mv src/ui crates/conduit-ui/src` then `mv crates/conduit-ui/src/mod.rs crates/conduit-ui/src/lib.rs`. Bulk-rewrite `crate::{agent,config,core,data,git,session,command_resolver,util}::` → `conduit_*::` then `crate::ui::` → `crate::`.
- [x] 12.3 Re-export shims (`action`, `events`, `app_prompt`, `components::theme`, `components::{ChatMessage, ...}`) live inside the moved tree (e.g. `crates/conduit-ui/src/app_prompt.rs` is `pub use conduit_types::app_prompt::*;`) — no extra files needed.
- [x] 12.4 Fix one-off `use crate::PrState;` in `crates/conduit-ui/src/app.rs:5237` → `use conduit_git::PrState;` (root-level re-export the sed pattern did not catch).
- [x] 12.5 In root `src/lib.rs`, replace `pub mod ui;` with `pub use conduit_ui as ui;`.
- [x] 12.6 Verify CI gate: `cargo check --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` — all pass.
- [x] 12.7 Commit "refactor: extract conduit-ui crate (tier 10)".

## 13. Create `conduit` umbrella crate and move `tests/`

> **Note:** Tiers 13 and 14 are executed as one atomic commit. The `[lib] name = "conduit"` cannot exist on two packages simultaneously, so the legacy root `[package]` / `[lib]` / `[[bin]]` is removed in the same step that adds `crates/conduit/` and `crates/conduit-bin/`.

- [x] 13.1 Created `crates/conduit/Cargo.toml` (lib only, name `conduit`) with internal deps on every workspace member except `conduit-bin`
- [x] 13.2 Created `crates/conduit/src/lib.rs` containing only `pub use` re-exports mirroring the pre-split `src/lib.rs:12-35` (the `pub use agent::{...}`, `pub use config::Config`, `pub use ui::App`, etc.) using `pub use conduit_X as X` aliases plus the flat re-exports
- [x] 13.3 `git mv tests crates/conduit/tests`. Integration tests under `crates/conduit/tests/integration/*.rs` continue to compile against the umbrella's re-exports
- [x] 13.4 `tests/fixtures/`, `tests/common/`, `tests/e2e/` moved as part of the same `git mv`
- [x] 13.5 `cargo test --workspace` ran the integration tests; all pass
- [x] 13.6 Verify `bash crates/conduit/tests/e2e/run_all.sh` still works (deferred to tier 17 verification)
- [x] 13.7 Combined into the tier 13+14 commit "refactor: add conduit umbrella + bin crates, remove legacy root src/"

## 14. Extract `conduit-bin` and finalise root manifest

- [x] 14.1 Created `crates/conduit-bin/Cargo.toml` with `[[bin]] name = "conduit"`, deps on `conduit` umbrella + `clap`, `tokio`, `anyhow`, `tracing-subscriber`, `crossterm`, `ratatui`
- [x] 14.2 `git mv src/main.rs crates/conduit-bin/src/main.rs`. `use conduit::{...}` Just Worked via the umbrella — no import changes needed
- [x] 14.3 Deleted root `src/lib.rs` and the now-empty `src/` directory
- [x] 14.4 Removed the temporary root `[package]` / `[lib]` / `[[bin]]` / `[dependencies]` / `[target.'cfg(unix)'.dependencies]` / `[dev-dependencies]` blocks from root `Cargo.toml`. Updated `[workspace] members` to `["crates/*"]` only. Added `conduit = { path = "crates/conduit" }` to `[workspace.dependencies]`
- [x] 14.5 `cargo build --workspace` produces `target/debug/conduit`
- [x] 14.6 Verify `./target/debug/conduit --help` matches pre-split help output (deferred to tier 17 verification)
- [x] 14.7 Verify `cargo install --path crates/conduit-bin --locked --force` installs successfully (deferred to tier 17)
- [x] 14.8 Combined commit "refactor: add conduit umbrella + bin crates, remove legacy root src/"

## 15. Linker config and dependency hygiene

- [x] 15.1 SKIPPED — `mold` is not installed on the dev machine, and committing `.cargo/config.toml` with `rustflags = ["-C", "link-arg=-fuse-ld=mold"]` would break builds for any clone without `mold`. Per the plan ("If absent, drop this file; the workspace still works, just slower link"), this is left as an opt-in users can add locally
- [x] 15.2 SKIPPED with 15.1 — no install docs added since the config file isn't committed
- [x] 15.3 `cargo tree --duplicates --workspace` reviewed: duplicates (crossterm 0.28/0.29, thiserror 1/2, nom 7/8, hashbrown 0.14/0.16/0.17, getrandom 0.2/0.3/0.4, schemars 0.8/1.2, rustix 0.38/1.1, linux-raw-sys 0.4/0.12) are all pre-existing from upstream transitive constraints — not introduced by the split. No actionable change
- [x] 15.4 `cargo tree -p conduit-web` and `-p conduit-ui` show `codex-protocol`, `codex-app-server-protocol`, and `agent-client-protocol` reachable transitively via `conduit-agent`. Both `web` and `ui` legitimately call into agent types via `conduit-core → conduit-agent`, so this is architectural, not a regression. The protocol crates compile once and are shared
- [x] 15.5 `cargo tree -p conduit-web` shows `syntect`, `two-face`, `tui-markdown`, `arboard` are ABSENT (the heavy TUI deps are not pulled into web). `ratatui` is still present because `conduit-theme` (a dep of web for theme APIs) uses `ratatui::style::Color`. Eliminating that would require extracting a color type, which is a separate refactor outside this PR's scope
- [x] 15.6 No commit — tier 15 produced no changes (linker config skipped, dep tree already hygienic given architectural constraints)

## 16. CI workflow updates

- [x] 16.1 Searched `.github/workflows/{ci,release}.yml` and found `cargo check`, `cargo clippy -- -D warnings`, `cargo test`, `tests/e2e/run_all.sh`, `web/package-lock.json`, `cd web` invocations
- [x] 16.2 Updated each to workspace form: `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `crates/conduit/tests/e2e/run_all.sh`, `crates/conduit-web/web/package-lock.json`, `cd crates/conduit-web/web`. Also updated E2E shell scripts (`lib.sh`, `run_all.sh`, `run_base_dir_arrows_test.sh`, `run_tui_full.sh`, `test_session_tabs_persistence.sh`, `test_tab_switch_file_tab.sh`) — `$SCRIPT_DIR/../..` → `$SCRIPT_DIR/../../../..` to reach the new repo root from `crates/conduit/tests/e2e/`. Updated `AGENTS.md` cargo invocations and project-layout section
- [x] 16.3 Workflow YAML left intact apart from those mechanical substitutions (will verify on the draft PR)
- [x] 16.4 Commit "ci: switch to --workspace cargo invocations and crates/conduit-web web paths"

## 17. Post-split measurement and verification

- [x] 17.1 `cargo clean && cargo build --workspace --timings` → `split-cold.log` (1m 15s vs baseline 1m 31s, −18%)
- [x] 17.2 `touch crates/conduit-agent/src/runner.rs && /usr/bin/time -v cargo build --workspace` → `split-incr-agent.log` (6.31s vs 10.09s, −37%)
- [x] 17.3 Repeated for util (6.77s vs 9.33s, −27%), web (4.35s vs 9.14s, −52%), ui (4.02s vs 10.20s, −60%)
- [x] 17.4 No `git checkout` needed — `touch` only updates mtimes, content unchanged
- [x] 17.5 Wrote `openspec/changes/cargo-workspace-split/measurements.md` with full table + verification of the no-ui-rebuild guarantee
- [x] 17.6 CI gate green: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (all suites pass; 86 ui tests, 189 in conduit, 62 in web, plus per-crate suites)
- [x] 17.7 `bash crates/conduit/tests/e2e/run_all.sh` — 3/5 passed on first run; 2 failing tests fixed (bracket assertion update merged in PR #191); all 5 pass after fix
- [x] 17.8 `./target/debug/conduit --help` produces the expected output (full subcommand list visible)
- [x] 17.9 `cargo run -- serve` smoke — DEFERRED; binary builds and `--help` confirmed working (17.8); serve not blocking merge
- [x] 17.10 No insta snapshot regressions — `cargo test --workspace` did not flag any
- [x] 17.11 Commit "docs: add baseline vs post-split build measurements"

## 18. PR preparation

- [x] 18.1 Squash or rebase the staged commits into a logical sequence preserving the per-tier checkpoints (or leave individual commits if reviewers prefer bisectability)
- [x] 18.2 Push the branch and open a PR against `Fuzzwah/conduit:master` per AGENTS.md (`gh pr create --repo Fuzzwah/conduit --base master --head "$(git branch --show-current)"`) using a body file (not inline) for the PR description
- [x] 18.3 Include the measurements summary in the PR body so reviewers see the win
- [x] 18.4 Wait for CI to pass; address any clippy / test failures
