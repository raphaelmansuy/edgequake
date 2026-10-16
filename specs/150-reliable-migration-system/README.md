# SPEC-150 — Reliable Migration System

> **Status:** Implemented (WP-1..WP-10) on HEAD through migration **169**.  
> **Proof:** PG16 replay of all **41** schema epochs → ledger **169**, **41/41 ok**, concurrent lock exit **75** (2026-10-15, `SKIP_SCHEMA_DIFF=1`).  
> **Epochs:** `scripts/spec150/epochs.toml` covers every published `vX.Y.Z` whose schema is at or below HEAD (**v0.34.0**, schema **169**, epoch `v0.33.0`). Later tags such as v0.35.0 (schema 206) are not upgrade sources for this binary. Enforced by `./scripts/check_epoch_coverage.sh`.  
> **Operator guide:** [`docs/operations/upgrading.md`](../../docs/operations/upgrading.md).  
> **Squash:** none — `T_fresh` ≈ 1–2s on PG16/17/18 (budget 180s).

## What operators need to know

| Goal | Do this |
|------|---------|
| Apply schema | `edgequake migrate` then (if dropping legacy) `edgequake migrate --confirm-drop` |
| Drain data jobs | `edgequake migrate drain` |
| Serve while migrate lags | Set `EDGEQUAKE_SCHEMA_GATE=wait` (Helm/compose default) — `/live` 200, `/ready` 503 |
| Refuse while behind | Default binary: `EDGEQUAKE_SCHEMA_GATE=fail` → exit **78** |
| Concurrent migrate | Second process exits **75** after `EDGEQUAKE_MIGRATE_LOCK_DEADLINE` |
| Unknown checksum | Exit **65** (fossils in `migrations/manifest.toml` auto-accept; `dev_only` never) |

Pragmatic short doc: [`edgequake/docs/migrations.md`](../../edgequake/docs/migrations.md).  
Day-2 ops: [11-ops-runbook.md](11-ops-runbook.md).

## Proof (honest bounds)

| Claim | What we actually ran |
|-------|----------------------|
| Epoch → HEAD | 2026-10-15: `FORCE_REPLAY=1 SKIP_SCHEMA_DIFF=1 PG=16` replayed every epoch in `epochs.toml`, then HEAD `migrate`. Each report has `ledger_max=169`. |
| PG majors | This pass is PG16 × all 41 epochs. PG17/18 reports were not refreshed (they still record ledger 168). |
| Equivalence | Schema-dump diff was not re-run (`SKIP_SCHEMA_DIFF=1`). |
| Chaos | Concurrent advisory-lock → exit **75** (re-run 2026-10-15). |
| `T_upgrade` | Wall-clock per matrix case (includes container start) — not pure SQL apply time. See [measurements/](measurements/). |

Artifacts: [reports/](reports/) (`*.json` + `SUMMARY.md`) · [measurements/](measurements/).

## One-screen architecture

```text
  edgequake migrate          edgequake serve
  ─────────────────          ────────────────
  fossil repair              gate evaluate
  advisory lock              wait → lite /live|/ready
  expand → drain → contract  OR fail → exit 78
  migration_run telemetry    never MIGRATOR.run
```

## Success criteria

| ID | Criterion | Evidence |
|----|-----------|----------|
| S1 | Published schema epochs upgrade to HEAD | [reports/pg16](reports/pg16) 41/41, ledger 169 |
| S2 | API never applies numbered migrations | `serve_boot.rs` + contract test |
| S3 | Pending schema does not crash-loop orchestrators | `SCHEMA_GATE=wait`; Helm/compose migrate |
| S4 | Known fossils apply without env | `manifest.toml`; unknown → 65 |
| S5 | Fresh DB under perf budget | [measurements/](measurements/) — no squash |
| S6 | Concurrent migrate deterministic | chaos lock → 75 |

## Reading order

1. **Operate now:** [migrations.md](../../edgequake/docs/migrations.md) · [11-ops-runbook.md](11-ops-runbook.md)
2. **Why / laws:** [00-why](00-why.md) · [01-first-principles](01-first-principles.md)
3. **Past / defects:** [02](02-incident-catalogue.md) · [03](03-release-schema-evolution.md) · [04](04-current-architecture.md) · [05](05-root-cause-analysis.md)
4. **Target / build:** [06](06-target-architecture.md) · [07](07-upgrade-path-matrix.md) · [08](08-implementation-plan.md) · [09](09-test-proof-protocol.md) · [10](10-performance-budget.md)
5. **Honesty:** [12-risks-honest-assessment.md](12-risks-honest-assessment.md) · [13-references.md](13-references.md)

## Non-goals

- Cross-major PostgreSQL `pg_upgrade` of the cluster itself.
- Closing SPEC-091 data-copy / `#396` guard RED (consumer of migrate lifecycle only).
- Publishing workspace crates to crates.io.
