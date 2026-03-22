## Why

The repository is already moving toward a governed `team-workflow` and multi-agent handoff model, but the next iteration still lacks an explicit design-development-review loop, durable compact-recovery artifacts, and a Windows-local validation contract. This needs to be formalized now so future implementation batches do not rely on hidden state or ad hoc testing decisions.

## What Changes

- Add a documented and recoverable review-first iteration contract for `team-workflow` work, including design, development, and review participation in every cycle.
- Define how current-stage state, local Windows environment rules, and OpenSpec artifacts are recorded before implementation starts.
- Continue hardening the `team-workflow` runtime and `multi_agents` collaboration surfaces against coordination drift, recovery gaps, and inconsistent validation.
- Standardize how targeted unit and end-to-end validation is selected and recorded for this workstream on Windows.

## Capabilities

### New Capabilities
- `team-rd-governance`: Defines the required design-development-review cycle, recovery checkpoints, and artifact handoff rules for this repository's active `team-workflow` development.
- `team-workflow-hardening`: Defines the expected behavior and validation contract for `team-workflow` runtime state, vertical/same-level handoffs, and multi-agent integration under the governed iteration loop.

### Modified Capabilities
- None.

## Impact

- Root-level workflow documents used for recovery and local development.
- `openspec/changes/stabilize-team-workflow-rd-loop/` artifacts and any new specs created by this change.
- `codex-rs/core/src/team/*`
- `codex-rs/core/src/tools/handlers/multi_agents*`
- Related unit and end-to-end validation for `codex-core` and `codex-app-server-protocol`

## Recovery Snapshot

- Mode: `parallel`
- Assumptions: current product direction remains `team-workflow` stabilization; Windows local development is the primary environment for this iteration.
- Blockers: next implementation delta still needs to be turned into explicit spec/design/tasks artifacts.
- Next Step: author the new specs, then the design, then the task breakdown before any coding begins.
