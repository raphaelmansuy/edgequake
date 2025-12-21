# Phase 3: Development Roadmap

**Phase Duration**: Weeks 5-8  
**Owner**: Full Development Team  
**Status**: 🔴 Not Started

---

## Objective

Implement the core EdgeQuake functionality: document processing pipeline, entity/relationship extraction, merging algorithms, all query modes, and the REST API layer.

---

## Reference Documentation

| Document | Purpose |
|----------|---------|
| [docs_retro/04-api-contracts.md](../../docs_retro/04-api-contracts.md) | API specifications |
| [docs_retro/05-algorithms.md](../../docs_retro/05-algorithms.md) | Core algorithm pseudocode |
| [tech_stack/axum.md](../../tech_stack/axum.md) | Web framework guide |
| [tech_stack/async-openai.md](../../tech_stack/async-openai.md) | LLM client integration |
| [tech_stack/openapi-swagger.md](../../tech_stack/openapi-swagger.md) | API documentation |
| [plan/integration/IMPLEMENTATION_ROADMAP.md](../../plan/integration/IMPLEMENTATION_ROADMAP.md) | Existing roadmap |

---

## Deliverables Overview

| Week | Focus Area | Key Deliverables |
|------|-----------|------------------|
| Week 5 | Chunking & Extraction | Token-based chunking, LLM entity extraction |
| Week 6 | Merging & Embeddings | Entity/relation merging, embedding generation |
| Week 7 | Query Modes | naive, local, global, hybrid implementation |
| Week 8 | REST API | Axum endpoints, OpenAPI documentation |

---

## Week 5: Chunking & Entity Extraction

### 5.1 Text Chunking Algorithm

```rust
// edgequake-pipeline/src/chunking.rs
use tiktoken_rs::CoreBPE;
use edgequake_core::types::Chunk;
use crate::error::PipelineError;

/// Configuration for text chunking
#[derive(Debug, Clone)]
pub struct ChunkingConfig {
    /// Maximum tokens per chunk
    pub chunk_token_size: usize,
    /// Overlap tokens between chunks
    pub chunk_overlap_token_size: usize,
    /// Optional character to split on first
    pub split_by_character: Option<char>,
    /// If true, only split by character (error if chunk too large)
    pub split_by_character_only: bool,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            chunk_token_size: 1200,
            chunk_overlap_token_size: 100,
            split_by_character: None,
            split_by_character_only: false,
        }
    }
}

/// Chunk a document into overlapping segments
/// Reference: docs_retro/05-algorithms.md#1-text-chunking-algorithm
pub fn chunk_by_token_size(
    tokenizer: &CoreBPE,
    content: &str,
    config: &ChunkingConfig,
    doc_id: &str,
    file_path: Option<&str>,
) -> Result<Vec<Chunk>, PipelineError> {
    let mut results = Vec::new();
    
    if let Some(split_char) = config.split_by_character {
        // Split by character first
        let raw_chunks: Vec<&str> = content.split(split_char).collect();
        let mut processed_chunks = Vec::new();
        
        for raw_chunk in raw_chunks {
            let trimmed = raw_chunk.trim();
            if trimmed.is_empty() {
                continue;
            }
            
            let tokens = tokenizer.encode_with_special_tokens(trimmed);
            
            if tokens.len() > config.chunk_token_size {
                if config.split_by_character_only {
                    return Err(PipelineError::ChunkingFailed(format!(
                        "Chunk exceeds token limit ({} > {}) and split_by_character_only is true",
                        tokens.len(),
                        config.chunk_token_size
                    )));
                }
                
                // Sub-split by token size
                let step = config.chunk_token_size - config.chunk_overlap_token_size;
                let mut start = 0;
                
                while start < tokens.len() {
                    let end = std::cmp::min(start + config.chunk_token_size, tokens.len());
                    let sub_tokens = &tokens[start..end];
                    let sub_content = tokenizer.decode(sub_tokens.to_vec())
                        .map_err(|e| PipelineError::ChunkingFailed(e.to_string()))?;
                    
                    processed_chunks.push((sub_tokens.len(), sub_content.trim().to_string()));
                    start += step;
                }
            } else {
                processed_chunks.push((tokens.len(), trimmed.to_string()));
            }
        }
        
        for (index, (token_count, chunk_content)) in processed_chunks.into_iter().enumerate() {
            results.push(Chunk {
                id: Chunk::generate_id(&chunk_content),
                content: chunk_content,
                tokens: token_count as u32,
                chunk_order_index: index as u32,
                full_doc_id: doc_id.to_string(),
                file_path: file_path.map(|s| s.to_string()),
            });
        }
    } else {
        // Split purely by token size with overlap
        let tokens = tokenizer.encode_with_special_tokens(content);
        let step = config.chunk_token_size - config.chunk_overlap_token_size;
        let mut index = 0;
        let mut start = 0;
        
        while start < tokens.len() {
            let end = std::cmp::min(start + config.chunk_token_size, tokens.len());
            let chunk_tokens = &tokens[start..end];
            let chunk_content = tokenizer.decode(chunk_tokens.to_vec())
                .map_err(|e| PipelineError::ChunkingFailed(e.to_string()))?;
            
            results.push(Chunk {
                id: Chunk::generate_id(&chunk_content),
                content: chunk_content.trim().to_string(),
                tokens: chunk_tokens.len() as u32,
                chunk_order_index: index,
                full_doc_id: doc_id.to_string(),
                file_path: file_path.map(|s| s.to_string()),
            });
            
            index += 1;
            start += step;
        }
    }
    
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiktoken_rs::cl100k_base;
    
    #[test]
    fn test_basic_chunking() {
        let tokenizer = cl100k_base().unwrap();
        let content = "Hello world. ".repeat(500);
        let config = ChunkingConfig::default();
        
        let chunks = chunk_by_token_size(
            &tokenizer,
            &content,
            &config,
            "doc-test",
            None,
        ).unwrap();
        
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(chunk.tokens <= config.chunk_token_size as u32);
        }
    }
    
    #[test]
    fn test_overlap() {
        let tokenizer = cl100k_base().unwrap();
        let content = "word ".repeat(2000);
        let config = ChunkingConfig {
            chunk_token_size: 100,
            chunk_overlap_token_size: 20,
            ..Default::default()
        };
        
        let chunks = chunk_by_token_size(
            &tokenizer,
            &content,
            &config,
            "doc-test",
            None,
        ).unwrap();
        
        // Verify chunks have overlap
        assert!(chunks.len() > 1);
    }
}
```

### 5.2 Entity Extraction

```rust
// edgequake-pipeline/src/extraction.rs
use edgequake_core::types::{GraphEntity, GraphRelationship};
use edgequake_llm::traits::{LLMProvider, ChatMessage, MessageRole, CompletionOptions};
use crate::error::PipelineError;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Delimiters for LLM output parsing
pub const TUPLE_DELIMITER: &str = "<|#|>";
pub const COMPLETION_DELIMITER: &str = "<|COMPLETE|>";

/// Extraction result for a single chunk
#[derive(Debug, Clone)]
pub struct ChunkExtractionResult {
    pub chunk_id: String,
    pub entities: HashMap<String, GraphEntity>,
    pub relationships: HashMap<String, GraphRelationship>,
}

/// Configuration for entity extraction
#[derive(Debug, Clone)]
pub struct ExtractionConfig {
    /// Entity types to extract
    pub entity_types: Vec<String>,
    /// Language for extraction
    pub language: String,
    /// Maximum gleaning iterations
    pub max_gleaning: usize,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            entity_types: vec![
                "person".to_string(),
                "organization".to_string(),
                "location".to_string(),
                "event".to_string(),
                "concept".to_string(),
            ],
            language: "English".to_string(),
            max_gleaning: 1,
        }
    }
}

/// Build the entity extraction prompt
/// Reference: docs_retro/05-algorithms.md#2-entity--relationship-extraction-algorithm
fn build_extraction_prompt(
    content: &str,
    config: &ExtractionConfig,
) -> String {
    let entity_types = config.entity_types.join(", ");
    
    format!(r#"
-Goal-
Given a text document, identify all entities and relationships.

-Steps-
1. Identify all entities. For each entity, extract:
   - entity_name: Name of the entity (capitalized)
   - entity_type: One of [{entity_types}]
   - description: Comprehensive description

2. Identify relationships between entities. For each relationship, extract:
   - source_entity: Name of source entity
   - target_entity: Name of target entity  
   - relationship_description: Description of the relationship
   - relationship_keywords: Key terms describing the relationship
   - relationship_strength: Float between 0-1

-Output Format-
Return output as a list of tuples using {TUPLE_DELIMITER} as delimiter.

For entities:
entity{TUPLE_DELIMITER}ENTITY_NAME{TUPLE_DELIMITER}ENTITY_TYPE{TUPLE_DELIMITER}DESCRIPTION

For relationships:
relationship{TUPLE_DELIMITER}SOURCE{TUPLE_DELIMITER}TARGET{TUPLE_DELIMITER}DESCRIPTION{TUPLE_DELIMITER}KEYWORDS

End with {COMPLETION_DELIMITER}

-Language-
Use {language}.

-Text-
{content}
"#,
        entity_types = entity_types,
        TUPLE_DELIMITER = TUPLE_DELIMITER,
        COMPLETION_DELIMITER = COMPLETION_DELIMITER,
        language = config.language,
        content = content,
    )
}

/// Parse the LLM extraction output
/// Reference: docs_retro/05-algorithms.md#function-parse_extraction_result
fn parse_extraction_result(
    result: &str,
    chunk_id: &str,
) -> ChunkExtractionResult {
    let mut entities: HashMap<String, GraphEntity> = HashMap::new();
    let mut relationships: HashMap<String, GraphRelationship> = HashMap::new();
    
    // Split by newlines and completion delimiter
    let records: Vec<&str> = result
        .split(|c| c == '\n' || result.contains(COMPLETION_DELIMITER))
        .filter(|s| !s.trim().is_empty())
        .collect();
    
    for record in records {
        let record = record.trim();
        if record.contains(COMPLETION_DELIMITER) {
            continue;
        }
        
        let fields: Vec<&str> = record.split(TUPLE_DELIMITER).collect();
        
        if fields.is_empty() {
            continue;
        }
        
        let record_type = fields[0].trim().to_lowercase();
        
        if record_type == "entity" && fields.len() >= 4 {
            let entity_name = GraphEntity::normalize_name(fields[1].trim());
            let entity_type = fields[2].trim().to_lowercase();
            let description = fields[3].trim().to_string();
            
            if !entity_name.is_empty() {
                let entity = GraphEntity {
                    id: entity_name.clone(),
                    entity_name: entity_name.clone(),
                    entity_type,
                    description,
                    source_id: chunk_id.to_string(),
                    file_path: None,
                    created_at: chrono::Utc::now(),
                };
                
                entities.insert(entity_name, entity);
            }
        } else if record_type == "relationship" && fields.len() >= 5 {
            let source = GraphEntity::normalize_name(fields[1].trim());
            let target = GraphEntity::normalize_name(fields[2].trim());
            let description = fields[3].trim().to_string();
            let keywords = if fields.len() > 4 {
                Some(fields[4].trim().to_string())
            } else {
                None
            };
            
            if !source.is_empty() && !target.is_empty() {
                let rel_id = GraphRelationship::generate_id(&source, &target);
                
                let relationship = GraphRelationship {
                    id: rel_id.clone(),
                    source_entity: source,
                    target_entity: target,
                    description,
                    keywords,
                    weight: 1.0,
                    source_id: chunk_id.to_string(),
                    file_path: None,
                    created_at: chrono::Utc::now(),
                };
                
                relationships.insert(rel_id, relationship);
            }
        }
    }
    
    ChunkExtractionResult {
        chunk_id: chunk_id.to_string(),
        entities,
        relationships,
    }
}

/// Extract entities and relationships from a chunk
pub async fn extract_entities_from_chunk<P: LLMProvider>(
    llm_provider: &P,
    chunk_content: &str,
    chunk_id: &str,
    config: &ExtractionConfig,
) -> Result<ChunkExtractionResult, PipelineError> {
    let prompt = build_extraction_prompt(chunk_content, config);
    
    let messages = vec![
        ChatMessage {
            role: MessageRole::System,
            content: "You are an expert knowledge graph builder. Extract entities and relationships precisely.".to_string(),
        },
        ChatMessage {
            role: MessageRole::User,
            content: prompt,
        },
    ];
    
    let options = CompletionOptions {
        temperature: Some(0.0),
        max_tokens: Some(4096),
        stream: false,
    };
    
    let response = llm_provider
        .chat_completion(messages, options)
        .await
        .map_err(|e| PipelineError::ExtractionFailed(e.to_string()))?;
    
    let result = parse_extraction_result(&response, chunk_id);
    
    // Optional: Gleaning for more entities
    // TODO: Implement gleaning loop if config.max_gleaning > 0
    
    Ok(result)
}

/// Extract from multiple chunks in parallel
pub async fn extract_entities_from_chunks<P: LLMProvider + Clone>(
    llm_provider: &P,
    chunks: &[(String, String)], // (chunk_id, content)
    config: &ExtractionConfig,
    max_concurrent: usize,
) -> Result<Vec<ChunkExtractionResult>, PipelineError> {
    use futures::stream::{self, StreamExt};
    
    let results: Vec<Result<ChunkExtractionResult, PipelineError>> = stream::iter(chunks)
        .map(|(chunk_id, content)| {
            let llm = llm_provider.clone();
            let cfg = config.clone();
            async move {
                extract_entities_from_chunk(&llm, content, chunk_id, &cfg).await
            }
        })
        .buffer_unordered(max_concurrent)
        .collect()
        .await;
    
    results.into_iter().collect()
}
```

---

## Week 6: Merging & Embeddings

### 6.1 Entity Merging Algorithm

```rust
// edgequake-pipeline/src/merging.rs
use edgequake_core::types::{GraphEntity, GraphRelationship};
use edgequake_storage::traits::graph::GraphStorage;
use edgequake_storage::traits::vector::VectorStorage;
use edgequake_llm::traits::{LLMProvider, EmbeddingProvider};
use crate::error::PipelineError;
use crate::extraction::ChunkExtractionResult;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for merging
#[derive(Debug, Clone)]
pub struct MergingConfig {
    /// Maximum tokens before summarizing descriptions
    pub summary_threshold_tokens: usize,
    /// Maximum source IDs per entity
    pub max_source_ids_per_entity: usize,
    /// Maximum source IDs per relation
    pub max_source_ids_per_relation: usize,
}

impl Default for MergingConfig {
    fn default() -> Self {
        Self {
            summary_threshold_tokens: 1200,
            max_source_ids_per_entity: 300,
            max_source_ids_per_relation: 300,
        }
    }
}

/// Keyed lock manager for concurrent entity updates
pub struct KeyedLocks {
    locks: RwLock<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl KeyedLocks {
    pub fn new() -> Self {
        Self {
            locks: RwLock::new(HashMap::new()),
        }
    }
    
    pub async fn get_lock(&self, key: &str) -> Arc<tokio::sync::Mutex<()>> {
        {
            let locks = self.locks.read().await;
            if let Some(lock) = locks.get(key) {
                return lock.clone();
            }
        }
        
        let mut locks = self.locks.write().await;
        locks.entry(key.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
}

/// Merge source IDs with limit
fn merge_source_ids(existing: &str, new_ids: &[String], limit: usize) -> String {
    let mut all_ids: Vec<&str> = existing
        .split('|')
        .filter(|s| !s.is_empty())
        .collect();
    
    for id in new_ids {
        if !all_ids.contains(&id.as_str()) {
            all_ids.push(id);
        }
    }
    
    // Keep most recent (FIFO strategy - drop oldest)
    if all_ids.len() > limit {
        all_ids = all_ids[all_ids.len() - limit..].to_vec();
    }
    
    all_ids.join("|")
}

/// Aggregate descriptions with separator
fn aggregate_descriptions(descriptions: &[String]) -> String {
    descriptions.join("\n---\n")
}

/// Merge extraction results into the knowledge graph
/// Reference: docs_retro/05-algorithms.md#3-entityrelationship-merging-algorithm
pub async fn merge_extraction_results<G, V, E>(
    graph_storage: &G,
    entity_vdb: &V,
    relation_vdb: &V,
    embedding_provider: &E,
    extraction_results: Vec<ChunkExtractionResult>,
    config: &MergingConfig,
    entity_locks: &KeyedLocks,
) -> Result<MergeStats, PipelineError>
where
    G: GraphStorage,
    V: VectorStorage,
    E: EmbeddingProvider,
{
    let mut stats = MergeStats::default();
    
    // Group entities by name
    let mut all_entities: HashMap<String, Vec<GraphEntity>> = HashMap::new();
    let mut all_relationships: HashMap<String, Vec<GraphRelationship>> = HashMap::new();
    
    for result in extraction_results {
        for (name, entity) in result.entities {
            all_entities.entry(name).or_default().push(entity);
        }
        for (key, relationship) in result.relationships {
            all_relationships.entry(key).or_default().push(relationship);
        }
    }
    
    // Process entities
    for (entity_name, entity_list) in all_entities {
        let lock = entity_locks.get_lock(&entity_name).await;
        let _guard = lock.lock().await;
        
        let existing = graph_storage.get_node(&entity_name).await
            .map_err(|e| PipelineError::MergeConflict(e.to_string()))?;
        
        let (merged_description, merged_source_ids) = if let Some(existing_node) = existing {
            // Merge with existing
            stats.entities_updated += 1;
            
            let existing_desc = existing_node.properties
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let existing_source_id = existing_node.properties
                .get("source_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            
            let all_descriptions: Vec<String> = std::iter::once(existing_desc.to_string())
                .chain(entity_list.iter().map(|e| e.description.clone()))
                .collect();
            
            let merged_desc = aggregate_descriptions(&all_descriptions);
            
            let new_source_ids: Vec<String> = entity_list.iter()
                .map(|e| e.source_id.clone())
                .collect();
            let merged_sources = merge_source_ids(
                existing_source_id,
                &new_source_ids,
                config.max_source_ids_per_entity,
            );
            
            (merged_desc, merged_sources)
        } else {
            // New entity
            stats.entities_created += 1;
            
            let descriptions: Vec<String> = entity_list.iter()
                .map(|e| e.description.clone())
                .collect();
            let merged_desc = aggregate_descriptions(&descriptions);
            
            let source_ids: Vec<String> = entity_list.iter()
                .map(|e| e.source_id.clone())
                .collect();
            let merged_sources = source_ids.join("|");
            
            (merged_desc, merged_sources)
        };
        
        // Update graph node
        let mut properties = HashMap::new();
        properties.insert("entity_name".to_string(), serde_json::json!(entity_name.clone()));
        properties.insert("entity_type".to_string(), serde_json::json!(entity_list[0].entity_type.clone()));
        properties.insert("description".to_string(), serde_json::json!(merged_description.clone()));
        properties.insert("source_id".to_string(), serde_json::json!(merged_source_ids.clone()));
        properties.insert("created_at".to_string(), serde_json::json!(chrono::Utc::now().timestamp()));
        
        graph_storage.upsert_node(&entity_name, properties).await
            .map_err(|e| PipelineError::MergeConflict(e.to_string()))?;
        
        // Update entity embedding
        let embed_content = format!("{}\n{}", entity_name, merged_description);
        let embeddings = embedding_provider.embed(&[embed_content.clone()]).await
            .map_err(|e| PipelineError::EmbeddingFailed(e.to_string()))?;
        
        if let Some(embedding) = embeddings.first() {
            let metadata = serde_json::json!({
                "entity_name": entity_name,
                "entity_type": entity_list[0].entity_type,
                "source_id": merged_source_ids,
            });
            
            entity_vdb.upsert(&[(
                format!("ent-{}", entity_name),
                embedding.clone(),
                metadata,
            )]).await
                .map_err(|e| PipelineError::EmbeddingFailed(e.to_string()))?;
        }
    }
    
    // Process relationships
    for (rel_key, rel_list) in all_relationships {
        let lock = entity_locks.get_lock(&rel_key).await;
        let _guard = lock.lock().await;
        
        let (src, tgt) = {
            let parts: Vec<&str> = rel_key.split("<SEP>").collect();
            if parts.len() != 2 {
                continue;
            }
            (parts[0].to_string(), parts[1].to_string())
        };
        
        let existing = graph_storage.get_edge(&src, &tgt).await
            .map_err(|e| PipelineError::MergeConflict(e.to_string()))?;
        
        let (merged_description, merged_keywords, merged_weight, merged_source_ids) = 
            if let Some(existing_edge) = existing {
                stats.relationships_updated += 1;
                
                let existing_desc = existing_edge.properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let existing_weight = existing_edge.properties
                    .get("weight")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.0) as f32;
                let existing_source_id = existing_edge.properties
                    .get("source_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let existing_keywords = existing_edge.properties
                    .get("keywords")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                
                let all_descriptions: Vec<String> = std::iter::once(existing_desc.to_string())
                    .chain(rel_list.iter().map(|r| r.description.clone()))
                    .collect();
                let merged_desc = aggregate_descriptions(&all_descriptions);
                
                let all_keywords: Vec<&str> = existing_keywords.split('|')
                    .chain(rel_list.iter().filter_map(|r| r.keywords.as_deref()))
                    .filter(|s| !s.is_empty())
                    .collect();
                let merged_kw = all_keywords.join("|");
                
                let new_weight = existing_weight + rel_list.iter().map(|r| r.weight).sum::<f32>();
                
                let new_source_ids: Vec<String> = rel_list.iter()
                    .map(|r| r.source_id.clone())
                    .collect();
                let merged_sources = merge_source_ids(
                    existing_source_id,
                    &new_source_ids,
                    config.max_source_ids_per_relation,
                );
                
                (merged_desc, merged_kw, new_weight, merged_sources)
            } else {
                stats.relationships_created += 1;
                
                let descriptions: Vec<String> = rel_list.iter()
                    .map(|r| r.description.clone())
                    .collect();
                let merged_desc = aggregate_descriptions(&descriptions);
                
                let keywords: Vec<&str> = rel_list.iter()
                    .filter_map(|r| r.keywords.as_deref())
                    .collect();
                let merged_kw = keywords.join("|");
                
                let weight: f32 = rel_list.iter().map(|r| r.weight).sum();
                
                let source_ids: Vec<String> = rel_list.iter()
                    .map(|r| r.source_id.clone())
                    .collect();
                let merged_sources = source_ids.join("|");
                
                (merged_desc, merged_kw, weight, merged_sources)
            };
        
        // Update graph edge
        let mut properties = HashMap::new();
        properties.insert("description".to_string(), serde_json::json!(merged_description.clone()));
        properties.insert("keywords".to_string(), serde_json::json!(merged_keywords.clone()));
        properties.insert("weight".to_string(), serde_json::json!(merged_weight));
        properties.insert("source_id".to_string(), serde_json::json!(merged_source_ids.clone()));
        properties.insert("created_at".to_string(), serde_json::json!(chrono::Utc::now().timestamp()));
        
        graph_storage.upsert_edge(&src, &tgt, properties).await
            .map_err(|e| PipelineError::MergeConflict(e.to_string()))?;
        
        // Update relationship embedding
        let embed_content = format!("{}\t{}\n{}\n{}", merged_keywords, src, tgt, merged_description);
        let embeddings = embedding_provider.embed(&[embed_content.clone()]).await
            .map_err(|e| PipelineError::EmbeddingFailed(e.to_string()))?;
        
        if let Some(embedding) = embeddings.first() {
            let metadata = serde_json::json!({
                "src_id": src,
                "tgt_id": tgt,
                "keywords": merged_keywords,
                "source_id": merged_source_ids,
            });
            
            relation_vdb.upsert(&[(
                format!("rel-{}", rel_key),
                embedding.clone(),
                metadata,
            )]).await
                .map_err(|e| PipelineError::EmbeddingFailed(e.to_string()))?;
        }
    }
    
    Ok(stats)
}

/// Statistics from merge operation
#[derive(Debug, Default)]
pub struct MergeStats {
    pub entities_created: usize,
    pub entities_updated: usize,
    pub relationships_created: usize,
    pub relationships_updated: usize,
}
```

---

## Week 7: Query Modes

### 7.1 Query Engine

```rust
// edgequake-query/src/lib.rs
mod modes;
mod context;
mod response;

pub use modes::*;
pub use context::*;
pub use response::*;

use edgequake_storage::traits::{graph::GraphStorage, vector::VectorStorage, kv::KVStorage};
use edgequake_llm::traits::{LLMProvider, EmbeddingProvider};
use crate::error::QueryError;

/// Query mode selection
/// Reference: docs_retro/05-algorithms.md#4-query-processing-algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryMode {
    /// Direct chunk retrieval via vector similarity
    Naive,
    /// Entity-centric search
    Local,
    /// Relationship-centric search
    Global,
    /// Combined local and global
    Hybrid,
    /// Skip retrieval, direct LLM chat
    Bypass,
}

/// Query parameters
#[derive(Debug, Clone)]
pub struct QueryParam {
    pub mode: QueryMode,
    pub top_k: usize,
    pub only_need_context: bool,
    pub stream: bool,
    pub conversation_history: Vec<ChatMessage>,
}

impl Default for QueryParam {
    fn default() -> Self {
        Self {
            mode: QueryMode::Hybrid,
            top_k: 40,
            only_need_context: false,
            stream: false,
            conversation_history: Vec::new(),
        }
    }
}

/// Query result
#[derive(Debug)]
pub struct QueryResult {
    pub response: Option<String>,
    pub context: QueryContext,
    pub is_streaming: bool,
}

/// Query engine
pub struct QueryEngine<G, V, K, L, E>
where
    G: GraphStorage,
    V: VectorStorage,
    K: KVStorage,
    L: LLMProvider,
    E: EmbeddingProvider,
{
    graph_storage: G,
    entity_vdb: V,
    relation_vdb: V,
    chunks_vdb: V,
    chunk_kv: K,
    llm_provider: L,
    embedding_provider: E,
}

impl<G, V, K, L, E> QueryEngine<G, V, K, L, E>
where
    G: GraphStorage,
    V: VectorStorage,
    K: KVStorage,
    L: LLMProvider,
    E: EmbeddingProvider,
{
    pub fn new(
        graph_storage: G,
        entity_vdb: V,
        relation_vdb: V,
        chunks_vdb: V,
        chunk_kv: K,
        llm_provider: L,
        embedding_provider: E,
    ) -> Self {
        Self {
            graph_storage,
            entity_vdb,
            relation_vdb,
            chunks_vdb,
            chunk_kv,
            llm_provider,
            embedding_provider,
        }
    }
    
    /// Process a query
    pub async fn query(
        &self,
        query: &str,
        param: QueryParam,
        system_prompt: Option<&str>,
    ) -> Result<QueryResult, QueryError> {
        if query.trim().is_empty() {
            return Err(QueryError::EmptyQuery);
        }
        
        // Get context based on mode
        let context = match param.mode {
            QueryMode::Naive => self.naive_query(query, param.top_k).await?,
            QueryMode::Local => self.local_query(query, param.top_k).await?,
            QueryMode::Global => self.global_query(query, param.top_k).await?,
            QueryMode::Hybrid => self.hybrid_query(query, param.top_k).await?,
            QueryMode::Bypass => QueryContext::default(),
        };
        
        if param.only_need_context {
            return Ok(QueryResult {
                response: None,
                context,
                is_streaming: false,
            });
        }
        
        // Generate response
        let formatted_context = context.format_for_llm();
        let response = self.generate_response(
            query,
            &formatted_context,
            &param.conversation_history,
            system_prompt,
        ).await?;
        
        Ok(QueryResult {
            response: Some(response),
            context,
            is_streaming: param.stream,
        })
    }
    
    /// Naive query mode - direct chunk retrieval
    async fn naive_query(&self, query: &str, top_k: usize) -> Result<QueryContext, QueryError> {
        // Embed query
        let query_embedding = self.embedding_provider
            .embed(&[query.to_string()])
            .await
            .map_err(|e| QueryError::ContextRetrievalFailed(e.to_string()))?;
        
        let query_vec = query_embedding.first()
            .ok_or_else(|| QueryError::ContextRetrievalFailed("No embedding generated".to_string()))?;
        
        // Search chunks
        let results = self.chunks_vdb
            .query(query_vec, top_k, None)
            .await
            .map_err(|e| QueryError::ContextRetrievalFailed(e.to_string()))?;
        
        let mut context = QueryContext::default();
        for result in results {
            context.chunks.push(ContextChunk {
                id: result.id,
                content: result.metadata.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                score: result.score,
            });
        }
        
        Ok(context)
    }
    
    /// Local query mode - entity-centric
    async fn local_query(&self, query: &str, top_k: usize) -> Result<QueryContext, QueryError> {
        let query_embedding = self.embedding_provider
            .embed(&[query.to_string()])
            .await
            .map_err(|e| QueryError::ContextRetrievalFailed(e.to_string()))?;
        
        let query_vec = query_embedding.first()
            .ok_or_else(|| QueryError::ContextRetrievalFailed("No embedding generated".to_string()))?;
        
        // Search entities
        let entity_results = self.entity_vdb
            .query(query_vec, top_k, None)
            .await
            .map_err(|e| QueryError::ContextRetrievalFailed(e.to_string()))?;
        
        let mut context = QueryContext::default();
        
        for result in entity_results {
            let entity_name = result.metadata.get("entity_name")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            
            // Get entity details from graph
            if let Ok(Some(node)) = self.graph_storage.get_node(entity_name).await {
                context.entities.push(ContextEntity {
                    name: entity_name.to_string(),
                    entity_type: node.properties.get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    description: node.properties.get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    score: result.score,
                });
                
                // Get related edges
                if let Ok(edges) = self.graph_storage.get_node_edges(entity_name).await {
                    for edge in edges {
                        context.relationships.push(ContextRelationship {
                            source: edge.source,
                            target: edge.target,
                            description: edge.properties.get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            keywords: edge.properties.get("keywords")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            weight: edge.properties.get("weight")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(1.0) as f32,
                        });
                    }
                }
            }
        }
        
        Ok(context)
    }
    
    /// Global query mode - relationship-centric
    async fn global_query(&self, query: &str, top_k: usize) -> Result<QueryContext, QueryError> {
        let query_embedding = self.embedding_provider
            .embed(&[query.to_string()])
            .await
            .map_err(|e| QueryError::ContextRetrievalFailed(e.to_string()))?;
        
        let query_vec = query_embedding.first()
            .ok_or_else(|| QueryError::ContextRetrievalFailed("No embedding generated".to_string()))?;
        
        // Search relationships
        let relation_results = self.relation_vdb
            .query(query_vec, top_k, None)
            .await
            .map_err(|e| QueryError::ContextRetrievalFailed(e.to_string()))?;
        
        let mut context = QueryContext::default();
        
        for result in relation_results {
            let src = result.metadata.get("src_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let tgt = result.metadata.get("tgt_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            
            if let Ok(Some(edge)) = self.graph_storage.get_edge(src, tgt).await {
                context.relationships.push(ContextRelationship {
                    source: src.to_string(),
                    target: tgt.to_string(),
                    description: edge.properties.get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    keywords: edge.properties.get("keywords")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    weight: edge.properties.get("weight")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.0) as f32,
                });
            }
        }
        
        // Get high-degree entities
        if let Ok(popular) = self.graph_storage.get_popular_labels(10).await {
            for entity_name in popular {
                if let Ok(Some(node)) = self.graph_storage.get_node(&entity_name).await {
                    context.entities.push(ContextEntity {
                        name: entity_name,
                        entity_type: node.properties.get("entity_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        description: node.properties.get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        score: 0.0,
                    });
                }
            }
        }
        
        Ok(context)
    }
    
    /// Hybrid query mode - combined local and global
    async fn hybrid_query(&self, query: &str, top_k: usize) -> Result<QueryContext, QueryError> {
        let local_ctx = self.local_query(query, top_k / 2).await?;
        let global_ctx = self.global_query(query, top_k / 2).await?;
        
        // Merge contexts
        let mut context = local_ctx;
        
        for entity in global_ctx.entities {
            if !context.entities.iter().any(|e| e.name == entity.name) {
                context.entities.push(entity);
            }
        }
        
        for rel in global_ctx.relationships {
            if !context.relationships.iter().any(|r| r.source == rel.source && r.target == rel.target) {
                context.relationships.push(rel);
            }
        }
        
        Ok(context)
    }
    
    /// Generate LLM response
    async fn generate_response(
        &self,
        query: &str,
        context: &str,
        history: &[ChatMessage],
        system_prompt: Option<&str>,
    ) -> Result<String, QueryError> {
        use edgequake_llm::traits::{ChatMessage, MessageRole, CompletionOptions};
        
        let default_system = "You are a helpful assistant. Answer the question based on the provided context.";
        let system = system_prompt.unwrap_or(default_system);
        
        let mut messages = vec![
            ChatMessage {
                role: MessageRole::System,
                content: system.to_string(),
            },
        ];
        
        // Add history
        messages.extend(history.iter().cloned());
        
        // Add context and question
        let user_message = format!(
            "Context:\n{}\n\nQuestion: {}",
            context,
            query
        );
        
        messages.push(ChatMessage {
            role: MessageRole::User,
            content: user_message,
        });
        
        let options = CompletionOptions {
            temperature: Some(0.7),
            max_tokens: Some(2048),
            stream: false,
        };
        
        self.llm_provider
            .chat_completion(messages, options)
            .await
            .map_err(|e| QueryError::GenerationFailed(e.to_string()))
    }
}
```

---

## Week 8: REST API

### 8.1 Axum Routes

```rust
// edgequake-api/src/routes/documents.rs
use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::AppState;
use crate::error::ApiError;

/// Insert documents request
/// Reference: docs_retro/04-api-contracts.md#method-insert--ainsert
#[derive(Debug, Deserialize)]
pub struct InsertRequest {
    /// Document content (single or multiple)
    pub content: StringOrVec,
    /// Optional document IDs
    pub ids: Option<Vec<String>>,
    /// Optional file paths
    pub file_paths: Option<Vec<String>>,
    /// Optional split character
    pub split_by_character: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum StringOrVec {
    Single(String),
    Multiple(Vec<String>),
}

/// Insert response
#[derive(Debug, Serialize)]
pub struct InsertResponse {
    pub track_id: String,
    pub document_count: usize,
    pub status: String,
}

/// POST /documents - Insert documents
#[utoipa::path(
    post,
    path = "/documents",
    request_body = InsertRequest,
    responses(
        (status = 200, description = "Documents queued", body = InsertResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal error")
    ),
    tag = "documents"
)]
pub async fn insert_documents(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InsertRequest>,
) -> Result<Json<InsertResponse>, ApiError> {
    let contents = match req.content {
        StringOrVec::Single(s) => vec![s],
        StringOrVec::Multiple(v) => v,
    };
    
    if contents.is_empty() {
        return Err(ApiError::BadRequest("Content cannot be empty".to_string()));
    }
    
    let track_id = state.rag
        .insert(contents.clone(), req.ids, req.file_paths)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    
    Ok(Json(InsertResponse {
        track_id,
        document_count: contents.len(),
        status: "queued".to_string(),
    }))
}

/// GET /documents/{id} - Get document status
#[utoipa::path(
    get,
    path = "/documents/{id}",
    params(
        ("id" = String, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document status"),
        (status = 404, description = "Document not found")
    ),
    tag = "documents"
)]
pub async fn get_document(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DocumentStatus>, ApiError> {
    let status = state.rag
        .get_document_status(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Document {} not found", id)))?;
    
    Ok(Json(status))
}

/// DELETE /documents/{id} - Delete document
#[utoipa::path(
    delete,
    path = "/documents/{id}",
    params(
        ("id" = String, Path, description = "Document ID")
    ),
    responses(
        (status = 200, description = "Document deleted"),
        (status = 404, description = "Document not found")
    ),
    tag = "documents"
)]
pub async fn delete_document(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DeletionResult>, ApiError> {
    let result = state.rag
        .delete_by_doc_id(&id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    
    Ok(Json(result))
}
```

### 8.2 Query Routes

```rust
// edgequake-api/src/routes/query.rs
use axum::{
    extract::{State, Json},
    response::sse::{Event, Sse},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use futures::stream::Stream;
use crate::AppState;
use crate::error::ApiError;

/// Query request
/// Reference: docs_retro/04-api-contracts.md#method-query--aquery
#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    /// Query text
    pub query: String,
    /// Query mode (naive, local, global, hybrid, bypass)
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Number of results to retrieve
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// Return only context without generation
    #[serde(default)]
    pub only_need_context: bool,
    /// Enable streaming response
    #[serde(default)]
    pub stream: bool,
    /// Custom system prompt
    pub system_prompt: Option<String>,
    /// Conversation history
    #[serde(default)]
    pub conversation_history: Vec<HistoryMessage>,
}

fn default_mode() -> String { "hybrid".to_string() }
fn default_top_k() -> usize { 40 }

#[derive(Debug, Deserialize, Clone)]
pub struct HistoryMessage {
    pub role: String,
    pub content: String,
}

/// Query response
#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub response: Option<String>,
    pub context: Option<ContextData>,
    pub mode: String,
}

#[derive(Debug, Serialize)]
pub struct ContextData {
    pub entities: Vec<EntityInfo>,
    pub relationships: Vec<RelationInfo>,
    pub chunks: Vec<ChunkInfo>,
}

/// POST /query - Execute query
#[utoipa::path(
    post,
    path = "/query",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Query result", body = QueryResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal error")
    ),
    tag = "query"
)]
pub async fn execute_query(
    State(state): State<Arc<AppState>>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, ApiError> {
    use edgequake_query::QueryMode;
    
    if req.query.trim().is_empty() {
        return Err(ApiError::BadRequest("Query cannot be empty".to_string()));
    }
    
    let mode = match req.mode.to_lowercase().as_str() {
        "naive" => QueryMode::Naive,
        "local" => QueryMode::Local,
        "global" => QueryMode::Global,
        "hybrid" => QueryMode::Hybrid,
        "bypass" => QueryMode::Bypass,
        _ => return Err(ApiError::BadRequest(format!("Invalid mode: {}", req.mode))),
    };
    
    let param = edgequake_query::QueryParam {
        mode,
        top_k: req.top_k,
        only_need_context: req.only_need_context,
        stream: false,
        conversation_history: req.conversation_history.into_iter().map(|m| {
            edgequake_llm::traits::ChatMessage {
                role: match m.role.as_str() {
                    "user" => edgequake_llm::traits::MessageRole::User,
                    "assistant" => edgequake_llm::traits::MessageRole::Assistant,
                    _ => edgequake_llm::traits::MessageRole::User,
                },
                content: m.content,
            }
        }).collect(),
    };
    
    let result = state.rag
        .query(&req.query, param, req.system_prompt.as_deref())
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    
    Ok(Json(QueryResponse {
        response: result.response,
        context: if req.only_need_context {
            Some(result.context.into())
        } else {
            None
        },
        mode: req.mode,
    }))
}

/// POST /query/stream - Streaming query
#[utoipa::path(
    post,
    path = "/query/stream",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Streaming response"),
        (status = 400, description = "Invalid request")
    ),
    tag = "query"
)]
pub async fn execute_query_stream(
    State(state): State<Arc<AppState>>,
    Json(req): Json<QueryRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, ApiError>>>, ApiError> {
    // Implementation for SSE streaming
    todo!("Implement streaming response")
}
```

### 8.3 OpenAPI Documentation

```rust
// edgequake-api/src/openapi.rs
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::documents::insert_documents,
        routes::documents::get_document,
        routes::documents::delete_document,
        routes::query::execute_query,
        routes::query::execute_query_stream,
        routes::graph::get_knowledge_graph,
        routes::graph::search_entities,
    ),
    components(
        schemas(
            routes::documents::InsertRequest,
            routes::documents::InsertResponse,
            routes::query::QueryRequest,
            routes::query::QueryResponse,
        )
    ),
    tags(
        (name = "documents", description = "Document management"),
        (name = "query", description = "Knowledge graph queries"),
        (name = "graph", description = "Graph exploration")
    ),
    info(
        title = "EdgeQuake API",
        version = "1.0.0",
        description = "High-performance RAG with knowledge graph",
        license(name = "MIT")
    )
)]
pub struct ApiDoc;

/// Configure Swagger UI
pub fn swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
}
```

---

## Week-by-Week Tasks

### Week 5: Chunking & Extraction

| Task | Description | Owner | Status |
|------|-------------|-------|--------|
| 3.1.1 | Implement chunking_by_token_size | Backend | ⬜ |
| 3.1.2 | Integrate tiktoken-rs | Backend | ⬜ |
| 3.1.3 | Build extraction prompts | Backend | ⬜ |
| 3.1.4 | Parse LLM extraction output | Backend | ⬜ |
| 3.1.5 | Implement async-openai wrapper | Backend | ⬜ |
| 3.1.6 | Write chunking unit tests | QA | ⬜ |
| 3.1.7 | Write extraction tests | QA | ⬜ |

### Week 6: Merging & Embeddings

| Task | Description | Owner | Status |
|------|-------------|-------|--------|
| 3.2.1 | Implement KeyedLocks | Backend | ⬜ |
| 3.2.2 | Implement entity merging | Backend | ⬜ |
| 3.2.3 | Implement relationship merging | Backend | ⬜ |
| 3.2.4 | Implement embedding generation | Backend | ⬜ |
| 3.2.5 | Description summarization | Backend | ⬜ |
| 3.2.6 | Integration test: full pipeline | QA | ⬜ |

### Week 7: Query Modes

| Task | Description | Owner | Status |
|------|-------------|-------|--------|
| 3.3.1 | Implement QueryEngine | Backend | ⬜ |
| 3.3.2 | Implement naive mode | Backend | ⬜ |
| 3.3.3 | Implement local mode | Backend | ⬜ |
| 3.3.4 | Implement global mode | Backend | ⬜ |
| 3.3.5 | Implement hybrid mode | Backend | ⬜ |
| 3.3.6 | Implement bypass mode | Backend | ⬜ |
| 3.3.7 | Query mode tests | QA | ⬜ |

### Week 8: REST API

| Task | Description | Owner | Status |
|------|-------------|-------|--------|
| 3.4.1 | Set up Axum application | Backend | ⬜ |
| 3.4.2 | Implement document routes | Backend | ⬜ |
| 3.4.3 | Implement query routes | Backend | ⬜ |
| 3.4.4 | Implement graph routes | Backend | ⬜ |
| 3.4.5 | Add OpenAPI with utoipa | Backend | ⬜ |
| 3.4.6 | Add CORS and auth middleware | Backend | ⬜ |
| 3.4.7 | API integration tests | QA | ⬜ |
| 3.4.8 | Swagger UI setup | Backend | ⬜ |

---

## Acceptance Criteria

- [ ] Full document→entities→graph pipeline functional
- [ ] All 5 query modes return correct results
- [ ] REST API matches LightRAG specification
- [ ] OpenAPI spec generated automatically
- [ ] End-to-end integration tests pass
- [ ] API responds within 100ms for simple queries
- [ ] Streaming responses work correctly

---

## Dependencies

```toml
[workspace.dependencies]
# Chunking
tiktoken-rs = "0.5"
text-splitter = "0.8"

# LLM
async-openai = "0.16"
reqwest = { version = "0.11", features = ["json"] }

# Web framework
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors"] }

# OpenAPI
utoipa = { version = "4", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "6", features = ["axum"] }

# Streaming
futures = "0.3"
tokio-stream = "0.1"
```

---

## Related Documents

- [Phase 2: Migration Strategy](phase-2-migration-strategy.md) - Previous phase
- [Phase 4: Onboarding Materials](phase-4-onboarding-materials.md) - Next phase
- [master.md](../master.md) - Overall plan
