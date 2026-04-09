# ADR-002 — Volume and Container Lifecycle Handling

**Status:** Accepted  
**Date:** 2026-04-09  
**Version:** EdgeQuake v0.9.12

---

## Context

The v0.9.11 script detected only **running containers** for the prior-install check.
Two additional states were not surfaced:

| State                        | v0.9.11 behaviour               | Problem                             |
| ---------------------------- | ------------------------------- | ----------------------------------- |
| Running containers           | Warning + Update/Quit prompt    | ✓ OK                                |
| Stopped containers + volumes | Silent restart (force-recreate) | User unaware data exists            |
| Volumes only (no containers) | Silent fresh start              | User's data deleted without consent |
| Nothing → fresh install      | Silent install                  | ✓ OK                                |

The script also lacked a "fresh start" option (wipe all data), which users hitting a broken
state needed to recover cleanly.

## Decision

Detect all three states and present a context-aware menu for each.

### Detection Logic

```
_running  = count of containers whose status is "running" and name contains "edgequake"
_stopped  = count of containers (any status) whose name contains "edgequake"
_volumes  = count of volumes whose name contains "edgequake"
```

> Note: `_running > 0` implies `_stopped > 0` (running ⊂ all containers).

### State Machine

```
┌─────────────────────────────────────────────────────────┐
│ State                     Menu shown                    │
├─────────────────────────────────────────────────────────┤
│ nothing (fresh)           → skip straight to wizard     │
│ running containers        → Update & Reconfigure | Quit │
│ stopped + volumes         → Restart (wizard) |          │
│                              Fresh Start | Quit         │
│ volumes only (orphaned)   → same as stopped + volumes   │
└─────────────────────────────────────────────────────────┘
```

### "Fresh Start" Confirmation Gate

Fresh start is irreversible. The user must type the exact string `DELETE` to confirm:

```
  ⚠  This will permanently delete ALL EdgeQuake data (PostgreSQL volumes, graph).

  Type DELETE to confirm, or press Enter to cancel:
```

- Any other input → "data preserved" message → falls through to restart with data.
- "DELETE" typed → `docker compose down -v --remove-orphans` → proceeds to wizard.

### Compose File Download Order

The compose file must be downloaded **before** the lifecycle detection step so that
`$COMPOSE_CMD -f "$COMPOSE_FILE" down -v` is available for the fresh start path.

## Consequences

### Positive
- Users never accidentally delete data (two-step confirmation for destructive operations).
- All four states are visible and handled explicitly.
- "Orphaned volumes" (leftover after manual `docker rm`) are caught.

### Negative
- Slightly longer code path for fresh installs (three extra `docker` calls that return
  quickly with empty output).

## Edge Cases

| Edge case                                           | Handling                                                   |
| --------------------------------------------------- | ---------------------------------------------------------- |
| `docker ps` returns error (Docker not running)      | `                                                          |  | true` prevents abort; counts default to 0 → fresh install path |
| Compose file changed between installs               | Always download fresh → `down -v` uses the new file        |
| User presses Ctrl+C during fresh start              | `set -e` exits; data may be partially removed → documented |
| Volumes present but with different prefix           | Only scoped to `name=edgequake` filter                     |
| Mixed state (some containers running, some stopped) | `_running > 0` check fires first                           |

## Implementation Notes

- Use `2>/dev/null || echo 0` pattern inside `$(...)` to prevent `set -e` abort on grep
  returning exit 1 (no matches).
- Use `docker ps -a --filter "name=edgequake"` (not project-label) for robustness across
  different compose project name settings.
- The `_confirm_fresh_start` function is separate to keep the main flow readable (SRP).
