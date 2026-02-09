# Deep Dive: Entity Extraction

> **How EdgeQuake Extracts Knowledge Entities from Documents**

Entity extraction is the foundation of EdgeQuake's Graph-RAG system. This document explains the algorithms, strategies, and design decisions behind entity extraction.

---

## Overview

Entity extraction transforms unstructured text into structured knowledge graph nodes:

```
┌─────────────────────────────────────────────────────────────────┐
│                 ENTITY EXTRACTION PIPELINE                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌────────────────────────────────────────────────────────┐     │
│  │ Input: Text Chunk (1200 tokens)                        │     │
│  │                                                        │     │
│  │ "Dr. Sarah Chen at MIT developed a novel approach      │     │
│  │  to neural network optimization using gradient         │     │
│  │  descent with adaptive learning rates..."              │     │
│  └────────────────────────────────────────────────────────┘     │
│                             │                                   │
│                             ▼                                   │
│  ┌────────────────────────────────────────────────────────┐     │
│  │ Entity Extractor (LLM-based)                           │     │
│  │                                                        │     │
│  │ • SOTAExtractor: Tuple-based parsing (production)      │     │
│  │ • LLMExtractor: JSON-based parsing (simple)            │     │
│  │ • GleaningExtractor: Multi-pass extraction             │     │
│  └────────────────────────────────────────────────────────┘     │
│                             │                                   │
│                             ▼                                   │
│  ┌────────────────────────────────────────────────────────┐     │
│  │ Output: ExtractionResult                               │     │
│  │                                                        │     │
│  │ entities:                                              │     │
│  │   - SARAH_CHEN (PERSON): "Researcher at MIT..."        │     │
│  │   - MIT (ORGANIZATION): "Academic institution..."      │     │
│  │   - NEURAL_NETWORK (CONCEPT): "Machine learning..."    │     │
│  │   - GRADIENT_DESCENT (METHOD): "Optimization..."       │     │
│  │                                                        │     │
│  │ relationships:                                         │     │
│  │   - SARAH_CHEN → works_at → MIT                        │     │
│  │   - SARAH_CHEN → developed → NEURAL_NETWORK            │     │
│  │   - NEURAL_NETWORK → uses → GRADIENT_DESCENT           │     │
│  └────────────────────────────────────────────────────────┘     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Why LLM-Based Extraction?

Traditional Named Entity Recognition (NER) systems use trained models with fixed entity types. EdgeQuake uses LLMs for extraction because:

| Traditional NER                       | LLM-Based Extraction            |
| ------------------------------------- | ------------------------------- |
| Fixed entity types (PERSON, ORG, LOC) | Configurable entity types       |
| Requires training data                | Zero-shot, no training          |
| Labels only (no descriptions)         | Rich semantic descriptions      |
| Explicit mentions only                | Infers implicit entities        |
| Rule-based relationships              | Semantic relationship inference |

### The Trade-Off

```
┌─────────────────────────────────────────────────────────────────┐
│                 EXTRACTION APPROACH COMPARISON                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Traditional NER (SpaCy, BERT)        LLM Extraction (GPT-4o)   │
│  ─────────────────────────────        ───────────────────────   │
│  Speed: ~1000 docs/sec                Speed: ~10 docs/sec       │
│  Cost: Free (local)                   Cost: $0.001/doc          │
│  Quality: Fixed patterns              Quality: Semantic understanding
│  Recall: 60-80%                       Recall: 85-95%            │
│  Relationships: None                  Relationships: Inferred   │
│                                                                 │
│  USE WHEN:                            USE WHEN:                 │
│  • High volume, low budget            • Quality matters most    │
│  • Standard entity types              • Domain-specific entities│
│  • Speed is critical                  • Need relationships      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

EdgeQuake chooses LLM extraction because **knowledge graph quality is paramount** for effective RAG.

---

## Extraction Strategies

EdgeQuake provides three extraction strategies:

### 1. SOTAExtractor (Production)

The state-of-the-art extractor uses tuple-based output format from LightRAG research:

```rust
pub struct SOTAExtractor<L>
where
    L: LLMProvider + ?Sized,
{
    llm_provider: Arc<L>,
    entity_types: Vec<String>,
    prompts: EntityExtractionPrompts,
    parser: HybridExtractionParser,
    language: String,
}
```

**Key Features:**

- Tuple-based output: `(ENTITY_NAME|ENTITY_TYPE|DESCRIPTION)`
- More robust parsing than JSON
- Adaptive max_tokens based on chunk complexity
- Automatic retry with token increase on truncation

**Example Output:**

```
("SARAH_CHEN"|"PERSON"|"Research scientist at MIT specializing in neural networks")
("MIT"|"ORGANIZATION"|"Massachusetts Institute of Technology, leading research university")
("SARAH_CHEN"|"MIT"|"works_at"|"Dr. Chen is a researcher at MIT's AI Lab")
```

### 2. LLMExtractor (Development)

Simpler JSON-based extraction for development and testing:

```rust
pub struct LLMExtractor<L>
where
    L: LLMProvider + ?Sized,
{
    llm_provider: Arc<L>,
    entity_types: Vec<String>,
}
```

**JSON Format:**

```json
{
  "entities": [
    { "name": "Sarah Chen", "type": "PERSON", "description": "..." }
  ],
  "relationships": [
    { "source": "Sarah Chen", "target": "MIT", "type": "works_at" }
  ]
}
```

### 3. GleaningExtractor (High-Stakes)

Multi-pass extraction for thorough entity discovery:

```rust
pub struct GleaningExtractor {
    llm_provider: Arc<dyn LLMProvider>,
    base_extractor: Arc<dyn EntityExtractor>,
    config: GleaningConfig,
}

pub struct GleaningConfig {
    pub max_gleaning: usize,    // Default: 1
    pub always_glean: bool,     // Default: false
}
```

---

## The EntityExtractor Trait

All extractors implement a common trait:

```rust
#[async_trait]
pub trait EntityExtractor: Send + Sync {
    /// Extract entities and relationships from a text chunk.
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult>;

    /// Extract from multiple chunks in batch.
    async fn extract_batch(&self, chunks: &[TextChunk]) -> Result<Vec<ExtractionResult>>;

    /// Get extractor name for logging.
    fn name(&self) -> &str;

    /// Get the LLM model name.
    fn model_name(&self) -> &str;

    /// Get the LLM provider name.
    fn provider_name(&self) -> &str;
}
```

---

## Entity Types

EdgeQuake supports configurable entity types:

```rust
// Default entity types
vec![
    "PERSON",
    "ORGANIZATION",
    "LOCATION",
    "EVENT",
    "CONCEPT",
    "TECHNOLOGY",
    "PRODUCT",
]
```

### Domain-Specific Types

Customize for your domain:

```rust
// Biomedical domain
let extractor = SOTAExtractor::new(llm)
    .with_entity_types(vec![
        "PROTEIN".into(),
        "GENE".into(),
        "DISEASE".into(),
        "DRUG".into(),
        "ORGANISM".into(),
    ]);

// Legal domain
let extractor = SOTAExtractor::new(llm)
    .with_entity_types(vec![
        "PARTY".into(),
        "COURT".into(),
        "STATUTE".into(),
        "CASE".into(),
        "JURISDICTION".into(),
    ]);
```

---

## Entity Normalization

Entities are normalized for consistent graph structure:

```
┌─────────────────────────────────────────────────────────────────┐
│                 ENTITY NORMALIZATION                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Raw Text              Normalized Entity                        │
│  ────────              ─────────────────                        │
│  "Dr. Sarah Chen"   →  SARAH_CHEN                               │
│  "Sarah Chen, PhD"  →  SARAH_CHEN                               │
│  "Chen, Sarah"      →  SARAH_CHEN                               │
│                                                                 │
│  "MIT"              →  MIT                                      │
│  "M.I.T."           →  MIT                                      │
│  "Massachusetts     →  MIT                                      │
│   Institute of                                                  │
│   Technology"                                                   │
│                                                                 │
│  Normalization Rules (BR0008):                                  │
│  1. UPPERCASE all characters                                    │
│  2. Replace spaces with underscores                             │
│  3. Remove special characters                                   │
│  4. Merge common variants                                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Business Rule BR0008**: Entity names must be normalized to UPPERCASE_UNDERSCORE format.

---

## Relationship Extraction

Relationships connect entities in the knowledge graph:

```rust
pub struct ExtractedRelationship {
    /// Source entity name (normalized)
    pub source: String,

    /// Target entity name (normalized)
    pub target: String,

    /// Relationship type (e.g., "works_at", "developed")
    pub relation_type: String,

    /// Detailed description
    pub description: String,

    /// Weight/strength (0.0 to 1.0)
    pub weight: f32,

    /// Keywords for search (max 5 per BR0004)
    pub keywords: Vec<String>,

    /// Embedding for similarity search
    pub embedding: Option<Vec<f32>>,
}
```

### Relationship Business Rules

| Rule   | Description                                         |
| ------ | --------------------------------------------------- |
| BR0004 | Max 5 keywords per relationship                     |
| BR0006 | No self-referential relationships (source ≠ target) |

---

## Gleaning: Multi-Pass Extraction

Single-pass extraction often misses entities. Gleaning performs multiple extraction passes:

```
┌─────────────────────────────────────────────────────────────────┐
│                   GLEANING PROCESS                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Pass 1 (Base Extraction)                                       │
│  ─────────────────────────                                      │
│  Input: "Dr. Sarah Chen at MIT developed..."                    │
│                                                                 │
│  Found: SARAH_CHEN, MIT, NEURAL_NETWORK                         │
│                                                                 │
│                           │                                     │
│                           ▼                                     │
│                                                                 │
│  Pass 2 (Gleaning Iteration 1)                                  │
│  ─────────────────────────────                                  │
│  Prompt: "What entities did you miss? Already found:            │
│           SARAH_CHEN, MIT, NEURAL_NETWORK"                      │
│                                                                 │
│  Found: GRADIENT_DESCENT, LEARNING_RATE, OPTIMIZATION           │
│                                                                 │
│                           │                                     │
│                           ▼                                     │
│                                                                 │
│  Pass 3 (Gleaning Iteration 2) - Optional                       │
│  ─────────────────────────────                                  │
│  Found: AI_LAB, BACKPROPAGATION                                 │
│                                                                 │
│                           │                                     │
│                           ▼                                     │
│                                                                 │
│  Final Result: 8 entities merged (vs 3 without gleaning)        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Gleaning Effectiveness

| Iterations | Recall | Cost Multiplier   |
| ---------- | ------ | ----------------- |
| 0 (none)   | ~65%   | 1x                |
| 1          | ~80%   | 2x                |
| 2          | ~90%   | 3x                |
| 3+         | ~92%   | 4x+ (diminishing) |

**Recommendation**: Use 1-2 gleaning iterations for best cost/recall balance.

---

## Adaptive Token Management

The SOTA extractor adapts to chunk complexity:

```
┌─────────────────────────────────────────────────────────────────┐
│                 ADAPTIVE TOKEN MANAGEMENT                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Chunk Size              Base max_tokens         Strategy       │
│  ──────────              ─────────────           ────────       │
│  <25KB (~6K tokens)      4,096                   Small doc      │
│  25-75KB                 8,192                   Medium doc     │
│  75-125KB                12,288                  Large doc      │
│  >125KB                  16,384                  Very large     │
│                                                                 │
│  Retry Strategy (on truncation):                                │
│  ─────────────────────────────────                              │
│  Attempt 1: base_max_tokens (e.g., 8,192)                       │
│  Attempt 2: 2x tokens (16,384) + 100ms backoff                  │
│  Attempt 3: 4x tokens (32,768 max) + 200ms backoff              │
│                                                                 │
│  Truncation Detection:                                          │
│  • finish_reason="length" → Hit token limit                     │
│  • JSON parse errors ("EOF", "unclosed") → Response cut off     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## The Extraction Prompt

SOTA extractor uses a carefully designed prompt:

```rust
fn build_prompt(&self, text: &str) -> String {
    let entity_types_str = self.entity_types.join(", ");

    format!(r#"
-Goal-
Given a text document, identify all entities and their relationships.

-Entity Types-
{entity_types_str}

-Output Format-
Use tuple format for each entity:
("entity_name"|"entity_type"|"entity_description")

Use tuple format for each relationship:
("source_entity"|"target_entity"|"relationship_type"|"relationship_description")

-Text-
{text}
    "#)
}
```

### Why Tuples Over JSON?

| JSON                         | Tuples                      |
| ---------------------------- | --------------------------- |
| LLM can produce invalid JSON | Tuples are simpler to parse |
| Nested structure errors      | Flat structure              |
| Quote escaping issues        | Delimiter-based             |
| Higher token count           | More compact                |

---

## Extraction Result Structure

```rust
pub struct ExtractionResult {
    /// Extracted entities
    pub entities: Vec<ExtractedEntity>,

    /// Extracted relationships
    pub relationships: Vec<ExtractedRelationship>,

    /// Source chunk ID for lineage
    pub source_chunk_id: String,

    /// Processing metadata
    pub metadata: HashMap<String, Value>,

    /// Token usage for cost tracking
    pub input_tokens: usize,
    pub output_tokens: usize,

    /// Timing information
    pub extraction_time_ms: u64,
}
```

---

## Cost Analysis

Entity extraction is the primary LLM cost driver:

| Model          | Cost per 1K tokens             | Typical doc cost |
| -------------- | ------------------------------ | ---------------- |
| GPT-4o-mini    | $0.00015 input, $0.0006 output | $0.001           |
| GPT-4o         | $0.005 input, $0.015 output    | $0.02            |
| Ollama (local) | Free                           | Free             |

### Cost Optimization Strategies

1. **Use GPT-4o-mini** - 10x cheaper than GPT-4o
2. **Optimize chunk size** - 1200 tokens is the sweet spot
3. **Limit gleaning** - 1 iteration is usually sufficient
4. **Cache results** - Don't re-extract unchanged documents
5. **Use local models** - Ollama for development

---

## Error Handling

The extractor handles common failure modes:

```rust
// Chunk too large
if estimated_tokens > MAX_CHUNK_TOKENS {
    return Err(PipelineError::Validation(format!(
        "Chunk too large for LLM processing. \
         Suggestions: Use chunk_size={} for this document size",
        recommended_chunk_size
    )));
}

// LLM timeout
if is_timeout {
    // Provide actionable error with recommendations
    return Err(PipelineError::ExtractionError(format!(
        "LLM timeout after 120s. \
         Suggestions: 1) Reduce chunk_size 2) Use Ollama (300s timeout)"
    )));
}

// JSON parse error
if is_json_truncation {
    // Retry with higher max_tokens
    current_max_tokens = (current_max_tokens * 2).min(32768);
    continue;
}
```

---

## Best Practices

1. **Chunk Size**: Use 1200 tokens (default) for optimal extraction
2. **Entity Types**: Customize for your domain
3. **Gleaning**: Enable 1 iteration for important documents
4. **Model Selection**: GPT-4o-mini for cost, GPT-4o for quality
5. **Monitoring**: Track extraction time and token usage
6. **Caching**: Use extraction cache to avoid re-processing

---

## See Also

- [Chunking Strategies](./chunking-strategies.md) - Document chunking deep dive
- [Entity Deduplication](./entity-deduplication.md) - Merging duplicate entities
- [Graph Storage](./graph-storage.md) - Storing extracted entities
- [Document Ingestion Tutorial](../tutorials/document-ingestion.md) - End-to-end guide
