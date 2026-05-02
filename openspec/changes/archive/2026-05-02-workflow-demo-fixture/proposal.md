## Why

The workflow demo GIF currently shows manual shell commands (`echo ... >> README.md`, `git commit`) to fake the "work" step. The real selling point of conduit is watching a Claude Code agent do the work autonomously. The demo should record an actual live agent session so viewers see real AI output, real file changes, and real commits — not a scripted terminal echo.

## What Changes

- Add a realistic fixture project (`greet` Rust CLI) under `demo/project-template/` with source, README, and a complete OpenSpec change (`update-readme`) containing all required artifacts (proposal, design, specs, tasks).
- The `update-readme` spec is written so Claude can complete it in a short, visually compelling session: improve the README with a "Recent Updates" section and a project description — simple enough to finish fast, substantive enough to look real.
- `workflow-seed.sh` copies the template into the fixture, commits it with realistic history, and no longer creates the spec inline via heredoc.
- `workflow.tape` is updated to remove the manual `!` shell mode section and the manual `git commit` — after the workspace is created, the recording just shows conduit running the Claude agent and completing the task.

## Capabilities

### New Capabilities

- `demo-project-template`: Versioned fixture project (`greet` Rust CLI) with full OpenSpec `update-readme` change, used as the origin remote for the workflow demo recording.

### Modified Capabilities

- *(none)*

## Impact

- `demo/project-template/`: new directory with all fixture files including the complete OpenSpec change.
- `demo/workflow-seed.sh`: replace inline spec heredoc with `cp -r project-template/` + structured commits.
- `demo/workflow.tape`: remove steps 4 (shell mode edit) and 5 (manual commit); add sleep time for agent to run.
- No conduit source changes.
