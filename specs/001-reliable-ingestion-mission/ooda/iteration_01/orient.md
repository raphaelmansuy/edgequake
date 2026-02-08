# Iteration 01 - Orient

**Date**: 2026-02-08  
**Mission**: Reliable Document Ingestion Pipeline

## Analysis

### 1. gpt-4o-mini → gpt-5-nano Migration

**Problem:** OpenAI has deprecated `gpt-4o-mini` or the quota is exceeded.

**Evidence from user:**

```bash
curl https://api.openai.com/v1/responses ... -d '{"model": "gpt-4o-mini", ...}'
# Response: "insufficient_quota" error

curl https://api.openai.com/v1/responses ... -d '{"model": "gpt-5-nano", ...}'
# Response: Success - "gpt-5-nano-2025-08-07"
```

**Impact Analysis:**
| Area | Files Affected | Risk |
|------|----------------|------|
| Documentation | 2 files | Low - docs only |
| Test Files | 1 file (cost_integration_tests.rs) | Medium - tests may fail |
| Production Code | 1 file (lineage.rs) | **High** - affects entity extraction |
| Config | 1 file (models.toml) | Medium - example config |

**First Principle Analysis:**

- `gpt-5-nano` is OpenAI's new cost-effective model
- Pricing may differ (need to update cost calculations)
- API compatibility should be maintained (same `/v1/chat/completions` endpoint)

### 2. In-Memory Storage Assessment

**Should NOT Remove:**

1. `InMemoryKeywordCache` - Valid LRU cache for performance
2. `test_state()` function - Required for unit tests
3. `new_memory()` function - Legitimate for dev mode

**Should Consider:**

- Add explicit warning if Memory mode is used without `--dev` flag
- Ensure CI/CD and production deployments always have `DATABASE_URL`

**First Principle Analysis:**

- In-memory storage is not inherently bad
- It provides fast iteration during development
- Problem is **accidental use in production**, not existence

### 3. Current Provider Configuration

**Model Selection Hierarchy:**

```
1. EDGEQUAKE_DEFAULT_LLM_MODEL env var
2. OLLAMA_HOST + model detection
3. OPENAI_API_KEY + default model
4. Mock provider (fallback)
```

**Current Defaults in ProviderFactory:**

- Need to verify if `gpt-4o-mini` is hardcoded anywhere in the defaults

### 4. Risk Assessment

| Change                          | Benefit            | Risk                     | Mitigation           |
| ------------------------------- | ------------------ | ------------------------ | -------------------- |
| Update gpt-4o-mini → gpt-5-nano | Fixes API quota    | Medium - API differences | Test with real API   |
| Keep in-memory for dev          | Fast dev iteration | Low                      | Warn at startup      |
| Update cost calculations        | Accurate billing   | Low                      | Update pricing table |

## Options

### Option A: Minimal Change (Recommended)

1. Update all `gpt-4o-mini` references to `gpt-5-nano`
2. Update documentation
3. Add startup warning for Memory mode
4. Verify API compatibility

**Pros:** Low risk, targeted fix
**Cons:** Doesn't address potential dead code

### Option B: Comprehensive Cleanup

1. All changes from Option A
2. Remove unused InMemory implementations
3. Remove dead code audit
4. Add feature flags for storage modes

**Pros:** Cleaner codebase
**Cons:** Higher risk, more testing needed

### Option C: Disable Memory Mode Entirely

1. Remove `new_memory()` function
2. Require `DATABASE_URL` always
3. Force PostgreSQL only

**Pros:** No accidental memory use
**Cons:** Breaks dev workflow, CI changes needed

## Recommendation

**Proceed with Option A** (Minimal Change):

- Low risk
- Targeted fix for immediate problem (gpt-4o-mini quota)
- Maintains dev workflow
- Can iterate to Option B later

## Dependencies to Check

Before making changes:

1. Verify `gpt-5-nano` API compatibility
2. Check if pricing is different
3. Confirm embedding model compatibility (text-embedding-3-small should still work)
