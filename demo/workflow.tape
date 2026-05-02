Output output/workflow.gif

Set Width 1080
Set Height 720
Set FontSize 16
Set Theme "Dracula"
Set PlaybackSpeed 1.0
Set TypingSpeed 150ms
Set Framerate 60
Set Shell "bash"
Set WindowBar "Rings"
Set WindowBarSize 28
Set BorderRadius 8
Set Padding 12

Env USER "demo"
Env GIT_AUTHOR_NAME "Demo User"
Env GIT_AUTHOR_EMAIL "demo@example.com"
Env GIT_COMMITTER_NAME "Demo User"
Env GIT_COMMITTER_EMAIL "demo@example.com"

Hide
Type "cd /tmp/conduit-demo"
Enter
Sleep 500ms
Type "clear"
Enter

# ─── Launch conduit ────────────────────────────────────────────────────────────
Type "./conduit-demo.sh"
Enter
Show
Sleep 2s

# ─── Step 1: Add project ───────────────────────────────────────────────────────
Sleep 1s
Ctrl+n
Sleep 2s

Down
Sleep 1s

Enter
Sleep 2s

# ─── Step 2: Pick a theme ──────────────────────────────────────────────────────
Sleep 1s
Alt+t
Sleep 2s

Type "tokyo"
Sleep 1.5s

Enter
Sleep 3s

# ─── Step 3: Create workspace from OpenSpec ────────────────────────────────────
Sleep 1s
Alt+Tab
Sleep 1s

Alt+n
Sleep 2s

Enter
Sleep 3s

Enter

# ─── Step 4: Let the agent work ────────────────────────────────────────────────
# Claude reads the spec and updates README.md — wait for it to finish
Sleep 20s
Type "do the needful"
Enter
Sleep 30s

# ─── Step 5: Archive the workspace ─────────────────────────────────────────────
Ctrl+p
Sleep 1.5s

Type "archive"
Sleep 1s
Enter
Sleep 5s

Enter
Sleep 5s

Ctrl+q
Sleep 3s
