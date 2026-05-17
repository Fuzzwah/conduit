## Context

Conduit currently supports orchestration for Claude Code only. When orchestration is enabled, Conduit writes `.claude/agents/conduit-*.md` agent definition files and injects instructions telling Claude to use its native `Agent` tool for sub-agent delegation. The `Agent` tool spawns sub-processes running cheaper models (e.g., Haiku) for exploration and review tasks.

Pi Agent runs via `pi --mode rpc` and supports loading TypeScript extensions (`.ts` files) and skills (SKILL.md files following the Agent Skills standard). Pi's RPC mode communicates over stdin/stdout JSONL — commands like `prompt`, `get_state`, `set_model` etc. Pi's SDK provides `createAgentSession()` for programmatic sub-agent creation.

The challenge is that Pi doesn't have a native `Agent` tool like Claude. We need to provide one via a custom extension that uses the SDK to spawn sub-sessions with different models.

## Goals / Non-Goals

**Goals:**
- Enable orchestration toggle in workspace config dialog for Pi Agent sessions
- Write Pi-native skill definitions (`SKILL.md`) for conduit-explore, conduit-review, and conduit-adversarial-review
- Build a Pi extension that registers an `Agent` tool capable of spawning sub-sessions with configurable models
- Pass orchestration config through the Pi runner to the Pi subprocess via CLI flags
- Inject orchestration instructions into Pi's system prompt
- Show orchestration badge and delegation info in the Conduit UI for Pi sessions
- Support adversarial review model selection in the workspace config dialog

**Non-Goals:**
- Full parity with Claude's sub-agent model hot-swapping (Pi sub-agents run independently, not as hot-swapped same-process agents)
- Real-time streaming of sub-agent output to the main agent (sub-agent completes and returns results as text)
- Migration of existing Claude orchestration to use the same extension system

## Decisions

### Decision 1: Pi Extension over Built-in Tool
**Chosen:** Write a `.ts` extension loaded via `--extension`.

**Alternatives considered:**
- **Built-in Rust code in the Pi runner that intercepts tool calls**: The Pi runner communicates via RPC — there's no way to add new tools to Pi's LLM tool registry from Rust without modifying Pi itself. Rejected.
- **SKILL.md + `read` pattern only**: Without a proper tool, the agent would need to manually read skill files and hand-craft sub-agent calls. Fragile and inconsistent. Rejected.
- **Nested Pi RPC sub-process from Conduit**: Conduit could detect "Agent" tool calls in Pi's RPC output and handle them by spawning new Pi processes. This would require complex event interception and state management in Conduit. The extension approach is simpler and keeps the logic with Pi where it belongs.

### Decision 2: Skill files for discoverability
**Chosen:** Write `.pi/skills/conduit-*/SKILL.md` files so Pi auto-discovers them.

Pi loads skills from `.pi/skills/` at startup. These appear as `/skill:conduit-explore` etc. in the command list. The orchestration instructions tell Pi to use these skills via the `Agent` tool. The extension's `Agent` tool implementation will read the skill content from the agent's instructions rather than from files — but the skill files serve as documentation and enable discovery.

### Decision 3: Agent tool protocol
**Chosen:** The extension's `Agent` tool accepts `subagent_type` (skill name) and optional `task` parameters, matching Claude's `Agent` tool signature. This means Conduit's existing tool-call detection code for delegation badges works without modification.

When called, the extension:
1. Reads the skill definition for the requested subagent_type
2. Uses `createAgentSession()` from Pi's SDK to spawn a sub-session
3. Configures the sub-session with the model specified in the skill definition
4. Sends the task prompt to the sub-session
5. Collects all output
6. Returns the result to the calling agent

### Decision 4: Extension hosted in Conduit's data directory
**Chosen:** Write the extension to `~/.conduit/pi-agent-extensions/` at startup, not to the project directory.

This avoids polluting the user's project with hidden files that Conduit manages. The extension is transient infrastructure — it gets cleaned up if Conduit doesn't need it anymore.

### Decision 5: Adversarial review model stored but advisory
Pi's `Agent` tool extension will accept a model name in the skill's metadata. The workspace config dialog stores the review model selection. The extension passes it to the sub-session. Unlike Claude which enforces sub-agent models at the CLI level, Pi's extension uses the SDK to select the model — so the model picker in the dialog directly controls which model the reviewer sub-agent uses.

## Risks / Trade-offs

- **Sub-agent output not streamed**: Claude's Agent tool streams sub-agent output in real-time as tool results. Pi's sub-sessions complete fully before returning. The user sees the final result, not intermediate thinking. Acceptable for exploration/review tasks.
- **Extension reliability**: A crash in the TypeScript extension could crash the Pi process. Pi extensions run in-process. The extension should have robust error handling.
- **SDK dependency**: The extension depends on `@mariozechner/pi-coding-agent` being installed (it ships with Pi). The extension is loaded by Pi, so the SDK is always available.
- **Model compatibility**: Some models used for sub-agents may not be available through Pi's provider configuration. The extension uses Pi's `setModel()` which will fail if the model isn't configured — the extension should surface this error clearly.
