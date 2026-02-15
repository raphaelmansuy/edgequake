# EdgeQuake MCP Server

> **Model Context Protocol (MCP) Server for EdgeQuake**  
> Use EdgeQuake as persistent agent memory for AI agents and autonomous systems

This package provides a Model Context Protocol (MCP) server that integrates EdgeQuake's graph-based retrieval and generation capabilities with AI agents, enabling them to maintain structured, contextual memory across conversations.

## What is MCP?

The [Model Context Protocol](https://modelcontextprotocol.io) is an open standard that allows AI models to safely access data and tools in external systems. With this MCP server, AI agents can:

- **Store memories** as a knowledge graph in EdgeQuake
- **Query memories** using sophisticated graph traversal
- **Reason over relationships** between concepts, entities, and events
- **Maintain context** across multiple conversations with full traceability

## Features

### 🧠 Persistent Agent Memory

- **Structured Storage**: Entities, relationships, and communities are stored in EdgeQuake's knowledge graph
- **Multi-Hop Reasoning**: Query engine traverses graph relationships for complex reasoning
- **Entity Deduplication**: Automatic normalization prevents memory fragmentation
- **Typology Support**: 7 entity types (Person, Organization, Location, Concept, Event, Technology, Product)

### 🔗 MCP Resources

- **Memory Documents**: Store and retrieve memories as indexed documents
- **Entity Registry**: Access all extracted entities with metadata
- **Relationship Map**: Query relationships between concepts
- **Query History**: Track all previous queries and responses

### 🛠️ MCP Tools

- **query**: Execute RAG queries against the knowledge graph
- **document_upload**: Upload text content for knowledge extraction
- **document_upload_file**: Upload files (.txt, .md, .pdf) from file paths
- **document_list**: List documents with pagination and filtering
- **document_get**: Get document details
- **document_delete**: Delete documents
- **document_status**: Check processing status
- **workspace_create**: Create new workspaces
- **workspace_list**: List all workspaces
- **workspace_get**: Get workspace details
- **workspace_delete**: Delete workspaces
- **workspace_stats**: Get workspace statistics
- **graph_entity_neighborhood**: Explore entity relationships
- **graph_search_entities**: Search for entities
- **graph_search_relationships**: Search relationships
- **health_check**: Check backend connectivity

### 📊 Multiple Query Modes

The MCP server supports all 6 EdgeQuake query modes:

- **Naive**: Fast vector-only search
- **Local**: Entity-centric with neighborhood exploration
- **Global**: Community-based semantic search
- **Hybrid**: Combined local + global (default)
- **Mix**: Custom weighted combination
- **Bypass**: Direct LLM without graph results

## Installation

### Prerequisites

- **Node.js** 18.0.0 or later
- **EdgeQuake Backend**: Running on `http://localhost:8080` (or configure URL)
- **MCP Compatible Client**: Claude for Desktop, VS Code CopilotKit, or custom MCP client

### Via npm

```bash
npm install @edgequake/mcp-server
```

### From Source

```bash
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake/mcp
npm install
npm run build
```

## Usage

### As an MCP Server

Run the server to make it available to MCP clients:

```bash
edgequake-mcp
```

The server starts on stdio and communicates via JSON-RPC 2.0. Configure your MCP client to connect to this server.

### Configuration

The server reads from environment variables:

```bash
# EdgeQuake backend URL
EDGEQUAKE_BASE_URL=http://localhost:8080

# Default Tenant
EDGEQUAKE_DEFAULT_TENANT=default

# Default Workspace
EDGEQUAKE_DEFAULT_WORKSPACE=default

# Optional: LLM model for entity extraction
EDGEQUAKE_MODEL=gpt-5-nano

# Optional: Enable debug logging
DEBUG=edgequake:*
```

## Security

### Authentication

EdgeQuake MCP supports two authentication methods:

#### 1. API Key (Recommended for Services)

**Local Development:**

```bash
# No API key needed for localhost
EDGEQUAKE_BASE_URL=http://localhost:8080 npx @edgequake/mcp-server
```

**Production (Required):**

```bash
export EDGEQUAKE_API_KEY="eq-key-YOUR-API-KEY-HERE"
export EDGEQUAKE_BASE_URL="https://api.edgequake.io"
npx @edgequake/mcp-server
```

**How It Works:**

- API key is sent via `X-API-Key` header (not in URL)
- Keys are workspace-scoped (cannot access other workspaces)
- Generate keys via EdgeQuake dashboard or API

#### 2. JWT Token (For User Applications)

```bash
# Login returns access + refresh tokens
export EDGEQUAKE_ACCESS_TOKEN="eyJhbGciOiJIUzI1..."
npx @edgequake/mcp-server
```

**Token Lifecycle:**

- Access tokens expire after 15 minutes
- SDK automatically refreshes using refresh token
- MCP server maintains session throughout

### Best Practices

✅ **DO:**

- Store API keys in environment variables
- Use workspace-scoped keys (least privilege)
- Rotate keys every 30-90 days
- Revoke keys when decommissioning agents
- Use HTTPS in production (`EDGEQUAKE_BASE_URL=https://...`)

❌ **DON'T:**

- Commit API keys to version control
- Log API keys in tool responses
- Share API keys between environments
- Use global admin keys for agents

### Multi-Tenant Isolation

**IMPORTANT:** If your API key can access multiple tenants, you **MUST** specify the tenant explicitly:

```bash
export EDGEQUAKE_DEFAULT_TENANT="tenant-uuid-abc123"
export EDGEQUAKE_DEFAULT_WORKSPACE="workspace-uuid-xyz789"
```

**Why?** Auto-discovery defaults to the first tenant returned by the API, which may not be the intended target.

**Security Implications:**

- Workspaces isolate knowledge graphs (no cross-workspace queries)
- Agents with access to multiple workspaces should use separate MCP instances
- Workspace deletion is permanent (no recovery)

### Reporting Security Issues

Found a security vulnerability? **DO NOT** open a public issue. Email: security@edgequake.io

### With Claude for Desktop

Add to `claude_desktop_config.json`:

**Local Development:**

```json
{
  "mcpServers": {
    "edgequake": {
      "command": "npx",
      "args": ["-y", "@edgequake/mcp-server"]
    }
  }
}
```

**Production (with API Key):**

```json
{
  "mcpServers": {
    "edgequake": {
      "command": "npx",
      "args": ["-y", "@edgequake/mcp-server"],
      "env": {
        "EDGEQUAKE_BASE_URL": "https://api.edgequake.io",
        "EDGEQUAKE_API_KEY": "eq-key-YOUR-API-KEY-HERE",
        "EDGEQUAKE_DEFAULT_TENANT": "your-tenant-id",
        "EDGEQUAKE_DEFAULT_WORKSPACE": "your-workspace-id"
      }
    }
  }
}
```

### With VS Code (GitHub Copilot Chat)

GitHub Copilot Chat supports MCP servers via your VS Code `settings.json`. Add the following configuration to enable EdgeQuake integration:

```json
{
  "github.copilot.chat.mcpServers": {
    "edgequake": {
      "command": "npx",
      "args": ["-y", "@edgequake/mcp-server"],
      "env": {
        "EDGEQUAKE_BASE_URL": "http://localhost:8080",
        "EDGEQUAKE_DEFAULT_TENANT": "default",
        "EDGEQUAKE_DEFAULT_WORKSPACE": "default"
      }
    }
  }
}
```

### With Cursor

Cursor supports MCP in its internal settings. Navigate to **Cursor Settings** > **Features** > **MCP Servers** and add:

- **Name**: `edgequake`
- **Type**: `command`
- **Command**: `npx -y @edgequake/mcp-server`

Then Claude can use commands like:

- "Remember this fact in my knowledge graph"
- "What do I know about X and how does it relate to Y?"
- "Show me all communities of related concepts"

### With CopilotKit

```typescript
import { CopilotKit } from "@copilotkit/react-core";
import { MCPProvider } from "@copilotkit/react-mcp";

export default function App() {
  return (
    <CopilotKit>
      <MCPProvider
        serverUrl="ws://localhost:3000/mcp"
        name="edgequake"
      >
        {/* Your app */}
      </MCPProvider>
    </CopilotKit>
  );
}
```

## Architecture

### How It Works

1. **Agent Issues Command**: AI agent sends request (e.g., "store this memory")
2. **MCP Handler**: Server receives JSON-RPC request
3. **EdgeQuake Integration**: Routes to EdgeQuake API for processing
4. **Graph Processing**: EdgeQuake extracts entities, relationships, communities
5. **Response**: Returns structured data to agent with next steps

### Storage Backend

The MCP server connects to EdgeQuake's backend, which supports:

- **PostgreSQL + pgvector**: Vector storage and similarity search
- **Apache AGE**: Property graph storage for relationships
- **LLM Integration**: OpenAI, Ollama, or custom providers

## Example Usage

### Storing Agent Memory

**Agent (Claude)**:

```
Remember: Sarah Chen founded TechCorp in 2020. It's a machine learning startup.
```

**MCP Server**:

1. Extracts entities: {Sarah Chen (Person), TechCorp (Organization), ML (Technology)}
2. Extracts relationships: {Sarah Chen --founded--> TechCorp, TechCorp --uses--> ML}
3. Stores in EdgeQuake knowledge graph

### Querying Agent Memory

**Agent**:

```
Who are the founders of companies in my memory that work on machine learning?
```

**MCP Server**:

1. Queries for entities matching query intent
2. Traverses relationships: Person --founded--> Organization --uses--> Technology
3. Returns relevant results with confidence scores

### Uploading Files

**Upload a Markdown file:**

```json
{
  "tool": "document_upload_file",
  "arguments": {
    "file_path": "/Users/alice/projects/research/paper.md",
    "title": "Research Paper on Graph Neural Networks",
    "metadata": {
      "category": "research",
      "year": "2024"
    }
  }
}
```

**Upload a PDF document:**

```json
{
  "tool": "document_upload_file",
  "arguments": {
    "file_path": "/Users/alice/documents/company-report.pdf"
  }
}
```

**Upload a text file:**

```json
{
  "tool": "document_upload_file",
  "arguments": {
    "file_path": "/tmp/meeting-notes.txt",
    "enable_gleaning": true
  }
}
```

**Supported file types:**
- `.txt` - Plain text files
- `.md`, `.markdown` - Markdown files
- `.pdf` - PDF documents

## Workspaces & Isolation

### Understanding Workspaces

EdgeQuake uses **workspaces** to isolate knowledge graphs:

- **One workspace = One knowledge graph** (entities, relationships, documents)
- **Workspaces cannot share data** (isolation enforced server-side)
- **Each workspace has its own LLM configuration** (provider, model, embedding)

### When to Create a New Workspace

Create a new workspace when:

- ✅ Starting a new project with unrelated documents
- ✅ Needing different LLM providers per project (e.g., Ollama for dev, OpenAI for prod)
- ✅ Isolating sensitive data from general knowledge
- ✅ Testing different extraction strategies without affecting production data

Use the same workspace when:

- ✅ Documents are related and should reference each other
- ✅ Entities should be deduplicated across documents
- ✅ Queries should span multiple document sources
- ✅ Building a unified knowledge base

### Example: Creating a Project Workspace

**Using the MCP Tool:**

```json
{
  "tool": "workspace_create",
  "arguments": {
    "name": "ML Research Project",
    "description": "Papers and notes on graph neural networks",
    "llm_provider": "ollama",
    "llm_model": "gemma3:12b",
    "embedding_provider": "ollama",
    "embedding_model": "nomic-embed-text"
  }
}
```

**Response:**

```json
{
  "id": "workspace-uuid-xyz789",
  "name": "ML Research Project",
  "slug": "ml-research-project"
}
```

### Supported LLM Providers

| Provider  | Value      | Models                    | Notes                         |
| --------- | ---------- | ------------------------- | ----------------------------- |
| Ollama    | `ollama`   | `gemma3:12b`, `llama3:8b` | Local, free, fast             |
| OpenAI    | `openai`   | `gpt-5-nano`, `gpt-4o`    | Cloud, API key required       |
| LM Studio | `lmstudio` | Custom models             | OpenAI-compatible API         |
| Mock      | `mock`     | N/A                       | Testing only (fake responses) |

### Supported Embedding Providers

| Provider | Value    | Models                   | Dimensions |
| -------- | -------- | ------------------------ | ---------- |
| Ollama   | `ollama` | `nomic-embed-text`       | 768        |
| OpenAI   | `openai` | `text-embedding-3-small` | 1536       |

### Workspace Lifecycle

**List workspaces:**

```json
{
  "tool": "workspace_list"
}
```

**Get workspace details:**

```json
{
  "tool": "workspace_get",
  "arguments": {
    "workspace_id": "workspace-uuid-xyz789"
  }
}
```

**Delete a workspace** when no longer needed:

```json
{
  "tool": "workspace_delete",
  "arguments": {
    "workspace_id": "workspace-uuid-xyz789"
  }
}
```

⚠️ **WARNING:** Deleting a workspace:

- Removes ALL documents, entities, relationships
- Cannot be undone
- Revokes workspace-scoped API keys

### Workspace Best Practices

1. **Use descriptive names**: "Customer Support Q4 2024" not "workspace1"
2. **Configure LLM per use case**: Small models for simple extraction, larger for complex reasoning
3. **Monitor workspace stats**: Check entity/document counts regularly
4. **Clean up old workspaces**: Delete workspaces when projects end
5. **One agent per workspace**: Avoid sharing workspaces between independent agents

## Development

### Build

```bash
npm run build
```

### Watch Mode

```bash
npm run dev
```

### Test

```bash
npm test                 # Unit tests
npm run test:e2e        # Integration tests with live EdgeQuake instance
```

### Lint

```bash
npm run lint
```

## API Reference

The EdgeQuake MCP server provides 17 tools across 5 categories:

### Query Tools

#### `query`

Execute a RAG query against the EdgeQuake knowledge graph.

**Parameters:**

| Name                    | Type    | Required | Default   | Description                                            |
| ----------------------- | ------- | -------- | --------- | ------------------------------------------------------ |
| `query`                 | string  | yes      | -         | Natural language question                              |
| `mode`                  | string  | no       | `hybrid`  | Query mode: `naive`, `local`, `global`, `hybrid`, `mix` |
| `max_results`           | number  | no       | `10`      | Maximum number of source references to return          |
| `include_references`    | boolean | no       | `true`    | Include source snippets in response                    |
| `conversation_history`  | array   | no       | `[]`      | Prior conversation messages for multi-turn context     |

**Returns:**

```json
{
  "answer": "EdgeQuake is a Graph-RAG framework...",
  "mode": "hybrid",
  "sources": [
    {
      "source_type": "entity",
      "snippet": "EdgeQuake combines graph databases...",
      "score": 0.95,
      "document_id": "doc-uuid-123"
    }
  ],
  "stats": {
    "total_time_ms": 234,
    "sources_retrieved": 5
  }
}
```

### Document Tools

#### `document_upload`

Upload a text document to EdgeQuake for knowledge graph extraction.

**Parameters:**

| Name              | Type    | Required | Default      | Description                                      |
| ----------------- | ------- | -------- | ------------ | ------------------------------------------------ |
| `content`         | string  | yes      | -            | Document text content                            |
| `title`           | string  | no       | `"Untitled"` | Document title                                   |
| `metadata`        | object  | no       | `{}`         | Custom metadata key-value pairs                  |
| `enable_gleaning` | boolean | no       | `true`       | Enable multi-pass extraction for better recall   |

**Returns:**

```json
{
  "document_id": "doc-uuid-123",
  "status": "processing",
  "task_id": "task-uuid-456",
  "chunk_count": 12,
  "entity_count": 45,
  "relationship_count": 78
}
```

#### `document_upload_file`

Upload a file from a file path to EdgeQuake for knowledge graph extraction. Supports text files (.txt, .md) and PDFs (.pdf).

**Parameters:**

| Name              | Type    | Required | Default      | Description                                           |
| ----------------- | ------- | -------- | ------------ | ----------------------------------------------------- |
| `file_path`       | string  | yes      | -            | Absolute path to the file to upload                   |
| `title`           | string  | no       | filename     | Document title (defaults to filename)                 |
| `metadata`        | object  | no       | `{}`         | Custom metadata key-value pairs                       |
| `enable_gleaning` | boolean | no       | `true`       | Enable multi-pass extraction for better recall        |

**Supported File Types:**
- `.txt` - Plain text files
- `.md`, `.markdown` - Markdown files
- `.pdf` - PDF documents

**Returns:**

For text/markdown files:
```json
{
  "document_id": "doc-uuid-123",
  "file_name": "paper.md",
  "file_type": "md",
  "status": "processing",
  "track_id": "track-uuid-456",
  "chunk_count": 12,
  "entity_count": 45,
  "relationship_count": 78,
  "message": "File uploaded successfully."
}
```

For PDF files:
```json
{
  "document_id": "pdf-uuid-123",
  "file_name": "report.pdf",
  "file_type": "pdf",
  "status": "processing",
  "track_id": "track-uuid-456",
  "message": "PDF uploaded successfully. Use document_status to track processing."
}
```

**Example:**
```json
{
  "tool": "document_upload_file",
  "arguments": {
    "file_path": "/Users/alice/research/paper.md",
    "title": "Neural Networks Research"
  }
}
```

#### `document_list`

List documents with pagination and filtering.

**Parameters:**

| Name        | Type   | Required | Default | Description                                                   |
| ----------- | ------ | -------- | ------- | ------------------------------------------------------------- |
| `page`      | number | no       | `1`     | Page number                                                   |
| `page_size` | number | no       | `20`    | Items per page                                                |
| `status`    | string | no       | -       | Filter by: `pending`, `processing`, `completed`, `failed`     |
| `search`    | string | no       | -       | Full-text search in title/content                             |

#### `document_get`

Get document details including full content and metadata.

**Parameters:**

| Name          | Type   | Required | Description  |
| ------------- | ------ | -------- | ------------ |
| `document_id` | string | yes      | Document UUID |

#### `document_delete`

Delete a document and its extracted knowledge.

**Parameters:**

| Name          | Type   | Required | Description  |
| ------------- | ------ | -------- | ------------ |
| `document_id` | string | yes      | Document UUID |

#### `document_status`

Check the processing status of a document.

**Parameters:**

| Name          | Type   | Required | Description  |
| ------------- | ------ | -------- | ------------ |
| `document_id` | string | yes      | Document UUID |

### Workspace Tools

#### `workspace_create`

Create a new workspace for document ingestion and knowledge graph.

**Parameters:**

| Name                  | Type   | Required | Description                                      |
| --------------------- | ------ | -------- | ------------------------------------------------ |
| `name`                | string | yes      | Workspace name                                   |
| `description`         | string | no       | Workspace description                            |
| `llm_model`           | string | no       | LLM model (e.g., `gemma3:12b`)                   |
| `llm_provider`        | string | no       | LLM provider: `ollama`, `openai`, `lmstudio`     |
| `embedding_model`     | string | no       | Embedding model name                             |
| `embedding_provider`  | string | no       | Embedding provider: `ollama`, `openai`           |

#### `workspace_list`

List all workspaces in the current tenant.

#### `workspace_get`

Get workspace details including document and entity counts.

**Parameters:**

| Name           | Type   | Required | Description   |
| -------------- | ------ | -------- | ------------- |
| `workspace_id` | string | yes      | Workspace UUID |

#### `workspace_delete`

Delete a workspace and all its data.

**Parameters:**

| Name           | Type   | Required | Description   |
| -------------- | ------ | -------- | ------------- |
| `workspace_id` | string | yes      | Workspace UUID |

#### `workspace_stats`

Get statistics for a workspace.

**Parameters:**

| Name           | Type   | Required | Description   |
| -------------- | ------ | -------- | ------------- |
| `workspace_id` | string | yes      | Workspace UUID |

### Graph Exploration Tools

#### `graph_entity_neighborhood`

Get an entity's neighborhood with connected entities and relationships.

**Parameters:**

| Name          | Type   | Required | Default | Description                       |
| ------------- | ------ | -------- | ------- | --------------------------------- |
| `entity_name` | string | yes      | -       | Entity name to explore            |
| `max_depth`   | number | no       | `1`     | Maximum relationship hops (1-3)   |

#### `graph_search_entities`

Search for entities by name, type, or description.

**Parameters:**

| Name          | Type   | Required | Default | Description                                                                                               |
| ------------- | ------ | -------- | ------- | --------------------------------------------------------------------------------------------------------- |
| `query`       | string | yes      | -       | Search query                                                                                              |
| `entity_type` | string | no       | -       | Filter by: `PERSON`, `ORGANIZATION`, `LOCATION`, `CONCEPT`, `EVENT`, `TECHNOLOGY`, `PRODUCT`              |
| `limit`       | number | no       | `10`    | Max results                                                                                               |

#### `graph_search_relationships`

Search for relationships between entities.

**Parameters:**

| Name                | Type   | Required | Default | Description                                                |
| ------------------- | ------ | -------- | ------- | ---------------------------------------------------------- |
| `source_entity`     | string | no       | -       | Source entity name                                         |
| `target_entity`     | string | no       | -       | Target entity name                                         |
| `relationship_type` | string | no       | -       | Type (e.g., `WORKS_AT`, `LEADS`, `DEPENDS_ON`)             |
| `limit`             | number | no       | `10`    | Max results                                                |

### Health Tools

#### `health_check`

Check EdgeQuake backend health and connectivity.

**Returns:**

```json
{
  "status": "healthy",
  "version": "0.2.2",
  "timestamp": "2024-02-15T10:30:00Z"
}
```

## Troubleshooting

### "Connection refused" to EdgeQuake

**Problem**: MCP server can't connect to EdgeQuake backend

**Solution**:

```bash
# Check EdgeQuake is running
curl http://localhost:8080/health

# Set correct URL if running elsewhere
export EDGEQUAKE_API_URL=http://your-server:8080
edgequake-mcp
```

### "Not authorized" errors

**Problem**: MCP client not configured with proper permissions

**Solution**:

- Ensure your MCP client is listed in EdgeQuake's allowed clients
- Check `EDGEQUAKE_WORKSPACE_ID` matches your workspace
- Verify API key if using authentication

### Slow memory queries

**Problem**: Queries taking >1000ms

**Solution**:

- Use "naive" mode for simple queries (`mode: "naive"`)
- Ensure EdgeQuake has built indices (run after document upload)
- Check database connection and network latency

## Contributing

Contributions welcome! Please see [CONTRIBUTING.md](../CONTRIBUTING.md) in the root repository.

## License

Apache License 2.0 - See [LICENSE](../LICENSE)

## Links

- **EdgeQuake Docs**: https://github.com/raphaelmansuy/edgequake/tree/edgequake-main/docs
- **MCP Spec**: https://spec.modelcontextprotocol.io
- **Report Issues**: https://github.com/raphaelmansuy/edgequake/issues

## Support

- **Questions?** Open an issue on [GitHub](https://github.com/raphaelmansuy/edgequake/)
- **Need Help?** Check the [FAQ](../docs/faq.md)
- **Want to Join?** See [Contributing Guide](../CONTRIBUTING.md)
