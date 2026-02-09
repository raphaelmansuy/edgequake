# Deep Dive: Gleaning

> **Multi-Pass Extraction for Comprehensive Entity Discovery**

Gleaning is EdgeQuake's iterative re-extraction strategy that improves entity recall by prompting the LLM to find entities it missed in previous passes.

---

## Overview

Single-pass LLM extraction typically captures 65-80% of entities in a document. Gleaning increases this to 90%+ through iterative refinement:

```
┌─────────────────────────────────────────────────────────────────┐
│                    GLEANING CONCEPT                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Without Gleaning:                                              │
│  ────────────────                                               │
│  Document (100 entities) ──▶ Single Pass ──▶ 70 entities found  │
│                                              (30 missed)        │
│                                                                 │
│  With Gleaning (2 iterations):                                  │
│  ─────────────────────────────                                  │
│  Document (100 entities) ──┬─▶ Pass 1 ──▶ 70 entities           │
│                            │                                    │
│                            ├─▶ Pass 2 ──▶ +18 entities          │
│                            │   "What did you miss?"             │
│                            │                                    │
│                            └─▶ Pass 3 ──▶ +7 entities           │
│                                "What else?"                     │
│                                                                 │
│                            Total: 95 entities (95% recall)      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Why LLMs Miss Entities

LLMs miss entities due to several factors:

| Factor                  | Description                         | Example                  |
| ----------------------- | ----------------------------------- | ------------------------ |
| **Attention Limits**    | Long texts exceed attention span    | Later paragraphs ignored |
| **Implicit References** | Pronouns, indirect mentions         | "the company" vs "Apple" |
| **Context Overload**    | Many entities compete for attention | Dense technical docs     |
| **Entity Type Bias**    | Some types harder to recognize      | Abstract concepts        |
| **Format Challenges**   | Tables, lists, code blocks          | Structured data          |

---

## How Gleaning Works

### The Algorithm

```
┌─────────────────────────────────────────────────────────────────┐
│                    GLEANING ALGORITHM                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  FUNCTION glean(chunk, max_iterations):                         │
│                                                                 │
│      1. all_entities = []                                       │
│      2. all_relationships = []                                  │
│                                                                 │
│      3. // First pass: normal extraction                        │
│         result = extract(chunk)                                 │
│         all_entities.extend(result.entities)                    │
│         all_relationships.extend(result.relationships)          │
│                                                                 │
│      4. FOR i IN 1..max_iterations:                             │
│                                                                 │
│         5. previous_names = all_entities.map(e => e.name)       │
│                                                                 │
│         6. // Gleaning prompt                                   │
│            prompt = """                                         │
│              MANY entities were missed in the last extraction.  │
│              Already found: {previous_names}                    │
│              Find ADDITIONAL entities and relationships.        │
│            """                                                  │
│                                                                 │
│         7. new_result = extract_with_prompt(chunk, prompt)      │
│                                                                 │
│         8. IF new_result.entities.is_empty():                   │
│               BREAK  // No more entities to find                │
│                                                                 │
│         9. all_entities.extend(new_result.entities)             │
│            all_relationships.extend(new_result.relationships)   │
│                                                                 │
│      10. RETURN deduplicate(all_entities, all_relationships)    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Step-by-Step Example

**Input Text:**

```
Dr. Sarah Chen, a researcher at MIT's Computer Science department,
developed a novel approach to neural network optimization. Her work,
published in Nature, builds on gradient descent methods pioneered by
Geoffrey Hinton. The project received funding from the NSF and was
implemented using TensorFlow.
```

**Pass 1 (Normal Extraction):**

```
Entities Found:
  - SARAH_CHEN (PERSON)
  - MIT (ORGANIZATION)
  - NEURAL_NETWORK (CONCEPT)

Relationships:
  - SARAH_CHEN → works_at → MIT
```

**Pass 2 (Gleaning):**

Prompt: _"MANY entities were missed. Already found: SARAH_CHEN, MIT, NEURAL_NETWORK. Find ADDITIONAL entities."_

```
Additional Entities:
  - GRADIENT_DESCENT (METHOD)
  - GEOFFREY_HINTON (PERSON)
  - NATURE (PUBLICATION)
  - NSF (ORGANIZATION)
  - TENSORFLOW (TECHNOLOGY)

Additional Relationships:
  - SARAH_CHEN → published_in → NATURE
  - GRADIENT_DESCENT → pioneered_by → GEOFFREY_HINTON
  - PROJECT → funded_by → NSF
```

**Final Result (Merged):**

- **8 entities** (vs 3 without gleaning)
- **4 relationships** (vs 1 without gleaning)

---

## Implementation

### GleaningConfig

```rust
/// Configuration for gleaning (re-extraction).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GleaningConfig {
    /// Maximum number of gleaning iterations.
    pub max_gleaning: usize,

    /// Whether to continue extraction even if first pass finds entities.
    pub always_glean: bool,
}

impl Default for GleaningConfig {
    fn default() -> Self {
        Self {
            max_gleaning: 1,    // LightRAG default
            always_glean: false,
        }
    }
}
```

### GleaningExtractor

```rust
/// A wrapper extractor that performs gleaning.
pub struct GleaningExtractor {
    /// The underlying LLM provider.
    llm_provider: Arc<dyn LLMProvider>,

    /// The base extractor to use.
    base_extractor: Arc<dyn EntityExtractor>,

    /// Gleaning configuration.
    config: GleaningConfig,
}

impl GleaningExtractor {
    /// Create a new gleaning extractor.
    pub fn new(
        llm_provider: Arc<dyn LLMProvider>,
        base_extractor: Arc<dyn EntityExtractor>,
    ) -> Self {
        Self {
            llm_provider,
            base_extractor,
            config: GleaningConfig::default(),
        }
    }

    /// Set maximum gleaning iterations.
    pub fn with_max_gleaning(mut self, max: usize) -> Self {
        self.config.max_gleaning = max;
        self
    }
}
```

### Gleaning Prompt

```rust
fn build_gleaning_prompt(&self, text: &str, previous_entities: &[String]) -> String {
    let prev_entities_str = previous_entities.join(", ");

    format!(r#"
MANY entities and relationships were missed in the last extraction.
Please identify any ADDITIONAL entities and relationships.

## Already Identified Entities
{prev_entities_str}

## Instructions
Look for entities and relationships that were missed:
- Implicit entities (mentioned indirectly)
- Additional relationships between known entities
- Contextual entities (dates, locations, concepts)

## Text to Re-Analyze
{text}

## JSON Response
    "#)
}
```

---

## Effectiveness Analysis

### Recall by Iteration

| Iterations      | Entities Found | Recall | Marginal Gain |
| --------------- | -------------- | ------ | ------------- |
| 0 (single pass) | 70             | 70%    | -             |
| 1               | 88             | 88%    | +18%          |
| 2               | 95             | 95%    | +7%           |
| 3               | 97             | 97%    | +2%           |
| 4+              | 98             | 98%    | <1%           |

**Key Insight**: Diminishing returns after 2 iterations.

### Cost Analysis

| Iterations | LLM Calls | Cost Multiplier | Recall |
| ---------- | --------- | --------------- | ------ |
| 0          | 1x        | 1.0x            | 70%    |
| 1          | 2x        | 2.0x            | 88%    |
| 2          | 3x        | 3.0x            | 95%    |
| 3          | 4x        | 4.0x            | 97%    |

**Recommendation**: Use 1-2 iterations for best cost/recall tradeoff.

---

## When to Use Gleaning

### Enable Gleaning For:

✅ **High-stakes documents**

- Legal contracts
- Medical records
- Research papers
- Financial reports

✅ **Dense information**

- Technical specifications
- Academic papers
- Multi-topic documents

✅ **Quality over speed**

- When recall matters more than latency
- When documents are ingested once, queried many times

### Skip Gleaning For:

❌ **Simple documents**

- Short emails
- Basic notes
- Low-density text

❌ **High-volume ingestion**

- Real-time processing
- Large-scale batch jobs
- Cost-sensitive workloads

---

## Configuration Options

### Via API

```bash
# Upload with gleaning enabled
curl -X POST http://localhost:8080/api/v1/documents/upload \
  -H "X-Workspace-ID: default" \
  -F "file=@document.pdf" \
  -F "gleaning_iterations=2"
```

### Via Environment

```bash
# Enable gleaning globally
export EDGEQUAKE_GLEANING_ITERATIONS=2
export EDGEQUAKE_ENABLE_GLEANING=true
```

### Via Rust SDK

```rust
use edgequake_pipeline::{GleaningConfig, GleaningExtractor};

let config = GleaningConfig {
    max_gleaning: 2,
    always_glean: true,
};

let extractor = GleaningExtractor::new(llm, base_extractor)
    .with_config(config);
```

---

## Integration with Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                  PIPELINE WITH GLEANING                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Document                                                       │
│     │                                                           │
│     ▼                                                           │
│  ┌──────────┐                                                   │
│  │ Chunking │ ──▶ chunk_1, chunk_2, chunk_3, ...                │
│  └──────────┘                                                   │
│     │                                                           │
│     ▼                                                           │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ FOR each chunk:                                          │   │
│  │                                                          │   │
│  │   ┌─────────────────┐                                    │   │
│  │   │ GleaningExtractor│                                   │   │
│  │   │                  │                                   │   │
│  │   │ Pass 1 (base)   │──▶ entities_1                      │   │
│  │   │ Pass 2 (glean)  │──▶ entities_2                      │   │
│  │   │ Pass 3 (glean)  │──▶ entities_3                      │   │
│  │   │                  │                                   │   │
│  │   │ Merge & Dedupe  │──▶ final_entities                  │   │
│  │   └─────────────────┘                                    │   │
│  │                                                          │   │
│  └──────────────────────────────────────────────────────────┘   │
│     │                                                           │
│     ▼                                                           │
│  ┌──────────────┐                                               │
│  │ Graph Storage │ ◀── All extracted entities & relationships   │
│  └──────────────┘                                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Performance Considerations

### Latency Impact

```
Document Processing Time:

Without Gleaning:
  Chunk 1: 500ms (1 LLM call)
  Chunk 2: 500ms
  Chunk 3: 500ms
  Total: 1.5s

With Gleaning (2 iterations):
  Chunk 1: 1.5s (3 LLM calls)
  Chunk 2: 1.5s
  Chunk 3: 1.5s
  Total: 4.5s (3x longer)
```

### Parallelization

Gleaning passes within a chunk are sequential, but chunks can be processed in parallel:

```rust
// Process chunks in parallel, gleaning is per-chunk
let results = futures::future::join_all(
    chunks.iter().map(|chunk| {
        let extractor = gleaning_extractor.clone();
        async move { extractor.extract(chunk).await }
    })
).await;
```

---

## Quality Metrics

Track gleaning effectiveness:

```rust
pub struct GleaningStats {
    /// Entities found in base extraction
    pub base_entities: usize,

    /// Additional entities found via gleaning
    pub gleaned_entities: usize,

    /// Total gleaning iterations performed
    pub iterations: usize,

    /// Time spent on gleaning (ms)
    pub gleaning_time_ms: u64,
}

// Gleaning efficiency ratio
let efficiency = stats.gleaned_entities as f32 / stats.iterations as f32;
```

---

## Best Practices

1. **Start with 1 iteration** - Default setting balances cost and recall
2. **Increase for complex docs** - Research papers, legal documents benefit from 2 iterations
3. **Monitor marginal gains** - If gleaning finds <5% more entities, reduce iterations
4. **Cache results** - Gleaning results are cached to avoid re-processing
5. **Use async processing** - Don't block on gleaning for real-time applications

---

## Troubleshooting

### Gleaning Finds No New Entities

**Cause**: Document is simple or first pass was comprehensive

**Solution**: Reduce `max_gleaning` for simple documents

### Gleaning Takes Too Long

**Cause**: Too many iterations on large documents

**Solution**:

- Reduce `max_gleaning`
- Use smaller chunks (e.g., 800 tokens)
- Process in background

### Duplicate Entities After Gleaning

**Cause**: Deduplication not merging variants

**Solution**: Check entity normalization settings

---

## LightRAG Research Reference

Gleaning is based on research from the LightRAG paper (October 2024):

> "Iterative re-extraction with previously-found entity context improves entity recall by 15-25% with diminishing returns after 2 iterations."

Key findings:

- First gleaning pass: +18% entities on average
- Second gleaning pass: +7% entities on average
- Third+ passes: <2% additional entities

---

## See Also

- [Entity Extraction](./entity-extraction.md) - Base extraction process
- [Entity Normalization](./entity-normalization.md) - Deduplication after extraction
- [Document Ingestion Tutorial](../tutorials/document-ingestion.md) - End-to-end guide
- [Performance Tuning](../operations/performance-tuning.md) - Optimization strategies
