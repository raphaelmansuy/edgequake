# F-dual-progress-ui — Two progress products (PDF vs non-PDF)

> **Finding ID**: `ux086_dual_progress_ui`  
> **Status**: FIXED  
> **Wave**: 2  
> **Laws**: LAW-22, LAW-25  
> **Verify**: `ux086_v_one_presenter`, `ux086_e_md_live_stage`, `ux086_e_pdf_parity`, `ux086_e_skip_converting`

---

## 1. Symptom

PDF uploads get multi-phase chrome (upload → convert pages → chunk → extract…). Markdown/text get a compact bar + single message (often stuck on Queued). ActiveRuns stepper can look richer when list data exists, but upload-list path remains second-class.

---

## 2. Evidence (code is law)

| Path | Symbol / lines | Observation |
|------|----------------|-------------|
| `edgequake_webui/src/components/documents/progress-panel-row.tsx` | `isPdf ? PdfUploadProgress : IngestionProgressPanel` (~134–150) | Hard UI fork |
| `edgequake_webui/src/components/documents/ingestion-progress-panel.tsx` | compact mode | Message + bar; no stepper in compact |
| `edgequake_webui/src/hooks/use-pdf-progress.ts` | PDF poll + SSE | PDF-only richness |
| `edgequake_webui/src/lib/pipeline/stage-timeline.ts` | skip converting when non-pdf | Skip exists but unused by compact panel |
| `edgequake_webui/src/components/documents/active-runs-panel.tsx` | ServerStageStepper | Better path, list-dependent |

---

## 3. Root cause

Progress UX grew as a PDF specialty product (`PdfUploadProgress`) and a generic leftover (`IngestionProgressPanel`). Format became a **product fork** instead of a detail under UnifiedStage. Even when MD progress data is correct, compact UI cannot express skipped converting or stage chips.

---

## 4. Fix (SOLID/DRY)

- Introduce one `IngestionRunCard` using `ServerStageStepper` + bars for all formats.  
- Nest PDF page detail under `converting` via `usePdfProgress` slot (OCP).  
- Compact = density, not fewer stages.  
- Non-goal: delete PDF page progress backend.

---

## 5. Edge cases

- Reprocess entities/merge (non-full) must use same card.  
- Image uploads classified non-PDF — same presenter.  
- Handoff: hide upload-list duplicate when ActiveRuns shows same track with live stage.

---

## 6. Proof

```text
Date: 2026-07-24
Commands:
  pnpm exec vitest run src/lib/pipeline/__tests__/ingestion-run-view-086.test.ts
  PLAYWRIGHT_BASE_URL=http://localhost:3010 PLAYWRIGHT_SKIP_STACK_CHECK=1 \
    pnpm exec playwright test e2e/spec086-ingestion-ux.spec.ts --project=chromium
Result: PASS — IngestionRunCard + ProgressPanelRow; e2e md_live_stage / pdf_parity / skip_converting
Follow-up: deleted dead IngestionProgressPanel.tsx (no runtime importers); barrel exports IngestionRunCard.
```
