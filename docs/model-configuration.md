# Model Configuration Guide

EdgeQuake uses a TOML configuration file to define available LLM and embedding providers. This guide explains how to configure providers, models, and their capabilities.

## Quick Start

1. Copy the example configuration:

   ```bash
   cp edgequake/models.toml ~/.edgequake/models.toml
   ```

2. Edit to match your setup:

   ```bash
   vim ~/.edgequake/models.toml
   ```

3. Set API keys as environment variables:
   ```bash
   export OPENAI_API_KEY="sk-..."
   export ANTHROPIC_API_KEY="sk-ant-..."
   ```

## Configuration File Locations

EdgeQuake searches for the configuration in this order:

1. `$EDGEQUAKE_MODELS_CONFIG` - Custom path via environment variable
2. `./models.toml` - Current working directory
3. `~/.edgequake/models.toml` - User home directory
4. Built-in defaults - Fallback configuration

## Configuration Structure

### Defaults Section

```toml
[defaults]
llm_provider = "openai"           # Default LLM provider
llm_model = "gpt-4o"              # Default chat/completion model
embedding_provider = "openai"      # Default embedding provider
embedding_model = "text-embedding-3-small"  # Default embedding model
```

### Provider Definition

```toml
[[providers]]
name = "openai"                   # Provider identifier
type = "openai"                   # Provider type (see below)
display_name = "OpenAI"           # Human-readable name
enabled = true                    # Enable/disable provider
priority = 1                      # Selection priority (lower = higher)
description = "OpenAI GPT models"
api_key_env = "OPENAI_API_KEY"    # Environment variable for API key
base_url = ""                     # Optional: Custom API endpoint
```

**Provider Types:**

- `openai` - OpenAI API
- `ollama` - Ollama local server
- `lmstudio` - LM Studio server
- `anthropic` - Anthropic Claude
- `azure` - Azure OpenAI Service
- `openaicompatible` - Any OpenAI-compatible API
- `mock` - Testing/development mock

### Model Definition

```toml
[[providers.models]]
name = "gpt-4o"                   # Model identifier (API name)
model_type = "llm"                # Type: llm, embedding, multimodal
display_name = "GPT-4o"           # Human-readable name
description = "Latest GPT-4o model with vision"
deprecated = false
replacement = ""                   # Model to use if deprecated

[providers.models.capabilities]
context_length = 128000           # Max context window (tokens)
max_output_tokens = 16384         # Max completion length
supports_vision = true            # Image input support
supports_function_calling = true  # Tool/function calling
supports_json_mode = true         # Guaranteed JSON output
supports_streaming = true         # SSE streaming responses
supports_system_message = true    # System prompt support
embedding_dimension = 0           # For embedding models only

[providers.models.cost]           # Cost per 1,000 tokens (USD)
input_per_1k = 0.0025
output_per_1k = 0.01
embedding_per_1k = 0.0            # For embedding models
image_per_unit = 0.0              # Per-image cost

[[providers.models.tags]]         # Optional tags for filtering
"flagship"
"vision"
"recommended"
```

## Example Configurations

### OpenAI Only

```toml
[defaults]
llm_provider = "openai"
llm_model = "gpt-4o-mini"
embedding_provider = "openai"
embedding_model = "text-embedding-3-small"

[[providers]]
name = "openai"
type = "openai"
display_name = "OpenAI"
enabled = true
priority = 1
api_key_env = "OPENAI_API_KEY"

[[providers.models]]
name = "gpt-4o-mini"
model_type = "llm"
display_name = "GPT-4o Mini"

[providers.models.capabilities]
context_length = 128000
supports_vision = true
supports_function_calling = true
supports_json_mode = true
supports_streaming = true
supports_system_message = true

[providers.models.cost]
input_per_1k = 0.00015
output_per_1k = 0.0006

[[providers.models]]
name = "text-embedding-3-small"
model_type = "embedding"
display_name = "Embedding 3 Small"

[providers.models.capabilities]
embedding_dimension = 1536

[providers.models.cost]
embedding_per_1k = 0.00002
```

### Ollama Local Setup

```toml
[defaults]
llm_provider = "ollama"
llm_model = "llama3.2"
embedding_provider = "ollama"
embedding_model = "nomic-embed-text"

[[providers]]
name = "ollama"
type = "ollama"
display_name = "Ollama"
enabled = true
priority = 1
base_url = "http://localhost:11434"

[[providers.models]]
name = "llama3.2"
model_type = "llm"
display_name = "Llama 3.2"

[providers.models.capabilities]
context_length = 128000
supports_streaming = true
supports_system_message = true

[[providers.models]]
name = "nomic-embed-text"
model_type = "embedding"
display_name = "Nomic Embed"

[providers.models.capabilities]
embedding_dimension = 768
```

### Multi-Provider Setup

```toml
[defaults]
llm_provider = "openai"
llm_model = "gpt-4o"
embedding_provider = "ollama"
embedding_model = "nomic-embed-text"

[[providers]]
name = "openai"
type = "openai"
enabled = true
priority = 1
api_key_env = "OPENAI_API_KEY"
# ... models ...

[[providers]]
name = "ollama"
type = "ollama"
enabled = true
priority = 2
base_url = "http://localhost:11434"
# ... models ...

[[providers]]
name = "anthropic"
type = "anthropic"
enabled = true
priority = 3
api_key_env = "ANTHROPIC_API_KEY"
# ... models ...
```

## API Endpoints

Once configured, the models API exposes these endpoints:

| Endpoint                                | Description                      |
| --------------------------------------- | -------------------------------- |
| `GET /api/v1/models`                    | List all providers and models    |
| `GET /api/v1/models/llm`                | LLM models with capabilities     |
| `GET /api/v1/models/embedding`          | Embedding models with dimensions |
| `GET /api/v1/models/health`             | Provider health status           |
| `GET /api/v1/models/{provider}`         | Specific provider details        |
| `GET /api/v1/models/{provider}/{model}` | Specific model card              |

### Example Response

```json
{
  "providers": [
    {
      "name": "openai",
      "display_name": "OpenAI",
      "enabled": true,
      "models": [
        {
          "name": "gpt-4o",
          "display_name": "GPT-4o",
          "model_type": "llm",
          "capabilities": {
            "context_length": 128000,
            "supports_vision": true,
            "supports_function_calling": true
          },
          "cost": {
            "input_per_1k": 0.0025,
            "output_per_1k": 0.01
          }
        }
      ]
    }
  ],
  "default_llm_provider": "openai",
  "default_llm_model": "gpt-4o"
}
```

## Troubleshooting

### Provider Not Available

If a provider shows as unavailable:

1. Check the API key environment variable is set
2. Verify the `base_url` is accessible
3. Run health check: `GET /api/v1/models/health`

### Model Not Found

If a model isn't listed:

1. Verify it's in the `[[providers.models]]` section
2. Check `enabled = true` on the provider
3. Restart the server after config changes

### Configuration Validation

The server validates configuration at startup. Check logs for:

- Duplicate provider names
- Invalid model types
- Missing required fields

## Best Practices

1. **Start Simple**: Use the example config and customize incrementally
2. **Use Environment Variables**: Never hardcode API keys
3. **Set Priorities**: Lower numbers = preferred providers
4. **Document Costs**: Keep cost information up-to-date
5. **Disable Unused**: Set `enabled = false` for inactive providers
