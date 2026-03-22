## 1. Workflow Baseline

- [x] 1.1 Re-read `TEAM-ORCHESTRATION.md`, `CURRENT-STAGE.md`, `IMPLEMENTATION-REVIEW.md`, `LOCAL-DEV.md`, and the active OpenSpec artifacts before implementation starts
- [x] 1.2 Confirm the first implementation target is `atomic-checkpoint-existence-enforcement` and keep the active branch/change aligned to that scope
- [x] 1.3 Record the design, development, and review evidence package in committed repo-root and OpenSpec artifacts before Rust edits begin

## 2. Atomic Checkpoint Enforcement

- [x] 2.1 Audit `codex-rs/core/src/team/runtime.rs`, `codex-rs/core/src/team/state.rs`, and `codex-rs/core/src/team/tests.rs` around `atomicWorkflows` and persisted checkpoint expectations
- [ ] 2.2 Update the atomic workflow delivery gate so required checkpoint files must both be declared in `prepared.artifact_refs` and exist on disk before finalize or handoff succeeds
- [ ] 2.3 Add or update focused tests in `codex-rs/core/src/team/tests.rs` that delete required checkpoint files after message preparation and assert `atomicWorkflows` blocks delivery

## 3. Validation And Review

- [ ] 3.1 Run the relevant Windows-local formatting and targeted `codex-core` validation commands using the documented root-level virtual environment
- [ ] 3.2 Record validation outcomes, cleanup status, and residual review findings for this slice before taking the next batch
