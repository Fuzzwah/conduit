# Conduit Demo GIFs

Six animated GIF clips demonstrating the full conduit workflow:

| Clip | Description |
|------|-------------|
| `output/01-add-project.gif` | Add a project to conduit |
| `output/02-create-workspace.gif` | Create a new workspace (git worktree) |
| `output/03-make-change.gif` | Make a code change in the shell |
| `output/04-commit.gif` | Stage and commit the change |
| `output/05-pr.gif` | Open a pull request |
| `output/06-merge-archive.gif` | Merge and archive the workspace |

## Prerequisites

- [VHS](https://github.com/charmbracelet/vhs) ≥ 0.11.0
- `git` and `sqlite3` on PATH
- A built conduit binary at `../target/debug/conduit` (run `cargo build` from the repo root)

On Linux, VHS requires `libnss3.so` from Chromium. If VHS fails with a missing library error:

```bash
export LD_LIBRARY_PATH=/home/linuxbrew/.linuxbrew/lib
```

## Regenerating

```bash
# From the repo root:
cargo build

# Generate all six GIFs from scratch:
cd demo
bash teardown.sh   # optional: wipe fixture state for a clean run
bash generate.sh
```

`generate.sh` calls `seed.sh` automatically before recording. Clips run sequentially and share fixture state — each clip builds on the previous one's database state.

To regenerate a single clip after a full run (clips share state, so earlier clips must have run first):

```bash
cd demo
LD_LIBRARY_PATH=/home/linuxbrew/.linuxbrew/lib vhs 06-merge-archive.tape
```

## Embedding

```markdown
![Add project](demo/output/01-add-project.gif)
![Create workspace](demo/output/02-create-workspace.gif)
![Make change](demo/output/03-make-change.gif)
![Commit](demo/output/04-commit.gif)
![Open PR](demo/output/05-pr.gif)
![Merge and archive](demo/output/06-merge-archive.gif)
```

## How clips share state

All clips use `--data-dir demo/fixtures/data`, pointing at the same SQLite database. Each clip starts conduit fresh and relies on the database state left by the previous clip. Clip 01 adds the project; clip 02 creates a workspace; clips 03–05 modify files and create a PR; clip 06 archives the workspace.

`generate.sh` always runs `seed.sh` first, which is idempotent — it only initialises fixtures that don't already exist. To start from a blank slate, run `teardown.sh` before `generate.sh`.
