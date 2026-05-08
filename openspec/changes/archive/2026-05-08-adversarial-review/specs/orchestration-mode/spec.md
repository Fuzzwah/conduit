## ADDED Requirements

### Requirement: ensure_orchestration_agents accepts optional adversarial review config
The `ensure_orchestration_agents()` function SHALL accept an optional `AdversarialReviewConfig` parameter containing `enabled: bool` and `model: String`. When `enabled` is `true`, the function SHALL write `conduit-adversarial-review.md` to `~/.claude/agents/` using the specified model in the frontmatter. This write follows the same idempotency rule as the existing `conduit-explore.md` and `conduit-review.md` writes.

#### Scenario: Adversarial review agent written when enabled
- **WHEN** `ensure_orchestration_agents(Some(AdversarialReviewConfig { enabled: true, model: "claude-sonnet-4-6".into() }))` is called
- **AND** the file does not exist or has different content
- **THEN** `~/.claude/agents/conduit-adversarial-review.md` is written with `model: claude-sonnet-4-6` in its frontmatter

#### Scenario: Adversarial review agent not written when disabled
- **WHEN** `ensure_orchestration_agents(Some(AdversarialReviewConfig { enabled: false, .. }))` is called
- **THEN** conduit does not write or modify `~/.claude/agents/conduit-adversarial-review.md`

#### Scenario: Adversarial review agent not written when config absent
- **WHEN** `ensure_orchestration_agents(None)` is called
- **THEN** conduit does not write or modify `~/.claude/agents/conduit-adversarial-review.md`

#### Scenario: Adversarial review agent updated when model changes
- **WHEN** `ensure_orchestration_agents` is called with `model: "claude-haiku-4-5"` and the on-disk file specifies `model: claude-sonnet-4-6`
- **THEN** conduit overwrites the file with `model: claude-haiku-4-5`

### Requirement: AgentStartConfig carries adversarial review configuration
The `AgentStartConfig` struct SHALL have an `adversarial_review: Option<AdversarialReviewConfig>` field. The `.with_adversarial_review(cfg)` builder method SHALL set this field. The Claude agent startup path SHALL pass this field to `ensure_orchestration_agents`.

#### Scenario: AgentStartConfig with adversarial review passes config to orchestration
- **WHEN** a Claude session is started with `AgentStartConfig::new().with_adversarial_review(cfg)` where `cfg.enabled = true`
- **THEN** `ensure_orchestration_agents` is called with that config
- **AND** the `conduit-adversarial-review.md` agent file is written accordingly

#### Scenario: AgentStartConfig without adversarial review does not break orchestration
- **WHEN** a Claude session is started without calling `.with_adversarial_review()`
- **THEN** `ensure_orchestration_agents` is called with `None` for the adversarial review config
- **AND** the existing `conduit-explore.md` and `conduit-review.md` files are still written normally (orchestration behaviour unchanged)
