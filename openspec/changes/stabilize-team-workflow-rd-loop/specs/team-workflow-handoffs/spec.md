## ADDED Requirements

### Requirement: Same-level team communication uses reviewable manifests
Sibling-team communication SHALL be represented as reviewable A2A-compatible manifests rather than opaque raw text handoffs.

#### Scenario: Sibling team sends context laterally
- **WHEN** one team sends input to another team at the same workflow level
- **THEN** the delivered content is represented as a reviewable manifest that preserves sender intent without bypassing the workflow protocol

#### Scenario: Lateral handoff stays protocol-aligned
- **WHEN** same-level team context is exchanged
- **THEN** the handoff uses the configured same-level protocol and does not fall back to an undocumented ad hoc message shape

### Requirement: Vertical handoff uses sanitized OpenSpec artifacts
Cross-level handoff from child to parent SHALL use `openspec-artifacts` manifests with sanitized, workspace-safe paths and reviewable integration metadata.

#### Scenario: Child team hands work upward
- **WHEN** a child team delivers work to its parent
- **THEN** the workflow emits an `openspec-artifacts` manifest with relative operator-visible paths, integration metadata, and no leaked hidden-child identifiers

#### Scenario: Non-git workspace remains valid
- **WHEN** a child team operates in a workspace that is not backed by a managed git worktree
- **THEN** the vertical handoff still succeeds without pretending that a managed worktree exists

### Requirement: Public workflow visibility hides child-private details
Public workflow session data SHALL expose root-visible lifecycle summaries while withholding child-private identifiers and sensitive handoff details that are not meant for root-level public views.

#### Scenario: Root session is queried
- **WHEN** a public team-workflow session is loaded for a root thread
- **THEN** it exposes lifecycle, memory-provider, and integration summary information appropriate for the root operator view

#### Scenario: Hidden child details are scrubbed
- **WHEN** handoff or integration data originated from a hidden child thread
- **THEN** child-private identifiers and unsafe absolute path details are not exposed through public workflow session data or rendered manifests
