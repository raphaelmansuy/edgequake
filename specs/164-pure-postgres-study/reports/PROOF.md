# SPEC-164 — PROOF (honest bounds)

## What we actually ran

| Item | Value |
|------|-------|
| Command | `./specs/164-pure-postgres-study/measurements/run_bench.sh` |
| Container | `eqdoc-scratch` (`ghcr.io/raphaelmansuy/edgequake-postgres:latest`) |
| Postgres | 18.6 (Debian), aarch64 |
| AGE | 1.8.0 |
| pgvector | 0.8.5 |
| UTC start (successful full run) | 2026-10-11T08:26:30Z |
| Artifact | [`../measurements/results.json`](../measurements/results.json) |
| EXPLAIN samples | [`../measurements/explains/`](../measurements/explains/) |

## Scales completed

| Scale | Nodes | Edges (plain=AGE) | Load wall | Correctness |
|-------|------:|------------------:|----------:|-------------|
| S | 50_000 | 69_998 | 4 s | 50/50 |
| M | 200_000 | 499_998 | 24 s | 50/50 |
| Hub | 200_000 | 500_998 (+1000 hub) | 26 s | 50/50 |
| L | 1_000_000 | 1_999_999 | 158 s | 50/50 |

SCALE-L ran because SCALE-M suite wall was **38 s** (< 900 s budget).

## Decision rule outcome

`migrate_feasible` — all of H-164-01, H-164-02, H-164-05 pass; H-164-03/04 also pass.

## SCALE-M constant table (median ms)

| Op | ARM-A | ARM-B | B/A |
|----|------:|------:|----:|
| OP-164-01 | 0.582 | 0.425 | 0.730 |
| OP-164-02 | 0.655 | 0.558 | 0.852 |
| OP-164-03 | 1.494 | 1.184 | 0.793 |
| OP-164-04 | 3.533 | 2.450 | 0.693 |
| OP-164-05 | 1.725 | 1.875 | 1.087 |
| OP-164-06 | 1.112 | 1.033 | 0.929 |
| OP-164-07 | 1.754 | 1.462 | 0.834 |
| OP-164-08 | 13.476 | 3.856 | 0.286 |

## Hub path-set control

| Arm | OP-164-07 median ms |
|-----|--------------------:|
| ARM-C-level | 1.324 |
| ARM-C-path | 669.684 |
| Ratio path/level | ≈ 506 |

## Method bounds (do not overclaim)

1. Synthetic ring + hash edges; not a production RAG degree distribution. Hub arm is the stress control.  
2. Timings are **inside one `psql` session** via `clock_timestamp` / plpgsql loops — not the Axum/sqlx pool p95.  
3. AGE arm uses `eq_*` columns (product default), not Cypher MERGE (already non-default).  
4. No LLM, HNSW, or community detection in this suite.  
5. SPEC-034’s 69× Cypher figure is **not** the ARM-A baseline.

## Inventory extension (OP-164-09..14)

Command: `./measurements/run_inventory_bench.sh`  
Artifact: [`../measurements/inventory_results.json`](../measurements/inventory_results.json)

| Op | ARM-A ms | ARM-B ms | B/A |
|----|---------:|---------:|----:|
| 09 edges_for_node_set | 165.302 | 0.779 | 0.005 |
| 10 search LIKE | 0.763 | 0.883 | 1.157 |
| 11 popular+degree | 1892.838 | 1239.675 | 0.655 |
| 12 lineage GIN | 1.110 | 0.555 | 0.500 |
| 13 list OFFSET | 182.841 | 0.557 | 0.003 |
| 14 keyset scan | 0.682 | 0.570 | 0.836 |

H-164-06..08 **pass**. Amendment: **strengthen_migrate_feasible**.  
Largest AGE taxes are Q-GRAPH-006 (parent `_ag_label_edge` + agtype) and Q-GRAPH-020 (list with agtype tenant filter).

## Reproduce

```bash
export SPEC164_PG_CONTAINER=eqdoc-scratch
./specs/164-pure-postgres-study/measurements/run_bench.sh
./specs/164-pure-postgres-study/measurements/run_inventory_bench.sh
# or: make -C specs/164-pure-postgres-study proof
```
