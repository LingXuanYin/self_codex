# Current Implementation Review

## Scope Reviewed

- Branch: `feature/stabilize-team-workflow-rd-loop`
- Direction reviewed: `team-workflow` runtime governance, multi-agent handoff boundaries, recovery artifacts, and Windows-local development constraints
- Primary modules reviewed:
  - `codex-rs/core/src/team/config.rs`
  - `codex-rs/core/src/team/runtime.rs`
  - `codex-rs/core/src/team/api.rs`
  - `codex-rs/core/src/tools/handlers/multi_agents*.rs`
  - `codex-rs/app-server-protocol/src/protocol/common.rs`
  - `codex-rs/app-server-protocol/src/protocol/v2.rs`

## Current Direction

- Recent implementation work is consolidating a governed `team-workflow` loop with required `design`, `development`, and `review` participation.
- The runtime already models review-first handoff gates, compact/recovery checkpoints, operator-visible artifacts, and `openspec-artifacts` vertical handoff.
- Public protocol surfaces for workflow lifecycle and session visibility now exist, but the operator workflow around them is still only partially documented and not yet translated into a complete implementation contract.

## Strengths In Current Implementation

- `team/config.rs` already encodes the triad loop, compact persistence, resume-from-artifacts behavior, and single-writer/atomic workflow rules.
- `team/runtime.rs` already persists state, generates governance assets, records compact/resume checkpoints, and shapes vertical handoff payloads.
- `multi_agents` handlers already integrate with the workflow for spawn, send, resume, and boundary-aware handoff conversion.
- Focused tests already cover key handoff and review-gate behavior in `core/src/team/tests.rs` and `core/src/tools/handlers/multi_agents_tests.rs`.

## Gaps To Address Next

- The first code batch still needs a narrower implementation target than the original cross-cutting governance and protocol scope.
- Root-level operator documents are present but not yet fully normalized around one canonical naming and recovery path.
- Windows-local validation rules are known in practice but need to be carried as first-class project artifacts for future batches and review handoffs.
- Full completion criteria for local validation still depend on targeted `cargo` checks/tests because some `just` recipes are not reliable in the current Windows shell setup.

## Prioritized Findings

### P0: `spawn_agent` can prepare workflow handoff artifacts before spawn success is known

- Source:
  - `codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`
  - `codex-rs/core/src/team/runtime.rs`
  - `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
- Risk:
  - If child spawn fails after workflow-side preparation, the parent can be left with ghost handoff artifacts that imply a child was created when it was not.
- Current decision:
  - Keep this as a follow-on slice, not the first implementation batch.

### P1: `atomicWorkflows` trusts declared checkpoint refs more than persisted files

- Source:
  - `codex-rs/core/src/team/runtime.rs`
  - `codex-rs/core/src/team/tests.rs`
- Risk:
  - A handoff can satisfy the atomic workflow gate with stale `artifact_refs` even if required checkpoint files were deleted after being declared.
  - That weakens the repository's artifact-first recovery contract because finalize and integration-ready transitions can proceed without real persisted checkpoints.
- Current decision:
  - This is the selected first implementation slice.

### P2: replan detection is still too trigger-word dependent

- Source:
  - `codex-rs/core/src/team/state.rs`
  - `codex-rs/core/src/team/runtime.rs`
- Risk:
  - Review and replanning transitions can miss intent when messages do not use the expected trigger terms.
- Current decision:
  - Defer until the atomic checkpoint gate is enforced and revalidated.

## Selected First Slice

- Slice name: `atomic-checkpoint-existence-enforcement`
- Reason:
  - It is the smallest concrete gap that directly strengthens the documented artifact-first recovery requirement.
  - It stays within the current `team/runtime.rs` and `team/tests.rs` boundary without expanding the public protocol surface.
  - It gives Design, Development, and Review a clean acceptance target before broader spawn/handoff or session-visibility work.
- Target behavior:
  - `atomicWorkflows` must reject finalize or handoff when any required checkpoint path is missing on disk, even if the path is still present in `prepared.artifact_refs`.
- Initial file boundary:
  - `codex-rs/core/src/team/runtime.rs`
  - `codex-rs/core/src/team/tests.rs`
  - `codex-rs/core/src/team/state.rs` as reference only unless implementation proves otherwise

## Pre-Code Document Gate

- `CURRENT-STAGE.md` records the active mode, current assumptions, blockers, and next step for compact recovery.
- `IMPLEMENTATION-REVIEW.md` records the prioritized findings and chosen first slice before coding starts.
- `openspec/changes/stabilize-team-workflow-rd-loop/design.md` and `tasks.md` must be updated to the selected first slice before any Rust edit lands.

## Test Surface Snapshot

- Confirmed viable locally:
  - `cargo fmt`
  - `cargo test -p codex-app-server-protocol`
  - `cargo check -p codex-core`
  - focused `codex-core` exact tests for `team` and `multi_agents`
- Not currently a reliable local completion gate:
  - POSIX-shell-backed `just` recipes on this Windows machine
  - treating the entire `codex-core` suite as a stable Windows-wide gate without targeted triage

## Risks

- The codebase already contains workflow semantics; if docs/specs drift from the implemented runtime, Design and Review will diverge quickly.
- Duplicate or inconsistent root-level workflow document names will weaken compact recovery and delegation quality.
- Environment-specific validation rules can be forgotten during iteration unless they are captured in canonical docs and referenced from tasks/review handoff.

## Role Boundaries For The Next Iteration

### Design

- Translate the selected first slice into bounded behavior, non-goals, acceptance criteria, and a focused test plan.
- Normalize which root-level documents are canonical for recovery and handoff.
- Keep the initial code batch scoped to artifact-first checkpoint enforcement instead of reopening protocol shape questions.

### Development

- Implement only after the proposal/spec/design/tasks contract is updated to the selected first slice.
- Keep code changes bounded to `codex-rs/core/src/team/runtime.rs` and focused tests unless the implementation proves a documented dependency.
- Record exact Windows-local commands, outputs, and cleanup steps for Review.

### Review

- Validate that each substantive batch shows evidence from Design, Development, and Review.
- Check that atomic workflow enforcement now depends on persisted checkpoint existence rather than stale declarations alone.
- Reject batches that lack recovery notes, targeted test evidence, or cleanup status.
