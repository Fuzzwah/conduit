---
description: Commit, create a PR, and merge the current branch to master
---

Commit, create a PR, and merge the current branch to master.

**Steps**

1. **Check working state**
   ```bash
   git status
   git diff
   git log master..HEAD --oneline
   ```
   - If there are no changes and no unpushed commits, tell the user there's nothing to ship and stop.

2. **Check for associated OpenSpec change**

   Derive the workspace name from the current working directory:
   ```bash
   basename "$(pwd)"
   ```

   Check if a matching tasks file exists:
   ```bash
   test -f "openspec/changes/<workspace_name>/tasks.md" && echo found || echo not found
   ```

   **If no tasks file found:** Skip to step 3.

   **If tasks file found:**

   Count incomplete vs complete tasks:
   ```bash
   grep -c '^- \['"'"' \]' "openspec/changes/<workspace_name>/tasks.md" || true
   grep -c '^- \[x\]' "openspec/changes/<workspace_name>/tasks.md" || true
   ```

   **If incomplete tasks exist:**
   - Extract and display each incomplete task line (the `- [ ] ...` lines), grouped by their nearest `##` section heading
   - Announce clearly: "⚠️ X incomplete task(s) remain in the OpenSpec change `<name>`:" followed by the list
   - Continue to step 3 (do not block shipping on incomplete tasks)

   **If all tasks are complete (zero `- [ ]` lines):**
   - Announce: "✓ All tasks complete in OpenSpec change `<name>`."
   - Use **AskUserQuestion tool** to ask: "All OpenSpec tasks are complete. Archive the `<name>` change before shipping?"
   - If user says **yes**: invoke `/skill:openspec-archive-change` for change `<name>`, wait for it to complete, then continue to step 3.
   - If user says **no**: continue to step 3.

3. **Stage and commit any uncommitted changes**
   - If there are uncommitted changes, stage and commit them.
   - Write a concise commit message describing what changed and why.
   - If there are already commits ahead of master but no uncommitted changes, skip this step.

4. **Push the branch**
   ```bash
   git push -u origin HEAD
   ```

5. **Create a PR**
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

6. **Ask for merge approval then merge**
   - Use **AskUserQuestion tool** to ask: "Merge the PR into master?"
   - If approved, merge:
     ```bash
     gh pr merge --repo Fuzzwah/conduit --squash --delete-branch "$(git branch --show-current)"
     ```
   - Use `--squash` to keep master history clean.

7. **Report the result** — print the PR URL and confirm it was merged.

**Guardrails**
- Never force-push.
- Never skip hooks (`--no-verify`).
- If any step fails, stop and report the error rather than continuing.
- Use `--repo Fuzzwah/conduit` for all `gh` calls — never `conduit-cli/conduit`.
- Never merge automatically — always ask for explicit approval before merging (step 6).
- Prefer stopping and asking a focused question over making assumptions about branch targets or merge strategy.
