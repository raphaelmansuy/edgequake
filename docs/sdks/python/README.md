---
title: "Python SDK"
description: "Python SDK for EdgeQuake."
---

# Python SDK

> **Product: v0.23.0** · Contract: [`openapi.snapshot.json`](../../../edgequake_webui/openapi/openapi.snapshot.json) · Spec ops: [Ingestion cancel & fairness](../../ingestion-cancel-and-fairness.md)

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

## Stateless parse (SPEC-094, v0.23)

Convert a PDF to Markdown **without** ingesting it — no document residue. `client.parse` maps to `POST /api/v1/parse` (multipart `file` + `options`); sync by default (≤ 15 pages / 20 MiB), use `async=True` for larger jobs (≤ 1000 pages) and poll `client.parse.job()` (`GET /api/v1/parse/jobs/{id}`).

```python
from pathlib import Path

from edgequake import EdgeQuake
from edgequake.resources.parse import ParseOptions

with EdgeQuake(base_url="http://localhost:8080") as client:
    # Sync parse (returns ParseResponse)
    res = client.parse.parse(
        Path("/tmp/paper.pdf"),
        options=ParseOptions(backend="vision", pages="1-5"),
        filename="paper.pdf",
    )
    print(res["markdown"][:200], res["page_count"], res["metrics"]["total_ms"])

    # List backends + ceilings
    backends = client.parse.backends()
    print(backends.default_backend, backends.limits["sync_max_pages"])

    # Async: submit, then poll (field is force_async; "async" is a Python keyword)
    accepted = client.parse.parse(
        Path("/tmp/big.pdf"),
        options=ParseOptions(force_async=True),
    )
    job_id = accepted["job_id"]
    status = client.parse.job(job_id)   # ParseJobStatusResponse
    if status.result:
        print(status.result["markdown"][:200])
```

Async client mirror: `await client.parse.parse(...)`, `await client.parse.backends()`, `await client.parse.job(job_id)`.

## Async

```python
from edgequake import AsyncEdgeQuake

async with AsyncEdgeQuake(base_url="http://localhost:8080") as client:
    health = await client.health()
    result = await client.query.execute(query="Hello")
```

## Document display fields (v0.23)

Document list/detail responses include `display_status` (badge key from `IngestionStatusMapper`) and `ui_phase` (`idle | running | stopping | terminal`). Prefer those over raw `status`/`stage` when rendering progress — see OpenAPI `DocumentSummary` / `DocumentDetailResponse` (SPEC-057 P4).

## LLM cache (server-side)

`EDGEQUAKE_LLM_CACHE=1` (default) caches keyword extraction + answers on the **server**; `EDGEQUAKE_KEYWORD_CACHE` / `EDGEQUAKE_QUERY_ANSWER_CACHE` override. No client change — repeat queries return cached answers transparently.

## See also

- [Quickstart](./quickstart.md)
- [Custom Clients](../../integrations/custom-clients.md) — raw HTTP fallback
- In-repo reference: `sdks/python/README.md`, `sdks/python/docs/API.md`
