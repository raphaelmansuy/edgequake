# OODA 65 - Decide: Model Capability Validation

## Decision

Add one focused test for LLM model capabilities:

- Verify `context_length > 0` for LLM models
- Verify `max_output_tokens >= 0` for LLM models
- Verify presence of key capability flags

## Why These Specific Checks

1. **context_length**: Critical for token budgeting in queries
2. **max_output_tokens**: Important for response length handling
3. **supports_streaming**: Already tested in OODA 63, included for completeness
4. **supports_function_calling**: Important for tool use features

## Implementation Note

Test only checks first 5 models to avoid slow tests while still validating data quality.
