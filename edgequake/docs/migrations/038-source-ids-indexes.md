# Migration 038 — source_ids / source_id Indexes

**Migration ID:** `038`  
**Risk:** Low (indexes only, no data mutation)  
**Spec:** [SPEC-006](../../../specifications/006-ensure-perf/000-index.md)

---

## Why This Migration Exists

Document-scoped graph operations (delete cascade, lineage, relationship lookup) filter AGE vertices and edges by `source_id` (legacy pipe format) and `source_ids` (jsonb array). Without indexes, large workspaces degrade to sequential scans → query timeouts and memory pressure (production exit code **137** class).

**First principle:** bounded ops must push filters to SQL; indexes make push-down fast at scale.

---

## Package Layout

```
edgequake/migrations/
├── 038_add_source_ids_gin_indexes.sql    # sqlx auto-apply
└── support/038/
    ├── preflight.sql                     # read-only checks
    ├── concurrent.sql                    # CREATE INDEX CONCURRENTLY
    ├── rollback.sql                      # DROP INDEX IF EXISTS
    └── verify.sql                        # post-apply gate

edgequake/scripts/migrations/
└── apply_038.sh                          # canonical ops wrapper
```

---

## Rollout Procedure

### 1. Pre-flight (required)

```bash
export DATABASE_URL="postgres://user:pass@host:5432/edgequake"
./edgequake/scripts/migrations/apply_038.sh --dry-run
```

Review NOTICE output:
- Is Apache AGE installed?
- Per-graph vertex/edge row counts
- **WARNING** if any graph >500k vertices → use `--concurrent`

### 2. Apply

**Normal graphs (<500k vertices):**

```bash
./edgequake/scripts/migrations/apply_038.sh --apply --yes
```

**Large production graphs:**

```bash
./edgequake/scripts/migrations/apply_038.sh --apply --concurrent --yes
```

### 3. Verify

```bash
./edgequake/scripts/migrations/apply_038.sh --verify
```

Or manual:

```sql
SELECT schemaname, indexname
FROM pg_indexes
WHERE indexname LIKE '%source_ids%' OR indexname LIKE '%vertex_source_id%';
```

### 4. Rollback (indexes only)

```bash
./edgequake/scripts/migrations/apply_038.sh --rollback --yes
```

No data loss. Prefix queries remain correct but slower.

---

## Compatibility Matrix

| Environment | How 038 is applied |
|-------------|-------------------|
| Fresh install (`make dev`) | sqlx on backend start |
| Existing prod (small graph) | `apply_038.sh --apply` |
| Existing prod (large graph) | `apply_038.sh --apply --concurrent` |
| No AGE extension | No-op with NOTICE (safe) |
| Partial graph deploy | Skips missing `_ag_label_*` tables |
| Re-run | Idempotent (`IF NOT EXISTS` / `IF EXISTS`) |

---

## FAQ

### Does sqlx already apply this on server restart?

**Yes** — `migration_bootstrap` runs sqlx migrations on startup, then verifies migration 038 indexes per graph with progression logs (`RUST_LOG=edgequake.migration=info`). Small graphs missing indexes are repaired inline; large graphs (≥500k vertices) defer to `apply_038.sh --concurrent` and `/health` reports `degraded`.

Use `apply_038.sh` when you need preflight, CONCURRENTLY rebuild, explicit verify, or rollback **without** restarting the API.

### Do I need `--concurrent` on a fresh install?

**No.** Standard apply is fine. Use CONCURRENTLY only for large **existing** graphs where index build time or write locks matter.

### What if migration 038 already ran via sqlx?

Run `--verify`. If indexes exist, you are done. `--apply` is idempotent (`CREATE INDEX IF NOT EXISTS`).

### Will rollback delete my entities or documents?

**No.** Rollback drops indexes only. All graph and relational data is unchanged.

### What about legacy `source_id` pipe format (`doc-chunk-0|other`)?

Application code handles both `source_id` (pipe-split) and `source_ids` (array). Migration 038 adds:
- btree on `source_id` text extraction
- GIN on `source_ids` jsonb

Partial document delete clears legacy `source_id` when updating `source_ids` (see SPEC-006 P4 edge-case tests).

### Multiple AGE graphs / workspaces?

Migration loops all graphs in `ag_catalog.ag_graph` and creates per-graph indexes with unique names (`idx_{graph}_vertex_source_ids_gin`, etc.).

### Index build failed mid-way — is the DB broken?

Per-index `EXCEPTION` handlers in the standard migration isolate failures. Re-run `--apply` or fix the specific graph, then `--verify`.

### How do I know indexes are used?

```sql
EXPLAIN (ANALYZE, BUFFERS)
SELECT * FROM my_graph."_ag_label_vertex"
WHERE ag_catalog.agtype_to_json(properties)->>'source_id' LIKE 'doc-uuid%'
LIMIT 100;
```

Anchored prefix queries (`doc%`) benefit most. Mid-string `LIKE '%|doc%'` may not use btree efficiently.

### What if I see exit 137 after deploying?

Indexes alone do not cap RAM. Also ensure:
- `make resource-proof` passes
- Docker `mem_limit` or K8s memory limits set
- No unguarded `detect_communities()` from API handlers

See [brutal assessment](../../../specifications/006-ensure-perf/010-brutal-assessment.md).

---

## Edge Cases Handled

| Edge case | Behavior |
|-----------|----------|
| AGE not installed | Pre-flight + apply no-op |
| Vertex table missing | Skip vertex indexes for that graph |
| Edge table missing | Skip edge indexes for that graph |
| Index already exists | `IF NOT EXISTS` — safe re-run |
| One index fails | Others still attempted (standard migration) |
| Graph >500k vertices | Pre-flight WARNING → use CONCURRENTLY |
| sqlx + manual apply | Idempotent; verify confirms state |
| Tenant with shared entities | App-level cascade (not migration scope) |

---

## Battle-Tested Verification

```bash
make resource-proof
```

Includes:
- `spec006_source_ids_migration.sh` — package integrity
- `resource_safety_*` integration tests
- `test_delete_preserves_shared_entities` E2E smoke

---

## Cross-References

- [migrations.md](../migrations.md) — general migration guide
- [runbook.md](../runbook.md) — OOM and performance tuning
- [011_migration_rollout.md](../../../specifications/006-ensure-perf/011_migration_rollout.md) — spec rollout doc
