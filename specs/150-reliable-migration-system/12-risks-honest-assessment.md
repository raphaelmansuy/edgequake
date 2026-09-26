# 12 — Risks and honest assessment

Parent: [README](README.md) · Incidents: [02](02-incident-catalogue.md) · Plan: [08](08-implementation-plan.md)

## What this pack does **not** oversell

We can make **schema apply** from any published epoch **deterministic** (fossils, lock bounds, migrate-not-serve, Helm/compose order, realistic CI). We cannot, in this spec, promise that the SPEC-091 **data-copy engine** is finished. [#396](https://github.com/raphaelmansuy/edgequake/issues/396) is still OPEN: `guard` RED on v0.26.5, cursor can advance after 21000/23505 skip, W3 parent-chunk gaps, CLI/API skew.

[#405](https://github.com/raphaelmansuy/edgequake/issues/405) (FTS on dropped relation) is a **stale code path**, not a migrator bug. WP-4/WP-5 will not close it.

## Residual risks (post-implementation)

| Risk | Why it remains | Mitigation |
|------|----------------|------------|
| Partner dumps unlike fixtures | SPEC-110 already admitted this | Shape-based seeds; still not their data |
| AGE catalog / graph name drift | Graphs created at runtime per workspace | Census in runner; 038-size class |
| HNSW rebuild hours | pgvector SHARE build | Maintenance window; budget XL |
| Inner COMMIT already in 128–144 | Cannot edit shipped files (LAW-150-4) | Lint **new** files (159+); live with historical risk |
| Epoch proof used replay, not every GHCR boot | Image mode optional; pull may fall back | Replay matches ledger checksums; schema-diff catches drift |
| sqlx 0.9 later | Breaking API | Stay 0.8.x through this cut |
| SPEC-091 data-copy / #396 | Guard can still be RED | Out of schedule for 150; migrate lifecycle only hosts drain |

## Non-goals

- PostgreSQL major `pg_upgrade` (cluster dump/restore is the supported PG jump).
- Rewriting Apache AGE on-disk format.
- Multi-region online DDL orchestration.
- Making `--confirm-drop` safe when `guard` is RED.

## Open issues this spec tracks but does not close

- #396 vector-copy / guard RED — SPEC-091 engine follow-up.
- #405 FTS 42P01 — product query path.

## Done bar (local attestation 2026-09-26)

- [x] S1–S5 via epoch matrix + measurements (see [README](README.md) honesty table).
- [x] Concurrent lock → exit 75.
- [x] Helm template / compose migrate wiring.
- [ ] Full nightly CI shard matrix on every push (workflow added; first green run is post-merge).
- [ ] Every chaos row in [09](09-test-proof-protocol.md) (kill-9, dirty message, fossil-only cases) — code present; re-run in CI as needed.
