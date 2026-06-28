# Findings Register

Consolidated, severity-ranked register of all findings. Every row is grounded in code
(`file:line`) and links to its remediation.

## Severity legend

- 🔴 **Critical** — correctness/security risk or order-of-magnitude scaling blocker.
- 🟠 **High** — significant performance or security weakness; fix before scale.
- 🟡 **Medium** — meaningful inefficiency; fix opportunistically.
- 🟢 **Low** — minor / config.

## Register

| ID  | Title                                                       | Sev | Dimension          | Evidence (file:line)                                                                                                                                                                                                | Root cause (5-WHY)                                                    | Remediation                                                                                                                                                                                              |
| --- | ----------------------------------------------------------- | --- | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F1  | Per-row vector upsert loop                                  | 🔴   | Ingestion/Mutation | [vector.rs#L540](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L540)                                                                                                                  | API exposes single-row upsert; batch path never built → N round trips | [QW2 batched upsert](../007-improvements/001-quick-wins.md) · [edge cases](../007-improvements/004-edge-cases-and-mitigations.md#qw2)                                                                    |
| F2  | AGE session tax per query (LOAD+search_path+query = 3 RT)   | 🔴   | Query/Mutation     | [helpers.rs#L82](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers.rs#L82)                                                                                                            | Session state set per-call instead of per-connection                  | [QW1 amortize to after_connect](../007-improvements/001-quick-wins.md) · [edge cases](../007-improvements/004-edge-cases-and-mitigations.md#qw1)                                                         |
| F3  | Edge upsert = 3 statements + N+1 node lookups               | 🔴   | Mutation/Ingestion | [graph/mod.rs#L687](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L687); merger [relationship.rs#L84](../../../edgequake/crates/edgequake-pipeline/src/merger/relationship.rs#L84) | No `MERGE`-based upsert; merger fetches each node                     | [SC1 batched graph writes](../007-improvements/002-structural-changes.md) · [edge cases](../007-improvements/004-edge-cases-and-mitigations.md#sc1)                                                      |
| F4  | No transaction around multi-store document write            | 🔴   | Mutation           | [ingestion.rs#L143](../../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs#L143)                                                                                                                    | 3-stage insert with no atomic boundary                                | [SC2 transactions](../007-improvements/002-structural-changes.md) · [edge cases](../007-improvements/004-edge-cases-and-mitigations.md#sc2)                                                              |
| F5  | Chunk text stored inline in vector metadata (JSONB+GIN)     | 🟠   | Storage            | [ingestion.rs#L287](../../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs#L287)                                                                                                                    | Convenience of single-row payload                                     | [SC3 relocate text](../007-improvements/002-structural-changes.md) · [migration M2](../007-improvements/003-migration-plan.md) · [edge cases](../007-improvements/004-edge-cases-and-mitigations.md#sc3) |
| F6  | `ef_search` never tuned (default 40 regardless of k)        | 🟠   | Query              | [vector.rs#L488](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L488)                                                                                                                  | No per-query GUC; relies on server default                            | [QW3 ef_search/iterative](../007-improvements/001-quick-wins.md) · [edge cases](../007-improvements/004-edge-cases-and-mitigations.md#qw3)                                                               |
| F7  | Post-filter recall loss (filter after ANN)                  | 🟠   | Query              | [vector.rs#L660](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L660)                                                                                                                  | HNSW returns k, filter drops some → <k results                        | [QW3 iterative_scan](../007-improvements/001-quick-wins.md) · [edge cases](../007-improvements/004-edge-cases-and-mitigations.md#qw3)                                                                    |
| F8  | Cypher built by string interpolation + hand-rolled escaping | 🟠   | Security/Query     | [helpers.rs#L233](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers.rs#L233), [#L101](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers.rs#L101)        | No parameterized cypher(); `$$`-blind escaper                         | [S1 security hardening](../007-improvements/005-security-hardening.md)                                                                                                                                   |
| F9  | Unbounded `get_neighbors` `[*1..depth]` (no LIMIT)          | 🟠   | Query/Security     | [graph/mod.rs#L1116](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L1116)                                                                                                          | Traversal exposes raw variable-length path                            | [QW4 bound traversal](../007-improvements/001-quick-wins.md) · [edge cases](../007-improvements/004-edge-cases-and-mitigations.md#qw4)                                                                   |
| F10 | Sequential `insert_batch` (docs inserted one at a time)     | 🟡   | Ingestion          | [ingestion.rs#L332](../../../edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs#L332)                                                                                                                    | Loop over docs, no concurrency                                        | [SC5 concurrent batch](../007-improvements/002-structural-changes.md)                                                                                                                                    |
| F11 | `max_connections = 10` default                              | 🟡   | Ingestion/Query    | [config.rs](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/config.rs)                                                                                                                            | Conservative default caps concurrency                                 | [QW5 connection sizing](../007-improvements/001-quick-wins.md) · [edge cases](../007-improvements/004-edge-cases-and-mitigations.md#qw5)                                                                 |

## By dimension

- **Storage:** F5 (and indirectly F8 surface).
- **Query / query plan:** F2, F6, F7, F9, F11.
- **Insert / update / delete:** F1, F3, F4, F10, F11.
- **Ingestion pipeline:** F1, F3, F4, F10.

## Headline

Write path is **round-trip-bound** (F1, F2, F3, F4): a dense page serializes ~hundreds of
round trips today; batching + session amortization + transactions collapse that to a
small constant — see [capacity](../006-capacity/001-limits-and-scaling.md). Read path is
sound except recall tuning (F6/F7). Only security finding to actually fix is F8.

## Prioritized order

1. **F1, F2** (QW1, QW2) — biggest write speedup, lowest risk, no schema change.
2. **F6, F7** (QW3) — recall + latency, no schema change.
3. **F9, F11** (QW4, QW5) — DoS bound + concurrency headroom.
4. **F3, F4** (SC1, SC2) — structural write correctness + speed.
5. **F8** (S1) — security; parameterize Cypher.
6. **F5, F10** (SC3, SC5) — storage hygiene + ingestion throughput (requires migration).
