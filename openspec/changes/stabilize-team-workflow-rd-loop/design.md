## Context

Recent work on this branch has already pushed the codebase toward a governed `team-workflow` model: `codex-rs/core/src/team/runtime.rs` owns team state, handoff shaping, and operator-facing runtime documents, while `codex-rs/core/src/tools/handlers/multi_agents*.rs` owns `spawn_agent`, `send_input`, and `resume_agent` integration behavior. The remaining gap is operational rather than purely mechanical: the next iteration needs an explicit review-first loop, durable recovery artifacts, and a documented Windows-local validation contract so further hardening does not depend on hidden state or inconsistent test choices.

## Goals / Non-Goals

**Goals:**
- Establish repository-local artifacts that let Design, Development, and Review coordinate every batch explicitly.
- Make compact recovery deterministic by recording active mode, assumptions, blockers, and next step in repo documents and OpenSpec artifacts.
- Define a bounded implementation path for the next `team-workflow` hardening batch across runtime, multi-agent handlers, and validation.
- Keep the workflow compatible with Windows local development and the existing root-level virtual environment.

**Non-Goals:**
- Redesign the entire `team-workflow` architecture in one batch.
- Treat current Windows limitations in POSIX-shell-backed `just` recipes as a blocker to all progress.
- Replace existing targeted validation with a premature workspace-wide gate before the next hardening batch is defined.

## Decisions

### Decision: Use root-level documents plus OpenSpec artifacts as the recovery surface
- Rationale: root-level documents are immediately visible for local work and compact recovery, while OpenSpec artifacts define implementation intent and acceptance boundaries.
- Alternatives considered:
  - Use chat state only: rejected because compact recovery becomes unreliable.
  - Use only OpenSpec files: rejected because environment and orchestration details also need a stable root-level operator surface.

### Decision: Enforce a single-writer workflow with delegated analysis
- Rationale: the user explicitly requires one lead writer per artifact while still encouraging heavy sub-agent use. Serial artifact authorship avoids merge and narrative drift.
- Alternatives considered:
  - Let each role write its own final document: rejected because it violates the single-writer rule and raises recovery ambiguity.
  - Avoid delegation entirely: rejected because it leaves useful parallel review and audit capacity unused.

### Decision: Bound the first implementation slice around `team/runtime`, `multi_agents`, and validation records
- Rationale: those modules are where the current direction is concentrated, and recent commits show this is the active risk surface.
- Alternatives considered:
  - Expand immediately into unrelated UI or app-server areas: rejected because it dilutes the current hardening direction.
  - Restrict the iteration to documentation only: rejected because the user expects the workflow to lead directly into real development and test validation.

### Decision: Standardize on Windows local execution with cargo-first fallbacks
- Rationale: this environment already supports `.venv-tools`, `cargo fmt`, `cargo check`, and targeted tests, while some `just` recipes are not reliably executable due to shell/WSL assumptions.
- Alternatives considered:
  - Use Docker: rejected by user requirement.
  - Depend on WSL-backed shell recipes as a hard gate: rejected because the current machine cannot treat them as deterministic local infrastructure.

## Risks / Trade-offs

- [Risk] Documentation and implementation can drift if delegated role outputs are not folded back into the lead-written artifacts. → Mitigation: every batch starts and ends with `CURRENT-STAGE.md` and OpenSpec updates.
- [Risk] Windows-local validation may still miss failures that only appear in broader POSIX-oriented CI paths. → Mitigation: record targeted validation plus any known unvalidated surfaces explicitly in review notes.
- [Risk] `team/runtime` remains a high-touch orchestration module, so new hardening work can accidentally increase coupling. → Mitigation: prefer bounded changes and extract new logic into smaller modules when the next code batch justifies it.

## Migration Plan

1. Land documentation and OpenSpec artifacts first.
2. Assign the next bounded implementation slice from `tasks.md`.
3. Run targeted validation on changed crates and behaviors.
4. Record review findings and decide whether the next batch expands scope or closes the change.

Rollback for documentation-only steps is standard git revert. Rollback for later code batches will be defined in the implementation batch notes if behavior changes become broader.

## Open Questions

- Which specific hardening delta should be prioritized first after the workflow documents land: runtime decomposition, handler contract tightening, or test-surface expansion?
- How much of the broader `codex-core` suite should become a required gate for Windows-local completion in this change, versus recorded but non-blocking validation?

## Recovery Snapshot

- Mode: `parallel`
- Assumptions: current direction remains `team-workflow` hardening with review-first governance.
- Blockers: the first bounded implementation batch still needs to be selected from the task list.
- Next Step: write `tasks.md` with explicit design, development, and review checkpoints.
