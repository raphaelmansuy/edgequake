# 016 — EdgeQuake Data Layer Audit (pgvector + Apache AGE)

> **Mandate:** Brutal, code-grounded audit of how EdgeQuake leverages **pgvector** and
> **Apache AGE** for maximum performance, security, and precision/recall. Every claim is
> traced to source (`file:line`). _Code is law._ Methodology: **First Principles**,
> **5‑WHY**, and **O(N) complexity** analysis.

## Scope

This audit covers the four mandated dimensions across the PostgreSQL data layer:

1. **Storage** — table layout, index design, on-disk cost → [`002-storage/`](002-storage/README.md)
2. **Query time / query plan** — ANN path, graph traversal, recall → [`003-query/`](003-query/README.md)
3. **Update / insert / delete** — mutation paths, round-trip amplification → [`004-mutations/`](004-mutations/README.md)
4. **Ingestion pipeline** — end-to-end document flow → [`005-ingestion/`](005-ingestion/README.md)

Plus cross-cutting:

- **Capacity & scaling** — how many documents/pages, how performance evolves → [`006-capacity/`](006-capacity/README.md)
- **Improvements & migration** — prioritized fixes with data-migration plans → [`007-improvements/`](007-improvements/README.md)
- **Findings register** — severity-ranked, consolidated → [`008-findings-register/`](008-findings-register/README.md)
- **Methodology** — how every claim was derived → [`001-methodology/`](001-methodology/README.md)

## Grounding sources

- Mental models & deep dives: [`zz-reference/001-pgvector/`](../../zz-reference/001-pgvector/README.md),
  [`zz-reference/002-apache-age/`](../../zz-reference/002-apache-age/README.md)
- Code under audit (primary):
  - [vector.rs](../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs) — pgvector adapter
  - [graph/mod.rs](../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs) — AGE adapter
  - [graph/helpers.rs](../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers.rs) — Cypher exec/escaping
  - [connection.rs](../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs) — pool & extensions
  - [config.rs](../../edgequake/crates/edgequake-storage/src/adapters/postgres/config.rs) — defaults
  - [ingestion.rs](../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs) — orchestration

## Executive verdict (brutal)

EdgeQuake's **read path is fundamentally sound** — bare-operator ANN with an HNSW index
is index-eligible and fast. The **write path is the systemic weakness**: it is
*round-trip bound*, not CPU/disk bound. Three compounding anti-patterns dominate:

| #   | Finding                                                                                             | Location                                                                                                | Severity |
| --- | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- | -------- |
| F1  | Vector `upsert` issues **one `INSERT` per row** in a `for` loop                                     | [vector.rs#L543](../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L543)         | 🔴 High   |
| F2  | Every Cypher op runs **`LOAD 'age'` + `SET search_path` + query = 3 round trips**                   | [helpers.rs#L82](../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers.rs#L82)   | 🔴 High   |
| F3  | `upsert_edge` = **3 Cypher ops (~9 round trips) per edge**; `merge_*` add N+1 `get_node`            | [graph/mod.rs#L687](../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L687)   | 🔴 High   |
| F4  | **No transactions** — partial failure leaves vector/graph inconsistent                              | storage adapters (none found)                                                                           | 🔴 High   |
| F5  | Chunk **content duplicated into vector `metadata` JSONB** (heap + GIN bloat)                        | [ingestion.rs#L287](../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs#L287)           | 🟠 Med    |
| F6  | `hnsw.ef_search` **never tuned** → recall capped at default 40                                      | (no occurrence)                                                                                         | 🟠 Med    |
| F7  | Filtered ANN uses **post-filter** without `iterative_scan` → silent recall loss / short result sets | [vector.rs#L488](../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L488)         | 🟠 Med    |
| F8  | Cypher built by **string interpolation** + hand-rolled escaping → injection surface                 | [helpers.rs#L233](../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers.rs#L233) | 🟠 Med    |
| F9  | `get_neighbors` variable-length `[*1..depth]` with **no `LIMIT`** → `O(branching^depth)`            | [graph/mod.rs#L1116](../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L1116) | 🟠 Med    |
| F10 | `insert_batch` processes documents **sequentially**                                                 | [ingestion.rs#L332](../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs#L332)           | 🟡 Low    |
| F11 | `max_connections=10` default vs 16 concurrent extractions → pool contention                         | [config.rs#L81](../../edgequake/crates/edgequake-storage/src/adapters/postgres/config.rs#L81)           | 🟡 Low    |

## Capacity headline

- **Read (vector search):** sub‑100 ms p99 while the HNSW index fits in RAM. For
  `vector(1536)`, that is roughly **1–5 M vectors on a 16–32 GB box** (≈6 GB index/M rows).
- **Write (ingestion):** the binding constraint. Graph writes for **one page**
  (~10 entities, ~15 relationships) cost **~225 serialized DB round trips** today —
  graph ingestion throughput, not storage, is the ceiling.
- Full derivation: [`006-capacity/001-limits-and-scaling.md`](006-capacity/001-limits-and-scaling.md).

See [`008-findings-register/`](008-findings-register/README.md) for the full severity-ranked list with remediation links.
