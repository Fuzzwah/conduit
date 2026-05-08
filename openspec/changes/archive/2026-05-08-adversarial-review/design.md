## Context

Conduit already has two orchestration sub-agents (`conduit-explore`, `conduit-review`) that the primary Claude agent can invoke via the `Agent` tool. These are written to `~/.claude/agents/` at session start by `ensure_orchestration_agents()` in `conduit-agent/src/orchestration.rs`. The workspace-ready config panel (established in the `workspace-ready-config` spec) already has a pattern for per-workspace toggles (Orchestration row) stored in `workspaces.orchestration_enabled` and `repositories.orchestration_enabled`. The Work Complete dialog already injects prompts back into the primary agent via `WorkCompleteCommand::SendAgentPrompt` (used by `ShowRemainingTasks`).

The adversarial review feature is additive: it follows every established pattern rather than introducing new ones.

## Goals / Non-Goals

**Goals:**
- Add a `conduit-adversarial-review` sub-agent that critiques changes from a security/correctness/quality perspective
- Make the reviewer model user-configurable (stored in workspace/project DB records)
- Expose the feature as opt-in per workspace via the workspace-ready config panel
- Provide two entry points: `/adversarial-review` slash command and Work Complete dialog action
- Route the review report back to the primary agent automatically (via `submit_prompt`)

**Non-Goals:**
- Building a multi-session / forked-tab review flow — sub-agent orchestration is sufficient
- Supporting non-Claude providers — adversarial review relies on the Claude `Agent` tool and orchestration infrastructure
- Storing per-session model overrides — model is set at workspace/project creation time

## Decisions

### D1: Sub-agent, not a forked session

**Decision:** Implement adversarial review as a sub-agent invoked by the primary agent via the existing `Agent` tool.

**Rationale:** A forked session would require cross-session message passing (not currently supported), manual result copying, and a new tab the user must track. The sub-agent approach: (a) uses infrastructure that already exists, (b) feeds results back to the primary agent automatically (the sub-agent's output appears in the primary agent's context), and (c) allows the primary agent to act on findings immediately.

**Alternative considered:** Forked session with a "send to original" action. Rejected because the cross-session communication bus doesn't exist and the payoff (visual separation) doesn't outweigh the complexity.

### D2: Model written into the agent definition file at session start

**Decision:** `ensure_orchestration_agents()` accepts an optional `AdversarialReviewConfig { enabled: bool, model: String }` and writes the `conduit-adversarial-review.md` file with the configured model substituted into the frontmatter. The write is idempotent (only updates if content differs), consistent with how `conduit-explore.md` and `conduit-review.md` are managed.

**Rationale:** Agent definition files are the only mechanism Claude Code uses to specify model for a sub-agent. The model must be baked in. Since conduit workspaces are typically used one at a time per machine, last-writer-wins on the global `~/.claude/agents/` file is acceptable.

**Alternative considered:** Passing the model in the injected prompt text (e.g., "use claude-sonnet-4-6 for this"). Rejected because the `Agent` tool respects the model field in the definition file; prompt-level model overrides are unreliable.

**Default model:** `claude-sonnet-4-6` — materially better at finding subtle bugs than Haiku, and adversarial review is a quality-gate activity where cost matters less than thoroughness.

### D3: Feature defaults to off, opt-in per workspace

**Decision:** `adversarial_review_enabled` defaults to `false` (NULL in DB = off). The workspace-ready config panel exposes the toggle; when enabled, a model input row appears below it. Follows the exact pattern of the Orchestration toggle.

**Rationale:** An adversarial review that runs on every workspace would add noise and cost. Opt-in keeps it purposeful.

### D4: Two new DB columns, same pattern as orchestration

**Decision:** Add `adversarial_review_enabled INTEGER` and `adversarial_review_model TEXT` to both `workspaces` and `repositories` tables via a new migration. Resolution order mirrors orchestration: workspace override → repository default → hard default (off / `claude-sonnet-4-6`).

**Rationale:** This is the established pattern for per-project/per-workspace settings. Reusing it means zero new infrastructure in the data layer.

### D5: Work Complete entry point — condition on settings and changes

**Decision:** `SuggestedAction::AdversarialReview` is added to the suggested actions list by the preflight endpoint when (a) the workspace has `adversarial_review_enabled = true` and (b) there are changes to review (`is_dirty || commits_ahead > 0`). The suggestion does not appear in clean/merged scenarios.

**Rationale:** The Work Complete dialog context already has all necessary information. Conditioning on both settings and change presence avoids offering a no-op action.

### D6: Slash command always available for Claude sessions (no config gate)

**Decision:** `/adversarial-review` is registered as a Claude builtin command (like `/review`) and is always present in the slash menu for Claude sessions, regardless of workspace config.

**Rationale:** Users may want to trigger an ad-hoc adversarial review without having enabled it in the workspace config. The slash command is ephemeral — it just injects a prompt. No agent definition is needed at invocation time if the file was already written (and if orchestration is off, the primary agent will still receive the instructions and attempt to use the sub-agent, which may or may not work gracefully).

**Trade-off:** If adversarial review is off (agent definition not written), the command will inject the prompt but the primary agent won't have the sub-agent available. The command text itself is descriptive enough that the primary agent will likely attempt the review inline. This is acceptable for a slash command (vs. the Work Complete gate which is stricter).

## Risks / Trade-offs

- **Global agent definition file race**: If two workspaces with different `adversarial_review_model` settings are active simultaneously, the agent file will reflect whichever session started last. → Mitigation: document the limitation; in practice, single-active-workspace usage is the norm.
- **Sub-agent model cost**: Sonnet is the default model; a long diff could generate significant token usage. → Mitigation: user can configure Haiku for cheaper reviews; the review is explicitly opt-in.
- **Slash command with orchestration off**: `/adversarial-review` injects a prompt even if the feature is not configured; the primary agent may attempt the review without the sub-agent file present. → Mitigation: acceptable degradation — the primary agent will do a best-effort inline review.

## Migration Plan

1. New DB migration (Migration 23): `ALTER TABLE workspaces ADD COLUMN adversarial_review_enabled INTEGER; ALTER TABLE workspaces ADD COLUMN adversarial_review_model TEXT;` and the same two for `repositories`. Conduit applies migrations at startup — no manual steps required.
2. Existing workspaces get `NULL` for both columns → feature is off by default, no behaviour change.
3. No rollback complexity — the columns are nullable additive additions.

## Open Questions

- None — all design decisions above are resolved.
