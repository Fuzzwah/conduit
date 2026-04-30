## 1. Versioning and Toolchain

- [x] 1.1 In `Cargo.toml`: bump `version` to `"0.5.0"`, add `rust-version = "1.83"`, update `repository` to `"https://github.com/Fuzzwah/conduit"`
- [x] 1.2 Create `rust-toolchain.toml` at repo root with `channel = "stable"`, `components = ["rustfmt", "clippy"]`, `profile = "minimal"`
- [x] 1.3 Validate MSRV: `is_multiple_of` (used in ui/mod.rs and ui/app.rs) requires Rust 1.87; bumped `rust-version` to `"1.87"` and updated preflight.sh accordingly

## 2. Build Script Improvements

- [x] 2.1 Promote the `which` crate from runtime `[dependencies]` to also appear in `[build-dependencies]` in `Cargo.toml`
- [x] 2.2 In `build.rs`: add early checks using `which::which("node")` and `which::which("npm")` before any `Command::new` calls; emit `cargo::error=` with actionable messages (referencing Node.js install URL and `scripts/preflight.sh`) and return early on failure
- [x] 2.3 In `build.rs`: replace all `.expect("Failed to run npm install")` and similar panic calls with `cargo::error=` emissions that name the missing tool
- [x] 2.4 Verify: run `PATH=/usr/bin cargo build` (no npm) and confirm a single readable error line appears with no backtrace

## 3. Preflight Script

- [x] 3.1 Create `scripts/preflight.sh` as a POSIX `sh` script (executable bit set); copy color/info/warn/error helper functions from `website/public/install.sh`
- [x] 3.2 Implement OS detection via `uname -s` and package manager detection (`apt-get`, `brew`, `dnf`, `pacman`)
- [x] 3.3 Add checks for `git` (any version), `rustc ≥ 1.83`, `cargo ≥ 1.83`, `node ≥ 18`, `npm` (any version); print OK/MISSING/OUTDATED with version and copy-paste install command per tool
- [x] 3.4 Add non-fatal agent CLI check: warn if none of `claude`, `codex`, `gemini`, `opencode`, `copilot`, `pi`, `dirac` are on PATH
- [x] 3.5 Exit 0 if all required deps satisfied; exit 1 if any required dep is missing or outdated
- [x] 3.6 Verify: run `bash scripts/preflight.sh` on the dev machine and confirm all-green output and exit code 0

## 4. CI Workflow: Make ci.yml Reusable

- [x] 4.1 In `.github/workflows/ci.yml`: add `on: workflow_call:` alongside the existing `push` and `pull_request` triggers
- [x] 4.2 Open a test PR or push to a non-main branch to confirm ci.yml still runs normally as a PR check

## 5. Release Workflow: Rewrite release.yml

- [x] 5.1 Confirm the Warp runner slugs currently used in `release.yml` are correct (`warp-ubuntu-latest-x64-8x`, `warp-macos-26-arm64-12x`) — update if needed
- [x] 5.2 Add `verify` job: `uses: ./.github/workflows/ci.yml` triggered on `push: tags: ['v*']`
- [x] 5.3 Add `build` job (`needs: verify`) with a 4-target matrix:
  - `x86_64-unknown-linux-musl` on Linux x64 (native musl with `musl-tools`)
  - `aarch64-unknown-linux-musl` on Linux x64 (cross-compile via `cross` crate)
  - `aarch64-apple-darwin` on macOS arm64 (native)
  - `x86_64-apple-darwin` on macOS arm64 (`rustup target add x86_64-apple-darwin`, then `cargo build --target`)
- [x] 5.4 In each `build` matrix step: produce `conduit-<target>.tar.gz` and generate `conduit-<target>.tar.gz.sha256` sidecar via `sha256sum` (Linux) or `shasum -a 256` (macOS); upload both as Actions artifacts
- [x] 5.5 Add `smoke-test` job (`needs: build`):
  - Linux x64: `ubuntu:22.04` and `alpine:3.19` containers — download artifact, extract, run `./conduit --version`, assert output matches tag
  - Linux arm64: `docker/setup-qemu-action` + `linux/arm64` container — same assert
  - install.sh: clean `ubuntu:22.04` container, download artifact via `actions/download-artifact`, run `CONDUIT_INSTALL_FILE=<path> bash website/public/install.sh`, run `conduit --version`
- [x] 5.6 Add `release` job (`needs: smoke-test`): use `softprops/action-gh-release@v2` with `generate_release_notes: true`; attach all 8 files (4 archives + 4 sha256 sidecars)
- [x] 5.7 Add `announce` job (`needs: release`): Discord webhook using `DISCORD_RELEASE_WEBHOOK` secret; message URL must be `https://github.com/Fuzzwah/conduit/releases/tag/${{ github.ref_name }}`

## 6. Delete Broken Homebrew Workflow

- [x] 6.1 Delete `.github/workflows/update-homebrew.yml`
- [x] 6.2 Confirm `HOMEBREW_TAP_TOKEN` secret is no longer referenced anywhere in `.github/workflows/`

## 7. Install Script Updates

- [x] 7.1 In `website/public/install.sh`: change `REPO="conduit-cli/conduit"` → `REPO="Fuzzwah/conduit"`
- [x] 7.2 Add `aarch64`/`arm64` → `aarch64-unknown-linux-musl` mapping in the Linux architecture branch
- [x] 7.3 Add `x86_64` → `x86_64-apple-darwin` mapping in the macOS architecture branch
- [x] 7.4 After download: fetch `<asset>.sha256` sidecar and verify with `sha256sum -c` (Linux) or `shasum -a 256 -c` (macOS); hard-fail on mismatch with error message
- [x] 7.5 Add `CONDUIT_INSTALL_FILE` env override: if set, skip GitHub API + download steps and use the local path as the archive source; error if path does not exist
- [x] 7.6 Add `CONDUIT_VERSION` env override: if set, use that tag for the API call and asset URL instead of querying for latest

## 8. Documentation Updates

- [x] 8.1 In `FORK_INSTALL.md`: add a "Stable Release (recommended)" section at the top with the curl one-liner (`curl -fsSL https://raw.githubusercontent.com/Fuzzwah/conduit/master/website/public/install.sh | sh`); demote "From Source" to a second section; add `bash scripts/preflight.sh` as step 0 of the source-build path
- [x] 8.2 In `FORK_INSTALL.md`: replace the manual "Cargo 1.82 fails" warning with a one-line note that `rust-toolchain.toml` and `rust-version` enforce the minimum automatically
- [x] 8.3 In `README.md`: update install section so the fork's curl one-liner is the first/primary option; source build remains as an alternative linking to `FORK_INSTALL.md`
- [x] 8.4 In `FORK_INSTALL.md`: document the Gatekeeper workaround for unsigned macOS binaries (`xattr -d com.apple.quarantine conduit`)

## 9. Verification

- [x] 9.1 Run `cargo fmt --check && cargo clippy -- -D warnings && cargo test` and confirm all pass
- [x] 9.2 Push tag `v0.5.0-rc.1` to a test branch; watch the full `verify → build → smoke-test → release → announce` pipeline; inspect draft release for 4 archives + 4 `.sha256` files; confirm Discord URL is correct; delete rc release
- [x] 9.3 On a fresh `ubuntu:22.04` container outside CI: run `curl -fsSL .../install.sh | CONDUIT_VERSION=v0.5.0-rc.1 sh && conduit --version`; repeat in `alpine:3.19`
- [x] 9.4 Tag `v0.5.0` on master after PR merge; confirm full pipeline succeeds and Discord post lands
