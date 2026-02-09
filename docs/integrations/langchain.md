# Integration: LangChain

> **Using EdgeQuake as a Retriever in LangChain Applications**

This guide shows how to integrate EdgeQuake with [LangChain](https://langchain.com/) Python applications for building custom RAG pipelines.

---

## Overview

EdgeQuake provides a REST API that can be wrapped as a LangChain `BaseRetriever`, enabling you to:

- Use EdgeQuake's Graph-RAG in LangChain chains
- Combine with other retrievers (ensemble retrieval)
- Build custom RAG applications with LangChain's tooling

```
┌─────────────────────────────────────────────────────────────────┐
│                 LANGCHAIN + EDGEQUAKE                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────┐  │
│  │   LangChain     │    │ EdgeQuakeRet.   │    │ EdgeQuake   │  │
│  │   Application   │───▶│  (Custom)       │───▶│   API       │  │
│  │                 │    │                 │    │             │  │
│  │ • Chains        │    │ • _get_relevant │    │ • /query    │  │
│  │ • Agents        │◀───│   _documents()  │◀───│ • /chat     │  │
│  │ • Tools         │    │                 │    │             │  │
│  └─────────────────┘    └─────────────────┘    └─────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Prerequisites

```bash
pip install langchain langchain-core requests
```

Ensure EdgeQuake is running:

```bash
curl http://localhost:8080/health
# {"status":"healthy","database_connected":true}
```

---

## Custom EdgeQuake Retriever

Create a custom retriever that wraps EdgeQuake's API:

```python
"""EdgeQuake Retriever for LangChain."""

from typing import List, Optional
import requests
from langchain_core.documents import Document
from langchain_core.retrievers import BaseRetriever
from langchain_core.callbacks import CallbackManagerForRetrieverRun


class EdgeQuakeRetriever(BaseRetriever):
    """Retriever that uses EdgeQuake's Graph-RAG API.

    Example:
        retriever = EdgeQuakeRetriever(
            base_url="http://localhost:8080",
            workspace_id="default",
            query_mode="hybrid"
        )
        docs = retriever.invoke("What is the main topic?")
    """

    base_url: str = "http://localhost:8080"
    workspace_id: str = "default"
    query_mode: str = "hybrid"  # local, global, naive, hybrid, mix
    top_k: int = 10
    timeout: int = 60

    def _get_relevant_documents(
        self,
        query: str,
        *,
        run_manager: CallbackManagerForRetrieverRun,
    ) -> List[Document]:
        """Retrieve documents from EdgeQuake.

        Args:
            query: The search query.
            run_manager: Callback manager for the retrieval run.

        Returns:
            List of LangChain Document objects.
        """
        # Make API request to EdgeQuake
        response = requests.post(
            f"{self.base_url}/api/v1/query",
            json={
                "query": query,
                "mode": self.query_mode,
                "top_k": self.top_k,
            },
            headers={
                "Content-Type": "application/json",
                "X-Workspace-ID": self.workspace_id,
            },
            timeout=self.timeout,
        )
        response.raise_for_status()
        result = response.json()

        # Convert to LangChain Documents
        documents = []

        # Extract chunks from response
        chunks = result.get("chunks", [])
        for chunk in chunks:
            doc = Document(
                page_content=chunk.get("content", ""),
                metadata={
                    "source": chunk.get("document_id", ""),
                    "chunk_id": chunk.get("chunk_id", ""),
                    "score": chunk.get("score", 0.0),
                    "workspace_id": self.workspace_id,
                    "query_mode": self.query_mode,
                }
            )
            documents.append(doc)

        # Also include entities if available
        entities = result.get("entities", [])
        for entity in entities:
            doc = Document(
                page_content=f"Entity: {entity.get('name', '')} - {entity.get('description', '')}",
                metadata={
                    "type": "entity",
                    "entity_type": entity.get("type", ""),
                    "entity_name": entity.get("name", ""),
                }
            )
            documents.append(doc)

        return documents


class EdgeQuakeStreamRetriever(BaseRetriever):
    """Retriever with streaming support for real-time responses."""

    base_url: str = "http://localhost:8080"
    workspace_id: str = "default"
    query_mode: str = "hybrid"

    def _get_relevant_documents(
        self,
        query: str,
        *,
        run_manager: CallbackManagerForRetrieverRun,
    ) -> List[Document]:
        """Stream documents from EdgeQuake.

        Uses Server-Sent Events for real-time retrieval.
        """
        documents = []

        with requests.post(
            f"{self.base_url}/api/v1/chat/stream",
            json={
                "message": query,
                "workspace_id": self.workspace_id,
                "mode": self.query_mode,
            },
            stream=True,
            timeout=60,
        ) as response:
            response.raise_for_status()

            for line in response.iter_lines():
                if line:
                    # Parse SSE data
                    if line.startswith(b"data: "):
                        import json
                        data = json.loads(line[6:])

                        # Check for context in the stream
                        if "context" in data:
                            for chunk in data["context"]:
                                doc = Document(
                                    page_content=chunk.get("content", ""),
                                    metadata={
                                        "source": chunk.get("document_id", ""),
                                        "chunk_id": chunk.get("chunk_id", ""),
                                    }
                                )
                                documents.append(doc)

        return documents
```

---

## Basic Usage

### Simple Retrieval

```python
from edgequake_retriever import EdgeQuakeRetriever

# Create retriever
retriever = EdgeQuakeRetriever(
    base_url="http://localhost:8080",
    workspace_id="default",
    query_mode="hybrid",
    top_k=5,
)

# Retrieve documents
docs = retriever.invoke("What are the key concepts?")

for doc in docs:
    print(f"Content: {doc.page_content[:100]}...")
    print(f"Source: {doc.metadata.get('source')}")
    print(f"Score: {doc.metadata.get('score')}")
    print("---")
```

### With Different Query Modes

```python
# Local mode - entity-focused
local_retriever = EdgeQuakeRetriever(query_mode="local")
local_docs = local_retriever.invoke("Who is John Smith?")

# Global mode - relationship-focused
global_retriever = EdgeQuakeRetriever(query_mode="global")
global_docs = global_retriever.invoke("What are the themes?")

# Naive mode - vector search only (fastest)
naive_retriever = EdgeQuakeRetriever(query_mode="naive")
naive_docs = naive_retriever.invoke("climate change impacts")
```

---

## RAG Chain with EdgeQuake

Build a complete RAG chain using EdgeQuake as the retriever:

```python
from langchain_openai import ChatOpenAI
from langchain_core.prompts import ChatPromptTemplate
from langchain_core.output_parsers import StrOutputParser
from langchain_core.runnables import RunnablePassthrough

from edgequake_retriever import EdgeQuakeRetriever

# Components
retriever = EdgeQuakeRetriever(
    base_url="http://localhost:8080",
    workspace_id="default",
    query_mode="hybrid",
)

llm = ChatOpenAI(model="gpt-4o-mini", temperature=0)

# Prompt template
template = """Answer the question based on the following context:

Context:
{context}

Question: {question}

Answer:"""

prompt = ChatPromptTemplate.from_template(template)

# Helper function to format documents
def format_docs(docs):
    return "\n\n".join(doc.page_content for doc in docs)

# Build the chain
rag_chain = (
    {"context": retriever | format_docs, "question": RunnablePassthrough()}
    | prompt
    | llm
    | StrOutputParser()
)

# Use the chain
response = rag_chain.invoke("What is the main topic of the documents?")
print(response)
```

---

## Ensemble Retriever

Combine EdgeQuake with other retrievers for hybrid search:

```python
from langchain.retrievers import EnsembleRetriever
from langchain_community.vectorstores import FAISS
from langchain_openai import OpenAIEmbeddings

from edgequake_retriever import EdgeQuakeRetriever

# EdgeQuake retriever (Graph-RAG)
edgequake = EdgeQuakeRetriever(
    base_url="http://localhost:8080",
    query_mode="hybrid",
)

# Local FAISS retriever (fallback)
embeddings = OpenAIEmbeddings()
local_texts = ["Local document 1", "Local document 2"]
faiss_store = FAISS.from_texts(local_texts, embeddings)
faiss_retriever = faiss_store.as_retriever()

# Ensemble with weights
ensemble_retriever = EnsembleRetriever(
    retrievers=[edgequake, faiss_retriever],
    weights=[0.7, 0.3],  # Prefer EdgeQuake
)

# Use ensemble
docs = ensemble_retriever.invoke("What are the key topics?")
```

---

## Agent with EdgeQuake Tool

Create a LangChain agent that can query EdgeQuake:

```python
from langchain.agents import AgentExecutor, create_openai_tools_agent
from langchain_openai import ChatOpenAI
from langchain_core.tools import Tool
from langchain_core.prompts import ChatPromptTemplate, MessagesPlaceholder

from edgequake_retriever import EdgeQuakeRetriever

# Create the tool
retriever = EdgeQuakeRetriever(
    base_url="http://localhost:8080",
    query_mode="hybrid",
)

def search_knowledge_base(query: str) -> str:
    """Search the knowledge base for relevant information."""
    docs = retriever.invoke(query)
    if not docs:
        return "No relevant documents found."
    return "\n\n".join(doc.page_content for doc in docs[:3])

knowledge_tool = Tool(
    name="search_knowledge_base",
    description="Search the knowledge base for information about documents and entities. Use this when you need specific information.",
    func=search_knowledge_base,
)

# Create agent
llm = ChatOpenAI(model="gpt-4o-mini", temperature=0)

prompt = ChatPromptTemplate.from_messages([
    ("system", "You are a helpful assistant with access to a knowledge base."),
    ("user", "{input}"),
    MessagesPlaceholder(variable_name="agent_scratchpad"),
])

agent = create_openai_tools_agent(llm, [knowledge_tool], prompt)
agent_executor = AgentExecutor(agent=agent, tools=[knowledge_tool], verbose=True)

# Use the agent
response = agent_executor.invoke({
    "input": "What companies are mentioned in the documents?"
})
print(response["output"])
```

---

## LangGraph Integration

Use EdgeQuake in a LangGraph workflow:

```python
from typing import Annotated, Sequence, TypedDict
from langgraph.graph import StateGraph, END
from langchain_core.messages import BaseMessage, HumanMessage, AIMessage

from edgequake_retriever import EdgeQuakeRetriever

# State definition
class GraphState(TypedDict):
    messages: Annotated[Sequence[BaseMessage], "chat history"]
    context: str
    answer: str

# Nodes
def retrieve(state: GraphState) -> GraphState:
    """Retrieve relevant documents."""
    retriever = EdgeQuakeRetriever()
    last_message = state["messages"][-1].content

    docs = retriever.invoke(last_message)
    context = "\n\n".join(doc.page_content for doc in docs)

    return {"context": context, **state}

def generate(state: GraphState) -> GraphState:
    """Generate response using context."""
    from langchain_openai import ChatOpenAI

    llm = ChatOpenAI(model="gpt-4o-mini")

    prompt = f"""Based on the following context, answer the question.

Context:
{state["context"]}

Question: {state["messages"][-1].content}

Answer:"""

    response = llm.invoke([HumanMessage(content=prompt)])

    return {"answer": response.content, **state}

# Build graph
workflow = StateGraph(GraphState)
workflow.add_node("retrieve", retrieve)
workflow.add_node("generate", generate)
workflow.add_edge("retrieve", "generate")
workflow.add_edge("generate", END)
workflow.set_entry_point("retrieve")

app = workflow.compile()

# Use the graph
result = app.invoke({
    "messages": [HumanMessage(content="What are the main topics?")],
    "context": "",
    "answer": "",
})
print(result["answer"])
```

---

## Configuration Options

### EdgeQuakeRetriever Parameters

| Parameter      | Type | Default                 | Description                                |
| -------------- | ---- | ----------------------- | ------------------------------------------ |
| `base_url`     | str  | "http://localhost:8080" | EdgeQuake API URL                          |
| `workspace_id` | str  | "default"               | Workspace for document scope               |
| `query_mode`   | str  | "hybrid"                | Query mode (local/global/naive/hybrid/mix) |
| `top_k`        | int  | 10                      | Maximum documents to retrieve              |
| `timeout`      | int  | 60                      | Request timeout in seconds                 |

### Query Mode Selection

| Mode     | Best For                     | Performance |
| -------- | ---------------------------- | ----------- |
| `local`  | Entity-specific questions    | Medium      |
| `global` | Theme/relationship questions | Slower      |
| `naive`  | Quick keyword search         | Fastest     |
| `hybrid` | General queries              | Medium      |
| `mix`    | Adaptive blending            | Medium      |

---

## Error Handling

```python
from langchain_core.documents import Document

class RobustEdgeQuakeRetriever(EdgeQuakeRetriever):
    """Retriever with error handling and fallbacks."""

    fallback_message: str = "Unable to retrieve documents at this time."

    def _get_relevant_documents(
        self,
        query: str,
        *,
        run_manager,
    ) -> List[Document]:
        try:
            return super()._get_relevant_documents(query, run_manager=run_manager)
        except requests.exceptions.ConnectionError:
            # EdgeQuake is not available
            return [Document(
                page_content=self.fallback_message,
                metadata={"error": "connection_error"}
            )]
        except requests.exceptions.Timeout:
            # Request timed out
            return [Document(
                page_content=self.fallback_message,
                metadata={"error": "timeout"}
            )]
        except requests.exceptions.HTTPError as e:
            # HTTP error from EdgeQuake
            return [Document(
                page_content=f"API error: {e}",
                metadata={"error": "http_error"}
            )]
```

---

## Best Practices

1. **Choose the Right Query Mode**
   - Use `naive` for simple keyword searches
   - Use `local` for entity-focused questions
   - Use `global` for relationship/theme questions
   - Use `hybrid` as the default

2. **Handle Rate Limits**
   - EdgeQuake may have rate limits configured
   - Implement exponential backoff for retries

3. **Cache Results**
   - Use LangChain's caching for repeated queries
   - Reduces API calls and latency

4. **Monitor Performance**
   - Check EdgeQuake's cost dashboard
   - Use workspace isolation for different use cases

5. **Batch Queries**
   - Group similar queries when possible
   - Reduces overhead and improves throughput

---

## See Also

- [REST API Reference](../api-reference/rest-api.md) - Full API documentation
- [Query Modes Deep Dive](../deep-dives/query-modes.md) - Understanding query modes
- [Open WebUI Integration](./open-webui.md) - Alternative UI integration
