# 004 — Mutations (Insert / Update / Delete)

The write path: how rows enter, change, and leave the data layer.

## Documents

- [`001-insert-update-delete.md`](001-insert-update-delete.md) — per-operation behaviour and correctness.
- [`002-roundtrip-amplification.md`](002-roundtrip-amplification.md) — the round-trip cost accounting (the headline problem).

## Summary verdict

**Deletes are clean** — set-based `WHERE id = ANY($1)` and indexed metadata deletes.
**Inserts/updates are the systemic weakness:** the write path is *round-trip bound*.

| Path          | Pattern                                      | Round trips | Finding |
| ------------- | -------------------------------------------- | ----------- | ------- |
| Vector upsert | one `INSERT … ON CONFLICT` per row in a loop | `O(N)`      | 🔴 F1    |
| Node upsert   | `get_node` + `MERGE` per entity, each ×3 RT  | `6·Ne`      | 🔴 F3    |
| Edge upsert   | 3 Cypher ops + N+1 `get_node`, each ×3 RT    | `15·Re`     | 🔴 F3    |
| Atomicity     | no transaction around a document's writes    | —           | 🔴 F4    |
| Delete        | `DELETE … WHERE id = ANY($1)`                | `O(1)`      | ✅       |

See [`002-roundtrip-amplification.md`](002-roundtrip-amplification.md) for the full
arithmetic and the batched target.
