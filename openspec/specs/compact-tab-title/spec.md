### Requirement: Tab title shows truncated project name
The tab title SHALL display the project name truncated to a maximum of 10 characters. When the project name exceeds 10 characters, it SHALL be truncated and a `…` character appended.

#### Scenario: Short project name displayed in full
- **WHEN** the project name is 10 characters or fewer
- **THEN** the tab title displays the project name unchanged

#### Scenario: Long project name is truncated
- **WHEN** the project name exceeds 10 characters
- **THEN** the tab title displays the first 10 characters followed by `…`

### Requirement: Tab title shows trailing branch segment in brackets
The tab title SHALL display the trailing segment of the workspace's git branch name (the portion after the last `/`) enclosed in square brackets, appended after the project name.

#### Scenario: Branch with username prefix shows only trailing segment
- **WHEN** the workspace branch is `fuz/old-rose`
- **THEN** the tab title displays `[old-rose]`

#### Scenario: Branch without slash shows full branch name
- **WHEN** the workspace branch is `main`
- **THEN** the tab title displays `[main]`

#### Scenario: Combined format
- **WHEN** the project name is `conduit` and the branch is `fuz/old-rose`
- **THEN** the tab title displays `conduit [old-rose]`

#### Scenario: Long project name combined with branch
- **WHEN** the project name is `very-long-project` and the branch is `fuz/feature-x`
- **THEN** the tab title displays `very-long-… [feature-x]`

### Requirement: Branch name preserved through handoff
The branch name label SHALL be preserved when a session is handed off to a different agent type, so the tab title remains correct after handoff.

#### Scenario: Handoff retains branch label
- **WHEN** a session with branch `fuz/my-feature` is handed off to another agent
- **THEN** the new session's tab title continues to show `[my-feature]`
