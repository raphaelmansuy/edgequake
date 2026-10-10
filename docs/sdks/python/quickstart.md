---
title: Python SDK quickstart
description: Five-minute path from install to a first RAG answer with the EdgeQuake Python SDK.
---

# Python SDK quickstart

This walkthrough uploads a short document, waits for indexing, and asks a question. You need a running EdgeQuake server (`make dev` or an equivalent) and Python 3.10+.

## 1. Install

```bash
pip install edgequake-sdk==0.3.0
```

## 2. Create a client

```python
from edgequake import EdgeQuake

client = EdgeQuake(base_url="http://localhost:8080")
# With auth: EdgeQuake(base_url="...", api_key="eq-...", workspace_id="...")
print(client.health().status)  # "healthy" when the server is ready
```

## 3. Upload and wait

```python
import time

up = client.documents.upload(
    content="Marie Curie won Nobel Prizes in Physics (1903) and Chemistry (1911).",
    title="Curie",
)
if up.task_id is None:
    raise SystemExit(f"Duplicate of {up.duplicate_of}; nothing new to index")

while True:
    task = client.tasks.get(up.task_id)
    if task.status in ("indexed", "failed", "cancelled"):
        break
    time.sleep(1)
if task.status != "indexed":
    raise SystemExit(f"Ingestion ended with status {task.status}")
```

## 4. Query

```python
res = client.query.execute(query="How many Nobel Prizes did Marie Curie win?", mode="mix")
print(res.answer)
for s in res.sources:
    print("-", s.snippet or s.id)
```

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
sequenceDiagram
    participant You
    participant SDK as Python SDK
    participant API as EdgeQuake REST API
    You->>SDK: documents.upload
    SDK->>API: POST /api/v1/documents
    API-->>SDK: document_id and task_id
    loop until a final status
        You->>SDK: tasks.get
        SDK->>API: GET /api/v1/tasks/{track_id}
    end
    You->>SDK: query.execute
    SDK->>API: POST /api/v1/query
    API-->>SDK: QueryResponse
    SDK-->>You: answer and sources
```

Upload, poll the task until it reaches a final status, then query. Read it top to bottom.

Next: [Python README](README.md) for the full resource list, and [Document upload](../../api-reference/document-upload-quick-reference.md).
