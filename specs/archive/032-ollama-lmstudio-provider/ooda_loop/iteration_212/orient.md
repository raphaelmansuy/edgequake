# OODA Iteration 212 - Orient

## Analysis of Provider Verification Gap

### Current State

The E2E tests in [`e2e_workspace_provider_ingestion.rs`](../../../../edgequake/crates/edgequake-api/tests/e2e_workspace_provider_ingestion.rs) verify:

1. ✅ Workspace configuration is persisted correctly (lines 112-116)
2. ✅ Provider factory can create the configured provider (lines 126-132)
3. ✅ Pipeline is created (non-null Arc) (lines 121-123)

### What's NOT Verified

The tests cannot currently verify:

1. ❌ The **actual extraction** uses the workspace provider
2. ❌ The **actual embedding** uses the workspace provider
3. ❌ When provider fails, fallback behavior is correct

### Root Cause Analysis

The `Pipeline` struct is opaque - we can't inspect which provider it contains:

```rust
// pipeline.rs - providers are private fields
pub struct Pipeline {
    chunker: Arc<dyn Chunker>,
    extractor: Arc<dyn EntityExtractor>,  // ← Can't inspect
    embedding: Arc<dyn EmbeddingProvider>, // ← Can't inspect
}
```

### Possible Solutions

#### Option 1: Add Provider Introspection to Pipeline

```rust
impl Pipeline {
    pub fn llm_provider_name(&self) -> &str;
    pub fn embedding_provider_name(&self) -> &str;
}
```

**Pros**: Direct verification, simple to test
**Cons**: Exposes internal details, not needed in production

#### Option 2: Add Call Counting to Mock Provider

```rust
pub struct MockProvider {
    responses: Arc<Mutex<Vec<String>>>,
    call_count: Arc<AtomicUsize>,  // ← Add tracking
}
```

**Pros**: Verifies actual calls happened
**Cons**: Only works with Mock, complex to verify which provider

#### Option 3: Use Tracing Subscriber to Capture Logs

```rust
#[tokio::test]
async fn test_provider_used() {
    let (tx, rx) = tracing_subscriber::layer::bounded(100);

    // Run test
    // Check for log: "Using workspace-specific providers for document processing"
}
```

**Pros**: Uses existing logging, non-invasive
**Cons**: Brittle (log format changes break tests)

#### Option 4: Add Provider Name to Pipeline Result

```rust
pub struct PipelineResult {
    pub chunks: Vec<Chunk>,
    pub entities: Vec<Entity>,
    pub provider_used: String,  // ← Add this
}
```

**Pros**: Direct verification, useful for lineage
**Cons**: Requires Pipeline API change

### Recommended Approach

Combine options 1 and 4:

1. Add `provider_info()` method to Pipeline for testing
2. Include provider name in `ProviderLineage` struct (already exists!)

The `ProviderLineage` struct already captures this info:

```rust
pub struct ProviderLineage {
    pub extraction_provider: String,
    pub extraction_model: String,
    pub embedding_provider: String,
    pub embedding_model: String,
    pub embedding_dimension: usize,
}
```

### Action Plan (OODA 213)

1. Add a method to Pipeline that returns provider names (for testing only)
2. Create E2E test that:
   - Creates workspace with specific provider
   - Processes a document
   - Verifies the returned/logged provider matches workspace config
3. Use the existing `ProviderLineage` tracking in processor.rs

The key insight is that `processor.rs` already logs the provider lineage at line 460:

```rust
info!(
    extraction_provider = %provider_lineage.extraction_provider,
    extraction_model = %provider_lineage.extraction_model,
    embedding_provider = %provider_lineage.embedding_provider,
    "Processing document with workspace-specific pipeline"
);
```

We can add a tracing subscriber to capture and verify this log.
