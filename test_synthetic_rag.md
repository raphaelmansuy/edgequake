# EdgeQuake Knowledge Graph Test Document

This synthetic document is designed to test the RAG system's entity extraction and relevance scoring capabilities.

## Key Entities for Testing

### Organizations

**EdgeQuake Inc.** is a cutting-edge technology company specializing in knowledge graph systems. The company was founded by **Dr. Sarah Chen** and **Marcus Williams** in 2024.

**Acme Research Labs** partnered with EdgeQuake to develop advanced retrieval-augmented generation systems.

### Technologies

The **LightRAG Algorithm** is the foundation of EdgeQuake's entity extraction pipeline. It uses advanced NLP techniques to identify:
- Named entities (people, organizations, locations)
- Concepts and abstract ideas
- Relationships between entities

**Graph Neural Networks (GNNs)** power the relationship discovery mechanism, enabling the system to find hidden connections between seemingly unrelated pieces of information.

**PostgreSQL with Apache AGE** provides the graph storage backend, allowing for efficient traversal of complex knowledge structures.

### Projects

The **Project Alpha Initiative** aims to improve document indexing by 10x while maintaining high accuracy. Key metrics include:
- Precision: 95.5%
- Recall: 92.3%
- F1 Score: 93.9%

**Project Beta** focuses on real-time streaming responses and incremental knowledge graph updates.

## Technical Architecture

The system architecture consists of three main layers:

1. **Ingestion Layer**: Handles document upload, chunking, and preprocessing
2. **Knowledge Layer**: Manages entity extraction, relationship discovery, and graph construction
3. **Query Layer**: Processes user queries using hybrid retrieval (vector + graph)

### Performance Benchmarks

| Metric | Value | Notes |
|--------|-------|-------|
| Indexing Speed | 150 docs/min | With batch processing |
| Query Latency | 45ms p95 | Using caching |
| Entity Accuracy | 94.2% | Human-verified |

## Relationships

- **Dr. Sarah Chen** → FOUNDED → **EdgeQuake Inc.**
- **Marcus Williams** → FOUNDED → **EdgeQuake Inc.**
- **EdgeQuake Inc.** → DEVELOPED → **LightRAG Algorithm**
- **LightRAG Algorithm** → USES → **Graph Neural Networks**
- **Project Alpha** → IMPROVES → **Ingestion Layer**
- **Project Beta** → ENHANCES → **Query Layer**
- **Acme Research Labs** → PARTNERS_WITH → **EdgeQuake Inc.**

## Conclusion

This document contains multiple entities and relationships that should be extracted with high confidence scores. The relevance scoring should reflect:
- Direct mentions: High score (>0.85)
- Contextual references: Medium score (0.5-0.85)
- Weak associations: Low score (<0.5)

Use queries like:
- "What projects is EdgeQuake working on?"
- "Who founded EdgeQuake?"
- "How does LightRAG work?"
- "What is the relationship between Sarah Chen and EdgeQuake?"
