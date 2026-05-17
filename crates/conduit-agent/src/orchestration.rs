use std::fs;
use std::io;
use std::path::PathBuf;

/// Configuration for the adversarial review sub-agent.
#[derive(Debug, Clone)]
pub struct AdversarialReviewConfig {
    pub enabled: bool,
    pub model: String,
}

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

const ADVERSARIAL_REVIEW_AGENT_DEF_TEMPLATE: &str = "\
---
name: conduit-adversarial-review
description: Adversarial code reviewer that challenges changes from a critical perspective. Use to get a rigorous second opinion on correctness, security, and quality.
model: {MODEL}
---

You are an adversarial code reviewer. Your goal is to find problems — approach the diff with skepticism.

Review for:
- Correctness: logic errors, wrong assumptions, unhandled edge cases
- Security: injection, auth bypass, data exposure, insecure defaults
- Concurrency: race conditions, deadlocks, unsafe shared state (especially in async Rust)
- Error handling: unchecked errors, incorrect fallbacks, panic paths
- Performance: unnecessary allocations, blocking in async context, O(n²) patterns
- API design: breaking changes, missing validation, inconsistent behaviour
- Test coverage: missing edge case tests, incorrect assertions

Be specific: cite file and line, explain why the issue matters, and suggest a fix.

Return a structured report with severity ratings: CRITICAL / HIGH / MEDIUM / LOW.
Do not repeat large code blocks — quote only the relevant line(s).
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

// ============================================================================
// Pi skill template definitions
// ============================================================================

const PI_EXPLORE_SKILL_TEMPLATE: &str = "\
---
name: conduit-explore
description: Fast codebase exploration agent using a cheaper model. Use when you need to read multiple files, search for patterns, or summarize codebase context for a specific task. Returns concise summaries instead of raw file contents.
model: {MODEL}
---

You are a fast, efficient codebase exploration agent. Your purpose is to read files, search for patterns, and return concise summaries — never dump raw file contents.

Rules:
- Complete your task in 3-8 tool calls
- Summarize only what is relevant to the caller's question
- When multiple independent searches could run in parallel, use parallel tool calls in a single turn
- Do not edit files
- Return results immediately without narration
";

const PI_REVIEW_SKILL_TEMPLATE: &str = "\
---
name: conduit-review
description: Fast diff/change reviewer using a cheaper model. Use when you need a quick review of changes, want to identify issues in a diff, or need a summary of what changed and potential risks.
model: {MODEL}
---

You are a focused code reviewer. Analyze diffs and code changes, then return a brief structured report.

Cover these areas:
- Correctness issues
- Potential bugs or regressions
- Security concerns
- Performance implications

Do not repeat large blocks of code in your report.
";

const PI_ADVERSARIAL_REVIEW_SKILL_TEMPLATE: &str = "\
---
name: conduit-adversarial-review
description: Adversarial code reviewer that challenges changes from a critical perspective. Use to get a rigorous second opinion on correctness, security, and quality.
model: {MODEL}
---

You are an adversarial code reviewer. Your goal is to find problems — approach the diff with skepticism.

Review for:
- Correctness: logic errors, wrong assumptions, unhandled edge cases
- Security: injection, auth bypass, data exposure, insecure defaults
- Concurrency: race conditions, deadlocks, unsafe shared state (especially in async Rust)
- Error handling: unchecked errors, incorrect fallbacks, panic paths
- Performance: unnecessary allocations, blocking in async context, O(n²) patterns
- API design: breaking changes, missing validation, inconsistent behaviour
- Test coverage: missing edge case tests, incorrect assertions

Be specific: cite file and line, explain why the issue matters, and suggest a fix.

Return a structured report with severity ratings: CRITICAL / HIGH / MEDIUM / LOW.
Do not repeat large code blocks — quote only the relevant line(s).
";

fn pi_skills_base_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".pi").join("agent").join("skills"))
}

fn default_pi_fast_model() -> &'static str {
    "gemini-2.5-flash"
}

// ============================================================================
// Claude agent definitions
// ============================================================================

fn claude_agents_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("agents"))
}

pub fn ensure_orchestration_agents(
    adversarial_review: Option<AdversarialReviewConfig>,
) -> io::Result<()> {
    let dir = claude_agents_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "could not determine home directory",
        )
    })?;
    fs::create_dir_all(&dir)?;

    let fixed_files = [
        ("conduit-explore.md", EXPLORE_AGENT_DEF),
        ("conduit-review.md", REVIEW_AGENT_DEF),
    ];

    for (name, content) in fixed_files {
        let path = dir.join(name);
        let needs_write = match fs::read_to_string(&path) {
            Ok(existing) => existing != content,
            Err(_) => true,
        };
        if needs_write {
            fs::write(&path, content)?;
        }
    }

    if let Some(cfg) = adversarial_review {
        if cfg.enabled {
            let content = ADVERSARIAL_REVIEW_AGENT_DEF_TEMPLATE.replace("{MODEL}", &cfg.model);
            let path = dir.join("conduit-adversarial-review.md");
            let needs_write = match fs::read_to_string(&path) {
                Ok(existing) => existing != content,
                Err(_) => true,
            };
            if needs_write {
                fs::write(&path, content)?;
            }
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

// ============================================================================
// Pi orchestration skills
// ============================================================================

/// Write Pi-native orchestration skill files under ~/.pi/agent/skills/.
///
/// These are SKILL.md files following the Agent Skills standard, which Pi
/// auto-discovers via `--skill <dir>` at startup.
pub fn ensure_pi_orchestration_skills(
    adversarial_review: Option<AdversarialReviewConfig>,
) -> io::Result<()> {
    let base = pi_skills_base_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "could not determine home directory",
        )
    })?;

    // Write explore skill
    fs::create_dir_all(base.join("conduit-explore"))?;
    let explore_content = PI_EXPLORE_SKILL_TEMPLATE.replace("{MODEL}", default_pi_fast_model());
    write_skill_if_changed(
        &base.join("conduit-explore").join("SKILL.md"),
        &explore_content,
    )?;

    // Write review skill
    fs::create_dir_all(base.join("conduit-review"))?;
    let review_content = PI_REVIEW_SKILL_TEMPLATE.replace("{MODEL}", default_pi_fast_model());
    write_skill_if_changed(
        &base.join("conduit-review").join("SKILL.md"),
        &review_content,
    )?;

    // Write adversarial review skill with the configured model
    let ar_model = match &adversarial_review {
        Some(cfg) if cfg.enabled => &cfg.model,
        _ => default_pi_fast_model(),
    };
    fs::create_dir_all(base.join("conduit-adversarial-review"))?;
    let ar_content = PI_ADVERSARIAL_REVIEW_SKILL_TEMPLATE.replace("{MODEL}", ar_model);
    write_skill_if_changed(
        &base.join("conduit-adversarial-review").join("SKILL.md"),
        &ar_content,
    )?;

    Ok(())
}

/// Write a file only if its content differs from what's on disk.
fn write_skill_if_changed(path: &std::path::Path, content: &str) -> io::Result<()> {
    let needs_write = match fs::read_to_string(path) {
        Ok(existing) => existing != content,
        Err(_) => true,
    };
    if needs_write {
        fs::write(path, content)?;
    }
    Ok(())
}
