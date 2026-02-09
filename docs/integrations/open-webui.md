# Integration: Open WebUI

> **Using EdgeQuake as an Ollama Backend for Open WebUI**

This guide shows how to connect [Open WebUI](https://github.com/open-webui/open-webui) to EdgeQuake for a ChatGPT-like interface with Graph-RAG capabilities.

---

## Overview

Open WebUI is a popular ChatGPT-style interface that connects to Ollama. EdgeQuake provides **Ollama API emulation**, allowing it to work as a drop-in replacement.

```
┌─────────────────────────────────────────────────────────────────┐
│                 OPEN WEBUI + EDGEQUAKE                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    Open WebUI                           │    │
│  │  ┌─────────────────────────────────────────────────┐    │    │
│  │  │ User: "What is the relationship between X and Y?"    │    │
│  │  │                                                      │    │
│  │  │ Assistant: Based on the documents, X and Y...   │    │    │
│  │  └─────────────────────────────────────────────────┘    │    │
│  └────────────────────────┬────────────────────────────────┘    │
│                           │                                     │
│                     Ollama API                                  │
│                    (POST /api/chat)                             │
│                           │                                     │
│                           ↓                                     │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    EdgeQuake                            │    │
│  │                                                         │    │
│  │  • Ollama API Emulation                                 │    │
│  │  • Graph-RAG Query Processing                           │    │
│  │  • Knowledge Graph Retrieval                            │    │
│  │  • LLM Response Generation                              │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Quick Start

### 1. Start EdgeQuake

```bash
# Start EdgeQuake
make dev

# Or with Docker
docker compose up -d
```

EdgeQuake runs on `http://localhost:8080` by default.

### 2. Start Open WebUI

```bash
# Docker (simplest)
docker run -d \
  -p 3000:8080 \
  -e OLLAMA_BASE_URL=http://host.docker.internal:8080 \
  --name open-webui \
  ghcr.io/open-webui/open-webui:main
```

For Linux, replace `host.docker.internal` with your host IP:

```bash
docker run -d \
  -p 3000:8080 \
  -e OLLAMA_BASE_URL=http://172.17.0.1:8080 \
  --name open-webui \
  ghcr.io/open-webui/open-webui:main
```

### 3. Access Open WebUI

Open `http://localhost:3000` in your browser.

**First Time Setup**:

1. Create an admin account
2. EdgeQuake appears as model "edgequake:latest"
3. Start chatting with your documents!

---

## Configuration

### Open WebUI Settings

Navigate to **Settings → Connections** in Open WebUI:

| Setting          | Value                   |
| ---------------- | ----------------------- |
| Ollama Base URL  | `http://localhost:8080` |
| Enable Streaming | ✅ Enabled              |

### EdgeQuake Model

In the model selector, you'll see:

- **edgequake:latest** - Graph-RAG with your documents

---

## Query Modes via Prefixes

EdgeQuake supports special **prefixes** in your messages to control query behavior:

| Prefix    | Mode   | Description                    |
| --------- | ------ | ------------------------------ |
| `/local`  | Local  | Entity-focused retrieval       |
| `/global` | Global | Relationship-focused retrieval |
| `/naive`  | Naive  | Vector search only (fastest)   |
| `/hybrid` | Hybrid | Combined mode (default)        |
| `/mix`    | Mix    | Adaptive blending              |
| `/bypass` | Bypass | Skip RAG, direct LLM           |

### Example Usage

```
User: /local Tell me about John Smith

User: /global What are the main themes in the documents?

User: /naive Find mentions of "climate change"

User: /bypass Just chat without using documents
```

The prefix is automatically stripped from the query.

---

## Docker Compose Setup

For a complete stack with Open WebUI + EdgeQuake:

```yaml
# docker-compose.yml
version: "3.8"

services:
  edgequake:
    image: ghcr.io/edgequake/edgequake:latest
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgresql://edgequake:edgequake@postgres:5432/edgequake
      - OPENAI_API_KEY=${OPENAI_API_KEY}
    depends_on:
      postgres:
        condition: service_healthy

  postgres:
    image: ghcr.io/edgequake/postgres-age:16
    environment:
      POSTGRES_USER: edgequake
      POSTGRES_PASSWORD: edgequake
      POSTGRES_DB: edgequake
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U edgequake"]
      interval: 5s
      timeout: 5s
      retries: 5

  open-webui:
    image: ghcr.io/open-webui/open-webui:main
    ports:
      - "3000:8080"
    environment:
      - OLLAMA_BASE_URL=http://edgequake:8080
    depends_on:
      - edgequake

volumes:
  postgres_data:
```

Start everything:

```bash
docker compose up -d
```

---

## Uploading Documents

Before chatting, upload documents to EdgeQuake:

### Via API

```bash
# Upload a document
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -H "X-Workspace-ID: default" \
  -F "file=@document.pdf"
```

### Via EdgeQuake WebUI

1. Open `http://localhost:8080` (EdgeQuake native UI)
2. Navigate to Documents
3. Drag and drop files

### Via Open WebUI (Future)

Open WebUI's document upload feature is not yet integrated with EdgeQuake's RAG pipeline. Use the EdgeQuake API or WebUI for document uploads.

---

## Streaming

EdgeQuake supports real-time streaming responses:

```
┌─────────────────────────────────────────────────────────────────┐
│                 STREAMING FLOW                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Open WebUI → POST /api/chat {"stream": true}                   │
│                                                                 │
│  EdgeQuake Response (newline-delimited JSON):                   │
│                                                                 │
│  {"message":{"content":"Based"},"done":false}                   │
│  {"message":{"content":" on"},"done":false}                     │
│  {"message":{"content":" the"},"done":false}                    │
│  {"message":{"content":" documents"},"done":false}              │
│  ...                                                            │
│  {"message":{"content":""},"done":true,"total_duration":1234}   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Ollama API Endpoints

EdgeQuake implements these Ollama-compatible endpoints:

| Endpoint        | Method | Description           |
| --------------- | ------ | --------------------- |
| `/api/version`  | GET    | API version info      |
| `/api/tags`     | GET    | List available models |
| `/api/ps`       | GET    | List running models   |
| `/api/generate` | POST   | Text completion       |
| `/api/chat`     | POST   | Chat completion       |

### Example: List Models

```bash
curl http://localhost:8080/api/tags
```

```json
{
  "models": [
    {
      "name": "edgequake:latest",
      "model": "edgequake:latest",
      "size": 7000000000,
      "digest": "sha256:edgequake-rag-v1",
      "modified_at": "2024-01-15T10:30:00.000000Z",
      "details": {
        "format": "gguf",
        "family": "edgequake",
        "parameter_size": "7B",
        "quantization_level": "Q4_0"
      }
    }
  ]
}
```

### Example: Chat Completion

```bash
curl -X POST http://localhost:8080/api/chat \
  -H "Content-Type: application/json" \
  -d '{
    "model": "edgequake:latest",
    "messages": [
      {"role": "user", "content": "What are the main topics?"}
    ],
    "stream": false
  }'
```

---

## Troubleshooting

### Open WebUI Can't Connect

**Symptom**: "Connection Error" in Open WebUI

**Check**:

1. EdgeQuake is running: `curl http://localhost:8080/health`
2. OLLAMA_BASE_URL is correct
3. Firewall allows connection

**Solution** (Docker):

```bash
# Use host.docker.internal on Mac/Windows
OLLAMA_BASE_URL=http://host.docker.internal:8080

# Use host IP on Linux
OLLAMA_BASE_URL=http://172.17.0.1:8080
```

### No Models Showing

**Symptom**: Model dropdown is empty

**Check**:

```bash
curl http://localhost:8080/api/tags
```

Should return `edgequake:latest`.

### Slow Responses

**Cause**: First query triggers LLM warm-up

**Solution**: Use streaming (`stream: true`) for perceived faster responses.

### Empty Responses

**Symptom**: Assistant returns empty or generic responses

**Check**: Documents are uploaded and processed:

```bash
curl http://localhost:8080/api/v1/documents?workspace_id=default
```

---

## Best Practices

1. **Upload Documents First**: Chat is only useful with document context
2. **Use Query Prefixes**: `/local` for entities, `/global` for themes
3. **Enable Streaming**: Better user experience
4. **Monitor Costs**: Check EdgeQuake cost dashboard for LLM usage
5. **Use Workspaces**: Organize documents by project/topic

---

## Limitations

| Feature                        | Status                  
| ------------------------------ | ----------------------- 
| Chat completions               | ✅ Full support         |
| Streaming                      | ✅ Full support         |
| Query modes                    | ✅ Via prefixes         |
| Document upload via Open WebUI | ❌ Not integrated       |
| Multiple models                | ⚠️ Shows edgequake only |
| Model pull                     | ❌ Not applicable       |
| Model creation                 | ❌ Not applicable       |

---

## See Also

- [REST API Reference](../api-reference/rest-api.md) - Full API documentation
- [Extended API Reference](../api-reference/extended-api.md) - Ollama emulation details
- [Quick Start Guide](../getting-started/quick-start.md) - Getting started with EdgeQuake
