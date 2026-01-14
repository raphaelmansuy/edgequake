# OODA 63 - Orient: Streaming E2E Test Strategy

## Analysis

### What We Have
- Models API returns `capabilities.supports_streaming` for each model
- LLM models have `supports_streaming: true`
- Embedding models have `supports_streaming: false`
- Backend supports SSE streaming via `/chat/completions/stream`
- Frontend has `chatCompletionStream()` async generator

### What We Need to Test

1. **API Level**: Verify `supports_streaming` is returned correctly
2. **UI Level**: Test that streaming responses work in query interface (hard to test)
3. **Fallback**: Test that non-streaming providers work correctly

### Testing Challenges

1. **SSE Testing**: Playwright doesn't easily support SSE testing
2. **Timing**: Streaming is async and timing-dependent
3. **Mock Provider**: Backend uses mock provider which simulates streaming

### Options

#### Option 1: API-Only Tests for Streaming Capability
- Test that models API returns `supports_streaming`
- Test that streaming endpoint exists and accepts requests
- **Pro**: Easy to implement, reliable
- **Con**: Doesn't test actual streaming behavior

#### Option 2: UI Streaming Tests with Mocks
- Test query interface with mock responses
- Verify streaming indicators appear
- **Pro**: Tests user-facing behavior
- **Con**: Complex, may be flaky

#### Option 3: Hybrid Approach
- Add API test for streaming capability flag
- Test UI shows streaming in progress indicator
- **Pro**: Covers both levels
- **Con**: More work

## Recommendation

**Option 1: API-Only Tests for Streaming Capability**

Add tests to verify:
1. `supports_streaming: true` for LLM models
2. `supports_streaming: false` for embedding models
3. This validates Focus 8 (streaming support indicator) at API level

The UI streaming behavior is already covered by manual testing and existing query tests.
