# Install Script Specification — EdgeQuake Quickstart Wizard

## Overview

This directory contains Architecture Decision Records (ADRs) for the interactive
`quickstart.sh` setup wizard introduced in **EdgeQuake v0.9.12**.

## Problem Statement

The v0.9.11 quickstart script had three core defects:

1. **Flaky provider heuristic** — Provider was auto-detected from `OPENAI_API_KEY` presence.
   A user with the key set but wanting Ollama got OpenAI; a user without the key but with a
   custom Ollama setup got mis-configured silently.

2. **No model control** — Models were hard-coded defaults. Users had no UI to change them
   without editing env vars or re-reading documentation.

3. **Incomplete volume lifecycle** — Script only detected running containers. Stopped
   containers with existing data volumes were not surfaced, and there was no "fresh start"
   option for users wanting to wipe and restart.

## ADR Index

| ADR                                               | Title                                         | Status   |
| ------------------------------------------------- | --------------------------------------------- | -------- |
| [ADR-001](ADR-001-explicit-provider-selection.md) | Explicit provider selection via numbered menu | Accepted |
| [ADR-002](ADR-002-volume-lifecycle.md)            | Volume and container lifecycle handling       | Accepted |
| [ADR-003](ADR-003-model-catalogue.md)             | In-wizard model catalogue and selection       | Accepted |
| [ADR-004](ADR-004-tty-posix-compat.md)            | POSIX sh and curl-pipe TTY compatibility      | Accepted |
| [ADR-005](ADR-005-ux-design-system.md)            | UX design system for terminal wizards         | Accepted |

## Cross-Reference Map

```
User chooses provider (ADR-001)
    │
    ├─► OpenAI selected
    │       ├─► Prompt for / verify OPENAI_API_KEY
    │       └─► Choose GPT model + embedding model (ADR-003)
    │
    └─► Ollama selected
            ├─► Check /api/tags reachability
            └─► Choose Ollama model + embedding model (ADR-003)

Existing install detected (ADR-002)
    ├─► Running containers → Update & Reconfigure | Quit
    └─► Stopped + volumes → Restart | Reconfigure | Fresh Start | Quit
            └─► Fresh Start requires typing "DELETE" (irreversible guard)

All prompts via /dev/tty (ADR-004)
    └─► Works for: direct run | curl | sh | SSH with PTY
    └─► Fails gracefully without /dev/tty (CI) → clear error + env-var instructions

TUI design (ADR-005)
    └─► Colours only when stdout is a TTY
    └─► Unicode box-drawing in banner (UTF-8 source)
    └─► Consistent: section header → items → prompt → result
```

## Edge Cases Mitigated

| Edge Case                                    | Mitigation                                          |
| -------------------------------------------- | --------------------------------------------------- |
| `curl                                        | sh` (stdin is the script, not the terminal)         | All reads via `/dev/tty` |
| No `/dev/tty` (CI, Docker build)             | Detect early, exit with env-var instructions        |
| OPENAI_API_KEY set but user wants Ollama     | Always ask; show hint but don't auto-set            |
| Ollama not running                           | Check reachability; warn + ask to continue or abort |
| Ollama running but chosen model not pulled   | Warn with pull command; non-blocking                |
| Existing volumes + new install               | Detect, show status, offer 3 choices                |
| `docker volume rm` in compose vs direct      | Download compose first; use `down -v`               |
| Empty OPENAI_API_KEY injected into container | Unset before compose run; API strips empties        |
| Non-UTF-8 terminal (banner box-drawing)      | Graceful (display may have replacement chars)       |
| User enters invalid menu option              | Loop with clear error until valid choice given      |
| `set -e` with `grep -c` returning 1          | Wrap with `                                         |                          | echo 0` subshell to force exit 0 |
| Provider env already exported                | Script overrides with user's wizard choice          |
