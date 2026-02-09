# Quick Start Guide

> From zero to your first knowledge graph query in 10 minutes

---

## What You'll Build

By the end of this guide, you will have:

1. ✅ Ingested a document into EdgeQuake
2. ✅ Built a knowledge graph from extracted entities
3. ✅ Queried the graph using natural language
4. ✅ Visualized the knowledge graph in the WebUI

```
┌─────────────────────────────────────────────────────────────┐
│                      Your First Flow                        │
│                                                             │
│   Document ───▶ [EdgeQuake] ───▶ Knowledge Graph            │
│   "Marie Curie       │          ┌───────────────┐           │
│    discovered        │          │ MARIE_CURIE   │           │
│    radium..."        │          │      │        │           │
│                      │          │      ▼        │           │
│                      │          │   RADIUM      │           │
│                      │          └───────────────┘           │
│                      │                                      │
│   Query ─────────────┴───▶ "Marie Curie discovered radium   │
│   "Who discovered            in 1898..."                    │
│    radium?"                                                 │
└─────────────────────────────────────────────────────────────┘
```

---

## Prerequisites

Ensure EdgeQuake is running:

```bash
# Check health
curl http://localhost:8080/health
# Expected: {"status":"ok",...}
```

If not running, see [Installation Guide](installation.md).

---

## Step 1: Ingest Your First Document

### Option A: Via REST API

```bash
# Create a simple document about a famous scientist
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Marie Curie was a Polish-French physicist and chemist who conducted pioneering research on radioactivity. She was the first woman to win a Nobel Prize, and the only person to win Nobel Prizes in two different sciences (Physics in 1903, Chemistry in 1911). Curie discovered two elements: polonium (named after Poland) and radium. She worked at the University of Paris with her husband Pierre Curie. Their daughter, Irène Joliot-Curie, also won a Nobel Prize in Chemistry in 1935.",
    "title": "Marie Curie Biography"
  }'
```

**Expected Response**:

```json
{
  "document_id": "doc_abc123",
  "entities_extracted": 8,
  "relationships_extracted": 6,
  "chunks_created": 1,
  "processing_time_ms": 2500
}
```

### Option B: Via WebUI

1. Open http://localhost:3000
2. Navigate to **Documents** → **Upload**
3. Paste the text above or upload a file
4. Click **Process**

---

## Step 2: Explore the Knowledge Graph

### View Extracted Entities

```bash
curl http://localhost:8080/api/v1/entities | jq '.entities[:5]'
```

**Expected Entities**:

```
┌─────────────────────────────────────────────────┐
│ Extracted Knowledge Graph                       │
├─────────────────────────────────────────────────┤
│                                                 │
│   ┌─────────────┐       ┌─────────────┐         │
│   │MARIE_CURIE  │──────▶│NOBEL_PRIZE  │         │
│   │  (PERSON)   │       │  (EVENT)    │         │
│   └──────┬──────┘       └─────────────┘         │
│          │                                      │
│          │ married_to                           │
│          ▼                                      │
│   ┌─────────────┐       ┌─────────────┐         │
│   │PIERRE_CURIE │       │  POLAND     │         │
│   │  (PERSON)   │       │ (LOCATION)  │         │
│   └─────────────┘       └─────────────┘         │
│          │                    ▲                 │
│          │                    │ named_after     │
│          ▼                    │                 │
│   ┌─────────────┐       ┌─────────────┐         │
│   │UNIV_PARIS   │       │  POLONIUM   │         │
│   │(ORGANIZATION│       │ (CONCEPT)   │         │
│   └─────────────┘       └─────────────┘         │
│                                                 │
└─────────────────────────────────────────────────┘
```

### View Relationships

```bash
curl http://localhost:8080/api/v1/relationships | jq '.relationships[:5]'
```

**Sample Relationships**:

```json
[
  {
    "source": "MARIE_CURIE",
    "target": "NOBEL_PRIZE",
    "keywords": ["won", "received", "awarded"],
    "description": "Marie Curie won Nobel Prizes in Physics and Chemistry"
  },
  {
    "source": "MARIE_CURIE",
    "target": "PIERRE_CURIE",
    "keywords": ["married", "worked_with", "collaborated"],
    "description": "Marie Curie was married to and collaborated with Pierre Curie"
  }
]
```

---

## Step 3: Query the Knowledge Graph

### Simple Query

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "Who discovered radium and when?",
    "mode": "hybrid"
  }'
```

**Expected Response**:

```json
{
  "response": "Marie Curie discovered radium. She was a Polish-French physicist and chemist who conducted pioneering research on radioactivity. Curie also discovered polonium, which was named after Poland.",
  "sources": [
    {
      "type": "entity",
      "name": "MARIE_CURIE",
      "relevance": 0.95
    },
    {
      "type": "entity",
      "name": "RADIUM",
      "relevance": 0.92
    }
  ],
  "mode": "hybrid",
  "context_tokens": 450
}
```

### Try Different Query Modes

```bash
# Local mode: Entity-focused (best for specific facts)
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "What is radium?", "mode": "local"}'

# Global mode: Community-based (best for overview questions)
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "Summarize the Curie family achievements", "mode": "global"}'

# Naive mode: Vector search only (traditional RAG)
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "Who won Nobel Prizes?", "mode": "naive"}'
```

---

## Step 4: Visualize in WebUI

1. Open http://localhost:3000
2. Navigate to **Graph** (left sidebar)
3. See your knowledge graph visualization

```
┌─────────────────────────────────────────────────────────────┐
│                    WebUI Graph View                         │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                     ○ NOBEL_PRIZE                     │  │
│  │                    ╱                                  │  │
│  │         ○ MARIE_CURIE ──────○ RADIUM                  │  │
│  │        ╱│╲                                            │  │
│  │       ╱ │ ╲                                           │  │
│  │      ○  ○  ○                                          │  │
│  │   PIERRE POLAND POLONIUM                              │  │
│  │                                                       │  │
│  │  [Zoom] [Pan] [Reset] [Filter: PERSON ▼]              │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

**WebUI Features**:

- 🔍 **Zoom & Pan**: Mouse wheel and drag
- 🎯 **Click nodes**: See entity details
- 🔗 **Click edges**: See relationship details
- 📊 **Filter**: By entity type
- 🔎 **Search**: Find specific entities

---

## Step 5: Add More Documents

Build a richer knowledge graph:

```bash
# Add a related document
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Albert Einstein developed the theory of relativity while working at the Swiss Patent Office in Bern. He won the Nobel Prize in Physics in 1921 for his explanation of the photoelectric effect. Einstein corresponded with Marie Curie and they became friends. Both attended the famous Solvay Conference in 1911.",
    "title": "Albert Einstein"
  }'
```

Now query across both documents:

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "What connections existed between Einstein and Curie?"}'
```

**Expected**: The response should mention their friendship, the Solvay Conference, and both winning Nobel Prizes.

---

## Understanding What Happened

```
┌─────────────────────────────────────────────────────────────┐
│                    Processing Pipeline                      │
│                                                             │
│  1. CHUNKING                                                │
│     └─ Document split into 1200-token chunks                │
│                                                             │
│  2. ENTITY EXTRACTION                                       │
│     └─ LLM identifies: MARIE_CURIE, PIERRE_CURIE, etc.      │
│                                                             │
│  3. RELATIONSHIP EXTRACTION                                 │
│     └─ LLM finds: "married_to", "discovered", etc.          │
│                                                             │
│  4. EMBEDDING                                               │
│     └─ Vector embeddings for chunks, entities, relations    │
│                                                             │
│  5. GRAPH CONSTRUCTION                                      │
│     └─ Nodes + Edges stored in knowledge graph              │
│                                                             │
│  6. DEDUPLICATION                                           │
│     └─ Similar entities merged (MARIE_CURIE = Curie)        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Quick Reference: API Endpoints

| Endpoint                | Method | Purpose               |
| ----------------------- | ------ | --------------------- |
| `/health`               | GET    | Check server status   |
| `/api/v1/documents`     | POST   | Ingest document       |
| `/api/v1/documents`     | GET    | List documents        |
| `/api/v1/query`         | POST   | Query knowledge graph |
| `/api/v1/entities`      | GET    | List entities         |
| `/api/v1/relationships` | GET    | List relationships    |
| `/api/v1/graph/stats`   | GET    | Graph statistics      |

---

## Next Steps

Now that you've completed the quick start:

1. **[First Ingestion Deep Dive](first-ingestion.md)** — Understanding the pipeline
2. **[Architecture Overview](../architecture/overview.md)** — System design
3. **[Query Modes](../deep-dives/query-modes.md)** — Choosing the right mode
4. **[API Reference](../api-reference/rest-api.md)** — Full API documentation

---

## Troubleshooting

### No entities extracted

```bash
# Check LLM provider is responding
curl http://localhost:8080/api/v1/config | jq .llm_provider

# If using Ollama, verify model is available
ollama list
```

### Slow processing

```bash
# Check if using local (Ollama) vs cloud (OpenAI)
# OpenAI is typically faster for small documents
export OPENAI_API_KEY="sk-..."
# Restart backend
```

### Empty query results

```bash
# Verify documents exist
curl http://localhost:8080/api/v1/documents | jq '.documents | length'

# Check graph has nodes
curl http://localhost:8080/api/v1/graph/stats
```
