# IMPL-08 Decide — Implementation Plan

## Decision: Complete Type Alignment + Comprehensive E2E

### Approach

1. Read Rust source types (`conversations_types.rs`, `auth_types.rs`) line by line
2. Rewrite SDK type files to match Rust exactly
3. Update resource classes for new return types
4. Update unit test mocks
5. Create E2E tests for all untested resource groups

### Files Changed

#### Type Rewrites

- `src/types/conversations.ts` — Full rewrite: cursor-based pagination, wrapper DTOs, rich metadata
- `src/types/auth.ts` — Full rewrite: wrapped responses, scoped API keys, pagination

#### Resource Updates

- `src/resources/conversations.ts` — `list()` returns `PaginatedConversationsResponse`, bracket filter params
- `src/resources/auth.ts` — `me()` returns `GetMeResponse`
- `src/resources/users.ts` — `list()` accepts `ListUsersQuery`, returns `ListUsersResponse`
- `src/resources/api-keys.ts` — `list()` returns `ListApiKeysResponse`, `revoke()` returns `RevokeApiKeyResponse`

#### Test Updates

- `tests/unit/resources.test.ts` — Updated mocks for conversations, auth, users, API keys
- `tests/e2e/conversations-folders.test.ts` — NEW: 14 tests
- `tests/e2e/auth-costs.test.ts` — NEW: 7 tests

### Risk Mitigation

- All auth endpoints handle 401/403 gracefully (backend may not have auth configured)
- Costs endpoints handle 404/500 gracefully (endpoint may not exist)
- E2E tests create and cleanup their own resources
