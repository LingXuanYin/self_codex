## Active Mode

- Mode: `parallel`
- Lead: repository-writing integration owner
- Supporting roles: design, development, review

## Branch And Change

- Working branch: `feature/stabilize-team-workflow-rd-loop`
- OpenSpec change: `stabilize-team-workflow-rd-loop`

## Current Phase

- Phase: pre-implementation
- Rule in force: no production code changes before root docs and OpenSpec artifacts are committed

## Current Assumptions

- The current product direction is to stabilize and operationalize `team-workflow`, not to introduce a brand-new collaboration model.
- The next iteration should begin with implementation review, then design, then development, then review.
- Canonical artifacts for recovery must live in committed Markdown and OpenSpec files.
- Windows local development must remain first-class without Docker.

## Confirmed Facts

- Recent commits already addressed:
  - team handoff path hardening
  - A2A sibling messaging boundaries
  - vertical `openspec-artifacts` handoff behavior
  - public workflow lifecycle exports
- `openspec/changes/stabilize-team-workflow-rd-loop/` exists and is scaffolded but artifact files are not written yet.
- Existing `.venv-tools` should be reused instead of creating another environment unless the change proves it necessary.

## Current Blockers

- `just` recipes that require Unix shell integration are not currently reliable on this Windows setup.
- The precise implementation scope still needs to be formalized into proposal/specs/design/tasks.
- Broad workspace test policy must continue to respect repo rules for full-suite execution.

## Next Intended Step

1. Write OpenSpec `proposal.md`
2. Write OpenSpec `specs/.../spec.md`
3. Write OpenSpec `design.md`
4. Write OpenSpec `tasks.md`
5. Start implementation only after those artifacts are committed

## Recovery Checklist After Compact

Recover in this order:

1. `TEAM_CHARTER.md`
2. `IMPLEMENTATION_REVIEW.md`
3. `LOCAL_ENV.md`
4. `WORKING_STATE.md`
5. `openspec/changes/stabilize-team-workflow-rd-loop/proposal.md`
6. `openspec/changes/stabilize-team-workflow-rd-loop/specs/**/spec.md`
7. `openspec/changes/stabilize-team-workflow-rd-loop/design.md`
8. `openspec/changes/stabilize-team-workflow-rd-loop/tasks.md`

## Handoff Expectation For The Next Role

- Design must convert the current review baseline into explicit requirements and acceptance criteria.
- Development must not infer missing requirements from chat alone.
- Review must judge the change against committed docs first, then code and tests.
