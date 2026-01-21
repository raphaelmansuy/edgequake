# OODA Loop 55 - Decide

**Date:** 2026-01-14  
**Focus:** Multi-model support per provider (Focus 7) + Streaming fallback (Focus 8)

---

## ✅ TODO List

```markdown
- [ ] 1. Add missing OpenAI models (gpt-5o-nano, gpt-5o-mini) to models.toml
- [ ] 2. Add streaming fallback method to LMStudioProvider trait
- [ ] 3. Update LMStudioProvider::stream() to handle errors gracefully
- [ ] 4. Add stream_or_complete() method for fallback pattern
- [ ] 5. Update chat handler to use streaming fallback for LM Studio
- [ ] 6. Add tests for streaming fallback
- [ ] 7. Verify all models are accessible via API
- [ ] 8. Run E2E test to verify model selection works
- [ ] 9. Update OODA summary with progress
- [ ] 10. Commit changes with descriptive message
```

---

## Implementation Plan

### Step 1: Add OpenAI Models to models.toml

Add gpt-5o-nano and gpt-5o-mini as future/placeholder models.

### Step 2-4: LM Studio Streaming Fallback

Create `stream_with_fallback()` method that:

1. Attempts streaming
2. On streaming error, falls back to `complete()`
3. Returns unified response type

### Step 5: Update Chat Handler

Modify streaming handler to use fallback when provider is LM Studio.

### Step 6-8: Testing

Run unit tests and E2E tests to verify functionality.

---

## Risk Assessment

| Risk                     | Mitigation                                     |
| ------------------------ | ---------------------------------------------- |
| Streaming timeout        | Use reasonable timeout (30s) before fallback   |
| Fallback doubles latency | Log warning, don't retry streaming on fallback |
| Model not found errors   | Validate model exists in config before use     |
