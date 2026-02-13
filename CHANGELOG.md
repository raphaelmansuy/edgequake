# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added — Lineage Tracking & Metadata (OODA-01 through OODA-25)

#### Core Types
- Chunk position metadata: `start_line`, `end_line`, `start_offset`, `end_offset` fields (OODA-01)
- Chunk model tracking: `llm_model`, `embedding_model`, `embedding_dimension` fields (OODA-02)
- Document lineage metadata: `document_type`, `file_size`, `sha256_checksum`, `pdf_id`, `processed_at` fields (OODA-03)
- All new fields are `Option<T>` with `serde(default)` for backward compatibility (T5)

#### Pipeline & Storage
- PDF↔Document bidirectional linking with `pdf_id` in document metadata (OODA-04)
- Chunk metadata propagation to KV and vector storage (OODA-05)
- Lineage persistence to KV storage under `{document_id}-lineage` key (OODA-06)
- Lineage tracking enabled by default (`enable_lineage_tracking = true`)

#### API Endpoints
- `GET /api/v1/documents/{id}/lineage` — Complete document lineage tree (OODA-07)
- `GET /api/v1/documents/{id}/metadata` — All metadata in single response (OODA-07)
- `GET /api/v1/chunks/{id}/lineage` — Chunk lineage with parent refs (OODA-08)
- `GET /api/v1/documents/{id}/lineage/export?format=json|csv` — Download lineage as file (OODA-22)
- In-memory TTL cache (120s, 500 entries max) for lineage queries (OODA-23)
- OpenAPI/utoipa annotations for all new endpoints

#### WebUI
- TypeScript types for full lineage responses (OODA-10)
- React Query hooks: `useDocumentFullLineage`, `useDocumentMetadata` (OODA-11)
- Enhanced metadata component with KV storage fields (OODA-12)
- Document hierarchy tree: Document → Chunks → Entities (OODA-13)
- Lineage export buttons (JSON/CSV download) in metadata sidebar (OODA-24)

#### SDKs
- **Rust SDK**: `documents().get_lineage()`, `get_metadata()`, `chunks().get_lineage()` (OODA-14)
- **TypeScript SDK**: `documents.getLineage()`, `getMetadata()`, `chunks.getLineage()` (OODA-15)
- **Python SDK**: Same methods on sync and async resource classes (OODA-16)
- E2E tests for lineage/metadata in all 3 SDKs (OODA-21)

#### Documentation
- `docs/architecture/lineage-tracking.md` — Complete lineage architecture (~280 lines) (OODA-17)
- `docs/api-reference/lineage-endpoints.md` — API reference for 7 endpoints (~360 lines) (OODA-18)
- `docs/tutorials/tracing-entity-sources.md` — Step-by-step tracing tutorial (~230 lines) (OODA-19)
- `docs/operations/metadata-debugging.md` — Diagnostics & repair guide (~260 lines) (OODA-20)

### Migration Notes

All changes are **additive and backward compatible**:
- New fields use `Option<T>` with `serde(default)` — old documents read fine
- New API endpoints don't change existing ones
- Lineage/metadata KV keys (`{id}-lineage`, `{id}-metadata`) only populated for newly processed documents
- Existing documents continue to work; lineage data appears after reprocessing

## [v0.2.2] - 2026-02-13

### Changed

- Updated workspace version to 0.2.2
- Refactored embedding batch calculation to use `.div_ceil()` (clippy compliance)
- Fixed consecutive `str::replace` calls in build scripts (clippy compliance)
- Feature gating improvements for minimal builds (query, core, storage)
- All clippy warnings resolved; workspace is clean
- Full test suite run: all tests passing

## [v0.2.1] - 2026-02-12

### Fixed

- Fixed TypeScript build error in dashboard: removed non-existent `entity_type_count` property reference
- Set entity types count to 0 as placeholder until backend implementation is complete

## [v0.2.0] - 2026-02-12

- Visual feedback for tenant/workspace switching in the knowledge graph view
- Loading overlay with minimum 800ms duration during workspace/tenant transitions
- Toast notifications for tenant and workspace switch confirmation
- Early return guard for same tenant/workspace selection (no-op)
- Toast deduplication using IDs to prevent duplicate notifications
- Loading overlay now always appears during workspace/tenant switch, even for empty/fast workspaces
- Only one toast notification is shown per switch (no duplicates)
- No notification or reload when selecting the same tenant/workspace
- See [SDKs documentation](sdks/) and [SDK changelogs](sdks/python/CHANGELOG.md, sdks/typescript/CHANGELOG.md, etc.) for language-specific updates.

---

## SDKs

EdgeQuake provides official SDKs for multiple languages. See the following for details and changelogs:

- [Python SDK](sdks/python/README.md) ([Changelog](sdks/python/CHANGELOG.md))
- [TypeScript SDK](sdks/typescript/README.md) ([Changelog](sdks/typescript/CHANGELOG.md))
- [Other SDKs](sdks/) for C#, Go, Java, Kotlin, PHP, Ruby, Rust, Swift

---

For a full project history, see the [README.md](README.md) and documentation in [docs/].
