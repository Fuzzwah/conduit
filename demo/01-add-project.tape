Output output/01-add-project.gif

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

# Launch conduit — config and DB are pre-seeded
Type "./conduit-demo.sh"
Enter
Sleep 3000ms

# Dismiss the first-launch model picker
Escape
Sleep 800ms

# Press Ctrl+N to open the project picker (projects_base_dir already set in DB)
Ctrl+n
Sleep 3000ms

# Navigate to "project" (Down to skip "From git URL...") and confirm
Down
Sleep 300ms
Enter
Sleep 3000ms

# Project is now in the sidebar — give the viewer time to see it
Sleep 2000ms

# Exit conduit cleanly (Esc first to close any open overlay, then Ctrl+q)
Escape
Sleep 200ms
Ctrl+q
Sleep 500ms
