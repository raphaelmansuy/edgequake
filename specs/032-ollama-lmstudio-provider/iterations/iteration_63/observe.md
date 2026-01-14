# OODA 63 - Observe: Streaming API E2E Test Coverage

## Current State

### Streaming Implementation Status

- ✅ OpenAI provider: Streaming supported
- ✅ Ollama provider: Streaming supported
- ✅ LM Studio provider: Falls back to non-streaming
- ✅ Mock provider: Simulates streaming

### E2E Test Coverage Gaps

- No E2E tests for streaming query responses
- No tests verifying streaming vs non-streaming behavior
- No tests for streaming error handling

### Backend API Surface

#### Query Endpoint

```
POST /api/v1/tenants/{tenant_id}/workspaces/{workspace_id}/query
```

Response can be:

1. **Streaming**: Server-Sent Events (SSE) with `Content-Type: text/event-stream`
2. **Non-streaming**: JSON response with `Content-Type: application/json`

### Frontend Implementation

- `QueryInterface` component handles streaming responses
- Uses `EventSource` or `fetch` with streaming response reader
- Falls back gracefully if streaming fails

## Questions to Answer

1. Should we add E2E tests for streaming behavior?
2. What streaming scenarios should we test?
3. How do we simulate streaming in E2E tests?
