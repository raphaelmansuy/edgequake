---
title: Python SDK
description: Install and use the EdgeQuake Python client (edgequake-sdk). Sync and async APIs for documents, query, chat, graph, parse and more.
---

# Python SDK

Official Python client for the EdgeQuake API. Requires **Python 3.10+**. Source version **0.4.0**; latest on PyPI is **0.3.0**.

```bash
pip install edgequake-sdk==0.3.0
# Source tree (0.4.0):
# pip install ./sdks/python
```

```python
from edgequake import EdgeQuake, AsyncEdgeQuake

client = EdgeQuake(
    base_url="http://localhost:8080",
    api_key="eq-...",           # or jwt="..."
    workspace_id="...",         # optional X-Workspace-ID
)
print(client.health().status)
```

## Resources

| Attribute | Covers |
|-----------|--------|
| `documents`, `pdf` | Upload, list, delete, track, lineage, PDF ops |
| `parse` | Stateless PDF → Markdown |
| `query`, `chat` | RAG query and chat (sync + stream) |
| `graph`, `entities`, `relationships` | Knowledge graph |
| `conversations`, `folders` | Chat history |
| `auth`, `users`, `api_keys`, `tenants` | Identity |
| `workspaces`, `tasks`, `pipeline`, `costs` | Ops |
| `lineage`, `chunks`, `provenance` | Provenance |
| `settings`, `models`, `admin` | Config |

Async twin: `AsyncEdgeQuake` with the same attribute names (`await client.query.execute(...)`).

## Common calls

```python
# Text upload → 202 with task_id
up = client.documents.upload(content="Hello graph.", title="Note")
print(up.document_id, up.task_id)

# PDF
pdf = client.pdf.upload("paper.pdf", title="Paper")
print(pdf.pdf_id, pdf.task_id)  # task_id is pdf-<uuid>

# Query (prefer mode="mix")
# Note: this client currently sends top_k/rerank; the server ignores unknown fields.
ans = client.query.execute(query="What is this about?", mode="mix")
print(ans.answer)

# Cancel
client.tasks.cancel(up.task_id)
```

Quickstart walkthrough: [quickstart.md](quickstart.md). API shapes: [REST API](../../api-reference/rest-api.md). Connections are **not** wrapped — use raw HTTP ([Connections](../../api-reference/connections.md)).
