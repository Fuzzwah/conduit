## Context

Fuzzwah/conduit is a personal fork of conduit-cli/conduit with 35+ fork-specific features. The existing release infrastructure was copied from upstream without being adapted: install.sh points at the wrong repo, release.yml skips CI and only builds 2 of 4 targets, and the Homebrew workflow references a tap that doesn't exist. Additionally, `build.rs` panics with Rust backtraces when `npm` is not installed, producing a 30-line error that gives users no actionable guidance.

The goal is to ship a first `v0.5.0` release that users can pin to, download with a single `curl` command, and verify via sha256.

## Goals / Non-Goals

**Goals:**
- Full CI gate before any release artifact is produced (no tag-and-hope)
- 4 binary targets: `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `aarch64-apple-darwin`, `x86_64-apple-darwin`
- sha256 sidecar per artifact; install.sh verifies before extract
- Source-build users see actionable error messages when deps are missing
- `CONDUIT_VERSION` env pin and `CONDUIT_INSTALL_FILE` local-file override in install.sh
- Discord announce points at Fuzzwah/conduit

**Non-Goals:**
- Apple notarization / Gatekeeper bypass (document workaround instead)
- Windows binary targets
- Homebrew tap (revisit when Fuzzwah/homebrew-tap exists)
- Auto-update / self-upgrade mechanism

## Decisions

### D1: Reuse ci.yml as a reusable workflow rather than duplicating steps

**Decision:** Add `on.workflow_call:` to the existing `ci.yml` and call it from `release.yml` via `uses: ./.github/workflows/ci.yml`.

**Rationale:** Keeps the CI definition as a single source of truth. Any future lint or test addition automatically applies to both PR and release gates.

**Alternative considered:** Copy-pasting job steps into release.yml. Rejected because divergence is inevitable over time and a release that skips a new test category would go unnoticed.

### D2: Cross-compile Linux arm64 on x64 runners via `cross` crate; cross-compile Intel Mac on arm64 Mac runners via native target install

**Decision:** Use `cross` (cargo wrapper + Docker) for `aarch64-unknown-linux-musl` on Linux x64 runners. For `x86_64-apple-darwin` on Apple Silicon runners, install the target with `rustup target add x86_64-apple-darwin` and compile natively — no emulation needed on macOS.

**Rationale:** `cross` is the established approach for musl cross-compilation on Linux CI; it handles sysroot setup transparently. On macOS, the SDK ships both architectures, so `--target x86_64-apple-darwin` works without Docker.

**Alternative considered:** QEMU for arm64 Linux. Rejected: build time would be 5-10× slower; `cross` uses a native-ISA host compiler with a cross-sysroot, which is much faster.

**Risk:** `codex-protocol` uses git deps that may not build under the `cross` musl toolchain if they pull in C libraries. Mitigation: if `aarch64-unknown-linux-musl` cross-compile fails, fall back to a native arm64 Linux runner.

### D3: smoke-test uses `actions/download-artifact` + `CONDUIT_INSTALL_FILE` to avoid a circular dependency on the GitHub release

**Decision:** The smoke-test job downloads build artifacts directly from the GitHub Actions artifact store (before the release is published) and runs install.sh with `CONDUIT_INSTALL_FILE=<path>` pointing at the local archive.

**Rationale:** If smoke-test ran *after* the release was published and downloaded from GitHub Releases, a failed smoke test could not retract the release automatically. Pre-staging keeps the gate before the publish step.

**Alternative considered:** Post-release validation job that marks the release as draft on failure. Rejected as more complex and still leaves a publicly-visible bad release for a window of time.

### D4: MSRV set to 1.83 with validation step before merge

**Decision:** Set `rust-version = "1.83"` in Cargo.toml and `channel = "stable"` in `rust-toolchain.toml`. Validate by running `cargo +1.82 build` during implementation; bump to the actual minimum if higher.

**Rationale:** The "Cargo 1.82 fails" user report established that 1.82 is broken; 1.83 is the next stable release and likely sufficient. Using `rust-version` in Cargo.toml makes the error from old Cargo versions immediate and named.

### D5: Delete update-homebrew.yml rather than disable

**Decision:** Remove the file entirely.

**Rationale:** A disabled workflow file that references non-existent secrets and a non-existent tap will confuse contributors. Re-introducing it when the tap exists is trivial; leaving it causes noise and false confidence.

## Risks / Trade-offs

- **Cross-compile linker failures for codex-protocol git deps** → Mitigation: fall back to native arm64 Linux runner (`warp-ubuntu-latest-arm64-*` if available in Warp).
- **Gatekeeper on unsigned macOS binaries** → Document `xattr -d com.apple.quarantine conduit` in FORK_INSTALL.md; out of scope for this change.
- **MSRV value wrong** → Validation step 2 in verification plan catches this before the PR merges; easy one-line bump.
- **`warp-macos-26-arm64-12x` runner availability** → If the runner slug changes, builds silently queue forever. Mitigation: confirm runner name from current working release.yml before merging.

## Migration Plan

1. Implement changes on feature branch `fuz/icy-oak`.
2. Run `cargo fmt --check && cargo clippy -- -D warnings && cargo test` locally.
3. Open PR targeting `master`; confirm CI green.
4. Push tag `v0.5.0-rc.1` to a throwaway branch; observe full release pipeline; inspect draft release for 4 archives + 4 `.sha256` files; delete the rc release.
5. Merge PR; tag `v0.5.0`; watch live pipeline; confirm Discord post.

**Rollback:** If the release pipeline fails after tagging, delete the GitHub release draft and the tag. The code change itself (Cargo.toml version bump, etc.) has no runtime effect on existing installs.

## Open Questions

- What is the exact Warp runner slug for arm64 Linux? (Needed for `aarch64-unknown-linux-musl` native fallback.)
- Does `codex-protocol` build cleanly under `cross` + `aarch64-unknown-linux-musl`? Unknown until attempted.
- Is `DISCORD_RELEASE_WEBHOOK` already set as a secret in Fuzzwah/conduit? User must confirm before first release.
