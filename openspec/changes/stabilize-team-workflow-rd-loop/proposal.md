## Why

The current branch already extends `team-workflow` across runtime state,
multi-agent handoff, and public protocol surfaces, but the surrounding
development loop is still fragile. We need a documented and enforceable
research and development loop so design, development, and review can keep
iterating on the same workflow direction without losing context across
compaction, handoff, or Windows-local test runs.

## What Changes

- Define the operating contract for the active `team-workflow` direction so
  every substantive cycle includes design, development, and review
  participation.
- Require the active phase, assumptions, blockers, and next step to be
  recoverable from committed artifacts before implementation and before
  resuming after compaction.
- Stabilize the current `team-workflow` line around artifact-based vertical
  handoff, reviewability, and operator-visible recovery inputs.
- Document the Windows-local development baseline for this change line,
  including Python virtual environment usage, direct `cargo` fallbacks, and
  test resource hygiene.

## Capabilities

### New Capabilities

- `team-workflow-rd-loop`: Defines the required
  design-development-review cycle, compact recovery checkpoints, and
  artifact-backed handoff expectations for the active `team-workflow`
  implementation line.

### Modified Capabilities

- None.

## Impact

- Affected code:
  - `codex-rs/core/src/team/*`
  - `codex-rs/core/src/tools/handlers/multi_agents*`
  - `codex-rs/app-server-protocol/src/protocol/*`
- Affected artifacts:
  - `.codex/AGENT_TEAM.md`
  - `.codex/team-workflow.yaml`
  - `LOCAL_ENV.md`
  - `openspec/changes/stabilize-team-workflow-rd-loop/*`
- Affected systems:
  - team workflow runtime and recovery state
  - multi-agent handoff and review boundaries
  - Windows local validation workflow for this branch
