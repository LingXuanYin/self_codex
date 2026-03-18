# Team Workflow Release Smoke Test

This procedure validates packaged builds that include team workflow governance, public-session hardening, and Team Ops UI restrictions.

## Preconditions

- Use a packaged release artifact produced by CI or `fork-build-release.yml`.
- Run the binary on a clean workspace with a `.codex/team-workflow.yaml` file.
- Ensure the workspace can create `.codex/team-state/` and `.codex/team-ops/`.

## Smoke Test

1. Start the packaged app-server and confirm `readyz` and `healthz` respond.
2. Open a root scheduler session against a workspace that has team workflow enabled.
3. Trigger one delegation round so a child team is created and produces a vertical handoff.
4. Read `teamWorkflow/sessionRead` for the root thread and confirm:
   - nested teams use public-safe identifiers instead of real child thread ids
   - governance and artifact paths point at `.codex/team-ops/...`
   - the memory provider reports the expected mode and health
5. Confirm `thread/list` and `thread/loaded/list` do not surface hidden child threads.
6. Connect a non-experimental client and verify it does not receive `teamWorkflow/sessionUpdated` notifications.
7. Open `/team-ops` from loopback and confirm:
   - the UI loads
   - the UI reads only root scheduler state
   - governance docs and mirrored artifacts can be opened from `.codex/team-ops/...`
8. Attempt to open `/team-ops` from a non-loopback context and confirm it is withheld unless `CODEX_TEAM_OPS_UI_ALLOW_REMOTE` is set.
9. Review `.codex/team-governance/prompts/` and `.codex/skills/team-*` to confirm governance assets were generated.
10. Shut down the workflow and confirm stale-resource indicators are surfaced when managed worktrees remain.

## Expected Result

- Root scheduler remains the only public-facing agent.
- Vertical handoffs persist sanitized artifacts only.
- Public status surfaces fail closed.
- Tape remains optional and configuration-gated.
- Team Ops UI is restricted to the intended operator surface.

## Latest Run

- Date: `2026-03-18`
- Package under test: GitHub release `team-workflow-rust-v0.115.0-preview`
- Workspace: `K:\workspace\self_codex\release-smoke\workspace`

### Observed Results

- `codex-app-server.exe --listen ws://127.0.0.1:47071` started successfully.
- `GET /readyz` returned `200`.
- `GET /team-ops` returned `200` from loopback.
- `initialize` and `thread/start` succeeded against the packaged websocket server on a workspace with `.codex/team-workflow.yaml`.
- A non-experimental connection was rejected for `teamWorkflow/sessionRead` with `teamWorkflow/sessionRead requires experimentalApi capability`.
- The packaged `teamWorkflow/sessionRead` payload did not match the current hardening requirements:
  - it returned absolute workspace and team-state paths instead of `.codex/team-ops/...` operator-safe mirrors
  - it returned real root/team identifiers instead of the current redacted public-safe shape
  - it did not include the current `memoryProvider` status field
- The packaged runtime did not create the expected `.codex/team-ops/` mirror surface.
- The packaged runtime-generated `AGENT.md` also lacked the governance prompt/skill checkpoint lines expected from the current source tree.

### Conclusion

The smoke procedure was executed, but the latest downloadable preview artifact is not built from the current governance-boundary changes. A fresh packaged release from the current branch/commit is required before this procedure can pass and task `5.3` can be marked complete.
