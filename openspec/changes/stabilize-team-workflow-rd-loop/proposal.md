## Why

The current branch already carries meaningful `team-workflow` runtime and
multi-agent behavior, but the next iteration still needs a stable contract
that keeps design, development, and review aligned through compaction,
handoff, and Windows-local validation. We need that contract now so the
implementation can continue without drifting away from its recovery and review
requirements.

## What Changes

- Formalize the current `team-workflow` direction as a review-first
  implementation contract before the next code batch starts.
- Define the governance and recovery requirements for the active
  design-development-review loop.
- Define the hardening requirements for handoff, visibility, and validation
  across `team`, `multi_agents`, and workflow protocol surfaces.
- Record the Windows-local development baseline for this change line,
  including virtual-environment usage, direct `cargo` fallbacks, and cleanup
  expectations.

## Capabilities

### New Capabilities

- `team-rd-governance`: Defines the repository-visible design,
  development, and review loop, plus recovery checkpoints and review gates
  for active implementation batches.
- `team-workflow-hardening`: Defines the bounded runtime, handoff,
  visibility, and validation expectations for changed workflow surfaces.

### Modified Capabilities

- None.

## Impact

- Affected code:
  - `codex-rs/core/src/team/*`
  - `codex-rs/core/src/tools/handlers/multi_agents*`
  - `codex-rs/app-server-protocol/src/protocol/*`
- Affected artifacts:
  - `TEAM-ORCHESTRATION.md`
  - `CURRENT-STAGE.md`
  - `IMPLEMENTATION-REVIEW.md`
  - `LOCAL-DEV.md`
  - `openspec/changes/stabilize-team-workflow-rd-loop/*`
- Affected systems:
  - Root-level workflow documentation and compact recovery flow
  - Windows local development environment and validation path
  - Team-workflow public visibility, handoff, and review governance
