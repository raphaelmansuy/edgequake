# OODA 63 - Decide: Add Streaming Capability Tests

## Decision

Add API-level E2E tests for Focus 8 (Streaming Support) that verify:
1. LLM models from streaming-capable providers report `supports_streaming: true`
2. Embedding models report `supports_streaming: false`

## Rationale

### Why API-Level Tests
- SSE streaming is difficult to test in E2E with Playwright
- The streaming capability flag is the key indicator for UI decisions
- Testing the flag validates the backend implementation

### Why Not UI Streaming Tests
- Playwright doesn't easily handle SSE connections
- Streaming behavior is timing-dependent and prone to flakiness
- Manual testing and existing query tests cover UI behavior

## Test Design

### Test 1: LLM Models Report Streaming Capability
```typescript
// Filter to providers that support streaming
const streamingProviders = ["openai", "ollama", "anthropic"];
const llmModels = data.providers
  .filter((p: any) => streamingProviders.includes(p.name))
  .flatMap((p: any) => p.models.filter((m: any) => m.model_type === "llm"));

// All LLM models should support streaming
for (const model of llmModels) {
  expect(model.capabilities.supports_streaming).toBe(true);
}
```

### Test 2: Embedding Models Do Not Support Streaming
```typescript
const embeddingModels = data.providers.flatMap((p: any) =>
  p.models.filter((m: any) => m.model_type === "embedding")
);

// Embedding models should not support streaming
for (const model of embeddingModels) {
  expect(model.capabilities.supports_streaming).toBe(false);
}
```

## Expected Outcomes
- ✅ 11/11 E2E tests passing
- ✅ Focus 8 coverage at API level
- ✅ Validates streaming capability indicator
