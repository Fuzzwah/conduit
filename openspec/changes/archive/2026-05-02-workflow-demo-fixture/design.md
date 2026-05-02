## Context

The current demo fakes the work step with shell commands. The goal is to record a real conduit agent session: conduit creates a workspace, picks the `update-readme` spec, and then Claude actually reads the project files, updates the README, and commits — all visible in the TUI.

The fixture project must be realistic enough that Claude produces non-trivial, visually interesting output in a short time. The spec tasks must be completable in one agent turn (< 60 seconds) so the GIF stays watchable.

## Goals / Non-Goals

**Goals:**
- `demo/project-template/` contains a complete, self-consistent fixture project that `workflow-seed.sh` can copy and commit.
- The `update-readme` OpenSpec change in the template has all four spec-driven artifacts (proposal, design, specs, tasks) so conduit's picker sees a valid change.
- The tasks in the spec are genuinely doable by Claude in one short session: improve the `greet` project's README with a "Recent Updates" section and a project description.
- `workflow.tape` shows conduit running the agent to completion — no manual shell steps.

**Non-Goals:**
- Making `greet` a runnable/compilable Rust project (no `cargo build` in demo).
- Recording a multi-turn agent session (one turn is sufficient and keeps the GIF short).
- Handling agent failures or retries in the tape.

## Decisions

**Template as checked-in files, not heredocs**

All template files live under `demo/project-template/` in the repo. The seed script does `cp -r demo/project-template/ <fixture>` then commits. This makes the template easy to read, edit, and review in PRs.

**Three-commit fixture history**

1. `feat: add greet CLI` — `Cargo.toml` + `src/main.rs`
2. `docs: add README` — `README.md`
3. `feat: add update-readme spec` — `openspec/changes/update-readme/` (all four artifacts)

Commit 3 matches the message in the existing seed script so git log looks natural.

**Agent timing in the tape**

The agent turn is non-deterministic. The tape uses a long `Sleep` (60–90s) after the workspace is created to give Claude time to complete. The GIF is rendered at `PlaybackSpeed 1.0` so this sleep plays back in real time — acceptable since viewers expect to watch an agent work.

Alternative considered: use VHS `Ctrl+c` to cut recording after a fixed time. Rejected — we want to show the full commit appearing, not cut away.

**`update-readme` spec content**

Tasks:
- Add a `## Recent Updates` section to `README.md`
- Add a one-line project description to the top of `README.md`

Both are leaf-level file edits Claude can do in seconds. The spec includes a real `proposal.md`, `design.md`, and `specs/` so conduit's picker sees a fully valid spec-driven change.

## Risks / Trade-offs

- [Agent runtime variance] Claude might take longer than the sleep in some recordings. Mitigation: use a generous sleep (90s); re-record if the agent overruns.
- [API cost] Every recording run calls the real Anthropic API. Mitigation: the task is tiny (< 5 tool calls), cost is negligible.
- [Template staleness] If the `greet` project files need updating, they're easy to find and edit in `demo/project-template/`.
