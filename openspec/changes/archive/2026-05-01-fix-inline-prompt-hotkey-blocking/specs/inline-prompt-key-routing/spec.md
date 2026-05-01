## ADDED Requirements

### Requirement: Tab and arrow keys pass through when prompt has no question navigation
While an inline prompt is active, Tab, BackTab, Left (h), and Right (l) keys SHALL only be consumed by the prompt when they perform meaningful question-tab navigation (i.e., the prompt is an AskUserQuestion with multiple questions or a submit tab). In all other cases, these keys SHALL be returned as NotHandled so that global keybindings (tab switching, sidebar focus) remain accessible.

#### Scenario: Tab pressed during ExitPlanMode prompt
- **WHEN** an ExitPlanMode inline prompt is displayed
- **THEN** pressing Tab returns PromptAction::NotHandled, allowing the global tab-switching binding to fire

#### Scenario: BackTab pressed during ExitPlanMode prompt
- **WHEN** an ExitPlanMode inline prompt is displayed
- **THEN** pressing BackTab returns PromptAction::NotHandled, allowing global handlers to process it

#### Scenario: Tab pressed during single-question AskUserQuestion without submit tab
- **WHEN** an AskUserQuestion prompt with exactly one question and no submit tab is displayed
- **THEN** pressing Tab returns PromptAction::NotHandled

#### Scenario: Tab pressed during multi-question AskUserQuestion
- **WHEN** an AskUserQuestion prompt with more than one question is displayed
- **THEN** pressing Tab advances to the next question tab and returns PromptAction::Consumed

#### Scenario: Up/Down still navigate prompt options
- **WHEN** any inline prompt is displayed
- **THEN** pressing Up or Down navigates between prompt options and returns PromptAction::Consumed
