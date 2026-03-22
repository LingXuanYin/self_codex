# Current Stage

## Active Mode

- Mode: `parallel`
- Branch: `feature/stabilize-team-workflow-rd-loop`
- OpenSpec change: `stabilize-team-workflow-rd-loop`
- OpenSpec status: all required artifacts are complete

## Current Direction

The current product and code direction is to stabilize the `team-workflow` runtime and multi-agent handoff loop so that design, development, and review can run as a governed, review-first workflow with recoverable local state.

## Review Snapshot

- Recent commits focus on team-workflow handoff hardening, path sanitization, OpenSpec scaffolding, and cross-platform recovery.
- `codex-rs/core/src/team/runtime.rs` is the primary orchestration surface for team state, handoff shaping, runtime docs, and operator-visible artifacts.
- `codex-rs/core/src/tools/handlers/multi_agents*.rs` is the primary integration surface for `spawn_agent`, `send_input`, and `resume_agent`.
- Current tests already cover many handoff and multi-agent paths, but they are concentrated in focused unit/integration-style tests rather than a fully stable Windows-wide suite.

## Current Assumptions

- The next implementation batch will continue the existing `team-workflow` direction instead of pivoting to a different feature area.
- Windows local development remains the primary execution environment for this cycle.
- Existing root-level `.venv-tools` can be reused as the baseline Python environment unless a new requirement forces a separate virtual environment.

## Current Iteration Ownership

- Lead: main Codex thread, responsible for branch ownership, artifact writes, and final integration decisions.
- Design: sub-agent `Hilbert`, which proposed the first bounded implementation candidates and their acceptance criteria.
- Development: main Codex thread for the first code batch, with bounded implementation delegation allowed after the design handoff is folded back into docs.
- Review: `IMPLEMENTATION-REVIEW.md` is the active baseline, and sub-agent `Harvey` supplied the prioritized review brief that selected the first batch gate.

## Selected First Slice

- Slice name: `atomic-checkpoint-existence-enforcement`
- Intent: make `atomicWorkflows` validate the required checkpoint files by actual file existence, not only by stale `artifact_refs`, so artifact-first recovery cannot be bypassed by deleted or stale checkpoint files.
- Primary files:
  - `codex-rs/core/src/team/runtime.rs`
  - `codex-rs/core/src/team/tests.rs`
  - `codex-rs/core/src/team/state.rs` for reference and fixture alignment
- Primary validation:
  - `codex-rs/core/src/team/tests.rs`
- Non-goals:
  - reworking sibling A2A contracts
  - changing vertical `openspec-artifacts` semantics
  - changing public session wire shapes in this first batch
  - broad `team/runtime.rs` refactors
  - making the full Windows `codex-core` suite the completion gate

## Current Blockers

- Root workflow documents have been canonicalized, but older references may still point to compatibility aliases and should be normalized as code work begins.
- Some repo recipes depend on POSIX shell execution, so local Windows validation must document cargo-first fallbacks where `just` cannot execute.
- The full `codex-core` suite is not yet treated as a reliable Windows completion gate; targeted validation remains necessary until the next batch tightens that story.
- The selected slice and review priorities still need to be folded into `IMPLEMENTATION-REVIEW.md`, `design.md`, and `tasks.md` before code changes begin.

## Next Intended Step

1. Update `IMPLEMENTATION-REVIEW.md` with the prioritized findings and the first-slice decision.
2. Update `design.md` and `tasks.md` so they are specific to `atomic-checkpoint-existence-enforcement`.
3. Start implementation only after those doc updates are committed.

## Compact Recovery

On resume:

1. Read this file first.
2. Read `TEAM-ORCHESTRATION.md`.
3. Read `IMPLEMENTATION-REVIEW.md`.
4. Read `LOCAL-DEV.md`.
5. Treat `TEAM_CHARTER.md` only as a compatibility alias, not the source of truth.
6. Read the active OpenSpec artifacts for `stabilize-team-workflow-rd-loop`.
7. Continue from the recorded next step, not hidden chat state.
