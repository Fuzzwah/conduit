## ADDED Requirements

### Requirement: Template files exist under demo/project-template
The repository SHALL contain a `demo/project-template/` directory with the following project files:
- `README.md` — brief description of the `greet` CLI (thin enough that Claude has room to improve it)
- `Cargo.toml` — minimal Rust manifest (`name = "greet"`, `version = "0.1.0"`, `edition = "2021"`)
- `src/main.rs` — a `main()` that prints a greeting

#### Scenario: Template directory is present after checkout
- **WHEN** the repository is cloned
- **THEN** `demo/project-template/README.md`, `demo/project-template/Cargo.toml`, and `demo/project-template/src/main.rs` all exist

### Requirement: Template includes a complete OpenSpec update-readme change
The `demo/project-template/` directory SHALL contain a complete spec-driven OpenSpec change at `openspec/changes/update-readme/` with all four required artifacts: `proposal.md`, `design.md`, `specs/update-readme/spec.md`, and `tasks.md`.

#### Scenario: All four artifacts are present in the template
- **WHEN** the repository is cloned
- **THEN** all of the following exist:
  - `demo/project-template/openspec/changes/update-readme/proposal.md`
  - `demo/project-template/openspec/changes/update-readme/design.md`
  - `demo/project-template/openspec/changes/update-readme/specs/update-readme/spec.md`
  - `demo/project-template/openspec/changes/update-readme/tasks.md`

### Requirement: update-readme tasks are completable by Claude in one short session
The `tasks.md` SHALL contain exactly two unchecked tasks that Claude can complete with simple file edits to `README.md`: adding a `## Recent Updates` section and adding a one-line project description.

#### Scenario: Tasks are achievable without compilation or external tools
- **WHEN** Claude is given the update-readme spec
- **THEN** it can complete both tasks by editing only `README.md` with no build step required

### Requirement: Seed script builds fixture from template in one copy step
`workflow-seed.sh` SHALL copy `demo/project-template/` into the fixture clone using `cp -r` and commit in three structured commits, replacing the current inline heredoc approach.

#### Scenario: Fixture has three-commit history after seeding
- **WHEN** `bash demo/workflow-seed.sh` completes
- **THEN** `git -C demo/fixtures/project log --oneline` shows exactly three commits with messages matching "feat: add greet CLI", "docs: add README", and "feat: add update-readme spec"

### Requirement: workflow.tape records agent completion, not manual shell steps
`workflow.tape` SHALL NOT contain the `Type "!"` shell mode section or manual `git commit` commands. After workspace creation it SHALL sleep long enough for the Claude agent to complete the task autonomously.

#### Scenario: Tape contains no manual shell commands for the edit step
- **WHEN** `workflow.tape` is inspected
- **THEN** there is no `Type "!"` line and no `Type "git add"` or `Type "git commit"` line
