# Finding: Orphan staging Uploading across restart

| Field | Value |
|-------|-------|
| ID | `ux086_orphan_staging_restart` |
| Sev | P0 |
| Status | FIXED |
| Surfaces | api + webui |
| Date | 2026-07-24 |

## Symptom

Markdown (e.g. `invarian_*.md`) shows eternal **Uploading** / “Document received, starting processing” and header **Working · N** even when the app was just started — no worker progress.

## Root cause

1. Admit writes `staging:{id}-metadata` with `current_stage=uploading` + seed message.
2. Classic orphan recovery **skipped** `staging:` keys (mid-upload race).
3. SPEC-086 made staging **list-visible** → ActiveRuns renders the shell forever if the Insert task never hydrated / was lost on restart.
4. FE treated `uploading` as active and `detectStuckDocuments` skipped docs with `track_id` → looked healthy “Working”, not stuck.

## Fix

- Backend: `recover_orphaned_staging_admissions` on startup (no age), periodic + `recover-stuck` (aged); fail shell with `server_restart_interrupted` + re-upload copy when no live Pending/Processing task.
- FE: `isOrphanAdmissionShell` reclassifies aged upload seeds as stuck (not Working); run card projects failed/re-upload; filename infers `markdown` when `source_type` missing.

## Verify

- Unit: `orphan_staging_recovery` tests; `pipeline-document-state` orphan cases; `ingestion-run-view-086` orphan + filename.
- E2E: `ux086_e_orphan_staging_restart`, `ux086_e_orphan_staging_recovered`.
