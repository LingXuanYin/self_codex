# Local Development

## Scope

This repository is developed locally on Windows without Docker. All local-development guidance for the current iteration lives at the project root so the team can recover and continue after compact or interruption.

## Baseline Environment

- OS: Windows
- Shell: PowerShell
- Repository root: `I:\self_codex`
- Rust workspace root: `I:\self_codex\codex-rs`
- Python virtual environment: `I:\self_codex\.venv-tools`

## Required Environment Settings

Use the root-level virtual environment when running Rust commands that depend on Python-backed build steps.

PowerShell:

```powershell
$env:PATH = 'I:\self_codex\.venv-tools\Scripts;C:\Users\35465\.cargo\bin;' + $env:PATH
$env:PYTHON = 'I:\self_codex\.venv-tools\Scripts\python.exe'
```

## Tooling Notes

- Prefer `cargo` directly when `just` recipes depend on POSIX shell behavior that is not available in the local Windows setup.
- `just bazel-lock-check` currently depends on a shell path that falls through to WSL and is not a reliable local gate on this machine.
- Do not use Docker for this cycle.
- Keep all temporary development support under the project root when additional local files are needed.

## Validation Rules

- Run targeted crate tests first.
- Use targeted exact tests when validating `team-workflow` and `multi_agents` changes.
- Treat Windows-wide full-suite results carefully; distinguish product regressions from environment-limited failures.
- Record the exact validation commands and outcomes in iteration documents before handing off to Review.

## Cleanup Rules

- Stop stray `cargo`, `rustc`, `rustdoc`, `clippy`, or `just` processes if they block artifact locks.
- Remove temporary test logs and throwaway local helper files after use.
- Do not leave duplicate virtual environments behind unless a task explicitly requires them.
- Reuse `.venv-tools` unless the change introduces a conflicting interpreter/toolchain need.

## Current Local Constraint Summary

- Python-backed Rust builds are working with `.venv-tools`.
- `cargo fmt`, `cargo check`, and targeted tests are viable locally.
- POSIX-shell-backed `just` recipes are not a reliable Windows gate in the current environment and must be documented as such when encountered.
