Output output/04-commit.gif

Source common.tape

Env GIT_AUTHOR_NAME "Demo User"
Env GIT_AUTHOR_EMAIL "demo@example.com"
Env GIT_COMMITTER_NAME "Demo User"
Env GIT_COMMITTER_EMAIL "demo@example.com"

Type "cd /tmp/conduit-demo && clear"
Enter
Sleep 300ms

# Navigate into the workspace worktree (names are adjective-noun, no spaces)
Type "WORKTREE=$(ls -d fixtures/data/workspaces/project/*/ | head -1) && cd $WORKTREE"
Enter
Sleep 500ms

# Show the diff
Type "git diff"
Enter
Sleep 1000ms

# Stage and commit
Type "git add README.md"
Enter
Sleep 400ms
Type "git commit -m 'demo: add section to README'"
Enter
Sleep 1000ms

# Show the commit in the log
Type "git log --oneline -5"
Enter
Sleep 1500ms
