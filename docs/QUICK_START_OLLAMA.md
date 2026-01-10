# Quick Start with Ollama

EdgeQuake now defaults to using **Ollama** for local development, providing a free and fast alternative to OpenAI.

## Prerequisites

1. **Install Ollama**: https://ollama.ai/download
2. **Pull Required Models**:
   ```bash
   # LLM model (7B parameters, good quality/speed balance)
   ollama pull qwen2.5:7b
   
   # Embedding model (768 dimensions)
   ollama pull nomic-embed-text
   ```

3. **Verify Ollama is Running**:
   ```bash
   curl http://localhost:11434/api/version
   # Should return: {"version":"..."}
   ```

## Start Development

Simply run:

```bash
make dev
```

This will:
- ✅ Start PostgreSQL database
- ✅ Start backend API with Ollama provider
- ✅ Start frontend WebUI
- ✅ Configure LLM to use `qwen2.5:7b` model
- ✅ Configure embedding to use `nomic-embed-text` model

## Verify Provider Status

1. Open the WebUI: http://localhost:3000
2. Navigate to **Settings** page
3. Check the **Provider Status** card

You should see:
- **LLM Provider**: Ollama (qwen2.5:7b)
- **Embedding Provider**: Ollama (nomic-embed-text) - 768 dimensions
- **Status**: 🟢 Connected

## Alternative Providers

### OpenAI (Production)

Create a `.env` file:

```bash
OPENAI_API_KEY=sk-your-key-here
LLM_PROVIDER=openai
LLM_MODEL=gpt-4o-mini
EMBEDDING_PROVIDER=openai
EMBEDDING_MODEL=text-embedding-3-small
```

Then:
```bash
make dev
```

### LM Studio (Alternative Local)

1. Start LM Studio server
2. Create `.env`:

```bash
LLM_PROVIDER=lmstudio
LLM_MODEL=your-model-name
LLM_BASE_URL=http://localhost:1234/v1
EMBEDDING_PROVIDER=lmstudio
EMBEDDING_MODEL=your-embedding-model
EMBEDDING_BASE_URL=http://localhost:1234/v1
```

## Model Recommendations

### Ollama Models

| Use Case | LLM Model | Embedding Model | Total RAM |
|----------|-----------|-----------------|-----------|
| **Development** (recommended) | `qwen2.5:7b` | `nomic-embed-text` | ~8 GB |
| **Fast Testing** | `qwen2.5:3b` | `nomic-embed-text` | ~5 GB |
| **High Quality** | `qwen2.5:14b` | `nomic-embed-text` | ~16 GB |
| **Best Quality** | `qwen2.5:32b` | `nomic-embed-text` | ~32 GB |

### OpenAI Models

| Use Case | LLM Model | Embedding Model | Cost/1K tokens |
|----------|-----------|-----------------|----------------|
| **Production** | `gpt-4o-mini` | `text-embedding-3-small` | $0.0015 |
| **High Quality** | `gpt-4o` | `text-embedding-3-large` | $0.005 |

## Troubleshooting

### Ollama Not Responding

**Symptom**: Backend shows "Provider: Mock (1536 dimensions)"

**Solution**:
```bash
# Check Ollama is running
curl http://localhost:11434/api/version

# If not running, start it
ollama serve &

# Verify models are pulled
ollama list
```

### Dimension Mismatch Warning

**Symptom**: WebUI shows "⚠️ Dimension Mismatch" banner

**Cause**: Storage was initialized with different embedding dimension (e.g., 1536 from OpenAI)

**Solution**:
```bash
# Option 1: Clean database and restart
make db-clean-force
make dev

# Option 2: Switch to matching provider
# If storage has 1536 dimensions, use OpenAI or Mock provider
```

### Models Not Found

**Symptom**: Backend crashes with "model not found" error

**Solution**:
```bash
# Pull missing models
ollama pull qwen2.5:7b
ollama pull nomic-embed-text

# Restart backend
make stop
make dev
```

## Development Workflow

### 1. Start Stack
```bash
make dev
```

### 2. Upload Document
```bash
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -F "file=@your-document.pdf" \
  -F "title=My Document"
```

### 3. Query Knowledge Graph
```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "What are the main topics?", "mode": "hybrid"}'
```

### 4. View Results in WebUI
- Open http://localhost:3000
- Navigate to **Query** page
- Enter your question
- View graph visualization + answer

## Performance Benchmarks

### Ollama (qwen2.5:7b on M1 Max 64GB)
- **Document Ingestion**: ~2-3 docs/minute
- **Query Response**: ~1-2 seconds
- **Cost**: Free (local)

### OpenAI (gpt-4o-mini)
- **Document Ingestion**: ~10-15 docs/minute
- **Query Response**: ~500-800ms
- **Cost**: $0.0014/document, $0.0003/query

## Next Steps

- ✅ Completed: [Provider Status Visibility](./0005-llm-integration.md#provider-status-visibility)
- 🚧 In Progress: Provider health checks with timeout handling
- 📋 Planned: Dynamic provider switching in WebUI

See [SPEC-032](../specs/032-ollama-lmstudio-provider.md) for the full roadmap.

---

**Pro Tip**: Use `make dev-bg` to run services in background mode for automated testing or agent workflows. Logs are saved to `/tmp/edgequake-*.log`.
