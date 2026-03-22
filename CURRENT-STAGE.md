# Current Stage

## Active Mode

- Mode: `parallel`
- Branch: `feature/stabilize-team-workflow-rd-loop`
- OpenSpec change: `stabilize-team-workflow-rd-loop`

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

## Current Blockers

- The exact next implementation delta after the recent handoff fixes still needs to be turned into explicit proposal/spec/design/tasks artifacts.
- Some repo recipes depend on POSIX shell execution, so local Windows validation must document cargo-first fallbacks where `just` cannot execute.
- The full `codex-core` suite is not yet treated as a reliable Windows completion gate; targeted validation remains necessary until the next batch tightens that story.

## Next Intended Step

1. Write `LOCAL-DEV.md` for Windows environment, virtualenv, test, and cleanup rules.
2. Generate OpenSpec proposal/specs/design/tasks for `stabilize-team-workflow-rd-loop`.
3. Use those artifacts to assign explicit Design, Development, and Review workstreams before any coding starts.

## Compact Recovery

On resume:

1. Read this file first.
2. Read `TEAM-ORCHESTRATION.md`.
3. Read the active OpenSpec artifacts for `stabilize-team-workflow-rd-loop`.
4. Continue from the recorded next step, not hidden chat state.
