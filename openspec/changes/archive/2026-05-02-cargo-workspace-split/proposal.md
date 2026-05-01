## Why

Conduit is one ~95k-LOC crate (`conduit-cli`), so any edit forces rustc to recompile the entire library and relink the binary even when the change is local to a single module. This makes the dev loop slow — touching `agent/`, `web/`, or `config/` rebuilds all of `ui/` (58k LOC) and the heavy third-party deps (ratatui, axum, syntect, codex-protocol, reqwest) are recompiled into the same crate object. Splitting the codebase into a Cargo workspace lets cargo recompile only the changed crate plus its downstream dependents, leaving the rest cached.

## What Changes

- Convert the repo root from a single-package crate to a virtual Cargo workspace; move all source under `crates/`.
- Extract twelve crates: `conduit-util`, `conduit-types` (NEW — shared low-level types), `conduit-git`, `conduit-agent`, `conduit-resolver`, `conduit-data`, `conduit-session`, `conduit-config`, `conduit-theme` (NEW — extracted theme subtree), `conduit-core`, `conduit-ui`, `conduit-web`, plus a re-export-only umbrella `conduit` crate and a thin `conduit-bin` that produces the `conduit` executable.
- Break three forced inversions that block clean splitting: `agent → ui::components` (move shared chat/turn types to `conduit-types`), `config → ui::action`/`ui::events` (move action enum + input mode to `conduit-types`), `web → ui::app_prompt` and `ui::components::theme` (move `app_prompt` to `conduit-types`, extract `theme` to `conduit-theme`).
- Move the `web/` frontend directory and `build.rs` into `crates/conduit-web/`; the npm build step now only triggers when `conduit-web` rebuilds.
- Move `tests/` into the umbrella `crates/conduit/tests/` so existing `use conduit::*;` integration tests keep compiling unchanged.
- Add `[workspace.dependencies]` for shared third-party deps so feature unification stays consistent (notably `tokio = "full"`, `agent-client-protocol` features = ["unstable"]).
- Add `[profile.dev]` tunings: `split-debuginfo = "unpacked"`, `debug = "line-tables-only"`, `codegen-units = 256`, `incremental = true`.
- Add `.cargo/config.toml` configuring the `mold` linker for Linux dev builds (optional; falls back gracefully if `mold` is absent).
- **BREAKING (internal)**: top-level module paths (`crate::ui::*`, `crate::agent::*`, etc.) become workspace crates. The umbrella `conduit` crate preserves the public re-exports (`conduit::App`, `conduit::Config`, etc.) so external consumers and `tests/` keep working.

## Capabilities

### New Capabilities
- `build-workspace`: Defines the Cargo workspace layout, the per-crate boundaries, the dependency tier order, and the build-profile knobs that govern incremental compile times for the conduit codebase.

### Modified Capabilities
<!-- None. This change restructures source organization but does not alter any user-visible feature behavior or any spec under openspec/specs/. -->

## Impact

- **Affected code**: every file under `src/` moves to `crates/<name>/src/`; root `Cargo.toml` becomes a virtual manifest; `build.rs` and `web/` move into `crates/conduit-web/`.
- **Public API**: preserved via the umbrella `conduit` crate's re-exports (mirrors today's `src/lib.rs:12-35`). The `conduit` binary name is unchanged.
- **Build pipeline**: CI commands gain `--workspace` (`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`); existing four-command CI gate (`cargo check`/`fmt`/`clippy`/`test`) otherwise unchanged.
- **Dependencies**: no new third-party crates added; `[workspace.dependencies]` consolidates the existing list. Optional dev-environment requirement: `mold` + `clang` for the linker config to take effect (skippable).
- **Tests**: integration tests under `tests/` relocate to `crates/conduit/tests/`; insta snapshot paths shift with the moved test files; e2e shell tests and `target/debug/conduit` bin path are unchanged.
- **Risks**: cycle introduction during the inversion fixes, feature-flag fragmentation if `[workspace.dependencies]` isn't disciplined, `rust-embed` path drift if `web/` is moved without keeping the `folder = "web/dist"` literal relative to the new crate root, `codex-protocol` git-tag deps recompiling more than once if declared in multiple crates.
