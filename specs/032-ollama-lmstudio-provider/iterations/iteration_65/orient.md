# OODA 65 - Orient: Model Capability Validation

## Analysis

### Value of Capability Tests

1. **Data Quality**: Ensures model registry returns complete data
2. **UI Display**: Capabilities are shown in model selection UI
3. **Feature Gating**: Some features depend on capability flags

### Test Priority

| Capability | Priority | Reason |
|------------|----------|--------|
| context_length | High | Critical for token budgeting |
| max_output_tokens | Medium | Response length limits |
| supports_function_calling | Medium | Required for tool use |
| supports_vision | Low | Only subset of models |
| supports_json_mode | Low | Nice-to-have |
| supports_system_message | Low | Most models support this |

## Recommendation

Add one comprehensive test that validates:
1. All LLM models have `context_length > 0`
2. All models have complete capabilities object

Keep it focused to avoid test maintenance burden.
