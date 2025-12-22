# ADR-0003: Axum as Web Framework

## Status

Accepted

## Date

2024-01

## Context

EdgeQuake needs a high-performance HTTP API framework that supports:

1. **Async/await**: Full async support for non-blocking I/O
2. **Type-safe routing**: Compile-time route verification
3. **Middleware support**: Logging, auth, rate limiting
4. **OpenAPI integration**: Automatic API documentation
5. **WebSocket support**: For streaming responses
6. **Production ready**: Battle-tested in production

## Decision

We chose **Axum 0.8** as the HTTP framework because:

1. **Built on tokio**: Leverages the most mature async runtime
2. **Type-safe extractors**: Request parsing with compile-time safety
3. **Tower middleware**: Composable middleware ecosystem
4. **Excellent performance**: Among the fastest Rust frameworks
5. **Maintained by tokio team**: Long-term maintenance guaranteed
6. **Good documentation**: Comprehensive guides and examples

### Framework Comparison

| Framework | Performance | Type Safety | Middleware | Async | Maturity |
| --------- | ----------- | ----------- | ---------- | ----- | -------- |
| Axum      | ★★★★★       | ★★★★★       | ★★★★★      | ★★★★★ | ★★★★☆    |
| Actix-web | ★★★★★       | ★★★★☆       | ★★★★☆      | ★★★★★ | ★★★★★    |
| Warp      | ★★★★☆       | ★★★★★       | ★★★☆☆      | ★★★★★ | ★★★★☆    |
| Rocket    | ★★★☆☆       | ★★★★★       | ★★★★☆      | ★★★☆☆ | ★★★★☆    |

### Key Axum Features Used

```rust
// Type-safe extractors
async fn handler(
    State(state): State<AppState>,  // Shared state
    Json(body): Json<Request>,       // Validated JSON body
    Path(id): Path<String>,          // Path parameters
) -> impl IntoResponse { ... }

// Tower middleware stack
Router::new()
    .route("/api", get(handler))
    .layer(TraceLayer::new_for_http())
    .layer(CorsLayer::new())
    .layer(CompressionLayer::new())
```

## Consequences

### Positive

- **Type-safe request handling**: Compiler catches handler signature errors
- **Composable middleware**: Reuse Tower ecosystem (timeouts, rate limits)
- **Zero-cost abstractions**: No runtime overhead for type safety
- **Easy testing**: Router can be tested without HTTP
- **Stream responses**: Native support for SSE and WebSockets

### Negative

- **Steeper learning curve**: Tower service traits are complex
- **Compile times**: More type-level computation
- **Breaking changes**: 0.x versions may have breaking changes
- **Less battle-tested**: Newer than Actix-web

### Mitigations

- Team training on Tower concepts
- Pin to specific Axum version
- Comprehensive test coverage for upgrade safety
- Follow Axum changelog for migration guides
