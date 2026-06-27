# 007 — Postgres / AGE / pgvector Expert Lens

**Cross-ref:** [002 Ingestion](./002-ingestion-pipeline-audit.md) · [003 Query](./003-query-retrieval-audit.md) · [F-08](./README.md#cross-reference-matrix)

---

## Stack Topology (from code)

```text
  PostgreSQL (required — no memory fallback in server mode)
  │
  ├── pgvector extension          workspace-scoped vector tables
  │     ├── embedding vector(N)
  │     ├── metadata JSONB
  │     ├── document_id, tenant_id, workspace_id (denormalized)
  │     └── content_tsv TSVECTOR   (migration 045 — FTS)
  │
  ├── Apache AGE extension        graph per workspace
  │     └── Cypher via cypher_query_bound (parameterized)
  │
  ├── tasks table                 edgequake-tasks queue
  ├── documents / pdf_documents   relational dual-write
  └── KV tables                   document metadata, chunks, hashes
```

Docker: `Dockerfile.postgres`, `init-extensions.sql`, `verify-postgres-extensions.sh`  
Migrations: `042`–`045` upgrade markers with `support/*/apply.sql`

---

## pgvector Patterns

File: `edgequake-storage/src/adapters/postgres/vector/storage_impl.rs`

### Upsert (ingest)

```text
  INSERT INTO workspace_vectors
  SELECT * FROM UNNEST($ids, $embeddings, $metadata)
  ON CONFLICT (id) DO UPDATE
  ── batched 1000 rows per transaction
  ── dimension validated upfront
```

**Grade: A** — proper batching, not row-by-row.

### Query (retrieve)

```sql
  1 - (embedding <=> $query::vector) AS score
  ORDER BY embedding <=> $query
  LIMIT $k
```

Per-transaction GUC tuning (`search_tuning.rs`):
- `hnsw.ef_search`
- `ivfflat.probes`
- `hnsw.iterative_scan` for filtered queries

**Grade: A-** — production-aware; iterative scan for metadata filters is correct for filtered ANN.

### Indexes (DDL)

`vector/ddl.rs`:
- HNSW or IVFFlat on `embedding vector_cosine_ops`
- GIN on `metadata`, `content_tsv`

---

## FTS (migration 045)

File: `vector/fts.rs`

```sql
  content_tsv @@ websearch_to_tsquery('english', $1)
  ORDER BY ts_rank_cd(content_tsv, query) DESC
```

Generated column from `metadata->>'content'` — **same duplicated content problem as F-08**.

**Better design:** `content TEXT` column populated at upsert, `TSVECTOR` generated from `content`. Avoids JSON extraction at index time.

---

## Apache AGE Patterns

File: `graph/helpers/cypher_exec.rs`

- Parameterized bindings — **no string interpolation of user values** ✓
- Prepared statement support (`spec022_cypher_prepared_postgres.rs` tests)

File: `graph/nodes_ops.rs`, `edges_ops.rs`

- MERGE + per-key SET (AGE 1.6.0 workaround — no `ON CREATE SET`)
- Batch upserts for merger path

**Grade: B+** — AGE quirks handled; Cypher batching reduces round trips.

**Risk:** AGE version coupling — migrations `043_age_upgrade_marker.sql` acknowledge upgrade path.

---

## Community Labels on Graph

`community_persist.rs`:
- Louvain via `detect_communities_unchecked`
- Batch read existing nodes → merge `community_id` property → batch upsert

**Problem:** Called from **every ingest** (`ingestion_persister.rs`). On large graphs:
- Full graph read for Louvain
- Batch write all labeled nodes

**Postgres expert view:** This should be a **scheduled materialized view job** or incremental Leiden, not synchronous post-ingest hook.

---

## Workspace Isolation

`WorkspaceVectorRegistry` — per-workspace vector tables.

Query path resolves workspace embedding + vector table via API bootstrap.

**Failure mode:** Injection/orchestrator fallback to global storage when lookup fails — **cross-tenant data bleed risk** (P0 when misconfigured).

---

## Dual-Write Consistency

```text
  KV (chunks, metadata)     ──┐
                              ├── NOT transactional
  pgvector (embeddings)     ──┤
  AGE (graph)               ──┘
  documents table           ── best-effort async
```

Saga compensates vector+graph on merge failure. **KV orphans** remain possible.

**Postgres expert recommendation:** Single `ingest_id` correlation + outbox pattern for KV/documents sync, or move document metadata fully into Postgres and deprecate KV for documents.

---

## Migration Discipline

Markers `042`–`045` with support SQL — idempotent apply pattern in `migration_bootstrap.rs` (~1000 LOC).

**Strength:** Extension upgrades tracked explicitly.  
**Risk:** `migration_bootstrap.rs` size — bootstrap logic is becoming a god module (see 010).

---

## Postgres Expert Verdict

**Grade: B+**

Vector and graph adapters show **real Postgres expertise** — batch UNNEST, GUC tuning, GIN FTS, parameterized Cypher, workspace table isolation.

**Deductions:**
1. Content in JSONB metadata bloats rows (F-08)
2. Synchronous Louvain on ingest path
3. KV/Postgres split without transactional outbox
4. AGE version fragility (managed but present)

See [012-improvement-plan.md](./012-improvement-plan.md) Phase 2 (storage hardening).
