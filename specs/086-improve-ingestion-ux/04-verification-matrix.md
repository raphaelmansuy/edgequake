# SPEC-086 — Verification Matrix

> Named gates. A finding is FIXED only when its verify IDs pass.  
> Cross-refs: [Register](01-finding-register.md) · [E2E](e2e/README.md) · [Playbooks](05-surface-playbooks.md)

---

## Unit / API gates

| Verify ID | Layer | Command / assertion | Findings |
|-----------|-------|---------------------|----------|
| `ux086_v_staging_list` | api | After MD admit: `GET /documents` includes doc with `admission_staging` or staging-merged row; `GET /documents/track/{insert-*}` non-empty; extend `contract_068` or new `contract_086` | `ux086_staging_list` |
| `ux086_v_stage_ws` | api | On Insert stage change (chunking→extracting), WS bridge emits stage-transition (or equivalent) for `insert-*` even when chunk count &lt; 3 | `ux086_sparse_md_events` |
| `ux086_v_merge_rule` | webui | Vitest: seed `Queued…` + poll `chunking@40%` → UI shows chunking; terminal poll wins over running store | `ux086_store_beats_poll` |
| `ux086_v_one_presenter` | webui | Vitest/component: non-PDF row renders stepper stages (not message-only); PDF still exposes converting sub-detail | `ux086_dual_progress_ui` |
| `ux086_v_source_markdown` | api+webui | `.md` multipart + JSON admit → metadata `source_type == "markdown"`; converting skipped in progress timeline | `ux086_source_type` |
| `ux086_v_density_gate` | pipeline/qa | Golden pair script: entities/1k chars + section breadcrumb rate reported; fails only on density cliff (threshold pinned in finding) | `ux086_extract_quality` |

---

## Playwright / e2e gates

| Verify ID | Scenario (see e2e/) | Findings |
|-----------|---------------------|----------|
| `ux086_e_md_live_stage` | MD leaves Queued within 2s; stepper shows Chunking+ | dual UI, store, staging |
| `ux086_e_pdf_parity` | PDF still shows converting + page detail | dual UI |
| `ux086_e_skip_converting` | MD stepper marks converting skipped, not active | dual UI, source type |
| `ux086_e_ws_gap` | WS disabled/killed; poll advances stages | store beats poll |
| `ux086_e_admit_404` | Soft 404 ≤5 retries; no Done+Queued conflict | store beats poll |
| `ux086_e_small_md` | &lt;3 chunks still get live stage updates | sparse events |
| `ux086_e_batch_mixed` | PDF+MD concurrent; both live | staging, dual UI |
| `ux086_e_reprocess_md` | Reprocess MD uses same presenter | dual UI |
| `ux086_e_cancel_md` | Cancel → Stopping → Cancelled (057) | LAW-28 |
| `ux086_e_refresh_mid` | Reload mid-run recovers from list+progress | staging list |
| `ux086_e_staging_promote` | After complete, staging gone; final Completed | staging list |
| `ux086_e_fairness_queue` | Second upload shows Queued run(s) honestly | 084 inherit |
| `ux086_e_orphan_staging_restart` | Aged staging Uploading → Needs attention | orphan restart |
| `ux086_e_orphan_staging_recovered` | Failed shell re-upload guidance | orphan restart |
| `ux086_e_orphan_staging_dismiss` | Dismiss sync clears Needs attention | orphan dismiss |
| `ux086_e_md_single_activerun` | Bare+staging ids → one card; no `{{taskId}}` | staging id dual |
| `ux086_e_orphan_plus_live_pdf` | Needs attention separate from Active run | dual activeruns mix |
| `ux086_e_queued_behind_busy` | Aged MD seed + busy queue → Active, not Needs attention | false orphan pending |
| `ux086_e_reupload_after_orphan` | Dismiss failed → same-bytes admit OK | orphan hash / ops |
| `ux086_e_replace_waits_delete` | Replace waits for delete before re-admit | replace delete race |
| `ux086_e_cancel_stopping_md` | Cancel → Stopping → Cancelled on ActiveRuns | LAW-28 |
| `ux086_e_md_no_converting_pdf` | MD card has no “Converting PDF” | md converting label |
| `ux086_e_double_upload_inflight` | Second identical upload while live → dialog / single card | ops |

### Ops API / unit gates

| Verify ID | Layer | Assertion | Findings |
|-----------|-------|-----------|----------|
| `ux086_v_reingest_fail_closed` | api | Delete Err → StillProcessing / error, not ClearedForReingestion | `ux086_reingest_fail_closed` |
| `ux086_v_orphan_queue_coverage` | webui | Vitest: aged seed + hasQueueCoverage → not orphan | `ux086_false_orphan_pending` |
| `ux086_v_md_hide_converting` | webui | Vitest: markdown timeline omits converting / no Converting PDF | `ux086_md_converting_label` |

---

## Regression inherit (must stay green)

| Prior gate | Command hint |
|------------|--------------|
| 068 text progress | `cargo test -p edgequake-api --test contract_068_text_ingest_progress` |
| 068 FE | `pnpm exec vitest run …use-ingestion-progress-068…` |
| 068 Playwright | `e2e/spec068-text-ingest-progress.spec.ts` |
| Cancel/fairness | SPEC-057 suite / docs playbook |

---

## Proof template

```text
Verify ID: ux086_v_…
Date:
Command:
Result: PASS|FAIL
Notes:
```
