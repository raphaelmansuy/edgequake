# OODA-12 Decide: Data Model Validation Test Suite

## Decision
Create comprehensive E2E tests that validate the actual HTTP response shapes
from all major API endpoints, with timeout enforcement.

## Test Matrix

| Test | Endpoint | Validates |
|------|----------|-----------|
| test_upload_request_defaults | POST /documents | Default serde values |
| test_upload_empty_content_rejected | POST /documents | Empty content → error |
| test_upload_whitespace_content_rejected | POST /documents | Whitespace → error |
| test_upload_missing_content_rejected | POST /documents | Missing field → 400/422 |
| test_upload_response_structure | POST /documents | All response fields present |
| test_document_detail_response_structure | GET /documents/{id} | Detail fields + status consistency |
| test_unicode_content_handling | POST /documents | CJK, emoji, accents |
| test_metadata_special_characters | POST /documents | HTML entities, paths, unicode keys |
| test_list_documents_pagination_structure | GET /documents | Pagination fields |
| test_graph_response_structure | GET /graph | nodes/edges arrays |
| test_query_response_structure | POST /query | answer/mode/sources/stats |
| test_tenant_response_model_config | POST /tenants | SPEC-032 model config fields |
| test_health_response_structure | GET /health | status/version/components |
| test_delete_response_structure | DELETE /documents/{id} | Cascade counts |
| test_deletion_impact_preview_only | GET /documents/{id}/deletion-impact | preview_only=true |
| test_cost_estimation_response_fields | POST /pipeline/costs/estimate | Cost fields |
| test_get_nonexistent_document_404 | GET /documents/{uuid} | 404 status |
| test_delete_nonexistent_document_404 | DELETE /documents/{uuid} | 404 status |

## Files to Create
1. `edgequake/crates/edgequake-api/tests/e2e_data_model.rs` — 18 tests

## Risk Assessment
- **Zero risk**: New test file only, no modifications
- **High signal**: Validates actual API contract shapes
