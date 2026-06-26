# 22 — P-G2 Post-Ship Brutal Assessment (Code Is Law)

> **Spec**: 021-storage-study
> **Date**: 2026-06-26
> **Commit**: `404ce915` — `feat(pipeline,api,core): P-G2 single IngestionPersister path (RC-7)`
> **Method**: Four-lens review (GraphRAG, LightRAG, AI Engineer, System Engineer) +
> First Principles + DRY/SOLID + flakiness audit. Brutal and honest.
> **Verdict**: RC-7 **closed**. P-G2b/c shipped 2026-06-26 — see **`23-pg2-gaps-closed.md`**.
> This file remains the **pre-closure forensic record**.

---

## 0. Executive summary

| Lens | Grade | One-line verdict |
|------|-------|------------------|
| **GraphRAG** | B− | One merge path restores KG invariants for new writes; saga still leaves partial graph/entity-vector orphans on mid-merge failure. |
| **LightRAG** | B | Merger is canonical (correct); caller config/metadata divergence breaks parity with LightRAG's uniform chunk lineage. |
| **AI Engineer** | C+ | Tests lock happy path only; misnamed "E2E"; duplicate fixtures; acceptance criteria from plan-19 §P-G2 not met. |
| **System Engineer** | B− | DRY win on persistence sequence; processor still warn-and-continue; no persist span/metrics; merger still O(E) round-trips. |

**Code is Law**: `persist_processing_result` exists at
`edgequake-pipeline/src/persistence/ingestion_persister.rs:88-156`. Both production
callers delegate (`orchestrator/ingestion.rs:316`, `processor/text_insert.rs:734`).
Manual `upsert_nodes_batch` / edge prefetch in `text_insert.rs` is **gone** (verified:
`rg upsert_nodes_batch text_insert.rs` → comments only).

---

## 1. GraphRAG lens — knowledge graph integrity

### What actually improved ✅

- **Single merge semantics**: Entity dedup, description merge, relationship wiring, and
  `EntityId`-derived vector IDs all flow through `KnowledgeGraphMerger` — the only
  algorithm that preserves "one node per real-world entity."
- **Re-persist idempotency**: `contract_ingestion_persistence.rs` proves double-persist
  does not fork `SARAH_CHEN` or duplicate chunk vectors — a core GraphRAG invariant.
- **Compensation on merge failure**: Chunk vectors written before a failed merge are
  deleted via `compensate_orphan_chunk_vectors` — shrinks unreachable embedding class.

### What is still broken / unproven ❌

- **Partial merge failure**: Merger counts per-entity errors (`stats.errors`) but may
  have already upserted some entity vectors + graph nodes before failing. Compensation
  rolls back **chunk vectors only**, not entity vectors or partial graph writes (RC-10 /
  P-G5 still open).
- **Cross-document merge untested**: Contract uses one document, one entity. No test that
  doc A + doc B both mention "Sarah Chen" → one node with unioned `source_chunk_ids` and
  combined degree at query time.
- **No community / summary layer**: GraphRAG hierarchical summarization is out of scope;
  persister does not touch it — fine, but do not claim GraphRAG-complete ingestion.
- **Legacy corruption**: P-G1b backfill is admin-gated. Graphs corrupted pre-P-G1 remain
  fragmented until an operator runs reconcile.

---

## 2. LightRAG lens — algorithm fidelity

### Aligned ✅

- **Merger-over-manual-batch**: LightRAG merges entities across chunks/documents; the
  deleted processor path bypassed merger and wrote raw batches — that anti-pattern is
  removed.
- **Normalized entity keys**: `EntityId::new` in merger matches LightRAG's uppercase
  underscore convention.
- **Chunk-then-graph ordering**: Vectors first, merge second, compensate on failure —
  matches the SC2 saga documented in orchestrator comments.

### Diverged ⚠️ (Code is Law — read the call sites)

| Knob | Orchestrator (`ingestion.rs:316-322`) | Processor (`text_insert.rs:723-742`) |
|------|----------------------------------------|----------------------------------------|
| `MergerConfig::use_llm_summarization` | From `self.config.use_llm_summarization` | **`MergerConfig::default()`** (implicit true if default) |
| `ChunkVectorBuildOptions` | **`default()` → no lineage fields** | **`include_lineage_metadata: true`** |
| Error policy | Fail insert (`map_err`) | Push to `storage_errors`, may still **complete** document |
| Relational sink | Wired from orchestrator | Wired from processor |

These are **not** LightRAG-parity bugs in the algorithm, but they **are** storage-state
divergence between library `insert()` and production upload — exactly the class of bug
RC-7 was meant to eliminate. P-G2 fixed the **sequence**, not **configuration SSOT**.

---

## 3. AI Engineer lens — tests, contracts, flakiness

### SOLID / DRY violations in the test layer

| Violation | Evidence | Severity |
|-----------|----------|----------|
| **DRY**: three copies of Sarah/Alice fixtures | `ingestion_persister.rs:183-211`, `contract_ingestion_persistence.rs:16-47`, `e2e_spec021_ingestion_persister.rs:20-42` | Medium — drift risk when `ProcessingResult` shape changes |
| **Misleading name**: `e2e_spec021_ingestion_persister.rs` | Uses `MemoryGraphStorage` / `MemoryVectorStorage` only; **no HTTP, no worker, no Postgres** | High — false confidence in CI green |
| **Acceptance gap**: plan-19 P-G2 original acceptance | "byte-identical storage state through three callers" — sync path removed (P-G2b); two callers differ on metadata + merger config | **Acceptance not met** |
| **No postgres contract** | All P-G2 tests use memory adapters | Medium — UNWIND batch semantics untested |
| **Hardcoded node IDs** | `assert!(graph.get_node("ALICE")` — breaks if normalization rules change | Low flakiness today; brittle |

### Flakiness inventory

| Test / area | Flaky? | Why |
|-------------|--------|-----|
| `contract_double_persist_*` | **No** | Deterministic memory stores |
| `spec021_persist_processing_result_*` | **No** | Same |
| `make test-spec021` TS readiness | **Mild** | `isBackendReady` retry test sleeps ~902ms; bounded, not parallel-hostile |
| Worker-backed upload tests (elsewhere) | **Yes under parallel** | Mitigated by `TEST_WORKER_GUARD` mutex — serializes, slows CI |
| Production persist path | **Untested** | No test runs `process_text_insert` → Postgres AGE + pgvector |

### What a honest test pyramid would add (P-G2c, not shipped)

1. **Integration**: `persist_processing_result` against `PostgresGraphStorage` feature.
2. **True E2E**: upload text via API → poll track → assert one `JOHN_DOE` node in graph inspector.
3. **Config parity contract**: assert orchestrator and processor build identical
   `IngestionPersistConfig` for the same workspace flags.

---

## 4. System Engineer lens — architecture, saga, performance

### DRY — what P-G2 actually unified

**In scope (one function body):**

1. Build chunk vector batch
2. Upsert chunk vectors
3. Construct merger + optional summarizer
4. `merger.merge(extractions)`
5. Compensate chunk vectors on merge failure

**Still duplicated outside persister (honest boundary):**

- KV chunk upsert, metadata, lineage, checkpoints, PDF phases, status transitions → processor only
- No KV / relational document row → orchestrator only
- Final `documents` stats + failure classification → processor only

Plan-19 marketed an **8-step** persister; shipped persister covers **~2 stores** (vector
chunks + graph merge). Calling this "complete IngestionPersister" in docs is **marketing,
not code**.

### SOLID scorecard

| Principle | Score | Notes |
|-----------|-------|-------|
| **SRP** | ✅ Good | Persister writes; processor orchestrates task lifecycle |
| **OCP** | ❌ Weak | Free function, not trait — cannot swap persist strategy without editing callers |
| **LSP** | n/a | No trait hierarchy |
| **ISP** | ✅ OK | Small config structs |
| **DIP** | ⚠️ Partial | Storage traits injected; config/build policy still in callers |

Plan-19 §2.2 envisioned `trait IngestionPersister` — **not delivered**. Delivered a
**module-level function** (pragmatic, but do not conflate with the design doc).

### Performance (RC-9 reframed)

Removing manual processor loops **eliminated** the worst N+1 in `text_insert.rs`
(stages 6–11 in audit file 18 §2.2 — **historical**, now wrong).

**Still true**: `KnowledgeGraphMerger::merge` loops entities sequentially with
**per-entity** `vector_storage.upsert(&[(one)])` and `get_node` + `upsert_node`
(`merger/entity.rs:38-64`). Complexity is now **O(E) inside merger**, not O(E) in
processor — same DB round-trips, better correctness, **not** batched.

Edge prefetch N+1 (`text_insert.rs:830-844` in audit 18) is **removed** with manual
edge batch — **closed as side effect of P-G2**.

### Saga / reliability

- Processor: persist `Err` → `storage_errors.push` → downstream logic may mark **completed**
  with zero entities (FIX-1/2 mitigations exist but policy is **not** fail-fast like orchestrator).
- Compensation: fire-and-forget `compensate_orphan_vectors` — no structured quarantine event
  emitted from persister itself (relies on compensation module logging).
- No distributed trace linking chunk upsert → merge → compensate.

---

## 5. Stale documentation debt (must fix when reading audit 18)

File **18** §1 and §2 describe **pre-P-G2** production (`raw entity.name`, three paths,
N+1 at `text_insert.rs:686-714`). That was accurate **before** `404ce915`. After P-G2:

- §1 RC-6 narrative about "production uses raw names" is **fixed for new writes** via merger.
- §2.1 diagram (three paths) → **two paths**, both delegate to persister; sync upload enqueues only.
- §2.2 stage table (14-step processor) → **obsolete**; persist is ~lines 720-763.

**Do not delete** audit 18 — it is the forensic record. Read **21 + 22** for current state.

---

## 6. Honest status vs plan-19 acceptance

| Criterion (plan-19 P-G2) | Met? | Reality |
|--------------------------|------|---------|
| One function body for persist sequence | ✅ | `persist_processing_result` |
| Three callers byte-identical | ❌ | Two callers; metadata + config differ |
| Trait-backed `IngestionPersister` | ❌ | Free function in pipeline crate |
| KV + relational in persister | ❌ | Explicitly out of scope (plan-21) |
| Contract test locks invariants | ⚠️ Partial | Single-doc dedup only |
| E2E through production API | ❌ | Memory-only misnamed test |

**Recommendation**: Rename P-G2 status to **"P-G2a — structural SSOT"** and open **P-G2b**
(config parity + true API E2E) and **P-G2c** (trait extraction / full 8-step) as separate
items. Do not mark RC-10/RC-9 closed because P-G2 shipped.

---

## 7. Prioritized follow-ups (brutal priority order)

1. **P-G2b-config** — Single `IngestionPersistConfig::for_workspace(...)` factory used by
   orchestrator + processor; eliminate summarization + lineage divergence.
2. **P-G5** — Extend compensation to entity vectors + partial graph on merge partial failure.
3. **P-G4-merger** — Batch entity vector upserts inside merger (RC-9 moves here).
4. **P-G2c-e2e** — Replace misnamed memory test with worker + postgres upload contract.
5. **P-G2d-trait** — Optional; lowest ROI until config parity proven.

---

## 8. Verification commands (2026-06-26)

```bash
make test-spec021                                          # green
cargo test -p edgequake-pipeline --test contract_ingestion_persistence
cargo test -p edgequake-api --test e2e_spec021_ingestion_persister
rg 'upsert_nodes_batch|upsert_edges_batch' edgequake/crates/edgequake-api/src/processor/text_insert.rs  # comments only
```

---

## 9. Bottom line

P-G2 is a **real, valuable refactor** — it removes the most embarrassing DRY violation
(two hand-rolled persistence implementations) and forces production through the merger.
That alone justifies shipping.

It is **not** honest to call the ingestion persistence problem **solved**. Call it
**"one canonical merge path; config and saga gaps remain."** Anyone reading green CI on
`e2e_spec021_ingestion_persister.rs` and inferring production Postgres correctness is
**misled by naming**. Fix the docs before fixing the code again.
