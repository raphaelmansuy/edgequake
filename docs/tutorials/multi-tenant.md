# Tutorial: Multi-Tenant Setup

> **Building a SaaS Application with EdgeQuake**

This tutorial shows how to use EdgeQuake's built-in multi-tenancy to build applications that serve multiple customers with isolated data.

**Time**: ~25 minutes  
**Level**: Intermediate  
**Prerequisites**: Completed [First RAG App](first-rag-app.md)

---

## Multi-Tenancy Architecture

EdgeQuake provides tenant isolation at multiple levels:

```
┌─────────────────────────────────────────────────────────────────┐
│                   MULTI-TENANCY HIERARCHY                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                       TENANT A                             │ │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐   │ │
│  │  │  Workspace 1  │  │  Workspace 2  │  │  Workspace 3  │   │ │
│  │  │  (HR Docs)    │  │  (Legal)      │  │  (Product)    │   │ │
│  │  │               │  │               │  │               │   │ │
│  │  │  Documents    │  │  Documents    │  │  Documents    │   │ │
│  │  │  Entities     │  │  Entities     │  │  Entities     │   │ │
│  │  │  Graph        │  │  Graph        │  │  Graph        │   │ │
│  │  └───────────────┘  └───────────────┘  └───────────────┘   │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                       TENANT B                             │ │
│  │  ┌───────────────┐  ┌───────────────┐                      │ │
│  │  │  Workspace 1  │  │  Workspace 2  │                      │ │
│  │  │  (Research)   │  │  (Sales)      │                      │ │
│  │  └───────────────┘  └───────────────┘                      │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  Isolation: Complete data separation per workspace              │
│  Sharing: None by default, configurable                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Step 1: Understand the Data Model

### Hierarchy

```
Tenant (your SaaS customer)
  └── Workspace (logical container)
        ├── Documents
        ├── Chunks
        ├── Entities
        ├── Relationships
        └── Communities
```

### Workspace Properties

| Property             | Description              |
| -------------------- | ------------------------ |
| `id`                 | Unique identifier (UUID) |
| `name`               | Human-readable name      |
| `description`        | Optional description     |
| `tenant_id`          | Parent tenant ID         |
| `llm_provider`       | Override default LLM     |
| `llm_model`          | Override default model   |
| `embedding_provider` | Override embeddings      |
| `embedding_model`    | Override embedding model |
| `created_at`         | Creation timestamp       |

---

## Step 2: Create Tenants and Workspaces

### Create a Tenant (Your Customer)

```bash
curl -X POST http://localhost:8080/api/v1/tenants \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Acme Corporation",
    "external_id": "acme-corp-001",
    "settings": {
      "max_workspaces": 10,
      "max_documents_per_workspace": 1000
    }
  }'
```

**Response:**

```json
{
  "id": "tenant_abc123",
  "name": "Acme Corporation",
  "external_id": "acme-corp-001",
  "created_at": "2024-01-15T10:00:00Z"
}
```

### Create Workspaces for the Tenant

```bash
# HR Documents workspace
curl -X POST http://localhost:8080/api/v1/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "name": "HR Knowledge Base",
    "description": "Employee policies and procedures",
    "tenant_id": "tenant_abc123"
  }'

# Legal Documents workspace
curl -X POST http://localhost:8080/api/v1/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Legal Documents",
    "description": "Contracts and compliance",
    "tenant_id": "tenant_abc123"
  }'
```

---

## Step 3: Workspace-Level LLM Configuration

Each workspace can have its own LLM configuration:

```bash
# Create workspace with custom LLM settings
curl -X POST http://localhost:8080/api/v1/workspaces \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Premium Workspace",
    "tenant_id": "tenant_abc123",
    "llm_provider": "openai",
    "llm_model": "gpt-4o",
    "embedding_provider": "openai",
    "embedding_model": "text-embedding-3-large"
  }'
```

### Configuration Inheritance

```
Server Defaults (models.toml)
       │
       ▼
┌──────────────────┐
│ Workspace Config │ ◄── Overrides server defaults
└──────────────────┘
       │
       ▼
   All operations in this workspace
   use the workspace's LLM config
```

### Why This Matters

| Scenario               | Configuration                              |
| ---------------------- | ------------------------------------------ |
| Cost-conscious tenant  | Use `ollama` or `gpt-4o-mini`              |
| Premium tenant         | Use `gpt-4o` with `text-embedding-3-large` |
| Compliance requirement | Use self-hosted Ollama                     |
| Testing                | Use mock provider                          |

---

## Step 4: Data Isolation

### Document Isolation

Documents are automatically isolated by workspace:

```bash
# Upload to HR workspace
curl -X POST "http://localhost:8080/api/v1/documents?workspace_id=ws_hr" \
  -F "file=@employee_handbook.pdf"

# Upload to Legal workspace
curl -X POST "http://localhost:8080/api/v1/documents?workspace_id=ws_legal" \
  -F "file=@nda_template.pdf"
```

### Query Isolation

Queries only access data within their workspace:

```bash
# Query HR workspace - won't see Legal docs
curl -X POST "http://localhost:8080/api/v1/query?workspace_id=ws_hr" \
  -H "Content-Type: application/json" \
  -d '{"query": "What is the vacation policy?"}'

# Query Legal workspace - won't see HR docs
curl -X POST "http://localhost:8080/api/v1/query?workspace_id=ws_legal" \
  -H "Content-Type: application/json" \
  -d '{"query": "What are the NDA terms?"}'
```

### Graph Isolation

Each workspace has its own knowledge graph:

```bash
# Get entities from HR workspace only
curl "http://localhost:8080/api/v1/graph/entities?workspace_id=ws_hr"

# Get entities from Legal workspace only
curl "http://localhost:8080/api/v1/graph/entities?workspace_id=ws_legal"
```

---

## Step 5: Building a Multi-Tenant API

Wrap EdgeQuake with your own authentication layer:

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                   YOUR SAAS APPLICATION                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐                                            │
│  │  Your Auth API  │◄── JWT / API Key authentication            │
│  │  (Node/Python)  │                                            │
│  └────────┬────────┘                                            │
│           │                                                     │
│           │ Extracts tenant_id + workspace_id from token        │
│           │                                                     │
│           ▼                                                     │
│  ┌─────────────────┐                                            │
│  │  EdgeQuake API  │◄── Receives workspace_id for isolation     │
│  │  :8080          │                                            │
│  └─────────────────┘                                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Example: Express.js Wrapper

```javascript
const express = require("express");
const axios = require("axios");
const jwt = require("jsonwebtoken");

const app = express();
const EDGEQUAKE_URL = "http://localhost:8080";

// Middleware: Extract tenant from JWT
function extractTenant(req, res, next) {
  const token = req.headers.authorization?.split(" ")[1];
  if (!token) return res.status(401).json({ error: "No token" });

  try {
    const decoded = jwt.verify(token, process.env.JWT_SECRET);
    req.tenantId = decoded.tenant_id;
    req.workspaceId = decoded.workspace_id;
    next();
  } catch (err) {
    res.status(401).json({ error: "Invalid token" });
  }
}

// Proxy query with workspace isolation
app.post("/api/query", extractTenant, async (req, res) => {
  try {
    const response = await axios.post(
      `${EDGEQUAKE_URL}/api/v1/query?workspace_id=${req.workspaceId}`,
      req.body,
    );
    res.json(response.data);
  } catch (err) {
    res.status(err.response?.status || 500).json(err.response?.data || {});
  }
});

// Proxy document upload
app.post("/api/documents", extractTenant, async (req, res) => {
  try {
    const response = await axios.post(
      `${EDGEQUAKE_URL}/api/v1/documents?workspace_id=${req.workspaceId}`,
      req.body,
      { headers: { "Content-Type": req.headers["content-type"] } },
    );
    res.json(response.data);
  } catch (err) {
    res.status(err.response?.status || 500).json(err.response?.data || {});
  }
});

app.listen(3000);
```

### Example: Python FastAPI Wrapper

```python
from fastapi import FastAPI, Depends, HTTPException
from fastapi.security import HTTPBearer
import httpx
import jwt

app = FastAPI()
security = HTTPBearer()
EDGEQUAKE_URL = "http://localhost:8080"

def get_workspace(token: str = Depends(security)):
    try:
        payload = jwt.decode(token.credentials, "secret", algorithms=["HS256"])
        return payload["workspace_id"]
    except:
        raise HTTPException(401, "Invalid token")

@app.post("/api/query")
async def query(body: dict, workspace_id: str = Depends(get_workspace)):
    async with httpx.AsyncClient() as client:
        response = await client.post(
            f"{EDGEQUAKE_URL}/api/v1/query?workspace_id={workspace_id}",
            json=body
        )
        return response.json()

@app.post("/api/documents")
async def upload_document(
    file: UploadFile,
    workspace_id: str = Depends(get_workspace)
):
    async with httpx.AsyncClient() as client:
        response = await client.post(
            f"{EDGEQUAKE_URL}/api/v1/documents?workspace_id={workspace_id}",
            files={"file": file.file}
        )
        return response.json()
```

---

## Step 6: Workspace Management

### List Tenant's Workspaces

```bash
curl "http://localhost:8080/api/v1/workspaces?tenant_id=tenant_abc123"
```

**Response:**

```json
{
  "workspaces": [
    {
      "id": "ws_hr",
      "name": "HR Knowledge Base",
      "document_count": 45,
      "entity_count": 230
    },
    {
      "id": "ws_legal",
      "name": "Legal Documents",
      "document_count": 120,
      "entity_count": 580
    }
  ]
}
```

### Get Workspace Statistics

```bash
curl "http://localhost:8080/api/v1/workspaces/ws_hr/stats"
```

**Response:**

```json
{
  "workspace_id": "ws_hr",
  "documents": 45,
  "chunks": 890,
  "entities": 230,
  "relationships": 450,
  "storage_bytes": 15728640,
  "last_activity": "2024-01-15T10:00:00Z"
}
```

### Delete Workspace (with all data)

```bash
curl -X DELETE "http://localhost:8080/api/v1/workspaces/ws_hr"
```

⚠️ **Warning**: This deletes all documents, entities, and embeddings in the workspace.

---

## Step 7: Cross-Workspace Queries (Advanced)

For some use cases, you may want to query across workspaces:

```bash
# Query multiple workspaces (requires explicit permission)
curl -X POST "http://localhost:8080/api/v1/query/multi" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "Company policies overview",
    "workspace_ids": ["ws_hr", "ws_legal"],
    "mode": "global"
  }'
```

**Note**: Cross-workspace queries require tenant-level authentication.

---

## Step 8: Usage Tracking

Track usage per tenant for billing:

```bash
curl "http://localhost:8080/api/v1/tenants/tenant_abc123/usage" \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

**Response:**

```json
{
  "tenant_id": "tenant_abc123",
  "period": "2024-01",
  "usage": {
    "documents_uploaded": 165,
    "queries_executed": 2340,
    "llm_tokens_used": 1250000,
    "embedding_tokens_used": 650000,
    "storage_bytes": 52428800
  },
  "estimated_cost": {
    "llm": 18.75,
    "embedding": 1.3,
    "storage": 0.5,
    "total": 20.55
  }
}
```

---

## Best Practices

### 1. Workspace Naming Conventions

```
{tenant_slug}_{purpose}_{environment}

Examples:
- acme_hr_prod
- acme_legal_prod
- acme_hr_staging
```

### 2. LLM Configuration Strategy

| Tier    | LLM          | Embedding              | Cost |
| ------- | ------------ | ---------------------- | ---- |
| Free    | Ollama local | Ollama local           | $0   |
| Basic   | gpt-4o-mini  | text-embedding-3-small | $    |
| Premium | gpt-4o       | text-embedding-3-large | $$$  |

### 3. Quota Management

Set workspace-level quotas:

```bash
curl -X PATCH "http://localhost:8080/api/v1/workspaces/ws_hr" \
  -H "Content-Type: application/json" \
  -d '{
    "quotas": {
      "max_documents": 1000,
      "max_queries_per_day": 10000,
      "max_storage_bytes": 1073741824
    }
  }'
```

### 4. Audit Logging

EdgeQuake logs all operations with workspace context:

```json
{
  "timestamp": "2024-01-15T10:00:00Z",
  "action": "document.upload",
  "workspace_id": "ws_hr",
  "tenant_id": "tenant_abc123",
  "user_id": "user_456",
  "document_id": "doc_789",
  "file_size": 1048576
}
```

---

## Database Schema (PostgreSQL)

EdgeQuake stores tenant data with workspace isolation:

```sql
-- Workspaces table
CREATE TABLE workspaces (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    llm_provider VARCHAR(50),
    llm_model VARCHAR(100),
    embedding_provider VARCHAR(50),
    embedding_model VARCHAR(100),
    created_at TIMESTAMP DEFAULT NOW()
);

-- Documents always reference workspace
CREATE TABLE documents (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    title VARCHAR(255),
    content TEXT,
    status VARCHAR(50),
    created_at TIMESTAMP DEFAULT NOW()
);

-- Embeddings scoped to workspace
CREATE TABLE embeddings (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL REFERENCES workspaces(id),
    chunk_id UUID NOT NULL,
    embedding vector(1536)
);

-- Index for workspace isolation
CREATE INDEX idx_documents_workspace ON documents(workspace_id);
CREATE INDEX idx_embeddings_workspace ON embeddings(workspace_id);
```

---

## Troubleshooting

### Data Leaking Between Workspaces

**Symptoms**: Query returns docs from wrong workspace.

**Check**:

1. Verify `workspace_id` is passed to every API call
2. Check middleware is extracting correct workspace
3. Review database queries have workspace filter

### Wrong LLM Being Used

**Symptoms**: Responses differ from expected model.

**Check**:

```bash
# Get workspace config
curl "http://localhost:8080/api/v1/workspaces/ws_hr"
```

Verify `llm_provider` and `llm_model` are set correctly.

### Quota Exceeded

**Symptoms**: API returns 429 errors.

**Check**:

```bash
# Get workspace usage
curl "http://localhost:8080/api/v1/workspaces/ws_hr/stats"
```

Increase quotas or upgrade tier.

---

## What You Learned

✅ Multi-tenancy architecture and hierarchy  
✅ Creating tenants and workspaces  
✅ Workspace-level LLM configuration  
✅ Data isolation guarantees  
✅ Building authenticated API wrappers  
✅ Usage tracking for billing  
✅ Best practices for SaaS

---

## Next Steps

| Tutorial                                  | Description                |
| ----------------------------------------- | -------------------------- |
| [Custom Entity Types](custom-entities.md) | Domain-specific extraction |
| [API Integration](api-integration.md)     | Building on EdgeQuake      |
| [Scaling Guide](../operations/scaling.md) | Growing your deployment    |

---

## See Also

- [Architecture Overview](../architecture/overview.md) - System design
- [Configuration](../operations/configuration.md) - All settings
- [REST API](../api-reference/rest-api.md) - Complete API reference
