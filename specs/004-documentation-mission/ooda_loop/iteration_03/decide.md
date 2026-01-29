# OODA Iteration 03 - Decide

**Date**: 2025-01-XX
**Focus**: LightRAG Algorithm Deep-Dive Documentation

## 🎯 Decision

Create a comprehensive LightRAG algorithm deep-dive article that explains:

1. **Why Graph-RAG?** - First principles explanation
2. **The Algorithm** - Step-by-step walkthrough with diagrams
3. **Implementation Details** - EdgeQuake-specific innovations
4. **Query Modes** - When to use each mode

## 📄 Deliverables

### Primary Deliverable

`docs/deep-dives/lightrag-algorithm.md` (~600+ lines)

### Sections to Include

1. **Introduction**
   - What is Graph-RAG?
   - Why not traditional vector-only RAG?
   - LightRAG paper context

2. **Core Concepts**
   - Entity extraction
   - Relationship extraction
   - Knowledge graph construction

3. **Algorithm Walkthrough**
   - Document ingestion flow
   - Chunking strategy
   - LLM extraction prompts
   - Tuple parsing
   - Entity normalization

4. **Dual-Level Retrieval**
   - Low-level retrieval explained
   - High-level retrieval explained
   - How they combine

5. **Query Modes**
   - Mode selection decision tree
   - Use cases for each mode
   - Performance trade-offs

6. **Advanced Topics**
   - Gleaning (multi-pass extraction)
   - Adaptive token management
   - Error recovery strategies

7. **Comparisons**
   - EdgeQuake vs LightRAG Python
   - LightRAG vs GraphRAG
   - LightRAG vs NaiveRAG

## 📊 Diagram Requirements

| Diagram              | Type          | Purpose                            |
| -------------------- | ------------- | ---------------------------------- |
| Graph-RAG Overview   | ASCII         | Show document → graph → query flow |
| Extraction Pipeline  | Flowchart     | LLM call → parse → normalize       |
| Dual-Level Retrieval | Split diagram | Low vs High level                  |
| Query Mode Decision  | Decision tree | Mode selection guide               |
| Entity Normalization | Examples      | Before/after transformation        |
| Gleaning Loop        | State machine | Multi-pass extraction              |

## ✅ Acceptance Criteria

- [ ] Article has clear First Principles explanation
- [ ] All diagrams use ASCII art
- [ ] Code examples reference actual EdgeQuake code
- [ ] LightRAG paper is properly cited
- [ ] Query mode guide is actionable
- [ ] Minimum 600 lines of high-signal content

## 🔗 Cross-References

Link to:

- `docs/getting-started/quick-start.md` (for hands-on examples)
- `docs/architecture/data-flow.md` (for pipeline details)
- `docs/api-reference/query-modes.md` (future article)
