## ADDED Requirements

### Requirement: Vertical handoffs use sanitized artifact-based exchange
The `team-workflow` runtime SHALL represent vertical handoffs between parent and child teams through sanitized artifact-based manifests that preserve reviewable integration metadata without leaking hidden child details.

#### Scenario: Child hands work back to parent
- **WHEN** a child team sends a vertical handoff to its parent
- **THEN** the handoff SHALL use an artifact-based manifest suitable for operator review
- **AND** the manifest SHALL preserve integration metadata needed for review and merge decisions
- **AND** the manifest SHALL avoid exposing hidden child-only identifiers or unsafe absolute paths

### Requirement: Same-level team communication uses the peer contract
Same-level team communication SHALL use the approved peer messaging contract, and the system SHALL reject attempts to reuse that contract for vertical handoffs.

#### Scenario: Sibling team sends peer message
- **WHEN** one same-level team sends a message to another same-level team
- **THEN** the message SHALL be rendered through the approved same-level contract

#### Scenario: Vertical route receives same-level contract payload
- **WHEN** a vertical handoff attempts to use the same-level peer messaging contract
- **THEN** the system SHALL reject the request with a clear error instead of silently accepting the wrong boundary

### Requirement: Validation is recorded for changed team-workflow surfaces
Changes to `team-workflow` runtime and multi-agent collaboration surfaces SHALL include recorded targeted validation for the affected units and end-to-end behaviors under the active Windows local-development constraints.

#### Scenario: Runtime or multi-agent behavior changes
- **WHEN** code changes affect `codex-rs/core/src/team/*` or `codex-rs/core/src/tools/handlers/multi_agents*`
- **THEN** the iteration artifacts SHALL record the targeted validation commands and their outcomes
- **AND** the review phase SHALL identify any environment-limited gaps that prevent a broader test gate
