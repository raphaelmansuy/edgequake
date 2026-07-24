# F-staging-list-visibility — Staging visible to progress, not list/track

> **Finding ID**: `ux086_staging_list`  
> **Status**: FIXED  
> **Wave**: 1  
> **Laws**: LAW-24  
> **Verify**: `ux086_v_staging_list`, `ux086_e_refresh_mid`, `ux086_e_staging_promote`

---

## 1. Symptom

In-flight Markdown may be missing from Documents table / ActiveRuns / track status until promote-on-success, while `GET /ingestion/{insert-*}/progress` works (068). UI feels stuck or empty compared to PDF (final `queued` shell is list-visible immediately).

---

## 2. Evidence (code is law)

| Path | Symbol / lines | Observation |
|------|----------------|-------------|
| `…/upload/document_admission.rs` | admit metadata (~243–265, staging write) | `status: pending`, `admission_staging: true`, staging keys only |
| `…/services/document_metadata_scan.rs` | `load_scoped_document_metadata_for_progress` (~199–240) | Merges `staging:` for progress (068 FIXED) |
| `…/handlers/ingestion.rs` | uses `_for_progress` | Progress OK |
| `…/handlers/documents/query/list.rs` | limited metadata load (~86+) | No staging merge |
| `…/handlers/documents/query/track_status.rs` | `load_scoped_document_metadata` (~42–43) | No staging merge |
| `…/services/workspace_document_index.rs` | skip staging (~16–18) | wsdoc index ignores staging |
| PDF admit | `provision_queued_pdf_document_shell` | Final KV shell → list visible |

---

## 3. Root cause

Text admit uses staging-until-promote for safety. 068 fixed **progress** loaders only. List/track/activity still use final/wsdoc paths ⇒ **two visibility contracts**. PDF never needed staging for list visibility.

PARTIAL = progress path FIXED; list/track/activity OPEN.

---

## 4. Fix (SOLID/DRY)

- Promote `_for_progress` to shared `load_scoped_document_metadata_inflight` (name TBD).  
- Wire list, track_status, pipeline activity through it (optional `include_staging` if a caller must exclude).  
- Prefer final over staging when both exist (already in 068 merge).  
- Non-goal: remove staging promote semantics.

O(n): see [LENS-on-expert.md](../lenses/LENS-on-expert.md) — O(L+S) acceptable; index later if needed.

---

## 5. Edge cases

- Tenant mismatch on staging keys — must filter.  
- After promote, staging key gone; final Completed — no dupes.  
- Refresh mid-run relies on list visibility.  
- Fairness queue: staging row should show Queued honestly.

---

## 6. Proof

```text
Date: 2026-07-24
Commands:
  cargo test -p edgequake-api --test contract_086_ingestion_visibility
  e2e ux086_e_refresh_mid
Result: PASS — list/track/activity merge staging via merge_staging_metadata_*
```
