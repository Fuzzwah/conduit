# Conduit Workspace Workflow

## Phase 1: Workspace Creation

- **[Optional] Add new project**
  - From URL (clone remote repo into conduit)
  - Select existing local repo
- Create new workspace
- Sync local repo with remote (`git fetch origin`)
- **Decision: Open issues available?**
  - NO / SKIPPED → continue to Incomplete Spec(s)? check
  - YES → **Decision: Issue selected?**
    - YES → agent is primed to gather issue context on activation → proceed to Workspace Created (no spec prompt)
    - NO → continue to Incomplete Spec(s)? check
- **Decision: Incomplete Spec(s) available?**
  - NO / SKIPPED → Workspace Created
  - YES → **Decision: Spec selected?**
    - YES → agent is primed to gather spec context on activation → Workspace Created
    - NO → Workspace Created

→ **Workspace Created ✓**

---

## Phase 2: Workspace Configuration

- Select Provider & Model
- Build or Plan Mode?
  - **Build mode:** agent works directly on a solution
  - **Plan mode:** agent creates a plan rather than implementing
- Orchestration On / Off
  - **Off:** single agent handles everything
  - **On:** sub-agent delegation enabled
- Save as Project Defaults?

→ **Workspace Active**

---

## Phase 3: Active Session

### 3a. Issue-linked session

- Agent automatically gathers context around the selected issue
- Agent renames the branch and sets workspace title to match issue context
- Agent works to resolve the issue
- User triggers Work Complete when done

### 3b. Spec-linked session

- Agent automatically gathers context around the selected spec
- Agent renames the branch and sets workspace title to match spec context
- User runs `/opsx:apply` to implement tasks
- User runs `/opsx:archive` when implementation is complete
- User triggers Work Complete

### 3c. Unlinked session

- User interacts with the agent freely on any topic
- When ready to wrap up, one of three outcomes applies:
  - **Exploration / no changes** → Trigger Work Complete
  - **Trivial change made** → Trigger Work Complete
  - **More involved: spec creation** → Trigger Work Complete

---

→ **Trigger Work Complete (Alt+Shift+X)**

---

## Phase 4: Work Complete

- Display PR status (if any) + warn about uncommitted changes
- Classify workspace state:
  - **No changes** → archive immediately
  - **Changes, no PR** → prompt to create PR
  - **PR exists** → CI/CD / release steps
  - **Spec linked** → archive spec, then PR path

→ **Workspace Archived ✓**