## ADDED Requirements

### Requirement: Every iteration has explicit design-development-review participation
The repository workflow SHALL require every substantive implementation
iteration to include explicit Design, Development, and Review participation,
with each role's responsibilities and handoff boundaries recorded in committed
artifacts before coding starts.

#### Scenario: Iteration is prepared before implementation
- **WHEN** a new implementation batch is about to begin
- **THEN** the repository SHALL contain a current team orchestration artifact
  naming the Design, Development, and Review roles
- **AND** the repository SHALL record the active branch, active change,
  assumptions, blockers, and next step in committed recovery-oriented
  documents
- **AND** the canonical root-level workflow documents SHALL be discoverable
  from the repository root

### Requirement: Recovery state is recorded in committed artifacts
The active team state SHALL be recoverable from committed repository documents
and OpenSpec artifacts rather than hidden chat context.

#### Scenario: Session resumes after compact or interruption
- **WHEN** work resumes after compact, interruption, or context loss
- **THEN** the lead SHALL be able to recover the active mode, assumptions,
  blockers, and next step by reading the committed governance documents and
  the active OpenSpec change artifacts

### Requirement: Windows local development uses a documented root-level baseline
The active iteration SHALL use a documented Windows local-development baseline
that avoids Docker, uses a root-level virtual environment, and records the
validation and cleanup expectations needed for repeatable execution.

#### Scenario: Local environment is prepared for development
- **WHEN** a development batch is about to start on Windows
- **THEN** the repository SHALL document the root-level virtual environment,
  required environment variables, and preferred command patterns
- **AND** the documented baseline SHALL not depend on Docker

#### Scenario: Local validation is handed to review
- **WHEN** Development hands a batch to Review
- **THEN** the iteration artifacts SHALL record the Windows-local validation
  path and any cleanup actions or environment-specific constraints that remain

### Requirement: Review gates each development batch
The workflow SHALL require review findings, validation outcomes, and
residual-risk notes to be written before the next implementation batch is
started.

#### Scenario: Development batch completes
- **WHEN** a bounded development batch is ready for closure
- **THEN** review findings, validation outcomes, and unresolved risks SHALL be
  recorded before planning or coding the next batch

### Requirement: Vertical handoffs use sanitized artifact-based exchange
The `team-workflow` runtime SHALL represent vertical handoffs between parent
and child teams through sanitized artifact-based manifests that preserve
reviewable integration metadata without leaking hidden child details.

#### Scenario: Child hands work back to parent
- **WHEN** a child team sends a vertical handoff to its parent
- **THEN** the handoff SHALL use an artifact-based manifest suitable for
  operator review
- **AND** the manifest SHALL preserve integration metadata needed for review
  and merge decisions
- **AND** the manifest SHALL avoid exposing hidden child-only identifiers or
  unsafe absolute paths

#### Scenario: Failed child spawn does not leave ghost handoff artifacts
- **WHEN** `spawn_agent` prepares a vertical handoff for a child team but the
  child thread is not actually created
- **THEN** the runtime SHALL not leave behind a new spawn handoff manifest,
  operator-visible mirror, or spawn-only patch artifact that implies a
  successful delegation
- **AND** the parent team SHALL not record a delegation tape entry or
  handoff-produced artifact for that failed spawn attempt

### Requirement: Same-level team communication uses the peer contract
Same-level team communication SHALL use the approved peer messaging contract,
and the system SHALL reject attempts to reuse that contract for vertical
handoffs.

#### Scenario: Sibling team sends peer message
- **WHEN** one same-level team sends a message to another same-level team
- **THEN** the message SHALL be rendered through the approved same-level
  contract

#### Scenario: Vertical route receives same-level contract payload
- **WHEN** a vertical handoff attempts to use the same-level peer messaging
  contract
- **THEN** the system SHALL reject the request with a clear error instead of
  silently accepting the wrong boundary

### Requirement: Validation is recorded for changed team-workflow surfaces
Changes to `team-workflow` runtime and multi-agent collaboration surfaces
SHALL include recorded targeted validation for the affected units and
end-to-end behaviors under the active Windows local-development constraints.

#### Scenario: Runtime or multi-agent behavior changes
- **WHEN** code changes affect `codex-rs/core/src/team/*` or
  `codex-rs/core/src/tools/handlers/multi_agents*`
- **THEN** the iteration artifacts SHALL record the targeted validation
  commands and their outcomes
- **AND** the review phase SHALL identify any environment-limited gaps that
  prevent a broader test gate
