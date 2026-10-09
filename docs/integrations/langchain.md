---
title: 'Integration: LangChain'
description: "Use EdgeQuake from LangChain."
---

# Integration: LangChain

> **Product: v0.23.0** · Contract: [`openapi.snapshot.json`](../../edgequake_webui/openapi/openapi.snapshot.json) · Spec ops: [Ingestion cancel & fairness](../ingestion-cancel-and-fairness.md)

Use EdgeQuake as a retriever (or full RAG backend) in [LangChain](https://langchain.com/) Python apps.

**Prefer the official SDK** (`pip install edgequake-sdk`) over hand-rolled HTTP. The examples below show LangChain wiring; transport uses `edgequake-sdk` types that match OpenAPI.

---

## Prerequisites

```bash
pip install edgequake-sdk langchain langchain-core langchain-openai
```

EdgeQuake running:

```bash
curl -s http://localhost:8080/health | jq .status   # "healthy"
```

Documents ingested with `display_status: completed` before querying.

---

## QueryResponse shape (SSOT)

`POST /api/v1/query` returns:

```json
{
  "answer": "…",
  "sources": [
    {
      "document_id": "…",
      "snippet": "…",
      "score": 0.91,
      "file_path": "report.pdf"
    }
  ],
  "mode": "hybrid",
  "stats": { "total_time_ms": 850, "retrieval_time_ms": 200 }
}
```

There are **no** top-level `chunks`, `entities`, or `relationships` arrays. Map `sources[].snippet` → LangChain `Document.page_content`.

---

## Retriever with `edgequake-sdk`

```python
"""EdgeQuake retriever for LangChain — uses official SDK."""

from typing import List

from edgequake import EdgeQuake
from edgequake.types.query import QueryRequest
from langchain_core.callbacks import CallbackManagerForRetrieverRun
from langchain_core.documents import Document
from langchain_core.retrievers import BaseRetriever


class EdgeQuakeRetriever(BaseRetriever):
    """Graph-RAG retriever backed by edgequake-sdk."""

    base_url: str = "http://localhost:8080"
    workspace_id: str = "default"
    query_mode: str = "hybrid"
    top_k: int = 10

    def _get_relevant_documents(
        self,
        query: str,
        *,
        run_manager: CallbackManagerForRetrieverRun,
    ) -> List[Document]:
        with EdgeQuake(
            base_url=self.base_url,
            workspace_id=self.workspace_id,
        ) as client:
            result = client.query.execute(
                QueryRequest(query=query, mode=self.query_mode, top_k=self.top_k)
            )

        documents: List[Document] = []
        for src in result.sources:
            content = src.snippet or ""
            if not content:
                continue
            documents.append(
                Document(
                    page_content=content,
                    metadata={
                        "document_id": src.document_id,
                        "score": src.score,
                        "file_path": src.file_path,
                        "reference_id": src.reference_id,
                        "workspace_id": self.workspace_id,
                        "query_mode": self.query_mode,
                    },
                )
            )
        return documents
```

### Usage

```python
retriever = EdgeQuakeRetriever(query_mode="hybrid", top_k=5)
docs = retriever.invoke("What are the key findings?")
for doc in docs:
    print(doc.page_content[:120], doc.metadata.get("score"))
```

Query modes: `local`, `global`, `naive`, `hybrid`, `mix` — see [Query Modes](/docs/deep-dives/query-modes/).

---

## Full answer via SDK (skip LangChain LLM)

When you want EdgeQuake to generate the answer (not just retrieve):

```python
from edgequake import EdgeQuake
from edgequake.types.query import QueryRequest

with EdgeQuake(base_url="http://localhost:8080", workspace_id="default") as client:
    result = client.query.execute(QueryRequest(query="What is the main topic?", mode="hybrid"))
    print(result.answer)
    for src in result.sources:
        print(f"  [{src.score:.2f}] {src.file_path}: {src.snippet[:80]}…")
```

Streaming: `client.query.stream(...)` — SSE events with `chunk`, `sources`, `stats`.

---

## RAG chain (retrieve + external LLM)

```python
from langchain_core.output_parsers import StrOutputParser
from langchain_core.prompts import ChatPromptTemplate
from langchain_core.runnables import RunnablePassthrough
from langchain_openai import ChatOpenAI

retriever = EdgeQuakeRetriever(top_k=8)
llm = ChatOpenAI(model="gpt-4.1-nano", temperature=0)

prompt = ChatPromptTemplate.from_template(
    "Answer using only this context:\n\n{context}\n\nQuestion: {question}\n\nAnswer:"
)

def format_docs(docs):
    return "\n\n".join(d.page_content for d in docs)

chain = (
    {"context": retriever | format_docs, "question": RunnablePassthrough()}
    | prompt
    | llm
    | StrOutputParser()
)

print(chain.invoke("Summarize the risk factors"))
```

---

## Upload documents (SDK)

```python
from pathlib import Path

from edgequake import EdgeQuake

with EdgeQuake(base_url="http://localhost:8080", workspace_id="default") as client:
    # Text
    doc = client.documents.upload(content="Marie Curie discovered radium.", title="Biography")
    # PDF — multipart via SDK
    pdf = client.pdf.upload(Path("/path/to/paper.pdf"), title="Paper", enable_vision=True)
    print(pdf.task_id)  # progress/cancel identity
```

Poll until list/detail shows `display_status == "completed"`.

---

## Error handling

```python
import httpx
from edgequake import EdgeQuake

try:
    with EdgeQuake(base_url="http://localhost:8080") as client:
        result = client.query.execute(query="test")
except httpx.ConnectError:
    ...
except httpx.HTTPStatusError as exc:
    if exc.response.status_code == 429:
        ...  # backoff
```

SDK retries 429/503 with exponential backoff by default.

---

## Best practices

1. **SDK first** — [Python SDK](/docs/sdks/python/) tracks OpenAPI; avoid duplicating DTOs.
2. **Check ingestion** — query only after `display_status: completed`.
3. **Pick mode by question** — `local` for entities, `global` for themes, `naive` for keyword speed.
4. **Use workspaces** — pass `workspace_id` on client construction.
5. **Streaming UX** — `client.query.stream` or `client.chat.stream` for live tokens.

---

## See also

- [Python SDK README](/docs/sdks/python/)
- [Custom Clients](/docs/integrations/custom-clients/) — minimal HTTP cookbook
- [Query Modes Deep Dive](/docs/deep-dives/query-modes/)
- [Open WebUI Integration](/docs/integrations/open-webui/)
