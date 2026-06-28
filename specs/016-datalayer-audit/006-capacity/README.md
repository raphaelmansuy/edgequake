# 006 — Capacity & Scaling

How many documents/pages EdgeQuake can handle, and how performance evolves with growth.

## Documents

- [`001-limits-and-scaling.md`](001-limits-and-scaling.md) — RAM-bound capacity model, throughput ceiling, growth curves.

## Headline

Two independent ceilings, governed by different resources:

| Dimension                | Bound by                            | Practical ceiling (16–32 GB box)                            |
| ------------------------ | ----------------------------------- | ----------------------------------------------------------- |
| **Read (vector search)** | HNSW index fitting in RAM           | ~1–5 M vectors before recall/latency degrade                |
| **Write (ingestion)**    | DB round trips per document (F1–F3) | throughput, not storage — ~3 dense pages/s/connection today |

Storage capacity (disk) is effectively unbounded relative to these — the binding
constraints are **RAM for read** and **round-trip latency for write**.
