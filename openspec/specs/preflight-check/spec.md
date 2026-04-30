## ADDED Requirements

### Requirement: Preflight script checks required build dependencies
The system SHALL provide a POSIX-compatible shell script at `scripts/preflight.sh` that checks all required tools for building conduit from source and reports their status.

#### Scenario: All dependencies present and at minimum versions
- **WHEN** the user runs `bash scripts/preflight.sh` with all required tools installed at or above minimum versions
- **THEN** the script SHALL print an `OK` line for each tool with its detected version and exit with code 0

#### Scenario: Missing required dependency
- **WHEN** the user runs the script and a required tool (`git`, `rustc`, `cargo`, `node`, or `npm`) is not found on `PATH`
- **THEN** the script SHALL print a `MISSING` line for that tool, print a copy-paste install command appropriate for the detected OS, and exit with code 1

#### Scenario: Outdated required dependency
- **WHEN** the user runs the script and `rustc` or `cargo` is below 1.83, or `node` is below 18
- **THEN** the script SHALL print an `OUTDATED` line with the detected version and the minimum required version, print an upgrade command, and exit with code 1

#### Scenario: No agent CLI present
- **WHEN** none of `claude`, `codex`, `gemini`, `opencode`, `copilot`, `pi`, or `dirac` are on `PATH`
- **THEN** the script SHALL print a `WARN` line noting that no agent CLI was found, and SHALL NOT exit non-zero on this condition alone

### Requirement: Preflight script detects OS and provides tailored install commands
The system SHALL detect the host operating system and provide install commands appropriate to that OS for each missing or outdated dependency.

#### Scenario: Linux with apt-get
- **WHEN** `uname -s` returns `Linux` and `apt-get` is available
- **THEN** install commands for missing packages SHALL use `apt-get install`

#### Scenario: Linux with brew
- **WHEN** `uname -s` returns `Linux` and `brew` is available
- **THEN** install commands SHALL prefer `brew install`

#### Scenario: macOS
- **WHEN** `uname -s` returns `Darwin`
- **THEN** install commands SHALL use `brew install` for non-Rust tools and `rustup update stable` for Rust

#### Scenario: Rust toolchain via rustup
- **WHEN** `rustc` or `cargo` is missing or outdated on any OS
- **THEN** the install command SHALL reference `rustup` (or the rustup install URL if rustup itself is absent)

### Requirement: Preflight script uses consistent output formatting
The system SHALL use color-coded output helpers (`info`, `ok`, `warn`, `error`) consistent with the pattern used in `website/public/install.sh`.

#### Scenario: Color output in terminal
- **WHEN** stdout is a TTY
- **THEN** `OK` lines SHALL be green, `WARN` lines yellow, `MISSING`/`OUTDATED`/error lines red

#### Scenario: Non-TTY output
- **WHEN** stdout is not a TTY (e.g., piped to a file)
- **THEN** the script SHALL omit ANSI escape codes

### Requirement: build.rs emits actionable error messages for missing npm or node
The system SHALL check for `npm` and `node` early in `build.rs` and emit a `cargo::error=` message with installation guidance if either is missing, rather than panicking.

#### Scenario: npm not on PATH during cargo build
- **WHEN** the user runs `cargo build` and `npm` is not found on `PATH`
- **THEN** cargo SHALL print a single error line: "npm not found. Install Node.js v18+ (https://nodejs.org/) or run scripts/preflight.sh for setup help." and the build SHALL fail without a backtrace

#### Scenario: node not on PATH during cargo build
- **WHEN** the user runs `cargo build` and `node` is not found on `PATH`
- **THEN** cargo SHALL print a single error line naming the missing tool with a reference to Node.js installation and the build SHALL fail without a backtrace

### Requirement: Cargo.toml enforces minimum Rust version
The system SHALL declare `rust-version = "1.83"` in `Cargo.toml` so that users on older Cargo versions receive an immediate, named error.

#### Scenario: User runs cargo build with Rust 1.82
- **WHEN** a user invokes `cargo build` with Rust 1.82
- **THEN** Cargo SHALL emit an error stating the package requires Rust 1.83 or higher, before attempting dependency resolution

### Requirement: rust-toolchain.toml pins stable toolchain for rustup users
The system SHALL provide a `rust-toolchain.toml` at the repo root that pins the toolchain to `stable` with `rustfmt` and `clippy` components.

#### Scenario: User with rustup builds from source
- **WHEN** a user with `rustup` clones the repo and runs any `cargo` command
- **THEN** rustup SHALL automatically fetch the `stable` toolchain (with `rustfmt` and `clippy`) if not already present, without requiring a manual `rustup install`
