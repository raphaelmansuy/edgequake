# 07 — CQRS Dual-Store Design: AGE Graph + Relational Entities

> **Spec**: 021-storage-study  
> **File**: 06-first-principles/07-cqrs-dual-store-design.md  
> **Date**: 2026-06-25  
> **Answers Question 1**: "Can maintaining entities/relationships tables in sync with  
> the AGE graph improve query performance and developer readability?"  
> **Verdict**: YES — they serve orthogonal access patterns (CQRS)

---

## Why the Initial Analysis Was Wrong

The original spec (01-overview/01-executive-summary.md) classified `entities` and
`relationships` tables as "orphaned" and recommended dropping them. This was
premature. The question is not whether to use **one** store — it is whether the
**two stores serve different access patterns** that justify dual ownership.

---

## First-Principles Access Pattern Analysis

### Apache AGE (`Node`, `EDGE`)

AGE stores data as `agtype` (a JSON superset) in its internal tables:
```
{graph}._ag_label_vertex  -- all Node records
{graph}._ag_label_edge    -- all EDGE records
```

Every property access goes through `ag_catalog.agtype_to_json(properties)`:
```sql
-- From scan_ops.rs (actual production code):
ag_catalog.agtype_to_json(v.properties)->>'tenant_id'
```

**What AGE excels at (irreplaceable):**
- Multi-hop Cypher traversal: `MATCH (a)-[*1..3]->(b)`
- Pattern matching: `MATCH (p:Node)-[:EDGE]->(org:Node {entity_type:'ORGANIZATION'})`
- BFS/DFS graph algorithms
- Community detection queries
- Neighbor discovery without application-level loops

**Where AGE has structural limits (code-proven):**

| Limitation                              | Code Evidence                                                                                                      | Impact                                                     |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------- |
| `COUNT(*)` is O(N)                      | scan_ops.rs: `SELECT COUNT(*)::BIGINT AS total FROM {graph}._ag_label_vertex WHERE ...` — no pg_class optimization | Health checks, stats endpoints are slow at scale           |
| Expression indexes, not column indexes  | Migration 014: index on `(agtype_to_json(properties)->>'tenant_id')` — function call index, not plain column       | Cannot use partial indexes, statistics less accurate       |
| No cross-schema JOINs via Cypher        | Cypher cannot JOIN with `documents` table                                                                          | All entity-document queries require application-level code |
| `search_path` must include `ag_catalog` | After every connection, SET required                                                                               | Connection setup overhead, pooling complexity              |
| FTS requires function expression GIN    | Migration 015: `gin(to_tsvector(...agtype_to_json(properties)...))`                                                | Less efficient than `tsvector` stored column               |

### PostgreSQL Relational `entities` Table

**What relational tables excel at (native PostgreSQL strengths):**

| Capability                   | Mechanism                                                               | AGE equivalent                             |
| ---------------------------- | ----------------------------------------------------------------------- | ------------------------------------------ |
| O(1) approx count            | `pg_class.reltuples`                                                    | O(N) scan                                  |
| B-tree on plain column       | `CREATE INDEX ON entities(name)`                                        | Expression index                           |
| Composite B-tree             | `CREATE INDEX ON entities(tenant_id, workspace_id, entity_type)`        | 3 separate expression indexes              |
| Stored tsvector FTS          | `ADD COLUMN tsv tsvector GENERATED ALWAYS AS (to_tsvector(...)) STORED` | Expression GIN on function call            |
| JOIN with documents          | `JOIN documents ON d.id = ANY(source_chunk_ids::text[])`                | Application-level join only                |
| GIN on array column          | `CREATE INDEX ON entities USING GIN (source_chunk_ids)`                 | String-based source_ids                    |
| `pg_dump` / PITR             | Standard PostgreSQL                                                     | AGE internals may not restore cleanly      |
| BI tools (Metabase, Grafana) | Direct SQL connection                                                   | Requires Cypher knowledge                  |
| Admin tooling (psql `\d`)    | Relational schema                                                       | Opaque agtype                              |
| Planner statistics           | `ANALYZE entities` → accurate cardinality                               | Function-call expressions → poor estimates |

---

## CQRS Architecture: Two Stores, Two Roles

The correct mental model is **Command-Query Responsibility Segregation**:

```
                        WRITE PATH (Command)
                              |
                    Merger (entity.rs)
                              |
              +---------------+----------------+
              |                                |
              v                                v
    AGE graph (MERGE Node)         entities table (UPSERT)
    [PRIMARY: traversal]           [REPLICA: analytics]
    Cypher patterns               Standard SQL queries
    Multi-hop BFS/DFS             JOINs, GROUP BY, FTS
              |                                |
              v                                v
                        READ PATHS (Query)
                              |
         +--------------------+--------------------+
         |                    |                    |
         v                    v                    v
   Graph traversal      Analytics queries     Admin / BI
   (Local/Global mode)  (workspace stats)    (developer tools)
   Cypher MATCH          SQL GROUP BY         psql / Metabase
   get_neighbors()       COUNT(*) O(1)        direct SQL
```

---

## Performance Comparison (Concrete Queries)

### Query A: Count entities in workspace (health check / stats)

```sql
-- Current (AGE, O(N)):
SELECT COUNT(*)::BIGINT
FROM edgequake._ag_label_vertex v
WHERE ag_catalog.agtype_to_json(v.properties)->>'workspace_id' = $1

-- With relational (O(1) estimate):
SELECT reltuples::BIGINT FROM pg_class WHERE relname = 'entities'
-- Or for exact but indexed:
SELECT COUNT(*) FROM entities WHERE workspace_id = $1
-- Uses B-tree index (tenant_id, workspace_id) → O(log N + K)
```

**Speedup at 100K entities**: ~1000x (index scan vs full AGE scan)

### Query B: Full-text search on entity name + description

```sql
-- Current (AGE, expression GIN — migration 015):
SELECT ag_catalog.agtype_to_json(properties)->>'node_id'
FROM edgequake._ag_label_vertex
WHERE to_tsvector('english', ag_catalog.agtype_to_json(properties)->>'node_id')
      @@ plainto_tsquery('english', 'apple products')

-- With relational (stored tsvector GIN — no function call):
SELECT name FROM entities
WHERE tsv @@ plainto_tsquery('english', 'apple products')
-- Uses GIN on stored tsvector column → O(log N)
```

**Quality improvement**: Stored `tsvector` can include `description` + `name`
for richer FTS without runtime concatenation overhead.

### Query C: Find entities contributed by a specific document (delete cascade)

```sql
-- Current (deletion.rs): full KV scan + graph scan per-entity
-- O(K * E) where K = chunk count, E = edges per entity

-- With relational + GIN index on source_chunk_ids:
SELECT id, name FROM entities
WHERE source_chunk_ids && ARRAY['{doc_id}-chunk-0', '{doc_id}-chunk-1', ...]
-- Uses GIN index → O(N_matching) instead of O(All_entities)
```

**This is the exact pattern migration 038 builds GIN indexes for** — but on the
AGE internal tables via Cypher workarounds. Native PostgreSQL GIN on a TEXT[]
column is both simpler and faster.

### Query D: Entities with their source documents (provenance report)

```sql
-- AGE: impossible in Cypher (cross-store join)
-- Requires two separate queries + application-level merge

-- With relational entities table:
SELECT e.name, e.entity_type, d.title, d.created_at
FROM entities e
JOIN documents d ON d.id::text = ANY(e.source_chunk_ids)
WHERE e.workspace_id = $1
ORDER BY e.name
```

**Entirely new query class** that becomes possible only with relational entities.

---

## Dual-Write Cost Analysis

### Write Overhead

Adding a relational write in `merger/entity.rs` alongside the existing vector and
graph writes adds:

```
Current write per entity:
  1. vector_storage.upsert()  → ~1ms (UNNEST batch)
  2. graph_storage.upsert_node()  → ~5ms (Cypher MERGE)

Proposed write per entity:
  1. vector_storage.upsert()  → ~1ms
  2. graph_storage.upsert_node()  → ~5ms
  3. entities table UPSERT  → ~0.2ms (B-tree + simple SQL)
```

The relational write adds ~4% overhead. Because entities are written in batches
(500/UNNEST in graph), the relational write can also use batch UNNEST INSERT.

### Failure Handling

The relational write is **non-blocking on failure** (best-effort sync):
- If the relational write fails, the graph write is still authoritative
- The sync is marked as "pending repair" in a `entity_sync_pending` flag
- A background reconciler retries the sync later

This means ingestion latency is **unaffected** by relational sync failures.

---

## Verdict

| Decision                                                   | Rationale                                             |
| ---------------------------------------------------------- | ----------------------------------------------------- |
| **Keep AGE graph as PRIMARY source for traversal**         | Cypher patterns, multi-hop, BFS — irreplaceable       |
| **Populate `entities`/`relationships` as CQRS read model** | Analytics, FTS, JOINs, developer tools, BI            |
| **Dual-write at merger level (best-effort)**               | 4% overhead, non-blocking failure mode                |
| **AGE is source of truth for graph topology**              | On conflict, AGE wins; relational is rebuilt from AGE |
| **Relational is source of truth for analytics**            | Stats queries bypass AGE completely                   |

The original "these tables are orphaned, drop them" analysis was wrong. The correct
framing is: **they are not orphaned — they are unpopulated**. The remedy is to
populate them, not to drop them.

See [08-sync-ascending-compat.md](08-sync-ascending-compat.md) for the migration
strategy, and [09-drift-detection-autorepair.md](09-drift-detection-autorepair.md)
for the auto-repair design.
