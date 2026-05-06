use std::fs;
use std::io;
use std::path::PathBuf;

const EXPLORE_AGENT_DEF: &str = "\
---
name: conduit-explore
description: Fast codebase exploration agent using a cheaper model. Use when you need to read multiple files, search for patterns, or summarize codebase context for a specific task. Returns concise summaries instead of raw file contents.
model: claude-haiku-4-5
---

You are a fast, efficient codebase exploration agent. Your purpose is to read files, search for patterns, and return concise summaries — never dump raw file contents.

Rules:
- Complete your task in 3-8 tool calls
- Summarize only what is relevant to the caller's question, not everything you found
- When multiple independent searches could run in parallel, use parallel tool calls in a single turn
- Do not edit files
- Return results immediately without narration
";

const REVIEW_AGENT_DEF: &str = "\
---
name: conduit-review
description: Fast diff/change reviewer using a cheaper model. Use when you need a quick review of changes, want to identify issues in a diff, or need a summary of what changed and potential risks.
model: claude-haiku-4-5
---

You are a focused code reviewer. Analyze diffs and code changes, then return a brief structured report.

Cover these areas:
- Correctness issues
- Potential bugs or regressions
- Security concerns
- Performance implications

Do not repeat large blocks of code in your report.
";

fn claude_agents_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("agents"))
}

pub fn ensure_orchestration_agents() -> io::Result<()> {
    let dir = claude_agents_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "could not determine home directory",
        )
    })?;
    fs::create_dir_all(&dir)?;

    let files = [
        ("conduit-explore.md", EXPLORE_AGENT_DEF),
        ("conduit-review.md", REVIEW_AGENT_DEF),
    ];

    for (name, content) in files {
        let path = dir.join(name);
        let needs_write = match fs::read_to_string(&path) {
            Ok(existing) => existing != content,
            Err(_) => true,
        };
        if needs_write {
            fs::write(&path, content)?;
        }
    }

    Ok(())
}

pub fn orchestration_instructions() -> &'static str {
    "\
---
Orchestration mode is active for this session. Before loading large amounts of raw file content into this context, delegate to the sub-agents below via the Agent tool:

- **conduit-explore**: for reading/summarizing files, searching patterns, gathering codebase context
- **conduit-review**: for reviewing diffs or code changes before acting on them

These sub-agents use a cheaper model and return concise summaries, keeping this context window lean.\
"
}
