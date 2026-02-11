# OODA Iteration 12 — Decide: Rust SDK

## Decision

Implement full Rust SDK with:

- Builder pattern for client construction
- 21 resource modules covering all EdgeQuake API endpoints
- 9 type modules with serde Serialize/Deserialize
- Comprehensive error handling with HTTP status mapping
- wiremock-based integration tests

## Rationale

- Rust SDK is critical for ecosystem completeness (systems programming use case)
- Builder pattern is idiomatic and ergonomic for Rust
- Borrowed resource handles (`&'a`) avoid unnecessary Arc overhead
- wiremock provides excellent HTTP mock testing without external servers

## File Structure

```
sdks/rust/
├── Cargo.toml
├── src/
│   ├── lib.rs              (public API surface)
│   ├── client.rs           (EdgeQuakeClient + builder)
│   ├── config.rs           (ClientConfig, Auth, TenantContext)
│   ├── error.rs            (Error enum, Result alias)
│   ├── types/
│   │   ├── mod.rs
│   │   ├── common.rs       (HealthResponse, PaginationInfo)
│   │   ├── documents.rs    (UploadDocumentResponse, etc.)
│   │   ├── graph.rs        (Entity, Relationship, GraphNode)
│   │   ├── query.rs        (QueryRequest, QueryResponse)
│   │   ├── chat.rs         (ChatMessage, ChatCompletionRequest)
│   │   ├── auth.rs         (LoginRequest, TokenResponse, UserInfo)
│   │   ├── conversations.rs (ConversationInfo, Message, ShareLink)
│   │   ├── operations.rs   (TaskInfo, PipelineStatus, CostSummary)
│   │   └── workspaces.rs   (WorkspaceInfo, WorkspaceStats)
│   └── resources/
│       ├── mod.rs
│       ├── health.rs, documents.rs, graph.rs, entities.rs
│       ├── relationships.rs, query.rs, chat.rs, auth.rs
│       ├── users.rs, api_keys.rs, tenants.rs, conversations.rs
│       ├── folders.rs, tasks.rs, pipeline.rs, costs.rs
│       ├── chunks.rs, provenance.rs, models.rs, workspaces.rs
│       └── pdf.rs
└── tests/
    └── integration_tests.rs (54 tests)
```
