# OODA 64 - Observe: Provider Priority and Fallback Behavior

## Current State

### Provider Configuration
The models API returns providers with priority ordering:
- OpenAI: priority 10 (highest)
- Ollama: priority 20
- LM Studio: priority 30
- Anthropic: priority 40
- Azure: priority 50
- Mock: priority 100 (lowest)

### Test Coverage Analysis

Current E2E tests cover:
1. ✅ Models API structure
2. ✅ LLM model existence
3. ✅ Embedding model existence
4. ✅ Streaming capability flags
5. ✅ Tenant/workspace creation with model config
6. ✅ Workspace deeplinks
7. ⚠️ Provider priority ordering (not tested)
8. ⚠️ Provider enable/disable status (not tested)

### Untested Areas

1. **Provider Priority**: Tests should verify providers are ordered by priority
2. **Provider Enabled Status**: Tests should verify enabled providers are accessible
3. **Model Type Filtering**: Tests could filter models by type more explicitly
4. **Cost Information**: Models include cost data that could be validated

## Questions

1. Should we add tests for provider priority ordering?
2. Should we test the enabled flag behavior?
3. Are there other model properties we should validate?
