# E2E Proof 014 — P4 Migration Production Safety

**Requirement:** TR-006-006, NFR-006-004  
**Layer:** SQL + shell gates  
**Status:** ✅ Verified 2026-06-06

---

## Claim

Migration 038 can be applied safely on current production without data loss, with pre-flight, rollback, and concurrent variants for large graphs.

---

## Evidence

### Static package gate

```bash
./scripts/spec006_source_ids_migration.sh
```

Validates:
- sqlx migration + `support/038/` package (preflight, concurrent, rollback, verify)
- Main migration defines GIN/btree indexes
- Concurrent variant uses `CREATE INDEX CONCURRENTLY`
- Rollback uses `DROP INDEX IF EXISTS`
- `edgequake/scripts/migrations/apply_038.sh` exists

### Apply wrapper modes

```bash
./edgequake/scripts/migrations/apply_038.sh --dry-run
./edgequake/scripts/migrations/apply_038.sh --apply --yes
./edgequake/scripts/migrations/apply_038.sh --apply --concurrent --yes
./edgequake/scripts/migrations/apply_038.sh --verify
./edgequake/scripts/migrations/apply_038.sh --rollback --yes
```

### Production safety properties (code review)

| Property | Implementation |
|----------|----------------|
| Idempotent | `IF NOT EXISTS` / `IF EXISTS` |
| No data mutation | Indexes only |
| Partial deploy safe | `to_regclass` skip missing tables |
| Per-index fault isolation | `EXCEPTION WHEN OTHERS` in DO block |
| Large graph path | CONCURRENTLY script |

---

## Regression

Included in `make resource-proof` via `spec006_source_ids_migration.sh`.
