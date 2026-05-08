## MODIFIED Requirements

### Requirement: Config panel exposes four rows — Provider, Model, Mode, Orchestration
The config panel SHALL display five focusable rows: Provider, Model, Mode, Orchestration, and Adversarial Review. When Adversarial Review is toggled On, a sixth row — Review Model — SHALL appear immediately below it. The panel SHALL be keyboard-driven: Up/Down arrows move focus between rows; Enter or Space activates the focused row; Tab cycles focus forward.

#### Scenario: Arrow key navigation between rows
- **WHEN** the config panel is visible and the user presses Down
- **THEN** focus moves to the next row (wrapping from last to first)

#### Scenario: Up arrow wraps from first to last row
- **WHEN** focus is on the first row and the user presses Up
- **THEN** focus moves to the last interactive row

#### Scenario: Review Model row appears when Adversarial Review is On
- **WHEN** the Adversarial Review row is toggled to On
- **THEN** the Review Model row becomes visible below the Adversarial Review row

#### Scenario: Review Model row hidden when Adversarial Review is Off
- **WHEN** the Adversarial Review row is toggled to Off
- **THEN** the Review Model row is not rendered

## ADDED Requirements

### Requirement: Adversarial Review row toggles On/Off inline, greyed when not applicable
The Adversarial Review row SHALL show two options — Off and On — with the active option highlighted. Pressing Space or Left/Right on the Adversarial Review row SHALL toggle between Off and On. If the selected provider is not Claude, the entire Adversarial Review row SHALL be visually dimmed and non-interactive (adversarial review is a Claude-only feature).

#### Scenario: Adversarial Review toggles on Space for Claude provider
- **WHEN** the selected provider is Claude and the Adversarial Review row is focused
- **AND** the user presses Space
- **THEN** adversarial review toggles between On and Off

#### Scenario: Adversarial Review row dimmed for non-Claude provider
- **WHEN** the selected provider is not Claude
- **THEN** the Adversarial Review row is rendered with muted/dimmed styling and input is ignored

### Requirement: Review Model row accepts a free-text model identifier
The Review Model row SHALL display the currently configured adversarial review model as an editable text field. The default value SHALL be `claude-sonnet-4-6`. The user MAY type any model identifier. The row SHALL only be interactive when Adversarial Review is On.

#### Scenario: Review Model row shows default when not configured
- **WHEN** neither the workspace nor the repository has an `adversarial_review_model` set
- **THEN** the Review Model row shows `claude-sonnet-4-6`

#### Scenario: Review Model row shows configured value
- **WHEN** the repository has `adversarial_review_model = "claude-haiku-4-5"`
- **THEN** the Review Model row shows `claude-haiku-4-5` on panel open

#### Scenario: User can edit the model identifier
- **WHEN** the user focuses the Review Model row and types a new model string
- **THEN** the displayed value updates as the user types

### Requirement: Adversarial review initial value resolved from defaults chain
When the config panel is populated, the initial values for adversarial review enabled and review model SHALL be resolved in this order: (1) workspace-level override (`workspace.adversarial_review_enabled`, `workspace.adversarial_review_model`), (2) project-level override (`repository.adversarial_review_enabled`, `repository.adversarial_review_model`), (3) hard defaults (`false` / `"claude-sonnet-4-6"`).

#### Scenario: Workspace override takes precedence
- **WHEN** the workspace record has `adversarial_review_enabled = 1` and the repository has `adversarial_review_enabled = 0`
- **THEN** the Adversarial Review row shows On when the config panel opens

#### Scenario: Repository default used when workspace has no override
- **WHEN** the workspace has no adversarial review override and the repository has `adversarial_review_enabled = 1`
- **THEN** the Adversarial Review row shows On when the config panel opens

#### Scenario: Hard default applied when neither level is set
- **WHEN** neither workspace nor repository has adversarial review set
- **THEN** the Adversarial Review row shows Off and the Review Model row (when revealed) shows `claude-sonnet-4-6`

### Requirement: "Set as project default" saves adversarial review settings
When the user confirms the config panel with "Set as project default" checked, the system SHALL write `adversarial_review_enabled` and `adversarial_review_model` to the repository record in addition to the existing provider, model, and orchestration fields.

#### Scenario: Adversarial review defaults saved when checkbox checked
- **WHEN** the user enables Adversarial Review, sets model to `claude-haiku-4-5`, checks "Set as project default", and confirms
- **THEN** the repository record has `adversarial_review_enabled = 1` and `adversarial_review_model = "claude-haiku-4-5"`

#### Scenario: Adversarial review settings not saved when checkbox unchecked
- **WHEN** the user proceeds with the Continue button and the checkbox is unchecked
- **THEN** the repository record's `adversarial_review_enabled` and `adversarial_review_model` are unchanged

### Requirement: Workspace-level adversarial review settings persisted on Continue
When the user confirms the config panel (regardless of "Set as project default"), the workspace record SHALL be updated with `adversarial_review_enabled` and `adversarial_review_model` reflecting the user's selections. This allows per-workspace configuration independent of the project default.

#### Scenario: Workspace record updated with adversarial review settings
- **WHEN** the user enables Adversarial Review and confirms the config panel
- **THEN** the workspace record has `adversarial_review_enabled = 1`
- **AND** `adversarial_review_model` reflects the entered model value
