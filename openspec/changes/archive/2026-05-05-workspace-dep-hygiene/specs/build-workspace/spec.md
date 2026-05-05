## MODIFIED Requirements

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

## ADDED Requirements

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
