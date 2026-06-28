# Query Catalog — Storage SQL with O(N) Risk Analysis

Cross-reference for every SQL pattern in `edgequake-storage` PostgreSQL adapters.

**Risk levels**: 🔴 CRITICAL · 🟠 HIGH · 🟡 MEDIUM · 🟢 LOW

---

## KV Storage (`adapters/postgres/kv.rs`)

| ID | Query | Method | Complexity | Risk | Notes |
| -- | ----- | ------ | ---------- | ---- | ----- |
| KV-01 | `SELECT value FROM {t} WHERE key = $1` | `get_by_id` | O(1) PK | 🟢 | Primary key index |
| KV-02 | `SELECT value FROM {t} WHERE key = ANY($1)` | `get_by_ids` | O(k) | 🟢 | k = batch size |
| KV-03 | `SELECT key FROM {t} WHERE key = ANY($1)` | `filter_keys` | O(k) | 🟢 | Bounded input |
| KV-04 | `INSERT … ON CONFLICT` (loop) | `upsert` | O(n) RTT | 🟠 | n round-trips |
| KV-05 | `DELETE FROM {t} WHERE key = ANY($1)` | `delete` | O(k) | 🟢 | Batch delete |
| KV-06 | **`SELECT row_count FROM {t}_stats`** (was `COUNT(*)`) | **`count`** | **O(1)** | **🟢** | Maintained counter + triggers |
| KV-07 | **`SELECT key FROM {t}`** | **`keys`** | **O(N)** | **🔴** | No LIMIT |
| KV-08 | `DELETE FROM {t}` | `clear` | O(N) | 🟡 | Intentional wipe |
| KV-09 | `UPDATE … WHERE key=$1 AND value->>'status'=$2` | `transition_if_status` | O(1) | 🟢 | PK + filter |
| KV-10 | GIN index on `value` | DDL | — | 🟢 | JSON path queries only |

### New (SPEC-011)

| ID | Query | Method | Complexity | Risk |
| -- | ----- | ------ | ---------- | ---- |
| KV-11 | `SELECT EXISTS(SELECT 1 FROM {t} LIMIT 1)` | `is_empty` | O(1) | 🟢 |
| KV-12 | `SELECT 1 FROM {t} LIMIT 1` | `ping` | O(1) | 🟢 |
| KV-13 | `SELECT key FROM {t} WHERE key LIKE $1` | `keys_like` | O(K) | 🟡 |
| KV-14 | `INSERT … SELECT * FROM unnest($1::text[], $2::jsonb[])` | `upsert` batch | O(1) RTT | 🟢 |

---

## Vector Storage (`adapters/postgres/vector.rs`)

| ID | Query | Method | Complexity | Risk |
| -- | ----- | ------ | ---------- | ---- |
| VEC-01 | `SELECT … ORDER BY embedding <=> $1 LIMIT $k` | `query` | O(log N) | 🟢 | HNSW index |
| VEC-02 | `INSERT … ON CONFLICT` (loop) | `upsert` | O(n) RTT | 🟠 |
| VEC-03 | **`SELECT COUNT(*) FROM {t}`** | **`count`** | **O(N)** | **🔴** |
| VEC-04 | `DELETE FROM {t} WHERE metadata->>'workspace_id'=$1` | `clear_workspace` | O(N) | 🟡 | Should use materialized column |
| VEC-05 | HNSW / GIN / B-tree indexes | DDL | — | 🟢 |

### New (SPEC-011)

| ID | Query | Method | Complexity | Risk |
| -- | ----- | ------ | ---------- | ---- |
| VEC-06 | `SELECT EXISTS(… LIMIT 1)` | `is_empty` | O(1) | 🟢 |
| VEC-07 | `SELECT 1 … LIMIT 1` | `ping` | O(1) | 🟢 |

---

## Graph Storage (`adapters/postgres/graph/mod.rs`)

| ID | Query | Method | Complexity | Risk |
| -- | ----- | ------ | ---------- | ---- |
| G-01 | `COUNT(*) FROM {g}."_ag_label_vertex"` | `node_count` | O(N) | 🟠 | Native SQL, still seq scan |
| G-02 | `COUNT(*) FROM {g}."_ag_label_edge"` | `edge_count` | O(N) | 🟠 |
| G-03 | Cypher `MATCH (n:Node) WHERE n.workspace_id=$w RETURN count(n)` | `node_count_by_workspace` | O(N) | 🟡 | Per-workspace filter |
| G-04 | Cypher degree CTE over all edges | `search_nodes` | O(E) | 🟠 | Full edge GROUP BY |
| G-05 | `MATCH (n:Node) RETURN n` | `get_all_nodes` | O(N) | 🔴 | Full graph fetch |
| G-06 | Native SQL batch degree queries | `node_degrees_batch` | O(k) | 🟢 | Optimized path |
| G-07 | `LOAD 'age'` + `SET search_path` per query | All Cypher | O(1) overhead | 🟡 | Session setup cost |

### New (SPEC-011)

| ID | Query | Method | Complexity | Risk |
| -- | ----- | ------ | ---------- | ---- |
| G-08 | `SELECT 1 FROM {g}."Node" LIMIT 1` | `ping` | O(1) | 🟢 |

---

## Conversation / PDF (`conversation.rs`, `pdf_storage_impl.rs`)

| ID | Query | Method | Complexity | Risk |
| -- | ----- | ------ | ---------- | ---- |
| PDF-01 | `SELECT COUNT(*) FROM pdf_documents WHERE …` | `count_pdfs` | O(N) filtered | 🟡 | Has workspace index |
| PDF-02 | `SELECT … pdf_data …` in list | `list_pdfs` | O(N) × blob | 🟠 | Heavy pagination |
| CONV-01 | `SELECT COUNT(*) FROM messages WHERE conversation_id=$1` | message count | O(k) | 🟢 | FK indexed |

---

## Connection Pool (`connection.rs`)

| ID | Pattern | Complexity | Risk |
| -- | ------- | ---------- | ---- |
| POOL-01 | Lazy `PgPoolOptions::new().max_connections(N)` × 3 adapters | 3×N connections | 🔴 |
| POOL-02 | `SELECT 1 as health` | O(1) | 🟢 | Already exists, underused |

---

## Mitigation Summary

| Query class | Mitigation |
| ----------- | ---------- |
| COUNT for health | Replace with `ping()` |
| COUNT for is_empty | Replace with EXISTS |
| keys() full scan | Add `keys_like()`, update API handlers |
| Loop upsert | unnest batch INSERT |
| Triple pools | `PostgresPool::from_existing()` shared across adapters |
