# Current Stage

## Stage

- Phase: pre-implementation planning
- Mode: `parallel`
- Branch: `feature/stabilize-team-workflow-rd-loop`
- OpenSpec change: `stabilize-team-workflow-rd-loop`

## Current Assumptions

- The active engineering direction remains the current `team-workflow` hardening line already present on top of the latest branch history.
- This session must stay Windows-local and must not use Docker.
- `.venv-tools` is the active Python virtual environment for local tooling unless superseded by a later committed document.

## Current Blockers

- Some root `just` recipes depend on POSIX shell execution that is not currently reliable through the local WSL path.
- The prefilled OpenSpec `design/specs/tasks` need to be reconciled with the newly committed root-level workflow documents and the active proposal boundary.

## Next Step

1. Reconcile the active OpenSpec artifacts with the committed root-level workflow documents.
2. Freeze the first bounded implementation slice and its validation plan.
3. Only then start implementation work with explicit design, development, and review ownership.

## Recovery Order

1. `TEAM_ORCHESTRATION.md`
2. `CURRENT_STAGE.md`
3. `LOCAL_DEV_WINDOWS.md`
4. `openspec/changes/stabilize-team-workflow-rd-loop/proposal.md`
5. `openspec/changes/stabilize-team-workflow-rd-loop/design.md`
6. `openspec/changes/stabilize-team-workflow-rd-loop/tasks.md`
