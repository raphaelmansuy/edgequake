---
title: "Complexity matrix"
description: "Expected cost, limits, and failure modes for each class of data-layer operation. The 235 Ref IDs fall into seven classes. This is a design-time classification, not a measurement."
---

# Complexity matrix

This page tells you what each kind of data-layer operation costs, what limits apply, and how it fails. N is the number of rows in the table. K is the number of rows you ask for. These are design-time estimates from the SPEC-088 inventory, not measured timings. For measured results, see [improvements.md](./improvements.md).

The 235 registered operations fall into seven classes. Every operation in a class has identical text, so the table lists each class once and then gives the Ref ID numbers in it. A Ref number is the `NNN` suffix of an ID such as `DATA-AGE-GRAPH-HAS-NODE-025`. Look up the full entry on [postgres.md](./postgres.md), [pgvector.md](./pgvector.md), or [age.md](./age.md).

## Classes

| Class | Operations | Time | Space | Key limits | Failure modes |
|---|---|---|---|---|---|
| **Vector nearest-neighbor search** | 4 | O(ef * log N) expected; O(N) worst case if the planner picks a sequential scan | O(K + ef) | `hnsw.ef_search` = clamp(4*K, 40, 1000). Filtered search needs iterative scan (`max_scan_tuples` = 20000). | Timeout, fewer than K results, silent recall loss, sequential-scan cliff |
| **Ordinary indexed SQL** | 115 | O(log N) with an index; O(N) without | O(K) | Use keyset pagination, not large OFFSET. Keep `statement_timeout` set. | Timeout, lock wait, out of memory on an unbounded SELECT |
| **DDL and index builds** | 29 | O(N log N) for an HNSW build; O(N) for a btree | `maintenance_work_mem` | Never REINDEX on the request path. Use `max_parallel_maintenance_workers` for HNSW. pgvector 0.8.2 or later. | ACCESS EXCLUSIVE lock contention, out of memory during build |
| **Session settings** | 4 | O(1) | O(1) | Use `SET LOCAL` only inside short transactions. Never leak settings across pooled connections. | A leaked setting changes recall or plans for the next user of the connection |
| **Graph (AGE)** | 51 | O(K log N) for a batch by ID; O(branch^depth) for an expansion; O(N) full scan is forbidden | O(K) or O(branch^depth) | Prefer native writes (unique `node_id`). Cypher MERGE is for fallback and debugging. | Cartesian expansion, timeout, out of memory on an unbounded MATCH |
| **Legacy key-value adapter** | 18 | O(log N) for one key; O(K log N) for a batch; O(M) for a prefix scan | O(K) or O(M) | Upsert batches of at most 1000. PostgreSQL allows 65535 bind parameters. | Error on a batch that mixes workspaces; O(N) COUNT fallback |
| **Tasks** | 14 | O(log N) for a get; claim is O(W + log N) plus a row lock | O(1) for a claim; O(page) for a list | `SKIP LOCKED` for concurrency. Lease timeout lets another worker reclaim a task. | Starvation without the fair claim; lease expiry race |

## Ref numbers per class

| Class | Ref numbers |
|---|---|
| **Vector nearest-neighbor search** | 001 to 002, 017, 019 |
| **Ordinary indexed SQL** | 003 to 016, 093 to 130, 145 to 156, 158 to 194, 198 to 206, 209 to 213 |
| **DDL and index builds** | 018, 020 to 021, 023 to 024, 207 to 208, 214 to 235 |
| **Session settings** | 022, 195 to 197 |
| **Graph (AGE)** | 025 to 074, 157 |
| **Legacy key-value adapter** | 075 to 092 |
| **Tasks** | 131 to 144 |

## Reading notes

- The vector classes describe the original SPEC-088 view of the adapter. In the default typed mode, the search also runs inside a bounded transaction with a statement timeout. See [pgvector.md](./pgvector.md#how-a-vector-search-runs).
- The key-value class is legacy. The `eq_*_kv` tables were dropped by migration 125.
- The DDL class includes the `SCHEMA` entries for migrations. Today every migration is applied by `edgequake migrate`, which takes locks as listed in [postgres.md](./postgres.md#schema-and-migrations).
- "Forbidden" in the graph class means the request path must not call the operation. Admin tools and tests may.
