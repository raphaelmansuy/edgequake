# 005 — Query complexity catalog (CRUD / Query)

SSOT for **request-path allowed** storage ops and their asymptotic cost.
Cross-ref: [001-first-principles](./001-first-principles.md), [002-crossref](./002-crossref-query-postgres-age-pgvector.md), [003-budgets](./003-performance-budgets-and-gates.md).

**Legend**

| Tag | Meaning |
| --- | --- |
| **OK** | Allowed on HTTP/query/ingest request path |
| **ADMIN** | Admin / clear / rebuild / offline only |
| **FORBIDDEN** | Must never run on request path |

---

## 1. KV (`eq_{ns}_kv`)

| Op | Impl | Index | Complexity | Path |
| --- | --- | --- | --- | --- |
| `get_by_id` | PK lookup | `PRIMARY KEY (key)` | O(log N) | OK |
| `get_by_ids` / `_ordered` | `UNNEST` + PK | PK | O(K log N) one RT | OK |
| `upsert` | `UNNEST` + `ON CONFLICT` | PK | O(K log N) | OK |
| `delete` | `ANY($1)` | PK | O(K log N) | OK |
| `count` | `*_kv_stats` (+ trigger) | stats | **O(1)**; fallback COUNT\* O(N) | OK (prefer stats) |
| `keys_with_prefix` | `LIKE 'p%'` | PK text_pattern | O(M) | OK |
| `keys_with_suffix` | `reverse(key)` | `*_kv_reverse_key_idx` | O(M) | OK |
| `keys` / mid-wildcard LIKE | pattern scan | none | **O(N)** | ADMIN |
| `transition_if_status` | atomic UPDATE | PK | O(log N) | OK |

**Anti-pattern:** loop `get_by_id` instead of `get_by_ids` (N+1).

---

## 2. Vectors (`eq_{ns}_vectors` / pgvector)

| Op | Impl | Index | Complexity | Path |
| --- | --- | --- | --- | --- |
| `query` (unfiltered ANN) | `ORDER BY embedding <=>` + HNSW GUCs | HNSW/IVF | ~O(ef × log N) | OK |
| `query_filtered` | same + tenant/ws/doc filter + `iterative_scan` | HNSW + btree filters | ~O(ef × log N) + iterative | OK (**required** for scoped RAG) |
| `text_search_filtered` | `ts_rank_cd` + GIN | `content_tsv` GIN | O(log N + candidates) | OK |
| `upsert` | UNNEST chunks (~1000) | PK + HNSW insert | O(K log N) + graph insert | OK |
| `delete` / `delete_by_document` | PK / `document_id` | PK / `*_doc_id_idx` | O(K log N) | OK |
| `count` | `*_vectors_stats` | stats | **O(1)** | OK |
| `clear` | `DELETE` | — | **O(N)** | ADMIN |

**Session GUCs (filtered):** `hnsw.ef_search`, `hnsw.iterative_scan=relaxed_order` (default), `hnsw.max_scan_tuples=20000` when pgvector ≥0.8.

**Anti-patterns:** missing ANN index (seq-scan cliff); filtered search with `iterative_scan=off`; metadata GIN (removed — 0 scans).

---

## 3. Graph AGE — reads

| Op | Impl | Index | Complexity | Path |
| --- | --- | --- | --- | --- |
| `get_node` / `has_node` | Bound Cypher or native | UNIQUE `node_id` expr | O(log N) if UNIQUE used | OK |
| `get_nodes_batch` | Native SQL + `UNNEST` + UNIQUE | `idx_node_prop_node_id_unique` | O(K log N) one RT | OK (**prefer**) |
| `get_nodes_by_ids` | Cypher `IN` | may ignore GIN | O(K log N) best-effort | OK (prefer batch SQL) |
| `get_all_nodes` / `get_all_edges` | full scan | — | **O(N)** | **FORBIDDEN** |
| `node_degree` | Native SQL | edge ends | O(deg) | OK |
| `node_degrees_batch` | Native aggregate | edge props | O(K + E′) | OK |
| `get_edges_for_node_set` | Native `ANY` | edge property indexes | O(K + E′) | OK |
| `get_knowledge_graph` / `get_neighbors` | Bounded Cypher expand | start/end + labels | O(branch^depth) | OK (depth-bounded) |
| `list_nodes/edges_filtered` | Paginated scan | filters | COUNT O(N) filtered + page O(limit) | OK (paginated) |
| `node_count` / `edge_count` | Exact COUNT\* | — | **O(N)** | ADMIN / rare |
| `node_count_fast` / `edge_count_fast` | `pg_class.reltuples` | — | **O(1)** | OK (dashboards) |
| `node_count_by_source_prefix` | GIN `@>` chunk probes | `idx_*_source_ids_gin` | O(probes × log N), cap 256 | OK |
| `node_counts_by_source_prefixes` | Batched GIN probes | same | O(D × probes × log N) **one RT** | OK (**list reconcile**) |

**Anti-patterns:** per-document loop of prefix counts (N+1 RTs); unbounded Cypher `MATCH (n:Node) RETURN n`; Cypher property MATCH assuming GIN (AGE#2348).

---

## 4. Graph AGE — writes

| Op | Impl | Index | Complexity | Path |
| --- | --- | --- | --- | --- |
| Native `upsert_nodes_batch` | `INSERT … ON CONFLICT (node_id expr)` | UNIQUE | O(K log N) | OK (**production**) |
| Native `upsert_edges_batch` | `ON CONFLICT (source,target)` | UNIQUE | O(K log N) | OK (**production**) |
| Cypher `MERGE` upsert | Cypher | often unused | higher latency/locks | OK debug only (`EDGEQUAKE_NATIVE_GRAPH_WRITES=0`) |
| `delete_node` | Cypher DETACH | — | O(1 + deg) | OK |
| `clear` / `clear_workspace` | Cypher delete | — | **O(N)** | ADMIN |

---

## 5. Index SSOT (runtime)

| Source | Role |
| --- | --- |
| Checksum-locked `migrations/0NN_*.sql` | Applied once — **do not edit** after apply |
| `migrations/support/NNN/apply.sql` | Every-boot reconcile (edit here) |
| `graph_lifecycle::ensure_indexes` | Runtime ensure + skip-if-valid UNIQUE |
| `vector/ddl.rs` | Per-workspace ANN + btree + FTS |

---

## 6. Request-path decision tree

```
Need entity/edge by id?     → native batch SQL + UNIQUE
Need k-hop expand?          → bounded Cypher (labels explicit)
Need scoped ANN?            → query_filtered + iterative_scan
Need document entity count? → node_counts_by_source_prefixes (batch)
Need dashboard totals?      → *_fast / stats tables
Need full dump / clear?     → ADMIN only
```
