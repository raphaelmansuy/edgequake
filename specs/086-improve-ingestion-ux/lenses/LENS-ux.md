# LENS — UX

**Job:** define the user-facing state machine and copy so format never implies “stuck.”  
**Cites:** LAW-22…26 · SPEC-048 FP · `ux086_store_beats_poll` · `ux086_staging_list`

---

## 1. Happy path (all formats)

```text
Selected → Uploading (HTTP) → Admitted (202)
  → Queued (fairness / worker wait)     [honest, short]
  → Running stages (skip converting if non-PDF)
  → Completed | Partial failure | Failed | Cancelled
```

**Time-to-first-live-stage &lt; 2s** after 202 when worker is free (PO-086-01). If fairness delays, copy must say **Queued** with position/context — not Done.

---

## 2. Copy pins

| State | Allowed copy | Forbidden |
|-------|--------------|-----------|
| Admit race / soft 404 | “Queued for processing…” (brief) | “Done”, green check |
| Worker wait | “Queued — waiting for a worker…” | “Extracting…” |
| Active stage | “Chunking — Step N: …” / stage_message | Static seed after poll advances |
| Skipped converting | Stepper shows skipped / omitted | Grey step looking “blocked” |
| Terminal success | “Completed” | “Queued…” |
| Cancel | “Stopping…” → “Cancelled” (057) | Instant Completed |

---

## 3. Conflict to eliminate (screenshot bug)

**Observed:** green legend “Done” + blue text “Queued for processing…”.

**Rule:** Status chrome color encodes **ui_phase** (idle/running/terminal). Message and color must agree. Done/green only for terminal success.

---

## 4. Surfaces that must agree

| Surface | Must show |
|---------|-----------|
| Upload / Active run card | Same stage as progress API |
| Row status badge | `current_stage` or honest Queued |
| Pipeline busy | 057/048 busy rule |
| Processing Files legend | If kept, map to UnifiedStage — or remove in favor of ActiveRuns |

Recommendation: prefer **Active run(s)** stepper as SSOT; deprecate Reading/Uploading/Extracting/Done mini-legend if it conflicts.

---

## 5. Edge UX cases

| Case | UX |
|------|-----|
| Small MD (&lt;3 chunks) | Stage chips still advance (stage WS) |
| Refresh mid-run | List+progress restore card |
| Mixed batch | Per-file cards; no single global spinner only |
| Reprocess | Same card; mode badge (entities/merge/full) |
| Long extract | Determinate N/M when known; else indeterminate verb |

---

## 6. Acceptance

| ID | Gate |
|----|------|
| UX-086-01 | `ux086_e_md_live_stage` |
| UX-086-02 | `ux086_e_admit_404` (no Done+Queued) |
| UX-086-03 | `ux086_e_skip_converting` |
| UX-086-04 | `ux086_e_fairness_queue` |
