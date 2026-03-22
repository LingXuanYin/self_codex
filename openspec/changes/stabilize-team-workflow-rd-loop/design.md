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
  `.codex/AGENT_TEAM.md`, `.codex/team-workflow.yaml`, and `LOCAL_ENV.md`
- runtime orchestration in `team/runtime.rs`
- operator/public projection in `team/api.rs`
- tool-surface enforcement in `tools/handlers/multi_agents/*`
- protocol visibility in `app-server-protocol`

## Goals / Non-Goals

**Goals:**

- Make the triad workflow observable and recoverable from committed artifacts.
- Keep same-level and vertical handoff behavior aligned across runtime, handlers, and public protocol.
- Preserve Windows-first local development by documenting and reusing the current virtual-environment/tooling setup.
- Tighten unit and end-to-end tests around the change boundary rather than relying on implicit behavior.

**Non-Goals:**

- Introduce a new collaboration mode beyond the current single/delegate/parallel model.
- Replace the existing `openspec-artifacts` vertical handoff protocol.
- Rework unrelated product areas outside `team-workflow`, multi-agent coordination, or the affected public protocol surface.
- Depend on Docker or a new external orchestration stack for local development.

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

### Decision: Keep one OpenSpec change with one focused capability

- Decision:
  - Use one change, `stabilize-team-workflow-rd-loop`, with one focused
    capability that covers the triad loop, recovery surface, and handoff
    expectations for the active `team-workflow` line.
- Why:
  - Governance, recovery, and handoffs are coupled in implementation and can
    be tested as one bounded workflow capability in this batch.
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

### Decision: Prefer targeted verification on Windows before broader suites

- Decision:
  - Run focused crate tests and explicit end-to-end scenarios first, then broaden only as far as the environment and repo policy allow.
- Why:
  - The current Windows environment has known `just`/shell limitations and broader suite noise.
- Alternatives considered:
  - Start with full workspace test execution.
    - Rejected because it is slower, noisier, and currently constrained by environment policy.

## Risks / Trade-offs

- [Large runtime hotspot] -> Mitigation: keep new logic out of `team/runtime.rs` unless the behavior truly belongs there; prefer focused helper modules if implementation grows.
- [Windows shell drift for `just` workflows] -> Mitigation: document exact fallback command patterns and isolate the impact in `LOCAL_ENV.md`.
- [Spec/doc drift from implementation] -> Mitigation: treat proposal/specs/design/tasks as preconditions for coding and update them before any scope change.
- [Protocol/runtime mismatch] -> Mitigation: pair core tests with `codex-app-server-protocol` validation and targeted end-to-end checks.

## Migration Plan

1. Commit repo-root workflow and recovery docs.
2. Commit OpenSpec artifacts that define the scope and testable requirements.
3. Implement the runtime/handler/protocol changes needed to satisfy the specs.
4. Run focused crate tests, then required end-to-end checks.
5. Perform review against committed docs plus test evidence.

Rollback strategy:

- Revert the implementation commits while preserving the documentation commits
  if the docs remain valid for the next attempt.
- If the design itself is invalidated, update the OpenSpec and repo-root docs
  in follow-up commits before retrying.

## Open Questions

- Which concrete runtime gaps remain after the latest handoff fixes once we compare current behavior against the new governance specs?
- How far should this iteration go on repository automation versus code-level enforcement of the document-first workflow?
- Which end-to-end scenarios are the minimum credible set for Windows in this environment without overfitting to local shell constraints?
