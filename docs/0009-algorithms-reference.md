# EdgeQuake Algorithms Reference

> Detailed algorithm documentation for the EdgeQuake Graph-Enhanced RAG system

**Version**: 2.0.0 | **Last Updated**: January 2026

> **Implements**: [FEAT0050](features.md#feat0050) Entity Extraction | [FEAT0051](features.md#feat0051) Knowledge Graph | [FEAT0052](features.md#feat0052) Query Modes
> **Business Rules**: [BR0050](business_rules.md#br0050) Normalization Rules | [BR0051](business_rules.md#br0051) Merge Strategy | [BR0052](business_rules.md#br0052) Token Budgeting
> **Use Cases**: [UC0001](use_cases.md#uc0001) Document Ingestion | [UC0002](use_cases.md#uc0002) Knowledge Query
> **Code Reference**: See [edgequake/crates/edgequake-pipeline/](../edgequake/crates/edgequake-pipeline/) for pipeline algorithms and [edgequake/crates/edgequake-query/](../edgequake/crates/edgequake-query/) for query algorithms

---

## Quick Reference

| I want to understand...       | Go to                                         |
| ----------------------------- | --------------------------------------------- |
| Overall pipeline flow         | [Document Ingestion Pipeline](#document-ingestion-pipeline) |
| How entities are extracted    | [Entity Extraction Algorithm](#entity-extraction-algorithm) |
| Multi-pass extraction         | [Gleaning Algorithm](#gleaning-algorithm)     |
| Name normalization            | [Entity Normalization Algorithm](#entity-normalization-algorithm) |
| Knowledge graph building      | [Knowledge Graph Merging Algorithm](#knowledge-graph-merging-algorithm) |
| Query strategies              | [Query Modes](#query-modes-and-retrieval-strategies) |
| Token management              | [Token Budget Management](#token-budget-management) |

---

## Algorithm Summary

| Algorithm | Purpose | Complexity | Key Parameter |
| --------- | ------- | ---------- | ------------- |
| **Chunking** | Split documents | O(n) | `chunk_size` (1200) |
| **Extraction** | Find entities/relations | O(chunks) | `max_entities_per_chunk` (20) |
| **Gleaning** | Multi-pass extraction | O(passes × chunks) | `max_gleaning` (1) |
| **Normalization** | Standardize names | O(1) | Uppercase + underscore |
| **Merging** | Deduplicate graph | O(e log e) | `max_description_length` (4096) |
| **Retrieval** | Find context | O(log n) | `top_k` (10) |
| **Truncation** | Fit token budget | O(items) | `max_context_tokens` (4000) |

---

## Table of Contents

1. [Overview](#overview)
2. [Document Ingestion Pipeline](#document-ingestion-pipeline)
3. [Entity Extraction Algorithm](#entity-extraction-algorithm)
4. [Gleaning Algorithm](#gleaning-algorithm)
5. [Entity Normalization Algorithm](#entity-normalization-algorithm)
6. [Knowledge Graph Merging Algorithm](#knowledge-graph-merging-algorithm)
7. [Query Modes and Retrieval Strategies](#query-modes-and-retrieval-strategies)
8. [Context Truncation Algorithm](#context-truncation-algorithm)
9. [Token Budget Management](#token-budget-management)
10. [Performance Characteristics](#performance-characteristics)
11. [Troubleshooting](#troubleshooting)

---

## Overview

EdgeQuake implements a sophisticated Graph-Enhanced RAG pipeline that combines knowledge graph construction with vector similarity search. The system follows the LightRAG methodology with additional enhancements for production use.

### Core Algorithm Flow

```
Document → Chunk → Extract → Normalize → Merge → Store
                                                    ↓
Query → Embed → Retrieve (Vector + Graph) → Truncate → Generate
```

---

## Document Ingestion Pipeline

> **Code Reference**: [edgequake/crates/edgequake-pipeline/src/pipeline.rs](../edgequake/crates/edgequake-pipeline/src/pipeline.rs)

The document ingestion pipeline processes raw text through multiple stages to build a knowledge graph.

### Pipeline Stages

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      Document Processing Pipeline                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. CHUNKING          Text → [Chunk₁, Chunk₂, ..., Chunkₙ]                  │
│     └─ Token-based splitting with overlap                                   │
│     └─ Preserves sentence boundaries                                        │
│     └─ Default: 1200 tokens/chunk, 100 tokens overlap                       │
│                                                                              │
│  2. EMBEDDING         Chunk → Vector (1536 dimensions for OpenAI)           │
│     └─ Batch processing for efficiency                                      │
│     └─ Stored in VectorStorage for similarity search                        │
│                                                                              │
│  3. EXTRACTION        Chunk → {entities[], relationships[]}                 │
│     └─ LLM-based structured extraction                                      │
│     └─ Parallel processing (configurable concurrency)                       │
│     └─ Optional gleaning for multi-pass extraction                          │
│                                                                              │
│  4. NORMALIZATION     "Marie Curie" → "MARIE_CURIE"                         │
│     └─ Uppercase, underscore-separated                                      │
│     └─ Removes special characters                                           │
│                                                                              │
│  5. MERGING           Deduplicate + Aggregate descriptions                  │
│     └─ Entity deduplication by normalized name                              │
│     └─ Description aggregation (sentence-aware)                             │
│     └─ Relationship weight averaging                                        │
│                                                                              │
│  6. STORAGE           Persist to KV, Vector, and Graph stores               │
│     └─ Entities as graph nodes                                              │
│     └─ Relationships as graph edges                                         │
│     └─ Entity/relationship embeddings for retrieval                         │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Chunking Algorithm

> **Code Reference**: [edgequake/crates/edgequake-pipeline/src/chunker.rs](../edgequake/crates/edgequake-pipeline/src/chunker.rs)

The chunker splits documents into overlapping segments optimized for LLM context windows.

```rust
/// Token estimation: 1 token ≈ 4 characters
fn estimate_tokens(text: &str) -> usize {
    (text.len() as f32 / 4.0).ceil() as usize
}
```

**Configuration Parameters:**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `chunk_size` | 1200 | Target chunk size in tokens |
| `chunk_overlap` | 100 | Overlap between consecutive chunks |
| `min_chunk_size` | 100 | Minimum chunk size (won't create smaller) |
| `preserve_sentences` | true | Avoid splitting mid-sentence |

**Algorithm:**

1. **Split** text by separator hierarchy: `\n\n` → `\n` → `. ` → `! ` → `? ` → `; ` → `, ` → ` `
2. **Accumulate** segments until `chunk_size` is reached
3. **Overlap** by keeping last `chunk_overlap` tokens for next chunk
4. **Return** array of `TextChunk` with content, index, and token count

**Custom Chunking Strategy:**

Implement the `ChunkingStrategy` trait for custom chunking:

```rust
#[async_trait]
pub trait ChunkingStrategy: Send + Sync {
    async fn chunk(&self, content: &str, config: &ChunkerConfig) -> Result<Vec<ChunkResult>>;
    fn name(&self) -> &str;
}
```

---

## Entity Extraction Algorithm

> **Code Reference**: [edgequake/crates/edgequake-pipeline/src/extractor.rs](../edgequake/crates/edgequake-pipeline/src/extractor.rs)

Entity extraction uses LLM-based structured prompts to identify entities and relationships from text chunks.

### Entity Extraction Pipeline

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      ENTITY EXTRACTION PIPELINE                         │
└─────────────────────────────────────────────────────────────────────────┘

    ┌────────────┐     ┌────────────────┐     ┌─────────────────┐
    │ Text Chunk │────▶│  Build Prompt  │────▶│  LLM Provider   │
    │  (512 tok) │     │ (entity types, │     │ (OpenAI/Ollama) │
    └────────────┘     │  output format)│     └────────┬────────┘
                       └────────────────┘              │
                                                       ▼
    ┌─────────────────────────────────────────────────────────────────┐
    │                    LLM JSON Response                             │
    │  {"entities": [...], "relationships": [...]}                     │
    └────────────────────────────────────────┬────────────────────────┘
                                             │
                 ┌───────────────────────────┼───────────────────────────┐
                 ▼                           ▼                           ▼
    ┌────────────────────┐     ┌─────────────────────┐     ┌─────────────────┐
    │  Parse Entities    │     │ Parse Relationships │     │  Error Handling │
    │  - Normalize names │     │ - Link source/target│     │  - JSON repair  │
    │  - Validate types  │     │ - Assign weights    │     │  - Fallback     │
    │  - Score importance│     │ - Extract keywords  │     │    extraction   │
    └─────────┬──────────┘     └──────────┬──────────┘     └─────────────────┘
              │                           │
              ▼                           ▼
    ┌─────────────────────────────────────────────────────────────────┐
    │                   GLEANING (Multi-Pass)                         │
    │  ┌─────────┐    ┌─────────┐    ┌─────────┐                      │
    │  │ Pass 1  │───▶│ Pass 2  │───▶│ Pass N  │  (max_gleaning_iter) │
    │  │ Initial │    │ "What  │    │  Final  │                      │
    │  │ Extract │    │  missed?"│    │  Merge  │                      │
    │  └─────────┘    └─────────┘    └─────────┘                      │
    └────────────────────────────────────────┬────────────────────────┘
                                             │
                                             ▼
    ┌─────────────────────────────────────────────────────────────────┐
    │               DEDUPLICATION & RESOLUTION                         │
    │  - Merge entities by normalized name (MARIE_CURIE = Marie Curie) │
    │  - Prefer longer descriptions                                    │
    │  - Aggregate relationship weights                                │
    └────────────────────────────────────────┬────────────────────────┘
                                             │
                                             ▼
                           ┌─────────────────────────────────┐
                           │  ExtractedEntity[]              │
                           │  ExtractedRelationship[]        │
                           └─────────────────────────────────┘
```

### Extraction Prompt Structure

```
Extract entities and relationships from the following text.

## Entity Types
PERSON, ORGANIZATION, LOCATION, EVENT, CONCEPT, TECHNOLOGY, PRODUCT

## Output Format
Respond with valid JSON in this exact format:
{
  "entities": [
    {"name": "Entity Name", "type": "ENTITY_TYPE", "description": "Brief description"}
  ],
  "relationships": [
    {"source": "Source Entity", "target": "Target Entity", "type": "RELATIONSHIP_TYPE", "description": "Brief description"}
  ]
}

## Text to Analyze
{text}
```

### Extracted Data Structures

```rust
pub struct ExtractedEntity {
    pub name: String,           // "Marie Curie"
    pub entity_type: String,    // "PERSON"
    pub description: String,    // "Polish-French physicist..."
    pub importance: f32,        // 0.0 to 1.0
    pub source_spans: Vec<String>,
    pub embedding: Option<Vec<f32>>,
}

pub struct ExtractedRelationship {
    pub source: String,         // "MARIE_CURIE"
    pub target: String,         // "RADIUM"
    pub relation_type: String,  // "DISCOVERED"
    pub description: String,    // "Marie Curie discovered radium in 1898"
    pub weight: f32,            // 0.0 to 1.0
    pub keywords: Vec<String>,  // ["discovery", "1898", "radioactive"]
    pub embedding: Option<Vec<f32>>,
}
```

### JSON Extraction from LLM Response

The extractor handles various LLM response formats:

```rust
fn extract_json_from_response(response: &str) -> String {
    // 1. Try JSON code block markers: ```json ... ```
    // 2. Try to find JSON starting with { and ending with }
    // 3. Fall back to raw response
}
```

---

## Gleaning Algorithm

> **Code Reference**: [edgequake/crates/edgequake-pipeline/src/extractor.rs](../edgequake/crates/edgequake-pipeline/src/extractor.rs) - `GleaningExtractor`

Gleaning is a multi-pass extraction technique that improves entity and relationship coverage by asking the LLM to look again for missed items.

### Purpose

- **First pass** extracts obvious entities and relationships
- **Gleaning passes** find implicit, contextual, or subtle connections
- **Merge** combines all passes, preferring longer/better descriptions

### Gleaning Prompt

```
MANY entities and relationships were missed in the last extraction.
Please identify any ADDITIONAL entities and relationships that were not already captured.

## Already Identified Entities
{previous_entities_comma_separated}

## Instructions
Look for entities and relationships that were missed in the previous extraction.
Focus on:
- Implicit entities (mentioned indirectly)
- Additional relationships between known entities
- Contextual entities (dates, locations, concepts)

## Text to Re-Analyze
{text}
```

### Gleaning Configuration

```rust
pub struct GleaningConfig {
    pub max_gleaning: usize,    // Default: 1 (0 disables gleaning)
    pub always_glean: bool,     // Default: false
}
```

### Gleaning Merge Strategy

```rust
fn merge_results(&self, original: &mut ExtractionResult, glean_entities, glean_relationships) {
    // For entities: compare normalized names (case-insensitive)
    //   - If exists: keep entity with longer description
    //   - If new: add to result
    
    // For relationships: compare source+target (case-insensitive)
    //   - If exists: keep relationship with longer description
    //   - If new: add to result
}
```

---

## Entity Normalization Algorithm

> **Code Reference**: [edgequake/crates/edgequake-pipeline/src/merger.rs](../edgequake/crates/edgequake-pipeline/src/merger.rs) - `normalize_entity_name()`

Entity names are normalized to ensure consistent graph node IDs across multiple extractions.

### Normalization Rules

```rust
pub fn normalize_entity_name(name: &str) -> String {
    name.trim()                              // Remove leading/trailing whitespace
        .to_uppercase()                      // Convert to uppercase
        .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")  // Remove special chars
        .split_whitespace()                  // Split on whitespace
        .collect::<Vec<_>>()
        .join("_")                           // Join with underscores
}
```

### Examples

| Input | Output |
|-------|--------|
| `"John Doe"` | `"JOHN_DOE"` |
| `"  Hello  World  "` | `"HELLO_WORLD"` |
| `"O'Brien"` | `"OBRIEN"` |
| `"AI/ML"` | `"AIML"` |
| `"New York City"` | `"NEW_YORK_CITY"` |

---

## Knowledge Graph Merging Algorithm

> **Code Reference**: [edgequake/crates/edgequake-pipeline/src/merger.rs](../edgequake/crates/edgequake-pipeline/src/merger.rs) - `KnowledgeGraphMerger`

The merger integrates extracted entities and relationships into the knowledge graph, handling deduplication and description aggregation.

### Merge Process

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Knowledge Graph Merging Algorithm                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  For each ExtractionResult:                                                  │
│                                                                              │
│  1. ENTITY MERGE                                                             │
│     │                                                                        │
│     ├─ Normalize entity name: "Marie Curie" → "MARIE_CURIE"                 │
│     │                                                                        │
│     ├─ Check if entity exists in GraphStorage                               │
│     │   │                                                                    │
│     │   ├─ EXISTS: Update node                                               │
│     │   │   └─ Merge descriptions (avoid duplication)                        │
│     │   │   └─ Update importance (max of existing and new)                   │
│     │   │   └─ Append source spans (up to max_sources)                       │
│     │   │                                                                    │
│     │   └─ NEW: Create node                                                  │
│     │       └─ Set entity_type, description, importance                      │
│     │       └─ Initialize sources list                                       │
│     │                                                                        │
│     └─ Store entity embedding in VectorStorage (for Local query mode)       │
│                                                                              │
│  2. RELATIONSHIP MERGE                                                       │
│     │                                                                        │
│     ├─ Normalize source and target names                                     │
│     │                                                                        │
│     ├─ Ensure both nodes exist (create placeholder if needed)               │
│     │                                                                        │
│     ├─ Check if edge exists in GraphStorage                                  │
│     │   │                                                                    │
│     │   ├─ EXISTS: Update edge                                               │
│     │   │   └─ Merge descriptions                                            │
│     │   │   └─ Average weights: (existing + new) / 2                        │
│     │   │   └─ Merge keywords (deduplicated)                                 │
│     │   │                                                                    │
│     │   └─ NEW: Create edge                                                  │
│     │       └─ Set relation_type, description, weight, keywords              │
│     │                                                                        │
│     └─ Store relationship embedding in VectorStorage (for Global mode)      │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Description Merge Algorithm

```rust
fn merge_descriptions(existing: &str, new: &str, max_length: usize) -> String {
    // 1. If existing is empty, return new (truncated)
    // 2. If new is empty or contained in existing, return existing
    // 3. Split new into sentences
    // 4. Find sentences not already in existing
    // 5. Append unique sentences to existing
    // 6. Truncate at sentence boundary if exceeds max_length
}
```

**Truncation at Sentence Boundary:**

```rust
fn truncate_description(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        return text.to_string();
    }
    
    // Find last sentence boundary (. ! ?) before max_length
    let mut end = max_length;
    for (i, c) in text.char_indices().take(max_length) {
        if c == '.' || c == '!' || c == '?' {
            end = i + 1;
        }
    }
    
    text[..end].to_string()
}
```

### Merge Configuration

```rust
pub struct MergerConfig {
    pub max_description_length: usize,  // Default: 4096
    pub description_decay: f32,         // Default: 0.9
    pub min_importance: f32,            // Default: 0.1
    pub max_sources: usize,             // Default: 10
}
```

---

## Query Modes and Retrieval Strategies

> **Code Reference**: [edgequake/crates/edgequake-query/src/modes.rs](../edgequake/crates/edgequake-query/src/modes.rs) and [edgequake/crates/edgequake-core/src/types/query.rs](../edgequake/crates/edgequake-core/src/types/query.rs)

EdgeQuake supports 6 query modes, each optimized for different use cases.

### Query Mode Overview

| Mode | Vector Search | Graph Traversal | Use Case |
|------|--------------|-----------------|----------|
| **Naive** | ✅ | ❌ | Fast factual lookups |
| **Local** | ✅ | ✅ (entity-centric) | Specific entity questions |
| **Global** | ❌ | ✅ (community-based) | Broad topic questions |
| **Hybrid** | ❌ | ✅ (local + global) | General purpose (default) |
| **Mix** | ✅ | ✅ (all strategies) | Maximum coverage |
| **Bypass** | ❌ | ❌ | Direct LLM (no RAG) |

### Mode Capabilities

```rust
impl QueryMode {
    pub fn uses_vector_search(&self) -> bool {
        matches!(self, Self::Naive | Self::Local | Self::Mix)
    }

    pub fn uses_graph(&self) -> bool {
        matches!(self, Self::Local | Self::Global | Self::Hybrid | Self::Mix)
    }
}
```

### Retrieval Algorithm

> **Code Reference**: [edgequake/crates/edgequake-query/src/engine.rs](../edgequake/crates/edgequake-query/src/engine.rs) - `retrieve_context()`

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Context Retrieval Algorithm                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  INPUT: query, query_embedding, mode, tenant_context                         │
│                                                                              │
│  1. VECTOR SEARCH (if mode.uses_vector_search())                            │
│     │                                                                        │
│     ├─ Query VectorStorage with embedding                                    │
│     ├─ Retrieve top-k chunks (default: 10)                                   │
│     ├─ Filter by min_score threshold (default: 0.1)                         │
│     └─ Apply tenant context filter if set                                    │
│                                                                              │
│  2. GRAPH SEARCH (if mode.uses_graph())                                     │
│     │                                                                        │
│     ├─ Get popular entities (by degree/connectivity)                         │
│     ├─ For each entity (up to max_entities):                                │
│     │   ├─ Get node from GraphStorage                                        │
│     │   ├─ Filter by tenant context                                          │
│     │   ├─ Extract entity_type and description                               │
│     │   ├─ Get node degree                                                   │
│     │   └─ Get connected edges (relationships)                               │
│     └─ Filter relationships by tenant context                                │
│                                                                              │
│  3. CONTEXT ASSEMBLY                                                         │
│     │                                                                        │
│     ├─ Combine chunks, entities, relationships                               │
│     └─ Apply truncation to respect token limits                              │
│                                                                              │
│  OUTPUT: QueryContext { chunks, entities, relationships, token_count }       │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Naive Mode

- **Strategy**: Pure vector similarity search on chunk embeddings
- **Use Case**: Simple factual questions with direct answers
- **Speed**: Fastest (no graph traversal)

### Local Mode

- **Strategy**: Entity-centric search with local neighborhood
- **Algorithm**:
  1. Search entity embeddings for matching entities
  2. Retrieve entity descriptions
  3. Get immediate neighbors (depth=1)
  4. Include connecting relationships

### Global Mode

- **Strategy**: Community-based search using graph structure
- **Algorithm**:
  1. Identify relevant communities/clusters
  2. Retrieve community summaries
  3. Focus on high-level concepts over specific entities

### Hybrid Mode (Default)

- **Strategy**: Combines Local and Global approaches
- **Algorithm**:
  1. Execute Local retrieval
  2. Execute Global retrieval
  3. Merge and deduplicate results
  4. Rank by relevance

### Mix Mode

- **Strategy**: Weighted combination of all strategies
- **Algorithm**:
  1. Execute Naive (vector) search
  2. Execute Local (entity) search
  3. Execute Global (community) search
  4. Apply configurable weights to each source
  5. Merge with unified ranking

### Bypass Mode

- **Strategy**: Skip retrieval entirely
- **Use Case**: General chat, testing, or when RAG isn't needed
- **Behavior**: Send query directly to LLM without context

---

## Context Truncation Algorithm

> **Code Reference**: [edgequake/crates/edgequake-query/src/truncation.rs](../edgequake/crates/edgequake-query/src/truncation.rs)

Context truncation ensures the retrieved context fits within LLM token limits while maintaining information quality.

### Truncation Configuration

```rust
pub struct TruncationConfig {
    pub max_context_tokens: usize,      // Default: 4000
    pub entity_weight: f32,             // Default: 0.3
    pub relationship_weight: f32,       // Default: 0.3
    pub chunk_weight: f32,              // Default: 0.4
    pub min_entities: usize,            // Default: 5
    pub min_relationships: usize,       // Default: 5
    pub min_chunks: usize,              // Default: 3
}
```

### Balanced Context Algorithm

```rust
pub fn balance_context(
    entities: Vec<RetrievedEntity>,
    relationships: Vec<RetrievedRelationship>,
    chunks: Vec<RetrievedChunk>,
    config: &TruncationConfig,
    tokenizer: &dyn Tokenizer,
) -> (Vec<RetrievedEntity>, Vec<RetrievedRelationship>, Vec<RetrievedChunk>) {
    // 1. Calculate token budget for each category
    let entity_budget = (config.max_context_tokens as f32 * config.entity_weight) as usize;
    let relationship_budget = (config.max_context_tokens as f32 * config.relationship_weight) as usize;
    let chunk_budget = (config.max_context_tokens as f32 * config.chunk_weight) as usize;
    
    // 2. Sort each category by relevance/score
    // 3. Take items until budget exhausted, respecting minimums
    // 4. Return truncated lists
}
```

---

## Token Budget Management

> **Code Reference**: [edgequake/crates/edgequake-core/src/token_budget.rs](../edgequake/crates/edgequake-core/src/token_budget.rs)

Token budget management ensures efficient use of LLM context windows across multiple sources.

### Budget Allocation

```rust
pub struct TokenBudget {
    pub total: usize,           // Total available tokens
    pub reserved: usize,        // Reserved for system/prompt
    pub used: usize,            // Currently used
}

pub struct BudgetAllocation {
    pub source: ContextSource,
    pub tokens: usize,
    pub priority: f32,
}

pub enum ContextSource {
    Chunk,
    Entity,
    Relationship,
    ConversationHistory,
    SystemPrompt,
}
```

### Budget Priority

1. **System Prompt** - Always included (highest priority)
2. **Conversation History** - Included for context continuity
3. **Entities** - Core knowledge graph nodes
4. **Relationships** - Connections between entities
5. **Chunks** - Raw document content (lowest priority if tight)

---

## Performance Characteristics

### Ingestion Complexity

| Stage | Complexity | Notes |
|-------|-----------|-------|
| Chunking | O(n) | Linear scan of document |
| Embedding | O(n/b) | n tokens, b batch size |
| Extraction | O(c) | c chunks, LLM calls |
| Merging | O(e log e) | e entities, sorted operations |
| Storage | O(e + r) | e entities, r relationships |

### Query Complexity

| Mode | Complexity | Notes |
|------|-----------|-------|
| Naive | O(log n) | Vector ANN search |
| Local | O(k × d) | k entities, d depth |
| Global | O(c) | c communities |
| Hybrid | O(k × d + c) | Combined |
| Mix | O(log n + k × d + c) | All strategies |

---

## Best Practices

### Entity Extraction

1. **Entity Types**: Define specific entity types for your domain
2. **Gleaning**: Enable for high-quality extraction (at cost of LLM calls)
3. **Batch Size**: Balance between throughput and memory

### Query Optimization

1. **Mode Selection**: Use Naive for speed, Hybrid for quality
2. **Token Limits**: Set appropriate limits for your LLM
3. **Tenant Filtering**: Enable for multi-tenant deployments

### Storage Efficiency

1. **Description Length**: Limit to prevent bloat (4096 chars default)
2. **Source References**: Cap at reasonable number (10 default)
3. **Deduplication**: Normalization prevents duplicate entities

---

## Troubleshooting

### Algorithm Issues

| Symptom | Likely Cause | Solution |
| ------- | ------------ | -------- |
| Few entities extracted | Chunk size too small | Increase `chunk_size` to 1500-2000 |
| Duplicate entities in graph | Normalization not applied | Ensure `normalize_entity_name()` called |
| Query returns empty context | Wrong query mode | Try `hybrid` mode instead of `naive` |
| Context too large error | Token budget exceeded | Reduce `max_context_tokens` or `top_k` |
| Slow ingestion | Too many gleaning passes | Set `max_gleaning` to 0-1 |
| Missing relationships | Entity types too restrictive | Add more entity types to config |
| Low query relevance | Poor embeddings | Verify embedding model matches ingestion |
| Graph traversal timeout | Depth too high | Reduce `max_graph_depth` to 2 |

### Debug Commands

```bash
# Enable algorithm debug logging
export RUST_LOG=edgequake_pipeline=debug,edgequake_query=debug

# Check entity extraction output
cargo run --example extract_entities -- --input sample.txt --verbose

# Verify graph structure
cargo run --example inspect_graph -- --namespace default --stats
```

### Quality Metrics

| Metric | Target | How to Measure |
| ------ | ------ | -------------- |
| Entity deduplication ratio | 30-50% | Unique entities / Total extracted |
| Relationship coverage | 1.5-2.5 per entity | Total relationships / Total entities |
| Query latency P95 | <2s | Monitor `/metrics` endpoint |
| Context utilization | 70-90% | Tokens used / Token budget |

---

## Next Steps

| Document | When to Read |
| -------- | ------------ |
| [Architecture Overview](0002-architecture-overview.md) | Understand system design |
| [Storage Backends](0004-storage-backends.md) | Configure graph and vector storage |
| [LLM Integration](0005-llm-integration.md) | Set up extraction LLM |
| [Configuration Reference](0007-configuration-reference.md) | Tune algorithm parameters |
| [API Reference](0003-api-reference.md) | Use ingestion and query endpoints |

---

**Document Navigation**: [← Multi-Tenancy](0008-multi-tenancy.md) | [README](README.md) | [Quick Start →](0001-quick-start.md)
