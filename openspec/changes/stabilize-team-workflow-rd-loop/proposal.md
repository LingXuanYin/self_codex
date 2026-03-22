## Why

The repository already contains substantial `team-workflow` runtime behavior, but the next iteration still lacks a formal change contract that ties governance, handoff boundaries, recovery artifacts, and Windows-local validation into one review-first development loop. This change is needed now to prevent design, development, and review from drifting apart while the implementation is actively evolving.

## What Changes

- Formalize the current `team-workflow` direction as an implementation contract before any new code batch starts.
- Define a governed design-development-review loop with explicit persisted recovery points, role handoffs, and review gates.
- Define the operator-facing and local-development requirements for Windows-first, no-Docker development using a root-level virtual environment.
- Establish the validation contract for targeted unit and end-to-end evidence around `team`, `multi_agents`, and workflow protocol behavior.

## Capabilities

### New Capabilities

- `team-workflow-rd-loop`: Defines the governed review-first workflow, persisted recovery artifacts, local Windows development baseline, and verification contract for the current `team-workflow` direction.

### Modified Capabilities

- None.

## Impact

- Affected code:
  - `codex-rs/core/src/team/config.rs`
  - `codex-rs/core/src/team/runtime.rs`
  - `codex-rs/core/src/team/api.rs`
  - `codex-rs/core/src/tools/handlers/multi_agents*.rs`
  - `codex-rs/app-server-protocol/src/protocol/common.rs`
  - `codex-rs/app-server-protocol/src/protocol/v2.rs`
- Affected systems:
  - Root-level workflow documentation and compact recovery flow
  - Windows local development environment and validation path
  - Team-workflow public visibility, handoff, and review governance

## Planning State

- Active mode: `parallel`
- Current assumptions:
  - The current direction remains `team-workflow` hardening rather than a product pivot.
  - Windows local development without Docker remains the primary execution path for this cycle.
  - `.venv-tools` remains the baseline Python environment unless implementation forces a new root-level environment.
- Current blockers:
  - The exact implementation slice still needs to be narrowed into spec/design/tasks artifacts.
  - Some `just` recipes are not reliable on the current Windows shell path and require documented `cargo`-first fallbacks.
  - Full Windows-wide completion criteria still require targeted validation rather than broad suite trust.
- Next intended step:
  - Define the normative workflow and local-validation requirements in specs, then convert them into design and tasks.
