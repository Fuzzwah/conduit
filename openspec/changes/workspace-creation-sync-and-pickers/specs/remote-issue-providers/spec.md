## ADDED Requirements

### Requirement: IssueProvider trait abstracts remote issue sources
The system SHALL define an `IssueProvider` trait with at minimum the following methods: `name() -> &'static str`, `supports(remote_url: &str) -> bool`, `fetch_open_issues(repo_path: &Path) -> Vec<RemoteIssue>`, and `current_user(repo_path: &Path) -> Option<String>`. The system SHALL define a `RemoteIssue` struct with at minimum the fields `number: u32`, `title: String`, `labels: Vec<String>`, `assignee_logins: Vec<String>`. All issue-fetching call sites in the workspace-creation flow SHALL use the trait, never a concrete provider type.

#### Scenario: Picker UI consumes only the trait type
- **WHEN** the issue picker receives data
- **THEN** it operates on `RemoteIssue` values; no GitHub-specific type appears in `src/ui/components/issue_picker.rs`

### Requirement: Provider registry selects by remote URL
The system SHALL maintain a registry of available providers and SHALL select the first provider whose `supports(remote_url)` returns true for the repository's `origin` URL. If no provider matches, the system SHALL silently produce an empty issue list (causing the issue phase to skip).

#### Scenario: GitHub URL selects GitHub provider
- **GIVEN** a repository whose `origin` URL contains `github.com`
- **WHEN** the system needs to fetch issues
- **THEN** the GitHub provider is selected

#### Scenario: Unknown host selects no provider
- **GIVEN** a repository whose `origin` URL does not match any registered provider's `supports()`
- **WHEN** the system needs to fetch issues
- **THEN** an empty issue list is returned and the issue picker is skipped

### Requirement: GitHub provider via gh CLI
The system SHALL provide a GitHub implementation of `IssueProvider` that uses the `gh` CLI. `fetch_open_issues` SHALL invoke `gh issue list --json number,title,labels,assignees --state open --limit 50` in the repository directory. `current_user` SHALL invoke `gh api user --jq .login`. Both SHALL return empty/None on any error (missing `gh`, not authenticated, etc.) without propagating errors.

#### Scenario: gh missing yields empty list
- **GIVEN** the `gh` binary is not on PATH
- **WHEN** `fetch_open_issues` is called for a GitHub repo
- **THEN** an empty `Vec<RemoteIssue>` is returned and no error is raised

#### Scenario: Labels and assignees are populated
- **GIVEN** an open GitHub issue with labels `["bug","docs"]` and assignee `octocat`
- **WHEN** `fetch_open_issues` is called
- **THEN** the corresponding `RemoteIssue` has `labels == ["bug", "docs"]` and `assignee_logins == ["octocat"]`

### Requirement: Gitea provider via REST and configurable host allowlist
The system SHALL provide a Gitea implementation of `IssueProvider`. Detection SHALL be by matching the host of the repository's `origin` URL against a configurable allowlist `gitea_hosts: Vec<String>` in conduit's configuration. `fetch_open_issues` SHALL call `GET /api/v1/repos/{owner}/{repo}/issues?state=open&type=issues&limit=50` with the `GITEA_TOKEN` env var as bearer auth when present. `current_user` SHALL call `GET /api/v1/user`. All errors (missing token, network failure, non-2xx response, malformed JSON) SHALL yield an empty result without propagation.

#### Scenario: Configured Gitea host matches
- **GIVEN** conduit config has `gitea_hosts = ["gitea.example.com"]`
- **AND** a repository's `origin` is `git@gitea.example.com:owner/repo.git`
- **WHEN** the registry is queried
- **THEN** the Gitea provider's `supports()` returns true and it is selected

#### Scenario: Missing token yields empty list
- **GIVEN** `GITEA_TOKEN` is unset
- **WHEN** `fetch_open_issues` is called for a Gitea repo
- **THEN** an empty `Vec<RemoteIssue>` is returned

### Requirement: Forgejo provider mirrors Gitea
The system SHALL provide a Forgejo implementation of `IssueProvider` using the same REST shape as Gitea (Forgejo is API-compatible at `/api/v1`). Detection SHALL use a separate allowlist `forgejo_hosts: Vec<String>`. Auth SHALL use `FORGEJO_TOKEN`. Error handling SHALL be identical to the Gitea provider.

#### Scenario: Forgejo host matches independently of Gitea
- **GIVEN** conduit config has `forgejo_hosts = ["codeberg.org"]` and `gitea_hosts` is empty
- **AND** a repository's `origin` is `https://codeberg.org/owner/repo`
- **WHEN** the registry is queried
- **THEN** the Forgejo provider is selected

### Requirement: Current-user resolution is cached per picker session
When the issue picker requests the current user (e.g. for "mine only" filtering), the result SHALL be cached for the lifetime of that picker session and SHALL NOT trigger more than one provider call per session.

#### Scenario: Toggling mine-only twice fetches user only once
- **WHEN** the user toggles "mine only" on, then off, then on again within a single picker session
- **THEN** `IssueProvider::current_user` is invoked at most once

### Requirement: All provider operations are non-blocking on the UI thread
Provider operations (issue fetch, current-user fetch) SHALL be executed off the UI thread (e.g. via `tokio::task::spawn_blocking`). The UI SHALL remain responsive during fetches.

#### Scenario: UI ticks during fetch
- **WHEN** an issue fetch is in progress
- **THEN** the picker spinner advances and Esc still dismisses the dialog
