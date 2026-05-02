## 1. Scaffold demo directory structure

- [x] 1.1 Create `demo/` directory with subdirectories: `demo/output/` (GIF destination) and `demo/fixtures/` (seed state)
- [x] 1.2 Add `demo/output/.gitkeep` so the output directory is tracked, and add `demo/output/*.gif` to `.gitattributes` as binary (`*.gif binary`)
- [x] 1.3 Add `demo/fixtures/` to `.gitignore` (seed state is generated, not committed)

## 2. Write seed and teardown scripts

- [x] 2.1 Create `demo/seed.sh`: initialise a bare git repo at `demo/fixtures/remote.git` with one commit on `main` (a simple `README.md`)
- [x] 2.2 In `seed.sh`: clone `demo/fixtures/remote.git` into `demo/fixtures/project/` as the working tree
- [x] 2.3 In `seed.sh`: create `demo/fixtures/bin/gh` as an executable shell shim — `gh pr create` prints `https://github.com/demo/project/pull/1` and exits 0; `gh pr merge` exits 0; all other subcommands delegate to the real `gh` or print "not implemented" and exit 1
- [x] 2.4 In `seed.sh`: create `demo/fixtures/data/` as the conduit data directory (passed via `--data-dir`); verify `conduit --data-dir demo/fixtures/data --version` runs without error
- [x] 2.5 Make `seed.sh` idempotent: skip steps whose output already exists rather than erroring
- [x] 2.6 Create `demo/teardown.sh`: `rm -rf demo/fixtures/` with a confirmation guard (`set -euo pipefail`; no blind `rm -rf` without the path check)
- [x] 2.7 Smoke-test both scripts manually: run `bash demo/seed.sh`, verify fixtures exist, run `bash demo/teardown.sh`, verify fixtures are gone

## 3. Write shared VHS settings

- [x] 3.1 Create `demo/common.tape` with: `Set Width 160`, `Set Height 40`, `Set FontSize 13`, `Set Theme "Dracula"`, `Set PlaybackSpeed 1.5`, `Set TypingSpeed 80ms`, `Set FrameRate 30`
- [x] 3.2 Verify VHS accepts `Source common.tape` from a sibling tape (test with a minimal one-line tape)

## 4. Write clip 01 — Add project

- [x] 4.1 Create `demo/01-add-project.tape`: source `common.tape`, set `Output output/01-add-project.gif`, export env vars (`CONDUIT_DATA_DIR` or `--data-dir` passed inline), launch `conduit` in the fixture data dir context
- [x] 4.2 Script the "add project" interaction: navigate to the Projects screen, trigger add-project, type the path to `demo/fixtures/project/`, confirm, and show the project appearing in the list
- [x] 4.3 Run `vhs demo/01-add-project.tape` and verify `demo/output/01-add-project.gif` is produced and plays correctly; tune `Sleep` values as needed

## 5. Write clip 02 — Create workspace

- [x] 5.1 Create `demo/02-create-workspace.tape`: source `common.tape`, set output path; start from a conduit state where the fixture project is already added (seed.sh handles this by pre-populating the DB, or this tape depends on clip 01's state)
- [x] 5.2 Decide and document in `demo/README.md` whether clips share state sequentially or each clip re-seeds independently; prefer sequential (simpler seed, realistic flow)
- [x] 5.3 Script creating a new workspace: select the project, trigger new-workspace, name it `demo-change`, confirm, and show the workspace tab becoming active
- [x] 5.4 Run `vhs demo/02-create-workspace.tape` and verify output; tune timing

## 6. Write clip 03 — Make a change

- [x] 6.1 Create `demo/03-make-change.tape`: source `common.tape`, set output path
- [x] 6.2 Script opening the workspace file browser (or shell tab), navigating to `README.md`, appending a line (e.g. `echo "## Demo change" >> README.md` via a shell command in the TUI), and showing the change reflected in the file tab
- [x] 6.3 Run `vhs demo/03-make-change.tape` and verify output; tune timing

## 7. Write clip 04 — Commit

- [x] 7.1 Create `demo/04-commit.tape`: source `common.tape`, set output path
- [x] 7.2 Script staging the change and committing through conduit's interface: show the diff, stage all, enter commit message `"demo: add section to README"`, confirm commit, and show the commit appearing in the log
- [x] 7.3 Run `vhs demo/04-commit.tape` and verify output; tune timing

## 8. Write clip 05 — Open PR

- [x] 8.1 Create `demo/05-pr.tape`: source `common.tape`, set output path; ensure `demo/fixtures/bin/` is prepended to `PATH` so the `gh` shim is used
- [x] 8.2 Script pushing the branch and triggering conduit's "Open PR" action; show the fake PR URL printed in the TUI status area
- [x] 8.3 Run `vhs demo/05-pr.tape` and verify output; tune timing

## 9. Write clip 06 — Merge and archive

- [x] 9.1 Create `demo/06-merge-archive.tape`: source `common.tape`, set output path
- [x] 9.2 Script the merge step via the `gh` shim (`gh pr merge` exits 0); then trigger conduit's archive-workspace flow and show the workspace disappearing from the active list
- [x] 9.3 Run `vhs demo/06-merge-archive.tape` and verify output; tune timing

## 10. Write generation convenience script

- [x] 10.1 Create `demo/generate.sh`: run `bash demo/seed.sh`, then for each tape in order run `vhs demo/<tape>.tape`, capture exit code, report pass/fail per clip, exit non-zero if any failed
- [x] 10.2 Smoke-test `bash demo/generate.sh` end-to-end; all six GIFs should appear in `demo/output/`
- [x] 10.3 Check GIF file sizes; if any clip exceeds 8 MB, tune `SpeedFactor`/`Sleep` in the relevant tape and regenerate

## 11. Polish and commit

- [x] 11.1 Review all six GIFs for visual correctness: text readable, no garbled render frames, no excessively long pauses
- [x] 11.2 Write `demo/README.md`: prerequisites (vhs ≥ 0.11.0, git, conduit binary at `./target/debug/conduit`), how to regenerate (`bash demo/generate.sh`), how to embed (`![clip name](demo/output/01-add-project.gif)`), and a note on sequential clip state
- [x] 11.3 Commit `demo/` directory (scripts, tapes, README, generated GIFs, `.gitattributes` update) in one commit: `"feat: add VHS demo GIF workflow"`
