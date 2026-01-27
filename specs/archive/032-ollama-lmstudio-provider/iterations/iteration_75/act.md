# OODA 75 - Act: Model Configuration Field Tests

## Actions Taken

### Added model configuration field validation tests

```typescript
test("workspace has model configuration fields", async ({ request }) => {
  // Verify workspace response includes:
  // - llm_provider, llm_model
  // - embedding_provider, embedding_model, embedding_dimension
});

test("tenant has default model configuration", async ({ request }) => {
  // Verify tenant response includes:
  // - default_llm_provider, default_llm_model
  // - default_embedding_provider, default_embedding_model
});
```

### Fixed provider selector test stability

- Changed from `/query` to `/w/{slug}/query` deeplink
- Ensures workspace is selected before looking for provider selector
- Root cause: Without workspace selected, query page shows "Create Workspace" instead of query interface

## Test Results

```
Running 31 tests using 8 workers
  ✓ ... (all previous tests)
  ✓ workspace has model configuration fields  <-- NEW
  ✓ tenant has default model configuration  <-- NEW
  31 passed (5.0s)
```

## Coverage Update

| Category             | Tests  |
| -------------------- | ------ |
| Focus 1&2: Config    | 3      |
| Focus 3: Query UI    | 2      |
| Focus 4: Settings    | 3      |
| Focus 5: Rebuild     | 2      |
| Focus 6: Deeplinks   | 4      |
| Focus 7: Multi-model | 10     |
| Focus 8: Streaming   | 2      |
| Error Handling       | 2      |
| Pagination/Structure | 4      |
| **Total**            | **31** |
