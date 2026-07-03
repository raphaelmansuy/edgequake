# SPEC-041 — Upgrade Path Matrix (All Versions)

**First principle:** Every deployed EdgeQuake must reach a running backend with correct child `"Node"` indexes — regardless of starting version, AGE state, or whether M078 previously failed or skipped.

---

## Three repair layers (defense in depth)

| Layer | When | Mechanism | Population |
| ----- | ---- | --------- | ------------ |
| **L1 Pre-sqlx** | Before `MIGRATOR.run()` | `repair_migration_078_checksum_if_needed()` | v0.13.2 skip-path (v78 recorded, old checksum) |
| **L2 sqlx** | During migrate | Fixed M078 + M079 idempotent CREATE | Fresh upgrade, blocked retry, skip-path + M079 pending |
| **L3 Post-bootstrap** | After sqlx | `reconcile_migration_078()` → `support/078/apply.sql` | v78 recorded but indexes missing (graphs added later) |

**DRY SSOT:** `migrations/support/078/apply.sql` — bootstrap reconcile; M078/M079 sqlx files stay aligned.

---

## Upgrade path by starting version

| From | DB state | Outcome on v0.13.3+ |
| ---- | -------- | ------------------- |
| **≤ v0.13.1** | No v78 | M078 applies (fixed) → M079 no-op → L3 skip |
| **v0.13.2 blocked** | v78 **not** recorded | M078 applies → M079 no-op → indexes ✅ |
| **v0.13.2 skip-path** | v78 recorded, **old checksum** | L1 checksum repair → M079 applies if pending → L3 if indexes missing ✅ |
| **v0.13.2 skip-path + graphs added later** | v78 recorded, no Node indexes | L1 repair → M079 → L3 creates indexes ✅ |
| **v0.13.3+ fresh** | Clean install | M078 + M079 idempotent ✅ |
| **Any version, no AGE** | Extension absent | All layers no-op; server starts ✅ |
| **Any version, AGE, no graphs** | Empty `ag_graph` | Loop no-op; server starts ✅ |
| **Graph without Node label** | No `"Node"` rel | CONTINUE per graph ✅ |
| **Graph with Node, indexes from `graph_lifecycle`** | IF NOT EXISTS skip | No duplicate indexes ✅ |
| **Graph with Node, no EDGE** | Node indexes only | Edge index branch skipped ✅ |

---

## Edge cases

| EC | Scenario | Handler |
| -- | -------- | ------- |
| EC-01 | Invalid `->>>` in v0.13.2 M078 | Fixed file; blocked installs retry |
| EC-02 | Checksum mismatch on upgrade | L1 automatic repair (no manual script required) |
| EC-03 | Manual script still available | `e2e/repair_migration_078_checksum.sh` for ops |
| EC-04 | sqlx-only `migrate run` (no bootstrap) | M079 ensures indexes |
| EC-05 | Bootstrap-only retry after partial failure | L3 audit + apply |
| EC-06 | Concurrent ops on large graph | `support/078/concurrent.sql` (fixed `->>`) |
| EC-07 | Rollback | DROP INDEX only; no data loss |

---

## What we explicitly do NOT fix here (SPEC-041-B)

| Issue | Population | Track |
| ----- | ---------- | ----- |
| M071 HNSW dim > 2000 | Orphan vector tables | Future spec |
| Runtime DDL swallows HNSW errors | Silent no-index | Future spec |

---

## Verification

```bash
./specs/041-fix-migration/e2e/run_all.sh
./specs/041-fix-migration/e2e/simulate_upgrade_paths.sh
cargo test -p edgequake-api migration_bootstrap --features postgres --lib
DATABASE_URL=... ./scripts/test_migration_e2e.sh
```
