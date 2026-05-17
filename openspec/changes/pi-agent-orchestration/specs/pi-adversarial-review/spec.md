## ADDED Requirements

### Requirement: Pi runs adversarial review on demand
The system SHALL provide a conduit-adversarial-review skill and Agent tool subagent type that performs a rigorous adversarial code review when invoked by the main Pi agent. The review sub-agent SHALL use the configured model.

#### Scenario: Adversarial review invoked
- **WHEN** the main Pi agent calls the `Agent` tool with `subagent_type: "conduit-adversarial-review"`
- **THEN** the extension spawns a sub-session with the configured review model and returns the review report

#### Scenario: Adversarial review model stored in session
- **WHEN** the user selects a review model in the workspace config dialog
- **THEN** the model is stored in `session.adversarial_review_model` and passed to the Pi runner
