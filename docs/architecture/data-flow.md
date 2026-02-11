# EdgeQuake Data Flow

> How documents flow through ingestion and how queries are processed

---

## Overview

EdgeQuake has two main data flows:

1. **Ingestion Flow**: Document → Knowledge Graph + Vectors
2. **Query Flow**: Question → Hybrid Retrieval → Answer

```
┌─────────────────────────────────────────────────────────────────────┐
│                        EdgeQuake Data Flow                          │
│                                                                     │
│  INGESTION                              QUERY                       │
│  ─────────                              ─────                       │
│                                                                     │
│  Document ─┐                            Question ─┐                 │
│            │                                      │                 │
│            ▼                                      ▼                 │
│    ┌──────────────┐                      ┌──────────────┐           │
│    │   Pipeline   │                      │ QueryEngine  │           │
│    └──────┬───────┘                      └──────┬───────┘           │
│           │                                     │                   │
│           ▼                                     ▼                   │
│    ┌──────────────┐                      ┌──────────────┐           │
│    │  Knowledge   │◄────────────────────▶│   Hybrid     │           │
│    │    Graph     │                      │  Retrieval   │           │
│    └──────────────┘                      └──────┬───────┘           │
│                                                 │                   │
│                                                 ▼                   │
│                                          ┌──────────────┐           │
│                                          │   Answer     │           │
│                                          └──────────────┘           │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Document Ingestion Pipeline

### Sequence Diagram

```
┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐
│ Client │    │  API   │    │ Core   │    │Pipeline│    │  LLM   │    │Storage │
└───┬────┘    └───┬────┘    └───┬────┘    └───┬────┘    └───┬────┘    └───┬────┘
    │             │             │             │             │             │
    │ POST /docs  │             │             │             │             │
    │────────────▶│             │             │             │             │
    │             │ insert()    │             │             │             │
    │             │────────────▶│             │             │             │
    │             │             │ process()   │             │             │
    │             │             │────────────▶│             │             │
    │             │             │             │ chunk()     │             │
    │             │             │             │────────────▶│             │
    │             │             │             │   chunks    │             │
    │             │             │             │◀────────────│             │
    │             │             │             │             │             │
    │             │             │             │ extract()   │             │
    │             │             │             │────────────▶│             │
    │             │             │             │   (LLM)     │             │
    │             │             │             │  entities   │             │
    │             │             │             │◀────────────│             │
    │             │             │             │             │             │
    │             │             │             │ embed()     │             │
    │             │             │             │────────────▶│             │
    │             │             │             │  vectors    │             │
    │             │             │             │◀────────────│             │
    │             │             │             │             │             │
    │             │             │             │ merge()     │             │
    │             │             │             │────────────▶│             │
    │             │             │             │   (dedup)   │             │
    │             │             │             │◀────────────│             │
    │             │             │             │             │             │
    │             │             │             │ store()     │             │
    │             │             │             │────────────────────────▶  │
    │             │             │             │   ack       │             │
    │             │             │             │◀────────────────────────  │
    │             │             │  result     │             │             │
    │             │             │◀────────────│             │             │
    │             │  result     │             │             │             │
    │             │◀────────────│             │             │             │
    │  response   │             │             │             │             │
    │◀────────────│             │             │             │             │
    │             │             │             │             │             │
```

### Pipeline Stages Detail

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         STAGE 1: CHUNKING                                   │
│                                                                             │
│  Input: Raw document text                                                   │
│  Output: TextChunk[]                                                        │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │  "Marie Curie was a Polish-French physicist who discovered radium.   │   │
│  │   She won two Nobel Prizes..."                                       │   │
│  │                                                                      │   │
│  │   ──▶ Token count: 1,500 tokens                                      │   │
│  │   ──▶ Chunk size: 1,200 tokens                                       │   │
│  │   ──▶ Overlap: 100 tokens                                            │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                       │                                     │
│                                       ▼                                     │
│  ┌────────────────────────┐  ┌────────────────────────┐                     │
│  │ Chunk 1 (1,200 tokens) │  │ Chunk 2 (400 tokens)   │                     │
│  │ "Marie Curie was a..." │  │ "...Nobel Prizes..."   │                     │
│  └────────────────────────┘  └────────────────────────┘                     │
│                                                                             │
│  Config: chunk_token_size=1200, overlap=100                                 │
│  Business Rule: BR0002 (chunk size limits)                                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                         STAGE 2: ENTITY EXTRACTION                          │
│                                                                             │
│  Input: TextChunk[]                                                         │
│  Output: ExtractedEntity[], ExtractedRelationship[]                         │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │  LLM Prompt:                                                         │   │
│  │  "Extract entities from the following text. Entity types:            │   │
│  │   PERSON, ORGANIZATION, LOCATION, EVENT, CONCEPT, TECHNOLOGY, PRODUCT│   │
│  │                                                                      │   │
│  │   Text: 'Marie Curie was a Polish-French physicist...'               │   │
│  │                                                                      │   │
│  │   Output format: (entity_name; entity_type; description)"            │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                       │                                     │
│                                       ▼                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  Extracted Entities:                                                │    │
│  │  ┌────────────────────────────────────────────────────────────────┐ │    │
│  │  │ name: MARIE_CURIE                                              │ │    │
│  │  │ type: PERSON                                                   │ │    │
│  │  │ description: Polish-French physicist, chemist, Nobel laureate  │ │    │
│  │  └────────────────────────────────────────────────────────────────┘ │    │
│  │  ┌────────────────────────────────────────────────────────────────┐ │    │
│  │  │ name: RADIUM                                                   │ │    │
│  │  │ type: CONCEPT                                                  │ │    │
│  │  │ description: Radioactive element discovered by Marie Curie     │ │    │
│  │  └────────────────────────────────────────────────────────────────┘ │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  Business Rules: BR0003 (entity types), BR0005 (desc max 512 tokens)        │
│                  BR0008 (UPPERCASE_UNDERSCORE names)                        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                         STAGE 3: RELATIONSHIP EXTRACTION                    │
│                                                                             │
│  Input: Entities + Text context                                             │
│  Output: ExtractedRelationship[]                                            │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │  LLM Prompt:                                                         │   │
│  │  "Given entities: MARIE_CURIE, RADIUM, NOBEL_PRIZE                   │   │
│  │   Extract relationships between them.                                │   │
│  │                                                                      │   │
│  │   Format: (source; target; keywords; description)"                   │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                       │                                     │
│                                       ▼                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  Extracted Relationships:                                           │    │
│  │  ┌────────────────────────────────────────────────────────────────┐ │    │
│  │  │ source: MARIE_CURIE                                            │ │    │
│  │  │ target: RADIUM                                                 │ │    │
│  │  │ keywords: [discovered, isolated, researched]                   │ │    │
│  │  │ description: Marie Curie discovered radium in 1898             │ │    │
│  │  └────────────────────────────────────────────────────────────────┘ │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  Business Rules: BR0004 (max 5 keywords), BR0006 (no self-relationships)    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                         STAGE 4: EMBEDDING GENERATION                       │
│                                                                             │
│  Input: Chunks + Entities + Relationships                                   │
│  Output: Vector embeddings (1536 dimensions for OpenAI)                     │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                                                                     │    │
│  │   Text: "Marie Curie discovered radium"                             │    │
│  │                    │                                                │    │
│  │                    ▼                                                │    │
│  │   ┌──────────────────────────────────────────────────────────────┐  │    │
│  │   │ EmbeddingProvider.embed(text)                                │  │    │
│  │   │                                                              │  │    │
│  │   │ Result: [0.023, -0.041, 0.089, ..., 0.012]  (1536 dims)      │  │    │
│  │   └──────────────────────────────────────────────────────────────┘  │    │
│  │                                                                     │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  Batch Processing: embedding_batch_size (default: 100)                      │
│  Business Rule: BR0010 (embedding dimension validated)                      │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                         STAGE 5: MERGE & DEDUPLICATION                      │
│                                                                             │
│  Input: Raw entities/relationships                                          │
│  Output: Merged, deduplicated knowledge graph                               │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  Before Merge:                                                      │    │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐            │    │
│  │  │ MARIE_CURIE   │  │ Marie Curie   │  │ Curie         │            │    │
│  │  │ (from chunk 1)│  │ (from chunk 2)│  │ (from chunk 3)│            │    │
│  │  └───────────────┘  └───────────────┘  └───────────────┘            │    │
│  │                                                                     │    │
│  │  After Merge:                                                       │    │
│  │  ┌───────────────────────────────────────────────────────┐          │    │
│  │  │ MARIE_CURIE                                           │          │    │
│  │  │ description: Polish-French physicist and chemist who  │          │    │
│  │  │              discovered radium and polonium. First    │          │    │
│  │  │              woman to win Nobel Prize... (merged)     │          │    │
│  │  │ source_chunks: [chunk_1, chunk_2, chunk_3]            │          │    │
│  │  └───────────────────────────────────────────────────────┘          │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  Strategies: Embedding similarity, LLM-based summarization                  │
│  Deduplication: 20-40% reduction typical                                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                         STAGE 6: STORAGE                                    │
│                                                                             │
│  Input: Merged entities, relationships, chunks, vectors                     │
│  Output: Persisted data in 3 storage types                                  │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                                                                     │    │
│  │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐              │    │
│  │  │  KVStorage  │    │VectorStorage│    │GraphStorage │              │    │
│  │  │             │    │             │    │             │              │    │
│  │  │  Documents  │    │  Chunk      │    │   Nodes     │              │    │
│  │  │  Chunks     │    │  Vectors    │    │  (entities) │              │    │
│  │  │  Metadata   │    │             │    │             │              │    │
│  │  │             │    │  Entity     │    │   Edges     │              │    │
│  │  │             │    │  Vectors    │    │  (relations)│              │    │
│  │  └─────────────┘    └─────────────┘    └─────────────┘              │    │
│  │                                                                     │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  Multi-tenancy: namespace-based isolation (tenant_id, workspace_id)         │
│  Business Rule: BR0201 (tenant isolation)                                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Query Execution Flow

### State Machine

```
                              ┌─────────────────┐
                              │     START       │
                              └────────┬────────┘
                                       │
                                       ▼
                              ┌─────────────────┐
                              │  Parse Query    │
                              │  + Extract Mode │
                              └────────┬────────┘
                                       │
                          ┌────────────┴────────────┐
                          │                         │
                          ▼                         ▼
                   ┌─────────────┐          ┌─────────────┐
                   │ mode=bypass?│──Yes───▶ │ Direct LLM  │
                   └──────┬──────┘          └──────┬──────┘
                          │ No                     │
                          ▼                        │
                   ┌─────────────────┐             │
                   │ Keyword Extract │             │
                   └────────┬────────┘             │
                            │                      │
              ┌─────────────┼─────────────┐        │
              │             │             │        │
              ▼             ▼             ▼        │
       ┌───────────┐ ┌───────────┐ ┌───────────┐   │
       │   Naive   │ │   Local   │ │  Global   │   │
       │ (vector)  │ │ (entity)  │ │(community)│   │
       └─────┬─────┘ └─────┬─────┘ └─────┬─────┘   │
             │             │             │         │
             └──────┬──────┴──────┬──────┘         │
                    │             │                │
                    ▼             ▼                │
             ┌───────────┐ ┌───────────┐           │
             │  Hybrid   │ │    Mix    │           │
             │ (L+G)     │ │ (weighted)│           │
             └─────┬─────┘ └─────┬─────┘           │
                   │             │                 │
                   └──────┬──────┘                 │
                          │                        │
                          ▼                        │
                   ┌─────────────────┐             │
                   │ Context Assembly│             │
                   │ + Truncation    │             │
                   └────────┬────────┘             │
                            │                      │
                            └──────────┬───────────┘
                                       │
                                       ▼
                              ┌─────────────────┐
                              │  LLM Generation │
                              └────────┬────────┘
                                       │
                                       ▼
                              ┌─────────────────┐
                              │    Response     │
                              └─────────────────┘
```

### Query Modes Explained

| Mode       | Retrieval Strategy            | Best For                  |
| ---------- | ----------------------------- | ------------------------- |
| **naive**  | Vector similarity only        | Simple factual queries    |
| **local**  | Entity + neighbors (1-2 hops) | Specific entity questions |
| **global** | Community aggregation         | Broad topic overviews     |
| **hybrid** | Local + Global (default)      | General purpose           |
| **mix**    | Weighted combination all      | Tunable precision/recall  |
| **bypass** | No retrieval, direct LLM      | Creative tasks, chat      |

### Retrieval Detail by Mode

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                         NAIVE MODE                                           │
│                                                                              │
│  Query: "What is radium?"                                                    │
│                                                                              │
│  1. Embed query ──▶ [0.012, -0.034, ...]                                     │
│  2. Vector search in chunks                                                  │
│  3. Return top-k similar chunks                                              │
│                                                                              │
│  ┌─────────────┐                                                             │
│  │   Query     │                                                             │
│  │   Vector    │──▶ Similarity Search ──▶ [Chunk 1, Chunk 3, Chunk 7]        │
│  └─────────────┘    (cosine distance)                                        │
│                                                                              │
│  Pro: Fast, simple                                                           │
│  Con: Misses related context not in similar chunks                           │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────────────┐
│                         LOCAL MODE                                           │
│                                                                              │
│  Query: "Who discovered radium?"                                             │
│                                                                              │
│  1. Embed query                                                              │
│  2. Find similar entities (RADIUM)                                           │
│  3. Traverse graph to neighbors (1-2 hops)                                   │
│  4. Collect entity descriptions + related chunks                             │
│                                                                              │
│         ┌─────────────┐                                                      │
│         │   RADIUM    │◀─── Query matches this entity                        │
│         └──────┬──────┘                                                      │
│                │ 1 hop                                                       │
│    ┌───────────┼───────────┐                                                 │
│    │           │           │                                                 │
│    ▼           ▼           ▼                                                 │
│ ┌──────┐  ┌──────────┐  ┌──────────┐                                         │
│ │CURIE │  │POLONIUM  │  │NOBEL_PRIZE│ ◀─── Neighbors included                │
│ └──────┘  └──────────┘  └──────────┘                                         │
│                                                                              │
│  Pro: Entity-focused, relationship-aware                                     │
│  Con: May miss global patterns                                               │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────────────┐
│                         GLOBAL MODE                                          │
│                                                                              │
│  Query: "Summarize early 20th century physics"                               │
│                                                                              │
│  1. Find entities matching query                                             │
│  2. Identify community clusters                                              │
│  3. Aggregate community-level summaries                                      │
│                                                                              │
│  ┌──────────────────────────────────────────────────┐                        │
│  │              Community: Physics Pioneers         │                        │
│  │  ┌────────┐  ┌────────┐  ┌────────┐              │                        │
│  │  │ CURIE  │──│EINSTEIN│──│PLANCK  │              │                        │
│  │  └────────┘  └────────┘  └────────┘              │                        │
│  │       │           │           │                  │                        │
│  │       ▼           ▼           ▼                  │                        │
│  │  ┌─────────────────────────────────────────┐     │                        │
│  │  │ Community Summary: "Early 20th century  │     │                        │
│  │  │ physics was defined by discoveries in   │     │                        │
│  │  │ radioactivity, relativity, and quantum  │     │                        │
│  │  │ mechanics..."                           │     │                        │
│  │  └─────────────────────────────────────────┘     │                        │
│  └──────────────────────────────────────────────────┘                        │
│                                                                              │
│  Pro: Big picture, thematic understanding                                    │
│  Con: May miss specific details                                              │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────────────┐
│                         HYBRID MODE (Default)                                │
│                                                                              │
│  Combines Local + Global for balanced retrieval                              │
│                                                                              │
│  ┌────────────────────┐                                                      │
│  │       Query        │                                                      │
│  └──────────┬─────────┘                                                      │
│             │                                                                │
│      ┌──────┴──────┐                                                         │
│      │             │                                                         │
│      ▼             ▼                                                         │
│  ┌───────┐    ┌───────┐                                                      │
│  │ Local │    │Global │                                                      │
│  └───┬───┘    └───┬───┘                                                      │
│      │            │                                                          │
│      └─────┬──────┘                                                          │
│            ▼                                                                 │
│      ┌───────────┐                                                           │
│      │   Merge   │                                                           │
│      │  Results  │                                                           │
│      └───────────┘                                                           │
│                                                                              │
│  Pro: Best of both worlds                                                    │
│  Con: More compute                                                           │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Context Assembly

After retrieval, context is assembled and truncated to fit LLM limits:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     CONTEXT ASSEMBLY                                        │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │ Token Budget Allocation (example: 8000 tokens max)                  │    │
│  │                                                                     │    │
│  │  ┌────────────────────────────────────────────────────────────────┐ │    │
│  │  │ System Prompt                                            500   │ │    │
│  │  ├────────────────────────────────────────────────────────────────┤ │    │
│  │  │ Entity Descriptions (sorted by relevance)               2000   │ │    │
│  │  ├────────────────────────────────────────────────────────────────┤ │    │
│  │  │ Relationship Descriptions                               1500   │ │    │
│  │  ├────────────────────────────────────────────────────────────────┤ │    │
│  │  │ Source Chunks                                           3000   │ │    │
│  │  ├────────────────────────────────────────────────────────────────┤ │    │
│  │  │ User Query                                               500   │ │    │
│  │  ├────────────────────────────────────────────────────────────────┤ │    │
│  │  │ Reserved for Response                                    500   │ │    │
│  │  └────────────────────────────────────────────────────────────────┘ │    │
│  │                                                    Total: 8000      │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  Truncation Strategy:                                                       │
│  - Sort by relevance score                                                  │
│  - Truncate from end of each section                                        │
│  - Maintain minimum entity/relationship coverage                            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Error Handling Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     ERROR HANDLING                                          │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                          API Layer                                    │  │
│  │                                                                       │  │
│  │  Request ──▶ Validation ──▶ Handler ──▶ Response                      │  │
│  │                 │              │                                      │  │
│  │                 ▼              ▼                                      │  │
│  │            ValidationError  ServiceError                              │  │
│  │                 │              │                                      │  │
│  │                 └──────┬───────┘                                      │  │
│  │                        ▼                                              │  │
│  │                 ┌───────────────┐                                     │  │
│  │                 │ Error Handler │                                     │  │
│  │                 │  (RFC 7807)   │                                     │  │
│  │                 └───────┬───────┘                                     │  │
│  │                         ▼                                             │  │
│  │                 ┌───────────────┐                                     │  │
│  │                 │ JSON Response │                                     │  │
│  │                 │ {             │                                     │  │
│  │                 │   "type": "...",│                                   │  │
│  │                 │   "title": "...",│                                  │  │
│  │                 │   "status": 400,│                                   │  │
│  │                 │   "detail": "..."│                                  │  │
│  │                 │ }             │                                     │  │
│  │                 └───────────────┘                                     │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  Error Categories:                                                          │
│  - 400: Validation errors (bad input)                                       │
│  - 401: Authentication required                                             │
│  - 404: Resource not found                                                  │
│  - 429: Rate limit exceeded                                                 │
│  - 500: Internal server error                                               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Next Steps

- **[Architecture Overview](overview.md)** — System design
- **[Query Modes Deep Dive](../deep-dives/query-modes.md)** — Choosing the right mode
- **[API Reference](../api-reference/rest-api.md)** — Endpoint documentation
