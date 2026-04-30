---
name: "Ship It"
description: Commit, create a PR, and merge the current branch to master
category: Workflow
tags: [workflow, git, pr]
---

Commit all staged/unstaged changes on the current branch, open a PR against master, and merge it.

**Steps**

1. **Check working state**
   ```bash
   git status
   git diff
   git log master..HEAD --oneline
   ```
   - If there are no changes and no unpushed commits, tell the user there's nothing to ship and stop.

2. **Stage and commit any uncommitted changes**
   - If there are uncommitted changes, stage and commit them:
     ```bash
     git add <relevant files>
     git commit -m "<message>"
     ```
   - Write a concise commit message describing what changed and why.
   - If there are already commits ahead of master but no uncommitted changes, skip this step.

3. **Push the branch**
   ```bash
   git push -u origin HEAD
   ```

4. **Create a PR**
   - Write a PR body to a temp file and use `--body-file`:
     ```bash
     tmp_body="$(mktemp)"
     cat > "$tmp_body" <<'EOF'
     ## Summary
     - <bullet points>

     ## Testing
     - <what was verified>
     EOF

     gh pr create \
       --repo Fuzzwah/conduit \
       --base master \
       --head "$(git branch --show-current)" \
       --title "<title>" \
       --body-file "$tmp_body"
     rm -f "$tmp_body"
     ```

5. **Merge the PR**
   ```bash
   gh pr merge --repo Fuzzwah/conduit --squash --delete-branch "$(git branch --show-current)"
   ```
   Use `--squash` to keep master history clean.

6. **Report the result** — print the PR URL and confirm it was merged.

**Guardrails**
- Never force-push.
- Never skip hooks (`--no-verify`).
- If any step fails, stop and report the error rather than continuing.
- Use `--repo Fuzzwah/conduit` for all `gh` calls — never `conduit-cli/conduit`.
