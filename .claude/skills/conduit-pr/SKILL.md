---
name: conduit-pr
description: Open a pull request against Fuzzwah/conduit master using the body-file convention from AGENTS.md. Use when the user asks to open, create, or push a PR for the conduit fork.
disable-model-invocation: true
---

# conduit-pr

Open a PR against the **fork** (`Fuzzwah/conduit`), never upstream `conduit-cli/conduit`. Use a body file so quoting/newlines survive.

## Steps

1. Confirm the current branch and that all intended commits are pushed:

   ```bash
   git branch --show-current
   git status
   git log --oneline @{u}..HEAD 2>/dev/null || echo "(no upstream set)"
   ```

   If there is no upstream, push first: `git push -u origin "$(git branch --show-current)"`.

2. Verify the CI gate passes locally before opening the PR:

   ```bash
   cargo fmt --check && cargo clippy -- -D warnings && cargo test
   ```

3. Write the PR body to a temp file (preserves quotes and newlines):

   ```bash
   tmp_body="$(mktemp)"
   cat > "$tmp_body" <<'EOF'
   ## Summary
   - <change 1>
   - <change 2>

   ## Testing
   - cargo fmt --check
   - cargo clippy -- -D warnings
   - cargo test
   EOF
   ```

4. Open the PR against the fork's `master`:

   ```bash
   gh pr create \
     --repo Fuzzwah/conduit \
     --base master \
     --head "$(git branch --show-current)" \
     --title "<short imperative title>" \
     --body-file "$tmp_body"
   rm -f "$tmp_body"
   ```

5. Print the PR URL from the `gh` output so the user can click through.

## Rules

- **Never** target `conduit-cli/conduit`. Always `--repo Fuzzwah/conduit`.
- **Never** pass the body inline with `--body "..."` — quoting eats backticks and newlines.
- Do not create the PR if the CI gate fails locally — fix it first.
- Do not push to remote branches the user did not create.
