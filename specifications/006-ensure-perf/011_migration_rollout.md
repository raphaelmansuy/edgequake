# SPEC-006 — Migration 038 Production Rollout

**Spec ID:** `006-ensure-perf` P4  
**Migration:** `038_add_source_ids_gin_indexes.sql`  
**Risk:** Low (indexes only, no data mutation)

---

## Package Contents

| File | Purpose |
|------|---------|
| `support/038/preflight.sql` | Read-only checks (AGE, row counts, warnings) |
| `038_add_source_ids_gin_indexes.sql` | sqlx migration + standard apply |
| `support/038/concurrent.sql` | Zero-downtime for graphs >500k vertices |
| `support/038/rollback.sql` | Drop indexes only (no data loss) |
| `support/038/verify.sql` | Post-apply verification |
| `edgequake/scripts/migrations/apply_038.sh` | Canonical ops wrapper |
| `edgequake/docs/migrations/038-source-ids-indexes.md` | FAQ + edge cases |

---

## Rollout Procedure

### 1. Pre-flight (required)

```bash
export DATABASE_URL="postgres://..."
./edgequake/scripts/migrations/apply_038.sh --dry-run
```

Review NOTICE output:
- AGE installed?
- Per-graph vertex/edge counts
- WARNING if >500k vertices → use `--concurrent`

### 2. Apply (normal graphs)

```bash
./edgequake/scripts/migrations/apply_038.sh --apply --yes
```

Idempotent: safe to re-run. Skips missing AGE tables.

### 3. Apply (large production graphs)

```bash
./edgequake/scripts/migrations/apply_038.sh --apply --yes --concurrent
```

Runs `CREATE INDEX CONCURRENTLY` outside transaction. Schedule low-traffic window.

### 4. Rollback (if needed)

```bash
./edgequake/scripts/migrations/apply_038.sh --rollback --yes
```

Only drops indexes. Prefix queries remain correct but slower.

---

## Compatibility Matrix

| Environment | Script | Notes |
|-------------|--------|-------|
| Fresh install | `038_add_source_ids_gin_indexes.sql` | Via normal migration runner |
| Existing prod (small) | `--apply` | ~seconds per graph |
| Existing prod (large) | `--apply --concurrent` | Minutes per index, no write lock |
| No AGE extension | Any | No-op with NOTICE |
| Partial graph deploy | Any | Skips missing `_ag_label_*` tables |

---

## Verification

```bash
make resource-proof   # static gate validates package
# Post-apply (manual):
psql "$DATABASE_URL" -c "SELECT indexname FROM pg_indexes WHERE indexname LIKE '%source_ids%';"
```

---

## Cross-refs

- E2E proof: [014-p4-migration-production-safe-proof.md](e2e/014-p4-migration-production-safe-proof.md)
- Operator runbook: [009_operator_runbook.md](009_operator_runbook.md)
