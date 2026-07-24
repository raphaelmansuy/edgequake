# SPEC-084 — Issue Register

> **SSOT for audit status**  
> **Verified against**: v0.21.1 (SPEC-084 reliability cut 2026-07-24)  
> **Summary**: **6 FIXED / 0 PARTIAL / 0 CONFIRMED / 0 RETRACTED**

---

## Summary table

| ID | GitHub | Title | Priority | Sprint | Audit | Laws | Study |
|----|--------|-------|----------|--------|-------|------|-------|
| GH-331 | [#331](https://github.com/raphaelmansuy/edgequake/issues/331) | Pool exhaustion / source_ids count SQL | P0 | 0 | **FIXED** | 9,12,8 | [issues/GH-331-…](issues/GH-331-pool-exhaustion-source-ids.md) |
| GH-319 | [#319](https://github.com/raphaelmansuy/edgequake/issues/319) | Failed filter zero results | P0 | 0 | **FIXED** | 10,3,8 | [issues/GH-319-…](issues/GH-319-failed-filter-zero-results.md) |
| GH-317 | [#317](https://github.com/raphaelmansuy/edgequake/issues/317) | Selected bulk delete fails | P0 | 1 | **FIXED** | 12,9,8 | [issues/GH-317-…](issues/GH-317-selected-bulk-delete.md) |
| GH-255 | [#255](https://github.com/raphaelmansuy/edgequake/issues/255) | Gateway slash model passthrough | P1 | 1 | **FIXED** | 14,3,8 | [issues/GH-255-…](issues/GH-255-slash-model-passthrough.md) |
| GH-318 | [#318](https://github.com/raphaelmansuy/edgequake/issues/318) | Query readiness mid-upload (“GIA”) | P1 | 2 | **FIXED** | 11,3,8 | [issues/GH-318-…](issues/GH-318-query-readiness-mid-upload.md) |
| GH-316 | [#316](https://github.com/raphaelmansuy/edgequake/issues/316) | Workspace processing serialized | P1 | 2 | **FIXED** | 13,3,8 | [issues/GH-316-…](issues/GH-316-workspace-fairness.md) |

---

## Audit notes (one-liners)

### GH-331 — FIXED

`pg_node_counts_by_source_prefixes` JOINs `"Node"`; EXPLAIN hits `idx_node_source_ids_gin`. Parent GIN rejected (LAW-9).

### GH-319 — FIXED

`ListDocumentsRequest.status`; global `status_counts` then filter then paginate; FE skips client re-filter.

### GH-318 — FIXED

Track `expected_count` / `registered_count`; `is_complete` waits for expected registration; Query soft-gate + “Query anyway”.

### GH-317 — FIXED

`POST /documents/batch-delete` → one `TaskType::BatchDeletion`; FE one-shot; wipe-all (#309) unchanged.

### GH-316 — FIXED

Workspace-fair `claim_next` (least-loaded then oldest) + nested workspace ingest lane under tenant cap.

### GH-255 — FIXED

COMPAT-GUARD allows slash models with custom OpenAI-compatible base / `EDGEQUAKE_ALLOW_GATEWAY_MODEL_IDS`; `llm_full_id` no double-prefix.

---

## Close policy

| Condition | Action |
|-----------|--------|
| Audit FIXED + e2e green | Comment with evidence + close |
| PARTIAL / CONFIRMED | Comment with study link; **leave open** |

---

## Deferred / follow-ups (not blocking FIXED)

Root-cause fixes shipped; these matrix items are explicitly deferred for a later cut:

| Item | Why deferred |
|------|----------------|
| Playwright `issue319-failed-filter.spec.ts` | FE filter path covered by API e2e; UI flake harness follow-up |
| Playwright `issue317-bulk-delete.spec.ts` | API `issue317_batch_delete_unselected_remain` covers admit + unselected remain |
| Playwright `issue318-query-readiness.spec.ts` | API track `expected_count` + FE soft-gate shipped; browser harness follow-up |
| `issue317_batch_delete_200_opcount` | One `BatchDeletion` task is enough for UX/pool storm; scale opcount proof later |
| `issue316_claim_sql_uses_workspace_index` | M098 index present; EXPLAIN guard optional |

---

## Related but out of register

| Item | Note |
|------|------|
| #309 Wipe-all | FIXED v0.20.1; referenced by GH-317 |
| PR #229 | Unmerged; intent absorbed into GH-255 |
| SPEC-071 / M038 / M070 | Prerequisites for GH-331 correct fix |
| M098 | `batch_deletion` task type + claim index |
