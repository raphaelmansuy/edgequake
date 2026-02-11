# IMPL-08 Observe — Type Accuracy & E2E Expansion

## Observations

### SDK State Before This Iteration

- 243 unit tests, 46 E2E tests passing (IMPL-07)
- `conversations.ts` types used offset-based pagination, flat `ConversationDetail` extending `ConversationInfo`
- `auth.ts` types had flat response structures not matching Rust API
- E2E coverage gaps: conversations, folders, sharing, bulk ops, auth, users, API keys, costs

### Rust API Analysis

- **conversations_types.rs** (525 lines): Cursor-based pagination with `PaginatedConversationsResponse`, `ConversationWithMessagesResponse` wrapper pattern, bracket-style filter params (`filter[mode]`, `filter[archived]`)
- **auth_types.rs** (378 lines): Wrapped response DTOs (`GetMeResponse { user }`, `ListUsersResponse { users, total, page, page_size, total_pages }`), scoped API keys, pagination

### Gaps Identified

1. SDK `ConversationDetail` extended `ConversationInfo` — Rust uses `{ conversation, messages }` wrapper
2. SDK `list()` returned `Paginator<ConversationInfo>` — Rust uses cursor-based `PaginatedConversationsResponse`
3. SDK `MessageInfo` missing: `parent_id`, `mode`, `tokens_used`, `duration_ms`, `thinking_time_ms`, `context`, `is_error`
4. SDK `CreateUserResponse` was flat — Rust wraps in `{ user, created_at }`
5. SDK `CreateApiKeyRequest.name` required — Rust makes it optional
6. No E2E tests for conversations, folders, sharing, bulk ops, auth, users, API keys, costs

### Test Results Against Live Backend

- All 62 E2E tests pass (up from 46)
- All 243 unit tests pass
- Build produces clean ESM + CJS + DTS bundles
