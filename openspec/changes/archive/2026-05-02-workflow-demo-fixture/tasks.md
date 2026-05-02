## 1. Template Files — Project Source

- [x] 1.1 Create `demo/project-template/Cargo.toml` with `name = "greet"`, `version = "0.1.0"`, `edition = "2021"`
- [x] 1.2 Create `demo/project-template/src/main.rs` with a `main()` that prints a greeting
- [x] 1.3 Create `demo/project-template/README.md` with a minimal description (thin enough for Claude to meaningfully improve)

## 2. Template Files — OpenSpec Change

- [x] 2.1 Create `demo/project-template/openspec/changes/update-readme/proposal.md` — why the README needs improving
- [x] 2.2 Create `demo/project-template/openspec/changes/update-readme/design.md` — brief technical notes on how to improve it
- [x] 2.3 Create `demo/project-template/openspec/changes/update-readme/specs/update-readme/spec.md` — requirements: add "## Recent Updates" section and add a project description
- [x] 2.4 Create `demo/project-template/openspec/changes/update-readme/tasks.md` — two unchecked tasks matching the spec

## 3. Seed Script

- [x] 3.1 Update `workflow-seed.sh` to `cp -r demo/project-template/ <fixture-clone>` instead of creating files inline
- [x] 3.2 Commit in three steps: "feat: add greet CLI" (Cargo.toml + src/), "docs: add README", "feat: add update-readme spec" (openspec/)
- [x] 3.3 Remove the inline OpenSpec spec heredoc block from the bottom of `workflow-seed.sh`

## 4. Tape

- [x] 4.1 Remove the `Type "!"` shell mode section from `workflow.tape`
- [x] 4.2 Remove the manual `Type "git add ..."` and `git commit` lines from `workflow.tape`
- [x] 4.3 Add a long `Sleep` (90000ms) after workspace creation to allow the Claude agent to complete the task

## 5. Verification

- [x] 5.1 Run `bash demo/workflow-seed.sh` and confirm three commits with correct messages
- [x] 5.2 Confirm all template files including all four OpenSpec artifacts exist in the fixture
- [x] 5.3 Run `LD_LIBRARY_PATH=/home/linuxbrew/.linuxbrew/lib vhs demo/workflow.tape` and confirm the agent completes the task and commits within the sleep window
