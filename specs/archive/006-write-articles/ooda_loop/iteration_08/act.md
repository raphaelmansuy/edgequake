# Iteration 08: Act - Query Engine Deliverables

## Completed Deliverables

### Article: 008_query_engine

| Platform   | File            | Status      | Word Count |
| ---------- | --------------- | ----------- | ---------- |
| Medium     | `medium.md`     | ✅ Complete | ~2,200     |
| LinkedIn   | `linkedin.md`   | ✅ Complete | ~290       |
| X.com      | `xcom.md`       | ✅ Complete | 15 tweets  |
| HackerNews | `hackernews.md` | ✅ Complete | ~750       |
| Reddit     | `reddit.md`     | ✅ Complete | ~700       |
| Substack   | `substack.md`   | ✅ Complete | ~1,900     |

## Key Messages Delivered

### WHY (Pain Point)

- Vector similarity fails for relationship questions
- "How does A collaborate with B?" requires graph traversal
- One retrieval strategy doesn't fit all question types

### HOW (Solution)

- 5 query modes (Naive, Local, Global, Hybrid, Mix)
- LightRAG multi-level keyword extraction
- Mode selection matches question type
- Token budgeting with graph priority

### WHAT (Implementation)

- SOTAQueryEngine with configurable modes
- Keyword caching (24h TTL, 70-90% hit rate)
- Adaptive mode selection option
- Optional reranking for precision

## Technical Accuracy

All code snippets sourced from actual codebase:

- `edgequake-query/src/modes.rs`: QueryMode enum
- `edgequake-query/src/sota_engine.rs`: SOTAQueryEngine implementation
- `edgequake-query/src/truncation.rs`: Token budgeting

## Research Paper Citation

- LightRAG (arXiv:2410.05779) cited in all articles
- Multi-level keyword extraction credited to authors
- Graph-enhanced retrieval strategy acknowledged

## ASCII Diagrams Created

1. Vector similarity problem diagram
2. Query modes architecture
3. LightRAG keyword flow
4. Mode selection decision tree

## Benchmarks Included

| Mode   | Latency | Quality |
| ------ | ------- | ------- |
| Naive  | 48ms    | 6.2/10  |
| Local  | 142ms   | 7.8/10  |
| Global | 195ms   | 7.5/10  |
| Hybrid | 245ms   | 8.5/10  |

## Progress Summary

**Total articles this iteration**: 6 platform formats
**Running total**: 41 articles/posts created
**Iterations completed**: 8 of 50

## Next Iteration: Entity Deduplication & Normalization

Topic: How EdgeQuake handles duplicate entities across documents
Focus: Normalization rules, merge strategies, description aggregation
