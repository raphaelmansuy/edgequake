# Iteration 17: Orient — TypeScript & Python SDK Fix

## Analysis

- Chat API mismatch is the root cause of all SDK chat test failures
- Fix requires changing request type (`message` string) and response type (flat object)
- Conversation/folder tests can use default tenant/user IDs instead of requiring env vars
- Python uses `EDGEQUAKE_E2E_URL`, TypeScript uses `EDGEQUAKE_E2E_URL` for E2E base URL

## Approach

1. Fix chat request/response types in both SDKs
2. Update E2E tests to use default tenant/user IDs
3. Remove all `skip` conditions from conversation/folder tests
4. Verify against live backend
