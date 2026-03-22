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

## Implemented In This Slice

- `codex-rs/core/src/team/runtime.rs` now requires the handoff manifest plus `status.json`, `handoff.json`, `team-tape.jsonl`, `AGENT.md`, and `AGENT_TEAM.md` to remain both declared and present on disk before `atomicWorkflows` delivery can succeed.
- `codex-rs/core/src/team/tests.rs` now covers both the positive persisted-checkpoint path and the negative manifest-deletion regression.
- Root team initialization now regenerates runtime-owned `SKILL.md` files with valid frontmatter-first layout and preserves broken legacy skill content under a marker instead of overwriting it.
- Windows-sensitive `control` and `multi_agents` tests now use safer local cwd handling so the targeted validation set is viable after the rebase.

## Prioritized Findings

### P0: `spawn_agent` can prepare workflow handoff artifacts before spawn success is known

- Source:
  - `codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`
  - `codex-rs/core/src/team/runtime.rs`
  - `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
- Risk:
  - If child spawn fails after workflow-side preparation, the parent can be left with ghost handoff artifacts that imply a child was created when it was not.
- Current decision:
  - Promote this to the next implementation slice, with spawn-only two-phase persistence as the current minimal safe repair.

### P1: `atomicWorkflows` trusted declared checkpoint refs more than persisted files

- Source:
  - `codex-rs/core/src/team/runtime.rs`
  - `codex-rs/core/src/team/tests.rs`
- Risk:
  - A handoff can satisfy the atomic workflow gate with stale `artifact_refs` even if required checkpoint files were deleted after being declared.
  - That weakens the repository's artifact-first recovery contract because finalize and integration-ready transitions can proceed without real persisted checkpoints.
- Current decision:
  - Closed in the current working tree and validated with focused tests.

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
  - `atomicWorkflows` must reject finalize or handoff when the handoff manifest or any of `status.json`, `handoff.json`, `team-tape.jsonl`, `AGENT.md`, or `AGENT_TEAM.md` is missing on disk, even if the path is still present in `prepared.artifact_refs`.
- Initial file boundary:
  - `codex-rs/core/src/team/runtime.rs`
  - `codex-rs/core/src/team/tests.rs`
  - `codex-rs/core/src/team/state.rs` as reference only unless implementation proves otherwise

## Selected Next Slice

- Slice name: `spawn-agent-ghost-handoff-artifacts`
- Reason:
  - `spawn_agent` currently calls `prepare_child_team_spawn` before `spawn_agent_with_metadata` succeeds.
  - `prepare_vertical_handoff` writes the handoff manifest immediately, mirrors artifacts to the operator surface, and can emit an integration patch before the child thread exists.
  - Existing coverage confirms the happy-path manifest contract, but there is not yet a regression test proving failed spawn leaves no ghost handoff artifacts behind.
- Target behavior:
  - Failed `spawn_agent` attempts SHALL not leave a new `spawn-*.md` handoff manifest, integration patch, operator-visible mirror, or delegation bookkeeping behind when the child thread was never created.
- Minimum safe fix boundary:
  - `codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`
  - `codex-rs/core/src/team/runtime.rs`
  - `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
- Non-goals:
  - changing the vertical manifest format or `openspec-artifacts` protocol
  - redesigning the full successful child bootstrap lifecycle
  - broad cleanup of manifest typing or legacy skill migration follow-ups

## Design Decision For The Next Slice

- The safest bounded implementation is a spawn-only two-phase flow:
  - build sanitized child handoff input in memory
  - persist the spawn manifest, optional patch, mirrors, and delegation bookkeeping only after `spawn_agent_with_metadata` succeeds
- Return to design if implementation would require changing the manifest contract itself rather than deferring persistence.

## Proposed Validation For The Next Slice

- Add a failing-spawn regression under `codex-rs/core/src/tools/handlers/multi_agents_tests.rs` with team workflow enabled and a forced spawn failure, then assert no new `spawn-*.md` artifact, no integration patch, and no corresponding operator-visible mirrored artifact remain.
- Extend the regression to assert failed spawn attempts do not add delegation bookkeeping to produced artifacts, audit entries, or delegation tape state.
- Re-run the existing successful manifest handoff test so the child still receives the `openspec-artifacts` manifest on the happy path and artifacts appear only on success.

## Validation Executed

- `cargo test -p codex-core atomic_workflow_`
- `cargo test -p codex-core workflow_loader_accepts_openspec_artifacts_cross_level_handoff_alias`
- `cargo test -p codex-core root_team_initialization_persists_state_and_governance_docs`
- `cargo test -p codex-core root_team_initialization_repairs_legacy_skill_wrapper_layout`
- `cargo test -p codex-core compact_checkpoint_and_resume_enforce_artifact_first_recovery`
- `cargo test -p codex-core public_team_session_exposes_root_only_lifecycle_summary`
- `cargo test -p codex-core multi_agent_v2_spawn_returns_path_and_send_input_accepts_relative_path`
- `cargo test -p codex-core spawn_agent_rejects_when_depth_limit_exceeded`
- `cargo test -p codex-core resume_agent_rejects_when_depth_limit_exceeded`
- `cargo test -p codex-core spawn_thread_subagent_gets_random_nickname_in_session_source`
- `cargo test -p codex-core spawn_agent_can_fork_parent_thread_history`
- `cargo test -p codex-core resume_closed_child_reopens_open_descendants`
- `cargo test -p codex-core resume_agent_from_rollout_reads_archived_rollout_path`
- `cargo clippy --fix --tests --allow-dirty -p codex-core`
- `cargo fmt`
- `tools/argument-comment-lint/run.sh -p codex-core` via Git Bash with `.venv-tools`

## Residual Risks

- The handoff manifest is still inferred from `prepared.artifact_refs.first()` instead of a typed field. That ordering contract is stable enough for this slice but should become explicit in a follow-up change.
- Legacy skill repair preserves broken content under a marker rather than structurally migrating it. That fixes the current loader failure but may leave duplicated legacy guidance for later cleanup.
- `just` is still not a reliable Windows completion gate because the local shell resolution is incomplete; this cycle depended on `cargo` fallbacks and Git Bash for the linter wrapper.
- Full `cargo test -p codex-core` was not re-established as the Windows-wide completion gate for this slice. The accepted evidence remains the targeted test set above.

## Review Outcome

- Design confirmed the slice should stay bounded to the exact six-checkpoint contract and that OpenSpec wording must enumerate the checkpoint set explicitly.
- Development completed the runtime enforcement, focused tests, skill-wrapper repair, and Windows-targeted test stabilization within the bounded scope.
- Review found no remaining blocker in the implemented areas. The remaining items are follow-up risks, not reasons to return this slice to development.

## Role Boundaries For The Next Iteration

### Design

- Translate the selected next slice into bounded two-phase persistence behavior, non-goals, acceptance criteria, and a focused test plan.
- Return to design instead of widening scope if implementation shows the broader spawn lifecycle must be reordered beyond the handoff boundary.
- Keep the next code batch scoped to failed-spawn ghost artifact cleanup instead of reopening broader protocol shape questions.

### Development

- Commit the bounded implementation plus focused validation evidence without reopening broader workflow hardening work.
- Keep the next batch centered on failed-spawn ghost artifact elimination.

### Review

- Validate that each substantive batch shows evidence from Design, Development, and Review.
- Check that failed `spawn_agent` paths no longer leave ghost handoff artifacts behind.
- Reject batches that lack recovery notes, targeted test evidence, or cleanup status.
