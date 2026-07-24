# F — Ingestion UX→backend residual edge cases (post orphan staging)

**Date**: 2026-07-24  
**Parent**: SPEC-086 follow-up assessment  
**Status**: PARTIAL (critical P0/P1 closed in same wave; some P2 remain)

## Context

After fixing eternal Uploading (orphan staging) and stuck Deleting (sync dismiss), a full UX→backend pass found additional edge cases that break the “failed shell stays visible + re-upload works” contract.

## Findings

| ID | Sev | Status | Summary |
|----|-----|--------|---------|
| `ux086_worker_fail_wipe` | P0 | FIXED | Worker pipeline fail called full `rollback_staging` after marking failed → list shell vanished. Now `release_staging_reservation` (content+hash only). |
| `ux086_orphan_hash_block` | P1 | FIXED | Orphan fail kept `staging:hash` → same-bytes re-upload → `duplicate_processing`. Recovery now releases reservation. |
| `ux086_promote_before_cancel` | P1 | FIXED | Finalize promoted then checked cancel → dismiss/promote race. Cancel gate moved before promote. |
| `ux086_recover_stuck_staging` | P1 | FIXED | Orphan fail pass existed; requeue still wrote final pending for `admission_staging` shells. Requeue skips staging. |
| `ux086_detail_staging_body` | P1 | FIXED | Detail meta was staging-aware; body loader final-only. Now tries `staging:{id}-content`. |
| `ux086_fe_reupload_not_retry` | P1 | FIXED | Retry Failed / Reprocess Stuck could target orphan shells. `needsReuploadNotReprocess` wired; slow-upload false orphan tightened. |
| `ux086_dual_activeruns_mix` | P1 | FIXED | Orphan failed MD mixed under “Active runs” beside live PDF. Panel now partitions Working vs Needs attention + Dismiss all; Replace delete is fatal for text/MD. |
| `ux086_md_staging_id_dual` | P0 | FIXED | MD admit: list id was `staging:{uuid}` while pin used bare uuid → dual ActiveRuns; optimistic `t(queued)` left literal `{{taskId}}`. `parse_doc_metadata` strips staging; `queuedPending` + pin/track dedupe. |
| `ux086_ws_store_dual_ssot` | P2 | OPEN | WS can still seed store ahead of list poll merge (inherited store-beats-poll surface; mitigated not eliminated). |
| `ux086_zombie_processing_lease` | P2 | OPEN | Staging recovery treats any `Processing` as live (no lease check); relies on periodic lease reaper. Multi-replica valid-lease Processing can delay fail. |

## Verify

```bash
cargo test -p edgequake-api --lib orphan_staging_recovery
cargo test -p edgequake-api --lib staging_admission
cd edgequake_webui && pnpm exec vitest run src/lib/pipeline/__tests__/pipeline-document-state.test.ts
```

## Industry alignment

Durable ingest UX (Temporal/fred-style): never leave “pending/uploading” with no live worker; surface Failed durably; free idempotency keys when telling the user to re-upload. Resumable chunked upload remains out of scope (product gap, not regression).
