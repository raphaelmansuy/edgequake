# LENS — Front End Designer

**Job:** compose one progress product; kill PDF/non-PDF presenter fork.  
**Cites:** LAW-22, LAW-23, LAW-25 · `ux086_dual_progress_ui` · `ux086_store_beats_poll`

---

## 1. Current composition (problem)

```text
ProgressPanelRow
  ├─ isPdf? → PdfUploadProgress (phases, SSE, page %)
  └─ else   → IngestionProgressPanel compact (bar + message only)

DocumentManager
  ├─ ActiveRunsPanel (ServerStageStepper)  ← list-driven
  └─ UploadProgressList                    ← upload-driven; suppressed when ActiveRuns non-empty
```

Two products + handoff gaps ⇒ MD looks poor or stuck.

---

## 2. Target composition (DRY)

```text
IngestionRunCard (single)
  ├─ header: name + source chip + cancel
  ├─ ServerStageStepper (skipped converting when non-PDF)
  ├─ stage ProgressBar + overall est.
  ├─ message line (stage_message)
  └─ optional slot: PdfPageDetail (only if stage==converting && pdf)
```

| Data in | Source |
|---------|--------|
| Run view | `buildIngestionRunView` from list row **or** mapped progress |
| Live patch | `useIngestionProgress` merge (store ← poll/WS) |
| PDF pages | existing `usePdfProgress` nested when converting |

---

## 3. Hook / store responsibilities (SRP)

| Module | Owns |
|--------|------|
| `use-ingestion-store` | Tracks map; `startTracking`; `applyPolledProgress`; WS handlers; immutable updates |
| `use-ingestion-progress` | Subscribe track; poll; merge; cancel |
| `use-pdf-progress` | Page N/M detail only |
| Presentational cards | Render run view; no merge logic |

---

## 4. Subscription bug to fix

```ts
// BAD — getTrack identity stable; tracks mutations invisible
useMemo(() => getTrack(trackId), [trackId, getTrack])

// GOOD — select slice so re-render on track change
useIngestionStore(s => trackId ? s.tracks.get(trackId) : null)
```

---

## 5. Handoff rule

When ActiveRuns shows the same `track_id`, hide duplicate upload-list row **only if** ActiveRuns has live stage ≥ uploading (not empty). Avoid gap where both are blank.

---

## 6. Acceptance

| ID | Gate |
|----|------|
| FE-086-01 | `ux086_v_one_presenter` |
| FE-086-02 | `ux086_v_merge_rule` |
| FE-086-03 | No second top-level PDF-only panel for Insert stages after convert |
