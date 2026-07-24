# SPEC-086 — Implementation Roadmap

> DRY/SOLID waves. Wave 0 = this pack (done). Code starts at Wave 1 after review.  
> Cross-refs: [Register](01-finding-register.md) · [Verify](04-verification-matrix.md) · [Playbooks](05-surface-playbooks.md)

---

## Wave graph

```
  Wave 0 (docs)
      |
      v
  Wave 1 (backend visibility + stage WS)
      |
      v
  Wave 2 (FE presenter + merge) ----+
      |                             |
      v                             v
  Wave 3 (taxonomy + quality) --> Wave 4 (e2e matrix)
```

Wave 2 may start on FE merge tests in parallel with Wave 1 once contract pins are frozen; **presenter unification** should land after staging list visibility so ActiveRuns has data.

---

## Wave 0 — Spec pack (done)

| Deliverable | Path |
|-------------|------|
| Spine + laws | `00`…`06` |
| Lenses | `lenses/` |
| Findings | `findings/` |
| E2E matrix | `e2e/README.md` |

---

## Wave 1 — Backend visibility SSOT (ISP/DIP)

**Findings:** `ux086_staging_list`, `ux086_sparse_md_events`

### Goals

1. One staging-aware metadata helper used by progress **and** list / track / pipeline activity (extend 068; no third loader).  
2. In-flight MD rows appear with `current_stage` / `stage_message` from admit onward.  
3. Emit **stage-transition** WS events on Insert path (not only every-3rd `ChunkProgress`).

### Blast radius

| Area | Files |
|------|-------|
| Loader | `edgequake/crates/edgequake-api/src/services/document_metadata_scan.rs` |
| Callers | `handlers/documents/query/list.rs`, `track_status.rs`, `handlers/pipeline.rs`, `handlers/ingestion.rs` |
| WS | `pipeline_ws_bridge.rs` (or equivalent bridge) |
| Status writes | `processor/text_insert/prepare.rs`, `extraction.rs`, `status_updates.rs` |
| Facade | `services/progress_facade.rs` |

### SOLID notes

- **D**: Callers depend on a trait/helper, not ad-hoc `keys_with_prefix("staging:")`.  
- **I**: Optional filter `include_staging: bool` if completed-only views must exclude in-flight.  
- **O**: Stage WS event type extended without PDF SSE for MD.

### Exit

- `ux086_v_staging_list`, `ux086_v_stage_ws` green  
- Contract tests extend 068 for list/track staging

---

## Wave 2 — FE progress merge + one presenter (SRP/DRY)

**Findings:** `ux086_dual_progress_ui`, `ux086_store_beats_poll`

### Goals

1. Subscribe to `tracks.get(trackId)` (not memoized `getTrack` alone); **`applyPolledProgress`** merges into store.  
2. Immutable track updates in Zustand.  
3. Unify `progress-panel-row` → one `IngestionRunCard` / ActiveRuns-style stepper; nest PDF page detail under `converting`.

### Blast radius

| Area | Files |
|------|-------|
| Hooks | `edgequake_webui/src/hooks/use-ingestion-progress.ts` |
| Store | `edgequake_webui/src/stores/use-ingestion-store.ts` |
| UI | `progress-panel-row.tsx`, `ingestion-progress-panel.tsx`, `pdf-upload-progress.tsx`, `active-runs-panel.tsx`, `server-stage-stepper.tsx` |
| Timeline | `lib/pipeline/stage-timeline.ts`, `ingestion-run-view.ts` |
| Badges | `enhanced-status-badge.tsx` |

### SOLID notes

- **S**: Merge rule in one pure function; presenters only render.  
- **DRY**: Reuse `ServerStageStepper` + `buildIngestionRunView` — kill parallel PDF-only phase chrome as top-level product.

### Exit

- `ux086_v_one_presenter`, `ux086_v_merge_rule` green  
- Vitest: seed loses to advanced poll; terminal poll wins

---

## Wave 3 — Taxonomy + extraction quality

**Findings:** `ux086_source_type`, `ux086_extract_quality`

### Goals

1. `.md` admits as `source_type: "markdown"` end-to-end (JSON + multipart).  
2. Optimistic list stage aligned with server (`uploading`/`queued` → first real stage).  
3. Golden-pair density / section-breadcrumb harness (LAW-27).

### Blast radius

| Area | Files |
|------|-------|
| Admit | `document_admission.rs`, `multimodal_admission.rs`, `text_upload.rs` |
| FE upload | `file-kind.ts`, `perform-file-upload.ts`, `use-file-upload.ts` |
| Extract | chunk registry / section_context (pipeline) |
| QA | new or extended metrics under `edgequake/tests` or scripts |

### Exit

- `ux086_v_source_markdown`, `ux086_v_density_gate` green

---

## Wave 4 — E2E edge matrix

Run scenarios in [`e2e/README.md`](e2e/README.md). Prefer `PLAYWRIGHT_BASE_URL=http://localhost:…` (not `127.0.0.1`).

### Exit

All `ux086_e_*` IDs green; register findings flipped to FIXED with proof dates.

---

## Ordering constraints

| Constraint | Why |
|------------|-----|
| Wave 1 before trusting ActiveRuns for MD | List must see staging |
| Wave 2 merge before calling 068 “UX complete” | Seed stickiness remains without it |
| Wave 3 after Wave 2 | Taxonomy badges need unified presenter |
| LAW-28 | Do not change cancel API shapes in this pack |
