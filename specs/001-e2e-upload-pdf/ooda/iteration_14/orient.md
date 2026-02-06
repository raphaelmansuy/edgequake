# OODA-14 Orient: Analysis

## Root Causes of Initial Failures

1. **test_reprocess_specific_document (422)**: Reprocess handler needs Task::new() with valid UUID tenant/workspace. Without X-Tenant-ID header, tenant_id defaults to "default" string which fails UUID parsing.

2. **test_multiple_uploads_consistent_graph (empty nodes)**: Incorrect assumption that mock provider produces graph entities. Mock returns plain text, not entity JSON.

## Solution

- Add valid UUID constants for tenant/workspace headers in reprocess tests
- Fix graph assertion to accept 0 nodes (consistent with e2e_pipeline_comprehensive pattern)
- Add WHY comments explaining the requirement for tenant headers
