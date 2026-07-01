# SPEC-038 — First Principles

**Lens:** First Principles Decomposition  
**Method:** Strip assumptions; rebuild from irreducible truths  
**Anchors:** Reproducer measurements + live pipeline code

---

## P1 — What is "PDF ingestion"?

PDF ingestion is a **multi-phase state machine**, not a single operation:

```text
┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
│  ADMIT   │──►│ CONVERT  │──►│  CHUNK   │──►│ EXTRACT  │──►│ PERSIST  │
│ upload   │   │ PDF→MD   │   │ split    │   │ entities │   │ graph+vec│
└──────────┘   └──────────┘   └──────────┘   └──────────┘   └──────────┘
     │               │              │              │              │
   O(1)          O(?) pages     O(chars)      O(chunks)     O(entities)
```

**Truth:** Failure can occur at **any phase**. UX and timeouts must be **per-phase**, not one blob labelled "Processing".

---

## P2 — What is the irreducible cost of CONVERT?

| PDF class | Information source | Lower bound |
| --------- | ------------------ | ----------- |
| Born-digital | Embedded text layer + vector graphics | CPU parse: **O(pages)** |
| Scanned / photo | Pixels | Vision OCR: **O(pages × T_llm(page))** |

**Truth:** Using Vision on born-digital PDFs violates information theory — the text already exists.  
You are paying to **re-infer** what pdfium can **read**.

**Reproducer proof:** `pdftotext` → 1 443 139 bytes from 603 pages. Text layer is present.

---

## P3 — What is the irreducible cost of EXTRACT?

Entity extraction is **O(chunks)** LLM calls (with bounded parallelism):

```rust
// pipeline/extraction.rs — Semaphore::new(max_concurrent_extractions)
// default max_concurrent_extractions = 16 (pipeline/config.rs)
```

For PDF sources with `ChunkStrategy::Pdf`:

```rust
// ingestion_pipeline.rs:41–50 — chunks do not cross page boundaries
```

**Truth:** A 603-page doc produces **≥603 chunks** (often 1 per page).  
Extraction lower bound: `⌈603 / 16⌉ × T_median_chunk`.

---

## P4 — What is the irreducible cost of PERSIST?

Graph merge performs **sequential storage operations per entity and relationship** (SPEC-016 audit).

**Truth:** PERSIST can dominate EXTRACT for entity-dense documents.  
This is **O(entities + relationships)**, not O(chunks).

---

## P5 — What is a "timeout"?

A timeout is an **admission contract**: "this task will receive at most T seconds of wall clock."

Current contract:

```rust
// worker.rs:182 — T = 7200 s fixed
// safety_limits.rs:641 — vision outer T = 120 + pages × secs_per_page
```

**Truth:** Two independent timeout layers exist (worker vs vision wrapper).  
They are **not coordinated**. Vision may allow 4944 s while worker kills at 7200 s mid-Phase-B.

---

## P6 — What is "resume"?

Resume is **idempotent continuation** from the last durable checkpoint:

| Checkpoint | Storage | Enables |
| ---------- | ------- | ------- |
| Per-page vision | filesystem checkpoint_dir | Skip converted pages |
| Full markdown | `pdf_documents.markdown_content` | Skip entire Phase A |
| Partial extraction | chunk KV entries | Partial (not fully implemented) |

**Truth:** Resume only works if **durability happens before timeout**.  
Markdown must be flushed incrementally for large docs.

---

## P7 — What does the user actually need?

The user does not need "Vision" or "EdgeParse". The user needs:

1. **Correct text** in the knowledge graph  
2. **Predictable completion time**  
3. **Visible progress** during long runs  
4. **Actionable errors** when limits are hit  

**Truth:** Backend selection is an **implementation detail** hidden behind "PDF uploaded successfully".

---

## P8 — DRY / SRP decomposition

| Responsibility | Single owner (proposed) |
| -------------- | ----------------------- |
| Measure PDF (pages, bytes, text ratio) | `LargeDocumentProfile::from_pdf_bytes()` |
| Choose backend | `PdfRoutingPolicy::resolve(profile, user_override)` |
| Compute timeouts | `profile.task_timeout_secs(provider)` |
| Compute concurrency/DPI | `compute_safe_pdf_resource_profile` (exists — extend) |
| Surface ETA to UI | `IngestionEstimate` DTO from profile |
| Execute conversion | `pdf_processing.rs` (thin orchestrator) |

**Truth:** Today routing, timeout, concurrency, and UX are **scattered** across 4+ files with magic numbers.

---

## P9 — What is the "Upload" phase really?

The UI label **"Step 2: Uploading to server…"** hides a **synchronous HTTP admit pipeline**, not a simple file transfer:

```text
CLIENT                          SERVER (POST /api/v1/documents/pdf — blocks until 200)
──────                          ───────────────────────────────────────────────────
read File (10%)                 parse multipart → Vec<u8>     O(bytes) RAM
       │                        validate_pdf_data             O(bytes) scan
       │                        SHA-256 checksum              O(bytes) CPU
fetch ─┤  "40%" FAKE progress    find_pdf_by_checksum (DB)     O(1) query
       │                        create_pdf INSERT BYTEA        O(bytes) disk I/O
       │                        enqueue task + KV shell       O(1)
       └◄── JSON response ──────┘
```

**Measured on reproducer:** `guide_2606.24937v1-opt.pdf` = **11 043 120 bytes** (~10.5 MiB).

| Sub-step | Lower bound | Dominant risk at 11 MB |
| -------- | ----------- | ---------------------- |
| HTTP transfer (browser → API) | `bytes / bandwidth` | Slow Wi‑Fi; dev proxy buffering |
| Multipart buffer + `file_data.clone()` | **2× RAM** | Memory pressure on concurrent uploads |
| `calculate_pdf_checksum` | O(bytes) | Usually <1 s |
| `INSERT pdf_data BYTEA` | O(bytes) I/O | PostgreSQL write latency, pool wait |
| `get_workspace` + duplicate lookup | O(1) DB | Connection pool saturation |

**Truth:** The handler is **request–response synchronous**. The browser `fetch()` does not resolve until **all** sub-steps complete. The UI cannot advance to "Extracting" until then.

```typescript
// edgequake_webui/src/hooks/use-file-upload.ts:164–174
status: "uploading", progress: 40,  // ← NOT measured; set when fetch() starts
phase: "Step 2: Uploading to server..."
```

```rust
// edgequake-api/src/handlers/pdf_upload/upload.rs:605–616
pdf_data: file_data.clone(),  // second copy before INSERT
```

**Truth:** Progress **40% is a placeholder**, not bytes uploaded. Users perceive a **hang** when admit I/O exceeds ~30 s even though conversion has not started.

---

## P10 — Separate concerns: Transfer vs Admit vs Convert

| Concern | User mental model | Current implementation | Correct asymptotic class |
| ------- | ----------------- | ---------------------- | ------------------------ |
| **Transfer** | "Sending file to cloud" | Bundled inside one `fetch` | O(bytes) network |
| **Admit** | (invisible) | Sync in upload handler | O(bytes) CPU + DB |
| **Convert** | "Processing PDF" | Async worker Phase A | O(pages) EdgeParse or O(pages×LLM) Vision |

**Truth:** Conflating transfer + admit under one spinner **misattributes** slowness. A user stuck at "Uploading" may be waiting on:

1. Network upload of 11 MB (real transfer)  
2. PostgreSQL BYTEA write (server admit)  
3. Backend unreachable / proxy stall (no timeout on `apiClient` fetch)  

None of these are EdgeParse vs Vision — **conversion has not begun**.

**Rebuild rule:** Return **202 Accepted** with `track_id` after minimal validation + streaming store; move BYTEA persist + task enqueue to background **or** report **byte-level** `upload.onprogress` + server admit sub-status.

---

## Non-Goals

| Non-goal | Why |
| -------- | --- |
| Sub-second ingestion of 603-page PDFs | Physics — 603 LLM extractions cannot be instant |
| Unlimited document size | OOM and cost explosion remain real |
| Removing Vision entirely | Scanned PDFs still need OCR |
| Client-side-only PDF splitting | Server must own lineage and chunk IDs |

---

## Rebuild From First Principles

```text
  INPUT: pdf_bytes
      │
      ▼
  ┌─────────────────┐
  │ TRANSFER+ADMIT  │  ← REQ-038-11: decouple / show real progress
  │ (HTTP POST)     │     sync today: multipart + BYTEA + enqueue
  └────────┬────────┘
           │
           ▼
  ┌─────────────────┐
  │ PROFILE         │  pages, bytes, text_chars, image_ratio
  └────────┬────────┘
           │
           ▼
  ┌─────────────────┐     text_chars/page > THRESH
  │ ROUTE           │─────────────────────────────► EdgeParse
  └────────┬────────┘
           │ else
           ▼
        Vision (with checkpoint + scaled timeout)
           │
           ▼
  ┌─────────────────┐
  │ TIME BUDGET     │  T_task = f(profile, backend, provider)
  └────────┬────────┘
           │
           ▼
  EXTRACT → PERSIST (existing pipeline, profile-tuned chunk/gleaning)
```

---

## Principles → Requirements Mapping

| Principle | REQ |
| --------- | --- |
| P2 — don't OCR born-digital | REQ-038-01 |
| P5 — coordinated timeouts | REQ-038-02, REQ-038-03 |
| P7 — honest ETA | REQ-038-04, REQ-038-05 |
| P6 — durable resume | REQ-038-06 |
| P8 — DRY SSOT | REQ-038-02 |
| P9/P10 — honest upload/admit UX | REQ-038-11 |
| Real test is law | REQ-038-07 |
