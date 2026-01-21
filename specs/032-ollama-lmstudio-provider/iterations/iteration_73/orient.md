# OODA 73 - Orient: Provider Health Check Test Strategy

## Analysis

### Health Endpoint Response

The `/api/v1/models/health` endpoint returns an array of provider status objects with:

- `name`: Provider identifier
- `display_name`: Human-readable name
- `provider_type`: Provider type (openai, ollama, etc.)
- `enabled`: Whether provider is enabled
- `priority`: Provider priority for ordering
- `description`: Provider description
- `models`: Array of model details

### Test Scenarios

1. **Health endpoint responds**

   - GET `/api/v1/models/health`
   - Verify 200 status
   - Verify array response

2. **Provider status includes required fields**
   - Each provider has name, enabled, priority
   - At least one provider enabled

## Recommendation

Add 1 test for provider health check:

- "provider health check returns enabled providers"
