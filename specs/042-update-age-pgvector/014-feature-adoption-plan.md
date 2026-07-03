# SPEC-042-E — Feature Adoption Plan (Phase E)

**Date:** 2026-07-03  
**Status:** **Planned** — follows Phase A–D (pins, triple-track, battle test)  
**Parent:** [000-index.md](./000-index.md) · [013-version-feature-matrix-official-docs.md](./013-version-feature-matrix-official-docs.md)

Phase E adopts four upstream capabilities that are **available today** on pinned extension/server versions but **not yet wired** in EdgeQuake application code.

---

## Summary

| ID | Feature | Min requirement | Disk / perf impact | Priority |
| -- | ------- | ----------------- | ------------------ | -------- |
| **E-01** | halfvec storage | pgvector ≥ 0.8.0 (all tiers) | ~50% vector storage | P1 |
| **E-02** | AGE 1.7 RLS tenant isolation | AGE ≥ 1.7.0, PG17+ | Security hardening | P1 |
| **E-03** | `uuidv7()` document IDs | PostgreSQL 18 | Time-ordered B-tree inserts | P2 |
| **E-04** | AGE pg COPY bulk loader | AGE ≥ 1.7.0, PG17+ | Faster graph bulk ingest | P2 |

**Invariant (unchanged from Phase C):** PG16 tier must keep working without E-02/E-03/E-04; E-01 is pgvector-only and applies to all tiers once migrated.

---

## E-01 — halfvec storage (~50% disk savings)

### Official source

- [pgvector README — Half-Precision Vectors](https://github.com/pgvector/pgvector#half-precision-vectors)
- [pgvector README — Half-Precision Indexing](https://github.com/pgvector/pgvector#half-precision-indexing)

> Use the `halfvec` type to store half-precision vectors… 2 bytes per dimension instead of 4.

### Current state (code is law)

| Location | Today |
| -------- | ----- |
| `vector/ddl.rs` | `embedding vector({dim})` + HNSW `vector_cosine_ops` |
| `vector/storage_impl.rs` | `format_embedding` → full `f32` text |
| M042 apply | REINDEX on `eq_%_vectors` HNSW/IVFFlat |

### Target state

```sql
-- Column (migration M047)
embedding halfvec(1536) NOT NULL

-- Index (per pgvector docs)
CREATE INDEX ... USING hnsw (embedding halfvec_cosine_ops);
```

Query path unchanged: cast at search time if needed, or store/query natively as `halfvec`.

### Implementation steps

| Step | Owner | Action |
| ---- | ----- | ------ |
| E-01.1 | Storage | Add `VectorStorageMode { Full, Half }` config; default `Full` until opt-in |
| E-01.2 | Migration | `M047_halfvec_embeddings.sql` + `support/047/apply.sql` — `ALTER COLUMN embedding TYPE halfvec(n) USING embedding::halfvec(n)` per workspace table |
| E-01.3 | DDL | `ddl.rs` branch on mode; index opclass `halfvec_cosine_ops` |
| E-01.4 | M042 | Extend reindex loop to detect `halfvec` indexes |
| E-01.5 | Quality | Recall benchmark: compare top-k overlap `vector` vs `halfvec` on fixture corpus (target ≥95% overlap @ k=10) |
| E-01.6 | Ops | Env `EDGEQUAKE_VECTOR_STORAGE=halfvec\|full` (default `full`) |
| E-01.7 | E2E | `BT-PV-04` — halfvec HNSW insert + filtered ANN in battle test |

### Tier matrix

| Tier | Supported |
| ---- | --------- |
| PG16 / PG17 / PG18 | ✅ (pgvector 0.8.3 on all images) |

### Acceptance criteria

- [ ] Workspace with 1M vectors uses ≤55% previous disk on `embedding` + HNSW index
- [ ] Recall@10 ≥ 95% vs full `vector` baseline on standard test set
- [ ] `/health` reports `vector_storage_mode: halfvec|full`
- [ ] Rollback: `EDGEQUAKE_VECTOR_STORAGE=full` + M047 down migration documented

### Risks

| Risk | Mitigation |
| ---- | ---------- |
| Recall loss on small embeddings (384-dim) | Benchmark per dimension; allow per-workspace override |
| Online migration lock time | Batch `ALTER TABLE` per workspace off-hours; maintenance window flag |

---

## E-02 — AGE 1.7 RLS for tenant isolation

### Official source

- [AGE 1.7.0 release — Add RLS support (#2309)](https://github.com/apache/age/releases/tag/PG17/v1.7.0-rc0)
- [PostgreSQL RLS docs](https://www.postgresql.org/docs/current/ddl-rowsecurity.html)

### Current state

| Location | Today |
| -------- | ----- |
| Graph naming | Workspace-scoped graph per tenant (`ws_{id}` pattern) |
| `init.sql` | `edgequake_admin NOLOGIN BYPASSRLS` role stub (line 45) |
| Isolation | Application middleware `TenantContext` — no DB-enforced RLS on AGE label tables |

### Target state

On **PG17/PG18 + AGE ≥ 1.7.0**:

```sql
ALTER TABLE {graph_schema}._ag_label_vertex ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON {graph_schema}._ag_label_vertex
  USING (properties->>'tenant_id' = current_setting('edgequake.tenant_id', true));
-- Repeat for edge labels; service role sets edgequake.tenant_id per transaction
```

Application sets `SET LOCAL edgequake.tenant_id = $1` at start of each graph transaction.

### Implementation steps

| Step | Owner | Action |
| ---- | ----- | ------ |
| E-02.1 | Security design | Document policy matrix: tenant_id, workspace_id, admin bypass |
| E-02.2 | Bootstrap | `M048_age_rls.sql` + `support/048/apply.sql` — idempotent ENABLE RLS + policies |
| E-02.3 | Gate | `helpers::age_supports_rls()` → `extension_version_at_least(age, "1.7.0")` |
| E-02.4 | Storage | `connection.rs` / graph adapter: `set_config('edgequake.tenant_id', ...)` per request |
| E-02.5 | Entities | Ensure `tenant_id` written on all vertex/edge properties at ingest |
| E-02.6 | PG16 path | No-op when gate false; log `age_rls_enabled: false` on `/health` |
| E-02.7 | E2E | Cross-tenant read attempt must return 0 rows on PG17+ with RLS on |

### Tier matrix

| Tier | RLS |
| ---- | --- |
| PG16 (AGE 1.6.0) | ❌ App-level isolation only |
| PG17 / PG18 (AGE 1.7.0) | ✅ When `EDGEQUAKE_AGE_RLS=true` |

### Acceptance criteria

- [ ] Tenant A cannot MATCH vertices belonging to Tenant B (PG17 E2E)
- [ ] `edgequake_admin` BYPASSRLS can run maintenance/migrations
- [ ] PG16 regression: identical behaviour with RLS gate off
- [ ] `/health` → `age_rls_enabled: bool`

### Risks

| Risk | Mitigation |
| ---- | ---------- |
| Missing tenant_id on legacy vertices | Backfill migration before ENABLE RLS |
| Performance overhead | Benchmark VLE with/without policies; index on `(properties->>'tenant_id')` |

---

## E-03 — `uuidv7()` for document IDs (PG18)

### Official source

- [PostgreSQL 18 release — uuidv7()](https://www.postgresql.org/docs/release/18.0/) (§ E.5.3.4)
- [RFC 9562 UUIDv7](https://www.rfc-editor.org/rfc/rfc9562.html)

> `uuidv7()` function for generating timestamp-ordered UUIDs.

Benefit: monotonic insert order into B-tree primary keys on `documents` / `pdf_documents` — reduces page splits vs random UUIDv4.

### Current state

| Location | Today |
| -------- | ----- |
| `ingest_admission.rs` | `Uuid::new_v4().to_string()` |
| `document_admission.rs` | `Uuid::new_v4().to_string()` |
| `scan.rs` | `Uuid::new_v4().to_string()` |
| `init.sql` / migrations | `DEFAULT gen_random_uuid()` (v4) |

Battle test confirms `uuidv7()` exists on PG18 (`BT-PG-18-01`).

### Target state

```rust
// helpers/id_allocation.rs (new SSOT)
pub async fn allocate_document_id(pool: &PgPool) -> String {
    if postgres_supports_uuidv7(pool).await {
        // SELECT uuidv7()::text
    } else {
        Uuid::new_v4().to_string()
    }
}
```

SQL migrations for **new** PG18 deployments:

```sql
-- PG18 only branch in migration or init
id UUID PRIMARY KEY DEFAULT uuidv7()
```

Existing rows keep v4 IDs — no rewrite required.

### Implementation steps

| Step | Owner | Action |
| ---- | ----- | ------ |
| E-03.1 | Core | `postgres_supports_uuidv7()` — probe `SELECT uuidv7()` once at pool init |
| E-03.2 | API | Replace direct `Uuid::new_v4()` in ingest paths with `allocate_document_id()` |
| E-03.3 | Migration | Conditional migration: new PG18 installs use `DEFAULT uuidv7()`; document existing v4 |
| E-03.4 | Health | `/health` → `document_id_generator: uuidv7|uuidv4` |
| E-03.5 | E2E | PG18 ingest → verify ID is UUIDv7 (timestamp bits monotonic) |

### Tier matrix

| Tier | ID generator |
| ---- | ------------ |
| PG16 / PG17 | `Uuid::new_v4()` (unchanged) |
| PG18 | `uuidv7()` when PG18 detected |

### Acceptance criteria

- [ ] New documents on PG18 use v7 (verifiable via RFC 9562 version nibble)
- [ ] PG16/PG17 unchanged
- [ ] No collision with existing v4 document IDs

### Risks

| Risk | Mitigation |
| ---- | ---------- |
| Managed PG17 hosts without uuidv7 | Gate strictly on PG major ≥ 18, not extension |
| String IDs in KV vs UUID type in PG | Keep string format; v7 returned as text |

---

## E-04 — AGE pg COPY bulk loader

### Official source

- [AGE 1.7.0 — Replace libcsv with pg COPY (#2310)](https://github.com/apache/age/releases/tag/PG17/v1.7.0-rc0)

> Replace libcsv with pg COPY for csv loading.

AGE 1.7 routes CSV graph loads through native PostgreSQL `COPY`, improving throughput and memory vs libcsv.

### Current state

| Location | Today |
| -------- | ----- |
| Graph ingest | Row-by-row Cypher MERGE via pipeline entity extraction |
| `bulk_ops/rebuild_knowledge_graph.rs` | Re-processes documents — no COPY fast path |
| AGE loader | `age_load` / CSV paths exist upstream; not used by EdgeQuake |

### Target state

For **bulk rebuild** and **large batch entity ingest** on PG17/PG18:

1. Export entities/edges to staging CSV (vertex CSV + edge CSV per AGE format).
2. Call AGE load function using pg COPY backend (1.7.0+).
3. Fall back to Cypher MERGE when `age < 1.7.0` or row count below threshold.

### Implementation steps

| Step | Owner | Action |
| ---- | ----- | ------ |
| E-04.1 | Research | Confirm AGE 1.7 CSV format + `load_graph_from_csv` / `age_load` API from upstream docs |
| E-04.2 | Pipeline | `BulkGraphLoader` trait: `CypherMergeLoader` (default) + `AgeCopyLoader` (1.7+) |
| E-04.3 | Gate | `helpers::age_supports_copy_loader()` → `extversion >= 1.7.0` |
| E-04.4 | Threshold | Env `EDGEQUAKE_BULK_COPY_MIN_ROWS=1000` — below threshold use MERGE |
| E-04.5 | Integration | Wire into `rebuild_knowledge_graph` bulk op |
| E-04.6 | E2E | Load 10k vertices via COPY on PG17; compare count + sample MATCH vs MERGE path |

### Tier matrix

| Tier | Bulk loader |
| ---- | ----------- |
| PG16 (AGE 1.6.0) | Cypher MERGE only |
| PG17 / PG18 (AGE 1.7.0) | COPY when row count ≥ threshold |

### Acceptance criteria

- [ ] 10k entity ingest ≥3× faster on PG17 vs Cypher-only baseline (benchmark recorded in spec)
- [ ] Graph integrity: edge counts match MERGE path on same input
- [ ] PG16 behaviour unchanged
- [ ] `/health` → `age_copy_loader_enabled: bool`

### Risks

| Risk | Mitigation |
| ---- | ---------- |
| CSV format drift between AGE versions | Pin AGE 1.7.0; integration test on upgrade |
| Temp disk for staging CSV | Stream to `COPY FROM STDIN`; cleanup in `finally` |

---

## Phase E schedule (recommended order)

```
E-03 uuidv7 (PG18, low effort, quick win)
    ↓
E-01 halfvec (all tiers, highest storage ROI)
    ↓
E-04 AGE COPY loader (PG17+, ingest perf)
    ↓
E-02 AGE RLS (PG17+, security — needs entity backfill first)
```

**Rationale:** uuidv7 is isolated and PG18-only. halfvec benefits all deployments. COPY loader improves ingest before RLS adds policy complexity. RLS last because it requires `tenant_id` on all graph properties.

---

## Requirements traceability (REQ-042E)

| ID | Requirement | Verification |
| -- | ----------- | ------------ |
| REQ-042E-01 | halfvec opt-in with recall gate | E-01.5 benchmark + BT-PV-04 |
| REQ-042E-02 | AGE RLS on PG17+ when enabled | E-02.7 cross-tenant E2E |
| REQ-042E-03 | uuidv7 on PG18 new documents | E-03.5 E2E |
| REQ-042E-04 | COPY bulk loader on PG17+ above threshold | E-04.6 benchmark |
| REQ-042E-05 | PG16 tier unaffected by E-02/E-03/E-04 | Triple-tier regression |
| REQ-042E-06 | All gates use `extension_version_at_least` / PG major probe | Code review + battle test |

---

## Cross-reference

| Doc | Link |
| --- | ---- |
| Feature matrix | [013-version-feature-matrix-official-docs.md](./013-version-feature-matrix-official-docs.md) |
| Implementation phases A–D | [008-implementation-plan.md](./008-implementation-plan.md) |
| Triple-track policy | [012-dual-pg-major-compatibility.md](./012-dual-pg-major-compatibility.md) |
| Battle test harness | [e2e/run_version_feature_battle_test.sh](./e2e/run_version_feature_battle_test.sh) |
