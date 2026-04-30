## Why

Conduit already surfaces incomplete openspec changes during workspace creation, but has no awareness of spec-kit (specify) — GitHub's alternative spec workflow that stores specs under `.specify/specs/`. Users working in repos that adopt spec-kit get no spec-linking benefit when creating workspaces.

## What Changes

- Add a `SpecifySpec` type and `fetch_specify_specs()` scanner that reads `.specify/specs/*/tasks.md`
- Replace the separate `FetchOpenSpecs` effect and `OpenSpecsFetched` event with a unified `FetchAllSpecs` / `AllSpecsFetched` that runs both disk scans in one blocking task
- Add a `SpecifyPicker` UI component (modal picker) shown when specify specs are found
- Integrate specify detection into the workspace creation flow: prefer spec-kit picker if the repo has specify specs; fall back to openspec picker; skip both if neither has incomplete specs
- **BREAKING**: `Effect::FetchOpenSpecs` → `Effect::FetchAllSpecs`; `AppEvent::OpenSpecsFetched` → `AppEvent::AllSpecsFetched`
- `Effect::CreateWorkspace` gains a `specify_spec: Option<SpecifySpec>` field for use in workspace/branch naming

## Capabilities

### New Capabilities

- `specify-picker`: Modal picker shown during workspace creation for repos using spec-kit. Scans `.specify/specs/*/tasks.md`, lists specs with remaining tasks, and lets the user select one (or skip). Selected spec name is used to derive the workspace and branch name.

### Modified Capabilities

- (none — the openspec picker's external behavior is unchanged; only internal plumbing is refactored)

## Impact

- `src/git/` — new `specify.rs` module
- `src/ui/events.rs` — event enum change (breaking internal rename)
- `src/ui/effect.rs` — effect enum change + `CreateWorkspace` field addition
- `src/ui/app.rs` — effect/event handlers, render path
- `src/ui/app/app_input.rs` — new input mode handler
- `src/ui/components/` — new `specify_picker.rs`
- `src/ui/app_state.rs` — new picker state field
