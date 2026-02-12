# EdgeQuake Rust SDK

Production-ready async Rust client for the [EdgeQuake RAG API](https://github.com/raphaelmansuy/edgequake).

## Features

- **Async/await** — built on `reqwest` + `tokio`
- **Builder pattern** — fluent client configuration
- **Thread-safe** — `Clone + Send + Sync` via `Arc` internals
- **Automatic retry** — exponential backoff on 429/5xx
- **Multi-tenant** — first-class tenant + workspace headers
- **22 resources** — full API coverage
- **Strong types** — typed request/response structs with `serde`
- **Rich errors** — `thiserror` variants with `status_code()` + `is_retryable()`

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
edgequake-sdk = { git = "https://github.com/raphaelmansuy/edgequake", path = "sdks/rust" }
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use edgequake_sdk::EdgeQuakeClient;

#[tokio::main]
async fn main() -> edgequake_sdk::Result<()> {
    let client = EdgeQuakeClient::builder()
        .base_url("http://localhost:8080")
        .api_key("my-api-key")
        .build()?;

    let health = client.health().check().await?;
    println!("Status: {}", health.status);
    Ok(())
}
```

## Client Configuration

| Method                       | Description                       | Default                    |
| ---------------------------- | --------------------------------- | -------------------------- |
| `.base_url(url)`             | API server URL                    | `http://localhost:8080`    |
| `.api_key(key)`              | API key (X-API-Key header)        | None                       |
| `.bearer_token(token)`       | JWT token (Authorization: Bearer) | None                       |
| `.tenant_id(id)`             | Tenant ID header                  | None                       |
| `.workspace_id(id)`          | Workspace ID header               | None                       |
| `.timeout(duration)`         | Request timeout                   | 30s                        |
| `.connect_timeout(duration)` | Connection timeout                | 5s                         |
| `.max_retries(n)`            | Max retry attempts on 429/5xx     | 3                          |
| `.user_agent(ua)`            | Custom User-Agent string          | `edgequake-rust/{version}` |

```rust
use std::time::Duration;

let client = EdgeQuakeClient::builder()
    .base_url("https://api.example.com")
    .api_key("sk-...")
    .tenant_id("tenant-123")
    .workspace_id("ws-456")
    .timeout(Duration::from_secs(60))
    .max_retries(5)
    .build()?;
```

## API Reference

### Health

```rust
let health = client.health().check().await?;
println!("{} v{}", health.status, health.version.unwrap_or_default());
```

### Documents

```rust
// List documents
let docs = client.documents().list().await?;

// Get document by ID
let doc = client.documents().get("doc-id").await?;

// Upload text content
let body = serde_json::json!({"content": "hello", "title": "test"});
let result = client.documents().upload_text(&body).await?;

// Track processing status
let status = client.documents().track("track-id").await?;
println!("progress: {}", status.progress);

// Status check
let s = client.documents().status("doc-id").await?;

// Delete document
client.documents().delete("doc-id").await?;
```

### Graph

```rust
// Get full graph
let graph = client.graph().get().await?;
println!("nodes: {}, edges: {}", graph.nodes.len(), graph.edges.len());

// Search nodes
let results = client.graph().search("Alice").await?;
```

### Entities

```rust
use edgequake_sdk::types::graph::CreateEntityRequest;

// List entities
let entities = client.entities().list().await?;

// Get entity detail
let entity = client.entities().get("ALICE").await?;

// Create entity
let req = CreateEntityRequest {
    entity_name: "BOB".into(),
    entity_type: "person".into(),
    description: "A person".into(),
    source_id: "manual".into(),
    metadata: None,
};
let result = client.entities().create(&req).await?;

// Merge entities
let merged = client.entities().merge("Alice", "ALICE").await?;

// Delete entity
client.entities().delete("ALICE").await?;
```

### Relationships

```rust
use edgequake_sdk::types::graph::CreateRelationshipRequest;

// List relationships
let rels = client.relationships().list().await?;

// Create relationship
let req = CreateRelationshipRequest {
    source: "Alice".into(),
    target: "Bob".into(),
    relationship_type: "knows".into(),
    weight: Some(0.9),
    description: Some("friends".into()),
};
let rel = client.relationships().create(&req).await?;

// Delete relationship
client.relationships().delete("rel-id").await?;
```

### Query

```rust
use edgequake_sdk::types::query::{QueryRequest, QueryMode};

let req = QueryRequest {
    query: "What is EdgeQuake?".into(),
    mode: Some(QueryMode::Hybrid),
    top_k: Some(5),
    stream: None,
    only_need_context: None,
};
let response = client.query().execute(&req).await?;
println!("Answer: {}", response.answer.unwrap_or_default());
```

### Chat

```rust
use edgequake_sdk::types::chat::{ChatCompletionRequest, ChatMessage};

let req = ChatCompletionRequest {
    messages: vec![ChatMessage {
        role: "user".into(),
        content: "Hello!".into(),
    }],
    model: Some("gpt-4".into()),
    temperature: Some(0.7),
    max_tokens: None,
    stream: None,
};
let response = client.chat().completions(&req).await?;
```

### Auth

```rust
use edgequake_sdk::types::auth::{LoginRequest, RefreshRequest};

// Login
let token = client.auth().login(&LoginRequest {
    username: "admin".into(),
    password: "secret".into(),
}).await?;

// Get current user
let me = client.auth().me().await?;

// Refresh token
let new_token = client.auth().refresh(&RefreshRequest {
    refresh_token: token.refresh_token.unwrap(),
}).await?;
```

### Users

```rust
use edgequake_sdk::types::auth::CreateUserRequest;

let users = client.users().list().await?;

let user = client.users().create(&CreateUserRequest {
    username: "bob".into(),
    email: "bob@example.com".into(),
    password: "secret".into(),
    role: None,
}).await?;

let user = client.users().get("user-id").await?;
client.users().delete("user-id").await?;
```

### API Keys

```rust
let keys = client.api_keys().list().await?;
let key = client.api_keys().create("my-key").await?;
client.api_keys().revoke("key-id").await?;
```

### Tenants

```rust
use edgequake_sdk::types::auth::CreateTenantRequest;

let tenants = client.tenants().list().await?;

let tenant = client.tenants().create(&CreateTenantRequest {
    name: "Acme Corp".into(),
    slug: Some("acme".into()),
}).await?;

let tenant = client.tenants().get("tenant-id").await?;
client.tenants().delete("tenant-id").await?;
```

### Conversations

```rust
use edgequake_sdk::types::conversations::*;

let convos = client.conversations().list().await?;

let convo = client.conversations().create(&CreateConversationRequest {
    title: Some("Discussion".into()),
    folder_id: None,
}).await?;

let detail = client.conversations().get("conv-id").await?;

// Send a message
let msg = client.conversations().create_message("conv-id", &CreateMessageRequest {
    role: "user".into(),
    content: "Hello!".into(),
}).await?;

// Share conversation
let share = client.conversations().share("conv-id").await?;

// Bulk delete
let result = client.conversations().bulk_delete(&["c1".into(), "c2".into()]).await?;

// Pin/unpin
client.conversations().pin("conv-id").await?;
client.conversations().unpin("conv-id").await?;

// Delete
client.conversations().delete("conv-id").await?;
```

### Folders

```rust
use edgequake_sdk::types::conversations::CreateFolderRequest;

let folders = client.folders().list().await?;
let folder = client.folders().create(&CreateFolderRequest {
    name: "Work".into(),
    parent_id: None,
}).await?;
client.folders().delete("folder-id").await?;
```

### Tasks

```rust
let tasks = client.tasks().list().await?;
let task = client.tasks().get("track-id").await?;
client.tasks().cancel("track-id").await?;
```

### Pipeline

```rust
let status = client.pipeline().status().await?;
println!("busy: {}, pending: {}", status.is_busy, status.pending_tasks);

let metrics = client.pipeline().metrics().await?;
println!("queue: {}", metrics.queue_depth);
```

### Costs

```rust
let summary = client.costs().summary().await?;
println!("total: ${:.2}", summary.total_cost_usd);

let history = client.costs().history().await?;
let budget = client.costs().budget().await?;
```

### Chunks

```rust
let chunks = client.chunks().list("doc-id").await?;
let chunk = client.chunks().get("chunk-id").await?;
```

### Provenance

```rust
let records = client.provenance().for_entity("ALICE").await?;
let lineage = client.provenance().lineage("ALICE").await?;
```

### Models

```rust
let catalog = client.models().list().await?;
let provider = client.models().current_provider().await?;
let health = client.models().providers_health().await?;
let status = client.models().set_provider("ollama").await?;
```

### Workspaces

```rust
use edgequake_sdk::types::workspaces::CreateWorkspaceRequest;

let workspaces = client.workspaces().list("tenant-id").await?;

let ws = client.workspaces().create("tenant-id", &CreateWorkspaceRequest {
    name: "production".into(),
    slug: None,
    description: None,
}).await?;

let stats = client.workspaces().stats("ws-id").await?;
```

### PDF

```rust
let progress = client.pdf().progress("track-id").await?;
let content = client.pdf().content("pdf-id").await?;
```

## Error Handling

All methods return `edgequake_sdk::Result<T>`. Errors are strongly typed:

```rust
use edgequake_sdk::Error;

match client.documents().get("doc-id").await {
    Ok(doc) => println!("Found: {}", doc.id),
    Err(Error::NotFound { message }) => println!("Not found: {message}"),
    Err(Error::Unauthorized { message }) => println!("Auth failed: {message}"),
    Err(Error::RateLimited { retry_after, .. }) => {
        println!("Rate limited, retry after {:?}", retry_after);
    }
    Err(e) => {
        // status_code() returns Option<u16>
        println!("Error ({}): {e}", e.status_code().unwrap_or(0));
        // is_retryable() checks if 429/5xx/network
        println!("Retryable: {}", e.is_retryable());
    }
}
```

### Error Variants

| Variant        | HTTP Status | Description                |
| -------------- | ----------- | -------------------------- |
| `BadRequest`   | 400         | Invalid request parameters |
| `Unauthorized` | 401         | Authentication failed      |
| `Forbidden`    | 403         | Permission denied          |
| `NotFound`     | 404         | Resource not found         |
| `Conflict`     | 409         | Resource conflict          |
| `Validation`   | 422         | Validation error           |
| `RateLimited`  | 429         | Rate limit exceeded        |
| `Server`       | 5xx         | Server error               |
| `Network`      | —           | Transport error            |
| `Json`         | —           | Serialization error        |
| `Url`          | —           | URL parsing error          |
| `Config`       | —           | Configuration error        |
| `Timeout`      | —           | Operation timeout          |

## Retry Behavior

The client automatically retries on:

- **429 Too Many Requests**
- **500, 502, 503, 504 Server Errors**
- **Network errors** (connection timeouts, etc.)

Default: 3 retries with 500ms base exponential backoff (500ms → 1s → 2s).

## Testing

```bash
# Run unit + integration tests (uses wiremock, no server needed)
cargo test

# Run E2E tests against a live server
EDGEQUAKE_BASE_URL=http://localhost:8080 cargo test --features e2e
```

## License

Apache-2.0 — see [LICENSE](../../LICENSE).
