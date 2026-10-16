# Article 017: Plain Postgres GraphRAG (no Apache AGE required)

## Overview

EdgeQuake v0.35.0 runs GraphRAG on ordinary PostgreSQL tables. Apache AGE is
optional. That unlocks hosted Postgres that ships pgvector (RDS, Cloud SQL,
AlloyDB) without a custom AGE image.

## Files

- **WHY-AGE-IS-OPTIONAL.md** (canonical measured report):
  [`specs/164-pure-postgres-study/reports/WHY-AGE-IS-OPTIONAL.md`](../../specs/164-pure-postgres-study/reports/WHY-AGE-IS-OPTIONAL.md)
- Spec index and install steps:
  [`specs/164-pure-postgres-study/README.md`](../../specs/164-pure-postgres-study/README.md)
- Upgrade guide:
  [`docs/operations/upgrade-to-0.35.0.md`](../../docs/operations/upgrade-to-0.35.0.md)

## Key messages

1. **GraphRAG is SQL hops, not Cypher.** EdgeQuake already ran native SQL against
   AGE heaps. Plain adjacency is the same algorithm without the extension.
2. **Measured, not hoped.** At 200k nodes the worst plain/AGE ratio was 1.087×.
   Upserts were 3.5× faster on plain tables. One production close path was
   212× faster off AGE parent heaps.
3. **Hosting gate flips.** Managed Postgres with pgvector and without AGE can
   serve EdgeQuake. Fresh install: `edgequake migrate` then serve.
4. **Drop is a question.** Existing databases keep AGE until an operator answers
   yes (`--drop-age`). The default is leave it.

## Numbers to cite (do not invent)

| Claim | Number | Source |
|-------|--------|--------|
| Worst SCALE-M plain/AGE | 1.087 | SPEC-164 decision |
| Upsert plain/AGE | 0.286 | SPEC-164 decision |
| BFS close AGE vs plain | 165.302 ms vs 0.779 ms | inventory OP-164-09 |
| Fresh install without AGE | schema 206, 203 steps | e2e_spec164_age_cutover |

## Suggested outline for a public post

1. WHY — GraphRAG locked to a custom Postgres build because of AGE.
2. WHAT — Plain `eq_graph_*` tables with the same hop caps.
3. HOW — Migration 170 expand; optional 171 drop; indexes 172–206.
4. PROOF — Cite the table above; link the report.
5. UNLOCK — RDS / Cloud SQL / AlloyDB as first-class hosts.
