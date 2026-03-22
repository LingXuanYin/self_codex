# Current Stage

## Active Mode

- Mode: `parallel`
- Branch: `feature/stabilize-team-workflow-rd-loop`
- OpenSpec change: `stabilize-team-workflow-rd-loop`
- OpenSpec status: active change reopened for the next implementation slice

## Current Direction

The current product and code direction is to stabilize the `team-workflow` runtime and multi-agent handoff loop so that design, development, and review can run as a governed, review-first workflow with recoverable local state.

## Review Snapshot

- The first implementation slice is complete and pushed: `atomicWorkflows` requires the handoff manifest plus `status.json`, `handoff.json`, `team-tape.jsonl`, `AGENT.md`, and `AGENT_TEAM.md` to remain both declared and present on disk before delivery succeeds.
- `codex-rs/core/src/team/runtime.rs` remains the primary orchestration surface for team state, handoff shaping, runtime docs, and operator-visible artifacts.
- `codex-rs/core/src/tools/handlers/multi_agents*.rs` remains the primary integration surface for `spawn_agent`, `send_input`, and `resume_agent`; Windows-targeted tests for the touched paths passed in this cycle.
- Runtime-generated `.codex/skills/*/SKILL.md` files are now repaired to start with valid YAML frontmatter while preserving legacy content under a marker for manual follow-up if needed.
- The active spawn ghost-artifact slice is now implemented in the working tree: `spawn_agent` prepares sanitized child handoff input in memory and only persists the manifest and optional patch after child spawn succeeds.

## Current Assumptions

- The next implementation batch will continue the existing `team-workflow` direction instead of pivoting to a different feature area.
- Windows local development remains the primary execution environment for this cycle.
- Existing root-level `.venv-tools` can be reused as the baseline Python environment unless a new requirement forces a separate virtual environment.

## Current Iteration Ownership

- Lead: main Codex thread, responsible for branch ownership, artifact writes, and final integration decisions.
- Design: delegated analysis completed for the bounded spawn ghost-artifact slice; the lead thread owns final document wording and code integration.
- Development: main Codex thread for the next code batch, with bounded implementation delegation allowed only after the pre-code document gate is recommitted.
- Review: `IMPLEMENTATION-REVIEW.md` is the active baseline, and the current review outcome keeps the later post-spawn recording window as a follow-up risk rather than a blocker for this batch.

## Selected Next Slice

- Slice name: `spawn-agent-ghost-handoff-artifacts`
- Intent: ensure `spawn_agent` can prepare sanitized child handoff content in memory, but it does not persist spawn manifests, integration patches, or mirrored operator artifacts before child spawn success is known, while keeping the successful spawn path unchanged.
- Primary files:
  - `codex-rs/core/src/tools/handlers/multi_agents/spawn.rs`
  - `codex-rs/core/src/team/runtime.rs`
  - `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
- Primary validation:
  - `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
- Non-goals:
  - reordering the full `agent_control` spawn lifecycle outside the bounded spawn handoff boundary
  - redesigning the `openspec-artifacts` manifest format
  - reworking post-spawn `record_child_team_spawn` compensation or the full child bootstrap lifecycle after successful spawn
  - changing sibling A2A behavior or vertical handoff policy semantics
  - making the full Windows `codex-core` suite the completion gate

## Current Blockers

- `just` recipes that assume a POSIX shell are still not reliable on this Windows machine; this cycle used `cargo clippy --fix` and Git Bash for `argument-comment-lint` as documented fallbacks.
- The full `codex-core` suite is still not treated as a reliable Windows completion gate; targeted validation remains necessary until the next batch tightens that story.
- The current implementation still lives alongside unrelated dirty worktree noise in `codex-rs/app-server/src/codex_message_processor.rs`, `codex-rs/core/src/team/api.rs`, and `codex-rs/core/src/thread_manager.rs`; those changes are outside this slice and must stay out of the final handoff or commit.
- The current design assumption remains a spawn-only two-phase handoff flow: prepare sanitized child input in memory first, then persist the manifest, optional patch, mirrors, and delegation bookkeeping only after child spawn success is known.
- Review identified a deeper post-spawn `record_child_team_spawn` failure window, but that compensation problem is explicitly deferred to a later slice.
- The handoff manifest is still identified by `prepared.artifact_refs.first()` rather than a typed field; that ordering assumption remains a low follow-up risk rather than a blocker for this slice.
- Legacy skill repair currently preserves prior content under a marker rather than migrating it structurally; that is acceptable for loader recovery but remains a follow-up cleanup candidate.

## Next Intended Step

1. Isolate the bounded spawn ghost-artifact code and documentation changes from unrelated dirty worktree noise.
2. Commit the updated OpenSpec and repo-root recovery documents together with the bounded Rust implementation for this slice.
3. Push the branch and hand off the remaining follow-up risks from `IMPLEMENTATION-REVIEW.md` into the next iteration plan.

## Compact Recovery

On resume:

1. Read this file first.
2. Read `TEAM-ORCHESTRATION.md`.
3. Read `IMPLEMENTATION-REVIEW.md`.
4. Read `LOCAL-DEV.md`.
5. Treat `TEAM_CHARTER.md` only as a compatibility alias, not the source of truth.
6. Read the active OpenSpec artifacts for `stabilize-team-workflow-rd-loop`.
7. Continue from the recorded next step, not hidden chat state.
