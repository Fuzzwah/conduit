Output output/workflow.gif

Set Width 1200
Set Height 600
Set FontSize 13
Set Theme "Dracula"
Set PlaybackSpeed 2.0
Set TypingSpeed 60ms
Set Shell "bash"
Set WindowBar "Rings"
Set WindowBarSize 28
Set BorderRadius 8
Set Padding 12

Env ANTHROPIC_API_KEY "demo-key-not-real"
Env USER "demo"
Env GIT_AUTHOR_NAME "Demo User"
Env GIT_AUTHOR_EMAIL "demo@example.com"
Env GIT_COMMITTER_NAME "Demo User"
Env GIT_COMMITTER_EMAIL "demo@example.com"

Type "cd /tmp/conduit-demo && clear"
Enter
Sleep 300ms

# ─── Launch conduit ────────────────────────────────────────────────────────────
Type "./conduit-demo.sh"
Enter
Sleep 3500ms

# Dismiss any startup overlay (model picker guard)
Escape
Sleep 500ms

# ─── Step 1: Add project ───────────────────────────────────────────────────────
# Ctrl+N opens the project picker; projects_base_dir is pre-set in the DB
Ctrl+n
Sleep 1500ms

# "From git URL…" is the first entry; Down lands on "project/" (the only git repo)
Down
Sleep 3000ms

Enter
Sleep 3000ms

# ─── Step 2: Pick a theme ──────────────────────────────────────────────────────
# Alt+t (lowercase) opens the theme picker — no ESC-split ambiguity
Alt+t
Sleep 800ms

# "Tokyo Night" is in the built-in list; typing "tokyo" filters to it
Type "tokyo"
Sleep 3000ms

Enter
Sleep 1500ms

# ─── Step 3: Create workspace from the OpenSpec spec ───────────────────────────
# After adding the project the sidebar is already visible.
# Double Ctrl+T: first press hides it, second press shows AND focuses it.
# This guarantees sidebar is in SidebarNavigation mode regardless of prior state.
Ctrl+t
Sleep 300ms
Ctrl+t
Sleep 800ms

# Alt+N with a focused sidebar repository row → workspace creation flow
Alt+n
Sleep 2500ms

# Remote sync + issue/spec fetch complete; spec picker shows "update-readme"
Sleep 3000ms

# Select the spec
Enter
Sleep 500ms

# Naming dialog: conduit proposes a branch name — pause so the viewer can read it
Sleep 3000ms

# Accept the auto-generated name
Enter
Sleep 4500ms

# ─── Step 4: Edit README in shell mode ─────────────────────────────────────────
# Workspace tab is active, Normal mode. "!" on empty input enters shell mode.
Type "!"
Sleep 400ms

Type "echo '## Recent Updates' >> README.md"
Enter
Sleep 2000ms

# ─── Step 5: Commit ────────────────────────────────────────────────────────────
Type "git add README.md && git commit -m 'docs: update readme'"
Enter
Sleep 3000ms

# ─── Step 6: Archive the workspace ─────────────────────────────────────────────
# Escape exits shell mode (empty input), then command palette to archive
Escape
Sleep 400ms

Ctrl+p
Sleep 500ms
Type "archive"
Sleep 500ms
Enter
Sleep 3000ms

# Archive preflight dialog — pause for the viewer
Sleep 3000ms

# Confirm
Enter
Sleep 3000ms

# Workspace gone, sidebar shows only the project
Sleep 2000ms

Ctrl+q
Sleep 300ms
