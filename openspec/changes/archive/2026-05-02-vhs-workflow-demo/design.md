## Context

VHS (v0.11.0, installed at `/home/linuxbrew/.linuxbrew/bin/vhs`) drives a terminal session from a `.tape` script and outputs a GIF. Conduit is a TUI application backed by a SQLite database and a local git worktree per workspace. To demo the full workflow reproducibly, the environment must supply a fake remote git repository, a pre-cloned working tree, and stub env vars so conduit launches without a live Claude API key or GitHub token.

The previous VHS usage in this project was for static screenshots; this change extends that pattern to animated GIF clips.

## Goals / Non-Goals

**Goals:**
- Six short, focused GIF clips (30–90 seconds each) covering: add project, create workspace, make a code change, commit, open PR, merge & archive
- A `demo/seed.sh` script that creates all prerequisite state from scratch (bare repo, clone, conduit DB bootstrap) so the demo runs identically on any machine with `vhs` + `git` + `conduit` installed
- A `demo/teardown.sh` to clean up seed state between runs
- GIFs committed to `demo/output/` so they can be embedded in docs without requiring vhs to be installed
- A `demo/README.md` explaining how to regenerate

**Non-Goals:**
- CI-enforced GIF regeneration (nice to have later; too fragile for a first pass)
- Demoing the web UI (`conduit serve`) — TUI only
- Live GitHub API calls — the PR/merge steps use `gh` pointed at a local gitea/bare-repo stub or are simulated via direct git pushes and the conduit "mark merged" flow
- Subtitles or overlay graphics — plain terminal output only

## Decisions

### Split into six clips, not one long video
**Rationale:** A single 5-minute GIF would be 50–100 MB and unusable. Six clips of 30–90 s each stay under ~5 MB each, can be embedded individually in docs, and can be re-recorded independently when a step's UI changes. The clips are numbered (`01-add-project.gif` … `06-archive.gif`) so they compose naturally in documentation.

### Fake remote via local bare git repo, not GitHub
**Rationale:** A real GitHub repo would require auth tokens and network access, making the demo non-reproducible. A local bare repo (`demo/fixtures/remote.git`) created by `seed.sh` behaves identically to a real remote for all git operations conduit performs. The conduit "Open PR" action that calls `gh pr create` is replaced in the demo by a direct git merge + conduit's workspace status check — or `seed.sh` installs a `gh` shim that prints a fake PR URL and exits 0.

**Alternative considered:** Gitea in Docker — rejected as too heavyweight a prerequisite.

### Pre-built binary path hardcoded in tapes as `./target/debug/conduit`
**Rationale:** The demo runs from the repo root after `cargo build`. Using the debug binary (not `~/.cargo/bin/conduit`) means the demo always tests the current checkout. The tape scripts export `PATH` so conduit picks up the fake `gh` shim from `demo/fixtures/bin/`.

### VHS settings per clip, shared via `demo/common.tape` include
**Rationale:** VHS 0.11 supports `Source` to include a shared settings file. Font, dimensions, theme, and speed settings belong in one place so all clips look consistent. Per-clip tapes `Source ../common.tape` at the top.

### Committed GIFs vs .gitignore
**Rationale:** Committing GIFs (~5 MB total estimated) means the docs embed works without requiring vhs on the reader's machine, and reviewers can see the visual result in the PR diff preview. A `.gitattributes` entry marks `*.gif` as binary to prevent diff noise. Alternative (generate in CI and upload to S3) deferred.

## Risks / Trade-offs

- **GIF file size** → Mitigate with `SpeedFactor 2` on routine keystrokes, `Sleep` only at meaningful pauses, and `Width 120 Height 30` (compact terminal). Check sizes after generation; re-tune if any clip exceeds 8 MB.
- **TUI render timing** → Conduit's TUI draws asynchronously; VHS `Sleep` commands must be long enough for the TUI to settle. Use 500ms–1s sleeps after actions that trigger async work (workspace creation, git ops). If flaky, bump sleeps.
- **conduit DB state leakage between clips** → Each clip starts from a known DB snapshot written by `seed.sh`. `teardown.sh` deletes the demo data dir (`XDG_DATA_HOME=demo/fixtures/data`). The `CONDUIT_DATA_DIR` env var points all conduit invocations at the fixture dir.
- **`gh` shim brittleness** → The fake `gh` shim only needs to handle `gh pr create` (print fake URL, exit 0) and `gh pr merge` (exit 0). Any other `gh` subcommand falls through to the real `gh` or errors loudly — acceptable since tapes don't invoke other subcommands.
- **VHS version drift** → Pin the VHS version in a comment in `demo/README.md` and `common.tape`. Upgrade is manual.

## Open Questions

- Should `demo/output/*.gif` be tracked by Git LFS rather than committed as plain binary blobs? (Deferred — the repo does not currently use LFS; can migrate later if repo size becomes a concern.)
- Does conduit expose a `--data-dir` flag, or must we rely on `XDG_DATA_HOME`? Needs a quick check during implementation; if neither works, seed.sh may need to write to the default location and teardown must restore it.
