## ADDED Requirements

### Requirement: Release pipeline gates on full CI before building artifacts
The system SHALL run the full CI suite (fmt check, clippy, tests, e2e) as the first stage of the release pipeline, and SHALL NOT produce any binary artifacts if CI fails.

#### Scenario: Tag pushed against code with a clippy warning
- **WHEN** a git tag is pushed and the tagged commit fails `cargo clippy -- -D warnings`
- **THEN** the `verify` job SHALL fail, all subsequent jobs SHALL be skipped, and no binaries SHALL be uploaded or released

#### Scenario: Tag pushed against green code
- **WHEN** a git tag is pushed and the tagged commit passes all CI checks
- **THEN** the `verify` job SHALL succeed and the `build` job SHALL be triggered

### Requirement: Release pipeline builds four binary targets
The system SHALL produce release binaries for all four supported targets in a matrix build job.

#### Scenario: All four targets built
- **WHEN** the `build` job runs after a successful `verify`
- **THEN** the following targets SHALL each produce a `conduit-<target>.tar.gz` and a `conduit-<target>.tar.gz.sha256` sidecar:
  - `x86_64-unknown-linux-musl`
  - `aarch64-unknown-linux-musl`
  - `aarch64-apple-darwin`
  - `x86_64-apple-darwin`

#### Scenario: Individual target build failure
- **WHEN** any one target in the build matrix fails
- **THEN** the entire `build` job SHALL be marked failed and the `smoke-test` and `release` jobs SHALL not run

### Requirement: Release pipeline smoke-tests each artifact before publishing
The system SHALL run smoke tests against the built artifacts in clean container environments before creating the GitHub release.

#### Scenario: Linux x64 musl binary smoke test
- **WHEN** the `smoke-test` job runs
- **THEN** the `x86_64-unknown-linux-musl` binary SHALL be downloaded, extracted, and run as `./conduit --version` inside both `ubuntu:22.04` and `alpine:3.19` containers, and the output SHALL match the release tag

#### Scenario: Linux arm64 musl binary smoke test
- **WHEN** the `smoke-test` job runs
- **THEN** the `aarch64-unknown-linux-musl` binary SHALL be tested in a `linux/arm64` container via QEMU emulation, running `./conduit --version` with output matching the release tag

#### Scenario: install.sh smoke test using pre-staged artifact
- **WHEN** the `smoke-test` job runs
- **THEN** install.sh SHALL be executed with `CONDUIT_INSTALL_FILE` pointing at the pre-staged artifact archive (bypassing the GitHub API), and `conduit --version` SHALL succeed in a clean `ubuntu:22.04` container

### Requirement: Release pipeline publishes GitHub release with all artifacts and sha256 sidecars
The system SHALL create a GitHub release only after all smoke tests pass, attaching all 4 `.tar.gz` archives and their 4 `.sha256` sidecars.

#### Scenario: Successful release publication
- **WHEN** all smoke tests pass
- **THEN** a GitHub release SHALL be created with the tag name, auto-generated release notes, and all 8 files (4 archives + 4 sha256 files) attached as release assets

### Requirement: Release pipeline announces to Discord with correct fork URL
The system SHALL send a Discord webhook notification after a successful release pointing to the Fuzzwah/conduit GitHub releases page.

#### Scenario: Discord announcement URL
- **WHEN** a release is published successfully
- **THEN** the Discord message SHALL contain a URL in the form `https://github.com/Fuzzwah/conduit/releases/tag/<tag>` (not `conduit-cli/conduit`)

### Requirement: ci.yml is callable as a reusable workflow
The system SHALL add `on.workflow_call:` to `.github/workflows/ci.yml` so the release pipeline can invoke it without duplicating job definitions.

#### Scenario: release.yml calls ci.yml
- **WHEN** the release pipeline runs its `verify` stage
- **THEN** it SHALL invoke `ci.yml` via `uses: ./.github/workflows/ci.yml` and the full CI suite SHALL run exactly as it does on pull requests

### Requirement: Broken Homebrew workflow is removed
The system SHALL NOT contain `.github/workflows/update-homebrew.yml`.

#### Scenario: Repository does not contain the broken workflow
- **WHEN** the change is applied
- **THEN** `.github/workflows/update-homebrew.yml` SHALL not exist in the repository
