# SPEC-164 — Cross-Reference Matrix (SSOT)

Every id is defined once here. Other docs link here; they do not redefine.

## Why chains

| Id | Title | Doc | Falsifier |
|----|-------|-----|-----------|
| WHY-164-01 | Why AGE was chosen | [00](00-five-whys.md#why-164-01) | H-164-01 / migrate feasible |
| WHY-164-02 | Why request path left Cypher | [00](00-five-whys.md#why-164-02) | Cypher on hot path (none today) |
| WHY-164-03 | Why 034 cannot answer 164 | [00](00-five-whys.md#why-164-03) | ARM-B measured |
| WHY-164-04 | Why managed Postgres blocked | [00](00-five-whys.md#why-164-04) | LAW-148-1 + migrate |
| WHY-164-05 | Why CTE ≠ automatic replacement | [00](00-five-whys.md#why-164-05) | H-164-03 / H-164-04 |

## Laws

| Id | Statement | Doc |
|----|-----------|-----|
| LAW-164-01 | Authority = indexed adjacency | [01](01-first-principles.md#law-164-01) |
| LAW-164-02 | Hop caps are invariants | [01](01-first-principles.md#law-164-02) |
| LAW-164-03 | Identity is text `node_id` | [01](01-first-principles.md#law-164-03) |
| LAW-164-04 | Tenant predicates sargable | [01](01-first-principles.md#law-164-04) |
| LAW-164-05 | Extension needs used algorithm | [01](01-first-principles.md#law-164-05) |
| LAW-164-06 | Estimates ≠ results | [01](01-first-principles.md#law-164-06) |

## Arms

| Id | Physical store | Query shape |
|----|----------------|-------------|
| ARM-A | AGE `"Node"` / `"EDGE"` + `eq_*` | Current native SQL |
| ARM-B | `spec164.nodes` / `spec164.edges` | Same iterative hop SQL |
| ARM-C-level | same as B | `WITH RECURSIVE` level-set (`UNION`) |
| ARM-C-path | same as B | `WITH RECURSIVE` path-set (`UNION ALL`) negative control |
| ARM-C-unroll | same as B | Fixed 2-hop join |

## Scales

| Id | Nodes | Edges | Notes |
|----|------:|------:|-------|
| SCALE-S | 50_000 | 70_000 | SPEC-034 order |
| SCALE-M | 200_000 | 500_000 | Decision scale |
| SCALE-HUB | 200_000 | 500_000 + hub | Degree-1000 hub edges |
| SCALE-L | 1_000_000 | 2_000_000 | Only if M fits budget |

## Operations

| Id | Op | Product locus | Q-GRAPH |
|----|----|---------------|---------|
| OP-164-01 | Point get by `node_id` | `pg_get_nodes_batch` | 001 |
| OP-164-02 | Incident edges batch F=1 | `pg_get_incident_edges_batch` | 003 |
| OP-164-03 | Incident edges batch F=50 | same | 003 |
| OP-164-04 | Incident edges batch F=200 | same (chunk size) | 003 |
| OP-164-05 | Degree batch (50 ids) | `pg_node_degrees_batch` | 002 |
| OP-164-06 | 2-hop expand cap 500 | `pg_bfs_expand` | 004 |
| OP-164-07 | 3-hop expand cap 500 | `pg_bfs_expand` | 004 |
| OP-164-08 | Batch upsert 200 nodes + 500 edges | native upsert | 012/013 |
| OP-164-09 | Edges for node set (BFS close) | `pg_get_edges_for_node_set` | **006** |
| OP-164-10 | Search nodes LIKE | `pg_search_nodes` | 008 |
| OP-164-11 | Popular nodes + degree | `pg_get_popular_nodes_with_degree` | 010 |
| OP-164-12 | Source-prefix lineage GIN | `pg_find_nodes_by_source_prefixes` | 017 |
| OP-164-13 | List nodes OFFSET page | `pg_list_nodes_filtered` | 020 |
| OP-164-14 | Keyset scan after | `pg_scan_nodes_after` | 022 |

Full shape list: [10-query-inventory.md](10-query-inventory.md).

## Hypotheses (pre-registered)

| Id | Claim | Decision impact |
|----|-------|-----------------|
| H-164-01 | ARM-B ≤ 2× ARM-A on OP-164-01..08 at SCALE-M | Migrate feasible if true |
| H-164-02 | Neighbor sets equal for 50 starts (A/B/level) | Correctness gate |
| H-164-03 | ARM-C-level within 2× of ARM-B iterative at M sparse | Prefer CTE shape if true |
| H-164-04 | ARM-C-path > 10× ARM-C-level on SCALE-HUB | Confirms negative control |
| H-164-05 | ARM-B upsert ≤ 2× ARM-A upsert at M | Full migrate vs reads-only |
| H-164-06 | ARM-B ≤ 2× ARM-A on OP-164-09..14 at SCALE-M | Strengthen / caveat |
| H-164-07 | OP-164-09 plain faster than AGE `_ag_label_edge` agtype | Expected AGE tax |
| H-164-08 | OP-164-12 lineage GIN plain within 2× AGE | Cascade migrate OK |

## Code loci

| Symbol | Path |
|--------|------|
| `pg_bfs_expand` | `edgequake/.../graph/query_ops/expand.rs` |
| `pg_get_incident_edges_batch` | `edgequake/.../graph/edges_ops.rs` |
| `native_graph_writes_enabled` | `edgequake/.../graph/mod.rs` |
| AGE boot fail-closed | `edgequake/.../postgres/connection.rs` |
| AGE graph name | `edgequake/.../namespace_tables.rs` |
| Dead fallback DDL | `edgequake/migrations/013_add_age_graph.sql` |
| Next-release plan | [12-v035-migration-plan.md](12-v035-migration-plan.md) EC-01..EC-30 |

## Prior ids (external)

| External | Meaning in 164 |
|----------|----------------|
| SPEC-034 RC-1 | Cypher GIN — historical; not ARM-A baseline |
| IMP-031-04 | Native BFS — ARM-A shape |
| LD-04 | Reopened |
| LAW-148-1 | Hosting constraint |
| SPEC-149 D4 | CQRS ≠ ARM-B |
