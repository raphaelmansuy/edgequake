# 001 — Methodology

How every claim in this audit was derived. The rule is **code is law**: no claim
survives unless it can be re-derived from source at a cited `file:line`, or from the
upstream pgvector / Apache AGE source documented in
[`zz-reference/`](../../../zz-reference/README.md).

## Documents

- [`001-first-principles.md`](001-first-principles.md) — what the data layer *must*
  do, reduced to physical primitives (round trips, bytes, index probes).
- [`002-five-whys.md`](002-five-whys.md) — root-cause chains for the headline
  performance findings.
- [`003-complexity-model.md`](003-complexity-model.md) — the O(N) cost model and the
  per-operation round-trip accounting used throughout.

## Evidence standard

1. **Primary**: EdgeQuake Rust source (`edgequake/crates/.../postgres/*`).
2. **Secondary**: SQL migrations (`edgequake/migrations/*`).
3. **External grounding**: pgvector v0.8.2 C source and Apache AGE master, as captured
   in `zz-reference/` (HNSW defaults, `cypher()` semantics, agtype storage layout).

Claims that could not be grounded are explicitly marked _(inference)_.
