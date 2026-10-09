---
title: "Tutorial: Migrate from LightRAG"
description: Move a LightRAG Python project to EdgeQuake. Map concepts, configuration and API calls, then re-ingest your documents and compare answers.
---

In this tutorial you move a project from [LightRAG](https://github.com/HKUDS/LightRAG) (Python) to EdgeQuake. EdgeQuake implements the same retrieval idea (entities, relationships and chunks in a knowledge graph) as a Rust server with a REST API and PostgreSQL storage.

**Prerequisites:** your original source documents (or the LightRAG working directory), Docker or a Rust toolchain, and an LLM and embedding provider. You can reuse the same OpenAI key.

## What changes

LightRAG is a Python library that you call in-process and that stores files in a `working_dir`. EdgeQuake is a server that you call over HTTP and that stores everything in PostgreSQL.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
  subgraph L["LightRAG"]
    A["Your Python app"] --> B["LightRAG library"]
    B --> C["working_dir files"]
  end
  subgraph E["EdgeQuake"]
    D["Any client"] --> F["REST API"]
    F --> G["PostgreSQL with pgvector and AGE"]
  end
%% eq-classes
classDef eqActor fill:#FCE7F3,stroke:#EC4899,color:#500724
classDef eqStore fill:#D1FAE5,stroke:#10B981,color:#064E3B
class D eqActor
class G eqStore
```

Read each box group separately. On the left your code and the library share one process. On the right the client and the server are separate, so many clients can share one knowledge base.

| Topic | LightRAG | EdgeQuake |
|-------|----------|-----------|
| Interface | Python class | REST API under `/api/v1`, WebSocket, Web UI, SDKs |
| Storage | JSON files, optional Neo4j and others | PostgreSQL (`DATABASE_URL` is required) |
| Isolation | One `working_dir` per project | Tenants and workspaces |
| Ingestion | `insert()` blocks | Upload returns at once; processing runs in the background |
| Answers | A string | Answer, sources and timing stats |
| Query modes | `naive`, `local`, `global`, `hybrid`, `mix`, `bypass` | The same names. The API default is `mix` |

This guide makes no speed or cost claim. Measure with your own data.

## Concept map

| LightRAG | EdgeQuake |
|----------|-----------|
| `LightRAG(working_dir=...)` | A workspace: `POST /api/v1/tenants/{tenant_id}/workspaces` |
| `rag.insert(text)` | `POST /api/v1/documents` |
| Insert a file | `POST /api/v1/documents/upload` (PDFs: `/api/v1/documents/pdf`) |
| `rag.query(q, param=QueryParam(mode=...))` | `POST /api/v1/query` with `"mode"` |
| Graph backend (file, Neo4j, ...) | Apache AGE inside PostgreSQL |
| Vector backend | pgvector inside PostgreSQL |
| Return value (a string) | `answer`, plus `sources` and `stats` |

Entity and relationship extraction follow the same approach, but prompts, types and merge rules are EdgeQuake's own. Expect similar, not identical, graphs. See [LightRAG algorithm](../deep-dives/lightrag-algorithm.md).

## Migration plan

There is no importer for LightRAG graph or vector files. You re-ingest the source documents and let EdgeQuake rebuild the graph and the vectors. This keeps entity names, types and embeddings consistent with one model.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
  A["1. Start EdgeQuake"] --> B["2. Map configuration"]
  B --> C["3. Create workspace"]
  C --> D["4. Collect source documents"]
  D --> E["5. Re-ingest"]
  E --> F["6. Compare answers"]
  F --> G["7. Switch your client"]
%% eq-classes
classDef eqActor fill:#FCE7F3,stroke:#EC4899,color:#500724
class G eqActor
```

Read it top to bottom. Keep LightRAG running until step 6 shows that answers match your needs.

## 1. Start EdgeQuake

The fastest start is the Docker quickstart. It starts PostgreSQL, applies the schema, then starts the API and the Web UI.

```bash
curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/quickstart.sh | sh
```

To use OpenAI without prompts, run `sh quickstart.sh --yes --provider openai` with `OPENAI_API_KEY` set. More options are in [Getting started](../getting-started/index.md) and [Configure LLM providers](../providers/index.md).

From source, run `make dev`. It starts PostgreSQL, applies migrations and starts the backend and frontend.

If you run the binary yourself, apply the schema first. The API never changes the schema on its own; `edgequake migrate` is the only command that does:

```bash
export DATABASE_URL="postgresql://user:pass@localhost:5432/edgequake"
edgequake migrate
edgequake
```

Check the server:

```bash
export EQ_API=http://localhost:8080      # make dev uses 8090 by default
curl -s "$EQ_API/health" | jq '{status, version, llm_provider_name}'
```

## 2. Map configuration

LightRAG passes functions to the constructor. EdgeQuake reads environment variables, and each workspace can override them.

```bash
export OPENAI_API_KEY="sk-..."
export EDGEQUAKE_LLM_PROVIDER="openai"
export EDGEQUAKE_LLM_MODEL="<your-chat-model>"
export EDGEQUAKE_EMBEDDING_MODEL="<your-embedding-model>"
```

To reuse an existing LightRAG `.env`, these aliases also work: `MODEL_PROVIDER` or `CHAT_PROVIDER`, `CHAT_MODEL` or `LLM_MODEL`, and `EMBEDDING_MODEL`. If both an alias and the `EDGEQUAKE_*` name are set, the `EDGEQUAKE_*` value wins. The full list is in the [environment reference](../operations/env-reference.md). Available model names are in `edgequake/models.toml`.

Two cautions:

- An embedding model fixes the vector size of a workspace. Choose it before you ingest. If you change it later you must rebuild the embeddings.
- Chunk size differs. LightRAG's default is 1200 tokens with 100 overlap. EdgeQuake chooses a size from the document and the workspace policy. See [Chunking strategies](../deep-dives/chunking-strategies.md) to set a fixed size.

## 3. Create a workspace

A fresh server has a default tenant (ID `00000000-0000-0000-0000-000000000002`). Create one workspace per LightRAG `working_dir`. With auth on, add credentials as shown in [Auth quickstart](../operations/auth-quickstart.md).

```bash
export TENANT_ID=00000000-0000-0000-0000-000000000002

export WORKSPACE_ID=$(curl -s -X POST "$EQ_API/api/v1/tenants/$TENANT_ID/workspaces" \
  -H "Content-Type: application/json" \
  -d '{"name": "my-project", "description": "Migrated from LightRAG"}' | jq -r '.id')
echo "$WORKSPACE_ID"
```

For a SaaS with many customers, create one tenant per customer first. See [Multi-tenant deployment](multi-tenant.md).

## 4. Collect source documents

Use your original files if you have them. This is the best option.

If you only have the LightRAG working directory, the full document text is stored in its key-value files. In recent LightRAG versions the file is `kv_store_full_docs.json`. This guide did not verify the file layout of every LightRAG version, so open the file and check it. The sketch below assumes a JSON object that maps a document ID to an object with a `content` field.

```python
import json, pathlib

src = pathlib.Path("./rag_storage/kv_store_full_docs.json")   # check the name in your version
out = pathlib.Path("./export"); out.mkdir(exist_ok=True)

for doc_id, doc in json.loads(src.read_text()).items():
    (out / f"{doc_id}.txt").write_text(doc["content"])
```

## 5. Re-ingest

Upload each file. Loop over the folder and keep the track IDs:

```bash
for f in export/*.txt; do
  curl -s -X POST "$EQ_API/api/v1/documents/upload" \
    -H "X-Workspace-ID: $WORKSPACE_ID" \
    -F "file=@$f" | jq -r '"\(.filename) \(.status) \(.track_id)"'
done
```

Upload many files in one call with `POST /api/v1/documents/upload/batch` (repeat the `files` field). Send PDFs to `/api/v1/documents/pdf` or `/api/v1/documents/pdf/batch`. See [PDF ingestion](pdf-ingestion.md).

Wait until all documents are done, then compare totals:

```bash
curl -s "$EQ_API/api/v1/workspaces/$WORKSPACE_ID/stats" \
  | jq '{document_count, chunk_count, entity_count, relationship_count}'
```

Ingestion calls your LLM once or more per chunk, so a large corpus costs money. Estimate first with `POST /api/v1/pipeline/costs/estimate` and read [Cost tracking](../deep-dives/cost-tracking.md).

## 6. Compare answers

Ask the same questions in both systems. In EdgeQuake the mode names are the same:

```bash
for MODE in naive local global hybrid mix; do
  curl -s -X POST "$EQ_API/api/v1/query" \
    -H "Content-Type: application/json" \
    -H "X-Workspace-ID: $WORKSPACE_ID" \
    -d "{\"query\": \"What is X?\", \"mode\": \"$MODE\"}" | jq -r --arg mode "$MODE" '"== " + $mode + "\n" + .answer'
done
```

The response differs from LightRAG. LightRAG returns a string. EdgeQuake returns an object:

```json
{
  "answer": "X is ...",
  "mode": "mix",
  "sources": [
    { "source_type": "chunk", "id": "...", "document_id": "...", "score": 0.89, "snippet": "..." }
  ],
  "stats": { "total_time_ms": 2500, "retrieval_time_ms": 400, "generation_time_ms": 2000 }
}
```

If answers differ, tune retrieval with [Query optimization](query-optimization.md). Differences in extraction are normal because the LLM is not deterministic.

## 7. Switch your client

Replace the LightRAG calls with HTTP calls. This small wrapper keeps the old shape:

```python
import requests

class EdgeQuake:
    def __init__(self, base_url, workspace_id, api_key=None):
        self.base = base_url.rstrip("/")
        self.headers = {"X-Workspace-ID": workspace_id}
        if api_key:
            self.headers["X-API-Key"] = api_key

    def insert(self, content, title="Untitled"):
        r = requests.post(f"{self.base}/api/v1/documents", headers=self.headers,
                          json={"title": title, "content": content})
        r.raise_for_status()
        return r.json()

    def query(self, question, mode="mix"):
        r = requests.post(f"{self.base}/api/v1/query", headers=self.headers,
                          json={"query": question, "mode": mode})
        r.raise_for_status()
        return r.json()["answer"]
```

Unlike LightRAG's `insert()`, `insert` here returns before processing ends. Poll `GET /api/v1/documents/{document_id}` until `ui_phase` is `terminal` (see [Document ingestion](document-ingestion.md#2-track-progress)). The official Python, TypeScript and Rust clients are listed in [SDKs](../sdks/README.md).

## What you gain

- Sources and timing on every answer, and lineage back to chunks: [Tracing entity sources](tracing-entity-sources.md).
- Tenants, workspaces, API keys and optional SSO.
- PDF conversion with vision models and page-aware chunking.
- A Web UI for documents, graph and queries.
- Hand-written knowledge: [Knowledge injection](knowledge-injection.md).

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| Server exits at start with a schema message | The schema is not applied. | Run `edgequake migrate`, then start the server. See [Upgrading](../operations/upgrading.md). |
| Server exits asking for `DATABASE_URL` | PostgreSQL is required. | Set `DATABASE_URL`, or use `make dev` or the Docker quickstart. |
| Fewer entities than LightRAG | Different prompts, entity types or model. | Set `entity_types` on the workspace; try a larger model. See [Document ingestion](document-ingestion.md). |
| Document `failed` | Provider unreachable or rate limited. | Run `edgequake doctor`, then reprocess. |
| `401` or `403` | Auth is on. | Add credentials and `X-Tenant-ID`. See [Auth quickstart](../operations/auth-quickstart.md). |

## Next steps

- [First RAG app](first-rag-app.md)
- [Document ingestion](document-ingestion.md)
- [Upgrading EdgeQuake](../operations/upgrading.md)
