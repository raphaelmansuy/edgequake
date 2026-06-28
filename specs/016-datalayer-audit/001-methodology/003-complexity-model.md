# Complexity & Round-Trip Cost Model

The model used to quantify cost throughout this audit. We separate **algorithmic
complexity** (work inside Postgres) from **round-trip complexity** (client↔server
requests), because for EdgeQuake the latter dominates the write path.

## Notation

| Symbol     | Meaning                                            |
| ---------- | -------------------------------------------------- |
| `N`        | rows in the vector table                           |
| `V`, `E`   | vertices / edges in the AGE graph                  |
| `C`        | chunks in a document                               |
| `Ne`, `Re` | entities / relationships extracted from a document |
| `d`        | embedding dimension (1536)                         |
| `b`        | average graph branching factor                     |
| `RT`       | one Postgres round trip                            |

## Read path

| Operation               | Postgres work                     | Round trips | Notes                          |
| ----------------------- | --------------------------------- | ----------- | ------------------------------ |
| `query` (HNSW)          | `O(ef_search · log N)` dist comps | 1 `RT`      | index-eligible bare operator ✅ |
| `query_filtered`        | same + filter eval                | 1 `RT`      | post-filter, recall risk (F7)  |
| `get_node` (indexed)    | `O(log V)`                        | 3 `RT`      | LOAD+search_path+query (F2)    |
| `get_neighbors [*1..k]` | `O(b^k)` paths                    | 3 `RT`      | unbounded fan-out (F9)         |
| `node_count`            | `O(1)` catalog read               | 1 `RT`      | native SQL on child tables ✅   |

## Write path (the expensive one)

Per-operation round-trip accounting, **as implemented today**:

| Unit                   | Calls                                                         | Round trips each | Subtotal    |
| ---------------------- | ------------------------------------------------------------- | ---------------- | ----------- |
| 1 chunk → vector       | 1 `upsert([1 row])`                                           | 1 `RT`           | **C · 1**   |
| 1 entity → graph       | `get_node` + `upsert_node`                                    | 3 + 3            | **Ne · 6**  |
| 1 relationship → graph | `get_node`×2 (`ensure_node_exists`) + `upsert_edge`(3 cypher) | (3+3) + 3·3      | **Re · 15** |

> Each `upsert_edge` runs `MERGE both nodes` + `DELETE r` + `MATCH…CREATE` = 3 Cypher
> statements, and **each statement** pays the 3‑RT `LOAD 'age'`/`search_path`/query tax
> ([helpers.rs#L82](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers.rs#L82)).

### Worked example — one dense page

Assume `C≈8`, `Ne≈10`, `Re≈15` (typical for a content-rich page):

```
vector   : 8  · 1  =   8 RT
entities : 10 · 6  =  60 RT
edges    : 15 · 15 = 225 RT   (upper bound incl. N+1 get_node)
                     ------
                     ≈ 225–293 RT  serialized per page
```

At an optimistic **1 ms/RT** on localhost that is **~0.25 s/page of pure latency**,
before any LLM extraction or embedding. On a networked DB (2–5 ms/RT incl. Cypher
parse) it is **0.5–1.5 s/page**. This is the binding constraint on ingestion throughput.

### Target after remediation

| Unit                     | Batched approach                | Round trips |
| ------------------------ | ------------------------------- | ----------- |
| C chunks → vector        | one multi-row `INSERT … VALUES` | **1**       |
| Ne entities → graph      | one `UNWIND $rows … MERGE`      | **1**       |
| Re relationships → graph | one `UNWIND $rows … MERGE`      | **1**       |

→ from `O(C + 6·Ne + 15·Re)` round trips to **`O(1)` per document** (3 statements).
For the worked example: **293 → 3 round trips** (~100× reduction in latency-bound work).

## Storage cost

| Item                            | Bytes                                  | Source                                                                                                |
| ------------------------------- | -------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `vector(1536)` payload          | `4·1536 + 8 = 6152`                    | pgvector layout (`zz-reference/001-pgvector`)                                                         |
| chunk row heap (with content)   | ~6 KB + ~2–4 KB text                   | F5 ([ingestion.rs#L287](../../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs#L287)) |
| HNSW index / row (d=1536, m=16) | ≈ `4·d + m·2·4` ≈ 6.3 KB _(inference)_ | grounded m=16 default                                                                                 |

So **1 M chunks ≈ ~10 GB heap + ~6 GB HNSW index**. The index must stay resident in
RAM for sub-100 ms search — this fixes the practical capacity envelope analyzed in
[`006-capacity/`](../006-capacity/001-limits-and-scaling.md).
