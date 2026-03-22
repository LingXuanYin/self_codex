## ADDED Requirements

### Requirement: Every iteration has explicit design-development-review participation
The repository workflow SHALL require every implementation iteration to include explicit Design, Development, and Review participation, with each role's responsibilities and handoff boundaries recorded in repository artifacts before coding starts.

#### Scenario: Iteration is prepared before implementation
- **WHEN** a new implementation batch is about to begin
- **THEN** the repository SHALL contain a current orchestration document naming the Design, Development, and Review roles
- **AND** the repository SHALL record the active branch, active change, assumptions, blockers, and next step in a recovery-oriented stage document

### Requirement: Recovery state is recorded in repository artifacts
The active team state SHALL be recoverable from repository documents and OpenSpec artifacts rather than hidden chat context.

#### Scenario: Session resumes after compact or interruption
- **WHEN** work resumes after compact, interruption, or context loss
- **THEN** the lead SHALL be able to recover the active mode, assumptions, blockers, and next step by reading repository documents and the active OpenSpec change artifacts

### Requirement: Review gates each development batch
The workflow SHALL require review findings and residual-risk notes to be written before the next implementation batch is started.

#### Scenario: Development batch completes
- **WHEN** a bounded development batch is ready for closure
- **THEN** review findings, validation outcomes, and unresolved risks SHALL be recorded before planning or coding the next batch
