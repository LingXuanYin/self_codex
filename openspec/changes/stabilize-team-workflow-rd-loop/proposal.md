## Why

The current branch is already moving toward a role-aware team workflow, but
the development loop is still too dependent on hidden session context and
operator memory. We need a committed, reviewable contract for how design,
development, and review participate in each cycle, how Windows-local
development is bootstrapped, and how work recovers after compact or handoff.

## What Changes

- Add committed governance artifacts that define the team charter, role
  responsibilities, communication rules, recovery order, and Windows-local
  environment constraints for this workflow.
- Define an explicit triad loop in which design, development, and review must
  all participate in each substantive cycle, with clear return-to-design and
  return-to-development paths.
- Establish committed OpenSpec artifacts that describe the current direction,
  implementation approach, test strategy, and iteration tasks before coding
  continues.
- Implement and validate the next `team-workflow` stabilization slices against
  those artifacts, including unit coverage and targeted end-to-end coverage on
  Windows without Docker.

## Capabilities

### New Capabilities

- `team-rd-workflow`: Governs the triad-based research and development loop,
  artifact-backed recovery, Windows-local execution constraints, and validation
  expectations for the current `team-workflow` direction.

### Modified Capabilities

- None.

## Impact

- Governance and recovery artifacts under the repo-root workflow area
  (`.codex/` and root Markdown docs)
- OpenSpec artifacts under
  `openspec/changes/stabilize-team-workflow-rd-loop/`
- Team workflow runtime and state handling in `codex-rs/core/src/team/`
- Multi-agent coordination and handoff behavior in
  `codex-rs/core/src/tools/handlers/multi_agents*`
- Related protocol, public-session, unit, and targeted end-to-end test coverage
