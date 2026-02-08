# OODA Iteration 06 - Decide

## Decision: Update Default LLM Model to gpt-5-nano

### 1. Selected Action

Update hardcoded `gpt-4o-mini` defaults to `gpt-5-nano` in model_config.rs.

### 2. Changes to Make

| Line | Before | After |
|------|--------|-------|
| 407 | `"gpt-4o-mini".to_string()` | `"gpt-5-nano".to_string()` |
| 515 | `Some("gpt-4o-mini".to_string())` | `Some("gpt-5-nano".to_string())` |

### 3. WHY Comments to Add

Add context for future maintainers:

```rust
fn default_llm_model() -> String {
    // WHY: gpt-5-nano is the recommended default (2025-02).
    // gpt-4o-mini has quota issues and is being phased out.
    // See: OODA-06 in specs/001-reliable-ingestion-mission/
    "gpt-5-nano".to_string()
}
```

### 4. Test Plan

```bash
# 1. Verify LLM crate builds
cargo build -p edgequake-llm

# 2. Run LLM tests
cargo test -p edgequake-llm --lib

# 3. Run full test suite
cargo test --workspace --lib

# 4. Verify no clippy warnings
cargo clippy -p edgequake-llm
```

### 5. Success Criteria

- [ ] `default_llm_model()` returns "gpt-5-nano"
- [ ] OpenAI provider default is "gpt-5-nano"
- [ ] All tests pass
- [ ] No clippy warnings

### 6. Rollback Plan

If tests fail:
1. Revert changes
2. Investigate test failures
3. Add gpt-5-nano as alongside gpt-4o-mini if needed

### 7. Commit Message

```
OODA-06: Update default LLM model to gpt-5-nano

- Change default_llm_model() return value from gpt-4o-mini to gpt-5-nano
- Update OpenAI provider default_llm_model
- Add WHY comment explaining the change

gpt-4o-mini has quota issues and is deprecated. gpt-5-nano is the
recommended replacement for cost-effective entity extraction.

Addresses mission criterion: "Ensure no hardcoded models (gpt-4o-mini)"
```

## Decision Confirmed

Proceed with updating the two hardcoded defaults in model_config.rs.
