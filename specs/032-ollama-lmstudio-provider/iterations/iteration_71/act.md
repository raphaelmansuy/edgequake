# OODA 71 - Act: Focus 5 Rebuild Embeddings API Tests

## Actions Taken

### Added Focus 5: Rebuild Embeddings test section

Two new E2E tests added to `spec032-provider-integration.spec.ts`:

```typescript
test.describe("Focus 5: Rebuild Embeddings", () => {
  test("rebuild embeddings API validates request correctly", async ({ request }) => {
    // Get existing workspace
    // POST to rebuild-embeddings with force:false
    // Expect 400 error about config unchanged
  });

  test("rebuild embeddings API accepts force flag", async ({ request }) => {
    // Get existing workspace
    // POST with force:true
    // Expect 200 success with status property
  });
});
```

## Test Results

```
Running 24 tests using 8 workers
  ✓ models API returns available providers and models (840ms)
  ✓ default model configuration is valid (870ms)
  ✓ providers have priority property (910ms)
  ✓ core providers are enabled (860ms)
  ✓ LLM models exist in providers (890ms)
  ✓ embedding models exist in providers (820ms)
  ✓ LLM models have complete capabilities (850ms)
  ✓ models have cost information (910ms)
  ✓ models have tags property (600ms)
  ✓ query page has provider model selector (1.4s)
  ✓ provider selector shows available providers (1.9s)
  ✓ LLM models report streaming capability (810ms)
  ✓ embedding models do not support streaming (850ms)
  ✓ can create tenant with default model config via API (740ms)
  ✓ can create workspace with model config via API (900ms)
  ✓ workspace uses server defaults when tenant models not specified (860ms)
  ✓ workspace deeplink by slug resolves correctly (1.1s)
  ✓ invalid workspace slug shows error state (2.5s)
  ✓ /w/[slug] redirects to /w/[slug]/query (840ms)
  ✓ /w/[slug]/settings deeplink loads workspace settings (1.0s)
  ✓ settings page shows provider status (880ms)
  ✓ settings page shows rebuild embeddings option (840ms)
  ✓ rebuild embeddings API validates request correctly (750ms) <-- NEW
  ✓ rebuild embeddings API accepts force flag (710ms)  <-- NEW
  24 passed (4.8s)
```

## Coverage Summary

| Focus Area | Tests | Status |
|------------|-------|--------|
| Focus 3: Query Provider UI | 2 | ✅ |
| Focus 4: Workspace Settings | 3 | ✅ |
| Focus 5: Rebuild Embeddings | 2 | ✅ NEW |
| Focus 6: Deeplinks | 4 | ✅ |
| Focus 7: Multi-model | 9 | ✅ |
| Focus 8: Streaming | 2 | ✅ |
| Focus 1&2: Config | 3 | ✅ |
| **Total** | **24** | **All passing** |
