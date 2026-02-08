# OODA Iteration 06 - Observe

## Mission Re-Read Checkpoint

✅ Mission file re-read: `./specs/001-reliable-ingestion-mission.md`

## Observation: Hardcoded Model Audit

### 1. Success Criteria Status

| Criterion                  | Status | Notes                                  |
| -------------------------- | ------ | -------------------------------------- |
| Ensure no hardcoded models | ⚠️     | Found `gpt-4o-mini` hardcoded defaults |

### 2. Hardcoded Model Locations

**Critical - Should be `gpt-5-nano`:**

| File              | Line | Code                                        | Category               |
| ----------------- | ---- | ------------------------------------------- | ---------------------- |
| `model_config.rs` | 407  | `"gpt-4o-mini".to_string()`                 | Default function       |
| `model_config.rs` | 515  | `default_llm_model: Some("gpt-4o-mini"...)` | OpenAI provider config |

**Test Code (lower priority):**

| File                     | Lines       | Code                                  | Category        |
| ------------------------ | ----------- | ------------------------------------- | --------------- |
| `cost_tracking_tests.rs` | 25,73,80,87 | `ModelPricing::new("gpt-4o-mini"...)` | Test fixtures   |
| `cost_tracking_tests.rs` | 111,148-152 | Assertions about gpt-4o-mini pricing  | Test assertions |
| `cache.rs`               | 297,389     | gpt-4o-mini pricing comments/tests    | Cache tests     |

### 3. Code Analysis

**model_config.rs line 407:**

```rust
fn default_llm_model() -> String {
    "gpt-4o-mini".to_string()
}
```

This is the global default for LLM models when no config is provided.

**model_config.rs line 515:**

```rust
default_llm_model: Some("gpt-4o-mini".to_string()),
```

This is the OpenAI provider's default model.

### 4. Impact Assessment

| Change                                     | Risk   | Impact                                  |
| ------------------------------------------ | ------ | --------------------------------------- |
| Change `default_llm_model()` to gpt-5-nano | Low    | All new configs use gpt-5-nano          |
| Change OpenAI provider default             | Low    | Explicit OpenAI users get gpt-5-nano    |
| Change test fixtures                       | Medium | May need to add gpt-5-nano pricing data |

### 5. Test Files Analysis

The test files use `gpt-4o-mini` for:

- Testing cost tracking math
- Testing pricing assertions
- Testing cache behavior

These tests verify cost tracking works correctly but don't need to use the "default" model - they're explicitly testing with specific models for pricing validation.

**Decision Point:** Test files can remain with gpt-4o-mini references since they're testing cost tracking logic, not model defaults.

### 6. Progress.rs Status

From OODA-02, `progress.rs` already has:

```rust
#[deprecated(since = "0.1.0", note = "Use new_gpt5_nano() instead")]
pub fn new_gpt4o_mini() -> Self { ... }
```

This deprecation is correct and already in place.

## Key Finding

Two hardcoded defaults in `model_config.rs` should be changed:

1. Line 407: `default_llm_model()` → return `"gpt-5-nano"`
2. Line 515: OpenAI provider `default_llm_model` → `"gpt-5-nano"`

## Next Steps

1. Update the two defaults to gpt-5-nano
2. Add WHY comment explaining the change
3. Run tests to verify no breakage
4. Commit with OODA-06 prefix
