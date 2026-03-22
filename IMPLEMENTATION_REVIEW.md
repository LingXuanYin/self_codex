## Scope Reviewed

- Branch baseline: `feature/stabilize-team-workflow-rd-loop`
- Recent direction inferred from latest commits:
  - public team-workflow lifecycle exposure
  - A2A sibling-team boundaries
  - `openspec-artifacts` vertical handoff
  - path scrubbing and cross-platform artifact handling
  - recovery/compact safety

## Current Direction Summary

The active implementation direction is to turn `team-workflow` from an internal runtime experiment into a recoverable, reviewable, cross-platform collaboration loop with explicit public state, governed handoffs, and safer multi-agent boundaries.

## Primary Modules

### `codex-rs/core/src/team/config.rs`

- Defines workflow policy:
  - required triad roles: `design`, `development`, `review`
  - review-required loop
  - compact/resume expectations
  - artifact and memory provider policy

### `codex-rs/core/src/team/runtime.rs`

- Core orchestration surface for:
  - team initialization
  - child spawn preparation
  - message preparation/delivery
  - resume/compact checkpoints
  - artifact/handoff generation
- This is the highest-risk implementation hotspot due to size and cross-cutting behavior.

### `codex-rs/core/src/team/state.rs`

- Persists workflow metadata, recovery, audit, status, tape, handoff, and index files under `.codex/team-state`.

### `codex-rs/core/src/team/api.rs`

- Maps internal bundle/state into public workflow session objects and thread visibility views.

### `codex-rs/core/src/team/memory.rs`

- Encapsulates workflow memory-provider behavior and health exposure.

### `codex-rs/core/src/tools/handlers/multi_agents*.rs`

- Bridges user/model tool calls (`spawn_agent`, `send_input`, `resume_agent`, `wait_agent`) to team-aware runtime behavior.
- Responsible for enforcing path, depth, sandbox, handoff, and output-shape expectations.

## Current Test Surface

### Unit / focused tests

- `codex-rs/core/src/team/tests.rs`
  - workflow config validation
  - initialization artifacts
  - visibility
  - public session projection
  - handoff and recovery behavior

- `codex-rs/core/src/tools/handlers/multi_agents_tests.rs`
  - handler validation
  - spawn/send/resume behavior
  - team-workflow child handoff behavior
  - A2A manifest and vertical artifact manifest rendering

- `codex-rs/core/src/agent/control_tests.rs`
  - spawn/send/resume control semantics
  - descendant reopen/resume behavior

- `codex-rs/app-server-protocol`
  - public API and schema coverage for team-workflow session notifications/types

### Integration / end-to-end risk surface

- `codex-rs/core/tests/suite/*`
  - broader multi-agent, prompt, approval, and integration flows
- Windows full-suite stability is weaker than focused crate tests.

## Review Findings

### Finding 1: Runtime concentration risk

- `team/runtime.rs` carries workflow bootstrap, governance, path handling, handoff rendering, compact recovery, and artifact mapping in one place.
- Risk:
  - regression blast radius is high
  - review cost is high
  - future changes may violate the repo's module-size guidance

### Finding 2: Cross-platform path semantics remain fragile

- Recent commits already had to fix path escaping, root leakage, and non-git child worktree behavior.
- Risk:
  - Windows path normalization and git/worktree assumptions can still leak into artifact generation or public views

### Finding 3: Workflow policy and tool behavior can drift

- `team/config.rs` declares a strict design/development/review loop, but enforcement is distributed across runtime, handler, and public-session logic.
- Risk:
  - implementation may satisfy one layer while drifting in another
  - review gating and artifact gating can silently diverge

### Finding 4: Test evidence is split across crates

- Runtime behavior spans `core`, `app-server-protocol`, and potentially app-server consumers.
- Risk:
  - a focused fix may pass local unit tests while missing contract drift in public protocol or end-to-end flows

## Design Implications

- The next change should favor stabilization over feature breadth.
- New work should reduce orchestration ambiguity, not add new collaboration modes.
- Acceptance criteria should explicitly cover:
  - design/development/review loop visibility
  - Windows-safe local workflow
  - compact/recovery fidelity
  - public protocol consistency
