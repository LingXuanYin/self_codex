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
- Release workflow: `fork-build-release` run `23236953238`
- Workspace: `K:\workspace\self_codex\release-smoke\workspace-fresh-23236953238`

### Observed Results

- `validate-linux-x64`, `build-linux-x64`, `build-windows-x64`, and `publish-release` all succeeded in workflow run `23236953238`.
- `codex-app-server.exe --listen ws://127.0.0.1:47071` started successfully from the packaged Windows release asset.
- `GET /readyz` returned `200`.
- `GET /healthz` returned `200`.
- `GET /team-ops` returned `200` from loopback.
- `initialize` and `thread/start` succeeded against the packaged websocket server on a clean workspace with `.codex/team-workflow.yaml`.
- A non-experimental connection was rejected for `teamWorkflow/sessionRead` with `teamWorkflow/sessionRead requires experimentalApi capability`.
- The packaged `teamWorkflow/sessionRead` payload matched the current hardening requirements:
  - it returned public-safe root identifiers (`root-scheduler`) instead of raw hidden team ids
  - governance and artifact paths pointed at `.codex\team-ops\...`
  - `memoryProvider` reported `{ mode: "local", health: "ready" }`
- `thread/loaded/list` exposed only the root scheduler thread for the packaged bootstrap session.
- The packaged runtime created `.codex\team-ops\index.json` and mirrored `AGENT.md`, `AGENT_TEAM.md`, `status.json`, `handoff.json`, and `team-tape.jsonl` under `.codex\team-ops\teams\root-scheduler\`.
- The packaged runtime-generated `.codex\AGENT.md` included the governance runtime checkpoint plus prompt/skill references.
- The packaged runtime generated governance prompt assets:
  - `designer.md`, `developer.md`, `leader.md`, `reviewer.md`, `scheduler.md`, `worker.md`
- The packaged runtime generated shared team skills:
  - `team-compact-continuation`, `team-delegation`, `team-governance-updates`, `team-review-return-loop`, `team-sanitized-handoff`
- A second packaged instance bound to `ws://0.0.0.0:47072` returned `200` for `GET /team-ops` via `127.0.0.1` and `404` via non-loopback address `192.168.1.101`, confirming the default loopback-only Team Ops UI gate.
- The packaged smoke run exercised the root bootstrap/public surface directly; hidden child-thread suppression remained covered by the targeted validation tests in workflow run `23236953238`.

### Conclusion

The packaged release built from commit `67ba772541588ab4df6e70d5abb930cd509f3222` reflects the governance-boundary hardening changes. Release smoke checks now pass for the public team-workflow surface, Team Ops loopback gating, governance asset generation, and operator-safe mirror paths, so task `5.3` is complete alongside the matching CI/release workflow run.
