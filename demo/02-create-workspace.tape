Output output/02-create-workspace.gif

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

# Launch conduit — project is already in the DB from clip 01
Type "./conduit-demo.sh"
Enter
Sleep 3000ms

# Show the sidebar (hidden by default when no workspaces exist)
# Ctrl+T toggles the sidebar into view and focuses it (SidebarNavigation mode)
Ctrl+t
Sleep 500ms

# Cursor lands on the "project" repository row
# Press Alt+N to create a new workspace under it
Alt+n
Sleep 3000ms

# conduit creates a git worktree and a new workspace tab opens
# Let the viewer see the workspace tab become active
Sleep 2000ms

Ctrl+q
Sleep 300ms
