Output output/05-pr.gif

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

# Launch conduit — workspace is in the DB from clip 02
Type "./conduit-demo.sh"
Enter
Sleep 2000ms

# The workspace tab should be active (showing the agent chat area)
# Open a PR with Ctrl+Alt+P (OpenPr action)
# conduit will push the branch to the remote and call `gh pr create` (our shim)
Ctrl+Alt+p
Sleep 3000ms

# The fake PR URL should appear in the TUI status area
Sleep 2000ms

Ctrl+q
Sleep 300ms
