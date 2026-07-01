# SPEC-038 — O(n) Expert Lens

**Lens:** Complexity & Performance Engineering  
**Method:** Asymptotic analysis + measured constants + bottleneck identification  
**Reproducer:** 603 pages, 1.44 MB text, 11 MB PDF

---

## Variables

| Symbol | Meaning | Reproducer value |
| ------ | ------- | ---------------- |
| **P** | Page count | 603 |
| **B** | PDF file bytes | 11 × 10⁶ |
| **T** | Extracted text bytes | 1.44 × 10⁶ |
| **C** | Chunk count post-split | ≈ P (Pdf strategy) = 603 |
| **E** | Entities extracted | O(C) worst case; survey PDFs: 10³–10⁴ |
| **R** | Relationships | O(E) typical |
| **K** | max_concurrent_extractions | 16 |
| **J** | pdf_vision admission slots | 2 |
| **L** | Vision concurrency per job | 1–2 (603-page profile) |

---

## Phase Complexity Table

```text
┌─────────────┬──────────────────────────┬─────────────────┬──────────────────┐
│ Phase       │ EdgeParse path           │ Vision path     │ Dominant term    │
├─────────────┼──────────────────────────┼─────────────────┼──────────────────┤
│ ADMIT       │ O(B) hash + O(1) probe   │ same            │ O(B)             │
│ CONVERT     │ O(P) pdfium CPU          │ O(P/L)×T_llm    │ Vision: O(P×T)   │
│ CHUNK       │ O(T)                     │ O(T)            │ O(T)             │
│ EXTRACT     │ O(C/K)×T_chunk           │ same            │ O(C/K×T_chunk)   │
│ EMBED       │ O(C + E) batched         │ same            │ O(tokens)        │
│ MERGE       │ O(E + R) DB RTT each     │ same            │ O(E+R) ← killer  │
└─────────────┴──────────────────────────┴─────────────────┴──────────────────┘
```

**Key insight:** CONVERT asymptotic class **changes the game**.  
EdgeParse: **O(P)** with small constant (~50 ms/page).  
Vision: **O(P × T_llm)** with T_llm ∈ [8, 60] seconds.

---

## CONVERT — Detailed

### EdgeParse

```rust
// edgeparse.rs:51 — spawn_blocking + convert_bytes
// Amortized: ~30–100 ms/page for born-digital (measured class)
```

```text
T_edgeparse ≈ α × P
α ≈ 0.05–0.2 s/page  →  603 pages ≈ 30–120 s
```

**Memory:** O(B) pdf bytes in RAM once — 11 MB safe.

### Vision

```rust
// pdf_processing.rs:689–691 — tokio::time::timeout(vision_timeout, converter.convert)
// Inner parallelism: concurrency = 1–2 for 603 pages (local) or 2 (cloud)
```

```text
T_vision ≈ P/L × T_page
Cloud (L=2, T_page=8s):  603/2 × 8 ≈ 2412 s  (optimistic)
Cloud (L=2, T_page=15s): 603/2 × 15 ≈ 4522 s (realistic Mistral)
Outer cap: min(120 + P×8, 86400) = 4944 s for P=603
```

**Memory:** O(L × page_image_size). At dpi=120, A4 ≈ 1–3 MB/page PNG → **2–6 MB** per concurrent (safe).  
At dpi=150, P=603, concurrent=8 (old defaults): **OOM risk** (P-G13).

---

## CHUNK — Pdf Strategy

```rust
// ChunkStrategy::Pdf — splits at <!-- edgequake-page:N --> markers
// T ≈ 1.44 MB → chunk_size = 600 tokens (adaptive_chunking.rs:12-14)
```

```text
C ≥ P  (at least one chunk per page)
C ≤ T / min_chunk_chars  (upper bound ~ 3000 for this doc)
```

For reproducer: **C ≈ 603** (avg ~2400 chars/page ≈ one 600-token chunk).

**Complexity:** O(T) single pass — negligible vs EXTRACT.

---

## EXTRACT — Parallel LLM

```rust
// Semaphore(K=16), timeout 180s/chunk, retries 3
```

```text
Waves = ⌈C / K⌉ = ⌈603 / 16⌉ = 38
T_extract_best  = 38 × T_median     (T_median ≈ 15–30s mock/cloud)
T_extract_typ   ≈ 38 × 25s ≈ 950 s ≈ 16 min
T_extract_worst = C × 180s          (serial timeout disaster)
```

**With gleaning (max_gleaning=1):** multiply LLM calls ≈ **2×** for marginal quality gain on surveys.

**Recommendation for P>500:** `max_gleaning=0` default via profile.

---

## EMBED — Token Batching

```text
O(C + E) embedding calls, batched by token budget
```

**SPEC-011 EC-001:** Input **count** limit (512 inputs) not handled by token-only batching.

For survey PDFs with dense entity lists: **O(E)** can hit 400 permanent failure.

```text
Fix complexity: O(E) split into batches of ≤512 — already specified in SPEC-011
```

---

## MERGE — Graph Storage (Dominant Hidden Cost)

From SPEC-016 audit:

```text
Per entity:  get_node + upsert_node  → 2 DB round-trips
Per rel:     get_node×2 + upsert_edge → 3 DB round-trips

T_merge ≈ (2E + 3R) × RTT_db
```

If E=5000, RTT=5ms: **T_merge ≈ 50 s** (optimistic)  
If E=20000: **T_merge ≈ 200 s**  
Under load / AGE: **RTT → 50ms** → **T_merge ≈ 2000 s**

```text
MERGE is O(E+R) sequential — NOT parallelized today
This becomes the bottleneck AFTER CONVERT is fixed
```

---

## End-to-End Time Budget (Reproducer)

```text
                    EdgeParse              Vision (cloud realistic)
                    ─────────              ──────────────────────
CONVERT             60 s                   4522 s
CHUNK               2 s                    2 s
EXTRACT             950 s                  950 s
EMBED               120 s                  120 s
MERGE (E=5k)        200 s                  200 s
                    ─────                  ──────
TOTAL               ~1332 s (~22 min)      ~5794 s (~97 min)
WORKER CAP          7200 s  ✓              7200 s  ✗ (tight)
```

Vision at 7200 s worker cap: **fails** when merge + variance push over limit.  
EdgeParse: **comfortable margin**.

---

## Space Complexity

| Structure | Space | 603-page risk |
| --------- | ----- | ------------- |
| `pdf_data` in memory | O(B) | 11 MB |
| `markdown` string | O(T) | 1.4 MB |
| Chunk texts | O(T) | 1.4 MB |
| Extraction results | O(C × response_size) | 10–100 MB |
| Embedding vectors | O(C × dim) | 603 × 1536 × 4B ≈ 3.7 MB |
| Graph nodes | O(E) | unbounded |

**Peak RAM estimate:** 200–500 MB for reproducer — safe on 8 GB server.  
**OOM risk:** Vision with high dpi + high concurrency on **multiple simultaneous** large PDFs (P-G13).

---

## Admission Control Interactions

```text
pdf_vision semaphore (J=2):
  At most 2 vision jobs system-wide
  Each can run L concurrent pages

Worst vision RAM ≈ J × L × page_image ≈ 2 × 2 × 3 MB = 12 MB images
                  + 2 × 11 MB PDF buffers ≈ 34 MB  (manageable)
```

**Tenant fairness:** `max_tasks_per_tenant = 0.75 × workers` — large PDF monopolizes workers IO-bound.

---

## Optimization Priority (By Impact on P=603)

```text
 Impact │
   High │  [1] Route born-digital → EdgeParse     (−4300 s)
        │  [2] Scale worker timeout               (prevents false failure)
        │  [3] Disable gleaning for P>500       (−16 min extract)
        │  [4] Batch graph merge                  (−merge variance)
   Med  │  [5] Incremental markdown flush         (resume safety)
        │  [6] Embed input-count batching         (SPEC-011)
   Low  │  [7] Raise dpi only for small P         (already done)
        └──────────────────────────────────────────────────────► Effort
```

---

## Complexity Class Decision Matrix

| Document profile | Optimal CONVERT class | Reason |
| -------------- | --------------------- | ------ |
| P>200, text_chars/page>200 | **O(P) EdgeParse** | T_llm dominates otherwise |
| P>200, no text | **O(P×T_llm) Vision** | No cheaper signal source |
| P<50, mixed figures | Vision or Hybrid | Figures need OCR |
| B>50MB | Stream parse / split | Memory O(B) pressure |

---

## Benchmark Acceptance (CI)

| Benchmark | Assert |
| --------- | ------ |
| `bench_edgeparse_603_pages` | p99 < 180 s on CI runner |
| `bench_probe_text_layer` | < 2 s for 603 pages |
| `bench_profile_timeout` | monotonic in P |

Store results in `specs/038-ingestion-large-pdf/benchmarks/` (committed as JSON, not raw PDF).

---

## Formal Requirement

> **REQ-038-PERF:** For born-digital PDFs where `text_chars_per_page ≥ 200` and `P ≥ 100`, the system must not select a CONVERT algorithm worse than **O(P × c)** where `c < 1 s/page` amortized (EdgeParse class).

This is the O(n) expert's restatement of REQ-038-01.
