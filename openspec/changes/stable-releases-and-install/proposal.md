## Why

The Fuzzwah/conduit fork ships fork-specific features but has no working release pipeline: install.sh points at upstream, release.yml skips CI and only builds 2 of 4 targets, and build.rs panics with cryptic backtraces when npm is missing. Users cannot install the fork directly or pin to a stable version.

## What Changes

- Bump version to `0.5.0` to start a diverged fork version stream; add `rust-version = "1.83"` and correct `repository` URL in `Cargo.toml`
- Add `rust-toolchain.toml` at repo root so rustup auto-installs the right toolchain
- Rewrite `build.rs` early-exit checks to emit actionable `cargo::error=` messages instead of panicking when `npm`/`node` are missing
- Add `scripts/preflight.sh` — POSIX shell script that checks all build deps and prints copy-paste install commands for missing/outdated tools
- Rewrite `.github/workflows/release.yml` as a 5-stage gate: verify (full CI) → build (4 targets) → smoke-test → release → announce
- Add `on.workflow_call:` to `.github/workflows/ci.yml` to make it reusable by the release pipeline
- Delete `.github/workflows/update-homebrew.yml` (broken; tap doesn't exist)
- Fix `website/public/install.sh`: correct `REPO`, add all 4 targets, add sha256 verification, add `CONDUIT_INSTALL_FILE` and `CONDUIT_VERSION` env overrides
- Update `FORK_INSTALL.md` with a "Stable Release" first section and preflight step
- Update `README.md` install section to lead with the fork's curl one-liner

## Capabilities

### New Capabilities

- `preflight-check`: POSIX shell script (`scripts/preflight.sh`) that probes for all required build deps (`git`, `rustc ≥ 1.83`, `cargo ≥ 1.83`, `node ≥ 18`, `npm`) with OS-aware install hints; warns (non-fatal) if no agent CLI is present
- `verified-release-pipeline`: GitHub Actions release workflow that gates on full CI, builds 4 cross-compiled targets, smoke-tests each artifact, publishes a GitHub release with sha256 sidecars, and announces via Discord with the correct fork URL
- `fork-install`: Updated `website/public/install.sh` supporting all 4 targets, sha256 verification, `CONDUIT_VERSION` pinning, and `CONDUIT_INSTALL_FILE` local-file mode for CI smoke-testing

### Modified Capabilities

## Impact

- `Cargo.toml`: version, rust-version, repository field
- `build.rs`: early npm/node checks (build-time behavior change)
- `.github/workflows/ci.yml`: gains `workflow_call` trigger
- `.github/workflows/release.yml`: full rewrite
- `.github/workflows/update-homebrew.yml`: deleted
- `website/public/install.sh`: behavior changes (new targets, verification, env overrides)
- `rust-toolchain.toml`: new file, affects all `cargo` invocations via rustup
- `scripts/preflight.sh`: new file
- `FORK_INSTALL.md`, `README.md`: documentation only
