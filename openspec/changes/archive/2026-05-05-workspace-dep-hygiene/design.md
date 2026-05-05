## Context

PR #182 split the monolithic `conduit-cli` crate into a 14-crate Cargo workspace. The split achieved its primary goal — the two heaviest crates (`conduit-ui` and `conduit-web`) are now siblings and editing one no longer recompiles the other — but a post-merge review (this proposal's parent thread) identified four secondary issues:

1. **Type-only edges from low-tier crates into the heavyweight `conduit-agent`.** `conduit-config`, `conduit-data`, `conduit-resolver`, and `conduit-session` all list `conduit-agent` in `[dependencies]` mostly because they import `AgentType` (a 7-variant pure enum). `conduit-agent` brings in `reqwest`, `tokio`, `codex-protocol`, `image`, `futures`, and `async-trait`. Every internal edit to `conduit-agent` therefore cascades through these four crates and onward to `conduit-core`, `conduit-ui`, and `conduit-web`.
2. **Umbrella re-export gaps**: `crates/conduit/src/lib.rs` re-exports 10 of 12 per-tier crates as module aliases (`agent`, `config`, `core`, `data`, `git`, `command_resolver`, `session`, `ui`, `util`, `web`) but omits `theme` and `types`.
3. **Asymmetric alias** `command_resolver` for `conduit-resolver` while every other re-export uses the crate's short name.
4. A small cosmetic mix of `mod` declarations and `pub use` lines in `crates/conduit-ui/src/components/mod.rs`, plus a `tokio` dependency in `crates/conduit-theme/Cargo.toml` whose actual usage in that crate is unverified.

Verified findings from the source (`origin/master`):
- `AgentType` is a pure 7-variant enum at `crates/conduit-agent/src/runner.rs:14`. Derives only `serde` traits and `Hash`. Has impl blocks (`preferred_order`, `supports_plan_mode`, `as_str`, `parse`, `short_name`) that don't depend on any other agent runtime symbol.
- `AgentMode` is a pure 2-variant enum at `crates/conduit-agent/src/runner.rs:29`. Same shape — derives serde, no runtime-coupled methods.
- `ReasoningEffort` (also at runner.rs:36) has methods like `claude_arg_value`/`codex_config_value` that read as runner-specific. **Not moving it.**
- All four lower-tier crates use `conduit_agent::AgentType` and nothing else from `conduit-agent`, **except** `conduit-config` which also uses `conduit_agent::ModelRegistry`. `ModelRegistry` pulls in `claude`/`codex`/`gemini`/`opencode`/`pi` model entries internally and cannot move to `conduit-types`.

This change extends the existing `build-workspace` capability (from PR #182's archived spec) rather than introducing new behavior. It strengthens dependency hygiene rules and tightens the umbrella re-export contract.

## Goals / Non-Goals

**Goals:**
- Move `AgentType` and `AgentMode` from `conduit-agent` into `conduit-types`. Re-export from `conduit-agent` with `pub use` so existing `conduit_agent::AgentType` and `conduit_agent::AgentMode` import paths remain valid (zero source-compat break for in-tree code or downstream consumers).
- Drop `conduit-agent` from `[dependencies]` of `conduit-data`, `conduit-resolver`, and `conduit-session` (all of which only used it for `AgentType`). Each of those crates depends on `conduit-types` directly instead.
- Keep `conduit-agent` in `conduit-config`'s `[dependencies]` because of `ModelRegistry`, but document the carve-out with a one-line comment in `Cargo.toml` so future readers know why the dependency is retained.
- Add `pub use conduit_theme as theme;` and `pub use conduit_types as types;` to `crates/conduit/src/lib.rs`.
- Rename umbrella alias `command_resolver` → `resolver`. Keep the old name as a `#[deprecated]` re-export through one minor-version cycle so any external `conduit::command_resolver::*` user gets a compiler warning rather than a hard break.
- Group the `pub use conduit_theme as theme;` line with other `pub use` lines (or move it to the file end) in `crates/conduit-ui/src/components/mod.rs`.
- Audit `tokio` in `crates/conduit-theme/`. If unused, remove it.
- Verify the post-change tree passes `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.

**Non-Goals:**
- Moving `ModelRegistry` out of `conduit-agent`. Its internals depend on per-runner model entries (`claude::load_claude_models`, `codex::CodexModelEntry`, etc.); a clean extraction would require a much larger refactor. Acknowledged as a future follow-up only.
- Moving `ReasoningEffort` out of `conduit-agent`. Its methods are CLI-flag-shaped (`claude_arg_value`, `codex_config_value`) and belong with the runners.
- Aligning duplicate transitive versions (crossterm 0.28/0.29, thiserror 1/2). Pre-existing; out of scope per PR #182's own description.
- Splitting `conduit-ui` further. Explicit user trade-off in PR #182.
- Any change to the binary CLI surface, the TUI, the web UI, or the agent runner protocols.
- Capturing fresh build-time benchmarks. The original PR's measurements remain authoritative; this change should produce a small improvement on `runner.rs` edits but the measurement effort isn't worth the noise unless someone wants the receipt.

## Decisions

### Decision 1: Where does `AgentType` live in `conduit-types`?

**Choice:** Create a new module file `crates/conduit-types/src/agent.rs` containing both `AgentType` and `AgentMode`. Add `pub mod agent;` to `crates/conduit-types/src/lib.rs` and `pub use agent::{AgentType, AgentMode};` for top-level access.

**Alternatives considered:**
- Putting them at the crate root (in `lib.rs`) — fine for two enums but doesn't scale; `conduit-types` already has six modules (`action`, `app_prompt`, `chat_message`, `input_mode`, `skill`, `turn_summary`), so adding a seventh keeps the convention.
- Splitting into `agent_type.rs` and `agent_mode.rs` — gratuitous; they share a tight conceptual cluster.

### Decision 2: Where does the `pub use` shim live in `conduit-agent`?

**Choice:** Inside `crates/conduit-agent/src/runner.rs`, replace the existing `pub enum AgentType` and `pub enum AgentMode` blocks (and their impl blocks) with `pub use conduit_types::{AgentType, AgentMode};` at the top of the file. Move the impl blocks to `conduit-types/src/agent.rs` along with the enums (since they only use `Self`/string conversion, no agent runtime symbols).

The existing `crates/conduit-agent/src/lib.rs` line `pub use runner::{AgentHandle, AgentInput, AgentMode, AgentRunner, AgentStartConfig, AgentType, ReasoningEffort};` continues to work unchanged — it now re-exports the types from `conduit-types` transitively.

**Alternatives considered:**
- Keeping the impl blocks in `conduit-agent` as `impl conduit_types::AgentType { ... }` — would split the type's behavior across two crates; harder to reason about and offers no benefit since the impls are pure.

### Decision 3: How to drop `conduit-agent` from lower-tier crates without breaking imports

**Choice:** In each of `conduit-data/src/{models,fork_seed,session_tab}.rs`, `conduit-resolver/src/lib.rs`, `conduit-session/src/import.rs`, change `use conduit_agent::AgentType;` to `use conduit_types::AgentType;`. Then remove `conduit-agent = { workspace = true }` from each crate's `Cargo.toml` `[dependencies]` table.

For `conduit-config`: change `use conduit_agent::{AgentType, ModelRegistry};` to two lines — `use conduit_agent::ModelRegistry;` and `use conduit_types::AgentType;`. Keep the `conduit-agent` dependency in `Cargo.toml` and add a comment: `# kept for ModelRegistry; AgentType lives in conduit-types now`.

**Alternatives considered:**
- Searching for and patching every `use conduit_agent::AgentType` site to keep using the re-exported path — works but loses the rebuild-cascade benefit. The whole point is to break the manifest-level dep edge, which requires the source `use` lines to point at `conduit-types`.

### Decision 4: Umbrella alias rename strategy

**Choice:** Add `pub use conduit_resolver as resolver;` to `crates/conduit/src/lib.rs`. Keep the existing `pub use conduit_resolver as command_resolver;` line but mark it deprecated:

```rust
#[deprecated(since = "0.6.0", note = "use `conduit::resolver` instead")]
pub use conduit_resolver as command_resolver;
```

Update the only in-tree consumer (`crates/conduit-bin/src/main.rs:5` imports `conduit::{config::save_tool_path, ui::terminal_guard, util::{...}, App, Config}` — does not currently use `command_resolver`, so likely no in-tree update needed; verify with grep at task time).

The deprecated alias is removed in the next minor version (a separate, trivial change).

**Alternatives considered:**
- Hard-rename without deprecation — fastest but breaks any external `use conduit::command_resolver::*` line silently. The original PR description emphasised "no public API change for `tests/` or downstream consumers"; that should hold here too.

### Decision 5: Umbrella re-exports for `theme` and `types`

**Choice:** Add two lines to `crates/conduit-conduit/src/lib.rs` next to the other module aliases:

```rust
pub use conduit_theme as theme;
pub use conduit_types as types;
```

No top-level `pub use` of individual symbols from these crates is added (consumers reach in via `conduit::types::AgentType`, `conduit::theme::ThemeRegistry`, etc.). The umbrella's existing top-level re-exports already cover the symbols that downstream code historically imported — the new module aliases are for completeness and explicit reach-through.

### Decision 6: `pub use conduit_theme as theme;` placement

**Choice:** In `crates/conduit-ui/src/components/mod.rs`, move the `pub use conduit_theme as theme;` line to sit with the other `pub use` lines at the bottom of the file (after all `mod ...;` declarations). This is the conventional placement and matches the order Cargo `cargo fmt`'s convention encourages.

### Decision 7: `tokio` dependency in `conduit-theme`

**Choice:** Run `grep -rE "use tokio|tokio::" crates/conduit-theme/src/` at task time. If the result is empty, remove `tokio = { workspace = true }` from `crates/conduit-theme/Cargo.toml`. If it is non-empty, keep it.

If any file references `tokio::` only inside `#[cfg(test)]` blocks, the dep should move from `[dependencies]` to `[dev-dependencies]`.

### Decision 8: Crate `description` field swap

**Choice:** Defer. The current asymmetry (umbrella has the long description, bin has the short one) is harmless until either crate gets published. If the workspace ever ships to crates.io, swap at that time. Mentioned in the proposal for completeness; not implemented here.

## Risks / Trade-offs

- **[Risk] `AgentType` derives a trait that requires a non-trivial dep not present in `conduit-types`.** Currently it derives `Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize`. `serde` is already in `conduit-types`; the others are core. **Mitigation:** verified by reading `runner.rs:14-26`. If any future trait derivation (e.g. `clap::ValueEnum`) is added, that derive must be applied where the type lives or feature-gated.
- **[Risk] An impl method on `AgentType` reaches for an agent-runtime symbol I missed.** **Mitigation:** the design is verified against `runner.rs:122-180` — `preferred_order`/`supports_plan_mode`/`as_str`/`parse`/`short_name` are all pure. The task plan includes a cargo check after the move as the verification step.
- **[Risk] Removing `conduit-agent` from a Cargo.toml uncovers a latent `use conduit_agent::Foo` that wasn't on the grep radar.** **Mitigation:** task plan runs `cargo check --workspace` after each individual Cargo.toml edit so the failure is local and easy to diagnose. Re-add the dep with a comment if a non-type use is uncovered.
- **[Risk] The `#[deprecated]` re-export of `command_resolver` triggers `-D warnings` in CI for any in-tree file that still uses it.** **Mitigation:** grep for `command_resolver` across the workspace at task time and patch any remaining call sites before adding the `#[deprecated]` attribute. If none are found in-tree, no in-tree breakage is possible — the deprecation only fires for external consumers.
- **[Trade-off] `conduit-config` keeps its `conduit-agent` dep.** Edits to `runner.rs` will still recompile `conduit-config` (and everything downstream of it). The other three lower-tier crates skip that cascade, so the win is partial. Resolving fully requires moving `ModelRegistry` — explicit non-goal.
- **[Trade-off] The deprecated `command_resolver` alias adds two lines to the umbrella crate plus a `#[deprecated]` attribute.** Future maintainers may ignore the deprecation and leave it indefinitely. **Mitigation:** the `note` field includes a target version; a one-line follow-up change in 0.6.0 removes it.

## Migration Plan

This is an in-tree refactor with no runtime behavior change. There is no deployment, rollback, or data migration. All "migration" concerns are source-level:

1. **In-tree code** — every `use conduit_agent::AgentType` site is rewritten to `use conduit_types::AgentType` as part of this change. Verified by grep.
2. **External downstream consumers** — historical `use conduit::AgentType` (the umbrella's top-level re-export) continues to resolve unchanged because the umbrella keeps re-exporting `AgentType` from `agent` (which now re-exports it from `types`).
3. **External `conduit::command_resolver::*` users** — get a `deprecated` warning at compile time pointing them at `conduit::resolver`. Removal scheduled for 0.6.0.

## Open Questions

- Should the deprecated `command_resolver` alias be removed in the same PR (cleanest) or held back to a separate cleanup PR (lowest risk to external users)? **Default plan:** add deprecation in this change, remove in a follow-up labelled `0.6.0`.
- Does `crates/conduit-theme/` actually use `tokio`? Resolved at task time by grep; the design accommodates either outcome.
- Is `ReasoningEffort` worth moving alongside `AgentType`? Decided no — its impl methods are CLI-flag-shaped (`claude_arg_value`, `codex_config_value`) and live more naturally with the runners. Re-evaluate if a non-runner consumer ever wants it.
