Output output/06-merge-archive.gif

Source common.tape

Env ANTHROPIC_API_KEY "demo-key-not-real"
Env USER "demo"
Env GIT_AUTHOR_NAME "Demo User"
Env GIT_AUTHOR_EMAIL "demo@example.com"
Env GIT_COMMITTER_NAME "Demo User"
Env GIT_COMMITTER_EMAIL "demo@example.com"

Type "cd /tmp/conduit-demo && clear"
Enter
Sleep 300ms

# Simulate merging via the gh shim (no network needed)
Type "fixtures/bin/gh pr merge --merge"
Enter
Sleep 600ms

# Launch conduit and archive the workspace
Type "./conduit-demo.sh"
Enter
Sleep 4000ms

# Open command palette and select "Archive current workspace"
# (Avoids Alt+X ESC-split ambiguity in VHS's virtual terminal)
Ctrl+p
Sleep 500ms
Type "archive"
Sleep 500ms
Enter
Sleep 2500ms

# Confirm the archive dialog (workspace has unmerged commits — shows confirmation)
Enter
Sleep 2500ms

# The workspace is now gone from the active list, sidebar shows only the project
Sleep 1000ms

Ctrl+q
Sleep 300ms
