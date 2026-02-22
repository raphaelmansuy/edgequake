# Changelog (sdks)

All notable changes to the EdgeQuake SDKs directory are tracked here. See the root CHANGELOG.md for workspace-wide changes.

## [Unreleased]

## [0.4.0] - 2025-07-15

### Added

- **PDF vision pipeline** (`enable_vision`, `vision_provider`, `vision_model` upload options). Renders each PDF page to a high-res image and passes it to a multimodal LLM for high-fidelity Markdown extraction. Handles scanned documents, complex tables, and OCR-heavy content that classic text-layer extraction fails on.
- `PdfUploadOptions` type across Python, TypeScript, and Rust SDKs for structured vision options.
- `extraction_method` field on `PdfInfo` / `PdfStatusResponse` — returns `"vision"`, `"text"`, or `"ocr"` so callers can know which pipeline processed the document.
- `force_reindex` flag on PDF upload to re-process duplicates.
- Rust SDK: `PdfResource.upload()` with full multipart form-data support (was previously missing).
- Rust SDK: `PdfResource.list()` and `PdfResource.get()` methods.
- Rust SDK: `PdfInfo` response type with `extraction_method`.
- Rust SDK: `PdfUploadResponse.canonical_id()` helper — resolves `pdf_id` or legacy `id` regardless of server version.
- TypeScript SDK: `PdfUploadOptions` interface replaces `Record<string, string>` on `PdfResource.upload()`.
- Python SDK: `PdfResource.upload()` accepts individual vision keyword args or a `PdfUploadOptions` dataclass.
- Python SDK: `PdfUploadOptions` exported from top-level `edgequake` package.

### Changed

- Python `PdfUploadResponse`: primary field renamed from `id` to `pdf_id`; old `id` attribute kept for backward compatibility via `model_post_init`.
- Python `PdfInfo`: `file_name` (server) aliased to `filename` (SDK); both work.
- SDK versions bumped to `0.4.0`: Python, TypeScript, Rust, Ruby, Kotlin, Java, C#.
- Go and Swift SDKs version via git tag (no source version constant).

### Added (to CHANGELOG)

- CHANGELOG.md for SDKs directory.
