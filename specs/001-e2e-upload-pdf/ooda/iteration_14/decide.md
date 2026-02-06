# OODA-14 Decide: Actions

1. Add TEST_TENANT_ID and TEST_WORKSPACE_ID UUID constants
2. Add X-Tenant-ID and X-Workspace-ID headers to ALL reprocess requests
3. Change graph node assertion from `!is_empty()` to structure-only validation
4. Run full regression (504 tests)
