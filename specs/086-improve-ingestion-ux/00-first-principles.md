# SPEC-086 — First Principles

> **Status**: Active  
> **Product pin**: EdgeQuake v0.21.1  
> **Cross-refs**: [README](README.md) · [Register](01-finding-register.md) · [Roadmap](03-implementation-roadmap.md) · [Contract pins](06-contract-pins.md)  
> **Inherits**: [SPEC-048 FP-01…FP-10](../048-improve-ux/001-five-whys-first-principles.md) · [SPEC-057 cancel/fairness](../057-pipeline-reliability/) · [068 track identity](../001-benchmark/001-edgquake-improvements/068-text-ingest-progress-parity.md) · [SPEC-084 LAW-9…14](../084-reliability-fix/00-first-principles.md) · [SPEC-085 LAW-15…21](../085-fix-security/00-first-principles.md)

---

## 1. WHY this pack exists

Uploading Markdown often shows **“Queued for processing…”** (sometimes with a green Done-ish marker) while PDF uploads show converting pages, stage chips, and live bars. Users conclude MD is broken or lower quality.

Markdown is **not** a different knowledge-graph pipeline after text exists. The failure is **two progress products** and **two visibility contracts**.

---

## 2. Five WHYs (anchored in observed UI)

**Symptom (2026-07-24):** MD file `auto_disco_*.md` stuck on “Queued for processing…” under Processing Files (Reading/Uploading/Extracting/Done legend with conflicting green dot); separately, Active run can show a full stepper once list data catches up. PDF of the same paper gets rich converting + extract feedback and far higher entity counts.

### WHY 1 — Why does MD feel stuck?

Because the upload-list panel for non-PDF files is seeded with **“Queued for processing…”** and often never advances that string, even when the worker is chunking.

### WHY 2 — Why doesn’t the string advance?

Because FE prefers the Zustand **store seed** over polled progress unless poll is terminal; poll does not merge stage/message into the store. A stale `useMemo` on `getTrack` can freeze the seed.

### WHY 3 — Why is PDF fine?

Because PDF uses a **different presenter** (`PdfUploadProgress`) wired to PDF poll + SSE + page WS events — it never depends on the ingestion-store seed path.

### WHY 4 — Why can ActiveRuns also look empty/queued for MD?

Because text/MD admit writes **`staging:{id}-metadata` only**. Progress API merges staging (068); **documents list / track / pipeline activity often do not** → list-driven UI lags or maps `pending` → Queued.

### WHY 5 — Why does quality feel worse?

Because (a) feedback poverty hides real work, and (b) entity counts are compared as absolute numbers across unequal content (full PDF vs short draft) without density/section metrics — while chunk strategy differs (Markdown headings vs PDF page markers).

**Root cause:** Progress is a distributed state machine (048). Format-agnostic UX requires **one presenter**, **one merge rule**, and **one in-flight visibility SSOT** — not a PDF luxury path and an MD poverty path.

---

## 3. Laws (SPEC-086)

Reuse LAW-1…LAW-21 from prior packs. SPEC-086 adds:

```
  LAW-22  One work item → one progress presenter (format is a detail, not a product fork)
  LAW-23  Progress SSOT = max(store, poll) by stage rank; terminal poll wins; seed never sticky
  LAW-24  In-flight visibility SSOT = staging-aware metadata load for progress AND list/track/activity
  LAW-25  Stage vocabulary ⊆ UnifiedStage; non-PDF skips converting (skipped ≠ idle)
  LAW-26  Stage transitions emit on WS for all Insert tracks; poll is mandatory fallback
  LAW-27  Quality parity = density + structure proxies on golden pairs, not raw entity equality
  LAW-28  Cancel/fairness/display_status remain SPEC-057 SSOT (this pack does not fork them)
```

### ASCII: causal stack

```
  Upload MD/PDF
       |
       +-- PDF --> final KV queued shell --> PdfUploadProgress (rich)
       |
       +-- MD  --> staging: pending ----------+
                                              |
                    +-------------------------+
                    |
                    v
         +----------------------+     +---------------------------+
         | FE store seed        |     | List omits staging        |
         | "Queued…" sticky     |     | ActiveRuns lag / Queued   |
         +----------+-----------+     +-------------+-------------+
                    |                               |
                    +---------------+---------------+
                                    |
                                    v
                         User: "MD is stuck / worse"
```

---

## 4. SOLID mapping (how we implement)

| Letter | Meaning here | Shared primitives (DRY) |
|--------|--------------|-------------------------|
| **S** | One finding owns one defect class | `findings/F-*.md` |
| **O** | New formats extend via `source_type` + skipped stages, not new presenters | UnifiedStage + stepper |
| **L** | MD/TXT/image Insert path substitutable for PDF-after-convert | shared `TaskType::Insert` |
| **I** | Narrow APIs: progress load vs completed-only list filters | one staging-aware helper, callers choose filter |
| **D** | UI depends on progress contract, not PDF-only channels | `IngestionProgressResponse` + WS stage events |

Anti-patterns banned:

- Second progress product for “text formats”  
- Preferring seed over advanced poll  
- Progress-only staging merge (068 residual)  
- Fake % to hide idle queues  
- Claiming quality parity via absolute entity counts alone  

---

## 5. Locked architectural decisions

See [README locked decisions](README.md#locked-decisions) and [06-contract-pins.md](06-contract-pins.md).

Industry alignment (not adopted wholesale): source-agnostic staged artifact → one process path → status channel that cannot be starved by workers (Rapidflare Temporal pattern; LlamaIndex format-agnostic stages). EdgeQuake already has UnifiedStage; this pack closes the **presenter + visibility** gap without Temporal.
