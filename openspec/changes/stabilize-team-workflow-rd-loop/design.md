## Context

The active `team-workflow` work already spans repository documents, `codex-rs/core/src/team/*`, multi-agent tool handlers, and app-server protocol exports. Recent fixes show that the hardest failures are not isolated feature bugs; they are coordination bugs across governance state, public visibility, path handling, and handoff protocol shape.

This iteration must start from a document-first workflow because the user explicitly requires:

- design, development, and review participation in every cycle
- committed recovery state before coding
- Windows local development without Docker
- virtual-environment-backed local tooling
- recovery from compact using repository documents rather than hidden context

The repository already has a natural technical split:

- governance and recovery policy in `team/config.rs`, `team/state.rs`,
  `TEAM-ORCHESTRATION.md`, `CURRENT-STAGE.md`, `LOCAL-DEV.md`, and
  `IMPLEMENTATION-REVIEW.md`
- runtime orchestration in `team/runtime.rs`
- operator/public projection in `team/api.rs`
- tool-surface enforcement in `tools/handlers/multi_agents/*`
- protocol visibility in `app-server-protocol`

This batch intentionally does not span every boundary above. The current implementation slice is narrower: prevent `spawn_agent` from leaving ghost vertical handoff artifacts behind when child-thread creation fails after the workflow manifest has already been prepared.

## Goals / Non-Goals

**Goals:**

- Make the triad workflow observable and recoverable from committed artifacts.
- Strengthen artifact-first recovery by ensuring failed child spawn attempts do not leave behind artifacts that imply a delegation succeeded.
- Preserve Windows-first local development by documenting and reusing the current virtual-environment/tooling setup.
- Tighten unit tests around the selected runtime change boundary before reopening broader public-protocol work.

**Non-Goals:**

- Introduce a new collaboration mode beyond the current single/delegate/parallel model.
- Replace the existing `openspec-artifacts` vertical handoff protocol.
- Rework unrelated product areas outside bounded `team-workflow` spawn/handoff hardening.
- Depend on Docker or a new external orchestration stack for local development.
- Expand the public session or app-server protocol surface in this first batch.
- Refactor broad sections of `team/runtime.rs` beyond the checkpoint gate and its test support.

## Decisions

### Decision: Treat repository documents as first-class workflow state

- Decision:
  - Use committed repo-root governance files plus OpenSpec artifacts as the
    canonical recovery surface before implementation starts.
- Why:
  - The user requires compact recovery from documents.
  - The repo already persists `.codex` team state, but committed governance
    artifacts are better for cross-role review and branch-local iteration.
- Alternatives considered:
  - Rely only on `.codex/team-state` runtime files.
    - Rejected because they are runtime-oriented and weaker for human review handoff.
  - Rely on chat context alone.
    - Rejected because it fails the compact-recovery requirement.

### Decision: Keep one OpenSpec change with one cross-cutting capability

- Decision:
  - Use one change, `stabilize-team-workflow-rd-loop`, with one
    cross-cutting capability, `team-workflow-rd-loop`, that covers
    governance, recovery, handoffs, and Windows-local validation as one
    reviewable iteration contract.
- Why:
  - Governance, recovery, handoffs, and validation are coupled in both the
    user workflow and the implementation seams touched by the current branch.
- Alternatives considered:
  - Split governance and handoffs into separate changes.
    - Rejected because the same iteration must review the entire design/development/review loop together.

### Decision: Preserve single-writer integration with parallel analysis

- Decision:
  - Sub-agents may inspect, critique, and prepare bounded inputs, but only the lead writes canonical docs and final code.
- Why:
  - This matches both the user protocol and the repo's multi-agent safety model.
- Alternatives considered:
  - Let role-aligned sub-agents write separate canonical artifacts in parallel.
    - Rejected because document drift and merge conflicts would undermine recovery fidelity.

### Decision: Stabilize the contract at three boundaries

- Decision:
  - Validate behavior at:
    - repository workflow boundary
    - runtime/handler boundary
    - public protocol boundary
- Why:
  - Recent regressions crossed those exact seams.
- Alternatives considered:
  - Focus only on runtime internals.
    - Rejected because public protocol and tool handlers are part of the observed contract.

### Decision: Follow atomic checkpoint hardening with spawn failure cleanup

- Decision:
  - The next implementation batch will keep the current `spawn_agent` handoff preparation flow, but it will remove any newly created spawn-handoff manifest and mirrored operator artifact if child-thread creation fails before `record_child_team_spawn` can bind the handoff to a real child.
- Why:
  - `spawn.rs` currently calls `prepare_child_team_spawn` before `spawn_agent_with_metadata`.
  - `prepare_child_team_spawn` delegates to `prepare_vertical_handoff`, which writes a manifest to disk and mirrors operator-visible files immediately.
  - If the spawn attempt then fails, the repository can retain a fresh spawn handoff artifact that implies a child was created when no child exists.
- Alternatives considered:
  - Reorder agent creation so the child thread is spawned before any handoff preparation runs.
    - Rejected for this batch because the current agent-control API sends the initial input during spawn, so a full sequencing rewrite crosses a wider handler/runtime boundary.
  - Keep the current behavior and rely on review docs to explain the orphaned manifest.
    - Rejected because stale spawn manifests undermine artifact-first recovery and operator trust.

### Decision: Prefer targeted verification on Windows before broader suites

- Decision:
  - Run focused crate tests and explicit end-to-end scenarios first, then broaden only as far as the environment and repo policy allow.
- Why:
  - The current Windows environment has known `just`/shell limitations and broader suite noise.
- Alternatives considered:
  - Start with full workspace test execution.
    - Rejected because it is slower, noisier, and currently constrained by environment policy.

## Risks / Trade-offs

- [Large runtime hotspot] -> Mitigation: keep the batch scoped to the existing checkpoint gate, and only extract a helper if the existence check becomes materially more complex.
- [Windows shell drift for `just` workflows] -> Mitigation: document exact fallback command patterns and isolate the impact in `LOCAL-DEV.md`.
- [Spec/doc drift from implementation] -> Mitigation: treat proposal/specs/design/tasks as preconditions for coding and update them before any scope change.
- [Ghost spawn artifacts after failed child creation] -> Mitigation: add a focused failure-path test that proves no new spawn manifest survives a spawn failure under team workflow mode.

## Migration Plan

1. Commit repo-root workflow and recovery docs.
2. Commit OpenSpec artifacts that define the selected first slice and its testable requirements.
3. Implement spawn-failure cleanup for prepared child handoff artifacts in `codex-rs/core/src/tools/handlers/multi_agents/spawn.rs` and `codex-rs/core/src/team/runtime.rs`.
4. Add focused regression coverage in `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`.
5. Run focused crate tests, then required end-to-end checks.
6. Perform review against committed docs plus test evidence.

Rollback strategy:

- Revert the implementation commits while preserving the documentation commits
  if the docs remain valid for the next attempt.
- If the design itself is invalidated, update the OpenSpec and repo-root docs
  in follow-up commits before retrying.

## Open Questions

- Should the spawn cleanup stay as a handler-local rollback or move into a reusable team-runtime helper for other pre-delivery failure paths?
- Which additional handoff lifecycle regressions should become follow-on slices after spawn-failure cleanup lands?
- Which end-to-end scenarios are the minimum credible set for Windows in this environment without overfitting to local shell constraints?

## Acceptance Criteria

- When `spawn_agent` runs under team-workflow mode and child creation fails after `prepare_child_team_spawn`, the repository does not retain a fresh spawn manifest or mirrored operator artifact that implies a child team was created.
- Existing success behavior remains intact: successful `spawn_agent` calls still deliver the artifact manifest input to the child and still record the delegation through the normal workflow path.
- The implementation does not reorder the broader spawn lifecycle or change same-level/vertical messaging contracts in this batch.
- Focused tests cover at least one spawn-failure cleanup regression and one successful spawn regression.

## File Boundary

- Primary edit targets:
  - `codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`
  - `codex-rs/core/src/team/runtime.rs`
- Required tests:
  - `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
- Reference-only files unless implementation proves otherwise:
  - `codex-rs/core/src/team/state.rs`
  - `codex-rs/core/src/team/tests.rs`

## Test Plan

1. Add or update a focused `codex-rs/core/src/tools/handlers/multi_agents_tests.rs` case that enables team workflow, forces `spawn_agent` to fail after `prepare_child_team_spawn`, and asserts no new spawn manifest remains under the team artifact directory or operator mirror.
2. Re-run the existing successful spawn manifest test so the cleanup path does not break valid child handoff delivery.
3. Re-run the existing depth-limit or manager-unavailable failure coverage if it becomes the chosen failure trigger for the cleanup regression.
4. Run targeted `codex-core` tests on Windows using the documented `.venv-tools` environment, then broaden only if the results justify it.

## Planning State

- Active mode: `parallel`
- Current assumptions:
  - The active branch continues the current `team-workflow` direction.
  - Windows local development without Docker remains the primary execution path.
  - `.venv-tools` remains the baseline root-level Python environment unless implementation proves otherwise.
- Current blockers:
  - The next code batch must stay scoped to failed-spawn ghost handoff cleanup and avoid reopening wider protocol or public-session issues.
  - Some repo recipes remain constrained by POSIX-shell expectations on Windows.
  - Broad Windows-wide completion criteria remain less trustworthy than targeted test evidence.
- Next intended step:
  - Commit the updated iteration boundary, implement spawn-failure cleanup, validate the bounded handler/runtime path, and record the review outcome.
