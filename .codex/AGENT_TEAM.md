# AGENT_TEAM.md

## Mission

Stabilize and iterate the current `team-workflow` direction with an explicit
design-development-review loop, artifact-based recovery, and Windows-local
validation that can continue safely after compact or interruption.

## Current Working State

- Active mode: `parallel`
- Lead branch: `feature/stabilize-team-workflow-rd-loop`
- Active change: `openspec/changes/stabilize-team-workflow-rd-loop/`
- Current direction: continue hardening the team-workflow surface around
  public session visibility, handoff contracts, governance checkpoints,
  compact recovery, and role-based review gates.
- Assumptions:
  - The active product direction remains the current `team-workflow` branch
    rather than a separate feature line.
  - All local development must stay on Windows and avoid Docker.
  - Local Python-dependent tooling uses the repo-root virtual environment at
    `.venv-tools`.
  - No implementation starts until the governance, environment, and OpenSpec
    artifacts are written and committed.
- Known blockers:
  - `just bazel-lock-check` is currently blocked by missing usable Unix shell
    plumbing on this Windows setup; direct Cargo validation still works.
  - Full workspace test breadth may require selective Windows-safe execution
    rather than blindly running every integration test.
- Next intended step:
  - Finish proposal, design, spec, and task artifacts.
  - Review the current implementation against those artifacts.
  - Select the first bounded implementation slice for the next cycle.

## Team Topology

- Lead / root scheduler:
  - Owns branch, commits, final artifact writes, integration decisions, and
    stop conditions.
- Design lead:
  - Owns architecture boundaries, interface contracts, artifact schemas,
    replan criteria, and compact recovery expectations.
- Development lead:
  - Owns bounded implementation slices, local environment execution, and test
    evidence for the current slice.
- Review lead:
  - Owns drift detection, validation quality, security/scope checks, and the
    decision to return work to design or development.

## Required Iteration Loop

Every substantive cycle must include all three roles:

1. Design defines or updates the bounded contract.
2. Development implements the bounded slice and records evidence.
3. Review accepts the slice or returns it with concrete findings.

No cycle is complete if review is skipped or reduced to a build-only check.

## Information Flow

- Same-level coordination:
  - Use bounded A2A-style messages with explicit phase, intent, summary, and
    artifact references.
- Cross-level handoff:
  - Use `openspec-artifacts` as the contract surface.
- Required handoff payload:
  - Scope summary
  - Artifact refs
  - Validation evidence
  - Blockers and next action
- Review return path:
  - Review may return work to design for boundary drift.
  - Review may return work to development for implementation or validation
    gaps.
  - The lead updates this file before the next delegation round when review
    changes reusable rules.

## Single-Writer Rule

- The lead agent is the only writer for final artifacts in this workflow.
- Sub-agents may inspect, critique, and prepare bounded input.
- If a future implementation slice is delegated for code changes, write scopes
  must be disjoint and explicitly assigned before delegation begins.

## Compact Recovery

On resume after compact or interruption, restore context in this order:

1. `.codex/AGENT_TEAM.md`
2. `.codex/team-workflow.yaml`
3. `openspec/changes/stabilize-team-workflow-rd-loop/proposal.md`
4. `openspec/changes/stabilize-team-workflow-rd-loop/design.md`
5. `openspec/changes/stabilize-team-workflow-rd-loop/specs/`
6. `openspec/changes/stabilize-team-workflow-rd-loop/tasks.md`
7. `git log --oneline --decorate -10`

Do not rely on hidden chat state when these artifacts are available.

## Commit Policy

Before implementation, each documentation milestone must land in its own
reviewable commit:

- Governance/team setup
- Local environment notes
- Proposal
- Design
- Spec
- Tasks

## Validation Policy

- Prefer Windows-local, repo-root execution.
- Use `.venv-tools` for Python-backed tooling and build scripts.
- Run unit tests for the changed crate first, then targeted end-to-end coverage.
- Clean up stale `cargo`/`rustc` processes, temp repos, generated logs, and
  other disposable artifacts after validation rounds.

## Update Triggers

Update this file whenever any of the following occurs:

- Team created
- Replan
- Review handoff
- Compact
- Lead resume
