# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

#### Cooperative Pipeline Cancellation

- **`CancellationRegistry`** (`edgequake-tasks/src/cancellation.rs`): New per-task cooperative cancellation using `tokio_util::sync::CancellationToken`. Each running task gets a unique token registered at worker start and deregistered on completion.
- **`cancel_task` handler** (`handlers/tasks.rs`): `POST /tasks/{track_id}/cancel` now triggers the token, causing the pipeline to exit at the next cancellation gate instead of waiting for the current LLM call to finish.
- **6 cancellation gates in `text_insert.rs`**: Before chunking, after chunking, before extraction, after extraction, before embedding, and after storage — each calls `check_cancelled()`.
- **2 cancellation gates in `pdf_processing.rs`**: After PDF-to-markdown conversion and after vision extraction.
- **Per-chunk + per-retry cancellation** (`extraction.rs`, `processing.rs`): Entity extraction loop and resilience retry loop check the token between iterations.
- **Shared registry** (`main.rs`): `CancellationRegistry` is shared between `WorkerPool` and `AppState` so the cancel API endpoint and the worker pool use the same token store.

### Fixed

#### Undeletable Documents with KV Key/ID Mismatch

- **`resolve_kv_key_prefix()`** (`delete/single.rs`): New two-phase resolution — fast path checks `{id}-metadata` directly, slow path scans all metadata keys for matching JSON `id` field. Handles historical data where the KV key prefix diverged from the metadata JSON `id`.
- **Comprehensive key cleanup** (`delete/single.rs`): Delete now collects ALL keys under both the resolved KV prefix and the JSON id prefix (catches lineage, checkpoint, and other auxiliary keys).
- **Source prefix matching** (`delete/single.rs`): Graph entity/edge source filtering uses both prefixes in mismatch cases to prevent orphaned graph data.
- **Postgres cascade** (`delete/single.rs`): `delete_document_record` tries both UUIDs (KV prefix and JSON id) when they differ.
- **6 unit tests** covering fast-path resolution, mismatch resolution, not-found, full cascade with mismatch, lineage key cleanup, and 404 for truly nonexistent documents.

#### Clippy

- **`pipeline_checkpoint.rs`**: Fixed 3 `cloned_ref_to_slice_refs` warnings by using `std::slice::from_ref()` instead of `&[key.clone()]`.

## [0.5.4] - 2026-02-26

### Fixed

#### Dashboard KPIs: Accurate Document/Entity/Relationship Counts (closes #81)

- **Phase 1 — Stats endpoint** (`stats.rs`): Removed PostgreSQL-first fallback in `fetch_workspace_stats_uncached`. The endpoint now always uses KV storage for document counts and Apache AGE for entity/relationship counts, eliminating the premature short-circuit at `if stats.document_count > 0` that skipped the accurate data path.
- **Phase 2 — Dual-write** (`text_upload.rs`, `file_upload.rs`, `text_insert.rs`): Added `ensure_document_record` calls after document processing completes, so text/markdown/file uploads also populate the PostgreSQL `documents` table for consistency. Previously only PDF uploads called this function.

### Added

- **14 E2E test cases** (`e2e_dashboard_stats_issue81.rs`): Comprehensive regression tests covering empty workspace, mixed document types, entity/relationship counts, workspace isolation, cache contamination, orphan documents, chunk counts, storage bytes aggregation, response shape validation, and stress test (50 documents).

### Infrastructure

- Bumped version `0.5.3` → `0.5.4` in `Cargo.toml`, `VERSION`, and `package.json`.

## [0.5.3] - 2026-02-26

### Fixed

#### WebUI: Consistent API Base URL (closes #79)

- **`getPdfDownloadUrl()`** (`edgequake.ts`): Replaced incorrect `NEXT_PUBLIC_API_BASE_URL` env var with `SERVER_BASE_URL` (derived from `NEXT_PUBLIC_API_URL`), fixing PDF downloads that failed with `ERR_CONNECTION_REFUSED` in production when the non-standard env var was unset.
- **`exportDocumentLineage()`** (`edgequake.ts`): Same fix applied — lineage export downloads now use the same base URL as the rest of the API client.

### Infrastructure

- Bumped version `0.5.2` → `0.5.3` in `Cargo.toml`, `VERSION`, and `package.json`.

## [0.5.2] - 2026-02-26

### Fixed

#### Document Lifecycle & Cascade Delete (closes #73, #74)

- **FK constraint on PDF upload** (`pdf_processing.rs`): `ensure_document_record` now inserts into the `documents` table _before_ `pdf_documents`, preventing the foreign key violation that caused uploads to silently fail.
- **Cascade delete** (`single.rs`): Deleting a document now also removes the associated `pdf_documents` row and lets `ON DELETE CASCADE` clean up chunks and graph edges.
- **Status CHECK constraint** (`pdf_processing.rs`): Changed status value from `"completed"` (invalid) to `"indexed"` to satisfy the `documents_valid_status` CHECK constraint in migration 001/003.
- **UTF-8 boundary panic** (`pdf_processing.rs`): Markdown preview truncation (`&markdown[..65_536]`) now uses `char_indices()` to find a safe byte boundary, preventing panics on multi-byte characters.

#### Table Preprocessor Quality (SRP / DRY / Edge Cases)

- **Refactored `table_preprocessor.rs`** for single-responsibility: extracted `ParsedTable::from_lines()`, `group_rows_by_first_column()`, and `emit_grouped_sections()` as focused helper functions.
- **DRY**: Added `PreprocessResult::passthrough()` constructor to eliminate four identical block constructions.
- **Configurable title**: Replaced hard-coded `"Glossary / Data Dictionary"` with `document_title: Option<String>` field on `TablePreprocessorConfig`.
- **Separator false-positive fix**: `is_separator_line("| |")` no longer incorrectly returns `true` (guarded against `.all()` on empty iterators).
- **Test coverage**: Expanded from 9 → 30 tests covering: unicode grouping, deduplication toggle, truncation boundary, threshold semantics, alphabetical ordering, summary statistics, empty/whitespace inputs, mixed content, and more.

### Infrastructure

- Bumped version `0.5.1` → `0.5.2` in `Cargo.toml`, `VERSION`, and `package.json`.

## [0.5.1] - 2026-02-24

### Security

#### Tenant / Workspace Isolation (full audit)

- **`verify_workspace_tenant_access` helper** (`handlers/workspaces/helpers.rs`): Centralised guard that fetches a workspace by ID, checks that `workspace.tenant_id` matches the `X-Tenant-ID` request header, and returns **404** (not 403) on mismatch to prevent cross-tenant UUID enumeration. Access is permissive when the header is absent for backward-compat with admin/direct-API use.
- **Workspace CRUD** (`workspace_crud.rs`): `get_workspace`, `update_workspace`, and `delete_workspace` now require the workspace to belong to the requesting tenant before serving or mutating data.
- **Stats & metrics** (`stats.rs`): `get_workspace_stats` verifies tenant ownership **before** consulting the in-memory cache — cross-tenant requests never receive cached data from workspaces they do not own. Same check applied to `get_metrics_history` and `trigger_metrics_snapshot`.
- **Bulk operations** (`rebuild_embeddings`, `rebuild_knowledge_graph`, `reprocess_all_documents`): Inline `BR0201` guard added to all three destructive/long-running handlers.

### Fixed

#### Workspace / Tenant UX

- **Auto-select after creation** (`tenant-workspace-selector.tsx`, `use-tenant-context.ts`): When a new workspace or tenant is created, it is immediately pushed into the Zustand store (`setWorkspaces` / `setTenants`) before `selectWorkspace()` / `selectTenant()` is called. This eliminates the race-condition window where the Select dropdown showed "Select workspace…" until the async React Query refetch delivered the new item. The fix is applied in both the sidebar `TenantWorkspaceSelector` component and the `useTenantContext` hook so all call-sites are consistent.

### Infrastructure

- Bumped `[workspace.package] version` in `edgequake/Cargo.toml`, `VERSION`, and `edgequake_webui/package.json` from `0.5.0` → `0.5.1`.

## [0.5.0] - 2026-02-25

### Added

#### Query UX Enhancements

- **Wider query/answer layout** (`query-interface.tsx`): Message area widened from `max-w-3xl` to `max-w-4xl lg:max-w-5xl`; assistant message container set to `max-w-full` for long tables and code blocks
- **Response language support** (full-stack): Backend detects the `language` field sent by the frontend (`ChatCompletionRequest`) and appends `[IMPORTANT: You MUST respond in {Language}]` to the query before the LLM call via `enrich_query_with_language()` — frontend passes `i18n.language` automatically
- **Mermaid syntax sanitization** (`MermaidBlock.tsx`): `sanitizeMermaidCode()` fully rewritten — auto-quotes labels that contain `(){}|><` or non-ASCII characters (e.g., `A[label (note)]` → `A["label (note)"]`), maps non-ASCII node IDs, and shows the sanitized source in the error view

#### Source Citations & Deep-Links

- **Chunk deep-link on sidebar selection**: Clicking a source chunk in the sidebar navigates to the exact file location via deep-link
- **Auto-resolve chunk line range on deep-link**: Content highlights auto-open and the Data Hierarchy panel reveals the referenced section
- **Improved source-citations UX**: Uniform scroll height, per-document chunk expand/collapse, count badges, and better contrast for citations

#### Developer / Architecture

- **Centralised tenant isolation** (`handlers/isolation.rs`): DRY/SRP refactor — all workspace/tenant security checks route through a single `isolation.rs` module, reducing duplication across handlers
- **Workspace-scoped rebuild**: Exclude cross-workspace documents from incremental rebuild scope

### Changed

- **Streaming markdown UX**: Larger text rendering, light/dark theme consistency, full-view dialogs for long code and Mermaid blocks
- **Router history on deep-link**: `router.push()` used for chunk navigation to preserve browser back-button history
- **Yellow chunk highlight + source-citations contrast**: Selected chunks highlighted in amber; citation rows have improved foreground/background contrast ratio

### Fixed

- **Chunk deep-link propagation**: `chunk_id` correctly propagated from query citations to URL parameters
- **`chunk_id` in `convertServerMessage`**: Historical messages now carry `chunk_id` so citations reopen correctly after page reload
- **Source-mapper `chunk_id` propagation**: Fixed `isolation.rs`, `lineage.rs`, and source-mapper to consistently pass `chunk_id` through pipeline
- **Table streaming flicker**: Eliminated double-render and flicker when a streamed response block transitions from partial to complete table
- **Accessibility, responsive design, smooth display**: ARIA labels, keyboard navigation, reduced-motion preference, and mobile breakpoints across the query UI

## [0.4.1] - 2026-02-23

### Added

#### Tenant & Workspace Model Configuration (SPEC-041 / SPEC-032)

- **Vision LLM selector in Create Tenant form**: Users can now set a default Vision LLM (filtered to vision-capable models) when creating a tenant — inherited by all new workspaces
- **Vision LLM selector in Create Workspace form**: Per-workspace override for the Vision LLM used in PDF-to-Markdown extraction
- **`filterVision` prop on `LLMModelSelector` and `ModelSelector`**: Restricts the dropdown to models with `supports_vision === true`
- **`vision_llm_model` / `vision_llm_provider` in `CreateWorkspaceRequest`** type: Workspace creation API now accepts Vision LLM fields (SPEC-041)

### Changed

- **LLM Model, Embedding Model, and Vision LLM are now required** in both Create Tenant and Create Workspace forms; the Create button is disabled until all three are selected and labels show a red `*`

## [0.4.0] - 2026-02-19

### Added

#### PDF → LLM Vision Pipeline (SPEC-040)

- **Vision-Based PDF Extraction** (FEAT1010): Multimodal LLM reads PDF page images directly — handles scanned docs, complex layouts, and tables where text extraction fails
- **Multi-Page Image Extraction** (FEAT1011): Each PDF page rendered to high-resolution images (up to 2048px), encoded as base64 and streamed to the vision LLM
- **LLM-Powered Layout Understanding** (FEAT1012): GPT-4o / Claude / Gemini vision models interpret page structure, resolve multi-column text, reconstruct tables
- **Automatic Fallback** (BR1010): If vision extraction fails (quota, timeout, no vision model), the pipeline gracefully falls back to pdfium text extraction
- **Resolution Capping** (BR1011): Image DPI capped at 300 / max-side 2048px to balance quality vs. token cost
- **Zero-Config pdfium**: Switched to `edgequake-pdf2md` 0.4.1 – pdfium binary now embedded; no `PDFIUM_DYNAMIC_LIB_PATH` env var required
- **ExtractionMethod field on Block**: Each extracted block carries `vision`, `text`, or `ocr` metadata for traceability
- **Config flag `use_vision_llm`**: Opt-in per-request; set on `PdfExtractConfig` or pass `X-Use-Vision: true` HTTP header

#### Improved Developer Experience

- `cargo build` now works out-of-the-box without downloading pdfium — CI shaved ~40 s
- `vision`, `image_ocr`, and `formula` sub-modules extracted into focused files for maintainability
- `ProgressCallback` wired through vision pipeline for live extraction progress in WebUI

### Changed

- Workspace version bumped to `0.4.0` across all crates
- `edgequake-pdf` crate internal refactor: layout, processors, renderers grouped into sub-modules
- Default extraction mode is still text (`use_vision_llm = false`); vision is opt-in to avoid unexpected LLM cost
- README "Experimental" PDF warning upgraded to "Production Ready (vision mode optional)"

### Fixed

- PDF pipeline `block_in_place` / `spawn` issues that caused `Send` bound errors with async trait are fully resolved in 0.4.0
- PDFIUM path resolution in Docker images now works without manual env var

## [0.3.0] - 2025-02-17

### Added

#### Multi-Provider Support Expansion

- **9 Active Providers**: OpenAI, Anthropic, Google Gemini, xAI, OpenRouter, Ollama, LM Studio, Azure OpenAI, Mock
- **26 Model Configurations**: Comprehensive pricing data across all providers
- **Latest Model Support**:
  - Anthropic: Claude Opus 4.6, Sonnet 4.5, Haiku 4.5 (200K context, 128K max output)
  - xAI: Grok 4.1 Fast, Grok 4.0, Grok 3, Grok 3 Mini (up to 2M context)
  - Google Gemini: 2.5 Pro, 2.5 Flash, 2.5 Flash Lite, 2.0 Experimental (thinking capabilities)
  - OpenAI: o4-mini (reasoning model), o4, o1-2024-12-17

#### Cost Tracking Enhancements

- Updated pricing for 26 models (Feb 2025 verified rates)
- Expanded `default_model_pricing()` from 10 to 26 entries
- Added pricing for embedding models: text-embedding-3-small, gemini-embedding-001
- Cost tracking infrastructure fully seeded with latest pricing data

#### Provider Configuration

- Updated default models for all providers in safety limits
- Enhanced provider metadata with latest model information
- Improved WebUI configuration snippets with current models
- Auto-detection priority order for cloud providers

#### Lineage Tracking & Metadata (OODA-01 through OODA-25)

- Chunk position metadata: `start_line`, `end_line`, `start_offset`, `end_offset` fields (OODA-01)
- Chunk model tracking: `llm_model`, `embedding_model`, `embedding_dimension` fields (OODA-02)
- Document lineage metadata: `document_type`, `file_size`, `sha256_checksum`, `pdf_id`, `processed_at` fields (OODA-03)
- PDF↔Document bidirectional linking with `pdf_id` in document metadata (OODA-04)
- Lineage tracking enabled by default (`enable_lineage_tracking = true`)
- `GET /api/v1/chunks/{id}/lineage` — Chunk lineage with parent refs (OODA-08)
- `GET /api/v1/documents/{id}/lineage/export?format=json|csv` — Download lineage as file (OODA-22)
- In-memory TTL cache (120s, 500 entries max) for lineage queries (OODA-23)
- Enhanced metadata component with KV storage fields (OODA-12)
- Document hierarchy tree: Document → Chunks → Entities (OODA-13)
- Lineage export buttons (JSON/CSV download) in metadata sidebar (OODA-24)
- **TypeScript SDK**: `documents.getLineage()`, `getMetadata()`, `chunks.getLineage()` (OODA-15)
- **Python SDK**: Same methods on sync and async resource classes (OODA-16)
- E2E tests for lineage/metadata in all 3 SDKs (OODA-21)
- `docs/operations/metadata-debugging.md` — Diagnostics & repair guide (~260 lines) (OODA-20)
- "Unfiled" filter for conversations: displays all conversations not assigned to a folder
- Frontend and backend support for filtering by unfiled conversations

### Changed

#### Model Catalog Updates

- **Anthropic**: Updated to Claude 4.x series (Opus 4.6, Sonnet 4.5, Haiku 4.5)
- **xAI**: Updated to Grok 4.x/3.x series with 2M context models
- **Gemini**: Updated to 2.5 series with thinking capabilities
- **OpenAI**: Added o4-mini reasoning model, updated context limits
- **LM Studio**: Changed default from gemma2-9b-it to gemma-3n-e4b-it
- **OpenRouter**: Updated model references to latest versions

#### Default Model Changes

- Anthropic: claude-3-5-sonnet-20241022 → claude-sonnet-4-5-20250929
- xAI: grok-beta → grok-4-1-fast
- Gemini: gemini-1.5-pro → gemini-2.5-flash
- LM Studio: gemma2-9b-it → gemma-3n-e4b-it

#### Other Changes

- `sources_to_message_context()` uses `file_path` (then `document_id`) for source title instead of `source_type`
- Added `resolve_chunk_file_paths()` helper in query handler for reusable document name resolution from KV metadata
- **SDK updates**: Added `file_path` field to Rust, Java, and Kotlin SDK source reference types (Python and TypeScript already had it)
- Updated workspace version to 0.2.4
- Improved PATCH semantics for nullable fields in API and storage layers
- Refactored embedding batch calculation to use `.div_ceil()` (clippy compliance)
- Fixed consecutive `str::replace` calls in build scripts (clippy compliance)

### Fixed

- **Query/Chat source references show "chunk" instead of document name**: `sources_to_message_context()` was using `source_type` (always `"chunk"`) as the title. Now resolves `document_id` to actual document title from KV metadata. Affects `/api/v1/query`, `/api/v1/chat/completions`, and streaming endpoints
- **WebUI stored conversations**: Frontend `convertServerMessage` now uses `title` as fallback for `file_path` when displaying source citations from persisted conversations
- PATCH API for conversations now correctly distinguishes between "no change", "set to null", and "set to value" for folder assignment using `Option<Option<Uuid>>` pattern
- Moving conversations to/from folders now works reliably (E2E tested)
- Test assertions for LM Studio default model
- Provider status card configuration snippets
- Cost tracking consistency across all providers
- TypeScript build error in dashboard: removed non-existent `entity_type_count` property reference
- Visual feedback for tenant/workspace switching in the knowledge graph view

### Deprecated

- **gpt-4-turbo**: Superseded by gpt-4o and o4-mini (still functional, marked deprecated)
- **gpt-3.5-turbo**: Superseded by gpt-4o-mini (still functional, marked deprecated)

### Removed

- **gpt-oss:20b**: Removed from default model catalog

### Migration Notes

- No database migrations required for multi-provider support - cost tracking infrastructure already in place
- Existing cost data remains valid
- New pricing automatically applies to future operations
- Provider configurations are backwards compatible
- Lineage/metadata KV keys (`{id}-lineage`, `{id}-metadata`) only populated for newly processed documents
- Existing documents continue to work; lineage data appears after reprocessing

### Breaking Changes

None - all changes are additive or deprecations with backwards compatibility

## [v0.2.1] - 2026-02-12

### Fixed

- Fixed TypeScript build error in dashboard: removed non-existent `entity_type_count` property reference
- Visual feedback for tenant/workspace switching in the knowledge graph view

## [v0.2.4] - 2026-02-17

### Added

- "Unfiled" filter for conversations: displays all conversations not assigned to a folder
- Frontend and backend support for filtering by unfiled conversations

### Fixed

- PATCH API for conversations now correctly distinguishes between "no change", "set to null", and "set to value" for folder assignment using `Option<Option<Uuid>>` pattern
- Moving conversations to/from folders now works reliably (E2E tested)

### Changed

- Updated workspace version to 0.2.4
- Improved PATCH semantics for nullable fields in API and storage layers

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
