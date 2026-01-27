# OODA Iteration 07: Decide

## Date: 2025-01-27

## Implementation Plan

### Priority 1: Domain Types (multitenancy.rs)

```rust
// 1. Add module-level constants
pub const DEFAULT_EMBEDDING_MODEL: &str = "text-embedding-3-small";
pub const DEFAULT_EMBEDDING_PROVIDER: &str = "openai";
pub const DEFAULT_EMBEDDING_DIMENSION: usize = 1536;

// 2. Add fields to Workspace struct
pub embedding_model: String,
pub embedding_provider: String,
pub embedding_dimension: usize,

// 3. Add auto-detection helpers
fn detect_provider_from_model(model: &str) -> String
fn detect_dimension_from_model(model: &str) -> usize

// 4. Add builder methods
fn with_embedding_model(self, model: &str) -> Self
fn with_embedding_provider(self, provider: &str) -> Self
fn with_embedding_dimension(self, dim: usize) -> Self
```

### Priority 2: CreateWorkspaceRequest

```rust
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub max_documents: Option<usize>,
    // NEW: SPEC-032
    pub embedding_model: Option<String>,
    pub embedding_provider: Option<String>,
    pub embedding_dimension: Option<usize>,
}

impl CreateWorkspaceRequest {
    pub fn new(name: &str) -> Self { ... }
    pub fn with_embedding_model(self, model: &str) -> Self { ... }
}
```

### Priority 3: API DTOs

```rust
// CreateWorkspaceApiRequest - add optional fields
pub embedding_model: Option<String>,
pub embedding_provider: Option<String>,
pub embedding_dimension: Option<usize>,

// WorkspaceResponse - add required fields
pub embedding_model: String,
pub embedding_provider: String,
pub embedding_dimension: usize,
```

### Priority 4: Services

1. `InMemoryWorkspaceService.create_workspace` - apply embedding config
2. `WorkspaceServiceImpl.create_workspace` - store in metadata
3. `WorkspaceRow.into_workspace` - extract from metadata

### Priority 5: Tests

- Update 18 test usages of CreateWorkspaceRequest
- Add embedding field assertions

## Decision Rationale

1. **Backward Compatibility**: Using metadata JSONB means no DB migration needed initially
2. **Optional API Fields**: Clients don't need to send embedding config if defaults are OK
3. **Builder Pattern**: Clean API for tests and internal code
4. **Auto-Detection**: Reduces configuration burden for common models
