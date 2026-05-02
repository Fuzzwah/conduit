## ADDED Requirements

### Requirement: Model selector opens and closes
The system SHALL display a model selection dialog when the user presses Ctrl+O, and SHALL close it when Escape is pressed without changing the active model.

#### Scenario: Model selector appears
- **WHEN** the user presses Ctrl+O
- **THEN** a model selector dialog or list SHALL appear on screen containing at least one model name

#### Scenario: Model selector dismissed with Escape
- **WHEN** the model selector is visible and the user presses Escape
- **THEN** the model selector SHALL close and the main screen SHALL be restored

### Requirement: Theme picker opens and closes
The system SHALL display a theme picker when the user presses Alt+T, listing available themes, and SHALL close it when Escape is pressed.

#### Scenario: Theme picker appears
- **WHEN** the user presses Alt+T
- **THEN** a theme picker dialog SHALL appear containing multiple theme names

#### Scenario: Theme picker dismissed with Escape
- **WHEN** the theme picker is visible and the user presses Escape
- **THEN** the theme picker SHALL close without applying a new theme

### Requirement: Session import picker opens and closes
The system SHALL display a session import picker when the user presses Alt+I, and SHALL close it when Escape is pressed.

#### Scenario: Session import picker appears
- **WHEN** the user presses Alt+I
- **THEN** an import session picker dialog SHALL appear on screen

#### Scenario: Session import picker dismissed with Escape
- **WHEN** the session import picker is visible and the user presses Escape
- **THEN** the dialog SHALL close and the main screen SHALL be restored

### Requirement: Provider selector visible on main screen
The system SHALL display the active provider and model in a selector area on the main screen so users can see the current configuration at a glance.

#### Scenario: Provider selector shown on startup
- **WHEN** the application starts with no active workspace tab
- **THEN** the main screen SHALL show a provider selector or model selector element

#### Scenario: Default model selector shown on startup
- **WHEN** the application starts with no active workspace tab
- **THEN** the main screen SHALL show a default model selector element
