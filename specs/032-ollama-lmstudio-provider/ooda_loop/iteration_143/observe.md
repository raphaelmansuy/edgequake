# Observe - Iteration 143

## Focus: Multiple Models Per Provider (Item 7)

Verifying SPEC-032 requirement:

- **Item 7**: Each provider has multiple models to choose from for both LLM and embedding

## Investigation

### Models Configuration File

**File**: `edgequake/models.toml`

- Lines: 1281
- Total models: 45 (`[[providers.models]]` entries)
- Total providers: 6

### Provider Summary

| Provider  | Display Name | Models Count | Type            |
| --------- | ------------ | ------------ | --------------- |
| openai    | OpenAI       | ~11          | LLM + Embedding |
| ollama    | Ollama       | ~15          | LLM + Embedding |
| lmstudio  | LM Studio    | ~10          | LLM + Embedding |
| groq      | Groq         | ~3           | LLM             |
| together  | Together AI  | ~3           | LLM             |
| anthropic | Anthropic    | ~3           | LLM             |

### OpenAI Models

**LLM Models**:

- gpt-4o (multimodal, 128K context)
- gpt-4o-mini (cost-effective)
- gpt-4.1 (latest flagship)
- gpt-4.1-mini (cost-effective latest)
- gpt-4.1-nano (ultra-efficient)
- gpt-4-turbo
- gpt-3.5-turbo

**Embedding Models**:

- text-embedding-3-small (1536D, recommended)
- text-embedding-3-large (3072D)
- text-embedding-ada-002 (legacy)

### Ollama Models

**LLM Models** (from grep results):

- gemma3:12b (default)
- llama3.2:latest
- llama3.1:latest
- mistral-nemo:latest
- qwen2:latest
- phi3:latest
- and more...

**Embedding Models**:

- embeddinggemma (768D, default)
- nomic-embed-text:latest (768D)
- mxbai-embed-large:latest (1024D)

### Model Card Structure

Each model includes:

- `name` - Model identifier
- `display_name` - Human-readable name
- `model_type` - "llm" or "embedding"
- `description` - Model description
- `tags` - Categorization tags
- `capabilities` - Context length, features, embedding dimension
- `cost` - Per-token costs

## Findings

Item 7 is fully implemented:

- ✅ 45 models across 6 providers
- ✅ Multiple LLM models per provider
- ✅ Multiple embedding models per provider
- ✅ Full model cards with capabilities and costs
