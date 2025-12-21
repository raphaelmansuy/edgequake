# OpenAPI/Swagger with utoipa: Automatic API Documentation

**Version**: utoipa 5.0+, utoipa-axum 0.1+  
**Purpose**: Auto-generate OpenAPI 3.0 specs from Rust code

---

## Overview

**utoipa** provides compile-time OpenAPI specification generation for Axum APIs with **zero runtime cost**. The code IS the documentation.

### Why utoipa?

- **Type-Safe**: Compile-time validation
- **Auto-Generated**: Derive macros for types/routes
- **Swagger UI**: Interactive API testing
- **Client Generation**: TypeScript, Python, Rust clients
- **Zero Runtime Cost**: All work done at compile time

---

## Quick Start

### Dependencies

```toml
[dependencies]
utoipa = { version = "5", features = ["axum_extras", "chrono", "uuid"] }
utoipa-axum = "0.1"
utoipa-swagger-ui = { version = "8", features = ["axum"] }
axum = "0.8"
```

### Example

```rust
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

#[derive(ToSchema, Serialize, Deserialize)]
struct InsertRequest {
    /// Document content
    content: String,
}

#[derive(ToSchema, Serialize)]
struct InsertResponse {
    track_id: String,
}

#[utoipa::path(
    post,
    path = "/documents",
    request_body = InsertRequest,
    responses(
        (status = 200, description = "Success", body = InsertResponse),
        (status = 400, description = "Bad request")
    )
)]
async fn insert(Json(req): Json<InsertRequest>) -> Json<InsertResponse> {
    // Implementation
}

#[derive(OpenApi)]
#[openapi(
    info(title = "LightRAG API", version = "1.0.0"),
    paths(insert),
    components(schemas(InsertRequest, InsertResponse))
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(insert))
        .split_for_parts();

    let app = router
        .merge(SwaggerUi::new("/swagger-ui")
            .url("/api-docs/openapi.json", api));

    // Serve at localhost:8000
}
```

**Access Swagger UI**: `http://localhost:8000/swagger-ui`

---

## Resources

- [utoipa Docs](https://docs.rs/utoipa/latest/utoipa/)
- [GitHub](https://github.com/juhaku/utoipa)

---

**Status**: ✅ Production Ready
