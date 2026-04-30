## ADDED Requirements

### Requirement: install.sh targets Fuzzwah/conduit repository
The system SHALL set `REPO="Fuzzwah/conduit"` in `website/public/install.sh` so all API calls and download URLs reference the fork, not upstream.

#### Scenario: Install script fetches from correct repository
- **WHEN** a user runs the install script
- **THEN** all GitHub API calls and asset download URLs SHALL reference `Fuzzwah/conduit`

### Requirement: install.sh supports all four binary targets
The system SHALL detect the host architecture and OS and map it to the correct release asset name for all four supported targets.

#### Scenario: Linux x86_64 install
- **WHEN** `uname -m` returns `x86_64` on Linux
- **THEN** the script SHALL download `conduit-x86_64-unknown-linux-musl.tar.gz`

#### Scenario: Linux aarch64 install
- **WHEN** `uname -m` returns `aarch64` or `arm64` on Linux
- **THEN** the script SHALL download `conduit-aarch64-unknown-linux-musl.tar.gz`

#### Scenario: macOS Apple Silicon install
- **WHEN** `uname -m` returns `arm64` on macOS (`uname -s` returns `Darwin`)
- **THEN** the script SHALL download `conduit-aarch64-apple-darwin.tar.gz`

#### Scenario: macOS Intel install
- **WHEN** `uname -m` returns `x86_64` on macOS
- **THEN** the script SHALL download `conduit-x86_64-apple-darwin.tar.gz`

#### Scenario: Unsupported platform
- **WHEN** the detected OS/arch combination has no corresponding release asset
- **THEN** the script SHALL print an error message naming the unsupported platform and exit with code 1

### Requirement: install.sh verifies sha256 checksum before extracting
The system SHALL download the `.sha256` sidecar for the selected asset and verify the checksum before extracting the archive.

#### Scenario: Checksum matches
- **WHEN** the downloaded archive checksum matches the sidecar
- **THEN** the script SHALL proceed to extract and install the binary

#### Scenario: Checksum mismatch
- **WHEN** the downloaded archive checksum does not match the sidecar
- **THEN** the script SHALL print an error message indicating checksum failure, delete the downloaded file, and exit with code 1 without extracting

### Requirement: install.sh supports CONDUIT_VERSION environment variable for pinned installs
The system SHALL respect a `CONDUIT_VERSION` environment variable to install a specific release version instead of the latest.

#### Scenario: User pins a version
- **WHEN** the user sets `CONDUIT_VERSION=v0.5.0` before running the install script
- **THEN** the script SHALL fetch and install `v0.5.0` instead of querying for the latest release

#### Scenario: No version specified
- **WHEN** `CONDUIT_VERSION` is not set
- **THEN** the script SHALL query the GitHub releases API and install the latest stable release

### Requirement: install.sh supports CONDUIT_INSTALL_FILE for local archive installs
The system SHALL respect a `CONDUIT_INSTALL_FILE` environment variable to install from a local archive file, bypassing the GitHub API and download steps.

#### Scenario: Local file install (CI smoke test mode)
- **WHEN** the user sets `CONDUIT_INSTALL_FILE=/path/to/conduit-x86_64-unknown-linux-musl.tar.gz`
- **THEN** the script SHALL use the specified local file as the archive source, skip all GitHub API calls and downloads, and proceed directly to extraction and installation

#### Scenario: Local file does not exist
- **WHEN** `CONDUIT_INSTALL_FILE` is set but the path does not point to an existing file
- **THEN** the script SHALL print an error and exit with code 1
