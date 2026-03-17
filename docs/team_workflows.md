# Team Workflows

Codex team workflows add a durable team operating model on top of the existing thread and turn runtime. The model is driven by a declarative workflow file, persisted team-state artifacts, and a root-scheduler-only external surface.

## Workflow file

Team workflows are defined in `.codex/team-workflow.yaml`.

The workflow file is expected to capture:

- the root scheduler role and decision policy
- child-team templates and bounded self-organization rules
- artifact and handoff policy
- governance document triggers
- git worktree and environment-management policy
- the maximum child-team depth

If `maxDepth` is not set, Codex uses a default of `5`.

## Delivery cycle rules

Every team cycle is required to include three explicit participants:

- design: define system boundaries, architecture, and module splits
- development: implement bounded work items and prepare handoff artifacts
- review: validate correctness, process adherence, and integration readiness

Review can push a cycle back into design or development. Teams do not close a cycle directly from development without review.

## Communication boundaries

The runtime uses different rules for horizontal and vertical communication:

- same-level sibling agents can exchange structured context through A2A-aligned peer messages
- parent and child teams cannot exchange raw working context
- cross-level communication is limited to persisted artifacts, files, and structured status manifests

This keeps recovery, accountability, and compact-safe continuation grounded in persisted artifacts instead of hidden transcript state.

## Governance documents

Team workflows maintain two runtime governance documents:

- `AGENT.md`: a global charter owned by the root scheduler
- `AGENT_TEAM.md`: a per-team consensus document owned by each team leader

Typical update points are:

- team creation
- major replans
- review-driven course corrections
- compact or leader handoff preparation

These runtime documents are separate from repository `AGENTS.md` instruction files. `AGENTS.md` still provides repository instructions to Codex itself, while `AGENT.md` and `AGENT_TEAM.md` describe the active team's operating rules and recovery state.

## Worktree and version strategy

Each team leader owns an isolated git worktree and branch namespace. The default strategy is:

- root leader works in the root team worktree
- each child leader receives a dedicated child worktree
- parent teams accept child output through merge, cherry-pick, or patch-style integration after review

This avoids sibling-team checkout collisions, keeps test environments scoped to the owning team, and supports multi-version reflection without sharing mutable workspace state across teams.

## Persisted team state

Runtime state is persisted under `.codex/team-state/`.

Team state includes:

- status snapshots
- handoff artifacts
- recent tape entries
- worktree and branch metadata
- environment and stale-resource cleanup state
- governance document paths

That persisted state is the source used to recover after compact, resume, or team-leader replacement.

## Agent-server surface

The app-server exposes team workflows through a root-scheduler-only model:

- only the root scheduler is public-facing
- child-team threads are hidden from `thread/list`, `thread/loaded/list`, and `thread/read`
- public status notifications suppress hidden child-team thread activity
- external clients receive redacted nested-team summaries, artifacts, and review state through the team workflow session API

Current team workflow session APIs are experimental and require `initialize.params.capabilities.experimentalApi = true`:

- `teamWorkflow/sessionRead`
- `teamWorkflow/sessionUpdated`

External clients should continue to send user instructions through the root scheduler thread with the normal `turn/start` flow.

## Web UI

When `codex app-server` is running over websocket transport, it now also serves a lightweight operations UI:

- `GET /team-ops`

The UI connects to the same websocket app-server endpoint and lets an operator:

- attach to a root scheduler thread
- inspect team topology and current workflow phase
- review governance documents and persisted artifacts
- inspect cleanup pressure and blockers
- send instructions only to the root scheduler

The UI intentionally reflects the persisted runtime state. It does not expose child-team raw context or provide a second orchestration model.
