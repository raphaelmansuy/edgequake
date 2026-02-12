# EdgeQuake Go SDK

Official Go client library for the [EdgeQuake](https://github.com/edgequake/edgequake) RAG (Retrieval-Augmented Generation) API.

## Features

- **22 service clients** covering the full EdgeQuake API surface
- **Automatic retry** with exponential backoff for transient errors (429, 5xx)
- **Typed responses** — every endpoint returns strongly-typed Go structs
- **Functional options** pattern for flexible client configuration
- **Zero dependencies** — uses only the Go standard library
- **Context-aware** — all methods accept `context.Context` for cancellation and deadlines
- **Sentinel errors** — `errors.Is` support for `ErrNotFound`, `ErrUnauthorized`, etc.

## Requirements

- Go 1.21 or later

## Installation

```bash
go get github.com/edgequake/edgequake-go
```

## Quick Start

```go
package main

import (
    "context"
    "fmt"
    "log"

    edgequake "github.com/edgequake/edgequake-go"
)

func main() {
    // Create a client with default settings (localhost:8080)
    client := edgequake.NewClient(
        edgequake.WithAPIKey("your-api-key"),
    )

    ctx := context.Background()

    // Check server health
    health, err := client.Health.Check(ctx)
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Status: %s, Version: %s\n", health.Status, health.Version)

    // Upload a document
    upload, err := client.Documents.UploadText(ctx, map[string]interface{}{
        "content": "EdgeQuake is an advanced RAG framework.",
        "title":   "My Document",
    })
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Uploaded: %s\n", upload.ID)

    // Query the knowledge graph
    resp, err := client.Query.Execute(ctx, &edgequake.QueryRequest{
        Query: "What is EdgeQuake?",
        Mode:  "hybrid",
    })
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Answer: %s\n", resp.Answer)
}
```

## Client Configuration

Use functional options to configure the client:

```go
client := edgequake.NewClient(
    edgequake.WithBaseURL("https://api.example.com"),  // Default: http://localhost:8080
    edgequake.WithAPIKey("your-api-key"),               // X-API-Key header
    edgequake.WithBearerToken("jwt-token"),             // Authorization: Bearer header
    edgequake.WithTenantID("tenant-123"),               // X-Tenant-ID header
    edgequake.WithWorkspaceID("workspace-456"),         // X-Workspace-ID header
    edgequake.WithUserID("user-789"),                   // X-User-ID header
    edgequake.WithTimeout(60 * time.Second),            // Default: 30s
    edgequake.WithMaxRetries(5),                        // Default: 3
    edgequake.WithHTTPClient(customHTTPClient),         // Custom http.Client
)
```

| Option            | Default                 | Description                               |
| ----------------- | ----------------------- | ----------------------------------------- |
| `WithBaseURL`     | `http://localhost:8080` | EdgeQuake server URL                      |
| `WithAPIKey`      | `""`                    | API key for `X-API-Key` header            |
| `WithBearerToken` | `""`                    | JWT for `Authorization: Bearer` header    |
| `WithTenantID`    | `""`                    | Tenant ID for `X-Tenant-ID` header        |
| `WithWorkspaceID` | `""`                    | Workspace ID for `X-Workspace-ID` header  |
| `WithUserID`      | `""`                    | User ID for `X-User-ID` header            |
| `WithTimeout`     | `30s`                   | HTTP request timeout                      |
| `WithMaxRetries`  | `3`                     | Max retry attempts for 429/5xx errors     |
| `WithHTTPClient`  | `nil`                   | Custom `*http.Client` (overrides timeout) |

## API Reference

### Health

```go
health, err := client.Health.Check(ctx)
// health.Status, health.Version, health.StorageMode, health.LLMProvider
```

### Documents

```go
// List documents (paginated)
docs, err := client.Documents.List(ctx, page, perPage)

// Get a single document
doc, err := client.Documents.Get(ctx, "doc-id")

// Upload text content
upload, err := client.Documents.UploadText(ctx, map[string]interface{}{
    "content": "text content",
    "title":   "Document Title",
})

// Track upload progress
status, err := client.Documents.Track(ctx, "track-id")

// Scan a directory for documents
scan, err := client.Documents.Scan(ctx, &edgequake.ScanRequest{
    Path: "/data/documents",
})

// Check deletion impact before deleting
impact, err := client.Documents.DeletionImpact(ctx, "doc-id")

// Delete a single document
err := client.Documents.Delete(ctx, "doc-id")

// Delete all documents
err := client.Documents.DeleteAll(ctx)
```

### Knowledge Graph

```go
// Get graph data (nodes and edges)
graph, err := client.Graph.Get(ctx, limit)

// Search nodes
results, err := client.Graph.Search(ctx, "search query", limit)
```

### Entities

```go
// List entities (paginated, optional type filter)
entities, err := client.Entities.List(ctx, page, perPage, "PERSON")

// Get entity details with relationships and statistics
detail, err := client.Entities.Get(ctx, "ENTITY_NAME")

// Create an entity
created, err := client.Entities.Create(ctx, &edgequake.CreateEntityParams{
    EntityName:  "SARAH_CHEN",
    EntityType:  "PERSON",
    Description: "A researcher",
    SourceID:    "manual_entry",
})

// Check if entity exists
exists, err := client.Entities.Exists(ctx, "SARAH_CHEN")

// Get entity neighborhood graph
neighborhood, err := client.Entities.Neighborhood(ctx, "SARAH_CHEN", 2)

// Merge two entities
merged, err := client.Entities.Merge(ctx, &edgequake.MergeEntitiesParams{
    SourceEntity: "SARAH",
    TargetEntity: "SARAH_CHEN",
})

// Delete an entity
err := client.Entities.Delete(ctx, "ENTITY_NAME")
```

### Relationships

```go
// List relationships (paginated)
rels, err := client.Relationships.List(ctx, page, perPage)

// Create a relationship
rel, err := client.Relationships.Create(ctx, &edgequake.CreateRelationshipParams{
    Source:           "SARAH_CHEN",
    Target:           "MIT",
    RelationshipType: "AFFILIATED_WITH",
})
```

### Query

```go
resp, err := client.Query.Execute(ctx, &edgequake.QueryRequest{
    Query: "What are the key findings?",
    Mode:  "hybrid",  // "local", "global", or "hybrid"
})
// resp.Answer, resp.Sources, resp.Mode
```

### Chat

```go
resp, err := client.Chat.Completions(ctx, &edgequake.ChatCompletionRequest{
    Messages: []edgequake.ChatMessage{
        {Role: "user", Content: "Summarize the research paper."},
    },
    Model: "gpt-5-nano",
})
// resp.Choices[0].Message.Content
```

### Authentication

```go
// Login with username/password
token, err := client.Auth.Login(ctx, &edgequake.LoginParams{
    Username: "admin",
    Password: "password",
})
// token.AccessToken, token.RefreshToken

// Get current user info
user, err := client.Auth.Me(ctx)

// Refresh an expired token
newToken, err := client.Auth.Refresh(ctx, &edgequake.RefreshParams{
    RefreshToken: token.RefreshToken,
})
```

### Users

```go
// Create a user
user, err := client.Users.Create(ctx, &edgequake.CreateUserParams{
    Username: "newuser",
    Email:    "user@example.com",
    Password: "secure-password",
    Role:     "viewer",
})

// Get a user by ID
user, err := client.Users.Get(ctx, "user-id")

// List all users
users, err := client.Users.List(ctx)
```

### API Keys

```go
// Create an API key
key, err := client.APIKeys.Create(ctx, "my-api-key")
// key.ID, key.Key

// List API keys
keys, err := client.APIKeys.List(ctx)

// Revoke an API key
err := client.APIKeys.Revoke(ctx, "key-id")
```

### Tenants

```go
// List tenants
tenants, err := client.Tenants.List(ctx)

// Create a tenant
tenant, err := client.Tenants.Create(ctx, &edgequake.CreateTenantParams{
    Name: "My Organization",
    Slug: "my-org",
})
```

### Conversations

```go
// Create a conversation
conv, err := client.Conversations.Create(ctx, &edgequake.CreateConversationParams{
    Title: "Research Discussion",
})

// List conversations
convs, err := client.Conversations.List(ctx)

// Get conversation with messages
detail, err := client.Conversations.Get(ctx, "conv-id")

// Add a message
msg, err := client.Conversations.CreateMessage(ctx, "conv-id",
    &edgequake.CreateMessageParams{
        Role:    "user",
        Content: "What are the main topics?",
    })

// Share a conversation
link, err := client.Conversations.Share(ctx, "conv-id")

// Pin/unpin conversations
err := client.Conversations.Pin(ctx, "conv-id")
err := client.Conversations.Unpin(ctx, "conv-id")

// Bulk delete
resp, err := client.Conversations.BulkDelete(ctx, []string{"id1", "id2"})

// Delete a conversation
err := client.Conversations.Delete(ctx, "conv-id")
```

### Folders

```go
// Create a folder
folder, err := client.Folders.Create(ctx, &edgequake.CreateFolderParams{
    Name: "Research Papers",
})

// List folders
folders, err := client.Folders.List(ctx)

// Get a folder
folder, err := client.Folders.Get(ctx, "folder-id")

// Delete a folder
err := client.Folders.Delete(ctx, "folder-id")
```

### Tasks

```go
// List tasks (with optional status filter)
tasks, err := client.Tasks.List(ctx, "running", page, perPage)

// Get task details
task, err := client.Tasks.Get(ctx, "track-id")

// Cancel a task
err := client.Tasks.Cancel(ctx, "track-id")
```

### Pipeline

```go
// Get pipeline status
status, err := client.Pipeline.Status(ctx)

// Get queue metrics
metrics, err := client.Pipeline.Metrics(ctx)
```

### Costs

```go
// Get cost summary
summary, err := client.Costs.Summary(ctx)

// Get cost history
entries, err := client.Costs.History(ctx, "2024-01-01", "2024-12-31")

// Get budget info
budget, err := client.Costs.Budget(ctx)
```

### Chunks

```go
chunk, err := client.Chunks.Get(ctx, "chunk-id")
```

### Provenance

```go
records, err := client.Provenance.ForEntity(ctx, "ENTITY_NAME")
```

### Lineage

```go
graph, err := client.Lineage.ForEntity(ctx, "ENTITY_NAME", depth)
```

### Models

```go
// List available models and providers
catalog, err := client.Models.List(ctx)

// Get current provider status
status, err := client.Models.ProviderStatus(ctx)

// Get provider health
health, err := client.Models.ProviderHealth(ctx)
```

### Workspaces

```go
// List workspaces for a tenant
workspaces, err := client.Workspaces.ListForTenant(ctx, "tenant-id")

// Create a workspace
ws, err := client.Workspaces.CreateForTenant(ctx, "tenant-id",
    &edgequake.CreateWorkspaceParams{
        Name:        "My Workspace",
        Description: "Research workspace",
    })

// Get workspace details
ws, err := client.Workspaces.Get(ctx, "workspace-id")

// Get workspace statistics
stats, err := client.Workspaces.Stats(ctx, "workspace-id")

// Rebuild embeddings
rebuild, err := client.Workspaces.RebuildEmbeddings(ctx, "workspace-id")
```

### PDF

```go
// List PDF documents
pdfs, err := client.PDF.List(ctx)

// Get PDF processing progress
progress, err := client.PDF.Progress(ctx, "track-id")

// Get PDF status
status, err := client.PDF.Status(ctx, "pdf-id")

// Get extracted PDF content (markdown)
content, err := client.PDF.Content(ctx, "pdf-id")
```

## Error Handling

The SDK provides sentinel errors for common HTTP status codes:

```go
_, err := client.Documents.Get(ctx, "missing-id")
if errors.Is(err, edgequake.ErrNotFound) {
    // Handle 404
}

// Available sentinel errors:
// edgequake.ErrBadRequest   (400)
// edgequake.ErrUnauthorized (401)
// edgequake.ErrForbidden    (403)
// edgequake.ErrNotFound     (404)
// edgequake.ErrConflict     (409)
// edgequake.ErrValidation   (422)
// edgequake.ErrRateLimited  (429)
// edgequake.ErrServer       (5xx)
```

Access detailed error information:

```go
var apiErr *edgequake.APIError
if errors.As(err, &apiErr) {
    fmt.Printf("Status: %d, Code: %s, Message: %s\n",
        apiErr.StatusCode, apiErr.ErrorCode, apiErr.Message)

    if apiErr.IsRetryable() {
        // 429 or 5xx — consider retrying
    }
}
```

## Retry Behavior

The client automatically retries requests that fail with:

- **429 Too Many Requests** — rate limited
- **5xx Server Error** — transient server issues

Retries use exponential backoff: 500ms, 1s, 2s, 4s... up to `MaxRetries` attempts (default: 3). Disable retries with `WithMaxRetries(0)`.

## Testing

```bash
# Run unit tests
go test -v -count=1 ./...

# Run with coverage
go test -v -count=1 -cover ./...

# Run E2E tests (requires running EdgeQuake server)
EDGEQUAKE_BASE_URL=http://localhost:8080 go test -v -tags=e2e ./...
```

## License

Apache-2.0 — see [LICENSE](../../LICENSE) for details.
