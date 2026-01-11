# Provider Setup Guide

This guide covers setting up different LLM and embedding providers for EdgeQuake.

**SPEC-032**: Ollama/LM Studio Provider Integration  
**OODA Loop**: Iterations 41-45

---

## Quick Reference

| Provider | Chat Model | Embedding Model | Dimension | Env Prefix |
|----------|------------|-----------------|-----------|------------|
| **OpenAI** | gpt-4o-mini | text-embedding-3-small | 1536 | `OPENAI_` |
| **Ollama** | gemma3:12b | embeddinggemma:latest | 768 | `OLLAMA_` |
| **LM Studio** | gemma2-9b-it | nomic-embed-text-v1.5 | 768 | `LMSTUDIO_` |
| **Mock** | mock | mock | 384 | (none) |

---

## 1. OpenAI Provider

The default provider for production deployments.

### Requirements

- OpenAI API key from [platform.openai.com](https://platform.openai.com)

### Configuration

```bash
# Required
export OPENAI_API_KEY="sk-your-api-key-here"

# Optional overrides
export OPENAI_BASE_URL="https://api.openai.com/v1"
export OPENAI_MODEL="gpt-4o-mini"              # Chat model
export OPENAI_EMBEDDING_MODEL="text-embedding-3-small"  # Embedding model
```

### Models

| Type | Model | Dimensions | Cost (per 1M tokens) |
|------|-------|------------|---------------------|
| Chat | gpt-4o-mini | - | $0.15 input / $0.60 output |
| Chat | gpt-4o | - | $2.50 input / $10 output |
| Embedding | text-embedding-3-small | 1536 | $0.02 |
| Embedding | text-embedding-3-large | 3072 | $0.13 |

### Usage

```bash
# Start with OpenAI
export OPENAI_API_KEY="sk-..."
make dev
```

---

## 2. Ollama Provider

Local LLM inference, no cloud required.

### Requirements

1. Install Ollama: [ollama.ai](https://ollama.ai)
2. Pull required models

### Installation

```bash
# macOS/Linux
curl -fsSL https://ollama.ai/install.sh | sh

# Start Ollama service
ollama serve

# Pull models
ollama pull gemma3:12b           # Chat model (12GB)
ollama pull embeddinggemma       # Embedding model (~1GB)
```

### Configuration

```bash
# Auto-detected if Ollama is running locally
# Or configure explicitly:

export OLLAMA_HOST="http://localhost:11434"
export OLLAMA_MODEL="gemma3:12b"
export OLLAMA_EMBEDDING_MODEL="embeddinggemma:latest"
```

### Models

| Type | Model | Size | Dimensions |
|------|-------|------|------------|
| Chat | gemma3:12b | 12GB | - |
| Chat | llama3.1:8b | 4.7GB | - |
| Chat | mistral:7b | 4.1GB | - |
| Embedding | embeddinggemma | ~1GB | 768 |
| Embedding | nomic-embed-text | ~300MB | 768 |

### Remote Ollama

For running Ollama on a different machine:

```bash
export OLLAMA_HOST="http://192.168.1.100:11434"
```

### Usage

```bash
# Start with Ollama (auto-detected)
ollama serve &
make dev

# Or explicit configuration
export OLLAMA_HOST="http://localhost:11434"
make dev
```

---

## 3. LM Studio Provider

Desktop app for running local LLMs with a nice UI.

### Requirements

1. Download LM Studio: [lmstudio.ai](https://lmstudio.ai)
2. Load models in the UI
3. Start local server (Cmd+Shift+S or via menu)

### Installation

1. Download LM Studio from [lmstudio.ai](https://lmstudio.ai)
2. Open the app and download models from the Discover tab:
   - Chat: `gemma-2-9b-it-GGUF` (recommended)
   - Embedding: `nomic-embed-text-v1.5`
3. Load the models
4. Start Local Server:
   - Menu → Developer → Start Local Server
   - Or press `Cmd+Shift+S` (macOS) / `Ctrl+Shift+S` (Windows)
   - Default port: 1234

### Configuration

```bash
export LMSTUDIO_HOST="http://localhost:1234"
export LMSTUDIO_MODEL="gemma2-9b-it"
export LMSTUDIO_EMBEDDING_MODEL="nomic-embed-text-v1.5"
export LMSTUDIO_EMBEDDING_DIMENSION="768"
```

### Models

| Type | Model | Context | Dimensions |
|------|-------|---------|------------|
| Chat | gemma-2-9b-it | 8k | - |
| Chat | llama-3.1-8b | 8k | - |
| Embedding | nomic-embed-text-v1.5 | - | 768 |
| Embedding | gte-large | - | 1024 |

### Usage

```bash
# 1. Start LM Studio server (Cmd+Shift+S in app)
# 2. Configure environment
export LMSTUDIO_HOST="http://localhost:1234"
make dev
```

---

## 4. Mock Provider

For testing without real LLM. Returns deterministic fake responses.

### Configuration

No configuration needed - used automatically when no other provider is configured.

### Usage

```bash
# Just run without any provider env vars
unset OPENAI_API_KEY OLLAMA_HOST LMSTUDIO_HOST
cargo test  # Uses mock automatically
```

---

## Provider Auto-Detection

EdgeQuake automatically detects which provider to use based on environment variables:

```
Priority Order:
1. Ollama (if OLLAMA_HOST or OLLAMA_MODEL set)
2. LM Studio (if LMSTUDIO_HOST or LMSTUDIO_MODEL set)
3. OpenAI (if OPENAI_API_KEY set)
4. Mock (fallback for testing)
```

### Detection Logic

```rust
// Pseudocode for provider selection
if env::var("OLLAMA_HOST").is_ok() || env::var("OLLAMA_MODEL").is_ok() {
    use OllamaProvider
} else if env::var("LMSTUDIO_HOST").is_ok() || env::var("LMSTUDIO_MODEL").is_ok() {
    use LMStudioProvider
} else if env::var("OPENAI_API_KEY").is_ok() {
    use OpenAIProvider
} else {
    use MockProvider
}
```

---

## Switching Providers

### At Query Time

Use the provider selector dropdown in the Query interface:

1. Open Query page
2. Click the provider dropdown (next to mode selector)
3. Select desired provider (OpenAI, Ollama, LM Studio)

Provider selection persists in localStorage.

### At Workspace Creation

When creating a new workspace, select the embedding model:

1. Click "Create New Workspace" in header
2. Expand "Embedding Model" section
3. Select provider and model
4. Click "Create"

### Changing Existing Workspace

To change embedding model for an existing workspace (requires vector rebuild):

```bash
# API call
curl -X POST "http://localhost:8080/api/v1/workspaces/{workspace_id}/rebuild-embeddings" \
  -H "Content-Type: application/json" \
  -d '{
    "embedding_model": "embeddinggemma:latest",
    "embedding_provider": "ollama",
    "embedding_dimension": 768,
    "force": true
  }'
```

**Warning**: This clears all existing embeddings. Documents must be re-ingested.

---

## Dimension Compatibility

Embedding dimensions must match between:
- Workspace embedding configuration
- Vector storage (PostgreSQL or Memory)
- All documents in the workspace

| Provider | Default Model | Dimension |
|----------|---------------|-----------|
| OpenAI | text-embedding-3-small | **1536** |
| Ollama | embeddinggemma:latest | **768** |
| LM Studio | nomic-embed-text-v1.5 | **768** |
| Mock | mock | **384** |

### Mismatch Warning

If you see a dimension mismatch error:

```
Error: Vector dimension mismatch: expected 1536, got 768
```

You need to rebuild embeddings:

1. Use the rebuild endpoint (see above)
2. Or delete and recreate the workspace
3. Re-ingest all documents

---

## Environment Variables Reference

### OpenAI

| Variable | Default | Description |
|----------|---------|-------------|
| `OPENAI_API_KEY` | - | API key (required) |
| `OPENAI_BASE_URL` | `https://api.openai.com/v1` | API base URL |
| `OPENAI_MODEL` | `gpt-4o-mini` | Chat model |
| `OPENAI_EMBEDDING_MODEL` | `text-embedding-3-small` | Embedding model |

### Ollama

| Variable | Default | Description |
|----------|---------|-------------|
| `OLLAMA_HOST` | `http://localhost:11434` | Ollama server URL |
| `OLLAMA_MODEL` | `gemma3:12b` | Chat model |
| `OLLAMA_EMBEDDING_MODEL` | `embeddinggemma:latest` | Embedding model |

### LM Studio

| Variable | Default | Description |
|----------|---------|-------------|
| `LMSTUDIO_HOST` | `http://localhost:1234` | LM Studio server URL |
| `LMSTUDIO_MODEL` | `gemma2-9b-it` | Chat model |
| `LMSTUDIO_EMBEDDING_MODEL` | `nomic-embed-text-v1.5` | Embedding model |
| `LMSTUDIO_EMBEDDING_DIMENSION` | `768` | Embedding dimension |

### Server Defaults

| Variable | Default | Description |
|----------|---------|-------------|
| `EDGEQUAKE_DEFAULT_EMBEDDING_MODEL` | (provider default) | Default embedding model |
| `EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER` | (auto-detected) | Default embedding provider |
| `EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION` | (auto-detected) | Default embedding dimension |

---

## Troubleshooting

### Provider Not Available

```
Error: Provider 'ollama' is not available
```

**Fix**: Ensure the provider is running and environment variables are set.

```bash
# Check Ollama
curl http://localhost:11434/api/version

# Check LM Studio
curl http://localhost:1234/v1/models
```

### Connection Refused

```
Error: Connection refused (localhost:11434)
```

**Fix**: Start the provider service.

```bash
# Ollama
ollama serve

# LM Studio
# Start server via UI: Cmd+Shift+S
```

### Model Not Found

```
Error: Model 'gemma3:12b' not found
```

**Fix**: Pull or download the model.

```bash
# Ollama
ollama pull gemma3:12b

# LM Studio
# Download via Discover tab in UI
```

### Dimension Mismatch

```
Error: Embedding dimension mismatch: 1536 vs 768
```

**Fix**: Rebuild embeddings with matching dimension.

```bash
curl -X POST ".../rebuild-embeddings" -d '{"force": true}'
```

---

## Performance Comparison

| Provider | Latency (avg) | Throughput | Cost |
|----------|---------------|------------|------|
| OpenAI | 200-500ms | High | $$ |
| Ollama (local) | 500-2000ms | Medium | Free |
| LM Studio (local) | 500-2000ms | Medium | Free |
| Mock | <1ms | Very High | Free |

### Recommendations

- **Development**: Ollama or LM Studio (free, fast iteration)
- **Testing**: Mock (instant, deterministic)
- **Production**: OpenAI (reliable, high quality)
- **On-Premise**: Ollama with remote server

---

**Last Updated**: 2025-01-11  
**SPEC**: 032-ollama-lmstudio-provider  
**OODA Loops**: 41-45
