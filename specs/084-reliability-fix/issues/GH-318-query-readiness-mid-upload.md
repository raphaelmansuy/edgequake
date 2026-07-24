# `GH-318` — Query / “GIA” activates before all documents are uploaded

> **Priority**: P1  
> **Audit status**: FIXED  
> **Sprint**: 2  
> **Laws**: LAW-11, LAW-3, LAW-8  
> **GitHub**: https://github.com/raphaelmansuy/edgequake/issues/318  
> **Verified against**: v0.21.0 / `19477c2d`

---

## 1. WHY

During bulk upload, users (reporter: “GIA”) start asking questions against a partial corpus and get incomplete answers. Trust in the knowledge graph drops. There is no product gate tying Query readiness to “all selected files uploaded and processed.”

**Product mapping (locked):** “GIA” is not a code symbol and is not CopilotKit. It maps to the **Query / knowledge-graph assistant** (`/query`, FEAT0007).

---

## 2. Audit (code is law)

| Field | Value |
|-------|-------|
| Query enablement | Always on — submit only gated by non-empty input ([`query-interface.tsx`](../../../edgequake_webui/src/components/query/query-interface.tsx)) |
| Workspace footer | Always “ready for queries” ([`workspace-status-footer.tsx`](../../../edgequake_webui/src/components/workspace/workspace-status-footer.tsx)) |
| Batch track | Shared `batchTrackId` across sequential uploads ([`use-file-upload.ts`](../../../edgequake_webui/src/hooks/use-file-upload.ts)) |
| `is_complete` | `pending==0 && processing==0` on **currently registered** track docs only ([`track_status.rs`](../../../edgequake/crates/edgequake-api/src/handlers/documents/query/track_status.rs)) |
| CopilotKit | Skill only — not wired in WebUI |
| Verdict | **CONFIRMED** |

Race: file 1 finishes processing while files 2…N still uploading → track reports complete → toast “All documents processed successfully” → user queries partial graph.

---

## 3. Root cause (first principles)

**LAW-11**: Batch completeness requires an **expected N** (client-declared) plus server-observed terminal states. Completeness derived only from “docs we already know about” flips true while the upload loop is still running.

Query has no ingest-awareness contract at all (second violation: readiness SSOT missing).

---

## 4. Multi-lens analysis

### Product Owner

- Acceptance: During an active bulk upload batch, Query shows a clear “Ingestion in progress” state; primary CTA can still allow “query anyway” (soft gate) but default messaging warns of incomplete corpus.
- After batch: all selected files either terminal (completed / failed / cancelled) → readiness green.
- Failed docs must not permanently block Query.

### Full Stack

| Layer | Change needed |
|-------|---------------|
| Upload client | Declare `expected_count` on track create/update |
| Track API | `is_complete` iff `registered >= expected` AND no pending/processing |
| Query UI | Subscribe to active batch / workspace ingest summary; banner + optional disable |
| Chat/query API | Optional advisory header only; do not hard-500 (product soft-gate) |

### AI Engineer

- Partial corpus → retrieval under-recall, not model failure. Guardrails are UX/truthfulness, not prompt hacks.
- Do not auto-answer “with confidence” while ingest active without disclosing corpus freshness.

### O(n) / Systems

- Sequential upload + shared track without expected N is O(files) race windows.
- Polling Query readiness must be cheap (status_counts / track endpoint), not full graph scans.

### Postgres Expert

- Track/doc status are KV/SQL — no AGE involvement for the gate.
- Avoid coupling readiness to entity counts on `"Node"` (that path is #331-sensitive).

---

## 5. ASCII causal diagram

```
  User selects N files
        |
        v
  Shared track_id; upload file 1..N sequentially
        |
        +--> file 1 registered+completed
        |         |
        |         v
        |   is_complete = (pending+processing==0)  --> TRUE too early
        |
        +--> files 2..N not yet in KV
        |
        v
  Query always enabled --> answers on partial KG
```

---

## 6. Solution (SOLID + DRY)

| Principle | Application |
|-----------|-------------|
| S | `BatchTrack` owns expected vs observed completeness |
| O | SoftGate vs HardGate policy enum for Query |
| L | Same track contract for upload progress UI and Query banner |
| I | `TrackStatus { expected_count, registered_count, is_complete, … }` |
| D | Query UI depends on track/workspace ingest DTO, not ad-hoc file list |
| DRY | Reuse/finish `BatchProgressCard` instead of a second progress widget |

### Implementation steps

1. Extend track create/upload to accept `expected_count` (or set once at batch start).
2. Fix `is_complete` in `track_status.rs`: require `registered_count >= expected_count` (when expected set) AND no pending/processing.
3. Query page: if any active incomplete track for workspace (or status_counts.processing/pending > 0 during known batch), show banner; soft-disable submit with override.
4. Success toast only when track truly complete (or split “uploaded to server” vs “processed”).
5. Wire `BatchProgressCard` into documents flow (currently underused).

---

## 7. Edge cases

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | User navigates away mid-upload | Track + expected_count persist; Query still warns |
| EC-2 | Some files fail HTTP upload | expected stays N; failed/cancelled terminal; complete when all terminal |
| EC-3 | Single-file upload | expected=1; no banner flicker if fast |
| EC-4 | Two parallel batches | Per-track expected; Query warns if **any** incomplete |
| EC-5 | Stale track never finished | TTL / cancel; don’t warn forever after timeout policy |
| EC-6 | User wants to query mid-ingest | Soft override “Query anyway” |
| EC-7 | Empty workspace | Query empty state; no false “ready with graph” |

---

## 8. E2E / contract tests

| Test | Assertion |
|------|-----------|
| `issue318_track_not_complete_until_expected` | expected=3, only 1 registered completed → `is_complete=false` |
| `issue318_query_banner_during_batch` | Playwright: during multi-upload, Query shows ingest banner |
| `issue318_query_ready_after_batch` | After all terminal, banner clears; submit enabled without override |

---

## 9. Cross-refs

- FEAT0007 Query  
- Upload track handlers / `use-file-upload.ts`  
- Related UX: #319 status honesty; #316 multi-workspace wait (different axis)
