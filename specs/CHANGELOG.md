# Changelog (specs)

All notable changes to the EdgeQuake specs directory are tracked here. See the root CHANGELOG.md for workspace-wide changes.

## [Unreleased]

### Added

- CHANGELOG.md for specs directory.
- `specs/021-storage-study/06-first-principles/19-ingestion-query-improvement-plan.md` §11:
  multi-perspective assessment (GraphRAG / LightRAG / AI Engineer / System Engineer)
  of the implemented P-G1, P-G3, P-G6, P-G2b changes, with a verification matrix.
- P-G7: index-friendly KV scans. `keys()` + in-memory filter replaced by
  `keys_with_prefix` / `keys_with_suffix` in `reprocess.rs`, `pdf_processing.rs`,
  `stuck.rs`, `storage_helpers.rs`, and `delete/single.rs` (incl. a rewritten
  `resolve_kv_key_prefix`).
- P-G9: query embedding cache. New
  `edgequake-query/src/cache/embedding_cache.rs` (`CachingEmbeddingProvider`,
  LRU 10k / 1h TTL, model identity folded into the key) wired into the
  production query engine via `QueryEngine::with_embedding_cache`. Contract
  tests in `edgequake-query/tests/contract_embedding_cache.rs`.
- P-G11: streaming vision parity. `stream_answer_from_context` now delegates
  image-attached requests to `stream_vision_answer` (vision `chat` path with
  E30 text fallback). Contract tests in
  `edgequake-query/tests/contract_streaming_vision.rs`.
- P-G1b: legacy entity reconciliation. New
  `edgequake-storage/src/entity_reconcile.rs` (dry-run `plan` + confirm-token
  `execute`, idempotent) and admin endpoints
  `GET/POST /api/v1/admin/entities/reconcile` in `admin.rs` + `routes.rs`.
  5 unit tests covering E5/E6/E7, edge rewrite, vector re-key, idempotency.

### Changed

- Marked P-G1 (EntityId newtype) and P-G3 (Global N+1 fix) as ✅ DONE/TESTED in
  plan-19, with code-level evidence and acceptance-test verification notes.
- Marked P-G7, P-G9, P-G11, P-G1b as ✅ DONE/TESTED in plan-19.
- Marked P-G2 as ◑ PARTIAL: P-G2b + shared EntityId/batch/compensation
  invariants are done across both remaining persistence paths; the literal
  `IngestionPersister` trait extraction is deferred (structural DRY only — no
  correctness delta) and documented in plan-19.

### Fixed

- `edgequake-api/src/handlers/documents/recovery/reprocess.rs`: gated the new
  empty-markdown fallback's `state.storage.pdf_storage` access behind
  `#[cfg(feature = "postgres")]` to match the pre-existing pattern at line 444.
  Without this, `cargo build -p edgequake-api --lib --no-default-features` failed
  (feature-gating regression caught during pre-commit verification).
- `edgequake-api/src/error.rs`: added the `StorageError::InvalidInput` arm to
  `storage_error_category` (new variant introduced by P-G1b's confirm-token
  error path).
