# OODA 70 - Orient: Workspace Settings Model Configuration Tests

## Analysis

### What needs testing for Focus 4

1. **Workspace settings page displays model info**

   - Shows current LLM provider/model
   - Shows current embedding provider/model
   - Shows embedding dimension

2. **Workspace deeplink settings**
   - `/w/[slug]/settings` loads correctly
   - Shows workspace-specific configuration

### Test Strategy

Since we already have deeplink tests for `/w/[slug]/query`, adding settings route tests follows the same pattern:

```
test("workspace settings deeplink loads correctly")
test("settings page displays model configuration")
```

### Integration Points

The workspace API returns model configuration:

```json
{
  "llm_provider": "openai",
  "llm_model": "gpt-4o-mini",
  "embedding_provider": "openai",
  "embedding_model": "text-embedding-3-small",
  "embedding_dimension": 1536
}
```

The settings page should display these values.

## Recommendation

Add 2 tests to Focus 4 section:

1. "workspace settings deeplink loads" - tests `/w/[slug]/settings`
2. "settings displays workspace model configuration" - validates model info visible
