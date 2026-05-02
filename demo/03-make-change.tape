Output output/03-make-change.gif

Source common.tape

Env GIT_AUTHOR_NAME "Demo User"
Env GIT_AUTHOR_EMAIL "demo@example.com"
Env GIT_COMMITTER_NAME "Demo User"
Env GIT_COMMITTER_EMAIL "demo@example.com"

Type "cd /tmp/conduit-demo && clear"
Enter
Sleep 300ms

# Show the worktree that conduit created
Type "echo '==> Worktrees managed by conduit:'"
Enter
Type "ls fixtures/data/workspaces/project/"
Enter
Sleep 800ms

# Navigate into the workspace worktree (names are adjective-noun, no spaces)
Type "WORKTREE=$(ls -d fixtures/data/workspaces/project/*/ | head -1) && cd $WORKTREE"
Enter
Sleep 500ms

# Show current directory and file state
Type "pwd"
Enter
Sleep 400ms
Type "cat README.md"
Enter
Sleep 600ms

# Make a code change
Type "echo '' >> README.md"
Enter
Type "echo '## Demo Change' >> README.md"
Enter
Type "echo '' >> README.md"
Type "echo 'This section was added as part of the demo workflow.' >> README.md"
Enter
Sleep 400ms

# Show the result
Type "cat README.md"
Enter
Sleep 1500ms
