# OODA-11: Decide - Health API Enhancement Actions

## Decision

Implement enhanced Health API with full provider configuration visibility.

## Prioritized Changes

| Priority | Change                       | File              | Impact                         |
| -------- | ---------------------------- | ----------------- | ------------------------------ |
| 1        | Add provider health structs  | `health_types.rs` | Enable type-safe serialization |
| 2        | Add fields to HealthResponse | `health_types.rs` | Extend response schema         |
| 3        | Populate fields in handler   | `health.rs`       | Wire up state to response      |
| 4        | Add unit tests               | `health_types.rs` | Verify serialization works     |
| 5        | Verify via curl              | N/A               | Integration test               |

## Specific Changes

### 1. Add provider structs (`health_types.rs`)

```rust
/// LLM provider health information.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LlmProviderHealth {
    /// Provider name (e.g., "openai", "ollama").
    pub name: String,
    /// Model being used (e.g., "gpt-5-nano", "gemma3:latest").
    pub model: String,
}

/// Embedding provider health information.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EmbeddingProviderHealth {
    /// Provider name.
    pub name: String,
    /// Embedding model.
    pub model: String,
    /// Embedding dimension (e.g., 768, 1536, 3072).
    pub dimension: usize,
}

/// Combined provider health.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProvidersHealth {
    /// LLM provider details.
    pub llm: LlmProviderHealth,
    /// Embedding provider details.
    pub embedding: EmbeddingProviderHealth,
}
```

### 2. Add fields to HealthResponse

```rust
pub struct HealthResponse {
    // ... existing fields ...

    /// Provider configuration details (LLM and embedding).
    /// WHY: Mission requirement - "know all parts of the applied configuration"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<ProvidersHealth>,

    /// Whether PDF storage is enabled.
    /// WHY: Operators need to verify PDF processing is available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_storage_enabled: Option<bool>,
}
```

### 3. Update handler (`health.rs`)

```rust
// Get provider details
let providers = Some(ProvidersHealth {
    llm: LlmProviderHealth {
        name: state.llm_provider.name().to_string(),
        model: state.llm_provider.model().to_string(),
    },
    embedding: EmbeddingProviderHealth {
        name: state.embedding_provider.name().to_string(),
        model: state.embedding_provider.model().to_string(),
        dimension: state.embedding_provider.dimension(),
    },
});

let pdf_storage_enabled = Some(state.pdf_storage.is_some());
```

## Success Criteria

- [ ] curl /health shows `providers.llm.model` field
- [ ] curl /health shows `providers.embedding.dimension` field
- [ ] curl /health shows `pdf_storage_enabled` field
- [ ] Tests pass: `cargo test -p edgequake-api`
- [ ] No clippy warnings: `cargo clippy -p edgequake-api`
