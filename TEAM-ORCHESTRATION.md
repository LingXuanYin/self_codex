# Team Orchestration

## Objective

Drive the current `team-workflow` direction through repeatable review-first iterations on Windows local development, with explicit recovery checkpoints before any code implementation.

## Team Topology

- Lead Coordinator
  - Owns the branch, OpenSpec artifacts, root-level workflow documents, and all final writes.
  - Decides iteration boundaries, assigns work, integrates review feedback, and records recovery state.
- Designer
  - Reviews the current implementation direction, updates proposal and design intent, and defines acceptance criteria before coding starts.
  - Must hand off concrete design notes, affected modules, and non-goals to Development and Review.
- Developer
  - Implements only from approved tasks and design decisions.
  - Owns local environment hygiene, targeted tests, and implementation notes for the current batch.
- Reviewer
  - Performs review of implementation, tests, and iteration completeness.
  - Must record findings, residual risks, and release criteria before the next batch starts.

## Mandatory Iteration Contract

Every iteration MUST include all three roles: Design, Development, and Review.

1. Review current state and document the active direction.
2. Designer updates scope, constraints, and acceptance criteria.
3. Developer implements one bounded batch from `tasks.md`.
4. Reviewer checks diff, tests, and recovery notes.
5. Lead updates documents and either closes the batch or plans the next one.

No implementation starts until the current stage, assumptions, blockers, and next step are written into repo documents.

## Communication Contract

- All handoffs MUST be written into repo artifacts, not kept in chat-only state.
- `CURRENT-STAGE.md` is the authoritative recovery log for active mode, assumptions, blockers, and next step.
- `LOCAL-DEV.md` is the authoritative source for Windows environment setup, test commands, and cleanup rules.
- `openspec/changes/<change>/` holds the proposal, specs, design, and task contract for implementation.
- Reviewer findings MUST be written before any new development batch begins.

## Single-Writer Rule

- Only the Lead Coordinator writes final artifacts.
- Delegated agents may analyze, critique, and prepare bounded inputs.
- Design, Development, and Review may run in parallel as analysis, but artifact updates land serially through the Lead.

## Current Staffing Model

- Lead Coordinator: main Codex thread
- Designer: delegated design/research agent
- Developer: delegated implementation agent when coding starts
- Reviewer: delegated review agent for diff and test validation

## Recovery Checklist

- Confirm active branch and OpenSpec change name.
- Read `CURRENT-STAGE.md`.
- Read the active OpenSpec artifacts under `openspec/changes/stabilize-team-workflow-rd-loop/`.
- Resume from the recorded next step instead of relying on prior chat state.
