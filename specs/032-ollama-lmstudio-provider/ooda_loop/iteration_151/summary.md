# OODA Iteration 151 - Model Tags and Categories

## Observe

### Focus
Verify that model tags are used for categorization and filtering.

### Investigation

**Model Tags** (from `models.toml`):

```toml
tags = ["recommended", "multimodal", "reasoning"]
tags = ["cost-effective", "fast"]
tags = ["free", "local"]
tags = ["vision", "reasoning"]
tags = ["legacy"]
```

### Tag Categories

| Tag | Meaning |
|-----|---------|
| recommended | Best choice for general use |
| cost-effective | Good balance of cost/performance |
| fast | Low latency |
| free | No API cost |
| local | Runs on local machine |
| multimodal | Supports images |
| vision | Can process images |
| reasoning | Good for complex reasoning |
| legacy | Deprecated or old version |

## Orient

### Tag Usage

1. **Filtering**: Filter models by tag
2. **Recommendations**: Highlight recommended models
3. **Deprecation**: Mark legacy models
4. **Categorization**: Group similar models

## Decide

**Status**: ✅ COMPLETE

Model tags are properly defined and can be used for filtering.

## Act

### Verified

- All models have tags defined
- Tags describe capabilities and recommendations
- Deprecated models marked as `legacy`
- Free/local models easily identifiable

---
*Commit: docs(OODA 151): Verify model tags and categories*
