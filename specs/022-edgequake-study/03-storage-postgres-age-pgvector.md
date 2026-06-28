# 03 — Storage: PostgreSQL, Apache AGE, pgvector

> **Cross-ref**: [02-query](./02-query-pipeline-code-audit.md) · [01-ingestion](./01-ingestion-pipeline-code-audit.md) · [06-improvement-plan](./06-improvement-plan.md) P-H3  
> **Official docs**: see [README](./README.md#official-documentation-version-aligned)

---

## 1. Deployment topology

```
┌─────────────────────────────────────────────────────────────┐
│  postgres:16-bookworm (Dockerfile.postgres)                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │ pgvector    │  │ Apache AGE  │  │ uuid-ossp           │ │
│  │ v0.7.4      │  │ v1.6.0-rc0  │  │                     │ │
│  └──────┬──────┘  └──────┬──────┘  └─────────────────────┘ │
│         │                │                                   │
│         ▼                ▼                                   │
│  eq_*_vectors tables   ag_catalog graph (Cypher)              │
│  HNSW / IVFFlat idx    Node/Edge labels                       │
│  GIN metadata          MERGE upserts                          │
│  document_id cols      per-key SET (AGE 1.6 constraint)     │
└─────────────────────────────────────────────────────────────┘
         ▲                              ▲
         │ sqlx 0.8                     │
         └──────── edgequake-storage ───┘
```

Init script (`init-extensions.sql`) creates extensions; **tables come from Rust DDL/migrations**.

---

## 2. pgvector — version reality vs code ambition

### 2.1 Pinned version

```17:22:edgequake/docker/Dockerfile.postgres
    git clone --branch v0.7.4 https://github.com/pgvector/pgvector.git && \
    cd pgvector && \
    make OPTFLAGS="" && \
    make install && \
```

**Why OPTFLAGS=""**: Docker Desktop on macOS SIGILL on SIMD/SVE — documented inline.

### 2.2 Schema (DDL)

```20:56:edgequake/crates/edgequake-storage/src/adapters/postgres/vector/ddl.rs
            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                embedding vector({}) NOT NULL,
                metadata JSONB DEFAULT '{{}}',
                ...
            )
            ...
            USING hnsw (embedding vector_cosine_ops) WITH (m = {}, ef_construction = {})
            ...
            USING GIN (metadata jsonb_path_ops)
            ...
            document_id TEXT; tenant_id TEXT; workspace_id TEXT
```

**First principles**:

- Cosine distance via `<=>` operator — matches OpenAI embedding geometry when vectors normalized  
- Dual workspace columns (materialized + JSONB) — migration-safe deletes (`clear_workspace`)  
- O(1) `count()` via maintained stats table — avoids `COUNT(*)` on million-row tables  

### 2.3 Batch upsert (QW2) — ingestion-critical

Single transaction UNNEST for all vectors in batch:

- Dimension validation upfront (fail-fast)  
- Intra-batch dedup (last-write-wins) — prevents PostgreSQL ON CONFLICT error  
- **Atomic**: all chunk vectors for a document commit or none  

This is why **vectors-first saga ordering works**.

### 2.4 Query paths

| Method | Index use | Filter | Iterative scan |
|--------|-----------|--------|----------------|
| `query()` | HNSW/IVFFlat | optional id list | no |
| `query_filtered()` | HNSW/IVFFlat | SQL WHERE metadata | **if pgvector ≥0.8** |

Version gate:

```79:91:edgequake/crates/edgequake-storage/src/adapters/postgres/vector/search_tuning.rs
pub(crate) fn pgvector_supports_iterative_scan(version: &str) -> bool {
    ...
    Some(0) => minor >= 8,
```

**Brutal truth**: Production Docker ships **0.7.4**. Code is **forward-compatible** with 0.8.0 iterative scan, but **does not benefit today**. Filtered local/naive queries on large corpora may return **fewer than top_k** chunk results.

Official reference for iterative scan (requires upgrade):

> Starting with 0.8.0, enable `hnsw.iterative_scan` / `ivfflat.iterative_scan` — [PostgreSQL news: pgvector 0.8.0](https://www.postgresql.org/about/news/pgvector-080-released-2952/)

### 2.5 pgvector O(n) semantics

| Operation | Typical complexity | Notes |
|-----------|-------------------|-------|
| HNSW search | O(log N) approximate | ef_search tuned to 4× top_k, clamped [40,1000] |
| IVFFlat search | O(probes × lists) | probes = top_k clamped [10,200] |
| Batch upsert | O(B) single tx | B = batch size |
| Filtered search (0.7.4) | ANN then SQL filter | **Recall risk** — post-filter shrinkage |
| Filtered search (0.8.0+) | iterative ANN + filter | Code ready, infra not |

---

## 3. Apache AGE — version constraints drive upsert shape

### 3.1 Pinned version

```25:30:edgequake/docker/Dockerfile.postgres
    git clone --branch PG16/v1.6.0-rc0 https://github.com/apache/age.git && \
```

Release notes: operator support in Cypher, CREATE TABLE AS fixes — [PG16/v1.6.0-rc0](https://github.com/apache/age/releases/tag/PG16%2Fv1.6.0-rc0)

### 3.2 Cypher invocation (official format)

Per [AGE Cypher manual](https://age.apache.org/age-manual/master/intro/cypher.html):

```sql
SELECT * FROM cypher('graph_name', $$ ... $$) AS (col agtype);
```

EdgeQuake wraps with `agtype_to_json` for Rust parsing:

```33:37:edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers/cypher_exec.rs
        let sql = format!(
            "SELECT {} FROM cypher('{}', {} {} {}) AS ({})",
            select_clause, self.graph_name, tag, cypher, tag, as_clause
        );
```

### 3.3 MERGE upsert — AGE 1.6 limitations

```61:67:edgequake/crates/edgequake-storage/src/adapters/postgres/graph/nodes_ops.rs
        // WHY: AGE 1.6.0 does NOT support `ON CREATE SET` (added only in the
        // unreleased dev branch, apache/age#2347) ...
        // stable, version-safe pattern is per-key `SET n.key = <literal>`
```

**First principle**: storage adapter complexity is **forced by extension version**, not preference. Per-property SET expands Cypher size **O(properties)** per upsert — batch UNWIND would be better but AGE batch patterns need careful validation.

### 3.4 Security note (RC-022-6)

Node reads build Cypher via format string + `escape_cypher_string`:

```12:16:edgequake/crates/edgequake-storage/src/adapters/postgres/graph/nodes_ops.rs
        let cypher = format!(
            "MATCH (n:Node {{node_id: '{}'}}) RETURN n LIMIT 1",
            escaped_id
        );
```

Entity IDs are normalized (`EntityId`) — reduces but does not eliminate injection if escape is incomplete. **Prepared Cypher** (AGE supports parameters map on prepared statements per manual) would be stronger.

### 3.5 Graph O(n) semantics

| Operation | Implementation | Round-trips |
|-----------|----------------|-------------|
| `get_node` | 1 Cypher | 1 |
| `get_nodes_batch` | 1 Cypher UNWIND | 1 |
| `node_degrees_batch` | 1 Cypher | 1 |
| `upsert_node` | 1 Cypher MERGE+SET | 1 |
| `upsert_nodes_batch` | batched Cypher | 1 |
| `get_all_nodes` (reconcile) | full scan | **O(N)** — admin only |

Query hot path uses batch methods — **good**.

---

## 4. KV storage (Postgres JSONB)

- Document metadata, chunk text, keyword cache, content-hash dedup keys  
- SPEC-021 P-G7: `keys_with_prefix` / `keys_with_suffix` replace full-table scans  
- Workspace-scoped hash keys via `ContentHasher::workspace_hash_key`

---

## 5. Cross-store consistency model

```
┌──────────────┐         ┌──────────────┐
│  pgvector    │         │  AGE graph   │
│  (vectors)   │         │  (structure) │
└──────┬───────┘         └──────┬───────┘
       │    NO 2PC / NO FK      │
       └──────────┬──────────────┘
                  │
           Application saga
     (persister: vectors → merge → compensate)
```

**PostgreSQL cannot enforce** referential integrity between agtype vertices and vector rows. Saga is the **correct** first-principles choice.

---

## 6. Embedding provider alignment

Workspace config typically uses:

| Provider | Model | Dimensions | pgvector column |
|----------|-------|------------|-----------------|
| OpenAI | text-embedding-3-small | 1536 | vector(1536) |
| Ollama | embeddinggemma / nomic | varies | per-workspace table |

Dimension mismatch caught at upsert:

```127:135:edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs
            if embedding.len() != self.dimension {
                return Err(StorageError::InvalidQuery(...));
            }
```

**First principle**: one workspace → one vector table dimension. Mixing models in same table is a hard error — **good**.

---

## 7. Upgrade recommendations (infra)

| Upgrade | Benefit | Risk |
|---------|---------|------|
| pgvector **0.8.0+** | iterative scan on filtered queries | rebuild indexes, test SIMD flags |
| AGE **1.6.0 stable** (non-rc) | production support | retest MERGE patterns |
| PostgreSQL **16→17** | future AGE branches | full regression |

See P-H3 in improvement plan.

---

## 8. Brutal summary

Storage layer is ** thoughtfully engineered** for a dual-backend RAG system: atomic vector batches, version-gated pgvector features, AGE workarounds documented in code. The **weakest link is infra pin lag**: application code assumes 0.8.0 behaviors that **0.7.4 cannot deliver**. Upgrade pgvector or accept sub-threshold filtered recall as a known limitation.
