## Why

Tab titles currently display the full project name and a randomized workspace name in parentheses (e.g., `conduit (old-rose)`), wasting space and surfacing internal implementation details (the workspace name) rather than useful information (the branch name).

## What Changes

- Tab titles will truncate the project name to 10 characters (with a `…` suffix if longer)
- The workspace name in parentheses will be replaced with the trailing segment of the git branch name in square brackets (e.g., `fuz/old-rose` → `[old-rose]`)
- Resulting format: `conduit [old-rose]` or `very-long-p… [feature-x]`

## Capabilities

### New Capabilities

- `compact-tab-title`: Tab title rendering that shows a truncated project name and branch suffix instead of the full project name and randomized workspace name.

### Modified Capabilities

<!-- No existing spec-level requirements change -->

## Impact

- `src/ui/session.rs` — `AgentSession` struct and `tab_name()` method
- `src/ui/app_state.rs` — `PendingHandoffRequest` struct
- `src/ui/app.rs` — session creation and handoff paths that populate tab title fields
