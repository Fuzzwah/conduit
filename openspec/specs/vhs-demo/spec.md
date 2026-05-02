# Capability: vhs-demo

## Requirements

### Requirement: Demo seed environment
The system SHALL provide a `demo/seed.sh` script that creates all prerequisite state required to run the demo tapes from a clean machine. Seed state includes a local bare git repository acting as a fake remote, a working-tree clone of that repo, a conduit data directory initialised with no projects, and a `gh` shim script that simulates `gh pr create` and `gh pr merge` without network access.

#### Scenario: Seed creates fake remote
- **WHEN** `bash demo/seed.sh` is run on a machine with `git` and `conduit` installed
- **THEN** `demo/fixtures/remote.git` exists as a bare git repository with at least one commit on `main`

#### Scenario: Seed creates working clone
- **WHEN** `bash demo/seed.sh` is run
- **THEN** `demo/fixtures/project/` exists as a working-tree clone of `demo/fixtures/remote.git` with `main` checked out

#### Scenario: Seed installs gh shim
- **WHEN** `bash demo/seed.sh` is run
- **THEN** `demo/fixtures/bin/gh` exists and is executable; running `demo/fixtures/bin/gh pr create` prints a fake PR URL and exits 0; running `demo/fixtures/bin/gh pr merge` exits 0

#### Scenario: Seed is idempotent
- **WHEN** `bash demo/seed.sh` is run a second time without running teardown first
- **THEN** the script completes without error, overwriting or skipping existing fixtures

### Requirement: Demo teardown
The system SHALL provide a `demo/teardown.sh` script that removes all state created by `seed.sh` so the demo can be re-run from scratch.

#### Scenario: Teardown removes fixtures
- **WHEN** `bash demo/teardown.sh` is run after `seed.sh`
- **THEN** `demo/fixtures/` is deleted and conduit's demo data directory is removed

### Requirement: Shared VHS settings
The system SHALL provide a `demo/common.tape` file that defines shared VHS settings (terminal dimensions, font, theme, speed) sourced by all clip tapes, ensuring visual consistency across all clips.

#### Scenario: Common settings applied
- **WHEN** a clip tape begins with `Source common.tape`
- **THEN** all clips render at the same dimensions, font size, and colour theme

### Requirement: Six demo clip tapes
The system SHALL provide six VHS tape scripts in `demo/`, numbered `01` through `06`, each covering one step of the workflow. Each tape MUST be runnable independently after `seed.sh` has been executed, and MUST produce a GIF in `demo/output/`.

#### Scenario: Clip 01 — add project
- **WHEN** `vhs demo/01-add-project.tape` is run
- **THEN** `demo/output/01-add-project.gif` is created showing a user adding the fixture project to conduit and the project appearing in the project list

#### Scenario: Clip 02 — create workspace
- **WHEN** `vhs demo/02-create-workspace.tape` is run
- **THEN** `demo/output/02-create-workspace.gif` is created showing a user creating a new workspace for the fixture project and the workspace becoming active in the TUI

#### Scenario: Clip 03 — make a change
- **WHEN** `vhs demo/03-make-change.tape` is run
- **THEN** `demo/output/03-make-change.gif` is created showing a user editing a file in the workspace (e.g. appending a line to README.md) using the conduit TUI or a prompted editor session

#### Scenario: Clip 04 — commit
- **WHEN** `vhs demo/04-commit.tape` is run
- **THEN** `demo/output/04-commit.gif` is created showing a user staging and committing the change through conduit's interface

#### Scenario: Clip 05 — PR
- **WHEN** `vhs demo/05-pr.tape` is run
- **THEN** `demo/output/05-pr.gif` is created showing a user opening a pull request (via the `gh` shim) and the fake PR URL appearing in the TUI

#### Scenario: Clip 06 — merge and archive
- **WHEN** `vhs demo/06-merge-archive.tape` is run
- **THEN** `demo/output/06-merge-archive.gif` is created showing the PR being merged and the workspace being archived, ending with the workspace absent from the active list

### Requirement: Generated GIFs committed to repository
The system SHALL commit all generated GIF files under `demo/output/` to the repository so they can be embedded in documentation without requiring VHS to be installed.

#### Scenario: GIFs present after initial generation
- **WHEN** all six tapes have been run via `bash demo/generate.sh`
- **THEN** `demo/output/01-add-project.gif` through `demo/output/06-merge-archive.gif` exist and are non-empty

#### Scenario: GIFs marked as binary in git
- **WHEN** `git diff demo/output/*.gif` is run
- **THEN** git reports the files as binary, producing no text diff noise

### Requirement: Generation convenience script
The system SHALL provide a `demo/generate.sh` script that runs `seed.sh`, executes all six tapes in order, and reports success or failure per clip.

#### Scenario: Full generation run
- **WHEN** `bash demo/generate.sh` is run on a machine with `vhs`, `git`, and `conduit` available
- **THEN** all six GIFs are produced in `demo/output/` and the script exits 0

#### Scenario: Partial failure reported
- **WHEN** one tape fails (e.g. VHS render error)
- **THEN** `generate.sh` reports which clip failed, continues with remaining clips, and exits non-zero

### Requirement: Demo documentation
The system SHALL provide a `demo/README.md` explaining prerequisites, how to regenerate the GIFs, and how to embed them in other docs.

#### Scenario: README covers prerequisites
- **WHEN** a developer reads `demo/README.md`
- **THEN** they can identify the required tools (vhs, git, conduit binary) and the VHS version pinned for this demo
