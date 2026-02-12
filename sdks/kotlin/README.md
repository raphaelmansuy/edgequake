# EdgeQuake Kotlin SDK

Kotlin SDK for the [EdgeQuake](https://github.com/edgequake/edgequake) RAG (Retrieval-Augmented Generation) API.

## Features

- **Zero HTTP dependencies** — uses `java.net.http.HttpClient` (JDK 11+)
- **19 service clients** — health, documents, entities, relationships, graph, query, chat, auth, users, API keys, tenants, conversations, folders, tasks, pipeline, models, workspaces, PDF, costs
- **Jackson serialization** — type-safe JSON with Kotlin data classes
- **Multi-tenant support** — per-request tenant, user, and workspace headers
- **Lightweight** — ~600 lines of source code

## Requirements

- JDK 17+
- Maven 3.8+ or Gradle 8+

## Installation

### Maven

```xml
<dependency>
    <groupId>io.edgequake</groupId>
    <artifactId>edgequake-sdk-kotlin</artifactId>
    <version>0.1.0</version>
</dependency>
```

### Gradle (Kotlin DSL)

```kotlin
implementation("io.edgequake:edgequake-sdk-kotlin:0.1.0")
```

## Quick Start

```kotlin
import io.edgequake.sdk.EdgeQuakeClient
import io.edgequake.sdk.EdgeQuakeConfig

fun main() {
    val client = EdgeQuakeClient(
        EdgeQuakeConfig(
            baseUrl = "http://localhost:8080",
            apiKey = "your-api-key"
        )
    )

    // Health check
    val health = client.health.check()
    println("Status: ${health.status}")

    // Upload a document
    val upload = client.documents.uploadText(
        title = "My Document",
        content = "EdgeQuake is a graph-based RAG framework."
    )
    println("Document ID: ${upload.documentId}")

    // Query the knowledge graph
    val result = client.query.execute("What is EdgeQuake?")
    println("Answer: ${result.answer}")
}
```

## Configuration

```kotlin
val config = EdgeQuakeConfig(
    baseUrl = "http://localhost:8080",   // API base URL
    apiKey = "sk-your-key",             // Optional API key
    tenantId = "tenant-1",             // Optional tenant ID
    userId = "user-1",                 // Optional user ID
    workspaceId = "workspace-1",       // Optional workspace ID
    timeoutSeconds = 30                // Request timeout (default: 30s)
)

val client = EdgeQuakeClient(config)
```

## Services

### Documents

```kotlin
// List documents
val docs = client.documents.list(page = 1, pageSize = 20)
println("Total: ${docs.total}")

// Get a specific document
val doc = client.documents.get("document-id")

// Upload text content
val upload = client.documents.uploadText("Title", "Content here...")

// Scan a directory
val scan = client.documents.scan("/path/to/docs", recursive = true)

// Delete a document
client.documents.delete("document-id")
```

### Entities

```kotlin
// List entities
val entities = client.entities.list(page = 1, pageSize = 50)

// Get entity details
val entity = client.entities.get("ENTITY_NAME")

// Create an entity
val created = client.entities.create(
    CreateEntityRequest(
        entityName = "NEW_ENTITY",
        entityType = "CONCEPT",
        description = "A new entity",
        sourceId = "doc-1"
    )
)

// Check if entity exists
val exists = client.entities.exists("ENTITY_NAME")

// Merge entities
client.entities.merge("SOURCE_ENTITY", "TARGET_ENTITY")

// Delete an entity
client.entities.delete("ENTITY_NAME")
```

### Knowledge Graph

```kotlin
// Get the full graph
val graph = client.graph.get()
println("Nodes: ${graph.nodes?.size}, Edges: ${graph.edges?.size}")

// Search nodes
val results = client.graph.search("machine learning")

// List relationships
val rels = client.relationships.list(page = 1, pageSize = 100)
```

### Query

```kotlin
// Hybrid query (default)
val answer = client.query.execute("What are the main concepts?")

// Local-only query
val local = client.query.execute("Explain RAG", mode = "local")

// Global query
val global = client.query.execute("Summarize everything", mode = "global")
```

### Chat

```kotlin
import io.edgequake.sdk.models.ChatCompletionRequest
import io.edgequake.sdk.models.ChatMessage

val response = client.chat.completions(
    ChatCompletionRequest(
        messages = listOf(
            ChatMessage("system", "You are a helpful assistant."),
            ChatMessage("user", "What is EdgeQuake?")
        ),
        model = "default"
    )
)
println(response.choices?.first()?.message?.content)
```

### Authentication

```kotlin
// Login
val token = client.auth.login("username", "password")
println("Token: ${token.token}")

// List users
val users = client.users.list()

// Manage API keys
val keys = client.apiKeys.list()

// Multi-tenant
val tenants = client.tenants.list()
```

### Conversations

```kotlin
// List conversations
val convos = client.conversations.list()

// Create a conversation
val convo = client.conversations.create("My Chat")

// Get conversation with messages
val detail = client.conversations.get(convo.id!!)

// Delete a conversation
client.conversations.delete(convo.id!!)

// Bulk delete
val result = client.conversations.bulkDelete(listOf("id1", "id2"))
```

### Pipeline & Tasks

```kotlin
// Pipeline status
val status = client.pipeline.status()
println("Busy: ${status.isBusy}, Pending: ${status.pendingTasks}")

// Queue metrics
val metrics = client.pipeline.queueMetrics()
println("Workers: ${metrics.activeWorkers}/${metrics.maxWorkers}")

// List tasks
val tasks = client.tasks.list()

// Get task details
val task = client.tasks.get("task-id")
```

### Models & Providers

```kotlin
// Available models
val catalog = client.models.catalog()
catalog.providers?.forEach { provider ->
    println("${provider.name}: ${provider.models?.size} models")
}

// Provider health
val health = client.models.health()

// Provider status
val providerStatus = client.models.providerStatus()
```

### Additional Services

```kotlin
// Folders
val folders = client.folders.list()
val newFolder = client.folders.create("Research")
client.folders.delete(newFolder.id!!)

// Workspaces
val workspaces = client.workspaces.list()

// PDF processing
val progress = client.pdf.progress("track-id")
val content = client.pdf.content("pdf-id")

// Cost tracking
val costs = client.costs.summary()
println("Total cost: $${costs.totalCost}")
```

## Error Handling

All API errors throw `EdgeQuakeException`:

```kotlin
import io.edgequake.sdk.EdgeQuakeException

try {
    client.documents.get("nonexistent-id")
} catch (e: EdgeQuakeException) {
    println("Status: ${e.statusCode}")      // HTTP status code
    println("Message: ${e.message}")        // Error description
    println("Body: ${e.responseBody}")      // Raw response body
}
```

## Architecture

```
io.edgequake.sdk/
├── EdgeQuakeClient.kt          # Main client entry point
├── EdgeQuakeConfig.kt          # Configuration data class
├── EdgeQuakeException.kt       # Error type
├── internal/
│   └── HttpHelper.kt           # HTTP transport (java.net.http)
├── models/
│   └── Models.kt               # Request/response data classes
└── resources/
    └── Services.kt             # 19 service implementations
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

### Lint

```bash
mvn compile -q  # Kotlin compiler checks
```

## License

MIT
