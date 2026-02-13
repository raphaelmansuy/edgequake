# Observation - Iteration 21

## Focus: SDK E2E Tests for Lineage/Metadata

## Files Examined

- `sdks/rust/tests/e2e_tests.rs` (245 lines) — Rust SDK E2E tests behind `#[cfg(feature = "e2e")]`
- `sdks/typescript/tests/e2e/documents.test.ts` — TS SDK E2E document tests
- `sdks/typescript/tests/e2e/helpers.ts` — TS E2E helper utilities
- `sdks/python/tests/test_e2e.py` (304 lines) — Python SDK E2E tests with `pytest.mark.skipif`

## Current State

- Rust SDK: 15 E2E tests existed but NONE for lineage/metadata
- TypeScript SDK: E2E tests for health, documents, graph, query, etc. but NONE for lineage
- Python SDK: 14 E2E test classes but NONE for lineage/metadata
- Mission explicitly states: "Ensure there is e2e test for metadata for each SDK."

## Test Patterns

- Rust: `#[cfg(feature = "e2e")]` gate, `EDGEQUAKE_BASE_URL` env var
- TypeScript: `E2E_ENABLED` flag from `EDGEQUAKE_E2E_URL` env var, vitest
- Python: `pytest.mark.skipif(not E2E_URL)`, shared `test_doc_id` fixture
