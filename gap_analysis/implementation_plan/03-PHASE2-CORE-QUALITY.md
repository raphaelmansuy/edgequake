# Phase 2: Core Quality Enhancement

**Document ID:** 03-PHASE2-CORE-QUALITY  
**Priority:** 🟠 P1 HIGH  
**Effort:** 14 person-days  
**Duration:** Weeks 4-6  
**Dependencies:** [01-PHASE1-QUERY-ENGINE.md](./01-PHASE1-QUERY-ENGINE.md) (GAP-007 enables GAP-001)  
**Blocks:** None

---

## 📋 Overview

This document provides implementation guidance for core quality features that improve the accuracy and usefulness of EdgeQuake's RAG pipeline: entity deduplication, description summarization, keyword extraction, reranking, and token budget management.

### Gaps Addressed

| Gap ID      | Feature                    | Severity | Status         | Effort |
| ----------- | -------------------------- | -------- | -------------- | ------ |
| **GAP-005** | Entity Deduplication       | 🟠 P1    | 🔲 Not started | 3 days |
| **GAP-006** | Description Summarization  | 🟠 P1    | 🔲 Not started | 3 days |
| **GAP-007** | Keyword Extraction (HL/LL) | 🟠 P1    | 🔲 Not started | 2 days |
| **GAP-008** | Reranking Integration      | 🟠 P1    | 🔲 Not started | 4 days |
| **GAP-009** | Token Budget Management    | 🟠 P1    | 🔲 Not started | 2 days |

### Cross-References

- **Source Analysis:** [../gap-analysis.md](../gap-analysis.md#feature-f-010-entity-deduplication)
- **Master Plan:** [00-INDEX.md](./00-INDEX.md#phase-2-enhancement-weeks-4-6)
- **Query Engine (uses keywords):** [01-PHASE1-QUERY-ENGINE.md](./01-PHASE1-QUERY-ENGINE.md)
- **Testing Plan:** [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md#phase-2-validation)

---

## 🎯 Keyword Extraction

### 1.1 Objective

Implement LLM-based keyword extraction that identifies high-level (conceptual) and low-level (specific) keywords from queries, enabling global query mode.

### 1.2 Source Reference

**Location:** `lightrag/operate.py` with LLM prompt  
**Prompt Source:** `lightrag/prompt.py` - `PROMPTS["keywords_extraction"]`

### 1.3 Implementation

> **Note:** Core implementation is in [01-PHASE1-QUERY-ENGINE.md#task-132-create-keyword-extractor](./01-PHASE1-QUERY-ENGINE.md#task-132-create-keyword-extractor)

This section covers additional enhancements and testing.

#### Task 1.3.1: Add Caching to Keyword Extractor

**File:** `edgequake/crates/edgequake-core/src/keyword_extractor.rs`

```rust
// ENHANCE KeywordExtractor with caching

use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct KeywordExtractor {
    llm: Arc<dyn LLMProvider>,
    cache: RwLock<HashMap<String, ExtractedKeywords>>,
    cache_enabled: bool,
}

impl KeywordExtractor {
    pub fn new(llm: Arc<dyn LLMProvider>) -> Self {
        Self {
            llm,
            cache: RwLock::new(HashMap::new()),
            cache_enabled: true,
        }
    }

    pub fn with_cache(mut self, enabled: bool) -> Self {
        self.cache_enabled = enabled;
        self
    }

    pub async fn extract(&self, query: &str) -> Result<ExtractedKeywords> {
        // Check cache first
        if self.cache_enabled {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(query) {
                tracing::debug!(query = %query, "Keyword cache hit");
                return Ok(cached.clone());
            }
        }

        // Extract keywords via LLM
        let prompt = self.build_extraction_prompt(query);
        let response = self.llm.complete(&prompt).await
            .map_err(|e| crate::error::Error::internal(format!("LLM error: {}", e)))?;

        let keywords = self.parse_keywords(&response.content)?;

        // Store in cache
        if self.cache_enabled {
            let mut cache = self.cache.write().await;
            cache.insert(query.to_string(), keywords.clone());
        }

        Ok(keywords)
    }

    /// Clear the keyword cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}
```

### 1.4 Keyword Extraction Checklist

- [ ] KeywordExtractor implemented
- [ ] LLM prompt ported from LightRAG
- [ ] JSON response parsing handles edge cases
- [ ] Caching implemented
- [ ] Unit tests pass
- [ ] Integration with global query mode

---

## 🎯 Entity Deduplication

### 2.1 Objective

Implement intelligent entity deduplication that merges duplicate entities by name, combining their descriptions using LLM summarization.

### 2.2 Source Reference

**Location:** `lightrag/operate.py:merge_nodes_and_edges()` (lines 1700-2000)

**LightRAG Behavior:**

1. Normalize entity names (UPPERCASE with underscores)
2. Detect duplicates by normalized name
3. Merge descriptions using LLM summarization
4. Combine source_ids from all merged entities
5. Track merge statistics

### 2.3 Current State

**File:** `edgequake/crates/edgequake-pipeline/src/merger.rs`

**Status:** Basic upsert exists but lacks:

- LLM-based description merging
- Source ID tracking
- Proper deduplication statistics

### 2.4 Implementation Tasks

#### Task 2.4.1: Enhance Entity Merger

**File:** `edgequake/crates/edgequake-pipeline/src/merger.rs`

```rust
// ENHANCE merge_entity in merger.rs

use crate::summarizer::LLMSummarizer;

impl KnowledgeGraphMerger {
    /// Merge an entity into the knowledge graph with deduplication
    pub async fn merge_entity(
        &self,
        entity: &ExtractedEntity,
        source_id: &str,
    ) -> Result<MergeResult> {
        let normalized_name = self.normalize_entity_name(&entity.name);

        // Check for existing entity
        let existing = self.graph_storage.get_node(&normalized_name).await?;

        match existing {
            Some(existing_node) => {
                // Entity exists - merge descriptions
                let existing_desc = existing_node.properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let existing_source_ids = existing_node.properties
                    .get("source_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // Check if descriptions are different enough to merge
                let merged_description = if self.should_merge_descriptions(existing_desc, &entity.description) {
                    self.merge_descriptions(
                        &normalized_name,
                        existing_desc,
                        &entity.description,
                    ).await?
                } else {
                    // Keep existing or new based on length (prefer more detailed)
                    if entity.description.len() > existing_desc.len() {
                        entity.description.clone()
                    } else {
                        existing_desc.to_string()
                    }
                };

                // Merge source IDs
                let merged_source_ids = self.merge_source_ids(existing_source_ids, source_id);

                // Update node
                let updated_properties = serde_json::json!({
                    "name": normalized_name,
                    "entity_type": entity.entity_type,
                    "description": merged_description,
                    "source_id": merged_source_ids,
                    "updated_at": chrono::Utc::now().timestamp(),
                });

                self.graph_storage
                    .upsert_node(&normalized_name, updated_properties.as_object().unwrap().clone())
                    .await?;

                Ok(MergeResult::Merged {
                    entity_id: normalized_name,
                    descriptions_merged: true,
                })
            }
            None => {
                // New entity - insert
                let properties = serde_json::json!({
                    "name": normalized_name,
                    "entity_type": entity.entity_type,
                    "description": entity.description,
                    "source_id": source_id,
                    "created_at": chrono::Utc::now().timestamp(),
                });

                self.graph_storage
                    .upsert_node(&normalized_name, properties.as_object().unwrap().clone())
                    .await?;

                Ok(MergeResult::Inserted {
                    entity_id: normalized_name,
                })
            }
        }
    }

    /// Normalize entity name: UPPERCASE with underscores
    fn normalize_entity_name(&self, name: &str) -> String {
        name.trim()
            .to_uppercase()
            .replace(' ', "_")
            .replace('-', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    }

    /// Check if two descriptions are different enough to warrant merging
    fn should_merge_descriptions(&self, desc1: &str, desc2: &str) -> bool {
        if desc1.is_empty() || desc2.is_empty() {
            return false;
        }

        // Simple heuristic: merge if descriptions are significantly different
        let words1: std::collections::HashSet<_> = desc1.split_whitespace().collect();
        let words2: std::collections::HashSet<_> = desc2.split_whitespace().collect();

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        if union == 0 {
            return false;
        }

        let jaccard = intersection as f64 / union as f64;

        // Merge if less than 70% overlap (i.e., descriptions are sufficiently different)
        jaccard < 0.7
    }

    /// Merge two descriptions using LLM summarization
    async fn merge_descriptions(
        &self,
        entity_name: &str,
        desc1: &str,
        desc2: &str,
    ) -> Result<String> {
        let summarizer = LLMSummarizer::new(Arc::clone(&self.llm_provider));

        summarizer.merge_entity_descriptions(
            entity_name,
            &[desc1.to_string(), desc2.to_string()],
        ).await
    }

    /// Merge source IDs, maintaining FIFO limit
    fn merge_source_ids(&self, existing: &str, new_id: &str) -> String {
        const SEPARATOR: &str = "<SEP>";
        const MAX_SOURCE_IDS: usize = 10;

        let mut ids: Vec<&str> = existing
            .split(SEPARATOR)
            .filter(|s| !s.is_empty())
            .collect();

        if !ids.contains(&new_id) {
            ids.push(new_id);
        }

        // Apply FIFO limit
        if ids.len() > MAX_SOURCE_IDS {
            ids = ids.into_iter().skip(ids.len() - MAX_SOURCE_IDS).collect();
        }

        ids.join(SEPARATOR)
    }
}

/// Result of a merge operation
#[derive(Debug)]
pub enum MergeResult {
    Inserted { entity_id: String },
    Merged { entity_id: String, descriptions_merged: bool },
    Skipped { reason: String },
}
```

---

## 🎯 Description Summarization

### 3.1 Objective

Implement map-reduce style LLM summarization for merging multiple entity/relationship descriptions.

### 3.2 Source Reference

**Location:** `lightrag/operate.py:_handle_entity_relation_summary()` (lines 1400-1600)

**LightRAG Behavior:**

1. Check if descriptions exceed token limit
2. Apply map-reduce for long descriptions
3. Cache LLM responses
4. Respect `force_llm_summary_on_merge` threshold

### 3.3 Implementation Tasks

#### Task 3.3.1: Create LLM Summarizer

**File:** `edgequake/crates/edgequake-pipeline/src/summarizer.rs` (NEW/ENHANCE)

```rust
// ENHANCE or CREATE: edgequake/crates/edgequake-pipeline/src/summarizer.rs

use edgequake_llm::traits::LLMProvider;
use std::sync::Arc;

/// LLM-based summarization with map-reduce support
pub struct LLMSummarizer {
    llm: Arc<dyn LLMProvider>,
    max_tokens_per_chunk: usize,
    force_summary_threshold: usize,
}

impl LLMSummarizer {
    pub fn new(llm: Arc<dyn LLMProvider>) -> Self {
        Self {
            llm,
            max_tokens_per_chunk: 4000,
            force_summary_threshold: 4, // Force LLM summary if >= 4 descriptions
        }
    }

    /// Merge multiple entity descriptions into a coherent summary
    pub async fn merge_entity_descriptions(
        &self,
        entity_name: &str,
        descriptions: &[String],
    ) -> Result<String, Error> {
        if descriptions.is_empty() {
            return Ok(String::new());
        }

        if descriptions.len() == 1 {
            return Ok(descriptions[0].clone());
        }

        // Check if we need LLM summarization
        let total_length: usize = descriptions.iter().map(|d| d.len()).sum();
        let estimated_tokens = total_length / 4; // Rough estimate

        if descriptions.len() < self.force_summary_threshold && estimated_tokens < self.max_tokens_per_chunk {
            // Simple concatenation with deduplication
            return Ok(self.simple_merge(descriptions));
        }

        // Apply map-reduce for large description sets
        self.map_reduce_summarize(entity_name, descriptions).await
    }

    /// Simple merge without LLM (for small description sets)
    fn simple_merge(&self, descriptions: &[String]) -> String {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();

        for desc in descriptions {
            let normalized = desc.trim().to_lowercase();
            if !seen.contains(&normalized) {
                seen.insert(normalized);
                result.push(desc.trim());
            }
        }

        result.join(" ")
    }

    /// Map-reduce summarization for large description sets
    async fn map_reduce_summarize(
        &self,
        entity_name: &str,
        descriptions: &[String],
    ) -> Result<String, Error> {
        // Map phase: chunk descriptions into groups
        let chunks = self.chunk_descriptions(descriptions);

        let mut intermediate_summaries = Vec::new();

        for chunk in chunks {
            let summary = self.summarize_chunk(entity_name, &chunk).await?;
            intermediate_summaries.push(summary);
        }

        // Reduce phase: if we still have multiple summaries, reduce them
        while intermediate_summaries.len() > 1 {
            let new_chunks = self.chunk_descriptions(
                &intermediate_summaries.iter().map(|s| s.clone()).collect::<Vec<_>>()
            );

            let mut new_summaries = Vec::new();
            for chunk in new_chunks {
                let summary = self.summarize_chunk(entity_name, &chunk).await?;
                new_summaries.push(summary);
            }
            intermediate_summaries = new_summaries;
        }

        Ok(intermediate_summaries.into_iter().next().unwrap_or_default())
    }

    /// Chunk descriptions to fit within token limit
    fn chunk_descriptions(&self, descriptions: &[String]) -> Vec<Vec<String>> {
        let mut chunks = Vec::new();
        let mut current_chunk = Vec::new();
        let mut current_tokens = 0;

        for desc in descriptions {
            let desc_tokens = desc.len() / 4; // Rough estimate

            if current_tokens + desc_tokens > self.max_tokens_per_chunk && !current_chunk.is_empty() {
                chunks.push(std::mem::take(&mut current_chunk));
                current_tokens = 0;
            }

            current_chunk.push(desc.clone());
            current_tokens += desc_tokens;
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }

    /// Summarize a single chunk of descriptions
    async fn summarize_chunk(
        &self,
        entity_name: &str,
        descriptions: &[String],
    ) -> Result<String, Error> {
        let descriptions_text = descriptions
            .iter()
            .enumerate()
            .map(|(i, d)| format!("{}. {}", i + 1, d))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(r#"You are a helpful assistant responsible for generating a comprehensive summary of the data provided below.

Given one or more entities and their descriptions, generate a single comprehensive description that:
1. Captures all unique information from the input descriptions
2. Resolves any contradictions by preferring more specific information
3. Is written in a clear, coherent style
4. Does not exceed 500 words

# Entity: {entity_name}

# Descriptions to summarize:
{descriptions_text}

# Summary:"#);

        let response = self.llm.complete(&prompt).await
            .map_err(|e| Error::internal(format!("LLM error: {}", e)))?;

        Ok(response.content.trim().to_string())
    }

    /// Merge relationship descriptions
    pub async fn merge_relationship_descriptions(
        &self,
        source: &str,
        target: &str,
        descriptions: &[String],
    ) -> Result<String, Error> {
        let entity_name = format!("{} → {}", source, target);
        self.merge_entity_descriptions(&entity_name, descriptions).await
    }
}

use crate::error::Error;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_merge() {
        let summarizer = LLMSummarizer::new(/* mock */);
        let descriptions = vec![
            "Alice is a software engineer.".to_string(),
            "Alice works at TechCorp.".to_string(),
            "Alice is a software engineer.".to_string(), // Duplicate
        ];

        let result = summarizer.simple_merge(&descriptions);
        assert!(result.contains("software engineer"));
        assert!(result.contains("TechCorp"));
        // Should not have duplicate
    }

    #[test]
    fn test_chunk_descriptions() {
        let summarizer = LLMSummarizer {
            llm: /* mock */,
            max_tokens_per_chunk: 100, // Small for testing
            force_summary_threshold: 2,
        };

        let descriptions: Vec<String> = (0..10)
            .map(|i| format!("Description number {} with some content.", i))
            .collect();

        let chunks = summarizer.chunk_descriptions(&descriptions);
        assert!(chunks.len() > 1, "Should create multiple chunks");
    }
}
```

---

## 🎯 Reranking Integration

### 4.1 Objective

Implement reranking to improve retrieval precision by reordering results using a specialized reranking model.

### 4.2 Source Reference

**Location:** `lightrag/rerank.py` (576 lines)

**LightRAG Behavior:**

1. Support Jina Reranker and Cohere
2. Rerank retrieved chunks based on query relevance
3. Score aggregation across chunks
4. Configurable rerank model

### 4.3 Implementation Tasks

#### Task 4.3.1: Create Reranker Trait

**File:** `edgequake/crates/edgequake-llm/src/traits.rs`

```rust
// ADD to traits.rs

/// Reranker for improving retrieval precision
#[async_trait::async_trait]
pub trait Reranker: Send + Sync {
    /// Rerank documents based on query relevance
    async fn rerank(
        &self,
        query: &str,
        documents: &[RerankDocument],
        top_k: usize,
    ) -> Result<Vec<RerankResult>, ProviderError>;
}

/// Document to be reranked
#[derive(Debug, Clone)]
pub struct RerankDocument {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Result of reranking
#[derive(Debug, Clone)]
pub struct RerankResult {
    pub id: String,
    pub score: f32,
    pub original_index: usize,
}
```

---

#### Task 4.3.2: Implement Jina Reranker

**File:** `edgequake/crates/edgequake-llm/src/reranker/jina.rs` (NEW)

```rust
// NEW FILE: edgequake/crates/edgequake-llm/src/reranker/jina.rs

use crate::traits::{Reranker, RerankDocument, RerankResult, ProviderError};
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct JinaReranker {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl JinaReranker {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "jina-reranker-v2-base-multilingual".to_string(),
            base_url: "https://api.jina.ai/v1".to_string(),
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }
}

#[derive(Serialize)]
struct JinaRerankRequest {
    model: String,
    query: String,
    documents: Vec<String>,
    top_n: usize,
}

#[derive(Deserialize)]
struct JinaRerankResponse {
    results: Vec<JinaRerankResult>,
}

#[derive(Deserialize)]
struct JinaRerankResult {
    index: usize,
    relevance_score: f32,
}

#[async_trait::async_trait]
impl Reranker for JinaReranker {
    async fn rerank(
        &self,
        query: &str,
        documents: &[RerankDocument],
        top_k: usize,
    ) -> Result<Vec<RerankResult>, ProviderError> {
        let doc_texts: Vec<String> = documents.iter()
            .map(|d| d.content.clone())
            .collect();

        let request = JinaRerankRequest {
            model: self.model.clone(),
            query: query.to_string(),
            documents: doc_texts,
            top_n: top_k,
        };

        let response = self.client
            .post(format!("{}/rerank", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Api(error_text));
        }

        let jina_response: JinaRerankResponse = response.json().await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        let results: Vec<RerankResult> = jina_response.results
            .into_iter()
            .map(|r| RerankResult {
                id: documents[r.index].id.clone(),
                score: r.relevance_score,
                original_index: r.index,
            })
            .collect();

        Ok(results)
    }
}
```

---

#### Task 4.3.3: Integrate Reranking in Query Engine

**File:** `edgequake/crates/edgequake-core/src/query.rs`

```rust
// ADD to QueryEngine

impl QueryEngine {
    /// Set reranker for improving retrieval precision
    pub fn with_reranker(mut self, reranker: Arc<dyn Reranker>) -> Self {
        self.reranker = Some(reranker);
        self
    }

    /// Apply reranking to query results
    async fn apply_reranking(
        &self,
        query: &str,
        chunks: Vec<ContextChunk>,
        top_k: usize,
    ) -> Result<Vec<ContextChunk>> {
        let reranker = match &self.reranker {
            Some(r) => r,
            None => return Ok(chunks), // No reranker configured
        };

        let documents: Vec<RerankDocument> = chunks.iter()
            .map(|c| RerankDocument {
                id: c.chunk_id.clone(),
                content: c.content.clone(),
                metadata: HashMap::new(),
            })
            .collect();

        let rerank_results = reranker.rerank(query, &documents, top_k).await
            .map_err(|e| Error::internal(format!("Reranking error: {}", e)))?;

        // Reorder chunks based on rerank scores
        let mut reranked_chunks: Vec<ContextChunk> = rerank_results.iter()
            .filter_map(|r| {
                chunks.iter()
                    .find(|c| c.chunk_id == r.id)
                    .map(|c| ContextChunk {
                        score: r.score,
                        ..c.clone()
                    })
            })
            .collect();

        // Sort by reranked score (descending)
        reranked_chunks.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(reranked_chunks)
    }
}
```

---

## 🎯 Token Budget Management

### 5.1 Objective

Implement token budget management to ensure context fits within LLM context windows.

### 5.2 Implementation Tasks

#### Task 5.2.1: Create Token Counter

**File:** `edgequake/crates/edgequake-core/src/token_budget.rs` (NEW)

```rust
// NEW FILE: edgequake/crates/edgequake-core/src/token_budget.rs

use tiktoken_rs::{get_bpe_from_model, CoreBPE};

/// Manages token budgets for context construction
pub struct TokenBudget {
    encoder: CoreBPE,
    max_tokens: usize,
    reserved_for_response: usize,
}

impl TokenBudget {
    /// Create a new token budget for a specific model
    pub fn new(model: &str, max_tokens: usize) -> Self {
        let encoder = get_bpe_from_model(model)
            .unwrap_or_else(|_| get_bpe_from_model("gpt-4").unwrap());

        Self {
            encoder,
            max_tokens,
            reserved_for_response: 1000, // Reserve for response generation
        }
    }

    /// Get available tokens for context
    pub fn available_tokens(&self) -> usize {
        self.max_tokens.saturating_sub(self.reserved_for_response)
    }

    /// Count tokens in text
    pub fn count_tokens(&self, text: &str) -> usize {
        self.encoder.encode_with_special_tokens(text).len()
    }

    /// Truncate text to fit within budget
    pub fn truncate_to_budget(&self, text: &str, budget: usize) -> String {
        let tokens = self.encoder.encode_with_special_tokens(text);

        if tokens.len() <= budget {
            return text.to_string();
        }

        let truncated_tokens: Vec<_> = tokens.into_iter().take(budget).collect();
        self.encoder.decode(truncated_tokens)
            .unwrap_or_else(|_| text[..budget * 4].to_string()) // Fallback
    }

    /// Allocate budget across multiple content sources
    pub fn allocate_budget(&self, sources: &[BudgetSource]) -> Vec<usize> {
        let total_available = self.available_tokens();
        let total_weight: f64 = sources.iter().map(|s| s.weight).sum();

        sources.iter()
            .map(|s| ((s.weight / total_weight) * total_available as f64) as usize)
            .collect()
    }
}

/// Source for budget allocation
pub struct BudgetSource {
    pub name: String,
    pub weight: f64,
    pub min_tokens: usize,
}

impl Default for BudgetSource {
    fn default() -> Self {
        Self {
            name: String::new(),
            weight: 1.0,
            min_tokens: 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counting() {
        let budget = TokenBudget::new("gpt-4", 4000);
        let text = "Hello, world!";
        let count = budget.count_tokens(text);
        assert!(count > 0);
        assert!(count < 10);
    }

    #[test]
    fn test_budget_allocation() {
        let budget = TokenBudget::new("gpt-4", 4000);
        let sources = vec![
            BudgetSource { name: "entities".to_string(), weight: 2.0, min_tokens: 100 },
            BudgetSource { name: "chunks".to_string(), weight: 1.0, min_tokens: 100 },
        ];

        let allocations = budget.allocate_budget(&sources);
        assert_eq!(allocations.len(), 2);
        assert!(allocations[0] > allocations[1]); // Entities get 2x weight
    }
}
```

**Dependencies to add:**

```toml
# Add to edgequake/crates/edgequake-core/Cargo.toml
tiktoken-rs = "0.5"
```

---

## 📊 Testing Requirements

See [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md#phase-2-validation) for full specifications.

### Unit Tests

```bash
cargo test --package edgequake-core --lib keyword_extractor
cargo test --package edgequake-pipeline --lib merger
cargo test --package edgequake-pipeline --lib summarizer
cargo test --package edgequake-llm --lib reranker
cargo test --package edgequake-core --lib token_budget
```

### Integration Tests

```bash
cargo test --package edgequake-core --test entity_deduplication
cargo test --package edgequake-core --test reranking
```

---

## 🔗 Cross-References

| Topic        | Document                                                 | Section                           |
| ------------ | -------------------------------------------------------- | --------------------------------- |
| Gap Details  | [../gap-analysis.md](../gap-analysis.md)                 | F-010, F-011, F-021, F-025, F-026 |
| Query Engine | [01-PHASE1-QUERY-ENGINE.md](./01-PHASE1-QUERY-ENGINE.md) | Keyword Extraction                |
| Testing Plan | [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md)   | Phase 2 Validation                |
| Dependencies | [09-DEPENDENCY-GRAPH.md](./09-DEPENDENCY-GRAPH.md)       | Core Quality                      |
| Master Index | [00-INDEX.md](./00-INDEX.md)                             | Phase 2                           |

---

## ✅ Completion Criteria

| Criterion                              | Target              | Validation       |
| -------------------------------------- | ------------------- | ---------------- |
| Keywords extracted                     | HL + LL lists       | Unit test        |
| Entity deduplication works             | Merged descriptions | Integration test |
| Summarization produces coherent output | Quality check       | Manual review    |
| Reranking improves precision           | +15% precision      | A/B test         |
| Token budget respected                 | No overflow         | Unit test        |

---

_Document Version: 1.0_  
_Last Updated: 2024-12-24_  
_Owner: EdgeQuake Core Team_
