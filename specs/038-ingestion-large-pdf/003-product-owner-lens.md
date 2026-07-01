# SPEC-038 — Product Owner Lens

**Lens:** Product Owner  
**Persona:** Researcher ingesting arXiv surveys and technical guides (100–800 pages)  
**Evidence:** Reproducer failure + mission/04-heavy-pdf.md prior art

---

## Problem Statement

Users cannot reliably ingest **large born-digital PDFs** (textbooks, arXiv surveys, standards documents).  
EdgeQuake advertises PDF upload and knowledge-graph extraction, but **603-page documents fail silently or after hours of wasted Vision processing**.

This blocks the core value proposition for **research and enterprise knowledge bases** where large PDFs are the norm, not the exception.

---

## User Stories

### US-038-01 — Born-Digital Fast Path

> **As a** researcher uploading an arXiv PDF  
> **I want** the system to detect embedded text and parse without Vision OCR  
> **So that** my 600-page guide indexes in minutes, not hours

| AC | Given | When | Then |
| -- | ----- | ---- | ---- |
| AC1 | PDF with text layer (reproducer) | Upload completes | Backend = EdgeParse; no vision LLM calls for conversion |
| AC2 | Same PDF | Processing completes | `extraction_method` = EdgeParse in metadata |
| AC3 | Scanned PDF (no text) | Upload completes | Backend = Vision (or user override) |

### US-038-02 — Honest Time Estimate

> **As a** user uploading a 500+ page document  
> **I want** to see estimated processing time before I commit  
> **So that** I can choose to split, change model, or wait intentionally

| AC | Given | When | Then |
| -- | ----- | ---- | ---- |
| AC4 | PDF with 603 pages detected at upload | Upload dialog confirms | Shows "~X min (EdgeParse)" or "~Y min (Vision)" |
| AC5 | Estimate > 60 min | Pre-flight | Warning banner with mitigation tips |
| AC6 | User forces Vision on born-digital | Pre-flight | Explicit "slower" warning |

### US-038-03 — Progress Through Long Runs

> **As a** user waiting 30+ minutes  
> **I want** phase-accurate progress (pages, chunks)  
> **So that** I know the system is working, not stuck

| AC | Given | When | Then |
| -- | ----- | ---- | ---- |
| AC7 | 603-page PDF converting | WebSocket/poll | "Converting page N/603" or "Extracting chunk N/M" |
| AC8 | Processing > 15 min | UI | Elapsed time + ETA (updated) visible |
| AC9 | User cancels | Cancel clicked | Task stops; status = cancelled (not Failed) |

### US-038-04 — Actionable Failure

> **As a** user whose ingestion failed  
> **I want** a specific reason and next step  
> **So that** I don't blindly retry the same failing path

| AC | Given | When | Then |
| -- | ----- | ---- | ---- |
| AC10 | Worker timeout | Failed status | Message: timeout + which phase + suggested fix |
| AC11 | Circuit breaker | Failed status | Message: too many timeouts; suggests EdgeParse or split |
| AC12 | 10 MB text limit | Failed status | Message: document too large; split guidance |

### US-038-05 — Retry Without Wasted Work

> **As a** user retrying a failed large PDF  
> **I want** to resume from saved markdown/checkpoints  
> **So that** I don't re-pay Vision cost for 603 pages

| AC | Given | When | Then |
| -- | ----- | ---- | ---- |
| AC13 | Markdown stored from partial run | Retry | Skips Phase A (existing resume path) |
| AC14 | Vision checkpoint on disk | Retry | Continues from last page |

---

## Business Value

| Metric | Before | After |
| ------ | ------ | ----- |
| 603-page born-digital success rate | ~0% (timeout) | Target **≥95%** (EdgeParse path) |
| Time-to-index (reproducer) | Fails at 2 h+ | Target **<45 min** (mock LLM CI) |
| Wasted Vision API cost per retry | 603 × page calls | **0** for born-digital |
| Support tickets "PDF stuck/failed" | High for >100 pages | Reduced via ETA + routing |
| Trust in platform for enterprise docs | Low | Restored |

---

## Prioritization

| Priority | Item | Rationale |
| -------- | ---- | --------- |
| **P0** | Text probe → EdgeParse routing | Fixes reproducer; highest ROI |
| **P0** | Scaled worker timeout | Unblocks Phase B for large docs |
| **P1** | Pre-flight ETA UI | Prevents surprise 2 h waits |
| **P1** | Gold test on reproducer PDF | Regression lock (real test is law) |
| **P2** | Align 10 MB / 50 MB limits | Prevents future large OCR docs failing |
| **P2** | Graph merge batching (SPEC-016) | Secondary bottleneck post-extraction |

---

## KPIs

| KPI | Target | Measurement |
| --- | ------ | ----------- |
| Ingest success rate (pages ≥ 200, born-digital) | ≥ 95% | E2E + production metrics |
| P50 time-to-index (603-page reproducer, mock LLM) | < 30 min | CI benchmark |
| Vision calls for born-digital uploads | 0 | `extraction_method` audit |
| User-initiated retry rate on large PDFs | < 10% | Analytics |

---

## Out of Scope (This Spec)

- Automatic PDF splitting into volumes (future SPEC-039 candidate)
- Multi-document batch orchestration
- Cost billing / quota per page
