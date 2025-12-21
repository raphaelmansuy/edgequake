# Axum Web Framework Guide

**Version**: 0.8+  
**Category**: Web Framework  
**Use Case**: REST API Layer for LightRAG  
**Official Docs**: https://docs.rs/axum/latest/axum/

---

## Overview

Axum is a web framework built on Tokio and Hyper that focuses on ergonomics and modularity. It leverages the Tower ecosystem for middleware and is designed to be type-safe and performant.

### Core Concepts

1. **Handlers**: Async functions that process requests
2. **Extractors**: Type-safe request data extraction
3. **Responses**: Any type implementing `IntoResponse`
4. **Routers**: Composable routing with `Router`
5. **State**: Shared application state via `State` extractor
6. **Middleware**: Tower layers for cross-cutting concerns

---

## Installation

### Cargo.toml

```toml
[dependencies]
axum = { version = "0.8", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["trace", "compression", "cors"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

---

## Progressive Examples

### 1. Hello World

```rust
use axum::{routing::get, Router};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    axum::serve(listener, app).await.unwrap();
}
```

**Explanation**:
- `Router::new()` creates a new router
- `.route()` maps path to handler
- `get()` specifies HTTP method
- Handler is an async closure returning a string
- String automatically converts to `200 OK` response

### 2. JSON Request/Response

```rust
use axum::{
    routing::post,
    Json,
    Router,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

#[derive(Serialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

async fn create_user(Json(payload): Json<CreateUser>) -> Json<User> {
    let user = User {
        id: 1,
        name: payload.name,
        email: payload.email,
    };
    Json(user)
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/users", post(create_user));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    axum::serve(listener, app).await.unwrap();
}
```

**Key Points**:
- `Json<T>` extractor automatically deserializes request body
- `Json<T>` response automatically serializes with `Content-Type: application/json`
- Serde derives handle serialization

### 3. Shared State

```rust
use axum::{
    extract::State,
    routing::get,
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct AppState {
    counter: Arc<RwLock<u64>>,
}

async fn increment(State(state): State<AppState>) -> String {
    let mut counter = state.counter.write().await;
    *counter += 1;
    format!("Counter: {}", *counter)
}

#[tokio::main]
async fn main() {
    let state = AppState {
        counter: Arc::new(RwLock::new(0)),
    };

    let app = Router::new()
        .route("/increment", get(increment))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    axum::serve(listener, app).await.unwrap();
}
```

**Key Points**:
- `State` extractor provides type-safe access to shared state
- State must implement `Clone`
- Use `Arc` for reference counting and `RwLock`/`Mutex` for interior mutability
- `.with_state()` attaches state to router

### 4. Error Handling (Production Pattern)

```rust
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json, Router,
};
use serde::Serialize;
use thiserror::Error;

// Custom error type
#[derive(Error, Debug)]
enum ApiError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Invalid input: {0}")]
    Validation(String),
}

// Convert ApiError to HTTP response
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Database(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
        };
        
        let body = Json(serde_json::json!({
            "error": message
        }));
        
        (status, body).into_response()
    }
}

// Handler that can fail
async fn get_user(
    State(db): State<Database>,
) -> Result<Json<User>, ApiError> {
    let user = db.find_user(1).await
        .map_err(|e| ApiError::Database(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;
    
    Ok(Json(user))
}
```

**Best Practices**:
- Define custom error types with `thiserror`
- Implement `IntoResponse` for error types
- Use `Result<T, E>` for fallible handlers
- Map internal errors to API errors

### 5. Middleware (Logging & CORS)

```rust
use axum::{
    middleware,
    Router,
};
use tower_http::{
    trace::TraceLayer,
    cors::CorsLayer,
    compression::CompressionLayer,
};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/users", get(get_users))
        // Add middleware
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    axum::serve(listener, app).await.unwrap();
}
```

**Middleware Layers** (applied bottom-to-top):
- `TraceLayer`: Request/response logging
- `CorsLayer`: CORS headers
- `CompressionLayer`: gzip/brotli compression

### 6. Path Parameters

```rust
use axum::{
    extract::Path,
    routing::get,
    Router,
};

async fn get_user(Path(user_id): Path<u64>) -> String {
    format!("User ID: {}", user_id)
}

async fn get_post(Path((user_id, post_id)): Path<(u64, u64)>) -> String {
    format!("User {} - Post {}", user_id, post_id)
}

let app = Router::new()
    .route("/users/:id", get(get_user))
    .route("/users/:user_id/posts/:post_id", get(get_post));
```

**Key Points**:
- `:param` syntax for path parameters
- `Path<T>` extractor deserializes to any type implementing `Deserialize`
- Tuples for multiple parameters

### 7. Query Parameters

```rust
use axum::{
    extract::Query,
    routing::get,
    Router,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Pagination {
    page: Option<u32>,
    per_page: Option<u32>,
}

async fn list_items(Query(pagination): Query<Pagination>) -> String {
    let page = pagination.page.unwrap_or(1);
    let per_page = pagination.per_page.unwrap_or(10);
    
    format!("Page {} - {} items per page", page, per_page)
}

let app = Router::new()
    .route("/items", get(list_items));

// GET /items?page=2&per_page=20
```

---

## Production-Ready Pattern: LightRAG API

### Complete Example

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tower_http::trace::TraceLayer;
use tracing::{info, instrument};

// Error types
#[derive(Error, Debug)]
enum ApiError {
    #[error("LightRAG error: {0}")]
    LightRAG(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ApiError::LightRAG(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

// Request/Response types
#[derive(Deserialize)]
struct InsertRequest {
    content: String,
    #[serde(default)]
    file_path: Option<String>,
}

#[derive(Serialize)]
struct InsertResponse {
    track_id: String,
    status: String,
}

#[derive(Deserialize)]
struct QueryRequest {
    question: String,
    mode: QueryMode,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum QueryMode {
    Naive,
    Local,
    Global,
    Hybrid,
}

#[derive(Serialize)]
struct QueryResponse {
    answer: String,
    sources: Vec<String>,
}

// Application state
struct AppState {
    lightrag: Arc<LightRAG>,
}

// Handlers
#[instrument(skip(state, req))]
async fn insert_document(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InsertRequest>,
) -> Result<Json<InsertResponse>, ApiError> {
    info!(content_len = req.content.len(), "Inserting document");
    
    let track_id = state.lightrag
        .ainsert(&req.content)
        .await
        .map_err(|e| ApiError::LightRAG(e.to_string()))?;
    
    Ok(Json(InsertResponse {
        track_id,
        status: "processing".to_string(),
    }))
}

#[instrument(skip(state, req))]
async fn query(
    State(state): State<Arc<AppState>>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, ApiError> {
    info!(mode = ?req.mode, "Processing query");
    
    let result = state.lightrag
        .aquery(&req.question, req.mode)
        .await
        .map_err(|e| ApiError::LightRAG(e.to_string()))?;
    
    Ok(Json(QueryResponse {
        answer: result.content,
        sources: result.sources,
    }))
}

#[instrument(skip(state))]
async fn get_status(
    State(state): State<Arc<AppState>>,
    Path(track_id): Path<String>,
) -> Result<Json<DocumentStatus>, ApiError> {
    let status = state.lightrag
        .get_status(&track_id)
        .await
        .map_err(|e| ApiError::LightRAG(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Track ID not found: {}", track_id)))?;
    
    Ok(Json(status))
}

// Health check
async fn health() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Initialize LightRAG
    let lightrag = LightRAG::new(Config::from_env())
        .await
        .expect("Failed to initialize LightRAG");
    
    let state = Arc::new(AppState {
        lightrag: Arc::new(lightrag),
    });
    
    // Build router
    let app = Router::new()
        .route("/health", get(health))
        .route("/documents", post(insert_document))
        .route("/documents/:track_id/status", get(get_status))
        .route("/query", post(query))
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    
    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    info!("Server listening on {}", listener.local_addr().unwrap());
    
    axum::serve(listener, app)
        .await
        .unwrap();
}
```

---

## Best Practices (2025)

### Do's

✅ **Use extractors for type safety**
```rust
async fn handler(
    State(state): State<AppState>,
    Json(payload): Json<Request>,
) -> Result<Json<Response>, ApiError>
```

✅ **Implement `IntoResponse` for custom errors**
```rust
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Convert to HTTP response
    }
}
```

✅ **Use `#[instrument]` for tracing**
```rust
#[instrument(skip(sensitive_data))]
async fn handler(data: String) -> Result<(), Error>
```

✅ **Leverage Tower middleware**
```rust
use tower_http::{trace::TraceLayer, cors::CorsLayer};

app.layer(TraceLayer::new_for_http())
   .layer(CorsLayer::permissive())
```

✅ **Use nested routers for organization**
```rust
let api_routes = Router::new()
    .route("/users", get(list_users))
    .route("/users/:id", get(get_user));

let app = Router::new()
    .nest("/api/v1", api_routes);
```

### Don'ts

❌ **Don't use raw `String` for errors**
```rust
// Bad
async fn handler() -> Result<String, String>

// Good
async fn handler() -> Result<Json<Response>, ApiError>
```

❌ **Don't ignore error context**
```rust
// Bad
.map_err(|_| ApiError::Internal)

// Good
.map_err(|e| ApiError::Internal(e.to_string()))
```

❌ **Don't block the async runtime**
```rust
// Bad - blocks thread
std::thread::sleep(Duration::from_secs(1));

// Good - yields to runtime
tokio::time::sleep(Duration::from_secs(1)).await;
```

---

## Testing

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt; // for `oneshot`

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = Router::new()
            .route("/health", get(health));

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
```

### Integration Test with Mock State

```rust
#[tokio::test]
async fn test_insert_document() {
    let mock_rag = Arc::new(MockLightRAG::new());
    let state = Arc::new(AppState { lightrag: mock_rag });
    
    let app = Router::new()
        .route("/documents", post(insert_document))
        .with_state(state);
    
    let request_body = serde_json::json!({
        "content": "Test document"
    });
    
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/documents")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}
```

---

## Official Resources

- **Documentation**: https://docs.rs/axum/latest/axum/
- **GitHub**: https://github.com/tokio-rs/axum
- **Examples**: https://github.com/tokio-rs/axum/tree/main/examples
- **Discord**: Tokio Discord server (#axum channel)

---

## Migration from FastAPI

| FastAPI Concept | Axum Equivalent |
|----------------|-----------------|
| `@app.get("/")` | `Router::new().route("/", get(handler))` |
| `async def handler()` | `async fn handler()` |
| `Request` dependency | `Request` extractor |
| `HTTPException` | Custom error type + `IntoResponse` |
| `Depends()` | `State` extractor or middleware |
| `BackgroundTasks` | `tokio::spawn()` |
| Pydantic models | `serde` derives |

---

**Last Updated**: December 20, 2025  
**Version**: 1.0
