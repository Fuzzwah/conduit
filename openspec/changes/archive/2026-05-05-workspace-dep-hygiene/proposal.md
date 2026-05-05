## Why

The cargo workspace split (PR #182) achieved its primary goal — `conduit-ui` and `conduit-web` no longer rebuild each other — but a post-merge review found four lower-tier crates (`conduit-config`, `conduit-data`, `conduit-resolver`, `conduit-session`) still depend on the heavyweight `conduit-agent` crate solely (or primarily) for the `AgentType` enum. Because `conduit-agent` pulls in `reqwest`, `tokio`, `codex-protocol`, `image`, `futures`, and `async-trait`, every edit inside `conduit-agent` cascades through those four crates and on into `conduit-core`, `conduit-ui`, and `conduit-web` — exactly the wide rebuild fan-out the split was meant to prevent. The split also left a few cosmetic inconsistencies in the umbrella crate (missing `theme`/`types` re-exports, an asymmetric `command_resolver` alias) and an out-of-place `pub use` line in `conduit-ui/src/components/mod.rs` that are cheap to fix while the workspace is freshly in everyone's head.

## What Changes

- **Relocate `AgentType` and trivially-shared sibling enums** (e.g. `AgentMode`) from `conduit-agent` into `conduit-types`, where the other cross-cutting types from the original split already live.
- **Re-export the relocated types from `conduit-agent`** via `pub use conduit_types::{AgentType, AgentMode};` so existing call sites (`conduit_agent::AgentType`) continue to compile unchanged.
- **Drop `conduit-agent` from the `[dependencies]` of `conduit-config`, `conduit-data`, `conduit-resolver`, and `conduit-session`** wherever they only used it to reach `AgentType`. They now depend on `conduit-types` directly.
- **Add `pub use conduit_theme as theme;` and `pub use conduit_types as types;`** to the umbrella `conduit` crate so integration tests and downstream consumers can address them through the umbrella consistently with `agent`/`config`/`util`/etc.
- **Rename the umbrella alias `pub use conduit_resolver as command_resolver;` to `pub use conduit_resolver as resolver;`** to match the convention every other re-export uses (alias = short crate name). Keep the old name available as a deprecated re-export for one release cycle to avoid breaking external `conduit::command_resolver::*` imports.
- **Group `pub use conduit_theme as theme;` with the other `pub use` lines** in `crates/conduit-ui/src/components/mod.rs` (currently interleaved between `mod` declarations).
- **Audit the `tokio` dependency in `crates/conduit-theme/Cargo.toml`** and remove it if not actually used by any code in that crate (every unused declared dep extends cold builds and risks feature unification surprises).
- **Polish crate `description` fields**: swap the long product description currently on the `conduit` umbrella with the short one on `conduit-bin` so that whichever is published first reads correctly. Low priority; defer if the trade-off isn't obvious at edit time.

## Capabilities

### New Capabilities
None. This change tightens existing build-workspace requirements rather than introducing new behavior.

### Modified Capabilities
- `build-workspace`: tightens the dependency hygiene rules established by the original cargo workspace split — adds `AgentType`/`AgentMode` to the list of types that live in `conduit-types`; forbids the four lower-tier crates from depending on `conduit-agent` for type-only reasons; extends the umbrella's required re-export set with `theme` and `types`; renames the resolver alias for consistency.

## Impact

- **Affected crates**: `conduit-types` (gains `AgentType` + `AgentMode`), `conduit-agent` (loses the type definitions, gains `pub use` shims), `conduit-config`/`conduit-data`/`conduit-resolver`/`conduit-session` (drop `conduit-agent` from their internal deps where possible), `conduit` (umbrella adds two re-exports + rename), `conduit-ui` (one cosmetic mod.rs reorder), `conduit-theme` (potentially drops `tokio`).
- **Public API**: zero break for downstream `conduit::*` consumers — `conduit::App`, `conduit::Config`, `conduit::ConduitCore`, `conduit::AgentType`, etc. all continue to resolve. The new `conduit::theme::*` and `conduit::types::*` paths become available. The renamed `conduit::resolver` is added; the old `conduit::command_resolver` is kept as a deprecated alias for one release.
- **Build pipeline**: no CI change. Same four-command gate (`cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`).
- **Build-time wins (expected)**: editing `conduit-agent` no longer cascades through `conduit-config`/`conduit-data`/`conduit-resolver`/`conduit-session` solely because of a type import. Net effect on the runner.rs benchmark from PR #182 (currently 6.31s) should improve modestly; even a small absolute reduction matters because that path is hit on every agent-runner edit.
- **Risks**:
  - `AgentType` may have impl blocks or trait derives in `conduit-agent` that pull in deps not available to `conduit-types` (serde is fine; `clap::ValueEnum` derives may need feature gating). Mitigation: audit `AgentType`'s impl surface before moving and either migrate the impls along with it or keep the impls in `conduit-agent` via `impl SomeTrait for conduit_types::AgentType`.
  - The deprecated `command_resolver` alias adds two lines to the umbrella; if a future contributor mistakes it for live code, that's mild noise. Mitigation: comment with a removal target version.
  - Dropping `tokio` from `conduit-theme` is contingent on grep — must be verified before removal.
- **No behavioral change** to the TUI, web UI, agent runners, or any user-visible feature.
