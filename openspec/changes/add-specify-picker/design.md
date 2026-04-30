## Context

Conduit's workspace creation flow already detects incomplete openspec changes and surfaces a picker UI. The detection runs as a Tokio blocking task (`FetchOpenSpecs` effect) triggered after `RemoteSynced`. The result (`OpenSpecsFetched` event) feeds a `SpecPickerState` that drives a modal `SpecPicker` component. The `ShowSpecPicker` effect handles the race between the async fetch completing and the user reaching the spec-selection step; it uses a `pending_show` flag to defer the decision until data is ready.

Spec-kit (specify) uses an almost identical on-disk structure: `.specify/specs/<name>/tasks.md` with the same `- [ ]` / `- [x]` checkbox format. The only differences are the root directory and the absence of an `archive/` exclusion in the spec-kit layout.

## Goals / Non-Goals

**Goals:**
- Detect spec-kit specs at `.specify/specs/*/tasks.md` during workspace creation
- Show a `SpecifyPicker` modal when the repo has incomplete specify specs
- Prefer spec-kit over openspec when both are present
- Derive workspace name and branch from the selected specify spec's folder name
- Keep the existing openspec flow fully intact as the fallback

**Non-Goals:**
- Supporting spec-kit in any other part of conduit (e.g., sidebar display, workspace detail view)
- Merging openspec and spec-kit into a single unified spec abstraction
- Detecting spec-kit outside the workspace creation flow

## Decisions

### Unified `FetchAllSpecs` instead of two parallel effects

**Decision:** Replace `Effect::FetchOpenSpecs` and `AppEvent::OpenSpecsFetched` with `Effect::FetchAllSpecs` and `AppEvent::AllSpecsFetched { open_specs, specify_specs }`. Both disk scans run inside a single `spawn_blocking` call.

**Rationale:** Running two independent async fetches creates a coordination problem: `ShowSpecPicker` is triggered by `GithubIssuesFetched`, which may arrive before either fetch completes. Waiting for both to finish before making a "which picker to show" decision requires a synchronisation primitive (counter, boolean pair, or extra state). A single combined effect collapses this to one loading state with zero extra coordination logic — exactly the same as the current single-fetch design.

**Alternative considered:** Keep two separate effects and add a `specify_loading` flag alongside `loading` in picker state. Rejected because it doubles the state machine complexity for no architectural benefit.

### Spec-kit picker as a separate component (not a generic picker)

**Decision:** Create `SpecifyPicker` / `SpecifyPickerState` as distinct types mirroring `SpecPicker` / `SpecPickerState`, rather than making the picker generic over a trait.

**Rationale:** The two pickers are visually identical but carry different domain types (`OpenSpec` vs `SpecifySpec`). Introducing a generic `Picker<T>` or a shared trait would add abstraction without payoff — the spec and specify pickers are the only two instantiations, and they differ only in title and item field name. Duplication here is cheaper than premature abstraction.

**Alternative considered:** A `SelectedSpec` enum wrapping either type, shared through the picker. Rejected because it requires the picker component to know about both variants, coupling concerns that are currently separate.

### Priority: spec-kit wins over openspec

**Decision:** In `AllSpecsFetched`, if `specify_specs` is non-empty, show the specify picker; otherwise show the openspec picker; otherwise skip both.

**Rationale:** A repo using spec-kit has deliberately adopted it as its spec workflow. Showing the openspec picker in a spec-kit repo would be confusing. The inverse is equally true. Spec-kit takes priority because the feature being added is spec-kit support — users who adopt it should see it.

### `CreateWorkspace` gains `specify_spec: Option<SpecifySpec>`

**Decision:** Add a `specify_spec` field alongside the existing `spec: Option<OpenSpec>` in `Effect::CreateWorkspace`. The workspace/branch naming match arm is extended to a 3-tuple `(&issue, &spec, &specify_spec)`.

**Rationale:** Minimal change to the existing API. Avoids introducing a `SelectedSpec` enum just to unify the two optional fields — both are `None` in most cases and the match arm is easy to read.

## Risks / Trade-offs

- **Internal breaking change**: Renaming `FetchOpenSpecs`/`OpenSpecsFetched` affects all call sites in `app.rs`. The compiler will catch every missed reference, so the risk is low in practice.
- **Duplicate scanning on every workspace creation**: Both `.specify/specs/` and `openspec/changes/` are scanned even if only one exists. Both are fast local reads (no network), so the overhead is negligible.
- **Archive semantics for spec-kit**: The openspec scanner explicitly skips an `archive/` subdirectory. Spec-kit's documentation does not mention an archive convention. The initial implementation does not skip any subdirectory — if spec-kit gains an archive convention later, a follow-up can add it.

## Migration Plan

This is an internal-only change with no persisted state and no external API surface. No migration or rollback strategy is needed beyond reverting the commit.

## Open Questions

- Does spec-kit have an archive directory convention that should be skipped? (Assumed: no, based on current docs.)
