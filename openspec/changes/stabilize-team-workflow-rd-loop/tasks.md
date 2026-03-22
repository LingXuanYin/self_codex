## 1. Workflow Baseline

- [ ] 1.1 Re-read `TEAM-ORCHESTRATION.md`, `CURRENT-STAGE.md`, `IMPLEMENTATION-REVIEW.md`, `LOCAL-DEV.md`, and the active OpenSpec artifacts before implementation starts and update any stale assumptions
- [ ] 1.2 Confirm the active OpenSpec artifacts still match the implementation target before editing code
- [ ] 1.3 Define the exact design/development/review evidence package that must be produced for this iteration

## 2. Governance And Recovery Implementation

- [ ] 2.1 Audit `codex-rs/core/src/team/config.rs`, `state.rs`, and related runtime entry points against the new governance spec
- [ ] 2.2 Implement any missing behavior needed to keep triad workflow state and compact recovery aligned with committed artifacts
- [ ] 2.3 Add or update focused tests for governance, recovery ordering, and review-gate evidence handling

## 3. Handoff And Visibility Stabilization

- [ ] 3.1 Audit `codex-rs/core/src/team/runtime.rs` and `tools/handlers/multi_agents/*` against the handoff spec
- [ ] 3.2 Implement any missing same-level A2A, vertical `openspec-artifacts`, path-sanitization, or public-visibility fixes
- [ ] 3.3 Update `codex-rs/app-server-protocol` and generated schema fixtures if the public contract changes
- [ ] 3.4 Add or update focused unit and end-to-end tests for sibling handoffs, vertical handoffs, and public workflow session behavior

## 4. Verification And Review

- [ ] 4.1 Run the relevant crate-local unit tests and required end-to-end scenarios on Windows using the documented virtual environment
- [ ] 4.2 Run formatting, lint/fix steps, and any feasible lock/schema checks permitted by the environment and repo policy
- [ ] 4.3 Record cleanup status, unresolved trade-offs, and review findings before concluding the iteration
