## ADDED Requirements

### Requirement: Pi discovers orchestration skills
The system SHALL write Pi-native skill definition files (SKILL.md format) to `~/.pi/agent/skills/conduit-explore/SKILL.md`, `~/.pi/agent/skills/conduit-review/SKILL.md`, and optionally `~/.pi/agent/skills/conduit-adversarial-review/SKILL.md` when orchestration is enabled for a Pi session.

#### Scenario: Explore skill discoverable
- **WHEN** Pi starts with orchestration enabled
- **THEN** `/skill:conduit-explore` is available in Pi's command list

#### Scenario: Review skill discoverable
- **WHEN** Pi starts with orchestration enabled
- **THEN** `/skill:conduit-review` is available in Pi's command list
