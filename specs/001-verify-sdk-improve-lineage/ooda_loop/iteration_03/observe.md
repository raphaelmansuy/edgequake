# OODA Iteration 03 — Observe/Orient/Decide/Act: Add Lineage Export to Python SDK

**Date**: 2026-02-13

## Observe
- Python SDK missing `export_lineage()` method for `/api/v1/documents/{id}/lineage/export`
- Backend supports both JSON and CSV export formats (routes.rs:269)
- No tests exist for `get_lineage`, `get_metadata`, or `export_lineage` on documents resource

## Orient
- This is a critical lineage coverage gap — export enables compliance audit trails 
- JSON/CSV export requires raw bytes return (not JSON parsing)
- Need both sync and async implementations

## Decide
- Add `export_lineage()` to both `DocumentsResource` (sync) and `AsyncDocumentsResource` (async)
- Add 5 new tests: get_lineage, get_metadata, export_json, export_csv, export_default_format

## Act
### Changes Made
- `sdks/python/edgequake/resources/documents.py` — Added `export_lineage()` to sync (line ~256) and async (line ~510) classes
- `sdks/python/tests/test_resources_documents.py` — Added `TestDocumentLineageMethods` class with 5 tests

### Test Results
- Before: 435 passed, 32 skipped
- After: **440 passed**, 32 skipped (+5 new tests)
