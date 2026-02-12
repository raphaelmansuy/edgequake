# EdgeQuake Swift SDK

Swift SDK for the EdgeQuake RAG API. Built with native Swift concurrency (`async/await`) and zero external dependencies.

## Requirements

- Swift 5.9+
- macOS 13+ / iOS 16+ / tvOS 16+ / watchOS 9+

## Installation

### Swift Package Manager

Add to your `Package.swift`:

```swift
dependencies: [
    .package(url: "https://github.com/raphaelmansuy/edgequake.git", from: "1.0.0"),
]
```

Then add `"EdgeQuakeSDK"` to your target dependencies.

## Quick Start

```swift
import EdgeQuakeSDK

let client = EdgeQuakeClient(config: EdgeQuakeConfig(
    baseUrl: "http://localhost:8080",
    apiKey: "sk-your-api-key"
))

// Health check
let health = try await client.health.check()
print("Status: \(health.status ?? "unknown")")

// Upload a document
let upload = try await client.documents.uploadText(
    title: "My Document",
    content: "EdgeQuake is an advanced RAG framework."
)
print("Document ID: \(upload.documentId ?? "?")")

// Query the knowledge graph
let result = try await client.query.execute(query: "What is EdgeQuake?")
print("Answer: \(result.answer ?? "no answer")")
```

## Services

| Service | Description |
|---------|-------------|
| `client.health` | Health check endpoint |
| `client.documents` | Document CRUD and text upload |
| `client.entities` | Entity listing, creation, deletion |
| `client.relationships` | Relationship listing |
| `client.graph` | Full graph retrieval and search |
| `client.query` | RAG query execution |
| `client.chat` | Chat completions |
| `client.tenants` | Multi-tenant management |
| `client.users` | User management |
| `client.apiKeys` | API key management |
| `client.tasks` | Background task tracking |
| `client.pipeline` | Pipeline status and queue metrics |
| `client.models` | Model catalog and provider health |
| `client.costs` | Cost tracking and summaries |

## Configuration

```swift
let config = EdgeQuakeConfig(
    baseUrl: "http://localhost:8080",   // API server URL
    apiKey: "sk-your-key",             // Optional API key
    tenantId: "tenant-1",             // Optional tenant ID
    userId: "user-1",                 // Optional user ID
    workspaceId: "ws-1",             // Optional workspace ID
    timeoutSeconds: 30               // Request timeout (default: 30s)
)
```

## Error Handling

All service methods throw `EdgeQuakeError` on failure:

```swift
do {
    let doc = try await client.documents.get(id: "nonexistent")
} catch let error as EdgeQuakeError {
    print("Status: \(error.statusCode)")   // e.g., 404
    print("Message: \(error.message)")     // e.g., "Not found"
    print("Body: \(error.responseBody ?? "")")
}
```

## Examples

### Document Upload and Query

```swift
// Upload
let upload = try await client.documents.uploadText(
    title: "Research Paper",
    content: longTextContent
)

// Wait for processing, then query
let result = try await client.query.execute(
    query: "Summarize the key findings",
    mode: "hybrid"
)
print(result.answer ?? "")
```

### Graph Exploration

```swift
// List entities
let entities = try await client.entities.list(page: 1, pageSize: 50)
for item in entities.items ?? [] {
    print(item.entityName ?? "unknown")
}

// Search the graph
let search = try await client.graph.search(query: "Alice")
for node in search.nodes ?? [] {
    print(node)
}
```

### Chat Completions

```swift
let req = ChatCompletionRequest(message: "What is RAG?")
let response = try await client.chat.completions(req)
print(response.content ?? "")
```

## Testing

```bash
# Run tests (requires Xcode)
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer swift test

# Build only
swift build
```

## Architecture

- **Zero dependencies** — Uses only `Foundation` (built into Swift)
- **Native async/await** — All service methods are `async throws`
- **Type-safe** — Full `Codable` models for all request/response types
- **Testable** — `HttpHelper` accepts a custom `URLSession` for mocking

## License

Apache 2.0 — See [LICENSE](../../LICENSE) for details.
