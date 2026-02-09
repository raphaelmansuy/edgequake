# Query Modes Deep-Dive

> **Understanding EdgeQuake's Multi-Strategy Retrieval System**

EdgeQuake provides 6 distinct query modes, each optimized for different types of questions. This guide explains when and why to use each mode, with practical examples and tuning recommendations.

---

## Table of Contents

- [Why Multiple Modes?](#why-multiple-modes)
- [Mode Overview](#mode-overview)
- [Mode Selection Flowchart](#mode-selection-flowchart)
- [Naive Mode](#naive-mode)
- [Local Mode](#local-mode)
- [Global Mode](#global-mode)
- [Hybrid Mode](#hybrid-mode)
- [Mix Mode](#mix-mode)
- [Bypass Mode](#bypass-mode)
- [Performance Comparison](#performance-comparison)
- [Configuration & Tuning](#configuration--tuning)
- [API Usage Examples](#api-usage-examples)

---

## Why Multiple Modes?

Different questions require fundamentally different retrieval strategies. Consider these queries about a document about climate science:

| Question                                                     | Optimal Strategy                                     |
| ------------------------------------------------------------ | ---------------------------------------------------- |
| "What is the greenhouse effect?"                             | **Vector search** - Find semantically similar chunks |
| "How does Sarah Chen's work relate to atmospheric modeling?" | **Graph traversal** - Follow entity relationships    |
| "What are the main themes in this document?"                 | **Community detection** - Analyze topic clusters     |
| "Explain Sarah Chen's contributions to climate research"     | **Both** - Entity + broader context                  |

A single retrieval strategy cannot optimally serve all these query types. EdgeQuake's multi-mode system allows you to match the strategy to your question.

### The Information Retrieval Triangle

```
                    ┌─────────────────┐
                    │   PRECISION     │
                    │                 │
                    │  (Specific,     │
                    │   Accurate)     │
                    └────────┬────────┘
                             │
              Naive ─────────┼─────────
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        │       Hybrid       │                    │
        │                    │                    │
   ┌────┴────┐          ┌────┴────┐          ┌────┴────┐
   │  SPEED  │          │         │          │ COVERAGE│
   │         │──────────│  Mix    │──────────│         │
   │ (Fast,  │  Local   │         │  Global  │ (Broad, │
   │  Cheap) │          │         │          │Complete)│
   └─────────┘          └─────────┘          └─────────┘
```

No mode is universally "best" - each makes different trade-offs.

---

## Mode Overview

| Mode       | Vector Search | Graph Traversal | Best For                        |
| ---------- | :-----------: | :-------------: | ------------------------------- |
| **Naive**  |      ✅       |       ❌        | Factual queries, keyword lookup |
| **Local**  |      ✅       |       ✅        | Entity-specific questions       |
| **Global** |      ❌       |       ✅        | Theme/topic analysis            |
| **Hybrid** |      ✅       |       ✅        | Complex, multi-faceted queries  |
| **Mix**    |      ✅       |       ✅        | Custom weighted retrieval       |
| **Bypass** |      ❌       |       ❌        | Direct LLM, testing             |

### Quick Selection Guide

```
┌─────────────────────────────────────────────────────────────────┐
│                    QUERY MODE QUICK GUIDE                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  "What is X?"                    → Naive   (fast, direct)       │
│  "How does A relate to B?"       → Local   (entity graph)       │
│  "What are the main themes?"     → Global  (topic clusters)     │
│  "Tell me about X and its impact"→ Hybrid  (comprehensive)      │
│  "I need custom weights"         → Mix     (tunable)            │
│  "Skip RAG, just ask LLM"        → Bypass  (testing)            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Mode Selection Flowchart

Use this decision tree to select the optimal mode:

```
                        ┌─────────────────────────┐
                        │   Is RAG needed at all? │
                        └───────────┬─────────────┘
                                    │
                    ┌───────────────┴───────────────┐
                    │                               │
                   YES                              NO
                    │                               │
                    ▼                               ▼
        ┌───────────────────────┐          ┌───────────────┐
        │ Does query mention    │          │    BYPASS     │
        │ specific entities?    │          │  (no RAG)     │
        └───────────┬───────────┘          └───────────────┘
                    │
        ┌───────────┴───────────┐
        │                       │
       YES                      NO
        │                       │
        ▼                       ▼
┌───────────────────────┐  ┌───────────────────────┐
│ Also asking about     │  │ Asking about themes   │
│ broader context?      │  │ or overarching topics?│
└───────────┬───────────┘  └───────────┬───────────┘
            │                          │
    ┌───────┴───────┐          ┌───────┴───────┐
    │               │          │               │
   YES              NO        YES              NO
    │               │          │               │
    ▼               ▼          ▼               ▼
┌───────┐     ┌───────┐   ┌───────┐      ┌───────┐
│HYBRID │     │ LOCAL │   │GLOBAL │      │ NAIVE │
│       │     │       │   │       │      │       │
└───────┘     └───────┘   └───────┘      └───────┘
```

---

## Naive Mode

> **FEAT0101**: Vector similarity search only

Naive mode performs pure vector similarity search on document chunks, without graph traversal. It's the fastest mode and works well for simple factual queries.

### How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│                      NAIVE MODE FLOW                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Query: "What is machine learning?"                             │
│         │                                                       │
│         ▼                                                       │
│  ┌─────────────────┐                                            │
│  │ Embed Query     │  → [0.23, -0.45, 0.87, ...]                │
│  └────────┬────────┘                                            │
│           │                                                     │
│           ▼                                                     │
│  ┌─────────────────────────────────────────┐                    │
│  │  Vector Database (pgvector)              │                   │
│  │  ┌────────┐ ┌────────┐ ┌────────┐       │                    │
│  │  │chunk_1 │ │chunk_2 │ │chunk_3 │ ...   │                    │
│  │  │sim:0.92│ │sim:0.85│ │sim:0.78│       │                    │
│  │  └────────┘ └────────┘ └────────┘       │                    │
│  └─────────────────────────────────────────┘                    │
│           │                                                     │
│           ▼                                                     │
│  ┌─────────────────┐                                            │
│  │ Top-K Chunks    │  → ["ML is a subset of AI...",             │
│  │ (scored)        │      "Training neural networks..."]        │
│  └────────┬────────┘                                            │
│           │                                                     │
│           ▼                                                     │
│  ┌─────────────────┐                                            │
│  │ LLM Generation  │  → "Machine learning is..."                │
│  └─────────────────┘                                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### When to Use

✅ **Good for:**

- Simple factual questions ("What is X?")
- Keyword-based lookup
- Fast response requirements
- When graph data is sparse

❌ **Avoid when:**

- Asking about relationships
- Need comprehensive coverage
- Entities are important

### Example

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What is the greenhouse effect?",
    "mode": "naive"
  }'
```

### Performance

| Metric         | Typical Value |
| -------------- | ------------- |
| Latency        | 100-300ms     |
| Context tokens | 500-2000      |
| LLM calls      | 1             |

---

## Local Mode

> **FEAT0102**: Entity-centric graph traversal

Local mode combines vector search with graph traversal from identified entities. It excels at questions about specific entities and their relationships.

### How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│                      LOCAL MODE FLOW                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Query: "How does Sarah Chen work with the IPCC?"               │
│         │                                                       │
│         ▼                                                       │
│  ┌─────────────────┐   ┌─────────────────┐                      │
│  │ Embed Query     │   │ Extract Entities│                      │
│  └────────┬────────┘   └────────┬────────┘                      │
│           │                     │                               │
│           ▼                     ▼                               │
│  ┌─────────────────┐   ┌─────────────────────┐                  │
│  │  Vector Search  │   │  Entity Lookup      │                  │
│  │  (chunks)       │   │  SARAH_CHEN, IPCC   │                  │
│  └────────┬────────┘   └────────┬────────────┘                  │
│           │                     │                               │
│           │                     ▼                               │
│           │            ┌─────────────────────────┐              │
│           │            │  Graph Traversal        │              │
│           │            │                         │              │
│           │            │  SARAH_CHEN ──WORKS_WITH──▶ IPCC       │
│           │            │       │                    │           │
│           │            │       └──AUTHORED──▶ PAPER_1           │
│           │            │                         │              │
│           │            └─────────────────────────┘              │
│           │                     │                               │
│           └──────────┬──────────┘                               │
│                      ▼                                          │
│             ┌─────────────────┐                                 │
│             │ Merge Context   │                                 │
│             │ (chunks +       │                                 │
│             │  entities +     │                                 │
│             │  relationships) │                                 │
│             └────────┬────────┘                                 │
│                      ▼                                          │
│             ┌─────────────────┐                                 │
│             │ LLM Generation  │                                 │
│             └─────────────────┘                                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### When to Use

✅ **Good for:**

- Questions about specific people, places, organizations
- Relationship queries ("How does X relate to Y?")
- When entity context enriches the answer
- Named entity questions

❌ **Avoid when:**

- Entities not well-extracted
- Asking about abstract concepts
- Need speed over comprehensiveness

### Example

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What is Sarah Chen'\''s research focus?",
    "mode": "local"
  }'
```

### Performance

| Metric         | Typical Value |
| -------------- | ------------- |
| Latency        | 200-500ms     |
| Context tokens | 1000-3000     |
| Graph queries  | 3-10          |

---

## Global Mode

> **FEAT0103**: Community-based summarization

Global mode focuses on high-level topic clusters identified during indexing. It's ideal for theme analysis and summary questions.

### How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│                      GLOBAL MODE FLOW                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Query: "What are the main themes in this document?"            │
│         │                                                       │
│         ▼                                                       │
│  ┌─────────────────────────────────────────────────┐            │
│  │  Community Detection (pre-computed during index)│            │
│  │                                                 │            │
│  │  ┌─────────────────┐  ┌─────────────────┐       │            │
│  │  │   Community 1   │  │   Community 2   │       │            │
│  │  │   "Climate"     │  │   "Technology"  │       │            │
│  │  │                 │  │                 │       │            │
│  │  │  • IPCC         │  │  • MACHINE_     │       │            │
│  │  │  • SARAH_CHEN   │  │    LEARNING     │       │            │
│  │  │  • CO2_LEVELS   │  │  • NEURAL_NET   │       │            │
│  │  │  • WARMING      │  │  • PREDICTION   │       │            │
│  │  └─────────────────┘  └─────────────────┘       │            │
│  │           │                    │                │            │
│  │           ▼                    ▼                │            │
│  │  ┌─────────────────────────────────────┐        │            │
│  │  │        Community Summaries          │        │            │
│  │  │  "Climate: Research focuses on..."  │        │            │
│  │  │  "Technology: ML applications..."   │        │            │
│  │  └─────────────────────────────────────┘        │            │
│  └─────────────────────────────────────────────────┘            │
│                      │                                          │
│                      ▼                                          │
│             ┌─────────────────┐                                 │
│             │ LLM Generation  │                                 │
│             │ (theme synthesis)│                                │
│             └─────────────────┘                                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### When to Use

✅ **Good for:**

- "What are the main themes/topics?"
- Summary questions
- Overview requests
- When breadth matters more than depth

❌ **Avoid when:**

- Asking about specific entities
- Need precise factual answers
- Speed is critical

### Example

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What topics does this document cover?",
    "mode": "global"
  }'
```

### Performance

| Metric         | Typical Value |
| -------------- | ------------- |
| Latency        | 300-800ms     |
| Context tokens | 2000-4000     |
| Communities    | 5-20          |

---

## Hybrid Mode

> **FEAT0104**: Combines Local and Global (Default)

Hybrid mode uses both vector search and full graph traversal, combining the precision of Local with the coverage of Global. It's the default mode because it handles the widest variety of queries.

### How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│                      HYBRID MODE FLOW                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Query: "Explain Sarah Chen's impact on climate modeling"       │
│         │                                                       │
│         ├─────────────────────────────────────┐                 │
│         │                                     │                 │
│         ▼                                     ▼                 │
│  ┌─────────────────┐                 ┌─────────────────┐        │
│  │  LOCAL PATH     │                 │  GLOBAL PATH    │        │
│  │                 │                 │                 │        │
│  │  • Vector search│                 │  • Community    │        │
│  │  • Entity lookup│                 │    summaries    │        │
│  │  • Neighborhood │                 │  • Topic context│        │
│  │    traversal    │                 │                 │        │
│  └────────┬────────┘                 └────────┬────────┘        │
│           │                                   │                 │
│           │  ┌───────────────────────────┐    │                 │
│           └─▶│    CONTEXT FUSION         │◀───┘                 │
│              │                           │                      │
│              │  1. Deduplicate entities  │                      │
│              │  2. Merge relationships   │                      │
│              │  3. Combine chunks        │                      │
│              │  4. Apply token budget    │                      │
│              └─────────────┬─────────────┘                      │
│                            │                                    │
│                            ▼                                    │
│              ┌─────────────────────────┐                        │
│              │    LLM Generation       │                        │
│              │   (comprehensive answer)│                        │
│              └─────────────────────────┘                        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### When to Use

✅ **Good for:**

- Complex, multi-faceted questions
- When you're unsure which mode to use
- Production default
- Comprehensive answers needed

❌ **Avoid when:**

- Speed is critical
- Token budget is tight
- Simple factual queries

### Example

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "Explain the relationship between ML and climate research",
    "mode": "hybrid"
  }'
```

### Performance

| Metric         | Typical Value |
| -------------- | ------------- |
| Latency        | 400-1000ms    |
| Context tokens | 3000-4000     |
| LLM calls      | 1             |

---

## Mix Mode

> **FEAT0105**: Weighted combination with tunable parameters

Mix mode allows explicit weighting between vector and graph retrieval. Use it when you need fine-grained control over the retrieval strategy.

### Configuration

```json
{
  "query": "Your question here",
  "mode": "mix",
  "params": {
    "vector_weight": 0.7,
    "graph_weight": 0.3
  }
}
```

### When to Use

✅ **Good for:**

- A/B testing retrieval strategies
- Domain-specific tuning
- When default weights don't work well
- Research and experimentation

---

## Bypass Mode

> **FEAT0106**: Direct LLM, no retrieval

Bypass mode skips RAG entirely and sends the query directly to the LLM. Useful for testing or when external knowledge isn't needed.

### Example

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What is 2 + 2?",
    "mode": "bypass"
  }'
```

---

## Performance Comparison

| Mode       | Latency               | Accuracy        | Context | Cost        |
| ---------- | --------------------- | --------------- | ------- | ----------- |
| **Naive**  | ⚡ Fast (100-300ms)   | ⭐⭐⭐ Good     | Small   | 💵 Low      |
| **Local**  | 🚀 Medium (200-500ms) | ⭐⭐⭐⭐ High   | Medium  | 💵💵 Medium |
| **Global** | 🐢 Slow (300-800ms)   | ⭐⭐⭐⭐ High   | Large   | 💵💵 Medium |
| **Hybrid** | 🐢 Slow (400-1000ms)  | ⭐⭐⭐⭐⭐ Best | Large   | 💵💵💵 High |
| **Mix**    | Variable              | Tunable         | Tunable | Variable    |
| **Bypass** | ⚡ Fastest            | ⭐ LLM only     | None    | 💵 Low      |

### Resource Usage by Mode

```
┌─────────────────────────────────────────────────────────────────┐
│                    RESOURCE USAGE BY MODE                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Naive   ████░░░░░░░░░░░░░░░░  (Vector only)                    │
│                                                                 │
│  Local   ████████░░░░░░░░░░░░  (Vector + Graph node)            │
│                                                                 │
│  Global  ██████████░░░░░░░░░░  (Graph communities)              │
│                                                                 │
│  Hybrid  ████████████████░░░░  (All sources)                    │
│                                                                 │
│  Mix     ████████████░░░░░░░░  (Weighted blend)                 │
│                                                                 │
│          ─────────────────────────────────────────►             │
│          Low                                    High            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Configuration & Tuning

### Default Configuration

```rust
QueryEngineConfig {
    default_mode: QueryMode::Hybrid,
    max_chunks: 10,
    max_entities: 20,
    max_context_tokens: 4000,
    graph_depth: 2,
    min_score: 0.1,
    include_sources: true,
}
```

### Tuning Parameters

| Parameter            | Default | Effect                                  |
| -------------------- | ------- | --------------------------------------- |
| `max_chunks`         | 10      | More chunks = more context, higher cost |
| `max_entities`       | 20      | More entities = richer graph context    |
| `max_context_tokens` | 4000    | Token budget for LLM context            |
| `graph_depth`        | 2       | How many hops in graph traversal        |
| `min_score`          | 0.1     | Similarity threshold for inclusion      |

### Mode-Specific Tuning

**For Naive mode:**

- Increase `max_chunks` for better coverage
- Lower `min_score` for more permissive matching

**For Local mode:**

- Increase `graph_depth` for deeper relationships
- Balance `max_entities` vs `max_chunks`

**For Global mode:**

- Ensure communities are well-formed
- Consider community detection parameters

**For Hybrid mode:**

- Use `max_context_tokens` to balance cost
- Enable reranking for better precision

---

## API Usage Examples

### Basic Query with Mode

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -H "X-Workspace-ID: your-workspace" \
  -d '{
    "query": "What is the main finding?",
    "mode": "naive"
  }'
```

### Query with Reranking

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "Explain the climate research methodology",
    "mode": "hybrid",
    "enable_rerank": true,
    "rerank_top_k": 5
  }'
```

### Context-Only Mode (Debug)

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "Your question",
    "mode": "local",
    "context_only": true
  }'
```

This returns only the retrieved context without LLM generation, useful for debugging retrieval quality.

### Prompt-Only Mode (Debug)

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "Your question",
    "mode": "hybrid",
    "prompt_only": true
  }'
```

Returns the formatted prompt that would be sent to the LLM.

---

## See Also

- [LightRAG Algorithm](lightrag-algorithm.md) - The algorithm powering EdgeQuake
- [Entity Extraction](entity-extraction.md) - How entities are identified
- [REST API Reference](../api-reference/rest-api.md) - Full API documentation
- [Architecture Overview](../architecture/overview.md) - System design
