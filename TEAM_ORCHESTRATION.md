# Team Orchestration

## Objective

Stabilize the current `team-workflow` development direction and run the work through a repeatable research and development loop on Windows local development without Docker. The loop must require design, development, and review participation before a work cycle is considered complete.

## Active Mode

- Mode: `parallel`
- Single writer: the lead Codex session owns all final artifact edits, branch operations, commits, rebases, and pushes
- Parallel work: sub-agents may inspect code, critique plans, validate assumptions, or prepare bounded recommendations

## Scope Anchor

- Requirement branch: `feature/stabilize-team-workflow-rd-loop`
- OpenSpec change: `stabilize-team-workflow-rd-loop`
- Change directory: `openspec/changes/stabilize-team-workflow-rd-loop/`
- Current direction: continue the existing `team-workflow` implementation line already present in `codex-rs/core/team`, `codex-rs/core/src/tools/handlers/multi_agents*`, and the matching app-server protocol surface

## Team Roster

- Lead / Integrator / Writer: main Codex session
  Responsibility: own final artifacts, task slicing, branch hygiene, integration, test execution, cleanup, and user-facing decisions
- Design role: delegated design analyst
  Responsibility: inspect architecture, identify capability boundaries, challenge assumptions, and hand back design deltas for the lead to write
- Development role: lead Codex session plus bounded implementation workers when needed
  Responsibility: implement approved tasks, keep file ownership explicit, and return verification evidence
- Review role: delegated reviewer
  Responsibility: review diffs, behavioral risks, regression surface, and test gaps before a cycle is accepted

## Communication Contract

- All members recover context from this file and the OpenSpec change artifacts first, not from hidden chat state
- All proposed design or review feedback returns to the lead before any final artifact write
- Development workers receive explicit file ownership and must not overwrite unrelated edits
- Review findings are blocking until the lead records the disposition in the relevant artifact or commit
- Each work cycle must explicitly show design input, development output, and review output

## Iteration Contract

1. Design
   Produce or update proposal and design intent, assumptions, risks, and acceptance boundaries.
2. Development
   Implement only against recorded artifacts and keep validation scoped to the changed surface.
3. Review
   Check behavior, regression risk, test evidence, and recovery completeness before the next cycle or handoff.

## Current Stage Snapshot

- Phase: pre-implementation documentation and workflow setup
- Status: reviewing current implementation and generating the first change artifacts
- Assumptions:
  - The active product direction is the current `team-workflow` line on top of `migrate/team-workflow-rust-v0.115.0`
  - Windows local development is authoritative for this session
  - `.venv-tools` remains the Python virtual environment for local tooling unless replaced by a documented root-level alternative
- Known blockers:
  - Root `justfile` recipes that shell out to POSIX scripts depend on a Unix shell path not currently available through local WSL integration
  - Some full-suite Rust workflows are slower or noisier on Windows and may require direct `cargo` fallbacks instead of `just`
- Next intended step:
  - Write the Windows local development document
  - Generate `proposal.md`, `design.md`, `specs`, and `tasks.md` for `stabilize-team-workflow-rd-loop`
  - Only then begin implementation planning or code changes

## Recovery Protocol

- On compact or interruption, reload:
  - `TEAM_ORCHESTRATION.md`
  - `LOCAL_DEV_WINDOWS.md`
  - `openspec/changes/stabilize-team-workflow-rd-loop/proposal.md`
  - `openspec/changes/stabilize-team-workflow-rd-loop/design.md`
  - `openspec/changes/stabilize-team-workflow-rd-loop/tasks.md`
- Restore from those documents in this order:
  - branch and change name
  - active mode
  - current assumptions
  - blockers
  - next intended step
- Before moving from planning into implementation, the lead must update the current stage snapshot in the relevant documents
