## 1. Pre-flight checks

- [ ] 1.1 Confirm working tree is on `master` and synced with `origin/master` so the `crates/*` layout is present (`git fetch origin && git status` clean).
- [ ] 1.2 Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` against the unmodified tree to capture a green baseline. Abort and investigate if any of these fail before any change is made.
- [ ] 1.3 `grep -rn "use conduit_agent::" crates/conduit-config/src crates/conduit-data/src crates/conduit-resolver/src crates/conduit-session/src` — record the exact set of import sites. Compare against the design's enumeration; investigate any new ones before continuing.
- [ ] 1.4 `grep -rn "command_resolver" crates/ openspec/` — record any in-tree consumers of the deprecated umbrella alias.
- [ ] 1.5 `grep -rE "use tokio|tokio::" crates/conduit-theme/src/` — note the result; this drives Decision 7.

## 2. Move `AgentType` and `AgentMode` into `conduit-types`

- [ ] 2.1 Create `crates/conduit-types/src/agent.rs` containing the `AgentType` enum (7 variants) and the `AgentMode` enum (2 variants), copied verbatim from `crates/conduit-agent/src/runner.rs:14-32`. Preserve the existing derive list (`Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize` for `AgentType`; `Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize` for `AgentMode`).
- [ ] 2.2 In the same `agent.rs`, paste the existing `impl AgentMode { ... }` block (`as_permission_mode`, `display_name`, `as_str`, `parse`, `toggle`) and `impl AgentType { ... }` block (`preferred_order`, `supports_plan_mode`, `as_str`, `parse`, `short_name`) verbatim from `crates/conduit-agent/src/runner.rs:80-180`.
- [ ] 2.3 Add `pub mod agent;` and `pub use agent::{AgentType, AgentMode};` to `crates/conduit-types/src/lib.rs`.
- [ ] 2.4 In `crates/conduit-agent/src/runner.rs`, replace the two `pub enum` blocks (and their `impl` blocks) with a single line near the top: `pub use conduit_types::{AgentType, AgentMode};`.
- [ ] 2.5 Run `cargo check --workspace`. Resolve any breakage. The most likely failure is a derive mismatch or an `impl` somewhere in `conduit-agent` that references `AgentType` via `crate::AgentType` and now needs to go through the re-export — both are mechanical fixes.
- [ ] 2.6 Run `cargo test -p conduit-agent -p conduit-types`. All previously-passing tests must still pass.

## 3. Drop `conduit-agent` from `conduit-data`

- [ ] 3.1 Edit each of `crates/conduit-data/src/{models,fork_seed,session_tab}.rs` to change `use conduit_agent::AgentType;` to `use conduit_types::AgentType;`.
- [ ] 3.2 Remove the line `conduit-agent = { workspace = true }` from `crates/conduit-data/Cargo.toml` `[dependencies]`.
- [ ] 3.3 `cargo check -p conduit-data`. If the check fails because of any other `conduit_agent::*` use that wasn't on the grep radar, restore the dep with a comment naming the remaining symbol; otherwise proceed.

## 4. Drop `conduit-agent` from `conduit-resolver`

- [ ] 4.1 In `crates/conduit-resolver/src/lib.rs:7`, change `use conduit_agent::AgentType;` to `use conduit_types::AgentType;`.
- [ ] 4.2 Remove `conduit-agent = { workspace = true }` from `crates/conduit-resolver/Cargo.toml` `[dependencies]`.
- [ ] 4.3 `cargo check -p conduit-resolver`. Same fallback rule as 3.3.

## 5. Drop `conduit-agent` from `conduit-session`

- [ ] 5.1 In `crates/conduit-session/src/import.rs:17`, change `use conduit_agent::AgentType;` to `use conduit_types::AgentType;`.
- [ ] 5.2 Remove `conduit-agent = { workspace = true }` from `crates/conduit-session/Cargo.toml` `[dependencies]`.
- [ ] 5.3 `cargo check -p conduit-session`. Same fallback rule as 3.3.

## 6. Adjust `conduit-config` (partial drop, keep `ModelRegistry`)

- [ ] 6.1 In `crates/conduit-config/src/settings.rs:8`, split `use conduit_agent::{AgentType, ModelRegistry};` into two lines: `use conduit_agent::ModelRegistry;` and `use conduit_types::AgentType;`.
- [ ] 6.2 Add `conduit-types = { workspace = true }` to `crates/conduit-config/Cargo.toml` if not already present (it is — verified). Keep `conduit-agent = { workspace = true }` and add an inline comment after that line: `# kept for ModelRegistry; AgentType lives in conduit-types`.
- [ ] 6.3 `cargo check -p conduit-config`.

## 7. Update umbrella crate re-exports

- [ ] 7.1 In `crates/conduit/src/lib.rs`, add `pub use conduit_theme as theme;` and `pub use conduit_types as types;` next to the other module aliases (alphabetically placed).
- [ ] 7.2 In the same file, add `pub use conduit_resolver as resolver;` immediately above the existing `pub use conduit_resolver as command_resolver;` line.
- [ ] 7.3 Replace the existing `pub use conduit_resolver as command_resolver;` line with the deprecated form: `#[deprecated(since = "0.6.0", note = "use \`conduit::resolver\` instead")] pub use conduit_resolver as command_resolver;` (split across two lines per rustfmt convention).
- [ ] 7.4 Patch any in-tree call sites of `conduit::command_resolver::*` found in step 1.4 to use `conduit::resolver::*` instead.

## 8. Cosmetic cleanup in `conduit-ui`

- [ ] 8.1 In `crates/conduit-ui/src/components/mod.rs`, move the line `pub use conduit_theme as theme;` so it sits with the other `pub use` lines (typically at the bottom of the file, after every `mod ...;` declaration).

## 9. Verify and (conditionally) drop `tokio` from `conduit-theme`

- [ ] 9.1 Re-run the grep from step 1.5 over `crates/conduit-theme/src/`. If empty, remove `tokio = { workspace = true }` from `crates/conduit-theme/Cargo.toml` `[dependencies]`. If matches are confined to `#[cfg(test)]` blocks, move the dep to `[dev-dependencies]` instead. If actively used in production code, leave as-is.
- [ ] 9.2 `cargo check -p conduit-theme && cargo test -p conduit-theme`.

## 10. Full workspace verification

- [ ] 10.1 `cargo fmt --all`.
- [ ] 10.2 `cargo fmt --check`.
- [ ] 10.3 `cargo clippy --workspace --all-targets -- -D warnings`. The deprecated `command_resolver` alias must NOT trigger a warning in any in-tree code (step 7.4 should have caught it); if it does, patch and re-run.
- [ ] 10.4 `cargo test --workspace`.
- [ ] 10.5 `cargo build --workspace` and confirm `target/debug/conduit --help` produces the expected subcommand output (sanity check on the bin).

## 11. Confirm the rebuild-cascade win

- [ ] 11.1 `touch crates/conduit-agent/src/runner.rs && cargo build --workspace -v 2>&1 | grep -E "Compiling conduit-(config|data|resolver|session)"`. Expect: `conduit-data`, `conduit-resolver`, `conduit-session` are NOT in the output. `conduit-config` IS still in the output (it depends on `conduit-agent` for `ModelRegistry`); that is the documented carve-out.
- [ ] 11.2 `touch crates/conduit-types/src/agent.rs && cargo build --workspace -v 2>&1 | grep -E "Compiling conduit-(agent|config|data|resolver|session|core|ui|web)"`. Expect: every named crate is rebuilt (a change to a leaf type cascades widely; that is correct behavior).
- [ ] 11.3 Record the wall-clock time for the runner.rs touch in 11.1 and compare against the baseline 6.31s reported in PR #182's measurements. A modest reduction is the expected outcome; the success criterion is the qualitative skip in 11.1, not a specific time.

## 12. Documentation polish (optional, low priority)

- [ ] 12.1 If publishing to crates.io is on the near-term roadmap, swap the `description` fields between `crates/conduit/Cargo.toml` and `crates/conduit-bin/Cargo.toml`. Otherwise skip (see Decision 8).

## 13. Wrap up

- [ ] 13.1 Update `openspec/changes/workspace-dep-hygiene/` if any decision changed during implementation (e.g. a non-type usage was discovered that forced retaining a dep).
- [ ] 13.2 Stage and commit. Suggested commit message: `refactor(workspace): move AgentType to conduit-types and tighten umbrella re-exports`.
- [ ] 13.3 Open a PR against `Fuzzwah/conduit:master` per [AGENTS.md](AGENTS.md). Reference this change in the PR body and link back to PR #182's review thread for context.
