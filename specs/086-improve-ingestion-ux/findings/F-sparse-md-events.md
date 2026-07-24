# F-sparse-md-events — Few progress events for markdown Insert

> **Finding ID**: `ux086_sparse_md_events`  
> **Status**: FIXED  
> **Wave**: 1  
> **Laws**: LAW-26  
> **Verify**: `ux086_v_stage_ws`, `ux086_e_small_md`

---

## 1. Symptom

Small Markdown files appear idle after admit: no converting pages, no SSE, and chunk WS/metadata updates may not fire until late (every 3rd chunk). Combined with store seed stickiness, UX looks frozen.

---

## 2. Evidence (code is law)

| Path | Symbol / lines | Observation |
|------|----------------|-------------|
| `edgequake-pipeline/…/ingestion_types.rs` | `stages_for_source` | Non-PDF drops `converting` |
| `edgequake-api/…/text_insert/prepare.rs` | metadata update (~359–362) | Update every 3 chunks or last |
| PDF path | `PdfPageProgress` + SSE | Rich converting events |
| WS bridge | forwards ChunkProgress, GraphStorage, PdfPage… | No generic stage-transition event for all Insert stages |
| `progress_facade.rs` | `pending` → admission freeze | Timeline idle until non-pending status |

---

## 3. Root cause

PDF richness is page-conversion theater. MD correctly skips converting but relies on sparse chunk ticks. Small docs (C &lt; 3) may emit almost nothing before jumping stages. Status channel can be starved relative to worker work (industry lesson: dedicated status updates).

---

## 4. Fix (SOLID/DRY)

- Emit **stage-transition** WS (and KV stage_message) on every UnifiedStage enter for Insert — O(stages) per doc.  
- Keep every-3rd chunk updates for N/M.  
- Do **not** add PDF page SSE for MD (LAW-26 / locked decision).  
- Ensure worker clears `pending` promptly when claimed (facade freeze).

---

## 5. Edge cases

- C = 1 chunk → still see chunking→extracting transitions.  
- WS reconnect gap → poll fallback (Wave 2 merge).  
- Event flood ban: no per-entity WS.

---

## 6. Proof

```text
Date: 2026-07-24
Commands:
  cargo test -p edgequake-api --lib services::pipeline_ws_bridge
  (StageTransition emit from update_document_status)
Result: PASS — maps_stage_transition + emit on Insert status writes
```
