---
title: "EdgeQuake product limits"
---

# EdgeQuake product limits

**Who this is for:** operators sizing a deployment  
**Rule:** Do not promise a capacity number unless it appears below with evidence.  
**Claim gates (lab):** `make ceiling-proof` / `make product-limits-check` — not day-2 sizing.

Related: [FAQ](faq.md) · [Performance tuning](operations/performance-tuning.md) · [SPEC-066](../specs/066-ceiling-proof/e2e/artifacts/RUN_NOTES.md) · [SPEC-067](../specs/067-ops-real-floors/000-index.md) · [SPEC-068](../specs/068-recall-quality-scale/e2e/artifacts/RUN_NOTES.md) · [SPEC-069 dedicated](../specs/069-dedicated-midscale/e2e/artifacts/RUN_NOTES.md) · [SPEC-072 DiskANN Pareto](../specs/072-diskann-recall-pareto/e2e/artifacts/RUN_NOTES.md) · [SPEC-074 storage P0](../specs/074-storage-p0-hardening/000-index.md) · [SPEC-075 filtered recall](../specs/075-filtered-recall-gates/000-index.md) · [SPEC-076 precision](../specs/076-precision-reorder-rrf/000-index.md) · [SPEC-077 binary quantize](../specs/077-binary-quantize-bakeoff/000-index.md) · [SPEC-078 Filtered-DiskANN](../specs/078-filtered-diskann-labels/000-index.md) · [SPEC-079 mid-scale tips](../specs/079-midscale-quantize-labels/000-index.md) · [SPEC-080 tiny-slice](../specs/080-tiny-slice-exact/000-index.md) · [SPEC-081 serving view](../specs/081-serving-view-dual-ssot/000-index.md) · [SPEC-082 push-scale](../specs/082-push-scale-floors/000-index.md)

---

## Status words (read this first)

| Word | Meaning |
|------|---------|
| **Proven** | Measured under production-style stress (latency + concurrency as claimed) |
| **Supported** | Full claim gate green (filtered ANN Q1-d ∧ recall@20 ∧ concurrent) — safe to promise with the recipe below |
| **Opt-in** | Supported only with an explicit non-default recipe — **no silent flip** |
| **Not promoted** | Lab saw latency or partial wins; do **not** sell as a floor |

**Vectors ≠ documents.** Example: 50 000 chunks at ~100 chunks/doc ≈ **~500 documents**, not 50k docs.

**Promote metric:** **filtered** recall@20 under a workspace filter (`make filtered-recall-gate` / SPEC-075). Never promote from unfiltered-only demos.

---

## TL;DR — What you can promise

| Promise | Vectors @1536 | Status | Requires |
|---------|---------------|--------|----------|
| Comfortable default / laptop | ≤ **50k** | **Proven** | Host ≥16 GB; `shared_buffers` ≥2 GB |
| Product default filtered ANN | **100k** | **Supported** | **Wave-2** + residency (see below) |
| Larger dedicated ANN | **250k** | **Supported opt-in** | DiskANN recipe list≥800 (not default) |
| Wave-2 above 100k | 250k+ | **Not promoted** | Mid-scale wall (SPEC-068); single-spot ≠ concurrent floor |

Also proven separately: community Louvain gated at **50k** graph nodes; graph G1 degrees path at **100k** nodes (not a vector floor).

---

## Pick your size

| Goal | Vectors @1536 | Host RAM | Postgres residency | Recipe | Status |
|------|---------------|----------|--------------------|--------|--------|
| Laptop / default | ≤**50k** | ≥**16 GB** | `shared_buffers` ≥**2 GB** | defaults OK | **Proven** |
| Filtered ANN @100k | **100k** | ≥**32 GB** preferred | ≥**2 GB** (+ shm ≥4g in harness) | Wave-2 | **Supported** (default) |
| Dedicated DiskANN @250k | **250k** | ≥**32 GB** preferred | ≥**2 GB** (+ shm ≥4g) | Opt-in DiskANN (list≥800) | **Supported opt-in** (SPEC-082) |
| Wave-2 beyond 100k | 250k+ | lab-class | ≥**4 GB** class | Wave-2 HNSW | **Not promoted** |
| Graph entities (degrees) | — | — | — | — | **G1 100k nodes** proven (separate) |

### Ceiling evidence (authors / support)

| Field | Value | Artifact |
|-------|-------|----------|
| `highest_green_N` (Wave-2 default) | **100 000** | Shared+partial Wave-2 (SPEC-064/068) |
| `highest_green_N` (DiskANN opt-in) | **250 000** | Dedicated DiskANN q_list≥800 + HQ build ([SPEC-082](../specs/082-push-scale-floors/e2e/artifacts/RUN_NOTES.md)); **150 000** still green @q≥400 ([SPEC-072](../specs/072-diskann-recall-pareto/e2e/artifacts/RUN_NOTES.md)) |
| `first_fail_N` (Wave-2 HNSW) | **250 000** | Mid-scale wall (SPEC-068/069) — concurrent; single-spot @150k is not a floor raise |
| Dedicated HNSW concurrent | clients=16 fails from 100k | [SPEC-069](../specs/069-dedicated-midscale/e2e/artifacts/RUN_NOTES.md) |
| DiskANN opt-in | Full-gate @**250k** (list≥800, HQ build); @150k list≥400 still green | [SPEC-082](../specs/082-push-scale-floors/e2e/artifacts/RUN_NOTES.md) · [SPEC-072](../specs/072-diskann-recall-pareto/e2e/artifacts/RUN_NOTES.md) |
| L2 500 000 | Latency green on HNSW; recall cliff — **not promoted** | SPEC-066 L2 |
| Graph G1 | **100 000** nodes; degrees p95 ~12 ms | AGE 1.8 remasure |

Validate: `make product-limits-check` · claim gates: `make ceiling-proof` / `make recall-pareto` / `make diskann-recall-pareto` · turnkey: `make wave2-greenfield-env`

---

## What to set (copy-paste)

### 1) Default (≤50k) — Proven

No special vector flags. Keep the working set resident (`shared_buffers` ≥2 GB).

### 2) 100k Wave-2 — Supported default (greenfield only)

```bash
# Required for the supported 100k filtered-ANN shape — do NOT silent-flip existing DBs
export EDGEQUAKE_VECTOR_STORAGE=halfvec
export EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE=1
```

**Turnkey greenfield (SPEC-071)** — same flags, plus helper + warmup:

```bash
eval "$(make -s wave2-greenfield-env)"
# or: WAVE2_GREENFIELD=1 make backend-bg
./scripts/wave2_warmup.sh <workspace_uuid>
# or: POST /api/v1/admin/ann/warmup  {"workspace_ids":["<uuid>"]}
```

`/ready` with Wave-2 on checks **catalog** ANN presence (empty DB is ready; tables without HNSW → 503). It is not a plan-shape check. First filtered query also warms if warmup is skipped.

Dedicated `*_ws_*` tables = embedding **dimension** isolation only (SPEC-069) — **not** the turnkey 100k path.

### 3) Opt-in DiskANN @250k — Supported opt-in (not default)

Clears dedicated concurrent + recall full-gate @**250k** when **`diskann.query_search_list_size ≥ 800`**, **`diskann.query_rescore ≈ list/2`** (e.g. 400), and a higher-quality DiskANN build (SPEC-082: `num_neighbors=64`, `search_list_size=200`). **150 000** still clears the full-gate at **list≥400 / rescore≈200** on the default SBQ build ([SPEC-072](../specs/072-diskann-recall-pareto/e2e/artifacts/RUN_NOTES.md)). Helper: `diskann_optin_recipe_statements()` (ops/harness `SET LOCAL` only — not a silent boot default).

```bash
# Image (once): make postgres-image-build-pg18-vectorscale
# Runtime: EQ_POSTGRES_PROFILE=pg18-vectorscale  +  CREATE EXTENSION vectorscale CASCADE
# Index on dedicated *_ws_* table (vector, not halfvec):
#   CREATE INDEX … USING diskann (embedding vector_cosine_ops)
#     WITH (storage_layout='memory_optimized', num_neighbors=64, search_list_size=200);
# Query tip (required for 250k recall gate) — both GUCs:
#   SET diskann.query_search_list_size = 800;   -- ≥800 @250k; ≥400 still OK @150k
#   SET diskann.query_rescore = 400;            -- ≈ list/2
```

| Finding | Result |
|---------|--------|
| Wave-2 default | Still supported **100k** shared+partial |
| DiskANN opt-in floor | **`highest_green_N=250000`** (dedicated; SPEC-082) |
| 150k q_list≥400 | Full-gate green ([SPEC-072](../specs/072-diskann-recall-pareto/e2e/artifacts/RUN_NOTES.md)) |
| 250k q_list≥800 + HQ build | Full-gate green ([SPEC-082](../specs/082-push-scale-floors/e2e/artifacts/RUN_NOTES.md)) |
| Silent flip | **Forbidden** — vectorscale/DiskANN remain opt-in |

Claim gates: `make diskann-recall-pareto` · `make push-scale-ladder` · recipe smoke: `make diskann-rescore-smoke` (SPEC-074).

### Mid-scale wall (SPEC-068)

`make recall-pareto` measured Wave-2 at 150k/200k/250k × ef∈{80,160,240,400} plus a rebuild arm. **No concurrent full-gate green above 100k**. SPEC-082 Wave-2 filtered **single** spot @150k can look green — that does **not** raise the Wave-2 default floor. Do not promise mid-scale from latency-only / single-query cells.

**Postgres residency (ops-real floors):**

| Corpus | Min `shared_buffers` | Notes |
|--------|----------------------|-------|
| 50k / Mix | ≥2 GB | SPEC-061/065 |
| 100k Wave-2 | ≥2 GB (+ shm ≥4g in Docker) | Cold thrash at ~128 MB buffers |
| Lab L2 500k | ≥4 GB, `maintenance_work_mem` ≥2–8 GB | Claim gate only |

### Shared+partial vs dedicated tables (SPEC-069)

| Shape | What it is | Concurrent @100k (clients=16) | Use for |
|-------|------------|-------------------------------|---------|
| **Shared + Wave-2 partial HNSW** | Multi-WS table + `WHERE workspace_id=…` partial | **Supported** floor (SPEC-064/068) | Proven filtered ANN @100k |
| **Dedicated `*_ws_*` table** | One table per workspace; global HNSW on that table | **Worse** under stress (~3 s p95 @100k) | Per-WS embedding **dimension** isolation |

`make dedicated-midscale`: recall can look fine on dedicated HNSW, but concurrent fails from 100k@clients=16. **Do not** promise 150k from dedicated **HNSW**.

### Mix / hybrid honesty

Hybrid Mix (FTS+ANN) prod seed scales are still **≪** the ANN ladder (~5k vs 50k–100k). Mix is **not** a 100k/150k ANN floor.

---

## Optional tips (do not raise floors)

These improve quality or headroom. They are **not** required to claim the Supported floors above, and they do **not** unlock 250k+.

| Tip | Env / setting | Default | When to use |
|-----|---------------|---------|-------------|
| Concurrent headroom @100k | `EDGEQUAKE_HNSW_EF_SEARCH=240` | unset | Concurrent p95 skirts 500 ms (SPEC-068) |
| Filtered iterative_scan | `EDGEQUAKE_HNSW_ITERATIVE_SCAN` | `relaxed_order` on filtered | Underfill; unfiltered path leaves iterative_scan **off** |
| Bound iterative work | `EDGEQUAKE_HNSW_MAX_SCAN_TUPLES` | `20000` | Extreme selectivity |
| Memory for iterative scan | `EDGEQUAKE_HNSW_SCAN_MEM_MULTIPLIER` | `1` | If raising max_scan_tuples alone does not restore recall |
| ANN → exact reorder | `EDGEQUAKE_ANN_EXACT_REORDER=1` | **OFF** | Stricter order on candidate set (SPEC-076) |
| Reorder candidate pool | `EDGEQUAKE_ANN_REORDER_CANDIDATE_K` | `50` | With exact reorder; ≥ final `top_k` |
| Sparse FTS+ANN RRF | `EDGEQUAKE_SPARSE_FUSION=rrf` | weighted | Codes / names (SPEC-076); Mix/RRF ≠ ANN floor |
| Binary quantize + rerank | `EDGEQUAKE_BINARY_QUANTIZE=1` | **OFF** | Study only (SPEC-077); Hamming candidates → exact reorder — **not promoted** |
| Filtered-DiskANN labels | `EDGEQUAKE_FILTERED_DISKANN_LABELS=1` | **OFF** | Study only (SPEC-078); `labels smallint[]` + `&&` — **not promoted** |
| Tiny-slice exact (skip ANN bias) | `EDGEQUAKE_ANN_EXACT_MAX_ROWS` | `2000` | SPEC-080: skip Wave-2 `enable_seqscan=off` when workspace rows ≤ threshold |

**Filtered recall + iterative_scan (SPEC-075):** claim gate `make filtered-recall-gate` (soft-fails product floors; hang cliff hard-fails). 100k evidence: [SPEC-068](../specs/068-recall-quality-scale/e2e/artifacts/RUN_NOTES.md).

**Precision tips (SPEC-076):** claim gate `make precision-layers-gate` (contracts; `EQ_PRECISION_SMOKE=1` for DB smoke).

**Binary quantize bake-off (SPEC-077):** `make binary-quantize-bakeoff` — archives filtered recall vs Wave-2; does **not** raise floors or flip defaults.

**Filtered-DiskANN labels (SPEC-078):** `make filtered-diskann-labels-bakeoff` — archives Wave-2 vs post-filter DiskANN vs labels; does **not** raise floors or flip defaults.

**Mid-scale tip archive (SPEC-079):** `make midscale-quantize-labels` — B2+A6 @50k/100k; default decision **Not promoted** (no silent flip).

**Tiny-slice exact (SPEC-080):** `make tiny-slice-exact-gate` — planner honesty on small workspaces.

**Serving view (SPEC-081):** `make serving-view-check` — `eq_serving_chunk_presence` admin/debug; **not** RAG ANN SSOT.

**Push-scale ladder (SPEC-082):** `make push-scale-ladder` — A6 @150k/250k + Wave-2 filtered spot @150k + DiskANN primary full-gate @250k. **Decision:** DiskANN opt-in `highest_green_N→250000`; Wave-2 default stays 100k; A6 tip not default; silent flip forbidden.

### Binary quantize study (SPEC-077) — not default

pgvector `binary_quantize` + `bit_hamming_ops` HNSW with exact halfvec/vector rerank. Helpers: `build_binary_hnsw_index_sql` / `build_binary_rerank_select_sql`. Env: `EDGEQUAKE_BINARY_QUANTIZE` (off), `EDGEQUAKE_BINARY_CANDIDATE_K` (default 200). Wave-2 remains Supported @100k.

### Filtered-DiskANN labels study (SPEC-078) — not default

pgvectorscale Filtered-DiskANN: `labels smallint[]` in the DiskANN index + `labels && ARRAY[$ws]::smallint[]`. Helpers: `WorkspaceLabelMap`, `build_diskann_labels_index_sql`, `build_filtered_diskann_label_select_sql`. Env: `EDGEQUAKE_FILTERED_DISKANN_LABELS` (off). No product `labels` migration. Wave-2 @100k unchanged; dedicated DiskANN opt-in floor is **250k** (SPEC-082) — A6 tip still **not** default (soft-fail @250k labels).

### Tiny-slice exact (SPEC-080)

When filtered workspace row count ≤ `EDGEQUAKE_ANN_EXACT_MAX_ROWS` (default 2000), skip Wave-2 planner bias (`enable_seqscan=off`) so pgvector’s cost model can prefer exact search. Does not disable HNSW; floors unchanged.

### Serving view (SPEC-081) — admin/debug only

`eq_serving_chunk_presence(workspace_id)` lists relational chunks + `embedding_id` link. Optional `eq_serving_vector_presence(workspace_id, vectors_regclass)` joins a namespace vectors table. **Serving view ≠ RAG ANN SSOT** — do not route queries here; do not silent-unify stores.

---

## If queries are slow

| Symptom | Likely cause | What to do |
|---------|--------------|------------|
| Cold ~1.5 s @100k, warm ~50–70 ms | Default btree→exact on ~20% rows (**cold cliff**) | Wave-2 + residency; not “HNSW broken” |
| Concurrent p95 &gt;500 ms (Sort path) | Planner skipped partial HNSW | Wave-2 + SPEC-067 planner bias; raise buffers |
| Concurrent skirts 500 ms @100k (HNSW path) | Default ef depth under stress | Ops tip `EDGEQUAKE_HNSW_EF_SEARCH=240` (SPEC-068); not a floor raise |
| Concurrent multi-second on dedicated WS table | Hot-set isolation ≠ concurrent scale | Prefer Wave-2 shared+partial for 100k; see SPEC-069 |
| Latency green @250k but poor answers | HNSW recall cliff vs high-ef | Stay ≤100k; rebuild arm did not unlock promotion |
| Filtered top-k underfill / low recall | Post-filter ANN + low selectivity | Wave-2 + iterative_scan; raise `EDGEQUAKE_HNSW_MAX_SCAN_TUPLES` / `SCAN_MEM_MULTIPLIER` (SPEC-075) |
| First query after deploy slow | Partial HNSW created on first hot query | Warm with a filtered query; `/ready` checks ANN catalog when Wave-2 on |
| Upload rejected with `quota exceeded` | Workspace `max_documents` hit | Raise quota or delete docs |

---

## Hard caps (code)

| Cap | Default | Source |
|-----|---------|--------|
| Upload / body | **50 MiB** | `EDGEQUAKE_MAX_UPLOAD_BYTES` |
| Community / full-graph scan | **50 000** nodes | `DEFAULT_GRAPH_SCAN_THRESHOLD_NODES` |
| Graph API response | **500** nodes | `MAX_GRAPH_NODES` |
| Graph depth | **5** | `MAX_GRAPH_DEPTH` |
| Page size | **100** | `MAX_PAGE_SIZE` |
| Query chars | **10 000** | `MAX_QUERY_CHARS` |
| HNSW dim | vector ≤**2000**, halfvec ≤**4000** | `capabilities.rs` |
| Workspace `max_documents` | Fail-closed at upload / new PDF mint (committed + staging) | `document_quota` |

No workspace storage-GB quota.

---

## Do not

- Promise **documents ≈ vectors** without chunks-per-doc math.
- Treat cold ~1.5 s @100k default path as an ANN bug.
- Silent-flip existing databases to `halfvec`.
- Claim **500k / L2** from latency-only JSONL (recall can cliff).
- Claim **150k+** from dedicated **HNSW** single-query latency (SPEC-069 concurrent wall).
- Silent-flip to DiskANN / vectorscale / pg18-vectorscale on existing DBs.
- Treat **Mix/hybrid** seed scales as equal to the ANN ladder (Mix≪ANN today).
- Size from FAQ “8+ GB boots” alone — that is **minimum to start**, not proven 50k / supported 100k.
- Raise `ef_construction` expecting warm indexes to change without REINDEX.
- Run DiskANN @150k with default `query_search_list_size=100` or default `query_rescore=50` (recall fails — use list≥400 **and** rescore≈list/2).

---

## Glossary

| Term | Meaning |
|------|---------|
| **Wave-2** | `halfvec` + workspace partial HNSW + column filters (100k shape) |
| **Turnkey greenfield** | SPEC-071: `make wave2-greenfield-env` / `WAVE2_GREENFIELD=1` + ANN warmup (opt-in; no silent flip) |
| **Dedicated WS table** | Per-workspace `*_ws_*` vector table (HNSW: dimension isolation only; DiskANN opt-in: concurrent @150k) |
| **Opt-in DiskANN** | pg18-vectorscale + `USING diskann` + `query_search_list_size≥400` + `query_rescore≈list/2` (SPEC-072/074; not default) |
| **Q1-d** | Filtered ANN p95 &lt;500 ms SLO |
| **Cold cliff** | btree filter → exact distance; cold latency spike without residency/Wave-2 |
| **Residency** | Hot index pages fit in `shared_buffers` + OS page cache |
| **highest_green_N / first_fail_N** | Ceiling ladder fields (authors); operators: “supported up to / fails at” |
| **exact reorder** | SPEC-076 opt-in ANN→exact distance re-rank on a candidate set |
