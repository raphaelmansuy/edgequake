# OODA Iteration 06 - Orient

## Analysis of Hardcoded Models

### 1. Root Cause

**Why are defaults still gpt-4o-mini?**

The model_config.rs file was written before gpt-4o-mini quota exceeded. It provides:
1. A fallback default when no config file is present
2. A default for the OpenAI provider definition

These weren't updated during the gpt-5-nano migration (OODA-02/04) because:
- Focus was on deprecating `new_gpt4o_mini()` function
- Test assertions were fixed (OODA-04)
- Default functions in config were overlooked

### 2. Strategic Assessment

**What happens with current code?**

When a user runs EdgeQuake:
1. If no `models.toml` exists → `default_llm_model()` returns "gpt-4o-mini"
2. If models.toml is partial → OpenAI provider uses "gpt-4o-mini"
3. Runtime → API calls fail with quota exceeded error

This is the mission-critical failure path.

### 3. Risk Analysis

| Approach | Risk | Benefit |
|----------|------|---------|
| Change defaults only | Low | Users get working defaults |
| Change defaults + tests | Medium | May break test assertions |
| Add both models as options | None | Backward compatible |

**Recommendation:** Change defaults only. Test files test cost tracking math, not model selection.

### 4. First Principles Analysis

**Question:** What should the default LLM model be?

**Constraints:**
- Must be available (quota not exceeded)
- Must work for entity extraction
- Must be cost-effective
- Must be well-supported by OpenAI

**Answer:** `gpt-5-nano` satisfies all constraints per mission spec.

### 5. Dependency Check

Changing `default_llm_model()` affects:
- `DefaultsConfig::default()` (line 420-427)
- OpenAI provider fallback (line 515)
- Any code that calls `default_llm_model()`

```
default_llm_model() 
    └─> DefaultsConfig::default()
        └─> ModelRegistry::from_config()
            └─> WorkspaceConfig construction
                └─> Runtime model selection
```

### 6. Side Effect Assessment

| Code Path | Expected Behavior | After Change |
|-----------|-------------------|--------------|
| No config file | Uses gpt-4o-mini | Uses gpt-5-nano ✓ |
| Partial config | Falls back to gpt-4o-mini | Falls back to gpt-5-nano ✓ |
| Full config | Uses specified model | No change |
| Tests | Uses mock provider | No change |

### 7. Mental Model

```
Before (broken):
┌────────────────────────────────────────────┐
│  User runs EdgeQuake                       │
│  → No models.toml                          │
│  → default_llm_model() returns gpt-4o-mini │
│  → API call → QUOTA EXCEEDED ERROR ❌      │
└────────────────────────────────────────────┘

After (working):
┌────────────────────────────────────────────┐
│  User runs EdgeQuake                       │
│  → No models.toml                          │
│  → default_llm_model() returns gpt-5-nano  │
│  → API call → SUCCESS ✓                    │
└────────────────────────────────────────────┘
```

### 8. Validation Strategy

1. Change defaults to gpt-5-nano
2. Run `cargo test -p edgequake-llm --lib`
3. Verify no test failures
4. Run full test suite
5. Verify compilation

## Orientation Complete

The fix is straightforward: update two hardcoded "gpt-4o-mini" strings to "gpt-5-nano" in model_config.rs.
