# Iteration 10 — Orient

## Analysis

### Root Causes of Type Drift

1. **Early iterations used surface-level mapping**: Types were initially created from API endpoint observation and documentation rather than reading Rust source code directly. This led to oversimplified types.

2. **Shared types were duplicated**: `SourceReference` and `QueryStats` were defined separately in `chat.ts` with different shapes from `query.ts`, violating DRY and causing inconsistency.

3. **Pagination pattern inconsistency**: Documents used the generic `Paginator` class with offset/limit, but the Rust API uses a page/page_size pattern with structured `ListDocumentsResponse`.

4. **Field naming divergence**: The chat API in Rust uses `provider`/`model` field names while the query API uses `llm_provider`/`llm_model`. The SDK didn't capture this distinction.

### Design Decisions

- **Centralize shared types**: `SourceReference` and `QueryStats` should live in `query.ts` (where the Rust implementations live) and be re-exported from `chat.ts`.
- **Match Rust exactly**: Field names must match the JSON wire format exactly, especially for request types where wrong field names cause silent drops.
- **Deprecation over removal**: Use `type QuerySource = SourceReference` for backward compatibility.
- **Optional fields for graph**: Use optional fields (`node_type?`, `description?`) for graph types since not all API responses include them.

### Risk Assessment

- **Breaking changes**: `ChatCompletionRequest.llm_provider` → `provider` is a breaking change, but v0.1.0 is pre-release so this is acceptable.
- **QueryResponse.sources** shape change: Elements now have `source_type`/`id` instead of `content`. Breaking for consumers accessing `.content`, but aligns with actual API response.
