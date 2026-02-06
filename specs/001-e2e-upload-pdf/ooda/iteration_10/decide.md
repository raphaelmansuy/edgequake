# OODA-10 Decide: Implementation Plan

## Changes

1. Create `e2e_clean_tenant.rs` test file with `TestContext` helper
2. Implement 9 test cases:
   - `test_clean_tenant_isolation` - prove tenants are unique
   - `test_document_upload_clean_tenant` - upload + retrieve
   - `test_entity_extraction_clean_tenant` - entity counts
   - `test_query_clean_tenant` - RAG query with answer field
   - `test_document_upload_timeout_30s` - OODA-11 timeout
   - `test_query_timeout_30s` - OODA-11 timeout
   - `test_multiple_documents_same_tenant` - multi-doc
   - `test_tenant_with_model_config` - SPEC-032 model config propagation
   - `test_data_isolation_between_contexts` - prove isolation between AppState instances

## Risks

- Mock provider may change extraction behavior → use flexible assertions
- Workspace pipeline issue documented for future production test suite
