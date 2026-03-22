## Platform Baseline

- OS: Windows
- Container policy: do not use Docker for this workflow
- Workspace root: `I:\self_codex`
- Primary Rust workspace: `I:\self_codex\codex-rs`

## Local Python Environment

- Canonical virtual environment: `I:\self_codex\.venv-tools`
- Purpose:
  - provide stable Python for build scripts and test helpers
  - avoid mixing repo automation with global Python state

### Required activation variables

Use these variables for Rust commands that transitively need Python:

```powershell
$env:PATH = 'I:\self_codex\.venv-tools\Scripts;C:\Users\35465\.cargo\bin;' + $env:PATH
$env:PYTHON = 'I:\self_codex\.venv-tools\Scripts\python.exe'
```

## Tooling State

- Rust toolchain is user-local via `rustup`
- Working commands currently available:
  - `cargo`
  - `rustc`
  - `rustfmt`
  - `clippy`
  - `just`
  - `rg`

## Known Windows Constraints

### `just` shell resolution

- `just` exists, but several recipes require a Unix shell.
- `just bazel-lock-check` currently fails because the recipe shell cannot be resolved cleanly on this Windows setup.
- Forcing `bash.exe` routes into WSL, but the WSL side is not fully usable for `/bin/bash` execution in this environment.

### Cargo lock contention

- Do not run multiple `cargo` processes in parallel.
- Repeated test runs can block on package/artifact locks.

### Full-suite stability

- Focused crate tests are usable.
- Some broad Windows integration failures may still be environmental or pre-existing and must be separated from change-specific regressions.

## Development Workflow Rules

- Put workflow and local-development documentation directly in the project root.
- Keep canonical progress/recovery state in committed Markdown, not transient terminal context.
- Before implementation, update root docs and OpenSpec artifacts first.

## Test Strategy

### Preferred order

1. crate-local unit/focused tests
2. targeted integration tests
3. end-to-end tests needed by the change
4. only then broader suites, subject to repo policy and explicit gating

### Current known-good checks

- `cargo test -p codex-app-server-protocol`
- targeted `cargo test -p codex-core <exact-test>`
- `cargo check -p codex-core`

## Cleanup Policy

- Remove temporary logs, test repos, and transient helper files when no longer needed.
- Do not leave extra virtual environments beyond `.venv-tools` unless the change requires a dedicated one.
- Close or stop background `cargo`, `rustc`, or helper processes after interrupted runs.

### Emergency cleanup

```powershell
Get-Process | Where-Object { $_.ProcessName -match 'cargo|rustc|rustdoc|clippy|just' } | Stop-Process -Force
```

## Compact Recovery Inputs

After any compact or interruption, recover from:

- `TEAM_CHARTER.md`
- `IMPLEMENTATION_REVIEW.md`
- `LOCAL_ENV.md`
- `WORKING_STATE.md`
- `openspec/changes/stabilize-team-workflow-rd-loop/*`
