## ADDED Requirements

### Requirement: README has a project description
`README.md` SHALL include a one-line description of the `greet` project immediately below the `# greet` heading.

#### Scenario: Description is present
- **WHEN** `README.md` is read
- **THEN** the first non-heading line describes what the `greet` CLI does

### Requirement: README has a Recent Updates section
`README.md` SHALL include a `## Recent Updates` section with at least one changelog entry.

#### Scenario: Recent Updates section is present
- **WHEN** `README.md` is read
- **THEN** it contains a `## Recent Updates` heading followed by at least one bullet point describing a recent change
