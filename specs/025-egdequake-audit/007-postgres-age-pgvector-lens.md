# 007 — Postgres / AGE / pgvector Expert Lens

**Cross-ref:** [002 Ingestion](./002-ingestion-pipeline-audit.md) · [009 O(n)](./009-complexity-on-lens.md) · [008 System](./008-system-engineering-lens.md)

**Findings:** R-02, R-03, R-08, N-03, N-06, N-09, N-12, N-13

---

## Storage Topology (code law)

```text
  PostgreSQL (mandatory — no memory fallback)
  │
  ├── tasks                    JSONB payloads, worker queue
  ├── KV tables                doc metadata, chunks, hashes, checkpoints
  ├── eq_{ws}_vectors          pgvector per workspace (registry)
  ├── eq_{ws}_kv               FTS companion for BM25 join
  ├── Apache AGE graph         entities + edges (Cypher)
  ├── documents (relational)   dashboard KPI dual-write
  └── migrations m001..m045    bootstrap + reconcile modules
```

---

## pgvector Patterns (strength)

| Pattern | Implementation | Grade |
|---------|----------------|:-----:|
| Workspace isolation | `WorkspaceVectorRegistry` → `eq_{prefix}_vectors` | A |
| Metadata filters in SQL | `query_filtered` tenant/workspace pushdown | A |
| Chunk dedup | `content_ref` in metadata, text in KV | A |
| Batch upsert | Merger + persister batch paths | A |
| FTS hybrid | `adapters/postgres/vector/fts.rs` native search | A |
| Iterative scan cap | Migration reconcile m045 | A |

**Vector types in one table:**

```text
  metadata.type ∈ { chunk, entity, relationship }
  chunk → content_ref → KV key
  entity → normalized entity_id
  relationship → src_id, tgt_id, description
```

Query modes filter by type — correct single-table multi-vector LightRAG pattern.

---

## Apache AGE Patterns

| Pattern | Code | Grade |
|---------|------|:-----:|
| Batch node upsert | `merge_entities_batch` | A |
| Batch edge merge | `merge_relationships_batch` | A |
| Provenance props | `source_document_id`, `source_chunk_ids` | A |
| Multi-hop query | `graph_hops.rs` BFS | B- (N+1) |
| Community labels | Louvain → `community_id` property | B |
| Cypher execution | `cypher_exec.rs` helpers | A |

### N-06 — Graph read N+1

`graph_hops.rs` documents the limitation:

```9:10:edgequake/crates/edgequake-query/src/graph_hops.rs
/// Uses incident-edge lookup per frontier node (not `get_edges_for_nodes_batch`,
/// which only returns edges whose both endpoints lie in the input set).
```

Per frontier node per hop: `get_node_edges(node_id)`. **Correct semantics, wrong scale.**

**Fix direction:** Batch incident-edge API or materialized adjacency for hot workspaces.

---

## KV Storage Patterns

| Use | Key pattern | Issue |
|-----|-------------|-------|
| Document meta | `{uuid}-metadata` | ✓ |
| Document body | `{uuid}-content` | Duplicated in task JSONB (N-03) |
| Chunks | `{uuid}-chunk-{n}` | ✓ SSOT for text |
| Dedup hash | workspace-scoped hash→doc | ✓ |
| Injection | `injection::{ws}::{id}-metadata` | Prefix list O(n) (N-09) |
| Checkpoints | pipeline checkpoint keys | ✓ crash recovery |

### N-12 — KV outside saga

Persister saga covers vector/graph compensation. **Admission KV writes** precede worker — not in same transaction as persist success.

---

## Task Queue (Postgres)

`edgequake-tasks/src/postgres.rs`:

- Durable `tasks` table
- Orphan recovery (processing → pending/failed)
- Heartbeat stale detection (10 min)
- Payload stores full `TextInsertData` — **scale concern (N-03)**

**Good:** Tenant fairness, retry backoff, `/health` pressure metrics.  
**Bad:** Large JSONB payloads for big documents.

---

## Community Index (R-03)

```text
  persist success
       │
       v
  schedule_community_index_refresh (debounced)
       │
       v
  CommunityRefreshScheduler (in-memory timers)
       │
       v
  detect_and_persist_communities (Louvain)
       │
       v
  community_id written to AGE nodes
```

**Postgres expert note:** Debounce state is **process-local** (`OnceLock` scheduler). Multi-replica deployments can each fire Louvain unless external lock added — **P2 ops gap** for horizontal scale-out.

---

## Read Model Reconciliation (strength)

`document_read_model.rs`:

- Merge strategy: `max(postgresql, kv)`
- Drift detection exposed in `/health` → `operational.read_model`

Honest dual-write pattern for dashboard vs KV truth.

---

## Postgres Expert Verdict

**Grade: A-**

EdgeQuake uses Postgres **as intended**: pgvector for ANN, GIN/GIN-ts for FTS, relational for ops metadata, AGE for graph merge. Workspace registry is production-grade.

**Deductions:**

- N-03 task payload bloat
- N-06 graph N+1 reads
- N-09 prefix scan APIs without pagination
- Multi-replica community debounce not cluster-safe

**Do not:** Split vector types into separate databases — current single-table + metadata filter is correct for LightRAG scale to mid-millions of vectors per workspace.
