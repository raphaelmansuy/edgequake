# LightRAG Algorithm Deep-Dive

> **Understanding Graph-Augmented Retrieval: From First Principles to Implementation**

EdgeQuake implements a Rust-based version of the LightRAG algorithm, enhanced with
adaptive error recovery, multi-provider support, and extended query modes. This
deep-dive explains the algorithm from first principles, walking through each
component with diagrams and code references.

---

## Table of Contents

1. [Why Graph-RAG? First Principles](#why-graph-rag-first-principles)
2. [The LightRAG Innovation](#the-lightrag-innovation)
3. [Algorithm Walkthrough](#algorithm-walkthrough)
4. [Entity Extraction in Detail](#entity-extraction-in-detail)
5. [Dual-Level Retrieval](#dual-level-retrieval)
6. [Query Modes Explained](#query-modes-explained)
7. [Gleaning: Multi-Pass Extraction](#gleaning-multi-pass-extraction)
8. [EdgeQuake Innovations](#edgequake-innovations)
9. [Comparisons](#comparisons)
10. [References](#references)

---

## Why Graph-RAG? First Principles

### The Problem with Traditional RAG

Traditional Retrieval-Augmented Generation (RAG) systems use a simple approach:

```
┌─────────────────────────────────────────────────────────────────┐
│                    TRADITIONAL RAG (Naive)                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Documents ──> Chunks ──> Embeddings ──> Vector DB              │
│                                                                 │
│  Query ──> Embedding ──> Top-K Similar Chunks ──> LLM Answer    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

This works well for simple factual questions, but fails for complex queries:

**Example Query**: _"How did Sarah Chen's research on neural networks influence
the work of her colleagues at Quantum Dynamics Lab?"_

Traditional RAG might return:

1. A chunk mentioning "Sarah Chen"
2. A chunk about "neural networks"
3. A chunk mentioning "Quantum Dynamics Lab"

But these chunks are **disconnected**. The system cannot:

- Understand that Sarah Chen **works at** Quantum Dynamics Lab
- Connect her research **to** colleagues' work
- Follow the **influence chain** across documents

### Why Graphs Solve This

Graphs are fundamentally about **relationships**:

```
┌─────────────────────────────────────────────────────────────────┐
│                    KNOWLEDGE GRAPH STRUCTURE                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│           ┌─────────────┐                                       │
│           │ SARAH_CHEN  │                                       │
│           │   (PERSON)  │                                       │
│           └──────┬──────┘                                       │
│                  │                                              │
│      ┌───────────┼───────────┐                                  │
│      │           │           │                                  │
│      v           v           v                                  │
│  ┌───────┐  ┌─────────┐  ┌──────────────────┐                   │
│  │WORKS_AT  │RESEARCHES  │COLLABORATES_WITH                     │
│  └───┬───┘  └────┬────┘  └────────┬─────────┘                   │
│      │           │               │                              │
│      v           v               v                              │
│ ┌─────────────┐ ┌──────────────┐ ┌─────────┐                    │
│ │QUANTUM_LAB  │ │NEURAL_NETWORK│ │BOB_SMITH│                    │
│ │ (ORG)       │ │  (CONCEPT)   │ │ (PERSON)│                    │
│ └─────────────┘ └──────────────┘ └─────────┘                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

With a graph, we can:

1. **Traverse relationships**: Sarah → works_at → Quantum Lab
2. **Discover connections**: Sarah → collaborates_with → Bob
3. **Follow influence**: Sarah's research → used_by → Bob's work

### The Key Insight

> **Entities are the bridge between documents.**
>
> When the same entity (e.g., "Sarah Chen") appears in multiple documents,
> the graph connects those documents through shared nodes.

```
┌─────────────────────────────────────────────────────────────────┐
│              ENTITIES BRIDGE DOCUMENTS                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Document 1          Document 2          Document 3             │
│  ┌─────────┐        ┌─────────┐        ┌─────────┐              │
│  │"Sarah's │        │"Dr. Chen│        │"The lab │              │
│  │ neural  │        │ published│       │ team... │              │
│  │ network │        │ findings"│       │ Sarah"  │              │
│  │ paper"  │        └────┬────┘        └────┬────┘              │
│  └────┬────┘             │                  │                   │
│       │                  │                  │                   │
│       └──────────────────┼──────────────────┘                   │
│                          │                                      │
│                          v                                      │
│                   ┌─────────────┐                               │
│                   │ SARAH_CHEN  │ ← Single unified node         │
│                   │   (PERSON)  │                               │
│                   └─────────────┘                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## The LightRAG Innovation

LightRAG (arxiv:2410.05779) introduced three key innovations:

### 1. Graph-Enhanced Text Indexing

Instead of just storing text chunks, LightRAG:

- Extracts **entities** (people, organizations, concepts)
- Extracts **relationships** between entities
- Builds a **knowledge graph** from these extractions
- Generates **key-value pairs** for efficient retrieval

### 2. Dual-Level Retrieval

LightRAG retrieves information at two levels:

| Level          | Focus                                      | Best For                                |
| -------------- | ------------------------------------------ | --------------------------------------- |
| **Low-Level**  | Specific entities and direct relationships | "Who is Sarah Chen?"                    |
| **High-Level** | Broad topics and themes                    | "What are the main AI research trends?" |

### 3. Incremental Updates

Unlike GraphRAG which requires rebuilding community structures:

- New documents are processed independently
- Extracted entities merge into existing graph
- No full reindex required

### Performance Results (from paper)

| Metric            | LightRAG vs NaiveRAG | LightRAG vs GraphRAG |
| ----------------- | -------------------- | -------------------- |
| Comprehensiveness | 61-84% win rate      | 50-55% win rate      |
| Diversity         | 62-86% win rate      | 59-77% win rate      |
| Empowerment       | 57-84% win rate      | 49-59% win rate      |

---

## Algorithm Walkthrough

### The Complete Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                 EDGEQUAKE GRAPH-RAG PIPELINE                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                     INGESTION PHASE                      │   │
│  ├──────────────────────────────────────────────────────────┤   │
│  │                                                          │   │
│  │   Document ──┬──> Preprocess ──> Chunk ──> Extract       │   │
│  │              │                      │                    │   │
│  │              │                      v                    │   │
│  │              │               ┌─────────────┐             │   │
│  │              │               │ LLM Entity  │             │   │
│  │              │               │ Extraction  │             │   │
│  │              │               └──────┬──────┘             │   │
│  │              │                      │                    │   │
│  │              │        ┌─────────────┼─────────────┐      │   │
│  │              │        │             │             │      │   │
│  │              │        v             v             v      │   │
│  │              │   ┌────────┐   ┌──────────┐   ┌────────┐  │   │
│  │              │   │Entities│   │Relations │   │Chunks  │  │   │
│  │              │   └───┬────┘   └────┬─────┘   └───┬────┘  │   │
│  │              │       │             │             │       │   │
│  │              v       v             v             v       │   │
│  │         ┌────────────────────────────────────────────┐   │   │
│  │         │              KNOWLEDGE GRAPH               │   │   │
│  │         │  (PostgreSQL + Apache AGE + pgvector)      │   │   │
│  │         └────────────────────────────────────────────┘   │   │
│  │                                                          │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                      QUERY PHASE                         │   │
│  ├──────────────────────────────────────────────────────────┤   │
│  │                                                          │   │
│  │   Query ──> Keywords ──> Dual-Level Retrieval            │   │
│  │                               │                          │   │
│  │               ┌───────────────┼───────────────┐          │   │
│  │               │               │               │          │   │
│  │               v               v               v          │   │
│  │         ┌──────────┐   ┌──────────┐   ┌──────────┐       │   │
│  │         │ Entities │   │Relations │   │  Chunks  │       │   │
│  │         └────┬─────┘   └────┬─────┘   └────┬─────┘       │   │
│  │              │              │              │             │   │
│  │              └──────────────┼──────────────┘             │   │
│  │                             │                            │   │
│  │                             v                            │   │
│  │                    ┌────────────────┐                    │   │
│  │                    │ Context Fusion │                    │   │
│  │                    └───────┬────────┘                    │   │
│  │                            │                             │   │
│  │                            v                             │   │
│  │                    ┌────────────────┐                    │   │
│  │                    │  LLM Answer    │                    │   │
│  │                    └────────────────┘                    │   │
│  │                                                          │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Step 1: Document Chunking

Documents are split into manageable chunks for LLM processing:

```rust
// From edgequake-pipeline/src/chunker.rs
pub struct ChunkerConfig {
    pub chunk_size: usize,       // Default: 1200 tokens
    pub chunk_overlap: usize,    // Default: 100 tokens
    pub strategy: ChunkStrategy, // Token, Sentence, Semantic
}
```

**Why adaptive chunking?**

- Large documents (>100KB): Use smaller 600-token chunks
- Medium documents: Use standard 1200-token chunks
- Small documents: May not need chunking at all

### Step 2: Entity Extraction via LLM

EdgeQuake uses a **tuple-delimited format** for extraction:

```
entity<|#|>SARAH_CHEN<|#|>PERSON<|#|>Lead researcher at Quantum Lab
entity<|#|>NEURAL_NETWORK<|#|>CONCEPT<|#|>Machine learning architecture
relation<|#|>SARAH_CHEN<|#|>NEURAL_NETWORK<|#|>research<|#|>Sarah researches neural networks
<|COMPLETE|>
```

**Why tuples over JSON?**

| Aspect           | Tuple Format         | JSON Format                  |
| ---------------- | -------------------- | ---------------------------- |
| Streaming        | ✅ Line-by-line      | ❌ Need complete structure   |
| Partial recovery | ✅ Parse valid lines | ❌ All or nothing            |
| Escaping         | ✅ No special chars  | ❌ Quote/backslash issues    |
| LLM reliability  | ✅ Battle-tested     | ❌ Frequent malformed output |

### Step 3: Entity Normalization

Before storing, entity names are normalized:

```rust
// From edgequake-pipeline/src/prompts/normalizer.rs
normalize_entity_name("John Doe")     → "JOHN_DOE"
normalize_entity_name("the company")  → "COMPANY"
normalize_entity_name("John's team")  → "JOHN_TEAM"
```

**Why normalize?**

Without normalization, the same entity becomes multiple nodes:

```
Before Normalization:        After Normalization:
┌─────────────┐             ┌─────────────┐
│ "John Doe"  │             │  JOHN_DOE   │ ← Single node
└─────────────┘             └─────────────┘
┌─────────────┐                    ▲
│ "john doe"  │ ──────────────────┘
└─────────────┘
┌─────────────┐                    ▲
│ "JOHN DOE"  │ ──────────────────┘
└─────────────┘
```

### Step 4: Graph Construction

Entities and relationships are stored in a knowledge graph:

```sql
-- Entities become graph nodes (Apache AGE)
CREATE (:Entity {
    name: 'SARAH_CHEN',
    type: 'PERSON',
    description: 'Lead researcher...',
    embedding: [0.1, 0.2, ...]  -- pgvector
})

-- Relationships become edges
CREATE (s:Entity)-[:WORKS_AT {
    description: 'Sarah works at Quantum Lab',
    weight: 0.8
}]->(t:Entity)
```

---

## Entity Extraction in Detail

### The Extraction Prompt

EdgeQuake's SOTA extraction prompt (from `entity_extraction.rs`):

```
---Role---
You are a Knowledge Graph Specialist responsible for extracting
entities and relationships from the input text.

---Instructions---
1. **Entity Extraction:**
   - Identify clearly defined entities
   - Use entity types: PERSON, ORGANIZATION, LOCATION, CONCEPT...
   - Provide concise descriptions

2. **Relationship Extraction:**
   - Identify direct relationships between entities
   - Decompose N-ary relationships into binary pairs
   - Use keywords to summarize relationship nature

3. **Output Format:**
   entity<|#|>name<|#|>type<|#|>description
   relation<|#|>source<|#|>target<|#|>keywords<|#|>description

4. **Completion Signal:**
   Output <|COMPLETE|> when finished
```

### Extraction State Machine

```
           ┌─────────────────────────────────────────────────┐
           │                                                 │
           v                                                 │
    ┌──────────────┐                                         │
    │ PREPARE_PROMPT│                                        │
    │ (System + User)│                                       │
    └──────┬───────┘                                         │
           │                                                 │
           v                                                 │
    ┌──────────────┐     finish_reason      ┌──────────────┐ │
    │  LLM_CALL    │────────────────────────│ RETRY_WITH   │ │
    │              │     = "length"         │ 2x TOKENS    │ │
    └──────┬───────┘                        └──────┬───────┘ │
           │                                       │         │
           │ finish_reason = "stop"                └─────────┘
           │                                        (max 3x)
           v
    ┌──────────────┐
    │ PARSE_TUPLES │
    │ (Line by Line)│
    └──────┬───────┘
           │
           v
    ┌──────────────┐
    │  NORMALIZE   │
    │ ENTITY NAMES │
    └──────┬───────┘
           │
           ├────────────────────────────┐
           │                            │ (if gleaning enabled)
           v                            v
    ┌──────────────┐            ┌──────────────┐
    │    RESULT    │            │   GLEANING   │
    │   (Final)    │            │  RE-EXTRACT  │
    └──────────────┘            └──────┬───────┘
                                       │
                                       └───────── Loop back to PARSE
```

### Adaptive Token Management

EdgeQuake handles varying entity density with progressive token scaling:

```rust
// From extractor.rs - Adaptive max_tokens based on chunk complexity
let base_max_tokens = if chunk_size_bytes < 25_000 {
    4096   // Small chunks, few entities
} else if chunk_size_bytes < 75_000 {
    8192   // Medium complexity
} else if chunk_size_bytes < 125_000 {
    12288  // High entity density
} else {
    16384  // Very complex documents
};
```

**Retry strategy on truncation:**

1. Attempt 1: `base_max_tokens` (e.g., 8192)
2. Attempt 2: `2x tokens` (16384) - if truncated
3. Attempt 3: `4x tokens` (32768 max) - if still truncated

---

## Dual-Level Retrieval

### Low-Level Retrieval (Entity-Centric)

Focuses on specific entities and their immediate neighbors:

```
Query: "What is Sarah Chen's research about?"

Low-Level Retrieval:
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│     ┌───────────────┐                                        │
│     │  SARAH_CHEN   │ ← Direct entity match                  │
│     │   (PERSON)    │                                        │
│     └───────┬───────┘                                        │
│             │                                                │
│    ┌────────┼────────────┬─────────────────┐                 │
│    │        │            │                 │                 │
│    v        v            v                 v                 │
│ ┌──────┐ ┌────────┐ ┌─────────┐ ┌────────────────┐           │
│ │WORKS │ │RESEARCHES│ │PUBLISHED│ │COLLABORATES_WITH         │
│ └──┬───┘ └───┬────┘ └────┬────┘ └───────┬────────┘           │
│    │         │           │              │                    │
│    v         v           v              v                    │
│ ┌──────┐ ┌────────────┐ ┌──────┐ ┌──────────┐                │
│ │ LAB  │ │NEURAL_NETS │ │PAPER │ │BOB_SMITH │                │
│ └──────┘ └────────────┘ └──────┘ └──────────┘                │
│                                                              │
│ Returns: Entity descriptions + 1-hop neighbors               │
└──────────────────────────────────────────────────────────────┘
```

### High-Level Retrieval (Topic-Centric)

Focuses on broader themes and community summaries:

```
Query: "What are the main AI research trends?"

High-Level Retrieval:
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              TOPIC CLUSTER: "AI RESEARCH"               │ │
│  │                                                         │ │
│  │  Key themes:                                            │ │
│  │  • Neural network architectures                         │ │
│  │  • Machine learning optimization                        │ │
│  │  • Deep learning applications                           │ │
│  │                                                         │ │
│  │  Related entities: 45                                   │ │
│  │  Related relationships: 128                             │ │
│  │                                                         │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                              │
│  Uses global keywords to match relationship clusters         │
│  Returns: Aggregated summaries + theme keywords              │
└──────────────────────────────────────────────────────────────┘
```

### Hybrid Mode: Best of Both Worlds

```
┌──────────────────────────────────────────────────────────────┐
│                    HYBRID RETRIEVAL                          │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│                      USER QUERY                              │
│                          │                                   │
│            ┌─────────────┴─────────────┐                     │
│            │                           │                     │
│            v                           v                     │
│     ┌─────────────┐           ┌─────────────┐                │
│     │  LOW-LEVEL  │           │ HIGH-LEVEL  │                │
│     │  Entities   │           │  Summaries  │                │
│     │  + 1-hop    │           │  + Topics   │                │
│     └──────┬──────┘           └──────┬──────┘                │
│            │                         │                       │
│            └───────────┬─────────────┘                       │
│                        │                                     │
│                        v                                     │
│              ┌─────────────────┐                             │
│              │  CONTEXT FUSION │                             │
│              │                 │                             │
│              │ • Deduplicate   │                             │
│              │ • Score & rank  │                             │
│              │ • Truncate to   │                             │
│              │   token limit   │                             │
│              └────────┬────────┘                             │
│                       │                                      │
│                       v                                      │
│              ┌─────────────────┐                             │
│              │   LLM ANSWER    │                             │
│              └─────────────────┘                             │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

## Query Modes Explained

EdgeQuake extends LightRAG's 3 modes to 6:

### Mode Selection Decision Tree

```
                          START
                            │
                            v
                ┌───────────────────────┐
                │ Is this a test/debug? │
                └───────────┬───────────┘
                            │
               Yes          │          No
                │           │           │
                v           │           v
          ┌──────────┐      │  ┌─────────────────────┐
          │  BYPASS  │      │  │ Specific entity     │
          │ (No RAG) │      │  │ question?           │
          └──────────┘      │  └──────────┬──────────┘
                            │             │
                            │  Yes        │         No
                            │   │         │          │
                            │   v         │          v
                            │ ┌────────┐  │  ┌──────────────────┐
                            │ │ LOCAL  │  │  │ Broad theme/     │
                            │ │        │  │  │ summary needed?  │
                            │ └────────┘  │  └────────┬─────────┘
                            │             │           │
                            │             │  Yes      │      No
                            │             │   │       │       │
                            │             │   v       │       v
                            │             │ ┌──────┐  │  ┌──────────────┐
                            │             │ │GLOBAL│  │  │ Need both    │
                            │             │ └──────┘  │  │ entity +     │
                            │             │           │  │ context?     │
                            │             │           │  └──────┬───────┘
                            │             │           │         │
                            │             │           │  Yes    │    No
                            │             │           │   │     │     │
                            │             │           │   v     │     v
                            │             │           │ ┌──────┐│ ┌──────┐
                            │             │           │ │HYBRID││ │NAIVE │
                            │             │           │ └──────┘│ └──────┘
                            │             │           │         │
                            └─────────────┴───────────┴─────────┘
```

### Mode Comparison Table

| Mode       | Vector Search | Graph Traversal         | Best For               |
| ---------- | ------------- | ----------------------- | ---------------------- |
| **Naive**  | ✅ Yes        | ❌ No                   | Simple factual queries |
| **Local**  | ✅ Yes        | ✅ Entities + neighbors | "Who/What is X?"       |
| **Global** | ❌ No         | ✅ Community summaries  | "What are the themes?" |
| **Hybrid** | ✅ Yes        | ✅ Both approaches      | Complex multi-faceted  |
| **Mix**    | ✅ Weighted   | ✅ Weighted             | Custom blending        |
| **Bypass** | ❌ No         | ❌ No                   | Testing/debugging      |

### Code Reference

```rust
// From edgequake-query/src/modes.rs
pub enum QueryMode {
    Naive,   // FEAT0101: Vector similarity only
    Local,   // FEAT0102: Entity-centric graph
    Global,  // FEAT0103: Community summaries
    Hybrid,  // FEAT0104: Local + Global (DEFAULT)
    Mix,     // FEAT0105: Weighted combination
    Bypass,  // FEAT0106: No RAG, direct LLM
}
```

---

## Gleaning: Multi-Pass Extraction

### Why Gleaning?

LLMs often miss entities in a single pass due to:

- Attention limits on long texts
- Implicit entities ("the company" → previously mentioned "Apple")
- Context overload with many entities

### The Gleaning Process

```
┌─────────────────────────────────────────────────────────────────┐
│                    GLEANING (RE-EXTRACTION)                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Pass 1: Initial Extraction                                     │
│  ─────────────────────────                                      │
│  Input: "Sarah Chen leads the team at Quantum Lab.              │
│          The company recently expanded..."                      │
│                                                                 │
│  Extracted: SARAH_CHEN, QUANTUM_LAB                             │
│  Missed: "The company" = QUANTUM_LAB (implicit reference)       │
│                                                                 │
│  ────────────────────────────────────────────────────────────   │
│                                                                 │
│  Pass 2: Gleaning                                               │
│  ─────────────────                                              │
│  Prompt: "MANY entities were missed. Already found:             │
│           SARAH_CHEN, QUANTUM_LAB. Look for implicit mentions." │
│                                                                 │
│  Additional: TEAM (implicit), EXPANSION_EVENT (implicit)        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Research Finding

From LightRAG paper:

- **1-2 gleaning iterations** improve recall by 15-25%
- Diminishing returns after 2 iterations
- Cost: Each iteration = 1 additional LLM call

### Configuration

```rust
// From extractor.rs
pub struct GleaningConfig {
    pub max_gleaning: usize,  // Default: 1 (LightRAG recommendation)
    pub always_glean: bool,   // Default: false
}
```

---

## EdgeQuake Innovations

EdgeQuake extends the original LightRAG with:

### 1. Adaptive Error Recovery

```
┌─────────────────────────────────────────────────────────────────┐
│                ADAPTIVE TOKEN MANAGEMENT                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Chunk Size (KB)     Base Tokens    Retry Tokens                │
│  ───────────────     ───────────    ────────────                │
│  < 25 KB             4,096          8,192 → 16,384              │
│  25-75 KB            8,192          16,384 → 32,768             │
│  75-125 KB           12,288         24,576 → 32,768             │
│  > 125 KB            16,384         32,768 (max)                │
│                                                                 │
│  Detection:                                                     │
│  • finish_reason="length" → LLM hit token limit                 │
│  • JSON parse error → Response truncated mid-output             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 2. Hybrid Parser with Fallback

```rust
// From parser.rs
pub struct HybridExtractionParser {
    json_parser: JsonExtractionParser,
    tuple_parser: TupleParser,
    prefer_tuple: bool,  // Default: true
}
```

Priority:

1. Try tuple parsing (more robust)
2. Fallback to JSON if tuples empty
3. Return best available result

### 3. Extended Query Modes

LightRAG: 3 modes (local, global, hybrid)
EdgeQuake: 6 modes (+naive, mix, bypass)

### 4. Multi-Provider Support

```rust
// Providers available
- OpenAI (gpt-4o-mini, gpt-4o)
- Ollama (local models)
- LM Studio (local models)
- Mock (testing)
```

### 5. Multi-Tenant Support

```rust
// Query can be scoped to tenant/workspace
QueryRequest::new("What is AI?")
    .with_tenant_id("acme-corp")
    .with_workspace_id("research-team")
```

---

## Comparisons

### EdgeQuake vs LightRAG (Python)

| Feature        | LightRAG        | EdgeQuake                   |
| -------------- | --------------- | --------------------------- |
| Language       | Python          | Rust (async Tokio)          |
| Performance    | Single-threaded | Multi-threaded              |
| Query modes    | 3               | 6                           |
| Error handling | Basic           | Adaptive retry              |
| Multi-tenant   | No              | Yes                         |
| Streaming      | Limited         | Full SSE                    |
| Storage        | Neo4j           | PostgreSQL + AGE + pgvector |

### LightRAG vs GraphRAG

| Aspect              | LightRAG    | GraphRAG        |
| ------------------- | ----------- | --------------- |
| Retrieval cost      | ~100 tokens | ~610,000 tokens |
| API calls per query | 1-2         | Hundreds        |
| Update strategy     | Incremental | Full rebuild    |
| Community detection | No          | Yes             |
| Query speed         | Fast        | Slow            |

### LightRAG vs NaiveRAG

| Aspect            | LightRAG     | NaiveRAG   |
| ----------------- | ------------ | ---------- |
| Relationships     | ✅ Explicit  | ❌ None    |
| Multi-hop queries | ✅ Supported | ❌ Limited |
| Win rate          | 60-85%       | Baseline   |
| Index complexity  | Higher       | Lower      |
| Storage needs     | More         | Less       |

---

## References

1. **LightRAG Paper**: Guo et al., "LightRAG: Simple and Fast Retrieval-Augmented Generation", arXiv:2410.05779, 2024

2. **GraphRAG Paper**: Edge et al., "From Local to Global: A Graph RAG Approach to Query-Focused Summarization", arXiv:2404.16130, 2024

3. **EdgeQuake Source Code**:
   - [entity_extraction.rs](../edgequake/crates/edgequake-pipeline/src/prompts/entity_extraction.rs)
   - [normalizer.rs](../edgequake/crates/edgequake-pipeline/src/prompts/normalizer.rs)
   - [parser.rs](../edgequake/crates/edgequake-pipeline/src/prompts/parser.rs)
   - [modes.rs](../edgequake/crates/edgequake-query/src/modes.rs)

---

## Next Steps

- [Query Mode Selection Guide](./query-modes.md)
- [Entity Normalization Technical Note](./entity-normalization.md)
- [API Reference: Query Endpoints](../api-reference/query.md)
