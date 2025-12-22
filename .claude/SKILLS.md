# Claude Skills - EdgeQuake Development

## 🎯 Mission-Critical Patterns

### 1. Rust Trait Object Casting

**Problem:** Methods defined in multiple traits cause ambiguity when called on concrete types.

**Solution:** Cast to trait objects explicitly:

```rust
// Given: Arc<OpenAIProvider> that implements both LLMProvider and EmbeddingProvider
let provider = Arc::new(OpenAIProvider::new(api_key));

// Cast to specific trait objects to resolve ambiguity
let llm: Arc<dyn LLMProvider> = provider.clone();
let embedding: Arc<dyn EmbeddingProvider> = provider.clone();

// Now can call trait methods unambiguously
println!("LLM: {} ({})", llm.name(), llm.model());
println!("Embedding: {} ({})", embedding.name(), embedding.model());
```

**When to use:**

- Multiple traits define methods with same signatures
- Need to call trait methods on Arc<ConcreteType>
- Avoid "multiple applicable items in scope" errors

### 2. Environment-Based Provider Factory Pattern

**Pattern:** Automatically select real vs mock provider based on environment.

```rust
async fn create_llm_provider() -> (Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>) {
    // Check for API key in environment
    if let Ok(api_key) = env::var("OPENAI_API_KEY") {
        if !api_key.is_empty() && api_key != "test-key" {
            println!("🔑 Using REAL OpenAI provider");
            let provider = Arc::new(
                OpenAIProvider::new(api_key)
                    .with_model("gpt-4o-mini")
                    .with_embedding_model("text-embedding-3-small")
            );
            return (provider.clone(), provider);
        }
    }

    // Fallback to mock for testing
    println!("🔧 Using Smart Mock provider");
    let mock = create_smart_mock_provider().await;
    (mock.clone(), mock)
}
```

**Benefits:**

- Zero code changes between dev and prod
- CI/CD friendly (no API key = automatic mock)
- Cost-effective testing
- Production validation possible
- Backward compatible

### 3. Smart Mock Provider Design

**Problem:** Basic mocks return empty data causing test failures.

**Solution:** Create smart mocks that return valid, realistic data:

```rust
async fn create_smart_mock_provider() -> Arc<MockProvider> {
    let mut mock = MockProvider::new();

    // Pre-configure with valid extraction responses
    mock.add_response(serde_json::json!({
        "entities": [
            {
                "entity_name": "SARAH_CHEN",
                "entity_type": "PERSON",
                "description": "Project lead and architect",
                "importance_score": 9.0
            },
            // ... more entities
        ],
        "relationships": [
            {
                "src_id": "SARAH_CHEN",
                "tgt_id": "EDGEQUAKE",
                "description": "leads the development",
                "weight": 1.0,
                "order": 2
            },
            // ... more relationships
        ]
    }));

    Arc::new(mock)
}
```

**Key principles:**

- Return valid JSON matching expected schema
- Use realistic entity names (normalized: UPPERCASE_UNDERSCORE)
- Include proper entity types and descriptions
- Provide relationships with proper structure
- Match production data patterns

### 4. Entity Normalization Pattern

**Rule:** All entity names must be UPPERCASE with underscores.

```rust
fn normalize_entity_name(name: &str) -> String {
    name.trim()
        .to_uppercase()
        .replace(" ", "_")
        .replace("-", "_")
        .replace(".", "")
}

// Examples:
// "Sarah Chen" → "SARAH_CHEN"
// "EdgeQuake Framework" → "EDGEQUAKE_FRAMEWORK"
// "Dr. Michael Torres" → "DR_MICHAEL_TORRES"
```

**Why:**

- Consistent entity identification
- Enables proper deduplication
- Matches LightRAG algorithm
- Case-insensitive comparison
- Database-friendly identifiers

### 5. E2E Test Structure Pattern

**Pattern:** Build comprehensive tests that validate entire pipeline.

```rust
#[tokio::test]
async fn test_full_e2e_pipeline() {
    // 1. Setup: Create providers and storage
    let (llm, embedding) = create_llm_provider().await;
    let storage = create_storage_backend().await;

    // 2. Initialize: Setup EdgeQuake
    let eq = EdgeQuake::new(namespace, llm, embedding, storage, config).await?;

    // 3. Ingest: Add documents
    for doc in documents {
        eq.insert_document(doc).await?;
    }

    // 4. Verify: Check entity extraction
    assert!(entities_extracted > 0);

    // 5. Query: Test graph operations
    let neighbors = storage.get_neighbors("ENTITY", 1).await?;
    assert!(neighbors.len() > 0);

    // 6. Validate: Check deduplication
    assert!(unique_nodes < total_entities); // Deduplication worked
}
```

**Components:**

- Setup (providers, storage)
- Initialization (EdgeQuake config)
- Ingestion (document processing)
- Verification (entity extraction)
- Query (graph operations)
- Validation (quality checks)

### 6. LLM Cost Optimization

**Model Selection:**

| Model         | Input ($/1M) | Output ($/1M) | Use Case                     |
| ------------- | ------------ | ------------- | ---------------------------- |
| gpt-4o-mini   | $0.150       | $0.600        | **Production (recommended)** |
| gpt-4o        | $2.50        | $10.00        | High-quality extraction      |
| gpt-3.5-turbo | $0.50        | $1.50         | Budget option                |

**Cost per document (gpt-4o-mini):**

- Extraction: ~$0.0012 (800 input + 200 output tokens)
- Embedding: ~$0.0002 (1000 tokens)
- **Total: ~$0.0014 per document**

**Scaling:**

- 1K documents: ~$1.40
- 10K documents: ~$14.00
- 100K documents: ~$140.00

**Optimization tips:**

1. Cache entity extractions
2. Batch API calls
3. Use streaming for large documents
4. Implement rate limiting
5. Monitor usage with metrics

### 7. Async Rust Testing Patterns

**Key patterns:**

```rust
// Use #[tokio::test] for async tests
#[tokio::test]
async fn test_async_operation() {
    let result = async_function().await.unwrap();
    assert_eq!(result, expected);
}

// Arc<T> for shared ownership across async boundaries
let provider = Arc::new(OpenAIProvider::new(api_key));
let cloned = provider.clone(); // Cheap clone of Arc

// Result<T, E> for error handling
async fn fallible_operation() -> Result<Value, Error> {
    let result = operation().await?;
    Ok(result)
}

// Trait objects for dynamic dispatch
let storage: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new());
```

### 8. Documentation Best Practices

**Structure for production docs:**

```markdown
# Feature Name

## Quick Start

[3-5 line example]

## Overview

[What it is, why it matters]

## Prerequisites

[Dependencies, requirements]

## Configuration

[Environment variables, settings]

## Usage Examples

[Code samples with explanations]

## Cost/Performance

[Numbers, benchmarks]

## Troubleshooting

[Common issues, solutions]

## Best Practices

[Dos and don'ts]

## Next Steps

[What to do after]
```

**Key principles:**

- Start with quick start (copy-paste ready)
- Include real code examples
- Provide cost/performance data
- Add troubleshooting section
- Keep examples up-to-date
- Use concrete numbers

### 9. Cargo Test Workflow

**Commands:**

```bash
# Run all tests (uses mock provider)
cargo test

# Run with real OpenAI provider
export OPENAI_API_KEY="sk-..."
cargo test

# Run specific test with output
cargo test --package edgequake-core --test e2e_pipeline test_name -- --nocapture

# Run example
cargo run --example production_pipeline

# Lint before committing
cargo clippy

# Format code
cargo fmt
```

**Test organization:**

- Unit tests: In `src/` files with `#[cfg(test)]`
- Integration tests: In `tests/` directory
- E2E tests: In `crates/*/tests/`
- Examples: In `examples/` directory

### 10. Git Commit Messages

**Pattern:**

```
<type>: <subject>

<body>

<footer>
```

**Types:**

- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation
- `test:` Test additions
- `refactor:` Code restructuring
- `perf:` Performance improvement
- `chore:` Maintenance

**Example:**

```
feat: Add production LLM integration with OpenAI

- Implement environment-based provider factory
- Add smart mock provider with valid JSON responses
- Create production example demonstrating full workflow
- Add comprehensive documentation (900+ lines)
- Validate with real OpenAI API (30s, 20 entities)

Closes #123
Cost: $0.0014 per document (gpt-4o-mini)
```

## 🔧 EdgeQuake-Specific Knowledge

### Pipeline Flow

```
Document → Chunks → LLM Extraction → Normalization → Merge → Storage
```

### Entity Extraction JSON Schema

```json
{
  "entities": [
    {
      "entity_name": "ENTITY_NAME",
      "entity_type": "TYPE",
      "description": "Description",
      "importance_score": 0.0-10.0
    }
  ],
  "relationships": [
    {
      "src_id": "SOURCE",
      "tgt_id": "TARGET",
      "description": "relationship description",
      "weight": 0.0-1.0,
      "order": 1
    }
  ]
}
```

### Storage Adapters

- **Memory**: Fast, ephemeral (dev/test)
- **PostgreSQL AGE**: Graph DB (production)
- **Vector**: Embeddings storage

### Configuration Patterns

```rust
EdgeQuakeConfig {
    chunk_size: 1200,        // tokens per chunk
    chunk_overlap: 100,       // overlap between chunks
    max_async: 4,            // concurrent operations
    entity_extraction_prompt: "...",
    relationship_extraction_prompt: "...",
}
```

## 🎓 Key Learnings

### 1. Quality Metrics

- Real LLM extracts 2-3x more entities than mock
- Entity deduplication: 30-40% reduction typical
- Multi-hop relationships: Essential for graph quality

### 2. Cost-Quality Tradeoff

- gpt-4o-mini: Best cost/quality ratio for production
- gpt-4o: 10x more expensive, marginally better
- Mock: Free but low quality (testing only)

### 3. Testing Strategy

- Mock for CI/CD: Fast, free, reliable
- Real LLM for validation: Slow, costly, accurate
- Environment-based selection: Best of both

### 4. Production Readiness Checklist

- ✅ Environment-based provider selection
- ✅ Real API validation
- ✅ Cost analysis
- ✅ Documentation (900+ lines)
- ✅ Working examples
- ✅ Error handling
- ✅ Backward compatibility

### 5. Common Pitfalls

- **Trait ambiguity**: Cast to trait objects explicitly
- **Empty mock responses**: Use smart mocks with valid data
- **Entity normalization**: Always UPPERCASE_UNDERSCORE
- **Missing imports**: Import trait for trait methods
- **API key management**: Never commit, use .env

## 🚀 Future Enhancements

### Immediate

1. Anthropic provider (Claude)
2. Rate limiting middleware
3. Cost tracking/monitoring
4. Batch processing

### Medium-Term

1. Multiple model support
2. Model selection by document type
3. Response caching
4. Streaming extraction

### Long-Term

1. Fine-tuned models
2. Multi-modal support (PDFs, images)
3. Federated learning
4. Incremental graph updates

## 📚 References

### Documentation

- `docs/production-llm-integration.md` - Complete guide
- `docs/PRODUCTION_READY.md` - Mission summary
- `AGENTS.md` - Repository guidelines

### Code Examples

- `examples/production_pipeline.rs` - Production demo
- `tests/e2e_pipeline.rs` - E2E tests

### External

- [OpenAI Pricing](https://openai.com/pricing)
- [Rust Async Book](https://rust-lang.github.io/async-book/)
- [Apache AGE Docs](https://age.apache.org/)

---

**Last Updated:** 2025-01-22  
**Version:** 1.0  
**Status:** Production Ready ✅
