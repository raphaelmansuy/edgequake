# SPEC-042-B — PostgreSQL Major Upgrade Path Matrix (PG16 / PG17 / PG18)

**First principle:** Every EdgeQuake deployment must reach a running backend with **pgvector ≥ 0.8.3**, **AGE ≥ tier minimum** (1.6.0 on PG16; 1.7.0 on PG17/PG18), and intact graph + vector data — regardless of starting PG major.

---

## Three migration layers (defense in depth)

| Layer | When | Mechanism | Population |
| ----- | ---- | --------- | ---------- |
| **L0 Data move** | PG16 → PG18 cutover | `pg_dump` / `pg_restore` or `pg_upgrade` | All existing deployments |
| **L1 Extensions** | After restore | `CREATE EXTENSION` + M042/M043 apply.sql | Fresh PG18 volume |
| **L2 Bootstrap** | Backend startup | sqlx migrations + M038/M046/M078 reconcile | Every restart |

**DRY SSOT:**

- Data procedure: `scripts/migrate_postgres_major.sh`
- Extension apply: `migrations/support/042/apply.sql`, `migrations/support/043/apply.sql`
- Graph indexes: `migrations/support/078/apply.sql`

---

## Upgrade path by starting state

| From | DB state | Procedure | Outcome |
| ---- | -------- | --------- | ------- |
| **Fresh install** | Empty | PG18 image → `make dev` | CREATE EXTENSION → M042/M043 → ✅ |
| **Fresh install (PG17)** | Empty | PG17 image → `make dev` | AGE 1.7.0 + pgvector 0.8.3 ✅ |
| **PG16 dev volume** | Local docker | `migrate_postgres_major.sh` → PG17 or PG18 | Dump → restore → bootstrap ✅ |
| **PG16 prod RDS** | Managed PG16 | Dump → PG17/PG18 RDS → restore | Manual extension install on RDS if available |
| **PG16 + pgvector 0.7.4** | Stale catalog | Dump; M042 upgrades on target | 0.8.3 + REINDEX ✅ |
| **PG16 + AGE 1.6.0 graphs** | Active AGE data | Logical dump | PG17: may run 1.6→1.7 upgrade; PG18: fresh 1.7.0 ✅ |
| **PG16 + no AGE** | vector only | Standard dump/restore | M043 no-op ✅ |
| **PG17 → PG18** | Same AGE 1.7.0 | Dump/restore (PG major only) | Lighter than PG16 hop ✅ |
| **PG16 → PG17** | Incremental | Dump/restore to PG17 image | AGE 1.6→1.7 on restore ✅ |
| **Same major** | Extension bump only | N/A — M042/M043 idempotent | ✅ |

---

## Edge cases

| EC | Scenario | Handler |
| -- | -------- | ------- |
| EC-P18-01 | `pg_restore` extension already exists | Expected warning; continue |
| EC-P18-02 | AGE graphs missing after restore | Verify `CREATE EXTENSION age` + `LOAD 'age'` before restore |
| EC-P18-03 | `_sqlx_migrations` checksum mismatch | Do not re-run sqlx on restored DB; bootstrap reconciles |
| EC-P18-04 | M078 indexes missing post-restore | L2 `reconcile_migration_078()` on startup |
| EC-P18-05 | Large graph startup slow | `/ready` 503 until M078 completes; use maintenance window |
| EC-P18-06 | RDS without AGE package | **Blocker** — confirm extension availability before PG18 commitment |
| EC-P18-07 | pgvector REINDEX lock at startup | M042 per-index try/catch; ops off-hours |
| EC-P18-08 | Rollback needed | Restore `$DUMP_FILE` to PG16 — no in-place downgrade |
| EC-P18-09 | Dual-write during cutover | Forbidden — quiesce API before dump |
| EC-P18-10 | PG18 greenfield AGE (no 1.6→1.7 script) | Fresh 1.7.0 install — no long upgrade script |

---

## Path selection decision tree

```
Start
  │
  ├─ Greenfield? ──YES──► PG18 image + make dev
  │
  └─ NO (has PG16 data)
        │
        ├─ DB < 50 GB and window OK? ──YES──► pg_dump / pg_restore (default)
        │
        └─ NO (large + tight SLA) ──► pg_upgrade (ops playbook) + extension apply
```

---

## Verification matrix

| Check | Command | Pass |
| ----- | ------- | ---- |
| PG major | `SHOW server_version;` | 16.x / 17.x / 18.x per tier |
| pgvector | `SELECT extversion FROM pg_extension WHERE extname='vector';` | `≥ 0.8.3` |
| AGE | `SELECT extversion FROM pg_extension WHERE extname='age';` | `≥ 1.6.0` (PG16) or `≥ 1.7.0` (PG17/PG18) |
| sqlx | `SELECT max(version) FROM _sqlx_migrations;` | matches pre-migration |
| Graphs | `SELECT count(*) FROM ag_catalog.ag_graph;` | matches pre-migration |
| Health | `curl /health → operational.migration` | `ready_for_traffic: true` |
| E2E PG16 | `run_extension_upgrade_proof.sh` | exit 0 |
| E2E PG17/PG18 | `make postgres-image-build-pg17/pg18` | exit 0 |
| Pin gate | `check_extension_pins.sh all` | all profiles OK |

---

## What we explicitly defer (SPEC-042-C)

| Item | Reason |
| ---- | ------ |
| AGE 1.7 RLS adoption in EdgeQuake | Feature work — not migration blocker |
| pgvector halfvec / quantization | Separate perf spec |
| In-place PG16→18 without dump | Not supported by PostgreSQL |

---

## Verification commands

```bash
./scripts/migrate_postgres_major.sh --dry-run --source-url "$PG16_URL" --target-url "$PG17_URL"
./scripts/migrate_postgres_major.sh --dry-run --source-url "$PG16_URL" --target-url "$PG18_URL"
make postgres-image-build-pg17
make postgres-image-build-pg18
./scripts/check_extension_pins.sh all
cargo test -p edgequake-api migration_bootstrap_proof --features postgres
```
