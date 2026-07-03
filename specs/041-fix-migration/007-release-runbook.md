# SPEC-041 — Release Runbook

**Fixes:** [#273](https://github.com/raphaelmansuy/edgequake/issues/273)  
**Release:** v0.13.3 (recommended patch after v0.13.2)

---

## Pre-release checklist

- [ ] `./specs/041-fix-migration/e2e/run_all.sh` → exit 0
- [ ] `./scripts/check_migration_checksums.sh` → PASS
- [ ] Grep: `rg '->>>' edgequake/migrations/` → zero matches
- [ ] CHANGELOG entry under `### Fixed`

---

## Upgrade paths

### Path A — Blocked at M078 (AGE + Node graph) — most #273 reporters

**Symptom:** Backend logs `operator does not exist: json ->>> unknown`; v78 **not** in `_sqlx_migrations`.

**Action:** Deploy v0.13.3+. Fixed M078 applies automatically on next startup.

```bash
# Verify after deploy
psql "$DATABASE_URL" -c "SELECT version, success FROM _sqlx_migrations WHERE version = 78;"
psql "$DATABASE_URL" -c "SELECT pg_get_indexdef(indexrelid) FROM pg_indexes WHERE indexname = 'idx_node_workspace_id' LIMIT 1;"
```

---

### Path B — v0.13.2 applied M078 successfully (no Node table at upgrade time)

**Symptom:** After deploying v0.13.3, startup fails with:

```text
migration 78 was previously applied but has been modified
```

**Action:** Repair checksum **before** or **during** deploy:

```bash
./specs/041-fix-migration/e2e/repair_migration_078_checksum.sh "$DATABASE_URL"
# Then restart backend
```

Manual alternative:

```sql
-- New checksum from checksums.lock for 078_age_child_workspace_stats.sql
UPDATE _sqlx_migrations
SET checksum = decode('a043177271c82c65a7509855f1d64c02c46235343126a9bbb96c359f4c25aa35427c79bb50051d499b431d869eb8e930', 'hex')
WHERE version = 78;
```

Then restart backend. M078 body is idempotent — re-run creates missing indexes if graphs now exist.

---

### Path C — Large graph (>100k nodes)

1. Normal startup applies fixed M078 (inline CREATE INDEX)
2. If lock window too long, ops runs concurrent script:

```bash
psql "$DATABASE_URL" -f edgequake/migrations/support/078/concurrent.sql
```

---

## Production verification

```bash
./specs/040-edgequake-issues/e2e/measure_graph_stats_perf.sh "$DATABASE_URL"
./specs/041-fix-migration/e2e/run_all.sh
```

**Pass criteria:** Workspace filter query << 15s; indexdef contains `->> 'workspace_id'`.

---

## Rollback

| Step | Command |
| ---- | ------- |
| Revert container image | Deploy previous tag (⚠️ re-blocks Path A users) |
| Drop indexes only | `DROP INDEX IF EXISTS {graph}.idx_node_workspace_id` etc. |

No data loss on index rollback.

---

## Close issue #273

Comment template:

```markdown
Fixed in v0.13.3 — M078 `->>>` → `->>` (4 locations).

Evidence: specs/041-fix-migration/e2e/evidence/run_all_summary.txt

If you upgraded to v0.13.2 on a DB without Node tables and see checksum mismatch,
run: specs/041-fix-migration/e2e/repair_migration_078_checksum.sh
```
