# OODA Iteration 03 - Observe

**Date**: 2025-01-XX
**Focus**: LightRAG Algorithm Deep-Dive Documentation

## 📊 Observations from Codebase

### 1. Entity Extraction System (`edgequake-pipeline/src/prompts/`)

| File                   | Lines | Purpose                                          |
| ---------------------- | ----- | ------------------------------------------------ |
| `entity_extraction.rs` | 270   | SOTA prompts with tuple-delimited format         |
| `normalizer.rs`        | 180   | Entity name normalization (UPPERCASE_UNDERSCORE) |
| `parser.rs`            | 658   | Hybrid tuple/JSON parsing with fallback          |
| `summarization.rs`     | 218   | MapReduce summarization prompts                  |

### 2. LightRAG Paper Key Concepts (arxiv:2410.05779)

From the paper research:

1. **Dual-Level Retrieval Paradigm**
   - Low-level: Precise entity and relationship retrieval
   - High-level: Broader topics and themes

2. **Graph-Enhanced Text Indexing**
   - Entity/Relationship extraction via LLM
   - Profiling for Key-Value pair generation
   - Deduplication for graph optimization

3. **Incremental Knowledge Base Updates**
   - No full reprocessing required
   - Seamless integration of new documents

4. **Performance Results**
   - 60-85% win rate vs NaiveRAG across all metrics
   - 50-55% win rate vs GraphRAG
   - 610,000+ tokens savings in retrieval vs GraphRAG

### 3. EdgeQuake Implementation Differences

| LightRAG Paper        | EdgeQuake Implementation                       |
| --------------------- | ---------------------------------------------- |
| Tuple format only     | Hybrid parser (tuple + JSON fallback)          |
| Python implementation | Rust with async Tokio                          |
| Single LLM provider   | Multiple providers (OpenAI, Ollama, Mock)      |
| Basic error handling  | Adaptive retry with progressive token increase |
| Fixed chunking        | Adaptive chunk sizes (600-1200 tokens)         |

### 4. Query Modes Discovered (6 modes vs LightRAG's 3)

```
EdgeQuake Query Modes:
├── Naive  (FEAT0101): Vector similarity only
├── Local  (FEAT0102): Entity-centric graph
├── Global (FEAT0103): Community summaries
├── Hybrid (FEAT0104): Local + Global combined ← DEFAULT
├── Mix    (FEAT0105): Weighted naive + graph
└── Bypass (FEAT0106): Direct LLM (no RAG)
```

### 5. Gleaning Implementation

```rust
// GleaningConfig defaults
pub struct GleaningConfig {
    pub max_gleaning: usize,     // Default: 1 (LightRAG recommendation)
    pub always_glean: bool,      // Default: false
}
```

Research finding: 1-2 gleaning iterations improve recall by 15-25%

## 🔍 Key Code Patterns

### Tuple Parsing vs JSON

```
Tuple Format (Production - More Robust):
entity<|#|>Sarah Chen<|#|>PERSON<|#|>Lead researcher at Quantum Lab
relation<|#|>Sarah Chen<|#|>Quantum Lab<|#|>employment<|#|>Works as researcher

JSON Format (Development - More Readable):
{
  "entities": [...],
  "relationships": [...]
}
```

WHY tuple format:

1. Partial output recovery (streaming-friendly)
2. No escaping issues
3. Line-by-line processing
4. LightRAG battle-tested

### Entity Normalization Rules

```rust
normalize_entity_name("John Doe") → "JOHN_DOE"
normalize_entity_name("the company") → "COMPANY"
normalize_entity_name("John's") → "JOHN"
```

WHY normalization matters:

- Prevents graph fragmentation
- Enables proper node merging
- Improves query accuracy

## 📈 Metrics from Paper

| Dataset     | Tokens | Documents | LightRAG Win Rate vs NaiveRAG |
| ----------- | ------ | --------- | ----------------------------- |
| Agriculture | 2M     | 12        | 67.6%                         |
| CS          | 2.3M   | 10        | 61.2%                         |
| Legal       | 5M     | 94        | 84.8%                         |
| Mix         | 619K   | 61        | 60.0%                         |

## 🎯 Documentation Gaps Identified

1. **Missing**: LightRAG algorithm explanation with First Principles
2. **Missing**: Comparison with GraphRAG architecture
3. **Missing**: Detailed gleaning strategy explanation
4. **Missing**: Query mode selection guide with examples

## 📁 Files Read This Iteration

1. `edgequake-pipeline/src/prompts/entity_extraction.rs` (270 lines)
2. `edgequake-pipeline/src/prompts/normalizer.rs` (180 lines)
3. `edgequake-pipeline/src/prompts/parser.rs` (658 lines)
4. `edgequake-pipeline/src/prompts/summarization.rs` (218 lines)
5. `edgequake-query/src/modes.rs` (180 lines)
6. `edgequake-pipeline/src/extractor.rs` (sections on SOTA, Gleaning)
7. LightRAG paper (arxiv:2410.05779v3) - Full HTML version
