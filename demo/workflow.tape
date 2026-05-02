Output output/workflow.gif

Set Width 1080
Set Height 720
Set FontSize 18
Set Theme "Dracula"
Set PlaybackSpeed 1.0
Set TypingSpeed 200ms
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
Sleep 300ms

# ─── Launch conduit ────────────────────────────────────────────────────────────
Type "./conduit-demo.sh"
Enter
Show
Sleep 5000ms

# ─── Step 1: Add project ───────────────────────────────────────────────────────
# Ctrl+N opens the project picker; projects_base_dir is pre-set in the DB
Ctrl+n
Sleep 5000ms

# "From git URL…" is the first entry; Down lands on "project/" (the only git repo)
Down
Sleep 5000ms

Enter
Sleep 5000ms

# ─── Step 2: Pick a theme ──────────────────────────────────────────────────────
# Alt+t (lowercase) opens the theme picker — no ESC-split ambiguity
Alt+t
Sleep 5000ms

# "Tokyo Night" is in the built-in list; typing "tokyo" filters to it
Type "tokyo"
Sleep 5000ms

Enter
Sleep 5000ms

# ─── Step 3: Create workspace from the OpenSpec spec ───────────────────────────
# Alt+Tab (NextTab) wraps to the sidebar when no workspace tabs are open
Alt+Tab
Sleep 5000ms

# Alt+N with a focused sidebar repository row → workspace creation flow
Alt+n
Sleep 5000ms

# Remote sync + issue/spec fetch complete; spec picker shows "update-readme"
# Select the spec
Enter
Sleep 5000ms

# Naming dialog: conduit proposes a branch name — pause so the viewer can read it
# Accept the auto-generated name
Enter
Sleep 5000ms

# ─── Step 4: Edit README in shell mode ─────────────────────────────────────────
# Workspace tab is active, Normal mode. "!" on empty input enters shell mode.
Type "!"
Sleep 1000ms

Type "echo '## Recent Updates' >> README.md"
Enter
Sleep 5000ms

# ─── Step 5: Commit ────────────────────────────────────────────────────────────
Type "git add README.md && git commit -m 'docs: update readme'"
Enter
Sleep 5000ms

# ─── Step 6: Archive the workspace ─────────────────────────────────────────────
# Escape exits shell mode (empty input), then command palette to archive
Escape
Sleep 5000ms

Ctrl+p
Sleep 5000ms

Type "archive"
Sleep 1000ms

Enter
Sleep 5000ms

# Archive preflight dialog — confirm
Enter
Sleep 5000ms

# Workspace gone, sidebar shows only the project
Sleep 5000ms

Ctrl+q
Sleep 300ms
