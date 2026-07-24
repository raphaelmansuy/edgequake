# SPEC-086 — Finding Register

> **SSOT for status counts**  
> **Audit**: 2026-07-24 · **Wave 0 docs**: done · **Implementation**: 2026-07-24  
> **Status legend**: OPEN | PARTIAL | FIXED | WONTFIX  
> **Counts**: **19 FIXED / 0 PARTIAL / 2 OPEN**

DRY rule: one row per finding. Deep studies live under [`findings/`](findings/).

| ID | Finding | Sev | Surface | Wave | Study | Laws | Status |
|----|---------|-----|---------|------|-------|------|--------|
| `ux086_dual_progress_ui` | PDF and non-PDF use different progress presenters; compact MD panel is message-only | P0 | webui | 2 | [F-dual-progress-ui.md](findings/F-dual-progress-ui.md) | 22,25 | FIXED |
| `ux086_store_beats_poll` | Ingestion store seed (“Queued…”) wins over advanced poll; stale memo can freeze UI | P0 | webui | 2 | [F-store-beats-poll.md](findings/F-store-beats-poll.md) | 23 | FIXED |
| `ux086_staging_list` | Staging metadata visible to progress (068) but not list/track/activity | P0 | api | 1 | [F-staging-list-visibility.md](findings/F-staging-list-visibility.md) | 24 | FIXED |
| `ux086_sparse_md_events` | MD lacks converting/SSE; chunk WS every 3 chunks → small docs look idle | P1 | api+webui | 1 | [F-sparse-md-events.md](findings/F-sparse-md-events.md) | 26 | FIXED |
| `ux086_source_type` | `.md` upload taxonomy inconsistent (`text` / `file` / `markdown`) | P2 | api+webui | 3 | [F-source-type-taxonomy.md](findings/F-source-type-taxonomy.md) | 25 | FIXED |
| `ux086_extract_quality` | Absolute entity counts misread as format quality; need density/section gates | P2 | pipeline+qa | 3 | [F-extraction-quality-parity.md](findings/F-extraction-quality-parity.md) | 27 | FIXED |
| `ux086_orphan_staging_restart` | Staging Uploading seed survives restart → eternal Working (Invarián class) | P0 | api+webui | follow-up | [F-orphan-staging-restart.md](findings/F-orphan-staging-restart.md) | 24 | FIXED |
| `ux086_worker_fail_wipe` | Pipeline fail wiped staging shell via full rollback → failed doc vanished | P0 | api | edge-pass | [F-ingestion-edge-residuals.md](findings/F-ingestion-edge-residuals.md) | 24 | FIXED |
| `ux086_orphan_hash_block` | Failed staging kept `staging:hash` → re-upload blocked as duplicate | P1 | api | edge-pass | [F-ingestion-edge-residuals.md](findings/F-ingestion-edge-residuals.md) | 24 | FIXED |
| `ux086_promote_before_cancel` | Finalize promote-before-cancel → dismiss/ghost-final race | P1 | api | edge-pass | [F-ingestion-edge-residuals.md](findings/F-ingestion-edge-residuals.md) | 24 | FIXED |
| `ux086_recover_stuck_staging` | recover-stuck requeued staging onto empty final pending | P1 | api | edge-pass | [F-ingestion-edge-residuals.md](findings/F-ingestion-edge-residuals.md) | 24 | FIXED |
| `ux086_detail_staging_body` | Detail body ignored `staging:{id}-content` | P1 | api | edge-pass | [F-ingestion-edge-residuals.md](findings/F-ingestion-edge-residuals.md) | 24 | FIXED |
| `ux086_fe_reupload_not_retry` | Retry Failed / Reprocess Stuck targeted orphan re-upload shells | P1 | webui | edge-pass | [F-ingestion-edge-residuals.md](findings/F-ingestion-edge-residuals.md) | 25 | FIXED |
| `ux086_ws_store_dual_ssot` | WS store seed can still race list poll merge | P2 | webui | residual | [F-ingestion-edge-residuals.md](findings/F-ingestion-edge-residuals.md) | 23 | OPEN |
| `ux086_zombie_processing_lease` | Staging recovery treats Processing as live without lease check | P2 | api | residual | [F-ingestion-edge-residuals.md](findings/F-ingestion-edge-residuals.md) | 24 | OPEN |
| `ux086_dual_activeruns_mix` | Orphan failed MD mixed under “Active runs” beside live PDF | P1 | webui | dual-run | [F-ingestion-edge-residuals.md](findings/F-ingestion-edge-residuals.md) | 25 | FIXED |
| `ux086_md_staging_id_dual` | MD admit → bare pin + `staging:{id}` list = dual ActiveRuns; `{{taskId}}` i18n | P0 | api+webui | dual-run | [F-ingestion-edge-residuals.md](findings/F-ingestion-edge-residuals.md) | 24,23 | FIXED |
| `ux086_false_orphan_pending` | Aged uploading seed → Needs attention while Insert still Pending / queue busy | P0 | webui | ops | [F-ingest-ops-reliability.md](findings/F-ingest-ops-reliability.md) | 24 | FIXED |
| `ux086_replace_delete_race` | MD Replace admits before 202 delete finishes → duplicate completed rows | P0 | webui | ops | [F-ingest-ops-reliability.md](findings/F-ingest-ops-reliability.md) | 24 | FIXED |
| `ux086_reingest_fail_closed` | Reingest proceeds after delete Err → ClearedForReingestion | P0 | api | ops | [F-ingest-ops-reliability.md](findings/F-ingest-ops-reliability.md) | 24 | FIXED |
| `ux086_md_converting_label` | Non-PDF stepper still shows “Converting PDF” | P1 | webui | ops | [F-ingest-ops-reliability.md](findings/F-ingest-ops-reliability.md) | 25 | FIXED |

---

## Wave summary

| Wave | Findings | Intent | Status |
|------|----------|--------|--------|
| 0 | (pack) | Contract + lenses + register | done |
| 1 | `ux086_staging_list`, `ux086_sparse_md_events` | Backend visibility + stage WS | done |
| 2 | `ux086_dual_progress_ui`, `ux086_store_beats_poll` | One presenter + merge rule | done |
| 3 | `ux086_source_type`, `ux086_extract_quality` | Taxonomy + density golden-pair gate | done |
| 4 | (all) | E2E edge matrix | done |
| ops | false orphan, replace race, reingest fail-closed, MD converting label | Upload/cancel/delete/replace reliability | done |

---

## Inherited residuals (not re-opened as new IDs)

| Prior | Residual for 086 |
|-------|------------------|
| [068](../001-benchmark/001-edgquake-improvements/068-text-ingest-progress-parity.md) | Track identity FIXED; list/track staging FIXED in Wave 1 |
| [048](../048-improve-ux/) | Progress contract axioms; dual presenter FIXED via IngestionRunCard |
| [057](../057-pipeline-reliability/) | Cancel/Stopping SSOT — LAW-28 inherit only |
| [084](../084-reliability-fix/) | Fairness / query readiness — queue honesty covered in e2e refresh/list |

---

## Status update rule

A finding moves to FIXED only when its verify IDs in [04-verification-matrix.md](04-verification-matrix.md) pass and the study file records proof date + command output summary.
