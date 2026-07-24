# SPEC-086 — Contract Pins

> Floors for progress identity, stages, visibility, and UI merge.  
> Inherit display_status / ui_phase from SPEC-057 — do not redefine.

---

## 1. Identity (from 054 / 068)

| Concern | Pin |
|---------|-----|
| Progress / cancel / WS key | Server `task_id` (`insert-*` / `pdf-*`) = metadata `track_id` |
| Client batch correlation | `client_track_id` only — never poll this for progress |
| Upload response | HTTP 202 + `document_id` + `task_id` / `track_id` |

---

## 2. Stage vocabulary

User-facing stages ⊆ `UnifiedStage` (+ admission aliases):

```
uploading → converting (PDF only) → preprocessing → chunking → extracting
→ gleaning → merging → summarizing → embedding → storing → completed|failed|cancelled
```

| Rule | Pin |
|------|-----|
| Non-PDF | `converting` **skipped** (not shown as active/grey idle) |
| Label | “Converting PDF” only when `source_type == pdf` |
| Skip source | `UnifiedStage::stages_for_source` + FE `stage-timeline.ts` |

---

## 3. Visibility

| API | Pin |
|-----|-----|
| `GET /ingestion/{track_id}/progress` | Staging-aware (068 FIXED) |
| `GET /documents` | Must include in-flight staging rows (or provisional final shell) — Wave 1 |
| `GET /documents/track/{track_id}` | Same staging-aware load — Wave 1 |
| `GET /pipeline/activity` | In-flight MD counted in working/queued honestly — Wave 1 |

Helper SSOT: extend `load_scoped_document_metadata_for_progress` (or rename to shared `load_scoped_document_metadata_inflight`) — **one** implementation.

---

## 4. Progress response shape (FE contract)

`IngestionProgressResponse` / `TrackProgressResponse` aliases:

| Field | Pin |
|-------|-----|
| `track_id` | `insert-*` / `pdf-*` |
| `progress.current_stage` | UnifiedStage string |
| `progress.completion_percentage` | 0–100 for **current** stage (facade) |
| `progress.latest_message` / `message` | Human stage_message |
| `progress.stages[]` | Timeline with skip flags |
| `source_type` | `pdf` \| `markdown` \| … |
| `ui_phase` / `display_status` | SPEC-057 mapper |

---

## 5. FE merge rule (LAW-23)

```
rank(stage) from UnifiedStage order
if poll.terminal && !store.terminal → use poll
else if rank(poll) > rank(store) OR poll.progress > store.progress → merge poll into store
else keep store (WS may be more granular within same stage)
seed is never SSOT after first successful poll or WS event
```

Zustand updates must be **immutable** (new track object) so React subscribers re-render.

---

## 6. Presenter pin (LAW-22)

| Format | Top-level UI | Nested detail |
|--------|--------------|---------------|
| All | `ServerStageStepper` + stage bar + overall est. | — |
| PDF during converting | same | page N/M (existing PDF progress/SSE) |
| MD/TXT/image | same | chunk N/M when message parses |

Forbidden: green Done marker + “Queued for processing…” on the same row.

---

## 7. Source type pin (Wave 3)

| Upload | `source_type` |
|--------|---------------|
| `.pdf` | `pdf` |
| `.md` / markdown body | `markdown` |
| plain `.txt` | `text` (or `markdown` if rendered as MD — pick one in Wave 3 and document) |
| images | existing image type |

---

## 8. Quality pin (LAW-27)

| Metric | Pin |
|--------|-----|
| Absolute entities MD == PDF | **Not required** |
| Density | entities / 1k chars on golden pair within threshold band |
| Structure | MD chunks with section breadcrumbs ≥ floor % when headings exist |

---

## 9. Ops reliability pins (upload / cancel / delete / replace)

| Concern | Pin |
|---------|-----|
| Orphan vs queued | Aged uploading seed is **not** Needs-attention while Insert task is live **or** pipeline has queue coverage |
| List document `id` | Bare admit UUID (never `staging:{id}`) |
| Staging fail / cancel | Release `staging:hash` (+ content); keep failed meta for list until dismiss |
| Sync dismiss | Staging-only DELETE → HTTP 200 `deleted: true` |
| Cancel vs promote | Cancel gate **before** staging→final promote |
| Reingest delete Err | Must **not** return ClearedForReingestion (StillProcessing / error) |
| MD Replace | Wait until old row gone (`deleted:true` or poll absent) before re-admit |
| Converting label | “Converting PDF” only for `source_type == pdf`; non-PDF omit converting step |
