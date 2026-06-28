# 005 — Ingestion Pipeline

End-to-end document flow, from raw content to indexed vectors + graph.

## Documents

- [`001-pipeline-flow.md`](001-pipeline-flow.md) — the three stages and where time goes.
- [`002-batching-and-concurrency.md`](002-batching-and-concurrency.md) — what is parallel, what is serial, what is N+1.

## Summary verdict

The **front of the pipeline is well-engineered**: adaptive chunking, semaphore-bounded
parallel extraction (16), and token-aware embedding batching. The **back of the
pipeline (graph + vector persistence) is where the round-trip amplification lives**
(see [`004-mutations/`](../004-mutations/README.md)).

| Stage                                 | Quality |
| ------------------------------------- | ------- |
| Chunking (adaptive 600–1200 tok)      | ✅       |
| Extraction (parallel, semaphore=16)   | ✅       |
| Embedding (token-aware batches)       | ✅       |
| Graph merge (sequential, chatty, N+1) | 🔴 F3    |
| Vector persist (one-by-one)           | 🔴 F1    |
| Batch ingest (sequential docs)        | 🟡 F10   |
| Atomicity (no transaction)            | 🔴 F4    |
