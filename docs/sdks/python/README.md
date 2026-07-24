---
title: "Python SDK"
---

# Python SDK

> **Product: v0.19.0** · Contract: [`openapi.snapshot.json`](../../../edgequake_webui/openapi/openapi.snapshot.json) · Spec ops: [Ingestion cancel & fairness](../../ingestion-cancel-and-fairness.md)

**Location:** `sdks/python`  
**PyPI name:** `edgequake-sdk` (from `sdks/python/pyproject.toml`)

## Install

```bash
pip install edgequake-sdk
```

WebSocket progress (async pipeline):

```bash
pip install edgequake-sdk[ws]
```

From source:

```bash
cd sdks/python && pip install -e ".[dev]"
```

## 30-second example

```python
from edgequake import EdgeQuake
from edgequake.types.documents import DocumentListParams
from edgequake.types.query import QueryRequest

client = EdgeQuake(
    base_url="http://localhost:8080",
    api_key="YOUR_KEY",          # when auth enabled
    tenant_id="…",               # multi-tenant
    user_id="…",
    workspace_id="default",
)

assert client.health().status == "healthy"

# List documents — lawful query keys only
page = client.documents.list(
    params=DocumentListParams(page=1, page_size=20, document_pattern="report")
)
for doc in page.documents:
    print(doc.id, doc.status)  # API also exposes display_status / ui_phase (see OpenAPI)

# Query — answer + sources (+ stats), not top-level chunks
result = client.query.execute(QueryRequest(query="What is EdgeQuake?", mode="hybrid"))
print(result.answer)
for src in result.sources:
    print(src.snippet, src.score)

client.close()
```

## PDF upload & cancel

```python
from pathlib import Path

from edgequake import EdgeQuake

with EdgeQuake(base_url="http://localhost:8080", workspace_id="default") as client:
    upload = client.pdf.upload(
        Path("/path/to/paper.pdf"),
        title="Paper",
        enable_vision=True,
    )
    task_id = upload.task_id  # progress + cancel SSOT
    client.tasks.cancel(task_id)
```

## Async

```python
from edgequake import AsyncEdgeQuake

async with AsyncEdgeQuake(base_url="http://localhost:8080") as client:
    health = await client.health()
    result = await client.query.execute(query="Hello")
```

## See also

- [Quickstart](./quickstart.md)
- [Custom Clients](../../integrations/custom-clients.md) — raw HTTP fallback
- In-repo reference: `sdks/python/README.md`, `sdks/python/docs/API.md`
