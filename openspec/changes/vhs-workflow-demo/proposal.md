## Why

Conduit lacks a polished animated demo showing its end-to-end workflow, making it harder for new users to understand the value proposition at a glance. A VHS-driven GIF demo, generated from checked-in tape scripts and fixture state, gives the project a reproducible, updatable showcase without requiring manual screen recording.

## What Changes

- Add a `demo/` directory at the repo root containing VHS tape scripts, a fixture-seeding shell script, and generated GIFs
- Tape scripts drive the real `conduit` binary through a full workflow: add project → create workspace → make a code change → commit → open PR → merge → archive workspace
- A `demo/seed.sh` script creates the required prerequisite state (a local bare git repo acting as the remote, a working-tree clone, dummy API key env vars) so the demo runs deterministically without a live GitHub account or Claude subscription
- GIFs are generated locally via `vhs` and committed (or optionally regenerated in CI) so they can be embedded directly in `README.md` and the Astro marketing site
- The flow is split into focused clips rather than one monolithic recording, so individual steps can be updated without re-recording everything

## Capabilities

### New Capabilities

- `vhs-demo`: A reproducible, script-driven animated GIF demo system covering the full conduit add-project → workspace → change → commit → PR → merge → archive workflow

### Modified Capabilities

<!-- None — no existing spec-level behavior changes -->

## Impact

- **New files**: `demo/` directory with tape scripts (`*.tape`), `seed.sh`, `teardown.sh`, generated `*.gif` output, and a `README.md` explaining how to regenerate
- **Existing files**: `README.md` and `website/` may gain `<img>` tags pointing at the committed GIFs; no Rust source changes
- **Dependencies**: `vhs` must be installed (already present); no new Cargo deps
- **CI**: Optional — a CI job can regenerate and diff GIFs on tape-script changes; not required for the initial implementation
- **Risks**: GIF file size if clips are too long; mitigated by splitting into short focused segments and tuning VHS `SpeedFactor`
