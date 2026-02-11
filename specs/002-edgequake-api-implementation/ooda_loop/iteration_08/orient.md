# IMPL-08 Orient — Analysis & Prioritization

## Root Cause Analysis

The type mismatches between SDK and Rust API fell into three categories:

### 1. Pagination Model Mismatch

- **SDK**: Used `Paginator<T>` with offset/limit (page-based)
- **Rust**: Uses cursor-based pagination with `{ items, pagination: { cursor, has_more, total } }`
- **Impact**: Conversations `list()` returned wrong type; messages `list()` returned raw array instead of paginated response
- **Fix**: Replace `Paginator` with direct cursor-based response types

### 2. Response Wrapper Pattern

- **SDK**: Flat response types (e.g., `ConversationDetail` extends `ConversationInfo`)
- **Rust**: Uses wrapper DTOs (e.g., `ConversationWithMessagesResponse { conversation, messages }`)
- **Impact**: Client code accessing `detail.title` would fail; should be `detail.conversation.title`
- **Fix**: Restructure all wrapper types to match Rust exactly

### 3. Missing Optional Fields

- **SDK**: Types missing many optional fields that Rust includes
- **Rust**: Rich metadata fields (tokens_used, duration_ms, thinking_time_ms, scopes, etc.)
- **Impact**: TypeScript wouldn't auto-complete these fields; users would need to cast
- **Fix**: Add all optional fields to match Rust structs

## Prioritization

1. ✅ Conversation types — most complex, cursor-based pagination
2. ✅ Auth types — multiple endpoints affected
3. ✅ E2E tests for conversations/folders/sharing/bulk
4. ✅ E2E tests for auth/users/API keys/costs
