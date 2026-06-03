# PDF Metadata Enrichment Pipeline — Design Spec

**Date:** 2026-06-03  
**Status:** Approved  

---

## Overview

Before the full Graph-RAG pipeline runs, extract a lightweight metadata summary
(summary, topic, language, keywords) from the first 5 pages of a PDF using
EdgeParser (text mode) and a locally-hosted OpenAI-compatible VLM. The enrichment
runs async immediately after upload so users get a summary within seconds, before
full processing completes.

---

## Goals

- Summary available within seconds of upload (not after full pipeline)
- No dependency on cloud VLM — uses local OpenAI-compatible endpoint
- Enrichment failure never blocks the main pipeline
- Max 4 concurrent enrichment tasks (configurable)

---

## Architecture & Flow

```
POST /upload (PDF)
        │
        ├─→ HTTP response immediately (document_id, status: "processing")
        │
        ├─→ Enqueue: MetadataEnrich task   ← dedicated pool, 4 workers
        │       │
        │       ├─ EdgeParser: extract text from pages 1–max_pages
        │       ├─ Truncate to 8000 tokens
        │       ├─ POST text to local VLM (OpenAI-compatible chat API)
        │       ├─ Parse JSON response: {summary, topic, language, keywords}
        │       └─ Write enrichment fields to KV document metadata
        │
        └─→ Enqueue: PdfProcessing task    ← main pool (unchanged)
                Full chunking + graph extraction + embedding
```

Two separate worker pools. The enrichment pool (4 workers) handles only
`MetadataEnrich` tasks. The main pool is untouched.

Client polls `GET /api/v1/documents/{id}` to check `enrichment_status`.
A WebSocket progress event `enrichment_completed` is also emitted when done.

---

## New Components

### 1. `TaskType::MetadataEnrich`

Added to `edgequake-tasks/src/types/status.rs`:

```rust
pub enum TaskType {
    Upload, Insert, Scan, Reindex, PdfProcessing,
    MetadataEnrich,
}
```

Track ID prefix: `enrich-`.

### 2. `MetadataEnrichData`

New payload struct in `edgequake-tasks/src/types/data.rs`:

```rust
pub struct MetadataEnrichData {
    pub document_id: String,   // key for KV content lookup: "{document_id}-content"
    pub pdf_id: Option<Uuid>,  // set only when PDF went through pdf_storage path
    pub tenant_id: Uuid,
    pub workspace_id: Uuid,
    pub max_pages: usize,      // default 5
}
```

### 3. Enrichment Worker Pool

Separate `WorkerPool` started in `main.rs` alongside the existing pool.
Uses its own `ChannelTaskQueue`. Workers only receive `MetadataEnrich` tasks.

Configuration via environment variables:

| Variable | Default | Description |
|---|---|---|
| `ENRICHMENT_CONCURRENT` | `4` | Number of enrichment workers |
| `ENRICHMENT_VLM_BASE_URL` | `http://localhost:11434/v1` | Local VLM OpenAI-compatible base URL |
| `ENRICHMENT_VLM_MODEL` | `llava:7b` | Model name to use |
| `ENRICHMENT_MAX_PAGES` | `5` | Max pages to extract text from |

### 4. `MetadataEnrichProcessor`

New struct implementing `TaskProcessor` in `edgequake-api/src/tasks/`.

Steps:
1. Load PDF content from KV storage: key `{document_id}-content`
   (fallback: load binary via `pdf_id` from `pdf_storage` if `pdf_id` is set)
2. Run EdgeParser in text mode on pages `1..=max_pages`
3. If extracted text is empty → set `enrichment_status: "skipped"`, done
4. Truncate text to 8000 tokens
5. POST to local VLM with structured prompt (see below)
6. Parse JSON response
7. Write enrichment fields to KV document metadata
8. Emit WebSocket event `enrichment_completed`

---

## VLM Prompt

Fixed prompt, not customisable per request:

```
You are a document analyst. Given the text from the first pages of a document,
extract structured metadata. Respond ONLY with valid JSON, no markdown, no explanation:
{
  "summary": "2-3 paragraph summary written in the document's own language",
  "topic": "single short topic phrase",
  "language": "ISO 639-1 code (e.g. en, id, fr)",
  "keywords": ["up to 10 keywords"]
}

Document text:
<extracted text>
```

---

## Document Metadata Fields (KV Storage)

New fields written alongside existing metadata on the same `{document_id}-metadata` key:

```json
{
  "enrichment_status": "pending | processing | completed | failed | skipped",
  "enrichment_summary": "...",
  "enrichment_topic": "...",
  "enrichment_language": "id",
  "enrichment_keywords": ["keyword1", "keyword2"],
  "enrichment_completed_at": "2026-06-03T10:00:00Z",
  "enrichment_error": null
}
```

`enrichment_status` values:

| Value | Meaning |
|---|---|
| `pending` | Task enqueued, not started |
| `processing` | Worker is running |
| `completed` | Summary available |
| `failed` | All retries exhausted; see `enrichment_error` |
| `skipped` | PDF has no extractable text (scan-only) or is not a PDF |

Non-PDF uploads (txt, md, etc.) do not enqueue a `MetadataEnrich` task.
Their `enrichment_status` is omitted from the metadata response.

---

## Error Handling

| Condition | Action |
|---|---|
| VLM unreachable | Retry 3× with exponential backoff (1s, 2s, 4s); then `failed` |
| VLM returns invalid JSON | Retry once with stricter prompt; then `failed` |
| PDF text empty (scan-only) | `skipped` immediately, no retry |
| PDF not yet in storage when task starts | Retry with 2s delay, max 3× |
| Enrichment `failed` | Main pipeline is unaffected and continues normally |

Enrichment failure **never** blocks or fails the main `PdfProcessing` task.

---

## Upload Handler Changes

In `edgequake-api/src/handlers/documents/upload/` (both `file_upload.rs` and PDF path):

1. After storing initial metadata, detect if the file is a PDF
2. If PDF: enqueue `MetadataEnrich` task to the enrichment queue
3. Set `enrichment_status: "pending"` in initial metadata
4. Continue as today: enqueue `PdfProcessing` to main queue

---

## Out of Scope

- Enrichment for non-PDF formats (txt, md, images) — not in this spec
- User-configurable prompts per workspace
- Re-running enrichment on demand (can be added later)
- S3 source integration — separate spec
- OpenRouter / MCP provider integration — separate spec
