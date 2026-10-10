---
title: "Complexity matrix"
description: "Expected cost, limits, and failure modes for each class of data-layer operation. The 235 Ref IDs fall into seven classes. This is a design-time classification, not a measurement."
---

# Complexity matrix

This page describes what each class of data-layer operation costs, which limits apply, and how it tends to fail. The figures are design-time estimates from the SPEC-088 inventory, not measured timings. For measured results, see [improvements.md](./improvements.md).

Notation: N is the number of rows in the table, and K is the number of rows you ask for.

Each of the 235 registered operations belongs to exactly one class. A Ref number is the `NNN` suffix of an ID such as `DATA-AGE-GRAPH-HAS-NODE-025`. Look up the full entry in [postgres.md](./postgres.md), [pgvector.md](./pgvector.md), or [age.md](./age.md).

## Which class is an operation in?

Walk the questions from top to bottom and stop at the first yes.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
    q1{"Changes schema or builds an index?"} -- "yes" --> ddl["DDL and index builds"]
    q1 -- "no" --> q2{"Changes session settings?"}
    q2 -- "yes" --> sess["Session settings"]
    q2 -- "no" --> q3{"Uses the AGE graph?"}
    q3 -- "yes" --> graphNode["Graph (AGE)"]
    q3 -- "no" --> q4{"Nearest-neighbor vector search?"}
    q4 -- "yes" --> vec["Vector nearest-neighbor search"]
    q4 -- "no" --> q5{"Legacy eq_*_kv table?"}
    q5 -- "yes" --> legacy["Legacy key-value adapter"]
    q5 -- "no" --> q6{"Queue table tasks?"}
    q6 -- "yes" --> tasks["Tasks"]
    q6 -- "no" --> ord["Ordinary indexed SQL"]
%% eq-classes
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class q3,graphNode,q5,q6 eqStore
```

Each operation belongs to the first question it answers yes to, so ask them in order.

## Classes

| Class | Operations | Time | Space | Key limits | Failure modes |
|---|---|---|---|---|---|
| **Vector nearest-neighbor search** | 3 | O(ef * log N) expected; O(N) worst case if the planner picks a sequential scan | O(K + ef) | `hnsw.ef_search` = clamp(4 * K, 40, 1000). Filtered search uses iterative scan (`max_scan_tuples` = 20000). | Timeout, fewer than K results, silent recall loss, sequential-scan cliff |
| **Ordinary indexed SQL** | 115 | O(log N) with an index; O(N) without | O(K) | Use keyset pagination, not large OFFSET. Keep `statement_timeout` set. | Timeout, lock wait, out of memory on an unbounded SELECT |
| **DDL and index builds** | 30 | O(N log N) for an HNSW build; O(N) for a btree | `maintenance_work_mem` | Never REINDEX on the request path. Use `max_parallel_maintenance_workers` for HNSW. Use pgvector 0.8.2 or later. | ACCESS EXCLUSIVE lock contention, out of memory during build |
| **Session settings** | 4 | O(1) | O(1) | Use `SET LOCAL` only inside short transactions. Never leak settings across pooled connections. | A leaked setting changes recall or plans for the next user of the connection |
| **Graph (AGE)** | 51 | O(K log N) for a batch by ID; O(branch^depth) for an expansion; a full scan (O(N)) is forbidden on the request path | O(K) or O(branch^depth) | Prefer native writes (unique `node_id`). Cypher MERGE is for fallback and debugging. | Cartesian expansion, timeout, out of memory on an unbounded MATCH |
| **Legacy key-value adapter** | 18 | O(log N) for one key; O(K log N) for a batch; O(M) for a prefix scan | O(K) or O(M) | Upsert batches of at most 1000. PostgreSQL allows 65535 bind parameters per statement. | Timeout on a large prefix scan; slow COUNT on a large table |
| **Tasks** | 14 | O(log N) for a get; claim is O(W + log N) plus a row lock | O(1) for a claim; O(page) for a list | `SKIP LOCKED` for concurrency. A lease lets another worker reclaim an abandoned task. | Starvation without the fair claim; lease expiry race |

## Ref numbers per class

| Class | Ref numbers |
|---|---|
| **Vector nearest-neighbor search** | 001 to 002, 017 |
| **Ordinary indexed SQL** | 003 to 016, 093 to 130, 145 to 156, 158 to 194, 198 to 206, 209 to 213 |
| **DDL and index builds** | 018 to 021, 023 to 024, 207 to 208, 214 to 235 |
| **Session settings** | 022, 195 to 197 |
| **Graph (AGE)** | 025 to 074, 157 |
| **Legacy key-value adapter** | 075 to 092 |
| **Tasks** | 131 to 144 |

The counts add up to 235: 3 + 115 + 30 + 4 + 51 + 18 + 14.

## Reading notes

- The vector classes describe the original SPEC-088 view of the adapter. In the default typed mode, the search also runs inside a bounded transaction with a statement timeout. See [pgvector.md](./pgvector.md#how-a-vector-search-runs).
- The generic key-value relation is no longer created at runtime (SPEC-091 Wave D), and migration 125 dropped the `eq_*_kv` tables.
- The DDL class includes the `SCHEMA` entries for migrations. Every migration is applied by `edgequake migrate`, which takes the locks listed in [postgres.md](./postgres.md#schema-and-migrations).
- "Forbidden" in the graph class means the request path must not call the operation. Admin tools and tests may.
