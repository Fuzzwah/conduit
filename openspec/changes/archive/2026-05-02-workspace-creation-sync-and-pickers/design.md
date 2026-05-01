## Context

Workspace creation today emits effects in a sequence that *looks* sequential but is actually racy: `start_workspace_creation` (`src/ui/app.rs:5098`) returns `Effect::SyncRemote`, the `RemoteSynced` event handler (`src/ui/app.rs:7229`) then emits `Effect::FetchGithubIssues` and `Effect::FetchAllSpecs` together, and the `ShowSpecPicker` handler (`src/ui/app.rs:2873`) decides what to show based on which loading flag is currently true. The deeper issue is that `sync_remote()` (`src/git/worktree.rs:985`) only runs `git fetch origin --quiet`, which updates remote-tracking refs but **never touches the working tree**, while `fetch_open_specs()` (`src/git/specs.rs:17`) reads files directly from `repo.base_path/openspec/changes/`. So the picker shows whatever was last on disk regardless of how recently the user fetched.

Users have asked twice for this to be fixed. Both prior attempts addressed the *symptom* (picker shows after sync starts) without addressing the *cause* (scanner reads stale files). They also kept GitHub-only issue fetching and offered no way to filter long lists, so the picker becomes unusable on repos with many open specs or issues.

## Goals / Non-Goals

**Goals:**
- The spec picker shows the union of specs that are present and incomplete in `origin/<default_branch>`, not whatever the local working tree happens to contain.
- The order is strict and visible: sync → issues → specs → naming. Issue/spec fetch effects are never emitted before the prior phase has resolved.
- New issue providers can be added without touching the picker UI or the workspace flow.
- The user can filter both pickers by typing, and filter the issue picker further by label and assignee.
- Fail-soft behavior is preserved: missing `gh`, no auth, non-GitHub repo, no remote — all just yield an empty list and skip the picker silently.

**Non-Goals:**
- Mutating the user's working tree against their will. Fast-forward of `base_path` happens **only** when the branch is the default branch, the tree is clean, and the merge is FF-only.
- A general-purpose issue tracker UI inside conduit. The picker is purely a "link this workspace to issue X" affordance.
- Caching issue/spec results across workspace creations. Each Alt+N performs a fresh sync + fetch.
- Authentication management UI. Tokens come from env vars; if absent, the provider returns empty.

## Decisions

### Decision 1: Read specs from `origin/<default>` ref, not the working tree

**Chosen:** Add `fetch_open_specs_from_ref(repo_path, ref)` and `fetch_specify_specs_from_ref(repo_path, ref)`. Implementation walks the ref using `git ls-tree -d --name-only <ref> openspec/changes/` (skip `archive`), then `git show <ref>:openspec/changes/<id>/tasks.md` for each entry. Parse `- [ ]` / `- [x]` exactly as today. Resolve the ref via the existing `get_main_branch()` helper (`src/git/worktree.rs`, already used at `src/git/workspace_repo.rs:61`), formatted as `origin/<default>`. If the ref doesn't resolve, fall back to the working-tree variant.

**Why:**
- Always reads exactly what new worktrees branch from (`src/git/workspace_repo.rs:63` already branches off `origin/<default>`), so the picker is consistent with the resulting worktree contents.
- Doesn't depend on the user's working-tree state. Works correctly even when `base_path` is on a feature branch or is dirty.
- `git show` is read-only and cheap; one process per change file is fine for the small N involved (typically <100 changes).

**Alternatives considered:**
- *Always `git pull --ff-only` on `base_path`.* Rejected: refuses to do anything when `base_path` is on a feature branch or dirty, which is the common case for a developer mid-flight. Would leave the picker stale exactly when the user needs it most.
- *`git read-tree` into a temp index then scan.* Rejected: more complex, more failure modes, no real upside over `git show`.
- *In-memory walk via `git2` crate.* Rejected: adds a heavy dep for a small gain; the codebase already shells out to `git` everywhere else.

### Decision 2: Opportunistic fast-forward on top of fetch

**Chosen:** `sync_remote()` runs `git fetch origin --quiet`, then *additionally*, when all of these hold:
1. `git symbolic-ref --short HEAD` equals `get_main_branch()`,
2. `git status --porcelain` is empty,
3. local default branch is strictly behind `origin/<default>` and `git merge-base --is-ancestor <local> origin/<default>` succeeds,

run `git merge --ff-only origin/<default>`. Any failure is logged and ignored.

**Why:** It's cheap, it keeps the user's local default branch tidy in the common "I haven't switched away from master" case, and it composes harmlessly with Decision 1 — the spec picker still gets correct data even when the fast-forward is skipped.

**Alternatives considered:**
- *Fast-forward only, no read-from-ref.* Rejected: see Decision 1 — leaves the picker stale on feature branches.
- *Always pull, never ff-only.* Rejected: a non-FF merge would surprise the user and could conflict.

### Decision 3: Workspace creation as an explicit state machine

**Chosen:** Introduce `WorkspaceCreationPhase` enum in a new module `src/ui/workspace_creation.rs`:

```rust
pub enum WorkspaceCreationPhase {
    SyncingRemote,
    FetchingIssues,
    PickingIssue,
    FetchingSpecs,
    PickingSpec,
    Naming,
}
```

with a `transition()` function that takes the current phase and an event and returns `(next_phase, effects)`. Transitions are one-way; no event can rewind a phase. The `Effect::FetchRemoteIssues` is emitted only by `SyncingRemote → FetchingIssues`. The `Effect::FetchAllSpecs` is emitted only by `(PickingIssue|FetchingIssues) → FetchingSpecs`. This makes the race statically impossible.

**Why:**
- The current code spreads ordering logic across six handler sites; consolidating it into one transition function makes the order auditable and testable in isolation.
- Removes the need for "is this loading flag true?" guards in `ShowSpecPicker` (`src/ui/app.rs:2873`).
- Makes future additions (e.g. a milestone picker between issue and spec) a one-line phase insertion.

**Alternatives considered:**
- *Keep effect-based flow, add ordering asserts.* Rejected: doesn't remove the race, just turns it into a panic.
- *async/await chain.* Rejected: doesn't fit Ratatui's event-loop architecture. The codebase uses tokio + an effect queue; we work within that.

### Decision 4: `IssueProvider` trait with three providers

**Chosen:**

```rust
pub trait IssueProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn supports(&self, remote_url: &str) -> bool;
    fn fetch_open_issues(&self, repo_path: &Path) -> Vec<RemoteIssue>;
    fn current_user(&self, repo_path: &Path) -> Option<String>;
}

pub struct RemoteIssue {
    pub number: u32,
    pub title: String,
    pub labels: Vec<String>,
    pub assignee_logins: Vec<String>,
}
```

Layout:
- `src/git/issues/mod.rs` — trait, `RemoteIssue`, `providers()` registry that returns the first matching provider.
- `src/git/issues/github.rs` — wraps `gh issue list --json number,title,labels,assignees` and `gh api user --jq .login`.
- `src/git/issues/gitea.rs` — REST `/api/v1/repos/{owner}/{repo}/issues?state=open&type=issues`. `GITEA_TOKEN` env. Detection: host of `origin` URL matches an entry in conduit config's `gitea_hosts: Vec<String>`.
- `src/git/issues/forgejo.rs` — same REST shape (Forgejo is API-compatible with Gitea), `FORGEJO_TOKEN` env, `forgejo_hosts` config. Shares HTTP helpers with the Gitea module via a private `_rest.rs`.

Renaming `GithubIssue → RemoteIssue` is a one-time touch across `src/git/`, `src/ui/components/{issue_picker,spec_picker,specify_picker}.rs`, and any `app.rs` references — straightforward and contained.

**Why:**
- Lets us drop in a fourth provider later by adding one file.
- Keeps the picker UI provider-agnostic — it only ever sees `RemoteIssue`.
- Detection by config allowlist is the simplest robust answer for self-hosted Gitea/Forgejo, where the host name is unique to the user.

**Alternatives considered:**
- *Detect Gitea/Forgejo via HTTP probe of `/api/v1/version`.* Rejected for the first cut: latency on every workspace creation, and the failure mode (timeout against a private host) is annoying. We can revisit if the config-based approach proves clunky.
- *Single provider that branches internally.* Rejected: hard to add a new tracker without editing the same file repeatedly.

### Decision 5: HTTP client choice

**Chosen:** Reuse `reqwest` if it's already in the dep tree; otherwise add `ureq`. Verify before adding.

**Why:** The Gitea/Forgejo providers run inside `tokio::task::spawn_blocking` (matching the existing `gh`-shell pattern), so a blocking HTTP client is a fine fit. `ureq` is small and avoids pulling in async runtime deps if not needed. `reqwest`'s blocking feature is also acceptable if it's already present.

### Decision 6: Picker filtering reuses `SearchableListState`

**Chosen:** Wrap the picker states with the existing `SearchableListState` (`src/ui/components/searchable_list.rs`) instead of growing parallel filter state. Add a small helper alongside it:

```rust
pub fn filter_indices<T>(items: &[T], query: &str, key: impl Fn(&T) -> &str) -> Vec<usize>
```

doing case-insensitive substring matching. Issue picker also tracks:
- `selected_labels: BTreeSet<String>` — AND-filter against `RemoteIssue.labels`.
- `mine_only: bool` — when true, only issues whose `assignee_logins` contains the cached current user. Cached on first toggle via `IssueProvider::current_user()`. If `current_user()` returns `None`, the toggle is a no-op and the footer shows `mine: unavailable`.

Filters compose with AND. Sort (in spec picker only) applies to the filtered subset.

**Why:**
- `SearchableListState` already handles scroll/selection/filtered indices and is used elsewhere — no new primitive to maintain.
- AND-composition is the unsurprising default and is what the user verbally described.

### Decision 7: Spec picker visibility logic moves into the state machine

The current logic in `Effect::ShowSpecPicker` (`src/ui/app.rs:2873-2904`) — "if specify is loading or non-empty defer to it; else if openspec is loading show spinner; else if empty skip; else show" — moves into the state machine's `FetchingSpecs → ?` transition. Single source of truth.

## Risks / Trade-offs

- **[Risk] Renaming `GithubIssue` to `RemoteIssue` is a wide-touching change.** → Mitigation: it's a mechanical rename with strong type checking; `cargo check` will catch every site.
- **[Risk] Reading specs from `origin/<default>` could miss specs the user is actively editing locally on the default branch but hasn't committed.** → Acceptable: an uncommitted spec isn't ready to be picked yet; the previous behavior of showing it was an accident, not a feature. If the user wants to see local changes they can commit them.
- **[Risk] Opportunistic fast-forward could surprise users who expect their local default branch to never move.** → Mitigation: the preconditions are strict (already on default branch, clean, FF-only). Anyone who deliberately holds their local default behind origin will be on a feature branch and the FF will be skipped.
- **[Risk] Gitea/Forgejo detection relies on a config allowlist; users won't get any UX out of the box.** → Acceptable: the alternative (HTTP probe) has worse failure modes. Document the allowlist clearly in the proposal landing message.
- **[Risk] `git ls-tree`/`git show` per spec is N processes.** → Acceptable for typical N (<100). Can be batched later via a single `git archive` if it ever becomes a problem.
- **[Risk] The state machine refactor touches the busiest file in the project (`src/ui/app.rs`).** → Mitigation: do it as a contained extraction into `src/ui/workspace_creation.rs`; the net change to `app.rs` should be deletions plus a small `transition()` call site.
- **[Risk] `mine_only` requires `gh api user --jq .login`, an extra subprocess.** → Mitigation: only fetched when the user toggles `m`, and cached for the lifetime of the picker session.

## Migration Plan

This is a refactor of an internal flow with no public API or schema impact. Rollout is just merging the change. No feature flag needed; if a regression slips through, revert is trivial because the change is contained to the workspace-creation path and three picker components.

For users who already configure custom hosts: document `gitea_hosts` and `forgejo_hosts` in the conduit config docs (`docs/`) and in `FORK_CHANGES.md`.

## Open Questions

- **Q:** Should the spec picker also offer a sort like `recently modified` (using `git log -1 --format=%ct origin/<default> -- openspec/changes/<id>/`)? Useful, but adds N extra git invocations. *Defer until a user asks.*
- **Q:** When `base_path` is on a feature branch and we read specs from `origin/<default>`, the user might briefly be confused that "local files don't match what the picker shows." A footer hint like `reading from origin/master` would help. *Add the hint; cheap and low-risk.*
- **Q:** Should label and assignee filters be exposed in the spec picker too (specs don't have labels today, but spec-kit specs could)? *No — keep spec filters to text + sort for now.*
