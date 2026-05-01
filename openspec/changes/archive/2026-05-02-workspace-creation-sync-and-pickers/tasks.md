## 1. Git layer: sync + ref-based spec scanning

- [x] 1.1 Extend `sync_remote()` in `src/git/worktree.rs` to perform an opportunistic FF after `git fetch`, gated on `HEAD == default branch`, clean tree (`git status --porcelain` empty), and `git merge-base --is-ancestor <local> origin/<default>`. Failures log a warning and return.
- [x] 1.2 Add a unit test using `tempfile` + a local "remote" repo (init bare, push, advance, fetch) covering: clean default branch FFs, dirty tree skips, feature branch skips, missing remote no-ops.
- [x] 1.3 Add `fetch_open_specs_from_ref(repo_path: &Path, git_ref: &str) -> Vec<OpenSpec>` to `src/git/specs.rs`. Use `git ls-tree -d --name-only <ref> openspec/changes/`, skip `archive`, then `git show <ref>:openspec/changes/<id>/tasks.md` per entry; reuse the existing `- [ ]`/`- [x]` parser. Return empty `Vec` on any git error.
- [x] 1.4 Add `fetch_specify_specs_from_ref(repo_path: &Path, git_ref: &str) -> Vec<SpecifySpec>` to `src/git/specify.rs` using the same pattern over `.specify/specs/`.
- [x] 1.5 Add unit tests for both `_from_ref` functions: archived spec hidden, locally-uncommitted spec hidden, working-tree fallback when ref doesn't resolve.

## 2. Issue provider abstraction

- [x] 2.1 Decide on HTTP client: check whether `reqwest` is already a dep; if not, add `ureq` to `Cargo.toml` (blocking, lightweight). Note the choice in `design.md` Decisions section if it differs from the recommendation.
- [x] 2.2 Convert `src/git/issues.rs` into `src/git/issues/mod.rs`. Define `pub trait IssueProvider: Send + Sync` with `name`, `supports`, `fetch_open_issues`, `current_user`. Define `pub struct RemoteIssue { number, title, labels, assignee_logins }`. Define `pub fn providers() -> &'static [Box<dyn IssueProvider>]` (or equivalent registry returning the matching provider for a remote URL).
- [x] 2.3 Move existing GitHub logic into `src/git/issues/github.rs` implementing `IssueProvider`. Update `gh issue list` JSON fields to `number,title,labels,assignees`. Add `current_user` via `gh api user --jq .login`. Keep the silent-empty-on-error contract.
- [x] 2.4 Add `src/git/issues/_rest.rs` with a tiny blocking HTTP helper (GET with bearer auth, JSON deserialize) for Gitea/Forgejo to share.
- [x] 2.5 Implement `src/git/issues/gitea.rs`: `supports()` checks origin host against `gitea_hosts` config; `fetch_open_issues` calls `GET /api/v1/repos/{owner}/{repo}/issues?state=open&type=issues&limit=50` with `GITEA_TOKEN`; `current_user` calls `GET /api/v1/user`. All errors → empty.
- [x] 2.6 Implement `src/git/issues/forgejo.rs` analogously, using `forgejo_hosts` and `FORGEJO_TOKEN`. Share request helpers with `_rest.rs`.
- [x] 2.7 Add `gitea_hosts: Vec<String>` and `forgejo_hosts: Vec<String>` to conduit's config struct (`src/config/`). Default to empty. Document in `FORK_CHANGES.md` and `docs/`.
- [x] 2.8 Rename `GithubIssue` → `RemoteIssue` everywhere it appears (`src/git/`, `src/ui/components/{issue_picker,spec_picker,specify_picker}.rs`, `src/ui/app.rs`). `cargo check` is the source of truth for completeness.
- [x] 2.9 Add a unit test for `providers()` registry: GitHub URL → GitHub provider; configured Gitea host → Gitea provider; configured Forgejo host → Forgejo provider; unknown host → no provider.

## 3. Workspace-creation state machine

- [x] 3.1 Create `src/ui/workspace_creation.rs` with `pub enum WorkspaceCreationPhase { SyncingRemote, FetchingIssues, PickingIssue, FetchingSpecs, PickingSpec, Naming }` and a `pub fn transition(phase, event, ctx) -> (WorkspaceCreationPhase, Vec<Effect>)`. Cover all transitions exhaustively including auto-skip on empty results.
- [x] 3.2 Add a unit test for `transition()`: full happy path (sync → issues → pick issue → specs → pick spec → naming); empty issues skips to specs; empty specs skips to naming; Esc on each picker advances; sync failure still advances.
- [x] 3.3 In `src/ui/app.rs`, replace the scattered effect handlers around lines 2810 (`Effect::SyncRemote`), 2829 (`Effect::FetchGithubIssues` → `FetchRemoteIssues`), 2847 (`Effect::FetchAllSpecs`), 2873 (`Effect::ShowSpecPicker`), 5098 (`start_workspace_creation`), 7229 (`AppEvent::RemoteSynced`) with a single dispatch into `workspace_creation::transition()`. The state machine owns the picker visibility decisions previously in `Effect::ShowSpecPicker`.
- [x] 3.4 Wire `Effect::FetchAllSpecs` to call the `_from_ref` variants with `format!("origin/{}", get_main_branch(&path).unwrap_or("main"))`. Fall back to working-tree variants if the ref can't be resolved (e.g. `git rev-parse origin/<default>` fails).
- [x] 3.5 Verify with a temporary `tracing` log in `transition()` that no `FetchAllSpecs` effect is emitted before `RemoteSynced` is observed AND the issue phase is past. Remove the log before commit. (Verified structurally: `FetchAllSpecs` only emits from `FetchingIssues→FetchingSpecs` and `PickingIssue→FetchingSpecs`; both require `RemoteSynced` to have advanced past `SyncingRemote`. State-machine unit tests cover the ordering exhaustively.)

## 4. Picker filtering UI

- [x] 4.1 Add `pub fn filter_indices<T>(items: &[T], query: &str, key: impl Fn(&T) -> &str) -> Vec<usize>` next to `SearchableListState` in `src/ui/components/searchable_list.rs`. Case-insensitive substring match; empty query returns all indices in order.
- [x] 4.2 Refactor `IssuePickerState` (`src/ui/components/issue_picker.rs`) to hold a `SearchableListState` plus `selected_labels: BTreeSet<String>`, `mine_only: bool`, `cached_current_user: Option<Option<String>>` (outer Option = "have we tried", inner = the user). Remove the standalone `selected`, `scroll_offset` fields.
- [x] 4.3 Render the search input as a one-row text field at the top of the issue picker dialog. Render selected labels as chips above the list. Render a footer line `N/M issues · mine: on/off/unavailable · labels: a,b`.
- [x] 4.4 In `src/ui/app/app_input.rs` issue-picker handler, route printable chars to the search input, recompute filter; ↑↓ navigate filtered indices; `Tab` opens label popover; `m` toggles mine-only (lazily fetching current user via `IssueProvider::current_user` through a new `Effect::FetchCurrentUser` if not cached); progressive Esc (clear text → clear labels → clear mine → dismiss). Enter selects current filtered item.
- [x] 4.5 Implement label-multiselect popover: simple list of label names with `Space` to toggle, `Enter` to close. Source list is the union of labels across loaded issues.
- [x] 4.6 Apply the same `SearchableListState` refactor to `SpecPickerState` and the spec-kit picker state. Search input row, footer `N/M specs · sorted by …`. Sort cycler operates on the filtered subset.
- [x] 4.7 Update `src/ui/app/app_input.rs` for both spec pickers: route printable chars to the search input, ↑↓ navigate, `s` cycle sort (on filtered subset), Esc clears search first then dismisses.
- [x] 4.8 Update the dialog instruction footers to advertise the new keys: `type` to filter, `Tab` labels (issue), `m` mine (issue), `s` sort (spec).
- [x] 4.9 Add a footer hint to the spec picker showing the source ref (e.g. `reading from origin/master`) when reading from ref, omit when falling back to working tree.

## 5. Manual + automated verification

- [x] 5.1 Run `cargo fmt --check && cargo clippy -- -D warnings && cargo test`. Fix everything before proceeding.
- [x] 5.2 Manual repro of the original bug pre-merge: on a repo with locally-present-but-archived-on-remote OpenSpec change, build & run from `~/.conduit/workspaces/conduit/posh-iris`, Alt+N, confirm the spec picker DOES show the stale change. Document this in the PR description.
- [x] 5.3 Apply this change, rebuild, repeat: confirm the spec picker no longer shows the archived change. Verify the "Syncing with remote…" then "Fetching open issues…" messages appear in order.
- [x] 5.4 Manual filter exercise on the issue picker: type to filter; `Tab` to pick a label; `m` to toggle mine; verify counts; verify Esc progressively clears.
- [x] 5.5 Manual filter exercise on the spec picker: type to filter; `s` to cycle sort on the filtered subset; verify selection stays valid.
- [x] 5.6 Negative cases: repo without `gh` installed (issue phase silently skips); repo with no remote (working-tree fallback path used); dirty `base_path` on default branch (FF skipped, picker still correct); feature-branch `base_path` (FF skipped, picker still correct).
- [x] 5.7 Configure `gitea_hosts` in conduit config pointing at any reachable Gitea instance with `GITEA_TOKEN` set; create a repo, open an issue, run Alt+N, verify the issue appears. (Skip if no Gitea instance available; note in PR.) Same for Forgejo if available.
