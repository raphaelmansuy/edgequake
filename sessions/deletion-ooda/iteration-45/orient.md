# OODA-45: Orient + Decide

## Analysis

Tenant context tests:
1. Document with tenant header
2. Deletion respects tenant context

## Action Plan

Add 2 tests:
1. `test_document_with_tenant_context` - Upload with X-Tenant-ID
2. `test_deletion_with_tenant_context` - Delete with tenant header

## Success Criteria

- Tests pass
- Total deletion tests: 64
