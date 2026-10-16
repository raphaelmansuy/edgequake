# SPEC-164 — Experiment Protocol (pre-registered)

> Written **before** timings. Hypotheses H-164-* are locked. Do not edit pass/fail after seeing numbers; only fill results.

## Environment

| Item | Value |
|------|-------|
| Postgres | EdgeQuake image PG18 + AGE 1.8.0 + pgvector |
| Default container | `eqdoc-scratch` |
| Scratch only | schema `spec164`, AGE graph `spec164_age` |
| Production graphs | **untouched** |
| Seed | `setseed(0.164)` for reproducible random starts |

## Arms

See [04-cross-ref-matrix.md](04-cross-ref-matrix.md). AGE arm uses `LOAD 'age'` and a dedicated graph with `"Node"` / `"EDGE"` plus `eq_*` columns and the same btree shape the product expects.

## Scales

| Id | Load |
|----|------|
| SCALE-S | 50k nodes, 70k edges, avg degree ~2.8 |
| SCALE-M | 200k nodes, 500k edges |
| SCALE-HUB | SCALE-M edges + one hub node with 1000 edges |
| SCALE-L | 1M / 2M **iff** SCALE-M wall clock for full suite < 15 minutes |

Graph generation: `generate_series`; edges `(i → i+1)` ring plus random long-range edges; single tenant `t164` / workspace `w164`.

## Ops and timing method

For each (scale, arm, op):

1. `DISCARD ALL` or new session for cold (once per op family).  
2. Warm: run once discarded, then **5** timed runs; report **median**.  
3. Capture one `EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)` per op to `measurements/explains/`.  
4. Wall clock via `clock_timestamp()` around the statement (or hop loop for iterative BFS).

Iterative expand (ARM-A/B) implements the product loop in SQL/plpgsql: for depth in 1..k, incident batch on frontier, accumulate visited until 500 nodes.

## Correctness (H-164-02)

Pick 50 `node_id`s with `ORDER BY md5(node_id || '164')`. For each, compute neighbor id set at depth 2, cap 500, for ARM-A, ARM-B, ARM-C-level. Pass = all three sets equal.

## Decision rule (repeat)

- **Migrate feasible** if H-164-01 ∧ H-164-02 ∧ H-164-05.  
- **Migrate reads only** if H-164-01 ∧ H-164-02 ∧ ¬H-164-05.  
- **Keep AGE** if ¬H-164-01 or ¬H-164-02.  
- H-164-03 / H-164-04 inform query shape, not storage keep/migrate.

Timeouts: any single op > 120s → record as fail for that arm/op; do not invent a number.

## Artifacts

| Path | Content |
|------|---------|
| `measurements/harness.sql` | DDL + load + functions |
| `measurements/run_bench.sh` | driver |
| `measurements/results.json` | medians, ratios, correctness |
| `measurements/explains/*.txt` | EXPLAIN text |

## Honest bounds

- Synthetic degree distribution ≠ production RAG graphs. Hub arm is the stress control.  
- No LLM, no vector ANN, no community detection.  
- Network is docker exec local socket — round-trip cost is in-process to the container’s psql, not app pool.  
- App Rust BFS issues multiple queries over a pool; harness plpgsql loop is same hop count, one session — closer to best case for iterative.
