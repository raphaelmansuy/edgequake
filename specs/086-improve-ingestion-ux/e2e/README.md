# SPEC-086 — E2E Edge Matrix

> Maps Playwright scenarios to verify IDs in [04-verification-matrix.md](../04-verification-matrix.md).  
> Implementation: `edgequake_webui/e2e/spec086-ingestion-ux.spec.ts` (**Wave 4 + ops reliability** — mocked 068-style; core `ux086_e_*` + ops gates: queued-behind-busy, replace-waits-delete, cancel-stopping, md-no-converting-pdf, double-upload-inflight, reupload-after-orphan).  
> Inherit: `e2e/spec068-text-ingest-progress.spec.ts` must stay green.

---

## Prerequisites

```bash
make status
# Backend :8080 healthy; WebUI on localhost (prefer :3010 / :3025 — not 127.0.0.1)

cd edgequake_webui
PLAYWRIGHT_BASE_URL=http://localhost:3010 PLAYWRIGHT_SKIP_STACK_CHECK=1 \
  pnpm exec playwright test e2e/spec086-ingestion-ux.spec.ts --project=chromium
```

Fixtures: small `.md` (&lt;3 chunks), medium `.md`, PDF of same paper when testing density/parity, optional second file for fairness queue.

---

## Scenario matrix

| Verify ID | Edge case | Steps (intent) | Expect |
|-----------|-----------|----------------|--------|
| `ux086_e_md_live_stage` | MD leaves Queued quickly | Upload `.md`; watch Active run / progress card | Within 2s of 202 (worker free): stage ≠ Queued-only; stepper shows Chunking+ or honest worker Queued with non-Done chrome |
| `ux086_e_pdf_parity` | PDF richness preserved | Upload PDF | Converting visible with page detail when converting; later Insert stages on same card |
| `ux086_e_skip_converting` | MD skip converting | Upload `.md`; inspect stepper | Converting omitted — never “Converting PDF” |
| `ux086_e_queued_behind_busy` | False orphan while Pending | Aged uploading seed + busy pipeline | Working pill — not Needs attention |
| `ux086_e_reupload_after_orphan` | Re-upload after dismiss | Dismiss failed shell → same-bytes admit | Admit succeeds; no duplicate dialog loop |
| `ux086_e_replace_waits_delete` | Replace vs 202 delete | Replace MD; keep old row until delete done | No second admit while list still has doc |
| `ux086_e_cancel_stopping_md` | Cancel Stopping | Cancel ActiveRuns MD | Stopping → Cancelled |
| `ux086_e_md_no_converting_pdf` | Label | MD extracting card | No “Converting PDF” text/step |
| `ux086_e_double_upload_inflight` | Inflight dup | Second identical upload while live | Dialog; single Working card for file |
| `ux086_e_ws_gap` | Poll-only advance | Disable/kill WS (or block WS in test); upload MD | Stages still advance via poll merge |
| `ux086_e_admit_404` | Soft admit race | Assert early progress 404s retry; UI copy | No green Done + “Queued for processing…” conflict; brief Queued OK |
| `ux086_e_small_md` | &lt;3 chunks | Upload tiny MD | Stage transitions still observed (stage WS / status) |
| `ux086_e_batch_mixed` | PDF+MD concurrent | Upload both | Both cards live; neither stuck on Done+Queued |
| `ux086_e_reprocess_md` | Reprocess | Reprocess completed MD (entities or full) | Same presenter; stage resets honestly |
| `ux086_e_cancel_md` | Cancel mid-run | Cancel in-flight MD | Stopping → Cancelled (057); no Completed flash |
| `ux086_e_refresh_mid` | Reload | Hard refresh during chunking/extracting | Card/list recovers from server (staging list Wave 1) |
| `ux086_e_staging_promote` | Promote | Wait until Completed | Staging gone; final row Completed; progress 200 on final |
| `ux086_e_fairness_queue` | Queue honesty | Saturate workers; upload second MD | Shows Queued run(s) honestly — not Done |
| `ux086_e_orphan_staging_restart` | Restart orphan Uploading | Seed aged staging Uploading + dead track | Header Needs attention (not Working); card failed/re-upload |
| `ux086_e_orphan_staging_recovered` | Post-recovery shell | Seed failed staging + `server_restart_interrupted` | Re-upload copy visible; no Working pill |
| `ux086_e_orphan_staging_dismiss` | Dismiss failed shell | Click Dismiss on Needs attention card | DELETE doc; ActiveRuns + Needs attention clear |

---

## Cross-ref to findings

| Verify ID | Primary findings |
|-----------|------------------|
| `ux086_e_md_live_stage` | dual UI, store beats poll, staging list |
| `ux086_e_pdf_parity` | dual UI |
| `ux086_e_skip_converting` | dual UI, source type |
| `ux086_e_ws_gap` | store beats poll |
| `ux086_e_admit_404` | store beats poll |
| `ux086_e_small_md` | sparse md events |
| `ux086_e_batch_mixed` | staging list, dual UI, extract quality (observational) |
| `ux086_e_reprocess_md` | dual UI |
| `ux086_e_cancel_md` | LAW-28 / 057 |
| `ux086_e_refresh_mid` | staging list |
| `ux086_e_staging_promote` | staging list |
| `ux086_e_fairness_queue` | 084 / 057 inherit |
| `ux086_e_orphan_staging_restart` | orphan staging restart |
| `ux086_e_orphan_staging_recovered` | orphan staging restart |

---

## Screenshot / artifact policy

On failure, capture:

1. Progress/ActiveRuns region  
2. Documents table row for the track  
3. Network: last `/ingestion/*/progress` and `/documents` JSON snippets  

Store under `e2e/screenshots/` only when running Wave 4 proofs (optional, mirror SPEC-048).

---

## Exit criteria (Wave 4)

All 12 `ux086_e_*` IDs PASS on chromium (mocked harness), plus orphan-staging restart follow-ups (`ux086_e_orphan_staging_*`). Register: **7 FIXED / 0 PARTIAL**.
