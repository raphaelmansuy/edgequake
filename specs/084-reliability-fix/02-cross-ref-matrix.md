# SPEC-084 — Cross-Reference Matrix

> Issue ↔ primary files ↔ laws ↔ sprint ↔ e2e IDs

| ID | Primary files | Laws | Sprint | E2E IDs |
|----|---------------|------|--------|---------|
| GH-331 | `edgequake-storage/.../analytics_ops.rs`; `scan_ops.rs` (pattern); `migrations/support/038/apply.sql`; `m038.rs`; `document_read_model.rs` | 9,12,8 | 0 | `issue331_node_counts_uses_child_gin_explain`; `issue331_concurrent_reprocess_pool_stable`; `issue331_parity_count_vs_discovery` |
| GH-319 | `handlers/documents_types/listing.rs`; `handlers/documents/query/list.rs`; `edgequake-core/.../budget.rs`; `edgequake_webui/.../document-manager.tsx`; `use-document-filtering.ts`; `use-document-queries.ts` | 10,3,8 | 0 | `issue319_failed_filter_beyond_page_size`; `issue319_fe_failed_filter_lists_rows`; `issue319_status_query_honored_openapi` |
| GH-317 | `document-manager.tsx` (bulk confirm); `handlers/documents/delete/single.rs`; `delete/bulk.rs` (wipe contrast); `workspace_document_wipe.rs`; `edgequake-tasks` lifecycle fairness | 12,9,8 | 1 | `issue317_batch_delete_200_opcount`; `issue317_unselected_remain`; `issue317_playwright_toolbar_bulk_delete` |
| GH-255 | `edgequake-api/.../safety_limits.rs`; `edgequake-core/.../workspace.rs` (`llm_full_id`); PR #229 | 14,3,8 | 1 | `issue255_gateway_slash_model_not_rewritten`; `issue255_llm_full_id_no_double_prefix`; `issue255_local_model_on_openai_still_guarded` |
| GH-318 | `track_status.rs`; `use-file-upload.ts`; `query-interface.tsx`; `workspace-status-footer.tsx`; `batch-progress-card.tsx` | 11,3,8 | 2 | `issue318_track_not_complete_until_expected`; `issue318_query_banner_during_batch`; `issue318_query_ready_after_batch` |
| GH-316 | `edgequake-tasks/.../postgres.rs` (`claim_next`); `tenant_limiter.rs`; `worker.rs`; `pipeline/config.rs` local caps | 13,3,8 | 2 | `issue316_two_workspaces_interleaved_progress`; `issue316_tenant_cap_still_holds`; `issue316_claim_sql_uses_workspace_index` |

---

## Dependency graph

```
  M038/M070/SPEC-071 -----> GH-331 (retarget only)
  #309 WorkspaceWipe -----> GH-317 (pattern reuse, not a fix)
  PR #229 ----------------> GH-255 (absorb)
  GH-331 pool health -----> GH-317 scale delete (shared failure class)
  LAW-10 list SSOT -------> GH-319
  LAW-11 batch track -----> GH-318
  LAW-13 fairness --------> GH-316
```

---

## Explicit non-dependencies

| Claim | Reality |
|-------|---------|
| Parent GIN closes #331 | **False** — wrong table |
| #309 closes #317 | **False** — wipe-all ≠ selected IDs |
| factory.rs prefix skip closes #255 | **Insufficient** — factory already passthrough; fix COMPAT-GUARD |
| CopilotKit closes #318 | **False** — not in product |
