## Why

When the user starts a new workspace (Alt+N), the OpenSpec/spec-kit picker frequently lists changes that are already complete and archived on the remote. The picker is reading the local working tree, but the local working tree was never updated to match the remote — `git fetch` only refreshes remote-tracking refs, not the files on disk. This has been "fixed" twice already and the symptom persists, because the underlying scanning still reads stale local files. Past attempts also coupled the issue/spec pickers tightly to GitHub and offered no way to filter long lists, which compounds the friction.

## What Changes

- **BREAKING (internal):** `sync_remote()` no longer just fetches. It (1) fetches, then (2) opportunistically fast-forwards the base checkout when on the default branch, clean, and a fast-forward is possible. Failures are logged and ignored — never abort workspace creation.
- Spec scanners (`fetch_open_specs`, `fetch_specify_specs`) gain `_from_ref` variants that read directly from `origin/<default_branch>` via `git ls-tree` + `git show`, bypassing the working tree. The workspace-creation flow always uses these. Working-tree variants remain as a fallback when no remote ref resolves.
- Workspace creation becomes an explicit linear state machine: `SyncingRemote → FetchingIssues → PickingIssue (if any) → FetchingSpecs → PickingSpec (if any) → Naming`. Issue and spec fetches are emitted only by transitions out of the prior phase, eliminating the race that allowed specs to be scanned before sync completed.
- New `IssueProvider` trait with three implementations: GitHub (existing `gh` CLI), Gitea (REST `/api/v1`, `GITEA_TOKEN`), Forgejo (REST `/api/v1`, `FORGEJO_TOKEN`). Provider selected by remote URL match; Gitea/Forgejo hosts come from a configurable allowlist in the conduit config.
- `GithubIssue` renamed to `RemoteIssue` and gains `labels: Vec<String>` and `assignee_logins: Vec<String>` fields.
- Issue picker gains three composable filters: text search (number + title, always-on), label multiselect (`Tab`), assignee scope toggle "mine only" (`m`). Search input renders at the top of the dialog.
- Spec picker (and spec-kit picker) gains text search on `change_id`. Existing `s` sort cycler reapplies to the filtered subset.
- Issue picker UI shows distinct phase messages: "Syncing with remote…" then "Fetching open issues…", driven by the state machine rather than ad-hoc booleans.

## Capabilities

### New Capabilities
- `workspace-creation-prelude`: The pre-naming flow that runs before the new-workspace name dialog — remote sync, issue picker, spec picker — including the strict ordering and preconditions that guarantee the spec picker only ever sees freshly-fetched data.
- `remote-issue-providers`: The provider-abstraction layer for fetching open issues from a repository's hosting service. Defines the `IssueProvider` trait, the registry that selects a provider from the repo's `origin` URL, and the GitHub / Gitea / Forgejo implementations.
- `picker-filtering`: Filtering and search behavior shared by the issue picker and spec/spec-kit pickers — text-search input, additional filters (labels, assignee scope), and how filters compose with existing sort.

### Modified Capabilities
<!-- None — `workspace-spec-context-load` covers post-creation messaging, not the picker flow. -->

## Impact

- **Code:**
  - `src/git/worktree.rs` (`sync_remote` extended with ff-only step)
  - `src/git/specs.rs`, `src/git/specify.rs` (new `_from_ref` variants)
  - `src/git/issues.rs` → split into `src/git/issues/{mod,provider,github,gitea,forgejo}.rs`; `GithubIssue` renamed to `RemoteIssue`
  - `src/ui/app.rs` (replace scattered effect handlers around lines 2810, 2829, 2847, 2873, 5098, 7229 with a single workspace-creation state machine)
  - `src/ui/components/{issue_picker,spec_picker,specify_picker}.rs` (filter UI, wired via existing `SearchableListState`)
  - `src/ui/app/app_input.rs` (route printable chars into picker search inputs while a picker is visible)
- **Config:** new optional fields `gitea_hosts: Vec<String>` and `forgejo_hosts: Vec<String>` in conduit's config (see `src/config/`). Absent → no Gitea/Forgejo detection.
- **Env:** `GITEA_TOKEN`, `FORGEJO_TOKEN` consulted when fetching from those providers; absent → silently empty issue list (same fail-soft behavior as `gh` today).
- **Dependencies:** an HTTP client crate for Gitea/Forgejo REST calls. Reuse `reqwest` if already in the dep tree, otherwise add `ureq` (lightweight, blocking, fits the existing `spawn_blocking` pattern). Verify before adding.
- **No DB or schema changes.** No public web API changes.
- **Behavior on GitHub-only setups:** identical UX to today plus the new filters; no functional regression for users who never configure a Gitea/Forgejo host.
