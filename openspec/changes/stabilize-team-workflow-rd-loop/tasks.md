## 1. Workflow Baseline

- [x] 1.1 Re-read `TEAM-ORCHESTRATION.md`, `CURRENT-STAGE.md`, `IMPLEMENTATION-REVIEW.md`, `LOCAL-DEV.md`, and the active OpenSpec artifacts before implementation starts
- [x] 1.2 Confirm the first implementation target is `atomic-checkpoint-existence-enforcement` and keep the active branch/change aligned to that scope
- [x] 1.3 Record the design, development, and review evidence package in committed repo-root and OpenSpec artifacts before Rust edits begin

## 2. Atomic Checkpoint Enforcement

- [x] 2.1 Audit `codex-rs/core/src/team/runtime.rs`, `codex-rs/core/src/team/state.rs`, and `codex-rs/core/src/team/tests.rs` around `atomicWorkflows` and persisted checkpoint expectations
- [x] 2.2 Update the atomic workflow delivery gate so the handoff manifest plus `status.json`, `handoff.json`, `team-tape.jsonl`, `AGENT.md`, and `AGENT_TEAM.md` must both be declared in `prepared.artifact_refs` and exist on disk before finalize or handoff succeeds
- [x] 2.3 Add or update focused tests in `codex-rs/core/src/team/tests.rs` that delete the handoff manifest after message preparation and assert `atomicWorkflows` blocks delivery, while preserving a positive persisted-checkpoint delivery case

## 3. Validation And Review

- [x] 3.1 Run the relevant Windows-local formatting and targeted `codex-core` validation commands using the documented root-level virtual environment
- [x] 3.2 Record validation outcomes, cleanup status, and residual review findings for this slice before taking the next batch

## Validation Evidence

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

## Residual Review Notes

- The current atomic gate still treats `prepared.artifact_refs.first()` as the handoff manifest; that ordering contract is acceptable for this slice but should be made more explicit or typed in a follow-on batch.
- Legacy generated skill files are repaired by prepending valid frontmatter and preserving old content under a legacy marker. That closes the current loader failure without yet performing a structured migration of stale legacy guidance.
- Full `cargo test -p codex-core` is still not the Windows completion gate for this batch; targeted validation remains the accepted boundary.
