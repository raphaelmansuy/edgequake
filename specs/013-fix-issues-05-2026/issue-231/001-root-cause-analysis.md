# Issue #231 — Root Cause Analysis

**GitHub:** [#231](https://github.com/raphaelmansuy/edgequake/issues/231)

## Symptom (fact)

Swagger shows no `X-Workspace-ID` on document/PDF upload; reporter cannot scope uploads to workspace.

## 5 WHY

| # | Why | Evidence |
|---|-----|----------|
| 1 | Why can't users pass workspace in Swagger? | `#[utoipa::path]` on upload handlers lacked `params` for headers |
| 2 | Do handlers support the header? | Yes — `TenantContext` on `upload_file`, `upload_pdf_document`, `upload_document` |
| 3 | Why batch upload broken? | `upload_files_batch` hardcoded `workspace_id = "default"` and omitted `TenantContext` |
| 4 | Why OpenAPI global security not enough? | Workspace header is optional per-endpoint documentation in utoipa |
| 5 | Why isolation matters? | `ContentHasher::workspace_hash_key(workspace_id, ...)` scopes dedup/storage |

## Proof

- PDF handler logs `context.workspace_id` ([upload.rs:75-77](edgequake/crates/edgequake-api/src/handlers/pdf_upload/upload.rs))
- Batch had `let workspace_id = "default".to_string()` ([batch_upload.rs:67](edgequake/crates/edgequake-api/src/handlers/documents/upload/batch_upload.rs))

## Fix summary

1. Add OpenAPI header `params` on all upload paths  
2. Wire `TenantContext` into batch upload; use `workspace_id_or_default()`
