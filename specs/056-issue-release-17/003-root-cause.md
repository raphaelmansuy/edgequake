# Root cause & fix sketch — Issue #300

## Causal model

```text
Browser generates track_id = "ui_track_…"
        │
        ▼
POST /documents/pdf  (multipart track_id=ui_track_…)
        │
        ├─ enqueue.track_id = "pdf-<uuid>"     ← worker / WS / metadata use this
        ├─ start_pdf_progress("ui_track_…")    ← empty skeleton created here
        └─ response.track_id = "ui_track_…"    ← UI subscribes to this
                    response.task_id = "pdf-…"
        │
        ▼
PipelineProgressCallback { task_id: "pdf-…" }
        │
        └─ start_pdf_phase / page updates / complete_*("pdf-…")
                │
                ▼
        UI polls / WS on "ui_track_…"  → forever pending
        Docs list reads metadata track_id "pdf-…" → looks alive
```

## Code anchors (v0.17.0 tree)

1. **Response returns client id only** — `upload.rs` sets `track_id: options.track_id` while `task_id: enqueue.track_id`.
2. **Upload seeds progress on client id** — `start_pdf_progress(&effective_track_id, …)` with client preference.
3. **Worker updates server id only** — `pipeline_progress_callback.rs` always clones `self.task_id`.
4. **WebUI binds to response.track_id** — `performFileUpload` → `startTracking(uploadResult.track_id)`.

## Why Vision Ingest + Mistral still “works”

Vision conversion + entity extraction against `mistral-small-latest` / `mistral-embed` succeeded end-to-end on published `0.17.0` images. The regression users feel is **observability / UX correlation**, not “Mistral cannot ingest.”

## Recommended fix (SSOT)

Pick one correlation id and use it everywhere:

**Preferred:** treat client `track_id` as the canonical id when provided; otherwise use `pdf-<uuid>`.

1. Persist **one** `track_id` on the queued task / document shell.
2. Make `PipelineProgressCallback` update that same id (not a second server-only id).
3. Return the same value in both `track_id` and (if kept) `task_id`, or document `task_id` as an alias and dual-write progress until clients migrate.
4. Add a regression test:

```text
given upload with track_id=T
when conversion emits page progress
then GET /documents/pdf/progress/T shows active pdf_conversion
and WS events carry track_id=T
```

**Defence in depth for WebUI:** if `task_id` differs from `track_id`, subscribe to **both** (or prefer `task_id` when progress on `track_id` stays at 0). Backend SSOT is the real fix.

## Workarounds for operators (until patched)

1. Ignore the infinite “Waiting for Upload” progress row; watch document list `current_stage` / `stage_message`.
2. Poll progress with **`task_id`** from the upload response, not `track_id`.
3. For API-only clients, omit client `track_id` and use returned `task_id`.
4. Ensure PDFs go to `/api/v1/documents/pdf`, not `/documents/upload`.

## Out of scope / follow-ups

- Empty-env / model-resolution quirk: health LLM `mistral-medium-3-5` despite `EDGEQUAKE_LLM_MODEL=mistral-small-latest`.
- Migration 086 applied-count vs latest_version on fresh PG18 image.
- Pixtral Large deprecation ([docs](https://docs.mistral.ai/models/model-cards/pixtral-large-24-11)): prefer `mistral-small-latest` / Medium 3.5 for vision going forward.
