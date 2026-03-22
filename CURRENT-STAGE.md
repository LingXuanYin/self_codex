# Current Stage

## Active Mode

- Mode: `parallel`
- Branch: `feature/stabilize-team-workflow-rd-loop`
- OpenSpec change: `stabilize-team-workflow-rd-loop`
- OpenSpec status: all required artifacts are complete

## Current Direction

The current product and code direction is to stabilize the `team-workflow` runtime and multi-agent handoff loop so that design, development, and review can run as a governed, review-first workflow with recoverable local state.

## Review Snapshot

- The first implementation slice is now landed in the working tree: `atomicWorkflows` requires the handoff manifest plus `status.json`, `handoff.json`, `team-tape.jsonl`, `AGENT.md`, and `AGENT_TEAM.md` to remain both declared and present on disk before delivery succeeds.
- `codex-rs/core/src/team/runtime.rs` remains the primary orchestration surface for team state, handoff shaping, runtime docs, and operator-visible artifacts.
- `codex-rs/core/src/tools/handlers/multi_agents*.rs` remains the primary integration surface for `spawn_agent`, `send_input`, and `resume_agent`; Windows-targeted tests for the touched paths passed in this cycle.
- Runtime-generated `.codex/skills/*/SKILL.md` files are now repaired to start with valid YAML frontmatter while preserving legacy content under a marker for manual follow-up if needed.

## Current Assumptions

- The next implementation batch will continue the existing `team-workflow` direction instead of pivoting to a different feature area.
- Windows local development remains the primary execution environment for this cycle.
- Existing root-level `.venv-tools` can be reused as the baseline Python environment unless a new requirement forces a separate virtual environment.

## Current Iteration Ownership

- Lead: main Codex thread, responsible for branch ownership, artifact writes, and final integration decisions.
- Design: sub-agent `Boyle`, which revalidated the exact six-checkpoint contract and kept the slice bounded.
- Development: main Codex thread for the first code batch, with bounded implementation delegation allowed after the design handoff is folded back into docs.
- Review: `IMPLEMENTATION-REVIEW.md` is the active baseline, and sub-agent `Harvey` confirmed the original blocker areas are closed with only low residual risks left.

## Selected First Slice

- Slice name: `atomic-checkpoint-existence-enforcement`
- Intent: make `atomicWorkflows` validate the handoff manifest plus `status.json`, `handoff.json`, `team-tape.jsonl`, `AGENT.md`, and `AGENT_TEAM.md` by actual file existence, not only by stale `artifact_refs`, so artifact-first recovery cannot be bypassed by deleted or stale checkpoint files.
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

- `just` recipes that assume a POSIX shell are still not reliable on this Windows machine; this cycle used `cargo clippy --fix` and Git Bash for `argument-comment-lint` as documented fallbacks.
- The full `codex-core` suite is still not treated as a reliable Windows completion gate; targeted validation remains necessary until the next batch tightens that story.
- The handoff manifest is still identified by `prepared.artifact_refs.first()` rather than a typed field; that ordering assumption remains a low follow-up risk rather than a blocker for this slice.
- Legacy skill repair currently preserves prior content under a marker rather than migrating it structurally; that is acceptable for loader recovery but remains a follow-up cleanup candidate.

## Next Intended Step

1. Commit the bounded implementation batch and the updated recovery/OpenSpec documents in reviewable units.
2. Push the branch state after commit hygiene is complete.
3. Start the next design-review-development cycle from the remaining follow-up findings, with `spawn_agent` ghost handoff artifacts as the leading deferred candidate.

## Compact Recovery

On resume:

1. Read this file first.
2. Read `TEAM-ORCHESTRATION.md`.
3. Read `IMPLEMENTATION-REVIEW.md`.
4. Read `LOCAL-DEV.md`.
5. Treat `TEAM_CHARTER.md` only as a compatibility alias, not the source of truth.
6. Read the active OpenSpec artifacts for `stabilize-team-workflow-rd-loop`.
7. Continue from the recorded next step, not hidden chat state.
