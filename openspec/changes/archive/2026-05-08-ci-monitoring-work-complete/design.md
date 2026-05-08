## Context

The Work Complete dialog is driven by a pure state machine in `crates/conduit-ui/src/work_complete.rs`. Phases, events, and commands are defined as enums; the `transition()` function maps `(phase, event) → (new_phase, commands)`. Side effects (git actions, preflight loads) are dispatched from `crates/conduit-ui/src/app.rs` via an `Effect` enum and executed in `tokio::task::spawn_blocking`. Results come back as `AppEvent` variants routed through the main event loop.

The current action execution pattern:
1. `Effect::WorkCompleteAction` spawns blocking work
2. Result fires `AppEvent::WorkCompleteActionFinished`
3. Handler dispatches `WorkCompleteEvent::ActionCompleted` → `transition()` → `LoadingPreflight` → preflight refresh

CI monitoring fits naturally as a second async phase that slots in before the preflight refresh after `OpenPr` or `Push`.

## Goals / Non-Goals

**Goals:**
- After `OpenPr` succeeds: automatically enter `MonitoringCi` phase
- After `Push` succeeds when a PR already exists: automatically enter `MonitoringCi` phase
- Show spinner + PR URL + accumulated log during monitoring
- Run `gh pr checks --watch <pr_url>` to completion, then refresh preflight
- Swallow all keys during monitoring (no escape hatch — matches existing `Executing` behaviour)

**Non-Goals:**
- Live streaming of check names as they progress (blocking `gh pr checks --watch` is sufficient; individual check status is visible in the terminal output appended to the log)
- Cancellation of in-flight CI monitoring
- Monitoring checks triggered by events other than `OpenPr` / `Push` in Work Complete

## Decisions

### D1: Slot CI monitoring as a new `WorkCompletePhase` variant, not a modified `Executing` variant

**Chosen:** Add `WorkCompletePhase::MonitoringCi { pr_url: String }`.

**Rationale:** `Executing { action }` renders generically and transitions uniformly on completion. CI monitoring has different rendering (PR URL subtitle) and a different post-completion path (no action; always go straight to preflight refresh). A dedicated phase keeps `transition()` arms unambiguous and the renderer straightforward.

**Alternative considered:** Reuse `Executing` with a synthetic `SuggestedAction::MonitorCi` variant. Rejected because it pollutes `SuggestedAction` (a git-layer type) with a UI-only concept, and the classify/scenario logic would need to exclude it.

### D2: Inject the PR URL via a new `WorkCompleteEvent::CiStarted`, dispatched from the `WorkCompleteActionFinished` handler

**Chosen:** After `OpenPr` or `Push` with existing PR, the handler in `app.rs` dispatches `CiStarted { pr_url }` instead of (or after appending log then instead of) `ActionCompleted`.

**Rationale:** `transition()` is pure and stateless — it doesn't have access to the session's preflight data or the action's log output. Parsing the PR URL or looking up the existing PR belongs in the app layer where the session is visible.

**Alternative considered:** Enrich `ActionCompleted(Vec<String>)` to carry an `Option<String>` PR URL. Rejected because it changes the meaning of a general-purpose event and requires all call sites to handle the optional field.

### D3: Run `gh pr checks --watch` in `spawn_blocking`, show a spinner; no streaming

**Chosen:** `tokio::task::spawn_blocking(|| { /* run process to completion */ })` — same pattern as all other Work Complete actions.

**Rationale:** Streaming line-by-line from a child process requires a different async execution model (using `tokio::process::Command` and reading stdout incrementally, which would need a new event for each line). The blocking approach is consistent with all existing actions and delivers the final output atomically. The spinner makes the wait visible.

**Alternative considered:** Stream stdout line-by-line with `tokio::process::Command`. Could provide finer-grained progress but adds significant complexity (new `AppEvent::WorkCompleteCiLine` variant, separate accumulation buffer). Deferred as a future enhancement.

### D4: PR URL extraction for `OpenPr` — parse from log line; for `Push` — read from session preflight data

**Chosen:**
- `OpenPr` result log contains `"Created PR #N: <url>"`. Parse by splitting on `": "` and taking the last segment.
- `Push` handler reads `session.data.as_ref()?.pr.as_ref()?.url.clone()` from the existing preflight snapshot.

**Rationale:** Both sources are available in the `WorkCompleteActionFinished` handler. Parsing from the log is fragile but the format is controlled code — a named constant for the prefix is sufficient. Reading from preflight for Push avoids duplicating the PR URL in the action result.

## Risks / Trade-offs

- **Log parse fragility for OpenPr URL** → Mitigation: define a module-level prefix constant (`"Created PR #"`) and extract the URL with a simple `find`/split; add a unit test for the parse helper.
- **`gh pr checks --watch` not available** → Mitigation: fall back gracefully — if the command exits non-zero immediately (e.g. no checks configured, no PR found), treat it as "checks passed" with a log note, and proceed to preflight refresh. Don't block the flow.
- **Long-running CI blocks the dialog indefinitely** → This matches the existing behaviour for `Executing` (no escape). Acceptable given the use case; cancellation is a future enhancement.
- **Push with no upstream / no PR** → The handler already checks for an existing PR in preflight data before dispatching `CiStarted`; if `pr` is `None`, it falls through to the current `ActionCompleted` path with no CI phase.

## Open Questions

None — implementation can proceed.
