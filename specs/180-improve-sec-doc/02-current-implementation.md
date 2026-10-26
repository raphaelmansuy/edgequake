# 02 — Current implementation assessment

[Index](README.md) · Governing laws: [First principles](01-first-principles.md) · Responses: [Product](03-product-spec.md) · Delivery: [Plan](13-implementation-plan.md)

This is a static code assessment at the README baseline. A path plus symbol is the durable evidence locator; line numbers are snapshot hints. Security risks below are not claims of a reproduced production exploit. Existing SPEC-174 documents are historical design inputs and do not supersede current source.

## What is already valuable

- [Core evaluator](../../edgequake/crates/edgequake-core/src/clearance/decide.rs): pure `allows` and `allows_relabel`; rank dominance, compartment subset, uploader exception and steward-gated downgrades.
- [Compiler](../../edgequake/crates/edgequake-core/src/clearance/compile.rs): role grants combine, then deny rules subtract. `any_of` and `all_of` both accumulate subject tags; neither changes object subset semantics.
- [Migration 220](../../edgequake/migrations/220_spec174_document_clearance.sql): governed vocabulary, access profiles, membership assignments, document-label join rows, rank/array columns, restrictive RLS and indexes.
- [Clearance service](../../edgequake/crates/edgequake-api/src/services/clearance.rs): per-request scope, hidden-document guard, relabel child propagation and epoch increment in one transaction.
- [Existing UI](../../edgequake_webui/src/components/clearance/document-access-control.tsx): shared choice/checklist building blocks and a small access property inside preview; workspace vocabulary/profile management already exists.
- [Policy tests](../../edgequake/crates/edgequake-core/src/clearance/tests.rs): deny wins, subset, empty compartments, uploader, downgrade and enforcement-off scenarios. Preserve these as regression oracles.

## Findings and confidence

| ID | Observed code | User/security implication | Response |
|---|---|---|---|
| F-180-01 | [performFileUpload](../../edgequake_webui/src/lib/upload/perform-file-upload.ts), [PDF FormData](../../edgequake_webui/src/lib/upload/pdf-upload-form-data.ts), [UploadDocumentRequest](../../edgequake/crates/edgequake-api/src/handlers/documents_types/upload.rs) have no typed security-label path; [stamp_unverified](../../edgequake/crates/edgequake-api/src/services/clearance.rs) at ~527 supplies omission default | Upload cannot express the same governed label as property editing | R-180-01/07/08; WP-03/04 |
| F-180-02 | [DocumentAccessControl](../../edgequake_webui/src/components/clearance/document-access-control.tsx), `handleOpenChange`, submits a changed draft on close | Outside click/Escape can become a security mutation | R-180-02; WP-05 |
| F-180-03 | Same component gates on settings `can_manage`; [caller_can_manage](../../edgequake/crates/edgequake-api/src/handlers/clearance.rs) at ~175 returns owner/admin; `list_clearance_tags` at ~252 also requires tag manager | Ordinary writers cannot discover assignable tags through current catalog UI/API | R-180-05/06; WP-02 |
| F-180-04 | [summary.ts](../../edgequake_webui/src/lib/clearance/summary.ts) uses “Only you, until reviewed”; [accessToken](../../edgequake_webui/src/components/documents/document-table-row.tsx) at ~129 capitalizes raw words separately | Steward visibility is omitted; list/detail wording can disagree; tag AND rule is absent | R-180-04/13/18; WP-05 |
| F-180-05 | [PatchLabelsRequest](../../edgequake/crates/edgequake-api/src/handlers/clearance.rs) at ~113 has only classification and compartment IDs; [relabel](../../edgequake/crates/edgequake-api/src/services/clearance.rs) at ~562 reads old label before `begin`, no version or row lock | Concurrent edits can authorize against stale state and lose updates; not runtime-reproduced here | R-180-09/10; WP-01/02 |
| F-180-06 | `relabel` copies seven child tables, increments workspace epoch and records an audit row only for downgrade | Good transaction reuse; reason, before/after and all-change audit are missing in this path | R-180-11/12; WP-01/02 |
| F-180-07 | [ingest_admission.rs](../../edgequake/crates/edgequake-api/src/services/ingest_admission.rs), `provision_relational_document_shell` at ~46 inserts shell then calls stamp separately | Initial-label atomicity and worker races need integration proof; stamping later is not a sufficient new contract | R-180-07/17; WP-03 |
| F-180-08 | [document-preview-panel](../../edgequake_webui/src/components/documents/document-preview-panel.tsx) at ~293 renders Access only if classification exists | Legacy/null/off/error states have no access explanation | R-180-13; WP-05 |
| F-180-09 | [document-manager](../../edgequake_webui/src/components/documents/document-manager.tsx) at ~1028 filters current `documents` in memory, while `totalCount` is passed separately | Classification results/count can disagree with server pagination; no security-tag filter here | R-180-14; WP-06 |
| F-180-10 | [invalidateClearanceViews](../../edgequake_webui/src/lib/clearance/use-clearance.ts) broadly invalidates documents, document, graph and stats prefixes | Useful base, but no principal-specific key here and no guarantee stale content is removed before refetch | R-180-17/18; WP-05/07 |
| F-180-11 | [simulate_clearance](../../edgequake/crates/edgequake-api/src/handlers/clearance.rs) at ~757 accepts role IDs, returns up to 20 titles, and requires steward | Existing profile simulation cannot be used as document-specific audience preview or exact member count | R-180-03; WP-02 |
| F-180-12 | [routes](../../edgequake/crates/edgequake-api/src/routes.rs), [patch_document_labels](../../edgequake/crates/edgequake-api/src/handlers/clearance.rs) at ~819, [request_authorization](../../edgequake/crates/edgequake-api/src/services/request_authorization.rs) | Route gate rejects readonly writes; relabel uses old/new clearance without `require_tag_manager`, unlike the UI. Authority mismatch needs an explicit product decision | R-180-05; WP-02 |
| F-180-13 | [migration 220](../../edgequake/migrations/220_spec174_document_clearance.sql) has independent document/tag FKs; role tag references are arrays; delete handler relies on FK failure for in-use documents | Same-workspace association and role-only references need application/DB validation, not just a UUID FK | R-180-06/16; WP-01/02 |
| F-180-14 | [document-clearance.spec.ts](../../edgequake_webui/e2e/document-clearance.spec.ts) routes mock responses; [enforcement-off integration test](../../edgequake/crates/edgequake-api/tests/e2e_spec174_enforcement_off.rs) is limited to off-mode and optional DB | A hidden mocked title is not proof that RLS/retrieval/downloads enforce live labels | R-180-24; WP-07 |

## Surface inventory and integration seams

| Surface | Existing owner | Planned change |
|---|---|---|
| File selection/drop | [use-document-dropzone](../../edgequake_webui/src/hooks/use-document-dropzone.ts), [document-dropzone](../../edgequake_webui/src/components/documents/document-dropzone.tsx) | Queue files locally and show access review before network admission |
| Upload dispatch | [perform-file-upload](../../edgequake_webui/src/lib/upload/perform-file-upload.ts), [use-file-upload](../../edgequake_webui/src/hooks/use-file-upload.ts) | Carry an immutable per-file access snapshot across transports |
| Text/file/batch admission | [document_admission](../../edgequake/crates/edgequake-api/src/handlers/documents/upload/document_admission.rs) | Shared typed access admission; no metadata override |
| PDF admission | [pdf_upload/upload.rs](../../edgequake/crates/edgequake-api/src/handlers/pdf_upload/upload.rs), [ingest_admission](../../edgequake/crates/edgequake-api/src/services/ingest_admission.rs) | Label original, shell and task before exposure |
| MCP upload | [upload_session](../../edgequake/crates/edgequake-api/src/mcp/project/upload_session.rs), [upload_put](../../edgequake/crates/edgequake-api/src/handlers/mcp/upload_put.rs) | Validate at session creation and again at admission/finalization |
| Property editing | [document-access-control](../../edgequake_webui/src/components/clearance/document-access-control.tsx) | Reuse form body; explicit Save; conflict handling |
| Library list/filter | [document-manager](../../edgequake_webui/src/components/documents/document-manager.tsx), [document-filters](../../edgequake_webui/src/components/documents/document-filters.tsx) | Server predicates before count/page; common formatter |
| Settings | [access-section](../../edgequake_webui/src/components/settings/sections/access-section.tsx) | Govern vocabulary/profiles; contextual return to unsaved editor |
| Retrieval and cache | [document_filter_resolver](../../edgequake/crates/edgequake-api/src/handlers/query/document_filter_resolver.rs), [retrieval_id_cache](../../edgequake/crates/edgequake-api/src/services/retrieval_id_cache.rs) | Revalidate epoch/scope and prove no derived-content leaks |

## Scope boundary

This pack specifies upload labeling, property editing, controlled bulk edits, visible status, accessible presentation and the backend guarantees needed to make them truthful. It does not add anonymous sharing, arbitrary ABAC expressions, SSO attribute synchronization, retention, DRM/encryption, AI auto-labeling or per-user share ACLs. Those require separate policy decisions. See [the six lenses](README.md#six-decision-lenses).
