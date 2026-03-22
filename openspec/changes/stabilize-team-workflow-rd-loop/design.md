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

This batch intentionally does not span every boundary above. The first implementation slice is narrower: enforce that `atomicWorkflows` depends on the continued existence of persisted checkpoint files, not just historical references to them.

## Goals / Non-Goals

**Goals:**

- Make the triad workflow observable and recoverable from committed artifacts.
- Strengthen artifact-first recovery by ensuring atomic finalize or handoff gates depend on real persisted checkpoints.
- Preserve Windows-first local development by documenting and reusing the current virtual-environment/tooling setup.
- Tighten unit tests around the selected runtime change boundary before reopening broader public-protocol work.

**Non-Goals:**

- Introduce a new collaboration mode beyond the current single/delegate/parallel model.
- Replace the existing `openspec-artifacts` vertical handoff protocol.
- Rework unrelated product areas outside `team-workflow` runtime checkpoint enforcement.
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

### Decision: Start with atomic checkpoint existence enforcement

- Decision:
  - The first implementation batch will change `codex-rs/core/src/team/runtime.rs` so `atomicWorkflows` only passes when required checkpoint files both appear in `prepared.artifact_refs` and still exist on disk.
- Why:
  - `has_atomic_checkpoint` currently trusts stale declarations and does not verify that required files remain persisted.
  - That is the smallest concrete gap that directly weakens artifact-first recovery.
- Alternatives considered:
  - Fix `spawn_agent` ghost handoff artifacts first.
    - Rejected for this batch because it crosses handler and runtime seams and is less bounded.
  - Expand public workflow session visibility first.
    - Rejected because it broadens the protocol surface before the persistence contract is hardened.

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
- [False confidence from artifact refs] -> Mitigation: add a regression test that deletes a required checkpoint file after preparation and asserts atomic finalize or handoff fails.

## Migration Plan

1. Commit repo-root workflow and recovery docs.
2. Commit OpenSpec artifacts that define the selected first slice and its testable requirements.
3. Implement the runtime checkpoint existence enforcement in `codex-rs/core/src/team/runtime.rs`.
4. Add focused regression coverage in `codex-rs/core/src/team/tests.rs`.
5. Run focused crate tests, then required end-to-end checks.
6. Perform review against committed docs plus test evidence.

Rollback strategy:

- Revert the implementation commits while preserving the documentation commits
  if the docs remain valid for the next attempt.
- If the design itself is invalidated, update the OpenSpec and repo-root docs
  in follow-up commits before retrying.

## Open Questions

- Should the file-existence check stay inline with `has_atomic_checkpoint` or move to a helper if more persisted-artifact rules follow?
- Which additional atomic workflow regressions should become follow-on slices after this file-existence gate lands?
- Which end-to-end scenarios are the minimum credible set for Windows in this environment without overfitting to local shell constraints?

## Acceptance Criteria

- `atomicWorkflows` rejects finalize or handoff when any required checkpoint file is missing on disk, even if `prepared.artifact_refs` still names it.
- Existing success behavior remains intact when all required checkpoint files are both declared and present.
- The implementation does not change app-server protocol types or the same-level versus vertical handoff contract in this batch.
- Focused tests cover the missing-file regression and keep artifact-first recovery behavior explicit.

## File Boundary

- Primary edit target:
  - `codex-rs/core/src/team/runtime.rs`
- Required tests:
  - `codex-rs/core/src/team/tests.rs`
- Reference-only files unless implementation proves otherwise:
  - `codex-rs/core/src/team/state.rs`
  - `codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`

## Test Plan

1. Add or update a focused `codex-rs/core/src/team/tests.rs` case that prepares an atomic workflow handoff, removes one required checkpoint file, and asserts delivery is rejected.
2. Re-run the existing compact or resume artifact-first recovery coverage to ensure the new gate is aligned with the persisted-artifact contract.
3. Run targeted `codex-core` tests on Windows using the documented `.venv-tools` environment, then broaden only if the results justify it.

## Planning State

- Active mode: `parallel`
- Current assumptions:
  - The active branch continues the current `team-workflow` direction.
  - Windows local development without Docker remains the primary execution path.
  - `.venv-tools` remains the baseline root-level Python environment unless implementation proves otherwise.
- Current blockers:
  - The first code batch must stay scoped to atomic checkpoint existence enforcement and avoid reopening wider protocol or spawn-handoff issues.
  - Some repo recipes remain constrained by POSIX-shell expectations on Windows.
  - Broad Windows-wide completion criteria remain less trustworthy than targeted test evidence.
- Next intended step:
  - Update `tasks.md` to the selected first slice, then implement the runtime and test changes.
