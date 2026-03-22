## Objective

Stabilize the `team-workflow` development loop in `codex-rs` by iterating on the current direction: team-state governance, multi-agent handoff boundaries, public workflow visibility, and Windows-safe local development/testing.

## Current Direction

- Recent work is converging on `team-workflow` runtime hardening, public lifecycle exposure, A2A sibling messaging, and `openspec-artifacts` vertical handoff.
- The next iteration starts from review of the current implementation before any new code is written.

## Team Topology

### Lead

- Role: integration owner and single final writer.
- Responsibilities:
  - Own branch, commits, OpenSpec artifacts, and final integration decisions.
  - Assign bounded work to sub-agents and merge conclusions into canonical documents.
  - Keep `WORKING_STATE.md` current before implementation and after any compact-sensitive decision.

### Design

- Role: requirement and architecture owner.
- Responsibilities:
  - Audit current implementation direction and convert it into proposal, specs, and design.
  - Define module boundaries, non-goals, and acceptance criteria before development starts.
  - Produce an implementation-ready handoff for Development and a review checklist seed for Review.

### Development

- Role: implementation and verification owner.
- Responsibilities:
  - Execute tasks from OpenSpec artifacts.
  - Keep Windows local environment, virtual environment, and test assets usable without Docker.
  - Produce unit-test and end-to-end evidence, plus cleanup notes for transient resources.

### Review

- Role: quality gate owner.
- Responsibilities:
  - Review behavior, regressions, missing tests, and workflow/policy drift.
  - Validate that the iteration includes design, development, and review participation.
  - Block promotion when evidence, docs, or cleanup are incomplete.

## Required Iteration Loop

Every iteration SHALL include all three delivery roles in order:

1. Design
2. Development
3. Review

No implementation starts until Design has published the current scope, assumptions, blockers, and next step into repository documents.

## Handoff Contract

### Design -> Development

- Required inputs:
  - `IMPLEMENTATION_REVIEW.md`
  - `LOCAL_ENV.md`
  - `WORKING_STATE.md`
  - `openspec/changes/stabilize-team-workflow-rd-loop/proposal.md`
  - `openspec/changes/stabilize-team-workflow-rd-loop/design.md`
  - `openspec/changes/stabilize-team-workflow-rd-loop/tasks.md`
- Required contents:
  - Scope
  - explicit non-goals
  - affected modules
  - acceptance criteria
  - test expectations

### Development -> Review

- Required inputs:
  - task completion notes
  - changed files
  - unit-test evidence
  - end-to-end evidence
  - unresolved trade-offs
  - cleanup status

### Review -> Lead

- Required inputs:
  - findings ordered by severity
  - release/block decision
  - missing evidence list
  - follow-up tasks or reopened assumptions

## Information Flow Rules

- Canonical state lives in repository documents, not hidden chat context.
- Any compact-sensitive decision SHALL be copied into `WORKING_STATE.md` before proceeding.
- Sub-agents may analyze, critique, and prepare bounded inputs, but only the Lead writes canonical artifacts.
- Each document added for this workflow gets its own commit before coding begins.

## Naming

- Requirement branch: `feature/stabilize-team-workflow-rd-loop`
- OpenSpec change: `stabilize-team-workflow-rd-loop`
- Root-level workflow documents: uppercase snake-free Markdown names for easy discovery.
