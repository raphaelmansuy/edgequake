# SPEC-084 — E2E Test Matrix

> Every issue must land with named tests. Prefer postgres-backed e2e for AGE/pool; Playwright for UI filters/query.

---

## Sprint 0

| Test ID | Issue | Type | Setup | Assert |
|---------|-------|------|-------|--------|
| `issue331_node_counts_uses_child_gin_explain` | GH-331 | PG EXPLAIN | Graph with `"Node"` + M038 GIN; seeded source_ids | Plan references `idx_node_source_ids_gin` / `"Node"`; not parent Seq Scan for hits |
| `issue331_concurrent_reprocess_pool_stable` | GH-331 | PG stress | 130k-class optional / mix-scale; 8 concurrent count calls | No pool acquire timeout; health OK |
| `issue331_parity_count_vs_discovery` | GH-331 | PG | Known prefixes | count == discovery len |
| `issue319_failed_filter_beyond_page_size` | GH-319 | API | 120 completed + 5 older failed | `?status=failed` → 5 items; counts.failed=5 |
| `issue319_fe_failed_filter_lists_rows` | GH-319 | Playwright | Same seed via API | Failed chip → rows visible |
| `issue319_status_query_honored_openapi` | GH-319 | Contract | OpenAPI schema | `status` on list_documents |

---

## Sprint 1

| Test ID | Issue | Type | Setup | Assert |
|---------|-------|------|-------|--------|
| `issue317_batch_delete_200_opcount` | GH-317 | PG opcount | 200 docs with graph nodes | Batch delete completes; discovery ops ≪ 200 full scans |
| `issue317_unselected_remain` | GH-317 | API | 10 docs; delete 3 | 7 remain |
| `issue317_playwright_toolbar_bulk_delete` | GH-317 | Playwright | Multi-select | Confirm → gone; single progress track |
| `issue255_gateway_slash_model_not_rewritten` | GH-255 | Unit/API | custom base + slash model | effective_model unchanged |
| `issue255_llm_full_id_no_double_prefix` | GH-255 | Unit | provider+slash model | full_id not double-prefixed |
| `issue255_local_model_on_openai_still_guarded` | GH-255 | Unit | no custom base + gemma | mismatch true / rewrite |

---

## Sprint 2

| Test ID | Issue | Type | Setup | Assert |
|---------|-------|------|-------|--------|
| `issue318_track_not_complete_until_expected` | GH-318 | API | expected=3; 1 done | `is_complete=false` |
| `issue318_query_banner_during_batch` | GH-318 | Playwright | Multi-upload in flight | Query banner visible |
| `issue318_query_ready_after_batch` | GH-318 | Playwright | Batch terminal | Banner cleared |
| `issue316_two_workspaces_interleaved_progress` | GH-316 | Tasks PG | Same tenant; A backlog; B short | B processes before A drains |
| `issue316_tenant_cap_still_holds` | GH-316 | Tasks | Max caps | in-flight ≤ tenant max |
| `issue316_claim_sql_uses_workspace_index` | GH-316 | PG | Schema | Supporting index present / used |

---

## Regression watchlist

| Risk | Watch |
|------|-------|
| M070 re-adds parent indexes | Forbid in review |
| FE reintroduces client-only status filter without server filter | #319 |
| Bulk delete FE regresses to N× mutate | #317 |
| COMPAT-GUARD again treats all `/` as mismatch | #255 |
| Track complete without expected_count | #318 |
| claim_next drops SKIP LOCKED | #316 |

---

## Suggested file placement (when implementing)

| Area | Path hint |
|------|-----------|
| Storage/EXPLAIN | `edgequake/crates/edgequake-storage/tests/` or existing `e2e_spec054_mix_scale_perf.rs` update |
| API list/delete | `edgequake/crates/edgequake-api/tests/` |
| Tasks fairness | `edgequake/crates/edgequake-tasks/tests/` |
| Playwright | `edgequake_webui/e2e/issue3xx-*.spec.ts` |
