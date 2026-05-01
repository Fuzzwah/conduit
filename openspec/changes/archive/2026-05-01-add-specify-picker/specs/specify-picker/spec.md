## ADDED Requirements

### Requirement: Detect spec-kit specs during workspace creation
The system SHALL scan `.specify/specs/*/tasks.md` in the repository root when a user initiates workspace creation. For each spec directory, it SHALL count unchecked (`- [ ]`) and checked (`- [x]` / `- [X]`) task lines. It SHALL return only specs with at least one remaining task, sorted by remaining task count descending.

#### Scenario: Repo has spec-kit specs with remaining tasks
- **WHEN** `.specify/specs/` exists and one or more spec directories contain `tasks.md` files with unchecked tasks
- **THEN** `fetch_specify_specs()` returns a non-empty `Vec<SpecifySpec>` sorted by remaining tasks descending

#### Scenario: Repo has no `.specify/specs/` directory
- **WHEN** `.specify/specs/` does not exist in the repository root
- **THEN** `fetch_specify_specs()` returns an empty `Vec<SpecifySpec>`

#### Scenario: All specs are fully completed
- **WHEN** all `tasks.md` files under `.specify/specs/` have only checked tasks
- **THEN** `fetch_specify_specs()` returns an empty `Vec<SpecifySpec>`

### Requirement: Show specify picker when specify specs exist
During workspace creation, if `fetch_specify_specs()` returns a non-empty list, the system SHALL display the specify picker modal. It SHALL NOT display the openspec picker in this case.

#### Scenario: Repo uses spec-kit only
- **WHEN** `.specify/specs/` has incomplete specs and `openspec/changes/` has no incomplete changes
- **THEN** the specify picker is shown

#### Scenario: Repo uses both spec-kit and openspec
- **WHEN** both `.specify/specs/` and `openspec/changes/` have incomplete items
- **THEN** the specify picker is shown and the openspec picker is not shown

#### Scenario: Repo uses openspec only
- **WHEN** `openspec/changes/` has incomplete changes and `.specify/specs/` has no incomplete specs
- **THEN** the openspec picker is shown (existing behaviour unchanged)

#### Scenario: Repo uses neither spec system
- **WHEN** neither `.specify/specs/` nor `openspec/changes/` has incomplete items
- **THEN** neither picker is shown and workspace creation proceeds directly

### Requirement: Specify picker navigation and selection
The specify picker modal SHALL support the same navigation interactions as the openspec picker: directional navigation to move selection, sort cycling, confirmation to select a spec, and cancellation to skip without selecting.

#### Scenario: User navigates the specify picker
- **WHEN** the specify picker is visible with multiple specs
- **THEN** Up/k and Down/j move the selection, and the list scrolls when the selection reaches the viewport boundary

#### Scenario: User confirms a specify spec selection
- **WHEN** the user presses Enter with a spec selected
- **THEN** workspace creation proceeds with the selected spec's folder name used to derive the workspace name and branch name

#### Scenario: User skips the specify picker
- **WHEN** the user presses Esc
- **THEN** workspace creation proceeds without a linked spec, using auto-generated naming

#### Scenario: User cycles sort order
- **WHEN** the user presses s
- **THEN** the sort order cycles through remaining-descending → remaining-ascending → name-ascending

### Requirement: Workspace naming from specify spec
When the user selects a specify spec, the system SHALL use the spec's folder name as both the workspace name and the base for the branch name (via the existing `generate_branch_name` utility).

#### Scenario: Spec selected during workspace creation
- **WHEN** the user selects a specify spec named `my-feature`
- **THEN** the workspace is named `my-feature` and the branch is derived from `generate_branch_name(username, "my-feature")`
