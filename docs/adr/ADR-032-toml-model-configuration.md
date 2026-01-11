# ADR-032: TOML-based Model Configuration

**Status:** Accepted  
**Date:** 2025-01-27  
**Deciders:** EdgeQuake Team  
**Context:** SPEC-032 Ollama/LM Studio Provider Support

## Context

EdgeQuake supports multiple LLM and embedding providers (OpenAI, Ollama, LM Studio, Anthropic, Azure). Users need visibility into:
- Available models and their capabilities
- Cost information per 1K tokens
- Context length and output limits
- Provider health status

Previously, provider configuration was hardcoded or required environment variables. This made it difficult to:
1. Add new providers without code changes
2. Update model capabilities as providers evolve
3. Display rich model information in the UI

## Decision

Implement a TOML-based configuration system for model cards that:

1. **Configuration File Location** (in priority order):
   - `$EDGEQUAKE_MODELS_CONFIG` environment variable
   - `./models.toml` (working directory)
   - `~/.edgequake/models.toml` (user home)
   - Built-in defaults

2. **Schema Structure**:
   ```toml
   [defaults]
   llm_provider = "openai"
   llm_model = "gpt-4o"
   embedding_provider = "openai"
   embedding_model = "text-embedding-3-small"
   
   [[providers]]
   name = "openai"
   type = "openai"
   enabled = true
   priority = 1
   api_key_env = "OPENAI_API_KEY"
   
   [[providers.models]]
   name = "gpt-4o"
   model_type = "llm"
   display_name = "GPT-4o"
   
   [providers.models.capabilities]
   context_length = 128000
   supports_vision = true
   supports_function_calling = true
   
   [providers.models.cost]
   input_per_1k = 0.0025
   output_per_1k = 0.01
   ```

3. **API Endpoints**:
   - `GET /api/v1/models` - List all providers and models
   - `GET /api/v1/models/llm` - LLM models only
   - `GET /api/v1/models/embedding` - Embedding models only
   - `GET /api/v1/models/health` - Provider health checks
   - `GET /api/v1/models/{provider}` - Specific provider
   - `GET /api/v1/models/{provider}/{model}` - Specific model

4. **WebUI Integration**:
   - React hooks for fetching model configuration
   - ModelSelector component with capability badges
   - Cost and context length display

## Consequences

### Positive
- **Flexibility**: Administrators can customize models without code changes
- **Transparency**: Users see cost/capability information before selecting models
- **Extensibility**: New providers added by editing TOML file
- **Fallback**: Built-in defaults work out of the box

### Negative
- **Complexity**: Additional configuration layer to maintain
- **Sync Required**: Config must match actual provider capabilities
- **Health Check Overhead**: Periodic health checks add network requests

### Risks
- Config file could become stale vs actual provider capabilities
- Mitigation: Document update process, consider auto-detection

## Implementation

### Backend (Rust)
- `edgequake-llm/src/model_config.rs`: Config schema and loader
- `edgequake-api/src/handlers/models.rs`: API endpoints
- `edgequake/models.toml`: Example configuration

### Frontend (TypeScript)
- `lib/api/models.ts`: API client
- `hooks/use-models.ts`: React Query hooks
- `components/models/`: ModelCard, ModelSelector components

### Tests
- 11 unit tests for config parsing
- 7 integration tests for API endpoints
- TypeScript type checking

## Related Documents
- [SPEC-032: Ollama/LM Studio Provider Support](../specs/032-ollama-lmstudio-provider/032-ollama-lmstudio-provider.md)
- [models.toml Example](../edgequake/models.toml)
