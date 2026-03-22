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

### Decision: Follow atomic checkpoint hardening with spawn ghost-artifact hardening

- Decision:
- The next implementation batch will change the `spawn_agent` bootstrap flow so sanitized child handoff content can be prepared in memory, but spawn manifests, integration patches, mirrored operator files, and delegation bookkeeping are persisted only after `spawn_agent_with_metadata` succeeds.
- Why:
  - `spawn.rs` currently calls `prepare_child_team_spawn` before `spawn_agent_with_metadata`.
  - `prepare_child_team_spawn` delegates to `prepare_vertical_handoff`, which writes a manifest to disk and mirrors operator-visible files immediately.
  - If the spawn attempt then fails, the repository can retain a fresh spawn handoff artifact that implies a child was created when no child exists.
- Alternatives considered:
  - Delete newly created spawn artifacts only on failure.
    - Rejected as the primary design because it is harder to bound correctly around shared checkpoint files and mirrored operator paths.
  - Reorder agent creation so the child thread is spawned before any handoff preparation runs.
    - Rejected for this batch because the current agent-control API sends the initial input during spawn, so a full sequencing rewrite crosses a wider handler/runtime boundary.

### Decision: Prefer targeted verification on Windows before broader suites

- Decision:
  - Run focused crate tests and explicit end-to-end scenarios first, then broaden only as far as the environment and repo policy allow.
- Why:
  - The current Windows environment has known `just`/shell limitations and broader suite noise.
- Alternatives considered:
  - Start with full workspace test execution.
    - Rejected because it is slower, noisier, and currently constrained by environment policy.

## Risks / Trade-offs

- [Large runtime hotspot] -> Mitigation: keep the batch scoped to spawn bootstrap and only extract helpers if a clear side-effect-free preview step is needed.
- [Windows shell drift for `just` workflows] -> Mitigation: document exact fallback command patterns and isolate the impact in `LOCAL-DEV.md`.
- [Spec/doc drift from implementation] -> Mitigation: treat proposal/specs/design/tasks as preconditions for coding and update them before any scope change.
- [Ghost spawn artifacts after failed child creation] -> Mitigation: add a focused failure-path test that proves no new spawn manifest, patch, or mirror survives a spawn failure under team workflow mode.
- [Adjacent post-spawn recording failure] -> Mitigation: explicitly defer `record_child_team_spawn` compensation to a follow-up slice unless the bounded two-phase path proves insufficient.

## Migration Plan

1. Commit repo-root workflow and recovery docs.
2. Commit OpenSpec artifacts that define the selected first slice and its testable requirements.
3. Implement spawn-only two-phase handoff persistence across `codex-rs/core/src/tools/handlers/multi_agents/spawn.rs` and `codex-rs/core/src/team/runtime.rs`.
4. Add focused regression coverage in `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`.
5. Run focused crate tests, then required end-to-end checks.
6. Perform review against committed docs plus test evidence.

Rollback strategy:

- Revert the implementation commits while preserving the documentation commits
  if the docs remain valid for the next attempt.
- If the design itself is invalidated, update the OpenSpec and repo-root docs
  in follow-up commits before retrying.

## Open Questions

- Should the side-effect-free spawn preview step stay local to `spawn.rs` or move into a reusable team-runtime helper for other pre-delivery failure paths?
- Does any part of the current spawn-path bookkeeping still need compensation after child creation, or can that stay deferred to a later slice without weakening this batch's contract?
- Which additional handoff lifecycle regressions should become follow-on slices after spawn-failure cleanup lands?
- Which end-to-end scenarios are the minimum credible set for Windows in this environment without overfitting to local shell constraints?

## Acceptance Criteria

- A rejected or failed `spawn_agent` attempt leaves no new `spawn-*.md`, integration patch, or mirrored spawn artifact behind.
- Failed spawn attempts do not add `produced_artifacts`, delegation audit entries, or delegation tape entries.
- Existing success behavior remains intact: successful `spawn_agent` calls still deliver the artifact manifest input to the child and still record the delegation through the normal workflow path.
- The implementation does not reorder the broader spawn lifecycle or change same-level/vertical messaging contracts in this batch.

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

1. Add or update a focused `codex-rs/core/src/tools/handlers/multi_agents_tests.rs` case that enables team workflow, forces `spawn_agent` to fail after preview preparation, and asserts no new spawn manifest, patch, or operator mirror remains.
2. Extend the failure-path coverage to assert no delegation bookkeeping is recorded when child spawn never succeeds.
3. Re-run the existing successful spawn manifest test so the two-phase path does not break valid child handoff delivery.
4. Run targeted `codex-core` tests on Windows using the documented `.venv-tools` environment, then broaden only if the results justify it.

## Planning State

- Active mode: `parallel`
- Current assumptions:
  - The active branch continues the current `team-workflow` direction.
  - Windows local development without Docker remains the primary execution path.
  - `.venv-tools` remains the baseline root-level Python environment unless implementation proves otherwise.
- Current blockers:
- The next code batch must stay scoped to failed-spawn ghost handoff elimination and avoid reopening wider protocol or public-session issues.
  - Some repo recipes remain constrained by POSIX-shell expectations on Windows.
  - Broad Windows-wide completion criteria remain less trustworthy than targeted test evidence.
- Next intended step:
  - Commit the updated iteration boundary, implement spawn-failure cleanup, validate the bounded handler/runtime path, and record the review outcome.
