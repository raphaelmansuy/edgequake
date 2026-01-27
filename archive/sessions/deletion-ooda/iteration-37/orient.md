# OODA-37: Orient

## Analysis

**Gap Type**: Testing Gap - Workspace isolation for deletion
**Priority**: MEDIUM

## Existing Coverage

- `e2e_tenant_isolation.rs`: Tests tenant/workspace isolation for uploads and queries
- `e2e_document_workspace_provider.rs`: Tests workspace provider isolation

## Missing Coverage

Specific deletion isolation scenarios:

1. Deleting document only removes it from its workspace
2. Same-named document in different workspaces: delete one, other remains

## Decision

Add 2 workspace isolation tests for deletion.
