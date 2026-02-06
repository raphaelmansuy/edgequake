# OODA-18: Observe — Query Engine E2E Tests

## Observation Date
2025-07-13

## What Was Examined

### Query API Surface
- `POST /api/v1/query` — Execute RAG query with mode selection
- `POST /api/v1/query/stream` — Streaming query (SSE)
- `POST /api/v1/conversations` — Create conversation
- `GET /api/v1/conversations` — List conversations (paginated)

### Query Request Structure (query_types.rs)
- `query` (String) — required
- `mode` (Option<String>) — naive, local, global, hybrid, mix
- `context_only` (bool) — skip LLM generation, return context only
- `prompt_only` (bool) — return formatted prompt for debugging
- `conversation_history` (Option<Vec<ConversationMessage>>) — multi-turn context
- `enable_rerank` (bool, default true) — rerank retrieved chunks
- `include_references` (bool) — add document_id, file_path to sources

### Query Response Structure (QueryResponse)
- `answer` (String) — generated answer (or empty if context_only)
- `mode` (String) — mode used
- `sources` (Vec<SourceReference>) — retrieved context sources
- `stats` (QueryStats) — timing and token metrics
- `conversation_id` (Option<String>)
- `reranked` (bool)

### Conversation Endpoints
- Create requires `X-Tenant-ID` + `X-User-ID` headers (valid UUIDs)
- List returns `PaginatedConversationsResponse` with `items` + `pagination`
- Without tenant headers: 400 "Missing X-Tenant-ID header"

### Validation Rules
- Empty/whitespace query → 422 "Query cannot be empty"
- Query > max_query_length (10000) → 400
- validate_query() in validation.rs:137

## Key Findings
1. Query endpoint works without tenant headers (TenantContext fields are optional)
2. Conversation endpoints strictly require X-Tenant-ID + X-User-ID
3. Mock provider returns "Mock response" as answer, vec![0.1; 1536] embeddings
4. context_only=true returns empty answer string + valid response structure
5. prompt_only=true returns formatted prompt as the answer
