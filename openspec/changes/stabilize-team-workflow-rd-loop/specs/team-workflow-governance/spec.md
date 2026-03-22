## ADDED Requirements

### Requirement: Iteration triad artifacts are committed before implementation
The repository workflow SHALL require committed, repository-visible artifacts for design, development, and review coordination before implementation begins on a `team-workflow` iteration.

#### Scenario: Pre-implementation checkpoint exists
- **WHEN** a new `team-workflow` iteration is started
- **THEN** the repository contains committed root-level coordination documents covering team roles, implementation review, local environment constraints, and current working state before code changes begin

#### Scenario: Design handoff is explicit
- **WHEN** development is about to start
- **THEN** proposal, design, specs, and task artifacts exist for the active OpenSpec change and define scope, assumptions, blockers, and next steps

### Requirement: Compact recovery uses committed artifacts
The workflow SHALL recover state after compact, interruption, or lead-agent restart from committed repository artifacts rather than hidden conversational state.

#### Scenario: Recovery source order is defined
- **WHEN** a compact or interruption occurs during an active iteration
- **THEN** the workflow exposes an explicit recovery order for root-level workflow documents and active OpenSpec artifacts

#### Scenario: Recovery state includes execution intent
- **WHEN** the team records the current working state before implementation or after a significant planning update
- **THEN** that state includes the active mode, current assumptions, blockers, and next intended step

### Requirement: Review participation is a release gate
Every `team-workflow` iteration SHALL include review participation with repository-visible findings or an explicit no-findings decision before the iteration is treated as ready for promotion.

#### Scenario: Review handoff is evidence-backed
- **WHEN** development hands work to review
- **THEN** the handoff includes changed files, unit-test evidence, end-to-end evidence, unresolved trade-offs, and cleanup status

#### Scenario: Review result is explicit
- **WHEN** review completes
- **THEN** the workflow records either ordered findings or an explicit pass decision tied to the documented scope
