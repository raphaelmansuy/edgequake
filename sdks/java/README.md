# EdgeQuake Java SDK

Java SDK for the [EdgeQuake](https://github.com/edgequake/edgequake) RAG (Retrieval-Augmented Generation) API.

## Features

- **Zero HTTP dependencies** — uses `java.net.http.HttpClient` (JDK 11+)
- **19 service clients** — health, documents, entities, relationships, graph, query, chat, auth, users, API keys, tenants, conversations, folders, tasks, pipeline, models, workspaces, PDF, costs
- **Jackson serialization** — type-safe JSON with annotated POJOs
- **Builder pattern** — fluent configuration
- **Multi-tenant support** — per-request tenant, user, and workspace headers

## Requirements

- JDK 17+
- Maven 3.8+ or Gradle 8+

## Installation

### Maven

```xml
<dependency>
    <groupId>io.edgequake</groupId>
    <artifactId>edgequake-sdk</artifactId>
    <version>0.1.0</version>
</dependency>
```

### Gradle

```groovy
implementation 'io.edgequake:edgequake-sdk:0.1.0'
```

## Quick Start

```java
import io.edgequake.sdk.*;
import io.edgequake.sdk.models.QueryModels.*;

var config = EdgeQuakeConfig.builder()
    .baseUrl("http://localhost:8080")
    .apiKey("your-api-key")
    .build();

var client = new EdgeQuakeClient(config);

// Health check
var health = client.health().check();
System.out.println("Status: " + health.status);

// Upload a document
var upload = client.documents().uploadText(
    "EdgeQuake is a graph-based RAG framework.",
    "My Document"
);
System.out.println("Document ID: " + upload.documentId);

// Query the knowledge graph
var result = client.query().execute(new QueryRequest("What is EdgeQuake?", "hybrid"));
System.out.println("Answer: " + result.answer);
```

## Configuration

```java
var config = EdgeQuakeConfig.builder()
    .baseUrl("http://localhost:8080")   // API base URL (default)
    .apiKey("sk-your-key")             // Optional API key
    .tenantId("tenant-1")             // Optional tenant ID
    .userId("user-1")                 // Optional user ID
    .workspaceId("workspace-1")       // Optional workspace ID
    .timeoutSeconds(30)               // Request timeout (default: 30s)
    .build();

var client = new EdgeQuakeClient(config);
```

## Services

### Documents

```java
// List documents
var docs = client.documents().list(1, 20);
System.out.println("Total: " + docs.pagination.total);

// Get a specific document
var doc = client.documents().get("document-id");

// Upload text content
var upload = client.documents().uploadText("Content here...", "Title");

// Track upload progress
var track = client.documents().track("track-id");
System.out.println("Status: " + track.status + " Progress: " + track.progress);

// Scan a directory
var req = new DocumentModels.ScanRequest();
req.path = "/path/to/docs";
req.recursive = true;
var scan = client.documents().scan(req);

// Check deletion impact before deleting
var impact = client.documents().deletionImpact("doc-id");

// Delete a document
client.documents().delete("document-id");
```

### Entities

```java
// List entities
var entities = client.entities().list(1, 20, null);

// Filter by type
var people = client.entities().list(1, 20, "PERSON");

// Get entity details
var entity = client.entities().get("ENTITY_NAME");

// Create an entity
var created = client.entities().create(
    new GraphModels.CreateEntityRequest("NEW_ENTITY", "CONCEPT", "A new entity", "doc-1")
);

// Check if entity exists
var exists = client.entities().exists("ENTITY_NAME");

// Merge entities
var merged = client.entities().merge(
    new GraphModels.MergeEntitiesRequest("SOURCE_ENTITY", "TARGET_ENTITY")
);

// Get entity neighborhood
var neighborhood = client.entities().neighborhood("ENTITY_NAME", 2);

// Delete an entity (requires confirm=true, handled automatically)
var deleted = client.entities().delete("ENTITY_NAME");
```

### Knowledge Graph

```java
// Get the full graph
var graph = client.graph().get(100);
System.out.println("Nodes: " + graph.totalNodes + " Edges: " + graph.totalEdges);

// Search graph nodes
var results = client.graph().search("machine learning", 10);

// List relationships
var rels = client.relationships().list(1, 100);

// Create a relationship
var rel = client.relationships().create(
    new GraphModels.CreateRelationshipRequest("ENTITY_A", "ENTITY_B", "WORKS_WITH")
);
```

### Query

```java
import io.edgequake.sdk.models.QueryModels.*;

// Hybrid query (default)
var answer = client.query().execute(new QueryRequest("What are the main concepts?", "hybrid"));

// Local-only query
var local = client.query().execute(new QueryRequest("Explain RAG", "local"));

// Global query
var global = client.query().execute(new QueryRequest("Summarize everything", "global"));
```

### Chat

```java
import io.edgequake.sdk.models.QueryModels.*;

var response = client.chat().completions(
    new ChatCompletionRequest(List.of(
        new ChatMessage("system", "You are a helpful assistant."),
        new ChatMessage("user", "What is EdgeQuake?")
    ))
);
System.out.println(response.choices.get(0).message.content);
```

### Authentication

```java
import io.edgequake.sdk.models.AuthModels.*;

// Login
var token = client.auth().login(new LoginRequest("username", "password"));
System.out.println("Token: " + token.accessToken);

// Get current user
var me = client.auth().me();

// Refresh token
var refreshed = client.auth().refresh(new RefreshRequest(token.refreshToken));

// List users
var users = client.users().list();

// Manage API keys
var keys = client.apiKeys().list();
var newKey = client.apiKeys().create("my-key");
client.apiKeys().revoke(newKey.id);
```

### Conversations

```java
import io.edgequake.sdk.models.AuthModels.*;

// List conversations
var convos = client.conversations().list();

// Create a conversation
var convo = client.conversations().create(new CreateConversationRequest("My Chat"));

// Get conversation with messages
var detail = client.conversations().get(convo.id);

// Add a message
var msg = client.conversations().createMessage(convo.id,
    new CreateMessageRequest("user", "Hello!"));

// Share a conversation
var share = client.conversations().share(convo.id);

// Pin/unpin
client.conversations().pin(convo.id);
client.conversations().unpin(convo.id);

// Delete
client.conversations().delete(convo.id);

// Bulk delete
var result = client.conversations().bulkDelete(List.of("id1", "id2"));
```

### Pipeline & Tasks

```java
// Pipeline status
var status = client.pipeline().status();
System.out.println("Busy: " + status.isBusy + " Pending: " + status.pendingTasks);

// Queue metrics
var metrics = client.pipeline().metrics();

// List tasks
var tasks = client.tasks().list(null, 1, 20);

// Filter by status
var running = client.tasks().list("running", 1, 20);

// Get task details
var task = client.tasks().get("track-id");

// Cancel a task
client.tasks().cancel("track-id");
```

### Models & Providers

```java
// Available models
var catalog = client.models().list();
catalog.providers.forEach(p ->
    System.out.println(p.name + ": " + p.models.size() + " models")
);

// Provider health
var health = client.models().providerHealth();

// Provider status
var providerStatus = client.models().providerStatus();
```

### Additional Services

```java
// Folders
var folders = client.folders().list();
var newFolder = client.folders().create(new AuthModels.CreateFolderRequest("Research"));
client.folders().delete(newFolder.id);

// Tenants
var tenants = client.tenants().list();
var newTenant = client.tenants().create(
    new AuthModels.CreateTenantRequest("New Tenant", "new-tenant"));

// Workspaces (tenant-scoped)
var workspaces = client.workspaces().listForTenant("tenant-id");
var stats = client.workspaces().stats("workspace-id");
var rebuild = client.workspaces().rebuildEmbeddings("workspace-id");

// PDF processing
var progress = client.pdf().progress("track-id");
var content = client.pdf().content("pdf-id");
var pdfStatus = client.pdf().status("pdf-id");

// Cost tracking
var costs = client.costs().summary();
System.out.println("Total cost: $" + costs.totalCostUsd);
var history = client.costs().history("2024-01-01", "2024-12-31");
var budget = client.costs().budget();
```

## Error Handling

All API errors throw `EdgeQuakeException`:

```java
import io.edgequake.sdk.EdgeQuakeException;

try {
    client.documents().get("nonexistent-id");
} catch (EdgeQuakeException e) {
    System.out.println("Status: " + e.statusCode());       // HTTP status code
    System.out.println("Message: " + e.getMessage());      // Error description
    System.out.println("Body: " + e.responseBody());       // Raw response body
}
```

## Architecture

```
io.edgequake.sdk/
├── EdgeQuakeClient.java           # Main client entry point
├── EdgeQuakeConfig.java           # Builder-pattern configuration
├── EdgeQuakeException.java        # Error type
├── internal/
│   └── HttpHelper.java            # HTTP transport (java.net.http)
├── models/
│   ├── AuthModels.java            # Auth, users, tenants, conversations, folders
│   ├── DocumentModels.java        # Documents, uploads, scans
│   ├── GraphModels.java           # Entities, relationships, graph
│   ├── HealthResponse.java        # Health check response
│   ├── OperationModels.java       # Pipeline, tasks, models, costs, PDF
│   └── QueryModels.java           # Query, chat
└── resources/
    ├── ApiKeyService.java
    ├── AuthService.java
    ├── ChatService.java
    ├── ConversationService.java
    ├── CostService.java
    ├── DocumentService.java
    ├── EntityService.java
    ├── FolderService.java
    ├── GraphService.java
    ├── HealthService.java
    ├── ModelService.java
    ├── PdfService.java
    ├── PipelineService.java
    ├── QueryService.java
    ├── RelationshipService.java
    ├── TaskService.java
    ├── TenantService.java
    ├── UserService.java
    └── WorkspaceService.java
```

## Development

### Build

```bash
mvn compile
```

### Run Tests

```bash
# Unit tests (default, excludes E2E)
mvn test

# E2E tests (requires running EdgeQuake server)
mvn test -Pe2e
```

## License

Apache License 2.0
