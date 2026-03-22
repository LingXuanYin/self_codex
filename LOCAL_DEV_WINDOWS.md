# Local Development On Windows

## Environment Baseline

- Host OS: Windows local development
- Container policy: do not use Docker for this work
- Python virtual environment: `I:\self_codex\.venv-tools`
- Rust toolchain:
  - `cargo 1.94.0`
  - `rustc 1.94.0`
- Python version:
  - `Python 3.13.12`

## Required Local Paths

- Workspace root: `I:\self_codex`
- Rust workspace: `I:\self_codex\codex-rs`
- Python executable for local tooling:
  - `I:\self_codex\.venv-tools\Scripts\python.exe`
- User cargo bin:
  - `C:\Users\35465\.cargo\bin`

## Recommended Shell Environment

Use the virtual environment and cargo bin path explicitly before running Rust commands that may need Python-backed build steps:

```powershell
$env:PATH='I:\self_codex\.venv-tools\Scripts;C:\Users\35465\.cargo\bin;' + $env:PATH
$env:PYTHON='I:\self_codex\.venv-tools\Scripts\python.exe'
```

## Tooling Notes

- `just` is installed and callable as `just 1.47.1`
- `bash.exe` resolves to `C:\Windows\system32\bash.exe`
- The root `justfile` contains recipes that invoke POSIX shell scripts such as `./scripts/check-module-bazel-lock.sh`
- In the current environment, those POSIX-script recipes are not reliable because local WSL integration does not provide a working `/bin/bash`
- For Windows-local development in this session, prefer direct `cargo` commands when a `just` recipe shells out to Unix scripts

## Verified Constraints

- `openspec` is available from the workspace root
- `cargo test -p codex-app-server-protocol` is a known-good validation path on this machine
- `cargo check -p codex-core` is a known-good validation path on this machine
- Targeted `codex-core` tests are reliable when run serially
- Avoid running multiple `cargo` processes concurrently because package and artifact locks are noisy on this machine

## Validation Policy For This Change

1. Run document and planning updates before implementation.
2. For Rust implementation:
   - run the smallest relevant crate tests first
   - run targeted end-to-end or integration tests for the touched workflow paths
   - only escalate to broader suites when the changed surface is stable
3. If a required `just` recipe is blocked by shell availability:
   - record the exact blocker in the active artifacts
   - use the closest direct `cargo` fallback when possible
   - do not fake success for Bazel lock or shell-script-backed checks

## Known Windows-Specific Gaps

- `just bazel-lock-check` currently fails because the recipe requires a Unix shell path that is not available through the current WSL setup
- Other `just` recipes that only wrap `cargo` are conceptually valid, but direct `cargo` invocation is more reliable for this session
- Full Rust workspace suites may expose unrelated Windows-specific integration noise; treat those separately from targeted change validation

## Resource Hygiene

- Keep Python tooling inside `.venv-tools`
- Do not create extra ad hoc virtual environments unless the active one becomes unusable
- Clean up temporary git repos, temporary rollout artifacts, and stale test workspaces after end-to-end runs
- If `cargo` or `rustc` processes are left hanging and block new runs, stop the stale processes before retrying

## Compact Recovery Facts

- Continue to use `.venv-tools` unless a later commit documents a replacement
- Default to direct `cargo` commands from `I:\self_codex\codex-rs`
- Re-check shell-backed `just` recipes only when the shell situation changes
