# Phase 1: Query Engine Enhancement

**Document ID:** 01-PHASE1-QUERY-ENGINE  
**Priority:** 🔴 P0 CRITICAL  
**Effort:** 11 person-days  
**Duration:** Weeks 1-2  
**Dependencies:** None  
**Blocks:** [03-PHASE2-CORE-QUALITY.md](./03-PHASE2-CORE-QUALITY.md)

---

## 📋 Overview

This document provides high-precision implementation guidance for the two most critical missing features in EdgeQuake: **Global Query Mode** and **Mix Query Mode**. These are LightRAG's signature features that differentiate it from standard RAG implementations.

### Gaps Addressed

| Gap ID      | Feature            | Severity | Status         |
| ----------- | ------------------ | -------- | -------------- |
| **GAP-001** | Query Mode: Global | 🔴 P0    | 🔲 Not started |
| **GAP-002** | Query Mode: Mix    | 🔴 P0    | 🔲 Not started |

### Cross-References

- **Source Analysis:** [../gap-analysis.md](../gap-analysis.md#feature-f-015-query-mode-global)
- **Master Plan:** [00-INDEX.md](./00-INDEX.md#phase-1-foundation-weeks-1-3)
- **Dependency Graph:** [09-DEPENDENCY-GRAPH.md](./09-DEPENDENCY-GRAPH.md#query-engine-dependencies)
- **Testing Plan:** [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md#phase-1-validation)

---

## 🎯 Global Query Mode

### 1.1 Objective

Implement global query mode that retrieves high-level concepts and relationships from the entire knowledge graph, enabling synthesis across documents.

### 1.2 Source Reference

**Location:** `lightrag/operate.py` lines 2300-2800

**LightRAG Behavior:**

1. Extract HIGH-LEVEL keywords from query using LLM
2. Search **relationship vector store** for matching edges
3. Retrieve connected entity clusters
4. Aggregate global context from relationships
5. Generate response using global context

### 1.3 Implementation Tasks

#### Task 1.3.1: Create Relationship Vector Store

**File:** `edgequake/crates/edgequake-storage/src/traits/vector.rs`

**Current State:** VectorStorage trait only stores chunks and entities.

**Action:** Add relationship embedding support.

```rust
// ADD to VectorStorage trait in edgequake-storage/src/traits/vector.rs

/// Query vectors by namespace
async fn query_with_namespace(
    &self,
    query_vector: &[f32],
    top_k: usize,
    namespace: VectorNamespace,  // NEW: Chunk, Entity, or Relationship
    filter: Option<&serde_json::Value>,
) -> StorageResult<Vec<VectorQueryResult>>;

/// Insert vectors with namespace
async fn insert_with_namespace(
    &self,
    vectors: Vec<(String, Vec<f32>, HashMap<String, serde_json::Value>)>,
    namespace: VectorNamespace,  // NEW
) -> StorageResult<()>;

// NEW enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorNamespace {
    Chunk,
    Entity,
    Relationship,  // NEW for global mode
}
```

**Validation:**

- [ ] Unit test: `query_with_namespace` returns correct namespace results
- [ ] Integration test: Relationship vectors stored separately from chunks

---

#### Task 1.3.2: Create Keyword Extractor

**File:** `edgequake/crates/edgequake-core/src/keyword_extractor.rs` (NEW)

**Purpose:** Extract high-level and low-level keywords from query using LLM.

````rust
// NEW FILE: edgequake/crates/edgequake-core/src/keyword_extractor.rs

use crate::error::Result;
use edgequake_llm::traits::LLMProvider;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Keywords extracted from a query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedKeywords {
    /// High-level conceptual keywords (for global mode)
    pub high_level: Vec<String>,
    /// Low-level specific keywords (for local mode)
    pub low_level: Vec<String>,
}

/// Extracts keywords from queries for retrieval
pub struct KeywordExtractor {
    llm: Arc<dyn LLMProvider>,
}

impl KeywordExtractor {
    pub fn new(llm: Arc<dyn LLMProvider>) -> Self {
        Self { llm }
    }

    /// Extract keywords from query
    pub async fn extract(&self, query: &str) -> Result<ExtractedKeywords> {
        let prompt = self.build_extraction_prompt(query);

        let response = self.llm.complete(&prompt).await
            .map_err(|e| crate::error::Error::internal(format!("LLM error: {}", e)))?;

        self.parse_keywords(&response.content)
    }

    fn build_extraction_prompt(&self, query: &str) -> String {
        // Port from LightRAG: lightrag/prompt.py PROMPTS["keywords_extraction"]
        format!(r#"---Role---
You are a helpful assistant tasked with identifying both high-level and low-level keywords in the user's query.

---Goal---
Given the query, list both high-level and low-level keywords. High-level keywords focus on overarching concepts or themes, while low-level keywords focus on specific entities, details, or concrete terms.

---Instructions---
- Output the keywords in JSON format.
- The JSON should have two keys: "high_level_keywords" and "low_level_keywords".
- Each key should contain a list of strings (keywords).

######################
-Examples-
######################
Example 1:
Query: "How does international trade influence global economic stability?"
################
Output:
{{
  "high_level_keywords": ["International trade", "Global economic stability", "Economic impact"],
  "low_level_keywords": ["Trade agreements", "Tariffs", "Currency exchange", "Imports", "Exports"]
}}
#############################
Example 2:
Query: "What is the role of mitochondria in cellular respiration?"
################
Output:
{{
  "high_level_keywords": ["Cellular respiration", "Energy production", "Cell biology"],
  "low_level_keywords": ["Mitochondria", "ATP", "Electron transport chain", "Krebs cycle", "Oxygen"]
}}
#############################
-Real Data-
######################
Query: {query}
######################
Output:
"#)
    }

    fn parse_keywords(&self, response: &str) -> Result<ExtractedKeywords> {
        // Extract JSON from response (handle markdown code blocks)
        let json_str = if response.contains("```json") {
            response
                .split("```json")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(response)
        } else if response.contains("```") {
            response
                .split("```")
                .nth(1)
                .unwrap_or(response)
        } else {
            response
        };

        #[derive(Deserialize)]
        struct KeywordsResponse {
            high_level_keywords: Vec<String>,
            low_level_keywords: Vec<String>,
        }

        let parsed: KeywordsResponse = serde_json::from_str(json_str.trim())
            .map_err(|e| crate::error::Error::internal(format!("Failed to parse keywords: {}", e)))?;

        Ok(ExtractedKeywords {
            high_level: parsed.high_level_keywords,
            low_level: parsed.low_level_keywords,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_keywords() {
        let extractor = KeywordExtractor::new(/* mock */);
        let json = r#"{"high_level_keywords": ["AI", "ML"], "low_level_keywords": ["neural networks"]}"#;
        let result = extractor.parse_keywords(json).unwrap();
        assert_eq!(result.high_level, vec!["AI", "ML"]);
        assert_eq!(result.low_level, vec!["neural networks"]);
    }
}
````

**Validation:**

- [ ] Unit test: JSON parsing handles various LLM response formats
- [ ] Integration test: Keywords extracted from realistic queries

---

#### Task 1.3.3: Implement Global Query in QueryEngine

**File:** `edgequake/crates/edgequake-core/src/query.rs`

**Action:** Add `query_global` method to QueryEngine.

```rust
// ADD to QueryEngine in edgequake/crates/edgequake-core/src/query.rs

use crate::keyword_extractor::KeywordExtractor;

impl QueryEngine {
    // ADD after query_local method

    /// Global RAG: Relationship-centric high-level retrieval
    async fn query_global(&self, query: &str, params: &QueryParams) -> Result<QueryResult> {
        let retrieval_start = std::time::Instant::now();

        // 1. Extract high-level keywords from query
        let keyword_extractor = KeywordExtractor::new(Arc::clone(&self.llm));
        let keywords = keyword_extractor.extract(query).await?;

        tracing::debug!("Extracted keywords: HL={:?}, LL={:?}",
            keywords.high_level, keywords.low_level);

        // 2. Embed high-level keywords
        let keyword_texts: Vec<String> = keywords.high_level.clone();
        let keyword_embeddings = self.embedding
            .embed(&keyword_texts)
            .await
            .map_err(|e| crate::error::Error::internal(format!("Embedding error: {}", e)))?;

        // 3. Search relationship vector store for each keyword
        let mut all_relationships = Vec::new();
        let mut seen_relations = std::collections::HashSet::new();

        for keyword_embedding in &keyword_embeddings {
            let results = self.vector_storage
                .query_with_namespace(
                    keyword_embedding,
                    params.top_k / keyword_embeddings.len().max(1),
                    edgequake_storage::traits::VectorNamespace::Relationship,
                    None,
                )
                .await
                .map_err(|e| crate::error::Error::internal(format!("Vector search error: {}", e)))?;

            for result in results {
                let relation_key = (
                    result.metadata.get("source").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    result.metadata.get("target").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                );

                if !seen_relations.contains(&relation_key) {
                    seen_relations.insert(relation_key);
                    all_relationships.push(result);
                }
            }
        }

        // 4. Retrieve relationship details and connected entities
        let mut context_relationships = Vec::new();
        let mut context_entities = Vec::new();
        let mut context_text = String::new();

        for rel_result in all_relationships.iter().take(params.top_k) {
            let source_id = rel_result.metadata.get("source")
                .and_then(|v| v.as_str()).unwrap_or("");
            let target_id = rel_result.metadata.get("target")
                .and_then(|v| v.as_str()).unwrap_or("");
            let description = rel_result.metadata.get("description")
                .and_then(|v| v.as_str()).unwrap_or("");
            let keywords_str = rel_result.metadata.get("keywords")
                .and_then(|v| v.as_str()).unwrap_or("");

            // Add relationship to context
            context_relationships.push(ContextRelationship {
                source: source_id.to_string(),
                target: target_id.to_string(),
                relation_type: "RELATED".to_string(),
                description: description.to_string(),
                score: rel_result.score,
            });

            // Build context text for LLM
            context_text.push_str(&format!(
                "### Relationship: {} → {}\n{}\nKeywords: {}\n\n",
                source_id, target_id, description, keywords_str
            ));

            // Fetch connected entities for additional context
            for entity_id in [source_id, target_id] {
                if let Some(node) = self.graph_storage.get_node(entity_id).await? {
                    let entity_desc = node.properties
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    context_entities.push(ContextEntity {
                        name: entity_id.to_string(),
                        entity_type: node.properties
                            .get("entity_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("UNKNOWN")
                            .to_string(),
                        description: entity_desc.to_string(),
                        score: rel_result.score,
                    });
                }
            }
        }

        let retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;
        let generation_start = std::time::Instant::now();

        // 5. Generate response using global context
        let prompt = self.build_global_prompt(query, &context_text, &keywords);

        let response = self.llm
            .complete(&prompt)
            .await
            .map_err(|e| crate::error::Error::internal(format!("LLM error: {}", e)))?;

        let generation_time_ms = generation_start.elapsed().as_millis() as u64;

        Ok(QueryResult {
            response: response.content,
            mode: QueryMode::Global,
            context: QueryContext {
                entities: context_entities,
                relationships: context_relationships,
                ..Default::default()
            },
            stats: QueryStats {
                retrieval_time_ms,
                generation_time_ms,
                total_time_ms: 0,
                entities_retrieved: context_entities.len(),
                relationships_retrieved: context_relationships.len(),
                keywords_extracted: keywords.high_level.len() + keywords.low_level.len(),
                prompt_tokens: response.prompt_tokens,
                response_tokens: response.completion_tokens,
                ..Default::default()
            },
        })
    }

    fn build_global_prompt(
        &self,
        query: &str,
        context: &str,
        keywords: &ExtractedKeywords
    ) -> String {
        // Port from LightRAG: lightrag/prompt.py PROMPTS["rag_response"]
        format!(r#"---Role---
You are a helpful assistant responding to questions about data in the provided tables and relationships.

---Goal---
Generate a response of the target length and format that responds to the user's question, summarizing all information in the input data tables appropriate for the response length and format, and incorporating any relevant general knowledge.

If you don't know the answer, just say so. Do not make anything up.

Points supported by data should list their sources at the end of the response.

---Target response length and format---
Multiple paragraphs

---Data tables---
{context}

---Keywords identified---
High-level themes: {high_level_keywords}
Specific terms: {low_level_keywords}

---Goal---
Generate a response of the target length and format that responds to the user's question.

Query: {query}

Response:"#,
            context = context,
            high_level_keywords = keywords.high_level.join(", "),
            low_level_keywords = keywords.low_level.join(", "),
            query = query
        )
    }
}
```

**Validation:**

- [ ] Unit test: `query_global` returns relationship-based context
- [ ] Integration test: Full global query flow with real data
- [ ] Performance test: Within 20% of naive mode latency

---

#### Task 1.3.4: Update Query Match Statement

**File:** `edgequake/crates/edgequake-core/src/query.rs`

**Current State (line 36-40):**

```rust
let result = match params.mode {
    QueryMode::Naive => self.query_naive(query, &params).await?,
    QueryMode::Local => self.query_local(query, &params).await?,
    _ => self.query_naive(query, &params).await?, // Fallback for now
};
```

**Action:** Replace fallback with actual implementations.

```rust
// REPLACE in query.rs lines 36-40
let result = match params.mode {
    QueryMode::Naive => self.query_naive(query, &params).await?,
    QueryMode::Local => self.query_local(query, &params).await?,
    QueryMode::Global => self.query_global(query, &params).await?,  // NEW
    QueryMode::Mix => self.query_mix(query, &params).await?,        // NEW (see section 2)
    QueryMode::Hybrid => {
        // Hybrid = Local + Global (without naive chunks)
        self.query_hybrid(query, &params).await?
    },
};
```

---

#### Task 1.3.5: Store Relationship Embeddings During Ingestion

**File:** `edgequake/crates/edgequake-pipeline/src/merger.rs`

**Action:** After merging relationships, embed and store in vector DB.

```rust
// ADD to KnowledgeGraphMerger after relationship merge

/// Store relationship embedding for global query mode
async fn store_relationship_embedding(
    &self,
    source: &str,
    target: &str,
    description: &str,
    keywords: &str,
) -> Result<()> {
    // Create embedding text from relationship content
    let embedding_text = format!(
        "{} {} {} {}",
        source, target, description, keywords
    );

    let embeddings = self.embedding_provider
        .embed(&[embedding_text])
        .await?;

    let embedding = embeddings.first()
        .ok_or_else(|| Error::internal("No embedding generated"))?;

    // Store with relationship metadata
    let metadata = serde_json::json!({
        "source": source,
        "target": target,
        "description": description,
        "keywords": keywords,
    });

    let relation_id = format!("rel-{}-{}", source, target);

    self.vector_storage.insert_with_namespace(
        vec![(relation_id, embedding.clone(), metadata.as_object().unwrap().clone())],
        VectorNamespace::Relationship,
    ).await?;

    Ok(())
}
```

---

### 1.4 Global Query Checklist

- [ ] VectorNamespace enum added to traits
- [ ] KeywordExtractor implemented with LLM prompt
- [ ] query_global method in QueryEngine
- [ ] Relationship embeddings stored during ingestion
- [ ] Match statement updated for Global mode
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Performance within 20% of naive mode

---

## 🎯 Mix Query Mode

### 2.1 Objective

Implement mix query mode that combines local entity-centric retrieval with naive chunk retrieval, providing the most comprehensive context.

### 2.2 Source Reference

**Location:** `lightrag/operate.py` with mode="mix"

**LightRAG Behavior:**

1. Execute local query (entities + relationships)
2. Execute naive query (chunk retrieval)
3. Deduplicate overlapping sources by source_id
4. Merge context respecting token budget
5. Generate unified response

### 2.3 Implementation Tasks

#### Task 2.3.1: Implement Mix Query Method

**File:** `edgequake/crates/edgequake-core/src/query.rs`

```rust
// ADD to QueryEngine in edgequake/crates/edgequake-core/src/query.rs

impl QueryEngine {
    /// Mix RAG: Combines local entity-centric with naive chunk retrieval
    async fn query_mix(&self, query: &str, params: &QueryParams) -> Result<QueryResult> {
        let retrieval_start = std::time::Instant::now();

        // 1. Execute local and naive queries in parallel
        let (local_result, naive_result) = tokio::join!(
            self.query_local_context(query, params),
            self.query_naive_context(query, params)
        );

        let local_context = local_result?;
        let naive_context = naive_result?;

        // 2. Merge and deduplicate contexts
        let merged_context = self.merge_contexts(
            local_context,
            naive_context,
            params.max_tokens.unwrap_or(4000),
        )?;

        let retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;
        let generation_start = std::time::Instant::now();

        // 3. Build combined context text
        let context_text = self.build_mixed_context_text(&merged_context);

        // 4. Generate response
        let prompt = format!(
            r#"---Role---
You are a helpful assistant answering questions based on knowledge graph entities and document chunks.

---Context---
## Knowledge Graph Entities and Relationships
{entity_context}

## Document Chunks
{chunk_context}

---Instructions---
Use both the entity relationships and document chunks to provide a comprehensive answer.
Prioritize information from the knowledge graph for factual relationships.
Use document chunks for detailed supporting information.
If information conflicts, note the discrepancy.

---Question---
{query}

---Answer---"#,
            entity_context = context_text.entity_section,
            chunk_context = context_text.chunk_section,
            query = query
        );

        let response = self.llm
            .complete(&prompt)
            .await
            .map_err(|e| crate::error::Error::internal(format!("LLM error: {}", e)))?;

        let generation_time_ms = generation_start.elapsed().as_millis() as u64;

        Ok(QueryResult {
            response: response.content,
            mode: QueryMode::Mix,
            context: merged_context.into_query_context(),
            stats: QueryStats {
                retrieval_time_ms,
                generation_time_ms,
                total_time_ms: 0,
                chunks_retrieved: merged_context.chunks.len(),
                entities_retrieved: merged_context.entities.len(),
                relationships_retrieved: merged_context.relationships.len(),
                prompt_tokens: response.prompt_tokens,
                response_tokens: response.completion_tokens,
                ..Default::default()
            },
        })
    }

    /// Get local context without generating response
    async fn query_local_context(&self, query: &str, params: &QueryParams) -> Result<LocalContext> {
        // Reuse logic from query_local but return raw context
        let query_embeddings = self.embedding
            .embed(&[query.to_string()])
            .await
            .map_err(|e| crate::error::Error::internal(format!("Embedding error: {}", e)))?;

        let query_embedding = query_embeddings.first()
            .ok_or_else(|| crate::error::Error::internal("No embedding generated"))?;

        let entity_results = self.vector_storage
            .query_with_namespace(query_embedding, params.top_k, VectorNamespace::Entity, None)
            .await
            .map_err(|e| crate::error::Error::internal(format!("Vector search error: {}", e)))?;

        let mut entities = Vec::new();
        let mut relationships = Vec::new();
        let mut source_ids = std::collections::HashSet::new();

        for result in entity_results {
            let entity_id = result.id;

            if let Some(node) = self.graph_storage.get_node(&entity_id).await? {
                // Track source IDs for deduplication
                if let Some(sid) = node.properties.get("source_id").and_then(|v| v.as_str()) {
                    source_ids.insert(sid.to_string());
                }

                entities.push(LocalEntity {
                    id: entity_id.clone(),
                    name: node.properties.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&entity_id)
                        .to_string(),
                    entity_type: node.properties.get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("UNKNOWN")
                        .to_string(),
                    description: node.properties.get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    source_id: node.properties.get("source_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    score: result.score,
                });

                // Get relationships
                let edges = self.graph_storage.get_node_edges(&entity_id).await?;
                for edge in edges {
                    relationships.push(LocalRelationship {
                        source: edge.source,
                        target: edge.target,
                        description: edge.properties.get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        source_id: edge.properties.get("source_id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    });
                }
            }
        }

        Ok(LocalContext {
            entities,
            relationships,
            source_ids,
        })
    }

    /// Get naive context without generating response
    async fn query_naive_context(&self, query: &str, params: &QueryParams) -> Result<NaiveContext> {
        let query_embeddings = self.embedding
            .embed(&[query.to_string()])
            .await
            .map_err(|e| crate::error::Error::internal(format!("Embedding error: {}", e)))?;

        let query_embedding = query_embeddings.first()
            .ok_or_else(|| crate::error::Error::internal("No embedding generated"))?;

        let search_results = self.vector_storage
            .query_with_namespace(query_embedding, params.top_k, VectorNamespace::Chunk, None)
            .await
            .map_err(|e| crate::error::Error::internal(format!("Vector search error: {}", e)))?;

        let mut chunks = Vec::new();

        for result in search_results {
            chunks.push(NaiveChunk {
                id: result.id,
                content: result.metadata.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                document_id: result.metadata.get("document_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                source_id: result.metadata.get("source_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                score: result.score,
            });
        }

        Ok(NaiveContext { chunks })
    }

    /// Merge local and naive contexts, deduplicating by source_id
    fn merge_contexts(
        &self,
        local: LocalContext,
        naive: NaiveContext,
        max_tokens: usize,
    ) -> Result<MergedContext> {
        // Deduplicate: Remove naive chunks whose source_id appears in local entities
        let deduped_chunks: Vec<_> = naive.chunks
            .into_iter()
            .filter(|chunk| {
                match &chunk.source_id {
                    Some(sid) => !local.source_ids.contains(sid),
                    None => true, // Keep chunks without source_id
                }
            })
            .collect();

        // Apply token budget (simplified - could use tiktoken)
        let mut token_count = 0;
        let tokens_per_char = 0.25; // Rough estimate

        let mut final_entities = Vec::new();
        for entity in local.entities {
            let entity_tokens = (entity.description.len() as f64 * tokens_per_char) as usize;
            if token_count + entity_tokens <= max_tokens / 2 {
                token_count += entity_tokens;
                final_entities.push(entity);
            }
        }

        let mut final_chunks = Vec::new();
        for chunk in deduped_chunks {
            let chunk_tokens = (chunk.content.len() as f64 * tokens_per_char) as usize;
            if token_count + chunk_tokens <= max_tokens {
                token_count += chunk_tokens;
                final_chunks.push(chunk);
            }
        }

        Ok(MergedContext {
            entities: final_entities,
            relationships: local.relationships,
            chunks: final_chunks,
            total_tokens: token_count,
        })
    }
}

// Supporting types
struct LocalContext {
    entities: Vec<LocalEntity>,
    relationships: Vec<LocalRelationship>,
    source_ids: std::collections::HashSet<String>,
}

struct LocalEntity {
    id: String,
    name: String,
    entity_type: String,
    description: String,
    source_id: Option<String>,
    score: f32,
}

struct LocalRelationship {
    source: String,
    target: String,
    description: String,
    source_id: Option<String>,
}

struct NaiveContext {
    chunks: Vec<NaiveChunk>,
}

struct NaiveChunk {
    id: String,
    content: String,
    document_id: String,
    source_id: Option<String>,
    score: f32,
}

struct MergedContext {
    entities: Vec<LocalEntity>,
    relationships: Vec<LocalRelationship>,
    chunks: Vec<NaiveChunk>,
    total_tokens: usize,
}

impl MergedContext {
    fn into_query_context(self) -> QueryContext {
        QueryContext {
            entities: self.entities.into_iter().map(|e| ContextEntity {
                name: e.name,
                entity_type: e.entity_type,
                description: e.description,
                score: e.score,
            }).collect(),
            relationships: self.relationships.into_iter().map(|r| ContextRelationship {
                source: r.source,
                target: r.target,
                relation_type: "RELATED".to_string(),
                description: r.description,
                score: 1.0,
            }).collect(),
            chunks: self.chunks.into_iter().map(|c| ContextChunk {
                chunk_id: c.id,
                document_id: c.document_id,
                content: c.content,
                score: c.score,
            }).collect(),
        }
    }
}

struct MixedContextText {
    entity_section: String,
    chunk_section: String,
}
```

---

### 2.4 Mix Query Checklist

- [ ] `query_mix` method implemented
- [ ] `query_local_context` extracts raw context
- [ ] `query_naive_context` extracts raw context
- [ ] `merge_contexts` deduplicates by source_id
- [ ] Token budget respected
- [ ] Mix is default mode (update QueryParams default)
- [ ] Unit tests pass
- [ ] Integration tests pass

---

## 📊 Testing Requirements

See [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md#phase-1-query-engine-tests) for full test specifications.

### Unit Tests

```bash
cargo test --package edgequake-core --lib keyword_extractor
cargo test --package edgequake-core --lib query_global
cargo test --package edgequake-core --lib query_mix
```

### Integration Tests

```bash
# Create test file: edgequake/crates/edgequake-core/tests/query_modes.rs
cargo test --package edgequake-core --test query_modes
```

### Performance Benchmarks

```bash
cargo bench --package edgequake-core -- query
```

**Performance Targets:**

- Global query: ≤ 150% of naive query latency
- Mix query: ≤ 200% of naive query latency
- Keyword extraction: ≤ 500ms

---

## 🔗 Cross-References

| Topic        | Document                                                 | Section            |
| ------------ | -------------------------------------------------------- | ------------------ |
| Gap Details  | [../gap-analysis.md](../gap-analysis.md)                 | F-015, F-017       |
| Testing Plan | [07-VALIDATION-TESTING.md](./07-VALIDATION-TESTING.md)   | Phase 1 Validation |
| Dependencies | [09-DEPENDENCY-GRAPH.md](./09-DEPENDENCY-GRAPH.md)       | Query Engine       |
| Next Phase   | [03-PHASE2-CORE-QUALITY.md](./03-PHASE2-CORE-QUALITY.md) | Keyword Extraction |
| Master Index | [00-INDEX.md](./00-INDEX.md)                             | Phase 1            |

---

## ✅ Completion Criteria

| Criterion              | Target               | Validation              |
| ---------------------- | -------------------- | ----------------------- |
| Global mode functional | ✅                   | Integration test passes |
| Mix mode functional    | ✅                   | Integration test passes |
| Keywords extracted     | HL + LL lists        | Unit test               |
| Deduplication works    | No duplicate sources | Unit test               |
| Token budget respected | ≤ max_tokens         | Unit test               |
| Default mode is Mix    | QueryMode::Mix       | Config check            |
| Performance acceptable | Within targets       | Benchmark               |

---

_Document Version: 1.0_  
_Last Updated: 2024-12-24_  
_Owner: EdgeQuake Core Team_
