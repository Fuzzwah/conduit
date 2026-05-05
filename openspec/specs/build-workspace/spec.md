## ADDED Requirements

### Requirement: Virtual Cargo workspace at repo root

The repository SHALL be organized as a virtual Cargo workspace whose root `Cargo.toml` declares no `[package]` section, only `[workspace]`, `[workspace.package]`, `[workspace.dependencies]`, and shared `[profile.*]` blocks. All compilable code SHALL live in workspace member crates under the `crates/` directory.

#### Scenario: Root manifest has no package section
- **WHEN** a developer reads `Cargo.toml` at the repo root
- **THEN** it contains a `[workspace]` block listing `members = ["crates/*"]` (or equivalent enumeration) and `resolver = "2"`
- **AND** it contains no `[package]`, `[lib]`, or `[[bin]]` section
- **AND** it contains a `[workspace.dependencies]` block declaring shared third-party dependencies (versions and features) so member crates can inherit them via `<dep> = { workspace = true }`

#### Scenario: All source lives under crates/
- **WHEN** a developer lists files at the repo root
- **THEN** there is no `src/` directory at the repo root
- **AND** every Rust source file (excluding `build.rs`) lives under `crates/<crate-name>/src/`

### Requirement: Defined workspace member crates

The workspace SHALL contain the following member crates, each with the listed responsibility. Each crate's `Cargo.toml` SHALL declare only the third-party and internal dependencies it actually uses.

| Crate name | Path | Contents |
|---|---|---|
| `conduit-util` | `crates/conduit-util/` | Generic utilities (former `src/util/`) |
| `conduit-types` | `crates/conduit-types/` | Cross-module shared types: `Action`, `KeyBinding`, `InputMode`, `ViewMode`, `ChatMessage`, `MessageRole`, `TurnSummary`, `FileChange`, `app_prompt::*`, `GitTrackerUpdate`, `AgentType`, `AgentMode` |
| `conduit-git` | `crates/conduit-git/` | Git/worktree/PR management (former `src/git/`) |
| `conduit-agent` | `crates/conduit-agent/` | Agent runners and protocols (former `src/agent/`); `AgentType` and `AgentMode` SHALL be re-exported here via `pub use conduit_types::{AgentType, AgentMode};` for source compatibility |
| `conduit-resolver` | `crates/conduit-resolver/` | Command resolver (former `src/command_resolver.rs`) |
| `conduit-data` | `crates/conduit-data/` | SQLite-backed data layer (former `src/data/`) |
| `conduit-session` | `crates/conduit-session/` | External session discovery (former `src/session/`) |
| `conduit-config` | `crates/conduit-config/` | Settings and keybindings (former `src/config/`) |
| `conduit-theme` | `crates/conduit-theme/` | Theme types, persistence, migration (former `src/ui/components/theme/`) |
| `conduit-core` | `crates/conduit-core/` | Service orchestration (former `src/core/`) |
| `conduit-ui` | `crates/conduit-ui/` | Ratatui TUI (remaining `src/ui/`) |
| `conduit-web` | `crates/conduit-web/` | Axum web server, embedded frontend (former `src/web/` plus `web/` directory and `build.rs`) |
| `conduit` | `crates/conduit/` | Re-export-only umbrella library; hosts integration tests under `tests/` |
| `conduit-bin` | `crates/conduit-bin/` | CLI binary `conduit` (former `src/main.rs`) |

#### Scenario: Every required crate exists with a valid manifest
- **WHEN** a developer runs `cargo metadata --no-deps --format-version 1`
- **THEN** the `packages` array contains entries with `name` exactly matching each of: `conduit-util`, `conduit-types`, `conduit-git`, `conduit-agent`, `conduit-resolver`, `conduit-data`, `conduit-session`, `conduit-config`, `conduit-theme`, `conduit-core`, `conduit-ui`, `conduit-web`, `conduit`, `conduit-bin`
- **AND** each package's `manifest_path` is under `crates/<name>/Cargo.toml`

#### Scenario: AgentType is defined in conduit-types and re-exported from conduit-agent
- **WHEN** a developer reads `crates/conduit-types/src/` and `crates/conduit-agent/src/lib.rs`
- **THEN** the canonical definition of `pub enum AgentType` (and `pub enum AgentMode`) lives in `conduit-types` (e.g. `crates/conduit-types/src/agent.rs`)
- **AND** `crates/conduit-agent/src/lib.rs` (or its `models.rs` / `runner.rs`) contains `pub use conduit_types::{AgentType, AgentMode};` so that `conduit_agent::AgentType` and `conduit_agent::AgentMode` remain valid import paths

#### Scenario: Crates declare only the dependencies they use
- **WHEN** a developer runs `cargo tree -p conduit-web --prefix none --no-default-features --depth 1` (or equivalent)
- **THEN** the output does NOT contain `ratatui`, `syntect`, `two-face`, `tui-markdown`, or `arboard` (TUI-only deps)
- **AND** when the same is run for `cargo tree -p conduit-agent`, the output does NOT contain `axum`, `tower-http`, or `rust-embed` (web-only deps)

#### Scenario: conduit-theme declares no unused tokio dependency
- **WHEN** a developer runs `grep -rE "use tokio|tokio::" crates/conduit-theme/src/`
- **THEN** if no source file under `crates/conduit-theme/src/` references the `tokio` crate, then `crates/conduit-theme/Cargo.toml` SHALL NOT list `tokio` in its `[dependencies]` table

### Requirement: No cyclic dependencies between workspace crates

Internal workspace dependencies SHALL form a directed acyclic graph. The cyclic edges that exist in the pre-split codebase (`agent → ui::components`, `config → ui::action`/`ui::events`, `web → ui::app_prompt` and `web → ui::components::theme`) SHALL be eliminated by relocating shared types into `conduit-types` and the theme subtree into `conduit-theme`.

#### Scenario: Cargo accepts the workspace
- **WHEN** a developer runs `cargo check --workspace`
- **THEN** Cargo does not report any "cyclic package dependency" error
- **AND** the command exits with status 0

#### Scenario: Inversion fix targets are gone from offending crates
- **WHEN** a developer runs `grep -rE "use (crate|conduit_ui)::(components|action|events|app_prompt)" crates/conduit-agent/src crates/conduit-config/src crates/conduit-web/src`
- **THEN** no matches are returned

### Requirement: Public library surface preserved via umbrella crate

The `conduit` umbrella crate SHALL re-export every public symbol that the pre-split `conduit` library re-exported (mirroring `src/lib.rs:12-35` of the pre-split tree), so that external consumers and the integration tests under `tests/` continue to compile without rewriting their `use conduit::*` statements. The umbrella crate SHALL additionally expose every per-tier workspace crate under a module alias matching the crate's short name (e.g. `conduit::util`, `conduit::types`, `conduit::theme`, `conduit::resolver`).

#### Scenario: Existing public symbols importable from `conduit`
- **WHEN** a developer writes `use conduit::{App, Config, Database, ConduitCore, AgentEvent, AgentRunner, AgentType, WorktreeManager, PrManager, CommandResolver};` in a test or downstream consumer
- **THEN** the code compiles cleanly against the umbrella crate
- **AND** every symbol from the pre-split `pub use` lines remains accessible (no name removed without an equivalent re-export)

#### Scenario: Integration tests compile unchanged
- **WHEN** the integration tests under `crates/conduit/tests/integration/*.rs` (relocated from the pre-split `tests/integration/`) are compiled
- **THEN** no `use conduit::*` import line in those tests requires modification to compile

#### Scenario: Every per-tier crate is reachable as a module alias on the umbrella
- **WHEN** a developer writes `use conduit::{agent, config, core, data, git, resolver, session, theme, types, ui, util, web};` in a test
- **THEN** every named alias resolves to the corresponding `conduit-<name>` workspace crate
- **AND** in particular `conduit::theme` resolves to `conduit_theme` and `conduit::types` resolves to `conduit_types`

#### Scenario: Resolver alias uses the short name
- **WHEN** a developer reads `crates/conduit/src/lib.rs`
- **THEN** the umbrella declares `pub use conduit_resolver as resolver;`
- **AND** if a deprecated `pub use conduit_resolver as command_resolver;` line is also present, it SHALL carry a `#[deprecated]` attribute (or equivalent doc comment) naming the removal target so future maintainers can drop it

### Requirement: Single `conduit` binary with unchanged CLI

The workspace SHALL produce exactly one binary named `conduit`, defined by `crates/conduit-bin/Cargo.toml` as `[[bin]] name = "conduit"`. The binary's CLI surface (subcommands, flags, exit codes) SHALL be unchanged from the pre-split build.

#### Scenario: Binary builds and runs
- **WHEN** a developer runs `cargo build --workspace`
- **THEN** an executable appears at `target/debug/conduit`
- **AND** running `./target/debug/conduit --help` prints the same top-level help text as the pre-split binary (same subcommands and flags)

#### Scenario: Binary installable with cargo install
- **WHEN** a developer runs `cargo install --path crates/conduit-bin`
- **THEN** the install succeeds and produces a `conduit` binary on the user's PATH

### Requirement: Web frontend build script and embed paths localized to `conduit-web`

The npm-driven frontend build (`npm install && npm run build` against the React app) and the `rust-embed` derive that bundles `web/dist` SHALL both be owned by the `conduit-web` crate. The frontend source directory SHALL live at `crates/conduit-web/web/` so that `build.rs`'s `cargo::rerun-if-changed` directives and the `#[folder = "web/dist"]` literal continue to resolve relative to that crate's root.

#### Scenario: Frontend rebuilds only when conduit-web rebuilds
- **WHEN** a developer touches a file under `crates/conduit-util/src/` (a non-web crate) and runs `cargo build --workspace`
- **THEN** the npm build step does not execute
- **AND** when the developer touches `crates/conduit-web/web/src/App.tsx` and runs `cargo build --workspace`, the npm build step does execute

#### Scenario: Embedded assets resolve correctly
- **WHEN** the binary built from the workspace is run with `conduit serve`
- **THEN** the web UI is reachable at `http://127.0.0.1:3000` and serves the embedded `index.html` plus referenced assets without 404s

### Requirement: Shared dependency declarations via `[workspace.dependencies]`

Third-party dependencies that are used by more than one workspace crate SHALL be declared once in the root `[workspace.dependencies]` block with their version and feature set, and inherited by member crates via `<dep> = { workspace = true }`. Dependencies that are pulled by exactly one crate (e.g. `codex-protocol`, `codex-app-server-protocol`, `agent-client-protocol` for `conduit-agent`; `axum`, `tower`, `tower-http`, `rust-embed`, `mime_guess` for `conduit-web`; `ratatui`, `syntect`, `two-face`, `tui-markdown`, `arboard` for `conduit-ui`) SHALL be declared in `[workspace.dependencies]` but listed as a dependency only in that one crate.

#### Scenario: Shared deps unify features
- **WHEN** a developer runs `cargo tree --duplicates --workspace`
- **THEN** the output does NOT list `tokio`, `serde`, `serde_json`, `chrono`, `anyhow`, `thiserror`, `tracing`, `regex`, `uuid`, or `parking_lot` as duplicated (multiple versions or feature splits)

#### Scenario: Heavy single-consumer deps compile only once and are absent from unrelated crates
- **WHEN** a developer runs `cargo tree -p conduit-web` and `cargo tree -p conduit-ui`
- **THEN** `codex-protocol`, `codex-app-server-protocol`, and `agent-client-protocol` appear under neither (they are pulled only by `conduit-agent`)

### Requirement: Build profile tuned for incremental dev builds

The root `Cargo.toml` SHALL set the following `[profile.dev]` values to favour fast incremental rebuilds: `split-debuginfo = "unpacked"`, `debug = "line-tables-only"`, `codegen-units = 256`, `incremental = true`. A `[profile.dev.package."*"]` block SHALL set `opt-level = 0` for all packages. These settings SHALL apply unconditionally to dev builds.

#### Scenario: Profile values present
- **WHEN** a developer reads the root `Cargo.toml`
- **THEN** the `[profile.dev]` block contains exactly the four key/value pairs listed above
- **AND** the `[profile.dev.package."*"]` block contains `opt-level = 0`

#### Scenario: Profile values are picked up
- **WHEN** a developer runs `cargo build -v 2>&1 | grep "split-debuginfo\|line-tables-only"` (or inspects `cargo build --timings` output for invocation flags)
- **THEN** rustc invocations for workspace member crates include the corresponding flags

### Requirement: Linux dev linker config via `.cargo/config.toml`

A `.cargo/config.toml` file at the repo root SHALL configure the `mold` linker for the `x86_64-unknown-linux-gnu` target. The configuration SHALL only target Linux so that macOS and Windows developers inherit Cargo defaults. The file SHALL be safe to delete on machines without `mold`/`clang` installed (i.e., the workspace SHALL still build with the default linker if the file is removed).

#### Scenario: Linker config exists and targets Linux only
- **WHEN** a developer reads `.cargo/config.toml`
- **THEN** it contains a `[target.x86_64-unknown-linux-gnu]` block with `linker = "clang"` and `rustflags = ["-C", "link-arg=-fuse-ld=mold"]`
- **AND** it contains no other `[target.*]` block that would alter macOS or Windows builds

#### Scenario: Workspace builds without mold present
- **WHEN** a developer with neither `mold` nor `clang` installed deletes `.cargo/config.toml` and runs `cargo build --workspace`
- **THEN** the build succeeds using the default system linker

### Requirement: Incremental edit to a non-`ui` crate skips `conduit-ui` recompilation

When a developer modifies a single source file in any crate other than `conduit-ui` and runs `cargo build`, Cargo SHALL skip recompilation of `conduit-ui` (and of any third-party dependency unique to `conduit-ui`, e.g. `ratatui`, `syntect`).

#### Scenario: Editing conduit-agent does not rebuild conduit-ui
- **WHEN** a developer touches a file under `crates/conduit-agent/src/` and runs `cargo build --workspace -v 2>&1 | grep "Compiling conduit-ui"`
- **THEN** no line is printed (i.e., `conduit-ui` is not compiled)
- **AND** the same is true for `cargo build --workspace -v 2>&1 | grep "Compiling ratatui"`

#### Scenario: Editing conduit-util triggers downstream rebuilds but skips conduit-ui-internal work
- **WHEN** a developer touches `crates/conduit-util/src/lib.rs` and runs `cargo build --workspace -v`
- **THEN** crates that depend on `conduit-util` are recompiled (this is correct behavior)
- **AND** since `conduit-ui` depends on `conduit-util`, it is recompiled — but `ratatui`/`syntect`/`two-face` (third-party deps of `conduit-ui`) are NOT recompiled (they are cached separately as crate artifacts)

### Requirement: CI gate adapted to workspace and remains green

The CI quality gate SHALL run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`. The pre-split `tests/e2e/run_all.sh` shell harness SHALL continue to pass against the workspace-built binary. All checks SHALL pass on the final commit of the change branch.

#### Scenario: All four CI commands pass
- **WHEN** a developer runs `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && bash tests/e2e/run_all.sh` on the change branch
- **THEN** every command exits with status 0

### Requirement: Baseline and post-split build measurements captured

Before merging the change, baseline build timings (cold + four targeted incremental edits) SHALL be captured against the pre-split tree and compared against equivalent measurements taken against the post-split tree. The comparison SHALL be recorded in the change folder so the user can verify the incremental-build improvement.

#### Scenario: Measurements present before merge
- **WHEN** the change is ready for merge
- **THEN** the change folder contains a measurement file (e.g. `measurements.md`) summarising wall-clock timings for: cold `cargo build`, and incremental `cargo build` after touching one file each in `agent`, `util`, `web`, and `ui`
- **AND** the file shows both the baseline (pre-split) and post-split numbers
- **AND** for at least the `agent`, `util`, and `web` incremental cases, the post-split wall-clock time is lower than the baseline by a margin attributable to skipped `conduit-ui` recompilation

### Requirement: Lower-tier crates do not depend on `conduit-agent` for type-only reasons

The crates `conduit-config`, `conduit-data`, `conduit-resolver`, and `conduit-session` SHALL NOT list `conduit-agent` in their `[dependencies]` table when the only reason for that dependency is to import a type that lives in `conduit-types` (e.g. `AgentType`, `AgentMode`). Any of those crates that genuinely needs a runtime symbol from `conduit-agent` (a function, trait, or runner type) MAY still depend on it.

#### Scenario: conduit-config does not depend on conduit-agent
- **WHEN** a developer reads `crates/conduit-config/Cargo.toml`
- **THEN** the `[dependencies]` table does not list `conduit-agent`
- **AND** any reference to `AgentType` in that crate's source goes through `conduit_types::AgentType` (or a local alias of it)

#### Scenario: conduit-data does not depend on conduit-agent
- **WHEN** a developer reads `crates/conduit-data/Cargo.toml`
- **THEN** the `[dependencies]` table does not list `conduit-agent`
- **AND** the data models (`crates/conduit-data/src/models.rs`, `session_tab.rs`, etc.) reach `AgentType` via `conduit_types`

#### Scenario: conduit-resolver does not depend on conduit-agent for AgentType
- **WHEN** a developer reads `crates/conduit-resolver/Cargo.toml`
- **THEN** if the resolver source no longer uses any non-type symbol from `conduit-agent`, the dependency is removed
- **AND** if the resolver still uses some non-type runtime symbol from `conduit-agent`, that symbol is documented in a comment in the manifest explaining why the dep is retained

#### Scenario: conduit-session does not depend on conduit-agent for AgentType
- **WHEN** a developer reads `crates/conduit-session/Cargo.toml`
- **THEN** the same rule applies: removed if only types are needed; retained with a comment if a runtime symbol justifies it

### Requirement: Editing `conduit-agent` does not trigger recompilation of type-only consumers

When a developer modifies a single non-public-API source file inside `conduit-agent` (e.g. an internal helper in `runner.rs`) and runs `cargo build --workspace`, Cargo SHALL skip recompilation of `conduit-config`, `conduit-data`, `conduit-resolver`, and `conduit-session` provided those crates have been updated to depend only on `conduit-types` for their `AgentType` imports.

#### Scenario: Editing conduit-agent skips type-only dependents
- **WHEN** a developer touches `crates/conduit-agent/src/runner.rs` and runs `cargo build --workspace -v 2>&1 | grep "Compiling conduit-config\|Compiling conduit-data\|Compiling conduit-resolver\|Compiling conduit-session"`
- **THEN** for each of `conduit-config`, `conduit-data`, `conduit-resolver`, `conduit-session` whose Cargo.toml no longer lists `conduit-agent`, no `Compiling <name>` line is printed

#### Scenario: Public-API edit in conduit-agent still triggers downstream rebuilds
- **WHEN** a developer changes a `pub` item that `conduit-core`, `conduit-ui`, or `conduit-web` actually imports from `conduit-agent`, and runs `cargo build --workspace`
- **THEN** those downstream crates ARE recompiled (this is correct, expected behavior)

### Requirement: `pub use` statements grouped consistently in `conduit-ui` module roots

In each module root file under `crates/conduit-ui/src/` that mixes `mod` declarations with `pub use` re-exports of sibling workspace crates, the `pub use` lines SHALL be grouped together either at the top or at the bottom of the file rather than interleaved between `mod` declarations.

#### Scenario: components/mod.rs groups pub use lines
- **WHEN** a developer reads `crates/conduit-ui/src/components/mod.rs`
- **THEN** every line of the form `pub use conduit_<name> as <name>;` (or other `pub use` re-exports of workspace crates) is contiguous with other `pub use` lines and not surrounded by `mod <ident>;` declarations on both sides
