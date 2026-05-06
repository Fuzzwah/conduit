# /shipit

Use this command when the current branch is ready for finalization and you want an OpenSpec-aware path from local changes to PR and merge.

## Workflow

1. Inspect the current worktree before doing anything destructive:
   - review `git status` and the staged/unstaged diff
   - identify the current branch and the repository's default branch

2. Look back through the current conversation to determine whether this session was working on a specific OpenSpec change.
   - signs include a named change, edits under `openspec/changes/<name>/`, calls to `opsx:apply` or `openspec-apply-change`, or task checklist updates
   - if you can identify a change name, inspect only that change's tasks file with `cat openspec/changes/<name>/tasks.md`
   - do **not** scan unrelated OpenSpec changes just because they exist in the worktree

3. If you identified a specific OpenSpec change from the session:
   - count complete (`- [x]`) and incomplete (`- [ ]`) items and report the totals
   - if incomplete items remain, list them grouped by section heading, warn the user that the change still has unfinished tasks, and ask whether to continue anyway
   - if all items are complete, ask the user whether to archive that change before shipping

4. If you cannot identify an OpenSpec change that was worked on in this session, skip the OpenSpec task check entirely and continue with the standard finalization flow.

5. If the user wants to archive a completed OpenSpec change before shipping:
   - follow the repository's normal OpenSpec workflow
   - summarize what was archived
   - if the repository does not provide the needed OpenSpec tooling or instructions, pause and tell the user exactly what is missing instead of guessing

6. Finalization flow after any required confirmation or optional OpenSpec archive work:
   - confirm the commit scope with the user if it is ambiguous
   - commit the current changes
   - push the branch to the remote
   - create a pull request targeting the default branch
   - immediately begin monitoring the PR's check runs:
     - poll with `gh pr checks <PR-URL> --repo Fuzzwah/conduit --watch` and capture its output; this command blocks until all checks complete
     - if `--watch` is unavailable or times out, fall back to polling every 30 seconds with `gh pr checks <PR-URL> --repo Fuzzwah/conduit` until all checks reach a terminal state (pass/fail/cancelled) or 15 minutes have elapsed
     - keep the user informed as checks start, noting that monitoring is in progress
   - once all checks are terminal, report a summary table: check name, status (✓ / ✗ / skipped), and duration
   - call out any failures with the failing job name and a link to its logs via `gh run view <run-id> --log-failed`

7. After reporting PR status, ask the user whether to merge into the default branch.
   - never merge without explicit approval
   - if checks are failing or branch protection blocks merge, explain the blocker clearly

## Guardrails

- Never hide unfinished OpenSpec tasks from the user when you found a session-specific OpenSpec change.
- Never mark tasks complete unless the underlying work is actually done.
- Never merge automatically.
- Prefer stopping and asking a focused question over making assumptions about OpenSpec change names, tooling, branch targets, or merge strategy.
