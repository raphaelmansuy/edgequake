# OODA 64 - Orient: Provider API Hardening

## Analysis

### Current Test Gaps

1. **Provider Priority Not Tested**

   - Providers should be returned in priority order
   - Important for model selection defaults

2. **Provider Enabled Status Not Tested**

   - All returned providers should be enabled
   - Disabled providers should not appear in API response

3. **Model Capabilities Not Fully Validated**
   - Only `supports_streaming` and `embedding_dimension` tested
   - Other capabilities like `context_length`, `supports_function_calling` not validated

### Options

#### Option 1: Add Provider Ordering Test

- Verify providers are returned in priority order (lowest number = highest priority)
- Simple, validates important behavior

#### Option 2: Add Comprehensive Model Capability Tests

- Test all capability fields are present
- Test values are reasonable (e.g., context_length > 0)
- More thorough but more maintenance

#### Option 3: Add Provider Type Tests

- Test that each provider has correct `provider_type`
- Test that provider names match expected values

## Recommendation

**Option 1 + Option 3**: Add tests for:

1. Provider priority ordering
2. Provider enabled status (all should be true)
3. Provider type matches provider name

These are quick wins that validate important provider behavior without excessive maintenance burden.
