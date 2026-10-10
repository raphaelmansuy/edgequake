---
title: "Integration: LangChain"
description: Use EdgeQuake as a LangChain retriever or full RAG backend with the official Python SDK (edgequake-sdk).
---

# Integration: LangChain

Use EdgeQuake from [LangChain](https://langchain.com/) Python apps as a retriever, or let EdgeQuake generate the answer itself. Prefer `pip install edgequake-sdk` over hand-rolled HTTP.

## Prerequisites

```bash
pip install edgequake-sdk==0.3.0 langchain langchain-core langchain-openai
curl -s http://localhost:8080/health   # status healthy or degraded
```

Ingest documents and wait until `display_status` is `completed` before querying.

## Query response shape

`POST /api/v1/query` returns `answer`, `sources[]` (each with `snippet`, `score`, `document_id`, …), `mode` and `stats`. There are no top-level `chunks` / `entities` arrays. Map `sources[].snippet` to LangChain `Document.page_content`.

## Retriever

The Python SDK's `query.execute` takes keyword arguments (`query`, `mode`, …). It currently sends `top_k` / `rerank` in the JSON body; the server ignores unknown fields. Prefer `mode="mix"`.

```python
from typing import List

from edgequake import EdgeQuake
from langchain_core.callbacks import CallbackManagerForRetrieverRun
from langchain_core.documents import Document
from langchain_core.retrievers import BaseRetriever


class EdgeQuakeRetriever(BaseRetriever):
    """Graph-RAG retriever backed by edgequake-sdk."""

    base_url: str = "http://localhost:8080"
    workspace_id: str | None = None
    query_mode: str = "mix"

    def _get_relevant_documents(
        self,
        query: str,
        *,
        run_manager: CallbackManagerForRetrieverRun,
    ) -> List[Document]:
        client = EdgeQuake(
            base_url=self.base_url,
            workspace_id=self.workspace_id,
        )
        result = client.query.execute(query=query, mode=self.query_mode)

        documents: List[Document] = []
        for src in result.sources:
            content = getattr(src, "snippet", None) or ""
            if not content:
                continue
            documents.append(
                Document(
                    page_content=content,
                    metadata={
                        "document_id": getattr(src, "document_id", None),
                        "score": getattr(src, "score", None),
                        "file_path": getattr(src, "file_path", None),
                        "reference_id": getattr(src, "reference_id", None),
                        "query_mode": self.query_mode,
                    },
                )
            )
        return documents
```

```python
retriever = EdgeQuakeRetriever(query_mode="mix")
docs = retriever.invoke("What are the key findings?")
for doc in docs:
    print(doc.page_content[:120], doc.metadata.get("score"))
```

Modes: `naive`, `local`, `global`, `hybrid`, `mix`, `bypass`. See [Query modes](../deep-dives/query-modes.md).

## Full answer via EdgeQuake

```python
from edgequake import EdgeQuake

client = EdgeQuake(base_url="http://localhost:8080")
result = client.query.execute(query="What is the main topic?", mode="mix")
print(result.answer)
for src in result.sources:
    print(getattr(src, "score", None), getattr(src, "snippet", "")[:80])
```

Streaming: `client.query.stream(...)` yields SSE JSON events with `type` in `context`, `token`, `thinking`, `done`, `error` (not `chunk` / `sources` / `stats` event names).

## RAG chain (retrieve + external LLM)

```python
from langchain_core.output_parsers import StrOutputParser
from langchain_core.prompts import ChatPromptTemplate
from langchain_core.runnables import RunnablePassthrough
from langchain_openai import ChatOpenAI

retriever = EdgeQuakeRetriever(query_mode="mix")
llm = ChatOpenAI(model="gpt-5-nano", temperature=0)

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

## Upload with the SDK

```python
from pathlib import Path
from edgequake import EdgeQuake

client = EdgeQuake(base_url="http://localhost:8080")
doc = client.documents.upload(content="Marie Curie discovered radium.", title="Biography")
pdf = client.pdf.upload(Path("/path/to/paper.pdf"), title="Paper")
print(doc.task_id, pdf.task_id)
```

Poll `client.tasks.get(task_id)` until `status` is `indexed` (or check document `display_status == "completed"`).

Related: [Python SDK](../sdks/python/README.md), [REST query](../api-reference/rest-api.md#query).
