# Round-Trip Amplification

The single most important number in this audit: **how many Postgres round trips it
takes to ingest one document**, today vs. achievable.

## The accounting (today)

Per the cost model ([`001-methodology/003-complexity-model.md`](../001-methodology/003-complexity-model.md)):

| Unit                 | Source                                                                                             | RT each | Formula |
| -------------------- | -------------------------------------------------------------------------------------------------- | ------- | ------- |
| chunk → vector       | [vector.rs#L540](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L540) | 1       | `C`     |
| entity → graph       | `get_node`+`upsert_node`, each 3 RT (F2)                                                           | 6       | `6·Ne`  |
| relationship → graph | `ensure_node_exists`×2 (`get_node`) + `upsert_edge`(3 stmts), each 3 RT                            | 15      | `15·Re` |

**Total round trips per document ≈ `C + 6·Ne + 15·Re`.**

### Three representative workloads

| Document                | C   | Ne  | Re  | Round trips          | @1 ms/RT | @3 ms/RT |
| ----------------------- | --- | --- | --- | -------------------- | -------- | -------- |
| Short note              | 2   | 3   | 3   | `2+18+45 = 65`       | 65 ms    | 195 ms   |
| Dense page              | 8   | 10  | 15  | `8+60+225 = 293`     | 293 ms   | 879 ms   |
| Long section (10 pages) | 80  | 100 | 150 | `80+600+2250 = 2930` | 2.9 s    | 8.8 s    |

These are **serialized DB latency only** — they exclude LLM extraction and embedding.
For a 100-page document: `~29,300` round trips, i.e. **~30 s–90 s of pure DB latency**
on top of model time. This is why ingestion feels slow on rich documents (5‑WHY Chain A).

## The achievable target (batched)

| Unit                                                | Batched form                                                                      | Round trips               |
| --------------------------------------------------- | --------------------------------------------------------------------------------- | ------------------------- |
| all chunks                                          | `INSERT … VALUES (…),(…),…` or `UNNEST` arrays                                    | 1                         |
| all entities                                        | `UNWIND $rows AS r MERGE (n:Node {node_id:r.id}) SET n += r.props`                | 1                         |
| all relationships                                   | `UNWIND $edges AS e MERGE (a) MERGE (b) MERGE (a)-[r:EDGE]->(b) SET r += e.props` | 1                         |
| + AGE session amortized to `after_connect` (F2 fix) | —                                                                                 | removes the ×3 multiplier |

**Total per document → 3 round trips** (one per store/phase), independent of `C`, `Ne`,
`Re`.

| Document     | Today RT | Batched RT | Reduction   |
| ------------ | -------- | ---------- | ----------- |
| Dense page   | 293      | 3          | **~98×**    |
| 100-page doc | 29,300   | 3          | **~9,800×** |

## Why this is the highest-leverage fix

- It is **pure latency**, not compute — the database can already do the work in one
  statement; the code just doesn't ask it to.
- It removes the dependence of ingestion time on entity/relationship density, which is
  precisely the dimension that grows for the high-value, content-rich documents.
- It dovetails with **F4** (transactions): routing a document's writes through one
  `UNWIND`-based transaction gives both batching *and* atomicity in one change.

Remediation: [`007-improvements/002-structural-changes.md`](../007-improvements/002-structural-changes.md).
