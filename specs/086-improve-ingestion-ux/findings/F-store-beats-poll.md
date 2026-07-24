# F-store-beats-poll — Seeded “Queued…” wins over poll

> **Finding ID**: `ux086_store_beats_poll`  
> **Status**: FIXED  
> **Wave**: 2  
> **Laws**: LAW-23  
> **Verify**: `ux086_v_merge_rule`, `ux086_e_ws_gap`, `ux086_e_admit_404`

---

## 1. Symptom

Non-PDF upload panel shows **“Queued for processing…”** indefinitely. Poll may return `chunking` / higher %, but UI stays on seed. Green Done-ish chrome can coexist with Queued copy (screenshot conflict).

---

## 2. Evidence (code is law)

| Path | Symbol / lines | Observation |
|------|----------------|-------------|
| `edgequake_webui/src/stores/use-ingestion-store.ts` | `createInitialProgress` (~126–146) | Seeds `latest_message: "Queued for processing…"`, `status: "pending"` |
| `edgequake_webui/src/hooks/use-ingestion-progress.ts` | `storeProgress` useMemo (~86–88) | Deps `[trackId, getTrack]` — may miss track mutations |
| `edgequake_webui/src/hooks/use-ingestion-progress.ts` | merge (~193–211) | Prefers store unless poll is **terminal**; poll does not write stages into store |
| `edgequake_webui/src/hooks/use-ingestion-progress.ts` | effect (~146–155) | `startTracking` on poll only refreshes id/name, not progress |
| `edgequake_webui/src/components/documents/ingestion-progress-panel.tsx` | compact 404 path | Intentionally shows Queued (068 admit race) |

---

## 3. Root cause

068 correctly hydrated tracking early and softened admit 404s, but left **store as SSOT over poll** for non-terminal states. When WS stage events are sparse/missing, the seed never loses. Stale memoization amplifies stickiness. PDF path bypasses this store entirely.

---

## 4. Fix (SOLID/DRY)

- Add `applyPolledProgress(trackId, mapped)` — immutable store update.  
- Merge rule per [06-contract-pins.md](../06-contract-pins.md) §5 (`max` by stage rank; terminal poll wins).  
- Subscribe with `useIngestionStore(s => s.tracks.get(trackId))`.  
- Keep brief Queued copy only for true admit 404 / idle ui_phase.

---

## 5. Edge cases

- WS more granular within same stage (chunk N/M) — do not clobber with coarser poll if same stage and lower progress.  
- Missed WS completed → terminal poll must win (already partially coded).  
- `enhanced-status-badge` same subscription bug if present.

---

## 6. Proof

```text
Date: 2026-07-24
Commands:
  pnpm exec vitest run src/lib/pipeline/__tests__/merge-ingestion-progress.test.ts \
    src/hooks/__tests__/use-ingestion-progress-086.test.ts
  e2e ux086_e_ws_gap / ux086_e_admit_404
Result: PASS — applyPolledProgress + tracks.get subscription
```
