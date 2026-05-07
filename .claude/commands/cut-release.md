# /cut-release

Use this command when you want to tag and publish a new GitHub release from the `master` branch.

## Workflow

1. **Establish current state** — run these in parallel:
   - `gh release list --repo Fuzzwah/conduit --limit 5` to find the last published release and its tag
   - `git show master:Cargo.toml | grep -E "^version"` to get the version on master
   - `git tag -l "v*" | sort -V | tail -5` to see all existing version tags

2. **Determine the release version:**
   - If the user specified a version, use that.
   - Otherwise, use the version in `master`'s `Cargo.toml`.
   - Confirm the chosen version with the user before proceeding.

3. **Check for an orphaned tag** — if a git tag for this version already exists but has no GitHub release (`gh release view <tag> --repo Fuzzwah/conduit` returns "release not found"), note it and skip re-tagging.

4. **Draft release notes** from `git log <last-release-tag>..master --oneline --first-parent`:
   - Group into: **New Features**, **Bug Fixes**, **CI / Infrastructure** (omit empty sections)
   - Use PR merge commit messages as the source of truth; drop noise (docs, chore, openspec archive commits)
   - Present the draft to the user and wait for approval or edits before continuing

5. **Create the git tag** (unless it already exists):
   ```
   git tag <version> master
   git push origin <version>
   ```

6. **Publish the GitHub release** — always use a temp file for the body to avoid quoting issues:
   ```
   tmp="$(mktemp)"
   cat > "$tmp" << 'EOF'
   <approved release notes>
   EOF
   gh release create <version> \
     --repo Fuzzwah/conduit \
     --target master \
     --title "<version>" \
     --notes-file "$tmp"
   rm -f "$tmp"
   ```

7. **Confirm success** — verify `gh release list --repo Fuzzwah/conduit --limit 3` shows the new release, then report the release URL to the user.

## Guardrails

- Never push a tag or create a release without user approval of the version number and release notes.
- Never release from a branch other than `master` unless the user explicitly asks.
- Never skip step 7 — the release is not done until the GitHub release is confirmed published.
- If `Cargo.toml` version and the intended release version don't match, flag it to the user before proceeding.
