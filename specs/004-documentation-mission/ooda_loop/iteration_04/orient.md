# OODA Iteration 04 - Orient

**Date**: 2025-01-XX
**Focus**: Core Concepts Documentation

## 🧭 Orientation

### 1. Document Structure Strategy

Each concept document should follow this structure:

1. **One-sentence definition** - What is it?
2. **Why it matters** - First principles
3. **How it works** - Simplified explanation
4. **Key components** - Building blocks
5. **Code reference** - Link to implementation
6. **Learn more** - Link to deep-dives

### 2. Concept Relationships

```
┌─────────────────────────────────────────────────────────────────┐
│                    READER JOURNEY                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  Question: "What is EdgeQuake?"                                  │
│       │                                                          │
│       └──▶ [graph-rag.md] "It's a Graph-RAG system"             │
│                │                                                 │
│  Question: "How does it build the graph?"                       │
│       │                                                          │
│       └──▶ [entity-extraction.md] "By extracting entities"      │
│                │                                                 │
│  Question: "Where is the data stored?"                          │
│       │                                                          │
│       └──▶ [knowledge-graph.md] "In a knowledge graph"          │
│                │                                                 │
│  Question: "How does querying work?"                            │
│       │                                                          │
│       └──▶ [hybrid-retrieval.md] "Using hybrid search"          │
│                │                                                 │
│  Question: "I want more details!"                               │
│       │                                                          │
│       └──▶ [deep-dives/lightrag-algorithm.md]                   │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

### 3. Writing Guidelines

- **Length**: 150-200 lines per concept (digestible)
- **Diagrams**: 1-2 ASCII diagrams per concept
- **Code**: Minimal - reference files, don't duplicate
- **Tone**: Educational, friendly, not academic
- **Links**: Cross-reference related concepts

### 4. Priority Order

1. **graph-rag.md** - Foundation (must come first)
2. **entity-extraction.md** - Answers "how does it work?"
3. **knowledge-graph.md** - Answers "where is data stored?"
4. **hybrid-retrieval.md** - Answers "how do I query?"

### 5. Key Messages Per Concept

| Concept           | Key Message                                             |
| ----------------- | ------------------------------------------------------- |
| Graph-RAG         | Relationships matter - graphs capture what vectors miss |
| Entity Extraction | LLM as knowledge engineer - turning text into structure |
| Knowledge Graph   | Entities + edges = interconnected memory                |
| Hybrid Retrieval  | Best of both worlds - vector precision + graph context  |
