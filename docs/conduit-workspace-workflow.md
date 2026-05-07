# Conduit Workspace Workflow

## Phase 1: Workspace Creation

- **[Optional] Add new project**
  - From URL (clone remote repo into conduit)
  - Select existing local repo
- Create new workspace
- Sync local repo with remote (`git fetch origin`)
- **Decision: Open issues available?**
  - YES → User selects an issue (or skips)
    - If issue selected: agent is primed to gather issue context on activation → proceed to Workspace Created (no spec prompt)
    - If skipped: continue to Incomplete Spec? check
  - NO → continue to Incomplete Spec? check
- **Decision: Incomplete Spec available?**
  - YES → User selects a spec (or skips)
    - If selected: agent is primed to gather spec context on activation
  - NO → continue

→ **Workspace created**

---

## Phase 2: Workspace Configuration

- Select agent provider and model
- Build or Plan Mode?
  - **Build mode:** agent works directly on a solution
  - **Plan mode:** agent creates a plan rather than implementing
- Orchestration On/Off
  - **Off:** single agent handles everything
  - **On:** sub-agent delegation enabled
- **[Optional]** Save selections as project defaults

→ **Workspace becomes active**

---

## Phase 3: Active Session

### 3a. Issue-linked session

- Agent automatically gathers context around the selected issue
- Agent renames the branch and sets workspace title to match issue context
- **Build mode:** agent begins working on a solution immediately
- **Plan mode:** agent begins creating a plan
- User triggers Work Complete when done

### 3b. Spec-linked session

- Agent automatically gathers context around the selected spec
- Agent renames the branch and sets workspace title to match spec context
- User runs the appropriate apply command (e.g. `/opsx:apply`) to begin implementation
- User triggers Work Complete when done

### 3c. Unlinked session

- User interacts with the agent freely on any topic
- When ready to wrap up, one of three outcomes applies:
  - **Exploration / no changes** → Work Complete → no uncommitted changes detected → workspace archived instantly
  - **Trivial change** → Work Complete → agent creates PR → project proceeds through CI/CD / release process
  - **More involved work** → user triggers spec creation (e.g. `/opsx:propose`)
    - **Option A — work spec in current workspace (seamless, no reconfiguration)**
      → Agent implements the spec → user archives the spec → Work Complete → create PR
    - **Option B — commit spec for use in a future workspace**
      → Spec committed to repo via PR → spec available for selection when starting a new workspace later

---

## Phase 4: Work Complete

Triggered by the user (`Alt+Shift+X` in TUI, or "Complete Work" button in web UI).

- Display associated PR (if one exists) with current status
- Display warnings for any uncommitted changes
- Classify workspace state and present appropriate next actions:
  - **No changes detected** → archive workspace immediately
  - **Changes present, no PR** → prompt to create PR
  - **PR exists** → show PR status, guide through CI/CD / release steps
  - **Spec was worked through** → archive spec first, then follow PR path

→ **Workspace archived**
