# F — Ingest ops reliability (upload / cancel / delete / replace)

**Date**: 2026-07-24  
**Parent**: SPEC-086 reliability wave  
**Status**: implementation in progress → FIXED when verify IDs pass

## First principles

One `document_id` ↔ one live `track_id` ↔ one ActiveRun card.  
Queued-behind-busy ≠ orphan.  
Replace must retire the old row before a second admit.  
Delete Err must never look like “cleared”.

## Findings

| ID | Sev | Status | Summary |
|----|-----|--------|---------|
| `ux086_false_orphan_pending` | P0 | FIXED | FE aged seed + uploading must not Needs-attention while queue coverage / live track |
| `ux086_replace_delete_race` | P0 | FIXED | MD Replace waits for delete terminal (`deleted:true` or poll absent) before re-admit |
| `ux086_reingest_fail_closed` | P0 | FIXED | `document_reingest` delete Err → StillProcessing / error, not ClearedForReingestion |
| `ux086_md_converting_label` | P1 | FIXED | Non-PDF timeline omits converting; never “Converting PDF” |

## State machine (ops)

```
admit → StagingUploading (task Pending|Processing = live)
     → Processing stages
     → Completed (promote) | FailedShell (keep meta, release hash) | Cancelled
FailedShell → dismiss sync
Replace terminal → wait delete done → admit (fail closed)
```

## Verify

- `ux086_e_queued_behind_busy`
- `ux086_e_reupload_after_orphan`
- `ux086_e_replace_waits_delete`
- `ux086_e_cancel_stopping_md`
- `ux086_e_md_no_converting_pdf`
- `ux086_e_double_upload_inflight`
- Unit: reingest fail-closed; orphan + coverage; timeline omit converting
