# Phase 1: Wire Existing SOTA Features

## Objective

Connect already-implemented SOTA features into the main processing paths.

## Duration: 4-6 hours

---

## Task 1.1: Wire GleaningExtractor into Pipeline

### Current State

- `GleaningExtractor` exists at [extractor.rs#L645](../edgequake/crates/edgequake-pipeline/src/extractor.rs#L645)
- `Pipeline` uses `LLMExtractor` directly without gleaning wrapper

### Changes Required

**File: [edgequake/crates/edgequake-pipeline/src/pipeline.rs](../edgequake/crates/edgequake-pipeline/src/pipeline.rs)**

```rust
// Add to PipelineConfig
pub struct PipelineConfig {
    // ... existing fields ...

    /// Enable gleaning for multi-pass extraction.
    pub enable_gleaning: bool,

    /// Maximum gleaning iterations (default: 1).
    pub max_gleaning: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            enable_gleaning: true,  // Enable by default for SOTA
            max_gleaning: 1,
        }
    }
}

// In Pipeline::new() or build():
fn create_extractor(&self) -> Arc<dyn EntityExtractor> {
    let base_extractor = Arc::new(LLMExtractor::new(...));

    if self.config.enable_gleaning {
        Arc::new(
            GleaningExtractor::new(self.llm_provider.clone(), base_extractor)
                .with_max_gleaning(self.config.max_gleaning)
        )
    } else {
        base_extractor
    }
}
```

### Tests to Add

```rust
#[tokio::test]
async fn test_pipeline_with_gleaning() {
    let config = PipelineConfig {
        enable_gleaning: true,
        max_gleaning: 1,
        ..Default::default()
    };
    let pipeline = Pipeline::new(config, ...);

    let result = pipeline.process("doc1", "Sarah Chen founded OpenAI...").await?;

    // Gleaning should find more entities than without
    assert!(result.entities.len() >= 3);

    // Metadata should show gleaning was used
    assert!(result.metadata.get("gleaning_iterations").is_some());
}
```

---

## Task 1.2: Wire LLMSummarizer into Merger

### Current State

- `LLMSummarizer` exists at [summarizer.rs#L119](../edgequake/crates/edgequake-pipeline/src/summarizer.rs#L119)
- `KnowledgeGraphMerger` uses simple concatenation

### Changes Required

**File: [edgequake/crates/edgequake-pipeline/src/merger.rs](../edgequake/crates/edgequake-pipeline/src/merger.rs)**

```rust
use crate::summarizer::{LLMSummarizer, DescriptionSummarizer, SummarizerConfig};

pub struct MergerConfig {
    // ... existing fields ...

    /// Enable LLM-based description summarization.
    pub use_llm_summarization: bool,

    /// Summarizer configuration.
    pub summarizer_config: SummarizerConfig,
}

pub struct KnowledgeGraphMerger<G, V, L> {
    // ... existing fields ...

    /// Optional LLM summarizer for description merging.
    summarizer: Option<Arc<LLMSummarizer<L>>>,
}

impl<G, V, L> KnowledgeGraphMerger<G, V, L> {
    /// Merge descriptions using configured strategy.
    async fn merge_descriptions(
        &self,
        entity_name: &str,
        existing_desc: &str,
        new_desc: &str,
    ) -> Result<String> {
        if let Some(summarizer) = &self.summarizer {
            // Use LLM to merge intelligently
            let descriptions = vec![existing_desc.to_string(), new_desc.to_string()];
            summarizer.merge_entity_descriptions(entity_name, &descriptions).await
        } else {
            // Simple concatenation fallback
            Ok(format!("{}\n{}", existing_desc, new_desc))
        }
    }
}
```

### Integration Point

In `merge_entity()`:

```rust
async fn merge_entity(&self, entity: ExtractedEntity) -> Result<bool> {
    match existing {
        Some(mut node) => {
            // Use LLM summarizer instead of concatenation
            let merged_desc = self.merge_descriptions(
                &entity.name,
                node.get_property("description").and_then(|v| v.as_str()).unwrap_or(""),
                &entity.description,
            ).await?;

            node.set_property("description", serde_json::json!(merged_desc));
            // ...
        }
        None => { /* new entity */ }
    }
}
```

### Tests to Add

```rust
#[tokio::test]
async fn test_merger_with_llm_summarization() {
    let summarizer = Arc::new(LLMSummarizer::new(mock_llm(), SummarizerConfig::default()));
    let merger = KnowledgeGraphMerger::new(...)
        .with_summarizer(summarizer);

    // First insert
    let entity1 = ExtractedEntity::new("OPENAI", "ORG", "Founded in 2015.");
    merger.merge_entity(entity1).await?;

    // Second insert with different description
    let entity2 = ExtractedEntity::new("OPENAI", "ORG", "Created ChatGPT.");
    merger.merge_entity(entity2).await?;

    // Description should be merged intelligently
    let node = graph.get_node("OPENAI").await?.unwrap();
    let desc = node.get_property("description").unwrap().as_str().unwrap();

    // Should contain both facts without duplication
    assert!(desc.contains("2015") || desc.contains("founded"));
    assert!(desc.contains("ChatGPT") || desc.contains("created"));
    assert!(desc.len() < 300); // Should be summarized, not concatenated
}
```

---

## Task 1.3: Wire Reranker into SOTAQueryEngine

### Current State

- `Reranker` trait exists at [reranker.rs](../edgequake/crates/edgequake-llm/src/reranker.rs)
- `SOTAQueryEngine` doesn't use it

### Changes Required

**File: [edgequake/crates/edgequake-query/src/sota_engine.rs](../edgequake/crates/edgequake-query/src/sota_engine.rs)**

```rust
use edgequake_llm::Reranker;

pub struct SOTAQueryConfig {
    // ... existing fields ...

    /// Enable reranking for improved precision.
    pub enable_rerank: bool,

    /// Minimum rerank score to keep.
    pub min_rerank_score: f32,
}

pub struct SOTAQueryEngine {
    // ... existing fields ...

    /// Optional reranker for result refinement.
    reranker: Option<Arc<dyn Reranker>>,
}

impl SOTAQueryEngine {
    /// Apply reranking to chunks.
    async fn rerank_chunks(
        &self,
        query: &str,
        chunks: Vec<RetrievedChunk>,
    ) -> Result<Vec<RetrievedChunk>> {
        let Some(reranker) = &self.reranker else {
            return Ok(chunks);
        };

        if !self.config.enable_rerank || chunks.is_empty() {
            return Ok(chunks);
        }

        let documents: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();

        let ranked = reranker.rerank(query, &documents, None).await?;

        // Reorder chunks by rerank score
        let mut reordered: Vec<_> = ranked
            .into_iter()
            .filter(|r| r.relevance_score as f32 >= self.config.min_rerank_score)
            .filter_map(|r| chunks.get(r.index).cloned())
            .collect();

        Ok(reordered)
    }
}
```

### Integration Point

In `query()` or `build_context()`:

```rust
async fn query(&self, request: QueryRequest) -> Result<QueryResponse> {
    // ... retrieval ...

    // Apply reranking
    let reranked_chunks = self.rerank_chunks(&request.query, context.chunks).await?;

    // Update context with reranked chunks
    context.chunks = reranked_chunks;

    // ... continue with generation ...
}
```

### Tests to Add

```rust
#[tokio::test]
async fn test_query_with_reranking() {
    let reranker = Arc::new(MockReranker::new());
    let engine = SOTAQueryEngine::builder()
        .with_reranker(reranker)
        .with_config(SOTAQueryConfig { enable_rerank: true, ..Default::default() })
        .build();

    let response = engine.query(QueryRequest {
        query: "What is OpenAI?".to_string(),
        ..Default::default()
    }).await?;

    // Should have reranked chunks
    assert!(!response.context.chunks.is_empty());
}
```

---

## Task 1.4: Add Degree-Based Ranking

### Current State

- `node_degree()` and `node_degrees_batch()` exist
- Query results aren't sorted by degree

### Changes Required

**File: [edgequake/crates/edgequake-query/src/sota_engine.rs](../edgequake/crates/edgequake-query/src/sota_engine.rs)**

```rust
/// Rank entities by graph degree (importance).
async fn rank_by_degree(&self, entities: Vec<RetrievedEntity>) -> Result<Vec<RetrievedEntity>> {
    if entities.is_empty() {
        return Ok(entities);
    }

    let ids: Vec<String> = entities.iter().map(|e| e.id.clone()).collect();

    let degrees = self.graph_storage.node_degrees_batch(&ids).await?;
    let degree_map: HashMap<_, _> = degrees.into_iter().collect();

    let mut ranked: Vec<_> = entities
        .into_iter()
        .map(|e| {
            let degree = degree_map.get(&e.id).copied().unwrap_or(0);
            (e, degree)
        })
        .collect();

    // Sort by degree descending
    ranked.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(ranked.into_iter().map(|(e, _)| e).collect())
}
```

### Integration Point

In mode-specific queries:

```rust
async fn query_local(&self, ...) -> Result<QueryContext> {
    let entities = self.search_entities(...).await?;

    // Rank by degree
    let ranked_entities = self.rank_by_degree(entities).await?;

    // ...
}
```

---

## Verification Checklist

- [ ] `cargo test --package edgequake-pipeline -- gleaning` passes
- [ ] `cargo test --package edgequake-pipeline -- summarizer` passes
- [ ] `cargo test --package edgequake-query -- rerank` passes
- [ ] `cargo test --package edgequake-query -- degree` passes
- [ ] `cargo clippy` clean
- [ ] `cargo fmt --check` clean

---

## Cross-References

- **Next Phase**: [01-phase-2-api-integration.md](01-phase-2-api-integration.md)
- **Current State**: [00-current-state-analysis.md](00-current-state-analysis.md)
