# Integration: Custom Clients

> **Building Custom Applications with EdgeQuake's API**

This guide shows how to build custom client applications that integrate with EdgeQuake using REST API, Server-Sent Events (SSE), and WebSockets.

---

## Overview

EdgeQuake provides multiple integration patterns for custom applications:

```
┌─────────────────────────────────────────────────────────────────┐
│                 CLIENT INTEGRATION OPTIONS                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌────────────┐   ┌────────────┐   ┌────────────┐               │
│  │   REST     │   │    SSE     │   │ Ollama API │               │
│  │   API      │   │ Streaming  │   │ Emulation  │               │
│  │            │   │            │   │            │               │
│  │ • Query    │   │ • Chat     │   │ • /api/    │               │
│  │ • Upload   │   │ • Real-    │   │   chat     │               │
│  │ • Manage   │   │   time     │   │ • /api/    │               │
│  │            │   │            │   │   generate │               │
│  └─────┬──────┘   └─────┬──────┘   └─────┬──────┘               │
│        │                │                │                      │
│        └────────────────┼────────────────┘                      │
│                         │                                       │
│                         ▼                                       │
│           ┌─────────────────────────────┐                       │
│           │        EdgeQuake            │                       │
│           │     http://localhost:8080   │                       │
│           └─────────────────────────────┘                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Python Client

### Installation

```bash
pip install requests sseclient-py
```

### Complete Client Class

```python
"""EdgeQuake Python Client."""

import json
from typing import Generator, Optional, Dict, Any, List
from dataclasses import dataclass
import requests
import sseclient


@dataclass
class Document:
    """Uploaded document."""
    id: str
    name: str
    status: str
    chunks: int
    created_at: str


@dataclass
class QueryResult:
    """Query result with answer and sources."""
    answer: str
    chunks: List[Dict[str, Any]]
    entities: List[Dict[str, Any]]
    relationships: List[Dict[str, Any]]
    cost: Optional[float] = None


class EdgeQuakeClient:
    """Client for EdgeQuake Graph-RAG API.

    Example:
        client = EdgeQuakeClient("http://localhost:8080")

        # Upload document
        doc = client.upload_file("report.pdf")

        # Query
        result = client.query("What is the main topic?")
        print(result.answer)

        # Stream chat
        for chunk in client.chat_stream("Tell me more"):
            print(chunk, end="", flush=True)
    """

    def __init__(
        self,
        base_url: str = "http://localhost:8080",
        workspace_id: str = "default",
        timeout: int = 120,
    ):
        self.base_url = base_url.rstrip("/")
        self.workspace_id = workspace_id
        self.timeout = timeout
        self.session = requests.Session()
        self.session.headers.update({
            "X-Workspace-ID": workspace_id,
        })

    # Health & Status

    def health(self) -> Dict[str, Any]:
        """Check server health."""
        response = self.session.get(
            f"{self.base_url}/health",
            timeout=10,
        )
        response.raise_for_status()
        return response.json()

    def is_healthy(self) -> bool:
        """Check if server is healthy."""
        try:
            health = self.health()
            return health.get("status") == "healthy"
        except Exception:
            return False

    # Document Management

    def upload_file(
        self,
        file_path: str,
        wait_for_processing: bool = True,
    ) -> Document:
        """Upload a file for processing.

        Args:
            file_path: Path to the file to upload.
            wait_for_processing: If True, wait until processing is complete.

        Returns:
            Document object with upload details.
        """
        with open(file_path, "rb") as f:
            response = self.session.post(
                f"{self.base_url}/api/v1/documents/upload",
                files={"file": f},
                timeout=self.timeout,
            )
        response.raise_for_status()
        data = response.json()

        if wait_for_processing and data.get("status") == "processing":
            import time
            doc_id = data["id"]
            while True:
                doc = self.get_document(doc_id)
                if doc.status == "completed":
                    return doc
                elif doc.status == "failed":
                    raise RuntimeError(f"Document processing failed: {doc_id}")
                time.sleep(1)

        return Document(
            id=data["id"],
            name=data.get("name", ""),
            status=data.get("status", "processing"),
            chunks=data.get("chunks", 0),
            created_at=data.get("created_at", ""),
        )

    def upload_text(self, text: str, name: str = "text.txt") -> Document:
        """Upload text content directly.

        Args:
            text: Text content to upload.
            name: Name for the document.

        Returns:
            Document object.
        """
        response = self.session.post(
            f"{self.base_url}/api/v1/documents",
            json={
                "content": text,
                "name": name,
            },
            timeout=self.timeout,
        )
        response.raise_for_status()
        data = response.json()

        return Document(
            id=data["id"],
            name=data.get("name", name),
            status=data.get("status", "completed"),
            chunks=data.get("chunks", 0),
            created_at=data.get("created_at", ""),
        )

    def get_document(self, document_id: str) -> Document:
        """Get document by ID."""
        response = self.session.get(
            f"{self.base_url}/api/v1/documents/{document_id}",
            timeout=self.timeout,
        )
        response.raise_for_status()
        data = response.json()

        return Document(
            id=data["id"],
            name=data.get("name", ""),
            status=data.get("status", ""),
            chunks=data.get("chunks", 0),
            created_at=data.get("created_at", ""),
        )

    def list_documents(self) -> List[Document]:
        """List all documents in the workspace."""
        response = self.session.get(
            f"{self.base_url}/api/v1/documents",
            timeout=self.timeout,
        )
        response.raise_for_status()
        data = response.json()

        return [
            Document(
                id=doc["id"],
                name=doc.get("name", ""),
                status=doc.get("status", ""),
                chunks=doc.get("chunks", 0),
                created_at=doc.get("created_at", ""),
            )
            for doc in data.get("documents", [])
        ]

    def delete_document(self, document_id: str) -> bool:
        """Delete a document."""
        response = self.session.delete(
            f"{self.base_url}/api/v1/documents/{document_id}",
            timeout=self.timeout,
        )
        return response.status_code == 204

    # Query

    def query(
        self,
        query: str,
        mode: str = "hybrid",
        top_k: int = 10,
    ) -> QueryResult:
        """Execute a Graph-RAG query.

        Args:
            query: The question to answer.
            mode: Query mode (local, global, naive, hybrid, mix).
            top_k: Maximum number of chunks to retrieve.

        Returns:
            QueryResult with answer and sources.
        """
        response = self.session.post(
            f"{self.base_url}/api/v1/query",
            json={
                "query": query,
                "mode": mode,
                "top_k": top_k,
            },
            timeout=self.timeout,
        )
        response.raise_for_status()
        data = response.json()

        return QueryResult(
            answer=data.get("answer", ""),
            chunks=data.get("chunks", []),
            entities=data.get("entities", []),
            relationships=data.get("relationships", []),
            cost=data.get("cost"),
        )

    # Chat

    def chat(
        self,
        message: str,
        conversation_id: Optional[str] = None,
        mode: str = "hybrid",
    ) -> Dict[str, Any]:
        """Send a chat message and get a response.

        Args:
            message: The message to send.
            conversation_id: Optional conversation ID for context.
            mode: Query mode.

        Returns:
            Response with answer and context.
        """
        response = self.session.post(
            f"{self.base_url}/api/v1/chat",
            json={
                "message": message,
                "conversation_id": conversation_id,
                "mode": mode,
            },
            timeout=self.timeout,
        )
        response.raise_for_status()
        return response.json()

    def chat_stream(
        self,
        message: str,
        conversation_id: Optional[str] = None,
        mode: str = "hybrid",
    ) -> Generator[str, None, None]:
        """Stream a chat response.

        Args:
            message: The message to send.
            conversation_id: Optional conversation ID.
            mode: Query mode.

        Yields:
            Text chunks as they arrive.
        """
        response = self.session.post(
            f"{self.base_url}/api/v1/chat/stream",
            json={
                "message": message,
                "conversation_id": conversation_id,
                "mode": mode,
            },
            stream=True,
            timeout=self.timeout,
        )
        response.raise_for_status()

        client = sseclient.SSEClient(response)
        for event in client.events():
            if event.data:
                try:
                    data = json.loads(event.data)
                    if "content" in data:
                        yield data["content"]
                    elif "done" in data and data["done"]:
                        break
                except json.JSONDecodeError:
                    continue

    # Graph

    def get_graph(
        self,
        limit: int = 100,
    ) -> Dict[str, Any]:
        """Get the knowledge graph.

        Args:
            limit: Maximum number of nodes/edges to return.

        Returns:
            Graph data with nodes and edges.
        """
        response = self.session.get(
            f"{self.base_url}/api/v1/graph",
            params={"limit": limit},
            timeout=self.timeout,
        )
        response.raise_for_status()
        return response.json()

    def get_entity(self, entity_name: str) -> Dict[str, Any]:
        """Get a specific entity by name."""
        response = self.session.get(
            f"{self.base_url}/api/v1/graph/entities/{entity_name}",
            timeout=self.timeout,
        )
        response.raise_for_status()
        return response.json()

    # Workspaces

    def set_workspace(self, workspace_id: str) -> None:
        """Switch to a different workspace."""
        self.workspace_id = workspace_id
        self.session.headers.update({
            "X-Workspace-ID": workspace_id,
        })

    def list_workspaces(self) -> List[Dict[str, Any]]:
        """List all workspaces."""
        response = self.session.get(
            f"{self.base_url}/api/v1/workspaces",
            timeout=self.timeout,
        )
        response.raise_for_status()
        return response.json().get("workspaces", [])

    def create_workspace(self, workspace_id: str, name: str = "") -> Dict[str, Any]:
        """Create a new workspace."""
        response = self.session.post(
            f"{self.base_url}/api/v1/workspaces",
            json={
                "id": workspace_id,
                "name": name or workspace_id,
            },
            timeout=self.timeout,
        )
        response.raise_for_status()
        return response.json()


# Example usage
if __name__ == "__main__":
    client = EdgeQuakeClient()

    # Check health
    print(f"Healthy: {client.is_healthy()}")

    # Upload a document
    doc = client.upload_text("EdgeQuake is a Graph-RAG system.")
    print(f"Uploaded: {doc.id}")

    # Query
    result = client.query("What is EdgeQuake?")
    print(f"Answer: {result.answer}")

    # Stream chat
    print("\nStreaming: ", end="")
    for chunk in client.chat_stream("Tell me more about EdgeQuake"):
        print(chunk, end="", flush=True)
    print()
```

---

## TypeScript/JavaScript Client

### Installation

```bash
npm install axios eventsource
```

### Complete Client Class

```typescript
/**
 * EdgeQuake TypeScript Client
 */

import axios, { AxiosInstance, AxiosResponse } from "axios";

interface Document {
  id: string;
  name: string;
  status: string;
  chunks: number;
  created_at: string;
}

interface QueryResult {
  answer: string;
  chunks: Array<{
    content: string;
    document_id: string;
    chunk_id: string;
    score: number;
  }>;
  entities: Array<{
    name: string;
    type: string;
    description: string;
  }>;
  relationships: Array<{
    source: string;
    target: string;
    type: string;
  }>;
  cost?: number;
}

interface GraphData {
  nodes: Array<{
    id: string;
    label: string;
    type: string;
  }>;
  edges: Array<{
    source: string;
    target: string;
    label: string;
  }>;
}

export class EdgeQuakeClient {
  private client: AxiosInstance;
  private workspaceId: string;

  constructor(
    baseUrl: string = "http://localhost:8080",
    workspaceId: string = "default",
  ) {
    this.workspaceId = workspaceId;
    this.client = axios.create({
      baseURL: baseUrl,
      timeout: 120000,
      headers: {
        "Content-Type": "application/json",
        "X-Workspace-ID": workspaceId,
      },
    });
  }

  // Health

  async health(): Promise<{ status: string; database_connected: boolean }> {
    const response = await this.client.get("/health");
    return response.data;
  }

  async isHealthy(): Promise<boolean> {
    try {
      const health = await this.health();
      return health.status === "healthy";
    } catch {
      return false;
    }
  }

  // Documents

  async uploadFile(file: File | Blob, filename?: string): Promise<Document> {
    const formData = new FormData();
    formData.append("file", file, filename);

    const response = await this.client.post(
      "/api/v1/documents/upload",
      formData,
      {
        headers: {
          "Content-Type": "multipart/form-data",
        },
      },
    );

    return response.data;
  }

  async uploadText(text: string, name: string = "text.txt"): Promise<Document> {
    const response = await this.client.post("/api/v1/documents", {
      content: text,
      name,
    });
    return response.data;
  }

  async getDocument(documentId: string): Promise<Document> {
    const response = await this.client.get(`/api/v1/documents/${documentId}`);
    return response.data;
  }

  async listDocuments(): Promise<Document[]> {
    const response = await this.client.get("/api/v1/documents");
    return response.data.documents || [];
  }

  async deleteDocument(documentId: string): Promise<boolean> {
    const response = await this.client.delete(
      `/api/v1/documents/${documentId}`,
    );
    return response.status === 204;
  }

  // Query

  async query(
    query: string,
    mode: "local" | "global" | "naive" | "hybrid" | "mix" = "hybrid",
    topK: number = 10,
  ): Promise<QueryResult> {
    const response = await this.client.post("/api/v1/query", {
      query,
      mode,
      top_k: topK,
    });
    return response.data;
  }

  // Chat

  async chat(
    message: string,
    conversationId?: string,
    mode: string = "hybrid",
  ): Promise<{ answer: string; context: any[] }> {
    const response = await this.client.post("/api/v1/chat", {
      message,
      conversation_id: conversationId,
      mode,
    });
    return response.data;
  }

  chatStream(
    message: string,
    conversationId?: string,
    mode: string = "hybrid",
    onChunk: (chunk: string) => void = () => {},
    onDone: () => void = () => {},
    onError: (error: Error) => void = () => {},
  ): AbortController {
    const controller = new AbortController();

    fetch(`${this.client.defaults.baseURL}/api/v1/chat/stream`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Workspace-ID": this.workspaceId,
      },
      body: JSON.stringify({
        message,
        conversation_id: conversationId,
        mode,
      }),
      signal: controller.signal,
    })
      .then(async (response) => {
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`);
        }

        const reader = response.body?.getReader();
        const decoder = new TextDecoder();

        if (!reader) {
          throw new Error("No response body");
        }

        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          const text = decoder.decode(value);
          const lines = text.split("\n");

          for (const line of lines) {
            if (line.startsWith("data: ")) {
              try {
                const data = JSON.parse(line.slice(6));
                if (data.content) {
                  onChunk(data.content);
                }
                if (data.done) {
                  onDone();
                  return;
                }
              } catch {
                // Skip invalid JSON
              }
            }
          }
        }

        onDone();
      })
      .catch((error) => {
        if (error.name !== "AbortError") {
          onError(error);
        }
      });

    return controller;
  }

  // Graph

  async getGraph(limit: number = 100): Promise<GraphData> {
    const response = await this.client.get("/api/v1/graph", {
      params: { limit },
    });
    return response.data;
  }

  async getEntity(entityName: string): Promise<any> {
    const response = await this.client.get(
      `/api/v1/graph/entities/${encodeURIComponent(entityName)}`,
    );
    return response.data;
  }

  // Workspaces

  setWorkspace(workspaceId: string): void {
    this.workspaceId = workspaceId;
    this.client.defaults.headers["X-Workspace-ID"] = workspaceId;
  }

  async listWorkspaces(): Promise<Array<{ id: string; name: string }>> {
    const response = await this.client.get("/api/v1/workspaces");
    return response.data.workspaces || [];
  }

  async createWorkspace(
    workspaceId: string,
    name?: string,
  ): Promise<{ id: string; name: string }> {
    const response = await this.client.post("/api/v1/workspaces", {
      id: workspaceId,
      name: name || workspaceId,
    });
    return response.data;
  }
}

// React hook example
export function useEdgeQuake(baseUrl?: string, workspaceId?: string) {
  const client = new EdgeQuakeClient(baseUrl, workspaceId);

  return {
    query: client.query.bind(client),
    chat: client.chat.bind(client),
    chatStream: client.chatStream.bind(client),
    uploadFile: client.uploadFile.bind(client),
    uploadText: client.uploadText.bind(client),
    listDocuments: client.listDocuments.bind(client),
    getGraph: client.getGraph.bind(client),
  };
}
```

---

## cURL Examples

### Health Check

```bash
curl http://localhost:8080/health
```

### Upload Document

```bash
# File upload
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -H "X-Workspace-ID: default" \
  -F "file=@document.pdf"

# Text upload
curl -X POST http://localhost:8080/api/v1/documents \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: default" \
  -d '{"content": "Hello world", "name": "hello.txt"}'
```

### Query

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: default" \
  -d '{
    "query": "What are the main topics?",
    "mode": "hybrid",
    "top_k": 10
  }'
```

### Chat with Streaming

```bash
curl -X POST http://localhost:8080/api/v1/chat/stream \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: default" \
  -N \
  -d '{
    "message": "Tell me about the documents",
    "mode": "hybrid"
  }'
```

### Get Graph

```bash
curl http://localhost:8080/api/v1/graph?limit=50 \
  -H "X-Workspace-ID: default"
```

---

## Rust Client

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct QueryRequest {
    pub query: String,
    pub mode: String,
    pub top_k: usize,
}

#[derive(Debug, Deserialize)]
pub struct QueryResponse {
    pub answer: String,
    pub chunks: Vec<Chunk>,
    pub entities: Vec<Entity>,
}

#[derive(Debug, Deserialize)]
pub struct Chunk {
    pub content: String,
    pub document_id: String,
    pub score: f64,
}

#[derive(Debug, Deserialize)]
pub struct Entity {
    pub name: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    pub description: String,
}

pub struct EdgeQuakeClient {
    client: Client,
    base_url: String,
    workspace_id: String,
}

impl EdgeQuakeClient {
    pub fn new(base_url: &str, workspace_id: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
            workspace_id: workspace_id.to_string(),
        }
    }

    pub async fn query(&self, query: &str, mode: &str) -> Result<QueryResponse, reqwest::Error> {
        let request = QueryRequest {
            query: query.to_string(),
            mode: mode.to_string(),
            top_k: 10,
        };

        self.client
            .post(format!("{}/api/v1/query", self.base_url))
            .header("X-Workspace-ID", &self.workspace_id)
            .json(&request)
            .send()
            .await?
            .json()
            .await
    }

    pub async fn is_healthy(&self) -> bool {
        match self
            .client
            .get(format!("{}/health", self.base_url))
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
}

#[tokio::main]
async fn main() {
    let client = EdgeQuakeClient::new("http://localhost:8080", "default");

    if client.is_healthy().await {
        println!("EdgeQuake is healthy");

        match client.query("What is EdgeQuake?", "hybrid").await {
            Ok(response) => {
                println!("Answer: {}", response.answer);
                println!("Found {} chunks", response.chunks.len());
            }
            Err(e) => eprintln!("Query failed: {}", e),
        }
    } else {
        eprintln!("EdgeQuake is not available");
    }
}
```

---

## Go Client

```go
package edgequake

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

type Client struct {
	BaseURL     string
	WorkspaceID string
	HTTPClient  *http.Client
}

type QueryRequest struct {
	Query string `json:"query"`
	Mode  string `json:"mode"`
	TopK  int    `json:"top_k"`
}

type QueryResponse struct {
	Answer   string   `json:"answer"`
	Chunks   []Chunk  `json:"chunks"`
	Entities []Entity `json:"entities"`
}

type Chunk struct {
	Content    string  `json:"content"`
	DocumentID string  `json:"document_id"`
	Score      float64 `json:"score"`
}

type Entity struct {
	Name        string `json:"name"`
	Type        string `json:"type"`
	Description string `json:"description"`
}

func NewClient(baseURL, workspaceID string) *Client {
	return &Client{
		BaseURL:     baseURL,
		WorkspaceID: workspaceID,
		HTTPClient: &http.Client{
			Timeout: 120 * time.Second,
		},
	}
}

func (c *Client) Query(query, mode string, topK int) (*QueryResponse, error) {
	reqBody := QueryRequest{
		Query: query,
		Mode:  mode,
		TopK:  topK,
	}

	bodyBytes, err := json.Marshal(reqBody)
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequest("POST", c.BaseURL+"/api/v1/query", bytes.NewBuffer(bodyBytes))
	if err != nil {
		return nil, err
	}

	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-Workspace-ID", c.WorkspaceID)

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("query failed: %s", string(body))
	}

	var result QueryResponse
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, err
	}

	return &result, nil
}

func (c *Client) IsHealthy() bool {
	resp, err := c.HTTPClient.Get(c.BaseURL + "/health")
	if err != nil {
		return false
	}
	defer resp.Body.Close()

	return resp.StatusCode == http.StatusOK
}

func main() {
	client := NewClient("http://localhost:8080", "default")

	if client.IsHealthy() {
		fmt.Println("EdgeQuake is healthy")

		result, err := client.Query("What is EdgeQuake?", "hybrid", 10)
		if err != nil {
			fmt.Printf("Query failed: %v\n", err)
			return
		}

		fmt.Printf("Answer: %s\n", result.Answer)
		fmt.Printf("Found %d chunks\n", len(result.Chunks))
	}
}
```

---

## Authentication

If authentication is enabled, include the API key in requests:

```bash
# Header-based
curl -H "Authorization: Bearer your-api-key" ...

# Query parameter
curl "http://localhost:8080/api/v1/query?api_key=your-api-key" ...
```

---

## Error Handling

### HTTP Status Codes

| Code | Meaning                                |
| ---- | -------------------------------------- |
| 200  | Success                                |
| 201  | Created (document upload)              |
| 204  | No content (delete)                    |
| 400  | Bad request (invalid parameters)       |
| 401  | Unauthorized (missing/invalid API key) |
| 404  | Not found                              |
| 429  | Rate limit exceeded                    |
| 500  | Server error                           |

### Error Response Format

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Query cannot be empty",
    "details": {}
  }
}
```

---

## Best Practices

1. **Use Workspaces**: Isolate documents by project/tenant
2. **Handle Streaming**: Use SSE for real-time responses
3. **Retry with Backoff**: Handle temporary failures gracefully
4. **Cache Responses**: Cache repeated queries when appropriate
5. **Monitor Costs**: Track API usage via the cost endpoint

---

## See Also

- [REST API Reference](../api-reference/rest-api.md) - Full API documentation
- [Extended API Reference](../api-reference/extended-api.md) - Additional endpoints
- [LangChain Integration](./langchain.md) - Python RAG integration
- [Open WebUI Integration](./open-webui.md) - Chat UI integration
