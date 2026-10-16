# Why Apache AGE is optional for EdgeQuake GraphRAG

> **Product:** v0.35.0 · **Schema:** 206 · **Authority:** measured numbers only  
> **Study benches:** 2026-10-11 on PostgreSQL 18.6 / AGE 1.8.0 ([PROOF.md](PROOF.md))  
> **Product proof:** fresh install on `pgvector/pgvector:0.8.6-pg18-trixie` (no AGE package)

## Claim

EdgeQuake GraphRAG does not need the Apache AGE extension on the request path.
Plain adjacency tables (`eq_graph_nodes` / `eq_graph_edges`) match hop correctness and
beat or match AGE latency under the product hop caps (depth ≤ 3, neighbors ≤ 500).

## What GraphRAG actually runs

The product does not run Cypher on the query path. It runs batched SQL: point get,
incident edges, k-hop expand, degree, lineage GIN, label search, and upserts.
That inventory is [10-query-inventory.md](../10-query-inventory.md). AGE was already
a heap layout plus `agtype` tax, not a query language EdgeQuake depended on.

## Measured proof (study)

At SCALE-M (~200k nodes), every request-path op stayed within the 2× decision rule.
Worst plain/AGE ratio was **1.087**. Upserts were **0.286×** AGE (plain faster).
Neighbor sets matched **50/50** on S, M, Hub, and L.

The production shapes the first suite missed were worse for AGE:

| Op | AGE ms | Plain ms | Plain / AGE |
|----|-------:|---------:|------------:|
| BFS close on parent `_ag_label_edge` | 165.302 | 0.779 | 0.005 (~212× faster plain) |
| List OFFSET + tenant/workspace | 182.841 | 0.557 | 0.003 (~328× faster plain) |

Authority: [09-decision.md](../09-decision.md), [`inventory_results.json`](../measurements/inventory_results.json).

## What that unlocks

[SPEC-148](../../148-gcloud-hosting/) recorded that Cloud SQL and AlloyDB do not
ship Apache AGE. Amazon RDS for PostgreSQL is the same class: **pgvector** is
available; **age** is not.

With plain tables:

1. A greenfield EdgeQuake can `edgequake migrate` on Postgres that only has
   pgvector, pg_trgm, btree_gin, and uuid-ossp.
2. Migration 170 marks the cutover ledger `ready` with `age_absent` when AGE is
   not installed. Boot does not require `CREATE EXTENSION age`.
3. Dropping AGE on an existing database is a separate migrate question
   (`--drop-age`). The default is keep. `--confirm-drop` does not answer it.

## Product proof (v0.35 train)

On a throwaway `pgvector/pgvector:0.8.6-pg18-trixie` container where
`pg_available_extensions` has no `age` row:

- Non-interactive migrate applied **203** SAFE SCHEMA steps through **206**.
- Migration **171** stayed pending.
- `pg_extension` never listed `age`.
- Graph storage initialized, wrote nodes/edges, and searched by label.
- Contract: `fresh_install_serves_on_postgres_without_age` in
  `e2e_spec164_age_cutover.rs`.

Tenant-role follow-ups on a migrated corpus (this train):

| Path | Before RLS-safe indexes | After 203–206 |
|------|-------------------------|---------------|
| Chunk id lookup under tenant role | ~18 s | ~30 ms |
| Mix query “Who is Youcef A.” | statement timeout | ~1.9 s, HTTP 200 |

## What AGE still does in the release

The published `edgequake-postgres` image may still include the AGE library so an
existing AGE corpus can backfill into plain tables before an operator answers yes
to the drop. Leaving the extension installed after migrate is the default and is
required by the demo deploy gate (`\dx` must list `age` when the volume already
has it). Removing AGE is never automatic.
