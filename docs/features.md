---
title: 'EdgeQuake Feature Registry'
---

# EdgeQuake Feature Registry

This file maintains traceability between code features and business requirements.

## Index

| Feature ID | Description                                        | Status    | Spec / Issue         |
| ---------- | -------------------------------------------------- | --------- | -------------------- |
| FEAT-0001  | Tenant Workspace Quota Management                  | Completed | SPEC-0001 / #133     |
| FEAT-0002  | Knowledge Injection (Glossaries & Synonyms)        | Completed | SPEC-0002 / #131     |
| FEAT-0003  | Explainability                                     | Planned   | SPEC-0003 / #128     |
| FEAT-0004  | Graph Edge Labels                                  | Planned   | SPEC-0004 / #91      |
| FEAT-0005  | Custom Entity Configuration                        | Completed | SPEC-0005 / #85      |
| FEAT-006   | Unified Streaming Response Protocol                | Completed | SPEC-006 / #56       |
| FEAT-007   | Vector Storage SQL Pre-Filtering                   | Completed | SPEC-007             |
| FEAT-008   | Explicit Provider/Model Transparency in UI         | Completed | MISSION-01 / v0.9.19 |
| FEAT-009   | Document Deletion Correctness (MISSION-02)         | Completed | MISSION-02 / v0.9.19 |

## Feature Definitions

### FEAT-0002 — Knowledge Injection

**Issue**: [#131](https://github.com/raphaelmansuy/edgequake/issues/131)  
**Spec**: [specifications/0002_knowledge_injection_issue_131](../specifications/0002_knowledge_injection_issue_131/)  
**Released**: v0.8.0 (2026-04-03)  
**Status**: ✅ Completed

**Problem**: Domain-specific acronyms (OEE, NLP) and synonyms are unknown to the embedding model. Queries for "OEE" miss documents that say "Overall Equipment Effectiveness", degrading retrieval quality.

**Solution**: Workspace owners inject glossary definitions as named entries. These are processed through the standard entity-extraction pipeline, enriching the knowledge graph. At query time, injection entities expand the query terms. Injection entries are **never shown as source citations**.

**API Surface**:
- `PUT /api/v1/workspaces/:id/injection` — create/replace text injection
- `POST /api/v1/workspaces/:id/injection/upload` — upload file injection
- `GET /api/v1/workspaces/:id/injection` — list all entries
- `GET /api/v1/workspaces/:id/injection/:injection_id` — get detail
- `PATCH /api/v1/workspaces/:id/injection/:injection_id` — update name/content
- `DELETE /api/v1/workspaces/:id/injection/:injection_id` — delete + cascade cleanup

**UI**: `/knowledge` page with list, add dialog (text/file tabs), detail page, inline edit, delete confirmation.

**Test Coverage**: 1 000+ line Rust E2E suite + 5 Playwright browser tests.

---

### FEAT-0005 — Custom Entity Configuration

**Issue**: [#85](https://github.com/raphaelmansuy/edgequake/issues/85)  
**Spec**: [specifications/0005_custom_entity_config_issue_85](../specifications/0005_custom_entity_config_issue_85/)  
**Released**: v0.8.0 (2026-04-03)  
**Status**: ✅ Completed

**Problem**: Default generic entity types are insufficient for domain-specific corpora (manufacturing, healthcare, legal, research, finance), reducing extraction recall and graph quality.

**Solution**: Workspace creation supports `entity_types` with preset-driven and custom configuration. Types are normalized and stored in workspace metadata, then automatically injected into extraction prompts per workspace.

**Capabilities**:
- Workspace-scoped `entity_types` in create-workspace API payload.
- Validation and normalization: trim, uppercase, space/hyphen to underscore, dedupe, max 50.
- Pipeline fallback to server defaults when no custom entity types are configured.
- Frontend selector with presets (General, Manufacturing, Healthcare, Legal, Research, Finance) and custom add/remove chips.
- Workspace detail page displays configured entity types.
- Full i18n labels in en/fr/zh.

**API Surface**:
- `POST /api/v1/tenants/:tenant_id/workspaces` accepts `entity_types`.
- Workspace response surfaces `entity_types` (from metadata JSONB).

**Test Coverage**:
- Rust integration coverage for workspace request/metadata threading.
- Playwright coverage for selector UX edge cases and workspace detail display.

---

### FEAT-008 — Explicit Provider/Model Transparency in UI

**Released**: v0.9.19 (2026-04-10)
**Status**: ✅ Completed

**Problem**: Users could not see what LLM/embedding/vision provider and model would actually run
when no workspace-level override was configured. The UI showed "Server Default" with no further
detail, hiding effective runtime configuration and making debugging silent mismatches harder.

**Solution**:
- Extended `/health` to include `providers.vision.{name, default_model}` using the same
  `default_vision_model_for_provider()` source of truth as the PDF upload path.
- Created `EffectiveProviderBadge` component with `resolveEffective*Config()` pure resolver
  functions that mirror the backend priority rules exactly (including the orphaned-model invariant).
- System Status card now shows all three provider rows with actual names and model identifiers.
- Workspace settings view-mode panels replaced "Server Default" with `EffectiveProviderBadge`.

**Key Implementation**:
- `edgequake-api`: `health_types.rs` (VisionProviderHealth), `health.rs` (vision field)
- `edgequake_webui`: `effective-provider-badge.tsx`, `system-status.tsx`, `workspace/page.tsx`

**Test Coverage**:
- `health_types.rs` extended tests asserting JSON `vision` field serialization.
- Playwright E2E spec: `e2e/vision-provider-model-mismatch.spec.ts`.

---

### FEAT-009 — Document Deletion Correctness (MISSION-02)

**Released**: v0.9.19 (2026-04-10)
**Status**: ✅ Completed

**Problem**: Bulk and single document deletion left behind orphaned data:
- **GAP-4**: Entity embeddings not deleted when graph nodes removed (dangling vector entries).
- **GAP-5**: Content-hash keys not cleared, preventing re-upload of the same file.
- **GAP-6**: In-flight tasks not cancelled, processors continued writing into cleared system.
- Stuck-document recovery used a removed function name (`cleanup_document_graph_data`).

**Solution**:
- `storage_helpers.rs` refactored to `cleanup_document_graph_data_single()` — DRY, workspace-
  isolated, 2-call graph efficiency.
- `bulk.rs` cancels in-flight tasks and clears content-hash keys during bulk delete.
- `single.rs` and `stuck.rs` updated to use renamed helper and include embedding cleanup.
- `ContentHasher` integrated into bulk delete path.

**Key Implementation**:
- `edgequake-api`: `storage_helpers.rs`, `delete/bulk.rs`, `delete/single.rs`, `recovery/stuck.rs`

**Test Coverage**:
- `tests/e2e_document_deletion.rs` — 742 line E2E test suite covering all gap scenarios.

---

**Last Updated**: 2026-04-10
**Total Features**: 9
