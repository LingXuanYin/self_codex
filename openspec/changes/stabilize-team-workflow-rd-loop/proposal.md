## Why

The current branch already extends `team-workflow` across runtime state,
multi-agent handoff, and public protocol surfaces, but the surrounding
development loop is still fragile. We need a documented and enforceable
research and development loop so design, development, and review can keep
iterating on the same workflow direction without losing context across
compaction, handoff, or Windows-local test runs.

## What Changes

- Define the operating contract for the active `team-workflow` direction so
  every substantive cycle includes explicit design, development, and review
  participation.
- Require the active phase, assumptions, blockers, and next step to be
  recoverable from committed repository artifacts before implementation and
  after compaction.
- Stabilize the current `team-workflow` line around sanitized vertical
  artifact handoff, reviewable same-level manifests, and root-safe public
  workflow visibility.
- Record the Windows-local development baseline for this change line,
  including Python virtual environment usage, direct `cargo` fallbacks,
  targeted validation, and cleanup hygiene.

## Capabilities

### New Capabilities

- `team-rd-governance`: Defines the repository-visible
  design-development-review loop, recovery checkpoints, and review gates for
  active implementation batches.
- `team-workflow-governance`: Defines the pre-implementation artifact
  contract and compact recovery order for `team-workflow` iterations.
- `team-workflow-handoffs`: Defines reviewable same-level manifests,
  sanitized vertical artifact handoffs, and root-safe public visibility rules.
- `team-workflow-hardening`: Defines the bounded runtime and collaboration
  hardening expectations and the validation evidence required for changed
  workflow surfaces.

### Modified Capabilities

- None.

## Impact

- Affected code:
  - `codex-rs/core/src/team/*`
  - `codex-rs/core/src/tools/handlers/multi_agents*`
  - `codex-rs/app-server-protocol/src/protocol/*`
- Affected artifacts:
  - `TEAM_ORCHESTRATION.md`
  - `CURRENT_STAGE.md`
  - `LOCAL_DEV_WINDOWS.md`
  - `openspec/changes/stabilize-team-workflow-rd-loop/*`
- Affected systems:
  - repository-visible orchestration and recovery workflow
  - team workflow runtime and recovery state
  - multi-agent handoff and review boundaries
  - Windows local validation workflow for this branch
