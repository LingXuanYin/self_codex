## Context

The active `team-workflow` branch already enforces review-first handoff rules,
compact checkpoints, and artifact-based recovery. The next gap is narrower than
the original cross-cutting proposal: when `atomicWorkflows` is enabled,
finalize and handoff are currently gated by whether required checkpoint paths
appear in `prepared.artifact_refs`, not whether those files still exist on
disk.

That leaves an artifact-first recovery hole. A stale manifest can satisfy the
gate even after a required checkpoint file has been deleted. This first
implementation slice closes that hole without reopening broader protocol or
spawn-lifecycle work.

Constraints for this batch:

- Windows local development is the primary execution path.
- `.venv-tools` remains the canonical root-level virtual environment.
- The slice must stay bounded to one runtime policy gate and focused tests.
- Final artifact writes remain lead-owned; design and review inputs are folded
  back into committed docs before code changes start.

## Goals / Non-Goals

**Goals:**

- Make `atomicWorkflows` require real persisted checkpoint files, not only
  declared artifact references.
- Keep the implementation centered on the existing delivery gate in
  `codex-rs/core/src/team/runtime.rs`.
- Add focused tests that prove deleted checkpoint files block finalize or
  handoff.
- Preserve the current public protocol and workflow document contract while
  tightening runtime enforcement.

**Non-Goals:**

- Change public session or app-server protocol shapes.
- Rework child spawn lifecycle or fix ghost handoff artifacts in this batch.
- Redesign replan heuristics or broader workflow policy detection.
- Refactor unrelated portions of `team/runtime.rs`.
- Make the full Windows `codex-core` suite the required completion gate for
  this slice.

## Decisions

### Decision: enforce both declaration and on-disk existence for atomic checkpoints

- Decision:
  - Update the atomic workflow gate so success requires both:
    - each required checkpoint path is present in `prepared.artifact_refs`
    - each required checkpoint file currently exists on disk
- Why:
  - `prepared.artifact_refs` represents declared outputs, but the policy is
    about persisted recovery artifacts.
  - The resume flow already treats missing persisted files as a hard failure;
    finalize and handoff should align with that rule under `atomicWorkflows`.
- Alternatives considered:
  - Trust only `artifact_refs`.
    - Rejected because stale paths can outlive deleted files.
  - Recompute required artifacts from state without checking the filesystem.
    - Rejected because the gap is specifically missing persisted files.

### Decision: keep the change inside the existing runtime gate

- Decision:
  - Extend `has_atomic_checkpoint` in `codex-rs/core/src/team/runtime.rs`
    instead of introducing a wider structural refactor.
- Why:
  - The bug is localized to the current policy gate at delivery time.
  - A small helper change is easier to review and less risky in a large runtime
    module.
- Alternatives considered:
  - Move checkpoint validation into a new module now.
    - Rejected for this slice because the behavior change is small and urgent.

### Decision: prove the fix with delete-after-prepare tests

- Decision:
  - Use focused tests in `codex-rs/core/src/team/tests.rs` that prepare a
    message, delete one of the required checkpoint files, and assert that
    delivery fails under `atomicWorkflows`.
- Why:
  - The bug is not about manifest construction; it is about what happens after
    a valid manifest becomes stale.
  - Delete-after-prepare exercises the exact failure mode that currently slips
    through the gate.
- Alternatives considered:
  - Add only positive-path coverage.
    - Rejected because the missing-file regression is the core behavior to
      prove.

### Decision: defer adjacent review findings

- Decision:
  - Keep spawn ghost-handoff cleanup and replan heuristic redesign out of this
    batch.
- Why:
  - Those findings are real, but they touch different seams and would widen the
    implementation and review surface.
- Alternatives considered:
  - Bundle multiple review findings into one cross-cutting patch.
    - Rejected because it weakens the artifact-first, reviewable iteration
      contract.

## Risks / Trade-offs

- [Large runtime hotspot] -> Mitigation: keep the logic change small and
  targeted; do not refactor unrelated delivery behavior.
- [Cross-platform file existence edge cases] -> Mitigation: rely on the
  existing checkpoint paths already created by the runtime and validate with
  focused Windows-local tests.
- [Spec/doc drift] -> Mitigation: finish the doc updates and commit them before
  Rust edits begin.
- [Validation noise from Windows shell differences] -> Mitigation: use direct
  `cargo` commands and record any `just` limitations in the iteration evidence.

## Migration Plan

1. Align `CURRENT-STAGE.md`, `IMPLEMENTATION-REVIEW.md`, `design.md`, and
   `tasks.md` on the chosen slice.
2. Update the atomic checkpoint gate in `codex-rs/core/src/team/runtime.rs`.
3. Add or update focused tests in `codex-rs/core/src/team/tests.rs`.
4. Run targeted formatting and crate-local validation on Windows using the
   documented virtual environment.
5. Record review findings, validation evidence, and cleanup status before
   taking the next slice.

Rollback strategy:

- Revert the Rust implementation commit if the behavior change proves invalid.
- Keep the design and review docs if the slice remains the right direction, and
  revise only the implementation tasks.

## Open Questions

- Whether the later spawn-flow cleanup should reuse the same persisted-artifact
  validation primitives remains open, but it is not blocking this slice.
