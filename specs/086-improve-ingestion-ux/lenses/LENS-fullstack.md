# LENS — Full Stack

**Job:** map end-to-end identity, admission, loaders, and FE merge so MD and PDF share one contract.  
**Cites:** LAW-22…26 · `ux086_staging_list` · `ux086_store_beats_poll` · `ux086_dual_progress_ui` · 068

---

## 1. End-to-end sequence (target)

```text
Client upload
  → classify format
  → POST admit (202, task_id=insert-*|pdf-*)
  → FE startTracking(task_id) + subscribe WS
  → GET /ingestion/{task_id}/progress  (staging-aware)
  → GET /documents                    (staging-aware)  ← Wave 1 gap today
  → One presenter (stepper) fed by max(store, poll)
  → Worker status writes + stage WS
  → Terminal → promote staging→final (text) / complete PDF shell
```

---

## 2. Breakpoints (today)

| Layer | PDF | MD/text | Defect ID |
|-------|-----|---------|-----------|
| Admit KV | Final `queued` shell | `staging:` + `pending` | `ux086_staging_list` |
| Progress API | OK | OK (068) | — |
| List / track | Visible | Often invisible | `ux086_staging_list` |
| Channels | Poll+SSE+page WS | Poll + sparse chunk WS | `ux086_sparse_md_events` |
| Presenter | `PdfUploadProgress` | Compact `IngestionProgressPanel` | `ux086_dual_progress_ui` |
| Merge | N/A (own hooks) | Store seed beats poll | `ux086_store_beats_poll` |

---

## 3. Shared primitives (DRY)

| Primitive | Owner | Callers |
|-----------|-------|---------|
| Staging-aware metadata load | `document_metadata_scan` | progress, list, track, activity |
| Stage rank + merge | FE pure fn + store `applyPolledProgress` | hooks, badges |
| `ServerStageStepper` + run view | webui pipeline lib | ActiveRuns + upload row |
| UnifiedStage skip converting | pipeline + progress_facade + stage-timeline | API + FE |

---

## 4. API contract checklist

- [ ] Upload returns `track_id == task_id` (068)  
- [ ] Progress 200 during staging admit race (soft 404 retries only briefly)  
- [ ] List includes in-flight MD with `current_stage` / `stage_message`  
- [ ] Track status resolves `insert-*` during staging  
- [ ] Pipeline activity counts staging MD in queued/working  
- [ ] Stage WS on Insert transitions  
- [ ] Cancel still keys `task_id` (057)

---

## 5. Non-goals

- Dual-write final+staging forever without promote semantics  
- New SSE endpoint for MD pages  
- Changing fairness lane algorithms (084/057)

---

## 6. Implementation order (stack view)

1. Loader SSOT (Wave 1) — unblocks ActiveRuns  
2. Stage WS (Wave 1) — unblocks small docs  
3. FE merge (Wave 2) — unblocks upload-list panel  
4. One presenter (Wave 2) — kills format fork  
5. Taxonomy (Wave 3) — clean badges/filters  
