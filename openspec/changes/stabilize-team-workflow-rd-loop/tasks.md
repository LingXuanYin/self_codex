## 1. Governance And Recovery

- [ ] 1.1 Keep `TEAM-ORCHESTRATION.md`, `CURRENT-STAGE.md`, and `LOCAL-DEV.md` synchronized with the active batch boundary
- [ ] 1.2 Record active mode, assumptions, blockers, and next step in repo artifacts before any implementation batch starts
- [ ] 1.3 Confirm the Design, Development, and Review owners for the current batch before coding begins

## 2. Current-State Review And Design

- [ ] 2.1 Review `codex-rs/core/src/team/*` and `codex-rs/core/src/tools/handlers/multi_agents*` to select the first bounded hardening slice
- [ ] 2.2 Update design intent and acceptance criteria for the selected slice in the active artifacts
- [ ] 2.3 Define the targeted unit and end-to-end validation plan for the selected slice under Windows local-development constraints

## 3. Implementation

- [ ] 3.1 Implement the selected `team-workflow` runtime hardening changes
- [ ] 3.2 Implement the corresponding `multi_agents` integration, API, or test updates required by the selected slice
- [ ] 3.3 Update any root-level local-development artifacts required to keep the Windows workflow reproducible

## 4. Validation And Review

- [ ] 4.1 Run targeted crate tests for each touched Rust project
- [ ] 4.2 Run the required end-to-end scenarios for changed handoff and collaboration behavior, and record any environment-limited gaps
- [ ] 4.3 Record review findings, residual risks, and the next iteration decision before starting another development batch
