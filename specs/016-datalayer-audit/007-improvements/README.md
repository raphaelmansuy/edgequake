# 007 — Improvements & Migration Plan

A prioritized, edge-case-aware remediation program. Each change states the **fix**, the
**expected gain**, the **edge cases**, and the **mitigation**. Nothing here changes
behaviour silently — every item is backward-compatible or has an explicit migration.

## Documents

- [`001-quick-wins.md`](001-quick-wins.md) — low-risk, high-leverage changes (days).
- [`002-structural-changes.md`](002-structural-changes.md) — batched writes, transactions, traversal bounds (weeks).
- [`003-migration-plan.md`](003-migration-plan.md) — data migration for existing deployments, zero-/low-downtime.
- [`004-edge-cases-and-mitigations.md`](004-edge-cases-and-mitigations.md) — exhaustive edge-case register for every proposed change.
- [`005-security-hardening.md`](005-security-hardening.md) — injection surface, isolation, secrets, DoS.

## Prioritization (impact × inverse-risk)

| Rank | Change                                   | Finding | Effort | Gain                | Risk            |
| ---- | ---------------------------------------- | ------- | ------ | ------------------- | --------------- |
| 1    | Amortize AGE session to `after_connect`  | F2      | S      | ~3× fewer graph RT  | low             |
| 2    | Batch vector upsert (multi-row)          | F1      | S      | C→1 RT              | low             |
| 3    | Per-query `ef_search` + `iterative_scan` | F6, F7  | S      | recall restored     | low             |
| 4    | Batch graph writes via `UNWIND`          | F3      | M      | ~100× fewer RT      | med             |
| 5    | Wrap document writes in a transaction    | F4      | M      | atomicity           | med             |
| 6    | Move chunk text out of vector metadata   | F5      | M      | ~50% heap, GIN cost | med (migration) |
| 7    | Bound `get_neighbors` traversal          | F9      | S      | DoS-safe traversal  | low             |
| 8    | Parameterize/validate Cypher inputs      | F8      | M      | injection defense   | med             |
| 9    | Concurrent `insert_batch`                | F10     | S      | pipeline overlap    | low             |
| 10   | Raise `max_connections` default          | F11     | XS     | less contention     | low             |

> Sequencing rationale: ranks 1–3 are pure code, no data migration, immediately
> shippable. Rank 6 needs a backfill (see migration plan). Ranks 4–5 are best landed
> together (the `UNWIND` rewrite naturally introduces the transaction boundary).

## Guiding constraints

- **No silent recall/behaviour change.** Tuning knobs are opt-in or conservative
  defaults that only *raise* recall.
- **Backward compatibility with legacy rows.** All filters already use column-first +
  JSONB-fallback; migrations preserve that.
- **Idempotent migrations.** Every DDL uses `IF NOT EXISTS` / `IF EXISTS` and is safe to
  re-run, matching the existing migration style (027–036).
